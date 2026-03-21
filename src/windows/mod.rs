mod enumerator;

mod error;

mod session;

mod utils;

pub use enumerator::{EnumeratorConfig, SessionEnumerator};

pub use error::{WindowsError, WindowsResult};

pub use session::WindowsSession;

use crate::{AudioController, Session};

use std::sync::{Arc, Mutex};

pub struct WindowsController {
    enumerator: Arc<Mutex<SessionEnumerator>>,
}

impl WindowsController {
    pub fn new() -> Result<Self, ControllerError> {
        let _com_guard = utils::ComGuard::new().map_err(|e| {
            ControllerError::PlatformError(format!("COM initialization failed: {}", e))
        })?;

        let enumerator = SessionEnumerator::new().map_err(|e| {
            ControllerError::PlatformError(format!("Failed to create enumerator: {}", e))
        })?;

        Ok(Self {
            enumerator: Arc::new(Mutex::new(enumerator)),
        })
    }

    pub fn with_config(config: EnumeratorConfig) -> Result<Self, ControllerError> {
        let _com_guard = utils::ComGuard::new().map_err(|e| {
            ControllerError::PlatformError(format!("COM initialization failed: {}", e))
        })?;

        let enumerator = SessionEnumerator::with_config(config).map_err(|e| {
            ControllerError::PlatformError(format!("Failed to create enumerator: {}", e))
        })?;

        Ok(Self {
            enumerator: Arc::new(Mutex::new(enumerator)),
        })
    }

    pub fn device_name(&self) -> String {
        self.enumerator.lock().unwrap().device_name().to_string()
    }

    pub fn windows_sessions(&self) -> Vec<WindowsSession> {
        self.enumerator.lock().unwrap().sessions()
    }

    pub fn session_count(&self) -> usize {
        self.enumerator.lock().unwrap().len()
    }
}

impl AudioController for WindowsController {
    fn list_sessions(&self) -> Result<Vec<Session>, ControllerError> {
        let enumerator = self.enumerator.lock().unwrap();

        Ok(enumerator
            .sessions()
            .iter()
            .map(|ws| Session {
                id: ws.id,

                name: ws.name.clone(),

                pid: ws.pid,

                volume: ws.volume,

                mute: ws.mute,

                device: ws.device.clone(),

                channel_count: ws.channel_count,
            })
            .collect())
    }

    fn refresh_sessions(&mut self) -> Result<(), ControllerError> {
        let mut enumerator = self.enumerator.lock().unwrap();

        unsafe {
            enumerator.refresh().map_err(|e| ControllerError::from(e))?;
        }
    }

    fn set_volume(&mut self, id: u32, left: f32, right: f32) -> Result<(), ControllerError> {
        if id == 0 {
            return Err(ControllerError::InvalidParameter);
        }
        if !(0.0..=1.0).contains(&left) || !(0.0..=1.0).contains(&right) {
            return Err(ControllerError::InvalidParameter);
        }

        let enumerator = self.enumerator.lock().unwrap();

        let session = enumerator
            .get_session(id)
            .ok_or(ControllerError::NotFound)?;

        unsafe {
            session.set_volume(left, right)?;
        }

        Ok(())
    }

    fn set_mute(&mut self, id: u32, mute: bool) -> Result<(), ControllerError> {
        if id == 0 {
            return Err(ControllerError::InvalidParameter);
        }

        let enumerator = self.enumerator.lock().unwrap();

        let session = enumerator
            .get_session(id)
            .ok_or(ControllerError::NotFound)?;

        unsafe {
            session.set_mute(mute)?;
        }

        Ok(())
    }
}

impl std::fmt::Debug for WindowsController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let enumerator = self.enumerator.lock().unwrap();

        f.debug_struct("WindowsController")
            .field("device_name", &enumerator.device_name())
            .field("session_count", &enumerator.len())
            .field("config", &"See EnumeratorConfig".to_string())
            .finish()
    }
}

impl Clone for WindowsController {
    fn clone(&self) -> Self {
        Self {
            enumerator: Arc::clone(&self.enumerator),
        }
    }
}

unsafe impl Send for WindowsController {}

unsafe impl Sync for WindowsController {}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]
    #[cfg(target_os = "windows")]

    fn test_controller_creation() {
        let result = WindowsController::new();

        assert!(result.is_ok());

        let controller = result.unwrap();

        assert_eq!(controller.session_count(), 0);
    }

    #[test]
    #[cfg(target_os = "windows")]

    fn test_controller_with_config() {
        let config = EnumeratorConfig {
            device_role: DeviceRole::Console,

            include_system: false,

            min_volume_threshold: 0.1,
        };

        let result = WindowsController::with_config(config);

        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "windows")]

    fn test_device_name() {
        let controller = WindowsController::new().unwrap();

        let name = controller.device_name();

        assert!(!name.is_empty());
    }
}
