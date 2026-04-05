use std::fmt;

#[cfg(target_os = "windows")]
use crate::windows::WindowsError;

#[derive(Debug, Clone, PartialEq)]

pub struct Session {
    pub id: u32,
    pub name: String,
    pub pid: u32,
    pub volume: f32,
    pub left_volume: f32,
    pub right_volume: f32,
    pub mute: bool,
    pub device: Option<String>,
    pub channel_count: u32,
}

#[derive(Debug)]

pub enum ControllerError {
    NotFound,

    PermissionDenied,

    InvalidParameter,

    UnsupportedOperation,

    SessionExpired,

    PlatformError(String),

    Other(String),
}

impl fmt::Display for ControllerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "Session not found"),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::InvalidParameter => write!(f, "Invalid parameter"),
            Self::UnsupportedOperation => write!(f, "Unsupported operation"),
            Self::SessionExpired => write!(f, "Session expired"),
            Self::PlatformError(msg) => write!(f, "Platform error: {}", msg),
            Self::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ControllerError {}

pub trait AudioController: Send + Sync {
    fn list_sessions(&self) -> Result<Vec<Session>, ControllerError>;

    fn set_volume(&mut self, id: u32, left: f32, right: f32) -> Result<(), ControllerError>;

    fn set_mute(&mut self, id: u32, mute: bool) -> Result<(), ControllerError>;

    fn refresh_sessions(&mut self) -> Result<(), ControllerError>;

    fn device_name(&self) -> &str;
}

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "windows")]
pub use windows::WindowsController as DefaultController;

#[cfg(target_os = "linux")]
pub use linux::LinuxController as DefaultController;

#[cfg(target_os = "android")]
pub use android::AndroidController as DefaultController;

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "android")))]

compile_error!("Unsupported target OS");

#[cfg(target_os = "windows")]
impl From<WindowsError> for ControllerError {
    fn from(err: WindowsError) -> Self {
        match err {
            WindowsError::Com(e) => ControllerError::PlatformError(e.to_string()),
            WindowsError::NotFound => ControllerError::NotFound,
            WindowsError::PermissionDenied => ControllerError::PermissionDenied,
            WindowsError::InvalidParameter => ControllerError::InvalidParameter,
            WindowsError::SessionExpired => ControllerError::SessionExpired,
            WindowsError::DeviceError(msg) => ControllerError::PlatformError(msg),
            WindowsError::Unsupported(_msg) => ControllerError::UnsupportedOperation,
            WindowsError::Other(msg) => ControllerError::Other(msg),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Debug, Clone)]

    struct MockController {
        sessions: Vec<Session>,
    }

    impl MockController {
        fn new() -> Self {
            Self {
                sessions: vec![
                    Session {
                        id: 1,
                        name: "Test App 1".to_string(),
                        pid: 1234,
                        volume: 0.5,
                        left_volume: 0.5,
                        right_volume: 0.5,
                        mute: false,
                        device: Some("default".to_string()),
                        channel_count: 2,
                    },
                    Session {
                        id: 2,
                        name: "Test App 2".to_string(),
                        pid: 5678,
                        volume: 1.0,
                        left_volume: 1.0,
                        right_volume: 1.0,
                        mute: true,
                        device: None,
                        channel_count: 2,
                    },
                ],
            }
        }
    }

    impl AudioController for MockController {
        fn list_sessions(&self) -> Result<Vec<Session>, ControllerError> {
            Ok(self.sessions.clone())
        }

        fn set_volume(&mut self, id: u32, left: f32, right: f32) -> Result<(), ControllerError> {
            let session = self.sessions.iter_mut().find(|s| s.id == id);

            match session {
                Some(s) => {
                    s.volume = (left + right) / 2.0;

                    Ok(())
                }

                None => Err(ControllerError::NotFound),
            }
        }

        fn set_mute(&mut self, id: u32, mute: bool) -> Result<(), ControllerError> {
            let session = self.sessions.iter_mut().find(|s| s.id == id);

            match session {
                Some(s) => {
                    s.mute = mute;

                    Ok(())
                }

                None => Err(ControllerError::NotFound),
            }
        }

        fn refresh_sessions(&mut self) -> Result<(), ControllerError> {
            Ok(())
        }

        fn device_name(&self) -> &str {
            "Mock Device"
        }
    }

    #[test]

    fn test_list_sessions() {
        let controller = MockController::new();

        let sessions = controller.list_sessions().unwrap();

        assert_eq!(sessions.len(), 2);

        assert_eq!(sessions[0].name, "Test App 1");

        assert_eq!(sessions[1].name, "Test App 2");
    }

    #[test]

    fn test_set_volume() {
        let mut controller = MockController::new();

        controller.set_volume(1, 0.7, 0.9).unwrap();

        let sessions = controller.list_sessions().unwrap();

        let session = sessions.iter().find(|s| s.id == 1).unwrap();

        // Use approximate comparison due to floating point precision
        assert!((session.volume - 0.8).abs() < 1e-6);
    }

    #[test]

    fn test_set_mute() {
        let mut controller = MockController::new();

        controller.set_mute(2, false).unwrap();

        let sessions = controller.list_sessions().unwrap();

        let session = sessions.iter().find(|s| s.id == 2).unwrap();

        assert_eq!(session.mute, false);
    }

    #[test]

    fn test_session_not_found() {
        let mut controller = MockController::new();

        let result = controller.set_volume(999, 0.5, 0.5);

        assert!(matches!(result, Err(ControllerError::NotFound)));
    }
}
