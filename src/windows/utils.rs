use windows::core::GUID;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Media::Audio::IMMDevice;
use windows::Win32::System::Com::StructuredStorage::PropVariantClear;
use windows::Win32::System::Com::PROPVARIANT;
use windows::Win32::System::Com::{
    CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, STGM_READ,
};

use windows::Win32::System::ProcessStatus::K32GetModuleBaseNameW;
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;

pub const PKEY_DEVICE_FRIENDLY_NAME: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a85090),
    pid: 14,
};

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

unsafe fn prop_variant_to_string(variant: &PROPVARIANT) -> String {
    match variant.Anonymous.Anonymous.vt.0 {
        8 => {
            let bstr = variant.Anonymous.Anonymous.Anonymous.bstrVal;
            if bstr.is_null() {
                String::new()
            } else {
                bstr.to_string()
            }
        }
        31 => {
            let psz = variant.Anonymous.Anonymous.Anonymous.pszVal;
            if psz.is_null() {
                String::new()
            } else {
                psz.to_string().unwrap_or_default()
            }
        }
        _ => String::new(),
    }
}

pub unsafe fn get_device_name(device: &IMMDevice) -> String {
    let prop_store = match device.OpenPropertyStore(STGM_READ) {
        Ok(store) => store,
        Err(_) => return "Default Device".to_string(),
    };

    let mut variant = match prop_store.GetValue(&PKEY_DEVICE_FRIENDLY_NAME) {
        Ok(v) => v,
        Err(_) => return "Default Device".to_string(),
    };

    let name = prop_variant_to_string(&variant);

    let _ = PropVariantClear(&mut variant);

    if name.is_empty() {
        "Default Device".to_string()
    } else {
        name
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
