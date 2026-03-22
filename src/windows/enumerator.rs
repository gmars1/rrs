use super::error::WindowsResult;

use super::session::WindowsSession;

use crate::ControllerError;

use std::collections::HashMap;

use windows::core::Interface;
use windows::Win32::Media::Audio::{self, IAudioSessionEnumerator, IAudioSessionManager2};

use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

#[derive(Debug, Clone, Copy)]

pub enum DeviceRole {
    Console,

    Multimedia,

    Communications,
}

impl DeviceRole {
    pub fn as_erole(&self) -> Audio::ERole {
        match self {
            DeviceRole::Console => Audio::eConsole,

            DeviceRole::Multimedia => Audio::eMultimedia,

            DeviceRole::Communications => Audio::eCommunications,
        }
    }
}

#[derive(Debug, Clone)]

pub struct EnumeratorConfig {
    pub device_role: DeviceRole,

    pub include_system: bool,

    pub min_volume_threshold: f32,
}

impl Default for EnumeratorConfig {
    fn default() -> Self {
        Self {
            device_role: DeviceRole::Console,

            include_system: true,

            min_volume_threshold: 0.0,
        }
    }
}

pub struct SessionEnumerator {
    config: EnumeratorConfig,

    sessions: HashMap<u32, WindowsSession>,

    device_name: String,
}

impl SessionEnumerator {
    pub fn new() -> WindowsResult<Self> {
        Self::with_config(EnumeratorConfig::default())
    }

    pub fn with_config(config: EnumeratorConfig) -> WindowsResult<Self> {
        if !(0.0..=1.0).contains(&config.min_volume_threshold) {
            return Err(super::error::WindowsError::InvalidParameter);
        }

        Ok(Self {
            config,
            sessions: HashMap::new(),
            device_name: String::new(),
        })
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn sessions(&self) -> Vec<WindowsSession> {
        self.sessions.values().cloned().collect()
    }

    pub fn get_session(&self, id: u32) -> Option<WindowsSession> {
        self.sessions.get(&id).cloned()
    }

    pub unsafe fn refresh(&mut self) -> WindowsResult<usize> {
        self.discover_sessions()?;

        Ok(self.sessions.len())
    }

    unsafe fn discover_sessions(&mut self) -> WindowsResult<usize> {
        let enumerator: Audio::IMMDeviceEnumerator =
            CoCreateInstance(&Audio::MMDeviceEnumerator, None, CLSCTX_ALL)?;

        let role = self.config.device_role.as_erole();

        let device = enumerator.GetDefaultAudioEndpoint(Audio::eRender, role)?;

        self.device_name = super::utils::get_device_name(&device);

        let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;

        let session_enumerator: IAudioSessionEnumerator = manager.GetSessionEnumerator()?;

        let count = session_enumerator.GetCount()?;

        let mut target_sessions = HashMap::new();

        for i in 0..count {
            match self.try_get_session(i, &session_enumerator) {
                Ok(Some(session)) => {
                    if self.should_include_session(&session) {
                        target_sessions.insert(session.id, session);
                    }
                }

                Ok(None) => continue,

                Err(e) => {
                    eprintln!("Warning: Failed to get session {}: {}", i, e);
                }
            }
        }

        self.sessions = target_sessions;
        Ok(self.sessions.len())
    }

    unsafe fn try_get_session(
        &self,

        index: i32,

        enumerator: &IAudioSessionEnumerator,
    ) -> WindowsResult<Option<WindowsSession>> {
        let session = match enumerator.GetSession(index) {
            Ok(s) => s,

            Err(_) => return Ok(None),
        };

        let session_control: Audio::IAudioSessionControl2 = session.cast()?;

        let device_name = self.device_name.clone();

        let session_id = {
            let raw_id = session_control.GetSessionId()?;
            raw_id.Data1
        };

        match WindowsSession::new(session_id, session_control, &device_name) {
            Ok(session) => Ok(Some(session)),

            Err(e) => Err(e),
        }
    }

    fn should_include_session(&self, session: &WindowsSession) -> bool {
        if session.volume < self.config.min_volume_threshold {
            return false;
        }

        if !self.config.include_system && session.pid == 0 {
            return false;
        }

        true
    }

    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    pub fn clear(&mut self) {
        self.sessions.clear();
    }
}

impl Default for SessionEnumerator {
    fn default() -> Self {
        Self::new().expect("Failed to create SessionEnumerator")
    }
}

impl Iterator for SessionEnumerator {
    type Item = WindowsSession;

    fn next(&mut self) -> Option<Self::Item> {
        if self.sessions.is_empty() {
            None
        } else {
            let first = self.sessions.values().next().cloned();

            self.sessions.clear();

            first
        }
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]
    #[cfg(target_os = "windows")]

    fn test_enumerator_creation() {
        let enumerator = SessionEnumerator::new();

        assert!(enumerator.is_ok());
    }

    #[test]
    #[cfg(target_os = "windows")]

    fn test_default_config() {
        let config = EnumeratorConfig::default();

        assert!(matches!(config.device_role, DeviceRole::Console));

        assert!(config.include_system);

        assert_eq!(config.min_volume_threshold, 0.0);
    }

    #[test]
    #[cfg(target_os = "windows")]

    fn test_device_role_conversion() {
        assert_eq!(DeviceRole::Console.as_erole(), Audio::eConsole);

        assert_eq!(DeviceRole::Multimedia.as_erole(), Audio::eMultimedia);

        assert_eq!(
            DeviceRole::Communications.as_erole(),
            Audio::eCommunications
        );
    }
}
