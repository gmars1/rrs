mod enumerator;

mod error;

mod session;

mod utils;

pub use enumerator::{EnumeratorConfig, SessionEnumerator};

pub use error::WindowsError;

pub use session::WindowsSession;

use crate::{AudioController, ControllerError, Session};

use std::sync::{Arc, Mutex};

pub struct WindowsController {
    enumerator: Arc<Mutex<SessionEnumerator>>,
    device_name: String,
    _com_guard: utils::ComGuard,
}

impl WindowsController {
    pub fn new() -> Result<Self, ControllerError> {
        let com_guard = utils::ComGuard::new().map_err(|e| {
            ControllerError::PlatformError(format!("COM initialization failed: {}", e))
        })?;

        let enumerator = SessionEnumerator::new().map_err(|e| {
            ControllerError::PlatformError(format!("Failed to create enumerator: {}", e))
        })?;

        let mut controller = Self {
            enumerator: Arc::new(Mutex::new(enumerator)),
            device_name: String::new(),
            _com_guard: com_guard,
        };
        controller.refresh_sessions()?;
        Ok(controller)
    }

    pub fn with_config(config: EnumeratorConfig) -> Result<Self, ControllerError> {
        let com_guard = utils::ComGuard::new().map_err(|e| {
            ControllerError::PlatformError(format!("COM initialization failed: {}", e))
        })?;

        let enumerator = SessionEnumerator::with_config(config).map_err(|e| {
            ControllerError::PlatformError(format!("Failed to create enumerator: {}", e))
        })?;

        let mut controller = Self {
            enumerator: Arc::new(Mutex::new(enumerator)),
            device_name: String::new(),
            _com_guard: com_guard,
        };
        controller.refresh_sessions()?;
        Ok(controller)
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn windows_sessions(&self) -> Vec<WindowsSession> {
        self.enumerator
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .sessions()
    }

    pub fn session_count(&self) -> usize {
        self.enumerator
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }
}

impl AudioController for WindowsController {
    fn list_sessions(&self) -> Result<Vec<Session>, ControllerError> {
        let enumerator = self
            .enumerator
            .lock()
            .map_err(|_| ControllerError::Other("Mutex poisoned".to_string()))?;

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

    fn device_name(&self) -> &str {
        &self.device_name
    }

    fn refresh_sessions(&mut self) -> Result<(), ControllerError> {
        let mut enumerator = self
            .enumerator
            .lock()
            .map_err(|_| ControllerError::Other("Mutex poisoned".to_string()))?;

        unsafe {
            enumerator.refresh().map_err(|e| ControllerError::from(e))?;
        }

        // Update cached device name
        let enumerator = self
            .enumerator
            .lock()
            .map_err(|_| ControllerError::Other("Mutex poisoned".to_string()))?;
        self.device_name = enumerator.device_name().to_string();

        Ok(())
    }

    fn set_volume(&mut self, id: u32, left: f32, right: f32) -> Result<(), ControllerError> {
        if id == 0 {
            return Err(ControllerError::InvalidParameter);
        }
        if !(0.0..=1.0).contains(&left) || !(0.0..=1.0).contains(&right) {
            return Err(ControllerError::InvalidParameter);
        }

        let enumerator = self
            .enumerator
            .lock()
            .map_err(|_| ControllerError::Other("Mutex poisoned".to_string()))?;

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

        let enumerator = self
            .enumerator
            .lock()
            .map_err(|_| ControllerError::Other("Mutex poisoned".to_string()))?;

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
        let enumerator = self.enumerator.lock().unwrap_or_else(|e| e.into_inner());

        f.debug_struct("WindowsController")
            .field("device_name", &self.device_name)
            .field("session_count", &enumerator.len())
            .field("config", &"See EnumeratorConfig".to_string())
            .finish()
    }
}

impl Clone for WindowsController {
    fn clone(&self) -> Self {
        Self {
            enumerator: Arc::clone(&self.enumerator),
            device_name: self.device_name.clone(),
            _com_guard: utils::ComGuard::new().expect("COM reinit failed"),
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
