use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};

use super::{AudioController, ControllerError, Session};

pub struct AndroidController {
    _private: (),
}

impl AndroidController {
    pub fn new() -> Result<Self, ControllerError> {
        Err(ControllerError::PlatformError(
            "Android implementation not ready".to_string(),
        ))
    }

    pub unsafe fn from_jni(
        _jni_env: *mut c_void,
        _audio_service: *mut c_void,
    ) -> Result<Self, ControllerError> {
        Ok(Self { _private: () })
    }
}

impl AudioController for AndroidController {
    fn list_sessions(&self) -> Result<Vec<Session>, ControllerError> {
        Err(ControllerError::PlatformError(
            "Android implementation not ready".to_string(),
        ))
    }

    fn set_volume(&mut self, _id: u32, _left: f32, _right: f32) -> Result<(), ControllerError> {
        Err(ControllerError::PlatformError(
            "Android implementation not ready".to_string(),
        ))
    }

    fn set_mute(&mut self, _id: u32, _mute: bool) -> Result<(), ControllerError> {
        Err(ControllerError::PlatformError(
            "Android implementation not ready".to_string(),
        ))
    }
}

impl Drop for AndroidController {
    fn drop(&mut self) {}
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_example_audio_AudioController_nativeCreate(
    _env: *mut c_void,
    _obj: *mut c_void,
) -> *mut c_void {
    match AndroidController::new() {
        Ok(ctrl) => {
            let boxed = Box::new(ctrl);
            Box::into_raw(boxed) as *mut c_void
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_example_audio_AudioController_nativeDestroy(
    _env: *mut c_void,
    _obj: *mut c_void,
    ptr: *mut c_void,
) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut AndroidController);
        }
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn Java_com_example_audio_AudioController_nativeListSessions(
    _env: *mut c_void,
    _obj: *mut c_void,
    ptr: *mut c_void,
) -> *mut *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }

    let controller = unsafe { &*(ptr as *const AndroidController) };
    match controller.list_sessions() {
        Ok(sessions) => std::ptr::null_mut(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_android_controller_creation() {
        let result = AndroidController::new();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ControllerError::PlatformError(_)
        ));
    }
}
