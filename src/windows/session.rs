use super::error::{WindowsError, WindowsResult};

use std::ptr;

use windows::Win32::Media::Audio::{
    IAudioSessionControl2, IChannelAudioVolume, ISimpleAudioVolume,
};

pub struct WindowsSession {
    pub id: u32,

    pub name: String,

    pub pid: u32,

    pub volume: f32,

    pub mute: bool,

    pub device: Option<String>,

    pub channel_count: u32,

    session_control: Option<IAudioSessionControl2>,
}

impl WindowsSession {
    pub unsafe fn new(
        id: u32,
        session_control: IAudioSessionControl2,
        device_name: &str,
    ) -> WindowsResult<Self> {
        let pid = session_control.GetProcessId().unwrap_or(0);

        let name = if pid == 0 {
            "System Sounds".to_string()
        } else {
            super::utils::get_process_name(pid)
        };

        let simple_vol: ISimpleAudioVolume =
            unsafe { <IAudioSessionControl2 as windows::core::Interface>::cast(&session_control)? };

        let volume = simple_vol.GetMasterVolume()?;

        let mute = simple_vol.GetMute()?.into();

        let channel_count = match unsafe {
            <IAudioSessionControl2 as windows::core::Interface>::cast::<IChannelAudioVolume>(
                &session_control,
            )
        } {
            Ok(channel_vol) => {
                let channels = channel_vol.GetChannelCount()?;
                if channels == 0 {
                    0
                } else {
                    channels
                }
            }
            Err(_) => 0,
        };

        Ok(Self {
            id,
            name,
            pid,
            volume,
            mute,
            device: Some(device_name.to_string()),
            channel_count,
            session_control: Some(session_control),
        })
    }

    pub fn get_simple_volume(&self) -> WindowsResult<ISimpleAudioVolume> {
        let control = match self.session_control.as_ref() {
            Some(c) => c,
            None => {
                return Err(super::error::WindowsError::other(
                    "Session control not available",
                ))
            }
        };

        Ok(<IAudioSessionControl2 as windows::core::Interface>::cast(
            control,
        )?)
    }

    pub fn get_channel_volume(&self) -> WindowsResult<IChannelAudioVolume> {
        let control = match self.session_control.as_ref() {
            Some(c) => c,
            None => {
                return Err(super::error::WindowsError::other(
                    "Session control not available",
                ))
            }
        };

        Ok(<IAudioSessionControl2 as windows::core::Interface>::cast(
            control,
        )?)
    }

    pub unsafe fn set_volume(&self, left: f32, right: f32) -> WindowsResult<()> {
        if !(0.0..=1.0).contains(&left) || !(0.0..=1.0).contains(&right) {
            return Err(super::error::WindowsError::InvalidParameter);
        }

        match self.get_channel_volume() {
            Ok(channel_vol) => {
                let channel_count = self.channel_count as usize;

                if channel_count == 0 {
                    let avg = (left + right) / 2.0;
                    let simple_vol = self.get_simple_volume()?;
                    simple_vol.SetMasterVolume(avg, ptr::null())?;
                    return Ok(());
                }

                let mut volumes = vec![0.0f32; channel_count];
                if channel_count >= 2 {
                    volumes[0] = left;
                    volumes[1] = right;

                    for v in &mut volumes[2..] {
                        *v = (left + right) / 2.0;
                    }
                } else if channel_count == 1 {
                    volumes[0] = (left + right) / 2.0;
                }

                channel_vol.SetAllVolumes(&volumes, ptr::null())?;
                Ok(())
            }
            Err(_) => {
                let avg = (left + right) / 2.0;
                let simple_vol = self.get_simple_volume()?;
                simple_vol.SetMasterVolume(avg, ptr::null())?;
                Ok(())
            }
        }
    }

    pub unsafe fn set_mute(&self, mute: bool) -> WindowsResult<()> {
        let simple_vol = self.get_simple_volume()?;
        simple_vol.SetMute(mute, ptr::null())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub unsafe fn refresh(&self) -> WindowsResult<Self> {
        let simple_vol = self.get_simple_volume()?;
        let volume = simple_vol.GetMasterVolume()?;
        let mute = simple_vol.GetMute()?.into();

        Ok(Self {
            id: self.id,
            name: self.name.clone(),
            pid: self.pid,
            volume,
            mute,
            device: self.device.clone(),
            channel_count: self.channel_count,

            session_control: self.session_control.clone(),
        })
    }
}

impl Clone for WindowsSession {
    fn clone(&self) -> Self {
        let session_control = match self.session_control.as_ref() {
            Some(sc) => Some(sc.clone()),
            None => None,
        };
        Self {
            id: self.id,
            name: self.name.clone(),
            pid: self.pid,
            volume: self.volume,
            mute: self.mute,
            device: self.device.clone(),
            channel_count: self.channel_count,
            session_control,
        }
    }
}

impl Drop for WindowsSession {
    fn drop(&mut self) {
        self.session_control = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::ComPtr;
    use windows::Win32::Media::Audio::IAudioSessionControl2;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_clone_safety() {}
}
