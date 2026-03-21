use windows::core::Error as WindowsCoreError;

pub type WindowsResult<T> = Result<T, WindowsError>;

#[derive(Debug, Clone)]

pub enum WindowsError {
    Com(WindowsCoreError),

    NotFound,

    PermissionDenied,

    InvalidParameter,

    SessionExpired,

    DeviceError(String),

    Unsupported(String),

    Other(String),
}

impl From<WindowsCoreError> for WindowsError {
    fn from(err: WindowsCoreError) -> Self {
        let code = err.code();

        match code {
            windows::Win32::Foundation::E_ACCESSDENIED => Self::PermissionDenied,

            windows::Win32::Foundation::E_INVALIDARG => Self::InvalidParameter,

            windows::Win32::Media::Audio::AUDCLNT_E_DEVICE_INVALIDATED => Self::SessionExpired,
            windows::Win32::Media::Audio::AUDCLNT_E_NOT_STOPPED => Self::NotFound,
            windows::Win32::Media::Audio::AUDCLNT_E_ENDPOINT_CREATE_FAILED => {
                Self::DeviceError("Failed to create audio endpoint".to_string())
            }
            windows::Win32::Media::Audio::AUDCLNT_E_SERVICE_NOT_RUNNING => {
                Self::DeviceError("Windows Audio service is not running".to_string())
            }

            windows::Win32::Foundation::E_NOINTERFACE => {
                Self::Unsupported("Required interface not available".to_string())
            }
            windows::Win32::Foundation::E_NOTIMPL => {
                Self::Unsupported("Operation not implemented".to_string())
            }

            _ => Self::Com(err),
        }
    }
}

impl std::fmt::Display for WindowsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Com(err) => {
                let code = err.code();

                match code {
                    windows::Win32::Media::Audio::AUDCLNT_E_DEVICE_INVALIDATED => {
                        write!(f, "Audio device was removed or became invalid")
                    }

                    windows::Win32::Foundation::E_ACCESSDENIED => {
                        write!(
                            f,
                            "Access denied. Run as administrator or adjust permissions"
                        )
                    }
                    windows::Win32::Foundation::E_INVALIDARG => {
                        write!(f, "Invalid parameter passed to COM function")
                    }
                    _ => {
                        write!(f, "COM error (0x{:X}): {}", code.0 as u32, err.to_string())
                    }
                }
            }

            Self::NotFound => write!(f, "Audio session not found"),

            Self::PermissionDenied => {
                write!(f, "Permission denied. Run as administrator or adjust ACLs")
            }

            Self::InvalidParameter => write!(
                f,
                "Invalid parameter (volume must be 0.0-1.0, session ID must be valid)"
            ),

            Self::SessionExpired => write!(f, "Audio session has ended"),

            Self::DeviceError(msg) => write!(f, "Audio device error: {}", msg),

            Self::Unsupported(msg) => write!(f, "Unsupported operation: {}", msg),

            Self::Other(msg) => write!(f, "Audio controller error: {}", msg),
        }
    }
}

impl std::error::Error for WindowsError {}

impl WindowsError {
    pub fn from_core_error(err: windows::core::Error) -> Self {
        err.into()
    }

    pub fn device_error(msg: impl Into<String>) -> Self {
        Self::DeviceError(msg.into())
    }

    pub fn unsupported(msg: impl Into<String>) -> Self {
        Self::Unsupported(msg.into())
    }

    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }

    pub fn hresult(&self) -> Option<windows::core::HRESULT> {
        match self {
            Self::Com(err) => Some(err.code()),

            _ => None,
        }
    }

    pub fn code(&self) -> Option<u32> {
        self.hresult().map(|hr| hr.0 as u32)
    }

    pub fn is_session_expired(&self) -> bool {
        matches!(self, Self::SessionExpired)
            || matches!(self.hresult(), Some(hr) if {
                let code = hr.0;
                code == windows::Win32::Media::Audio::AUDCLNT_E_DEVICE_INVALIDATED.0 ||
                code == windows::Win32::Media::Audio::AUDCLNT_E_SERVICE_NOT_RUNNING.0
            })
    }

    pub fn is_permission_denied(&self) -> bool {
        matches!(self, Self::PermissionDenied)
            || matches!(self.hresult(), Some(hr) if {
                let code = hr.0;
                code == windows::Win32::Foundation::E_ACCESSDENIED.0 ||
                code == windows::Win32::Foundation::E_NOINTERFACE.0
            })
    }
}
