use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Media::Audio::IMMDevice;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

use windows::Win32::System::ProcessStatus::K32GetModuleBaseNameW;
use windows::Win32::System::Threading::OpenProcess;

pub unsafe fn get_process_name(pid: u32) -> String {
    let access = windows::Win32::System::Threading::PROCESS_QUERY_INFORMATION
        | windows::Win32::System::Threading::PROCESS_VM_READ;

    let handle: HANDLE = match OpenProcess(access, false, pid) {
        Ok(h) => h,
        Err(_) => return "System/Protected".to_string(),
    };

    let mut buffer = [0u16; 260];
    let len = K32GetModuleBaseNameW(handle, None, &mut buffer);

    let _ = CloseHandle(handle);

    if len > 0 {
        let slice_len = if len < buffer.len() as u32 {
            len as usize
        } else {
            buffer.len()
        };
        String::from_utf16_lossy(&buffer[..slice_len])
    } else {
        "Unknown".to_string()
    }
}

pub unsafe fn get_device_name(device: &IMMDevice) -> String {
    match device.GetId() {
        Ok(id) => {
            let name = id.to_string().unwrap_or_default();
            if name.is_empty() {
                "Default Device".to_string()
            } else {
                name
            }
        }
        Err(_) => "Default Device".to_string(),
    }
}

pub struct ComGuard {
    initialized: bool,
}

impl ComGuard {
    pub fn new() -> windows::core::Result<Self> {
        unsafe {
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if hr.is_ok() {
                let initialized = hr.0 == 0;
                Ok(Self { initialized })
            } else {
                Err(hr.into())
            }
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                CoUninitialize();
            }
        }
    }
}
