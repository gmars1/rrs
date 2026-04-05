use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::{AudioController, ControllerError, Session};

const PACTL_TIMEOUT: Duration = Duration::from_secs(5);

pub struct LinuxController {
    sessions: HashMap<u32, Session>,
    device_name: String,
}

impl LinuxController {
    fn get_default_sink_name() -> Result<String, ControllerError> {
        let output = Self::run_pactl(&["info"])?;
        for line in output.lines() {
            if line.trim().starts_with("Default Sink:") {
                if let Some(name) = line.split(':').nth(1) {
                    return Ok(name.trim().to_string());
                }
            }
        }
        Ok("default".to_string())
    }

    fn check_pactl() -> Result<(), ControllerError> {
        let output = Command::new("which")
            .arg("pactl")
            .output()
            .map_err(|e| ControllerError::PlatformError(e.to_string()))?;

        if !output.status.success() {
            return Err(ControllerError::PlatformError(
                "pactl not found. Install pulseaudio-utils".to_string(),
            ));
        }
        Ok(())
    }

    fn run_pactl(args: &[&str]) -> Result<String, ControllerError> {
        let child = Command::new("pactl")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ControllerError::PlatformError(e.to_string()))?;

        let pid = child.id();
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let result = child.wait_with_output();
            let _ = tx.send(result);
        });

        let output = rx
            .recv_timeout(PACTL_TIMEOUT)
            .map_err(|_| {
                let _ = std::process::Command::new("kill")
                    .arg(pid.to_string())
                    .output();
                ControllerError::PlatformError(format!(
                    "pactl {:?} timed out after {}s",
                    args,
                    PACTL_TIMEOUT.as_secs()
                ))
            })?
            .map_err(|e| ControllerError::PlatformError(e.to_string()))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            let code = output.status.code().unwrap_or(-1);
            return Err(ControllerError::PlatformError(format!(
                "pactl failed (exit {}): {}",
                code,
                err.trim()
            )));
        }

        String::from_utf8(output.stdout).map_err(|e| ControllerError::PlatformError(e.to_string()))
    }

    fn parse_sessions(output: &str) -> Vec<Session> {
        let mut sessions = Vec::new();

        let mut current = Session {
            id: 0,
            name: String::new(),
            pid: 0,
            volume: 1.0,
            mute: false,
            device: None,
            channel_count: 0,
        };

        let mut in_session = false;

        for line in output.lines() {
            let line = line.trim();

            if line.starts_with("Sink Input #") {
                if in_session && current.id != 0 {
                    sessions.push(current.clone());
                }

                in_session = true;

                current = Session {
                    id: 0,
                    name: String::new(),
                    pid: 0,
                    volume: 1.0,
                    mute: false,
                    device: None,
                    channel_count: 0,
                };

                if let Some(id_str) = line.strip_prefix("Sink Input #") {
                    current.id = id_str.parse().unwrap_or(0);
                }
            } else if in_session {
                if line.starts_with("application.name =") {
                    if let Some(name) = line.strip_prefix("application.name =") {
                        current.name = name.trim_matches('"').to_string();
                    }
                } else if line.starts_with("application.process.id =") {
                    if let Some(pid_str) = line.strip_prefix("application.process.id =") {
                        current.pid = pid_str.trim().parse().unwrap_or(0);
                    }
                } else if line.starts_with("sink =") {
                    if let Some(device) = line.strip_prefix("sink =") {
                        let device = device.trim();
                        current.device = Some(device.to_string());
                    }
                } else if line.starts_with("volume:") {
                    let channel_count = line.split(',').count();
                    if channel_count > 0 {
                        current.channel_count = channel_count as u32;
                    }

                    if let Some(percent_part) = line.split(',').next() {
                        if let Some(percent_str) = percent_part.split('%').next() {
                            if let Some(vol_str) = percent_str.split('/').next_back() {
                                if let Ok(vol) = vol_str.trim().parse::<u32>() {
                                    current.volume = (vol as f32 / 100.0).clamp(0.0, 1.0);
                                }
                            }
                        }
                    }
                } else if line.starts_with("Mute:") {
                    if let Some(mute_str) = line.strip_prefix("Mute:") {
                        current.mute = mute_str.trim().eq_ignore_ascii_case("yes");
                    }
                }
            }
        }

        if in_session && current.id != 0 {
            sessions.push(current);
        }

        sessions
    }

    pub fn new() -> Result<Self, ControllerError> {
        Self::check_pactl()?;
        let device_name = Self::get_default_sink_name()?;
        let mut controller = Self {
            sessions: HashMap::new(),
            device_name,
        };
        controller.refresh_sessions()?;
        Ok(controller)
    }
}

impl AudioController for LinuxController {
    fn device_name(&self) -> &str {
        &self.device_name
    }

    fn list_sessions(&self) -> Result<Vec<Session>, ControllerError> {
        Ok(self.sessions.values().cloned().collect())
    }

    fn refresh_sessions(&mut self) -> Result<(), ControllerError> {
        let output = Self::run_pactl(&["list", "sink-inputs"])?;

        let sessions = Self::parse_sessions(&output);

        let mut new_map = HashMap::new();
        for session in sessions {
            new_map.insert(session.id, session);
        }

        self.sessions = new_map;
        Ok(())
    }

    fn set_volume(&mut self, id: u32, left: f32, right: f32) -> Result<(), ControllerError> {
        if id == 0 {
            return Err(ControllerError::InvalidParameter);
        }
        if !(0.0..=1.0).contains(&left) || !(0.0..=1.0).contains(&right) {
            return Err(ControllerError::InvalidParameter);
        }

        if !self.sessions.contains_key(&id) {
            return Err(ControllerError::NotFound);
        }

        let avg = ((left + right) / 2.0 * 100.0).round() as u32;

        Self::run_pactl(&[
            "set-sink-input-volume",
            &id.to_string(),
            &format!("{}%", avg),
        ])?;

        if let Some(session) = self.sessions.get_mut(&id) {
            session.volume = avg as f32 / 100.0;
        }

        Ok(())
    }

    fn set_mute(&mut self, id: u32, mute: bool) -> Result<(), ControllerError> {
        if id == 0 {
            return Err(ControllerError::InvalidParameter);
        }

        if !self.sessions.contains_key(&id) {
            return Err(ControllerError::NotFound);
        }

        if let Some(session) = self.sessions.get(&id) {
            if session.mute == mute {
                return Ok(());
            }
        }

        Self::run_pactl(&[
            "set-sink-input-mute",
            &id.to_string(),
            if mute { "1" } else { "0" },
        ])?;

        if let Some(session) = self.sessions.get_mut(&id) {
            session.mute = mute;
        }

        Ok(())
    }
}
