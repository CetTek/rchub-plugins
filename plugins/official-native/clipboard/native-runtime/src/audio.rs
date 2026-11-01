//! Private Audio ABI and Windows endpoint provider; no shell or ambient device selection.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct State {
    pub volume: u8,
    pub muted: bool,
}
impl State {
    pub fn decode(value: i32) -> Result<Self, ()> {
        if value < 0 || value & !0x1ff != 0 || value & 255 > 100 {
            return Err(());
        }
        Ok(Self {
            volume: (value & 255) as u8,
            muted: value & 256 != 0,
        })
    }
}
pub(super) fn invoke(op: i32, value: i32, check: &dyn Fn() -> bool) -> Result<i32, ()> {
    if !matches!((op, value), (0, 0) | (1, 0..=100) | (2, 0..=1)) || !check() {
        return Err(());
    }
    #[cfg(windows)]
    {
        windows::invoke(op, value, check)
    }
    #[cfg(not(windows))]
    {
        Err(())
    }
}
#[cfg(windows)]
mod windows {
    use std::{
        ffi::c_void,
        ptr::{null, null_mut},
    };
    use windows_sys::{
        core::GUID,
        Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
        },
    };
    type Hr = i32;
    // COM vtables follow the Windows SDK endpointvolume.h/mmdeviceapi.h order.
    #[repr(C)]
    struct Unknown {
        query: usize,
        add_ref: usize,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
    }
    #[repr(C)]
    struct Enumerator {
        base: Unknown,
        enum_endpoints: usize,
        default_endpoint: unsafe extern "system" fn(*mut c_void, i32, i32, *mut *mut c_void) -> Hr,
    }
    #[repr(C)]
    struct Device {
        base: Unknown,
        activate: unsafe extern "system" fn(
            *mut c_void,
            *const GUID,
            u32,
            *mut c_void,
            *mut *mut c_void,
        ) -> Hr,
    }
    #[repr(C)]
    struct Volume {
        base: Unknown,
        register: usize,
        unregister: usize,
        channels: usize,
        set_db: usize,
        set_scalar: unsafe extern "system" fn(*mut c_void, f32, *const GUID) -> Hr,
        get_db: usize,
        get_scalar: unsafe extern "system" fn(*mut c_void, *mut f32) -> Hr,
        set_channel_db: usize,
        set_channel_scalar: usize,
        get_channel_db: usize,
        get_channel_scalar: usize,
        set_mute: unsafe extern "system" fn(*mut c_void, i32, *const GUID) -> Hr,
        get_mute: unsafe extern "system" fn(*mut c_void, *mut i32) -> Hr,
    }
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() }
        }
    }
    struct Interface(*mut c_void);
    impl Interface {
        unsafe fn table<T>(&self) -> &T {
            &**(self.0 as *const *const T)
        }
    }
    impl Drop for Interface {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    (self.table::<Unknown>().release)(self.0);
                }
            }
        }
    }
    fn ok(hr: Hr) -> Result<(), ()> {
        if hr >= 0 {
            Ok(())
        } else {
            Err(())
        }
    }
    pub fn invoke(op: i32, value: i32, check: &dyn Fn() -> bool) -> Result<i32, ()> {
        // All interfaces and apartment are owned/released synchronously on this worker thread.
        unsafe {
            ok(CoInitializeEx(null(), COINIT_MULTITHREADED as u32))?;
            let _apartment = Apartment;
            let class = GUID::from_u128(0xbcde0395_e52f_467c_8e3d_c4579291692e);
            let enum_id = GUID::from_u128(0xa95664d2_9614_4f35_a746_de8db63617e6);
            let volume_id = GUID::from_u128(0x5cdf2c82_841e_4546_9722_0cf74078229a);
            let mut enumerator = Interface(null_mut());
            ok(CoCreateInstance(
                &class,
                null_mut(),
                CLSCTX_ALL,
                &enum_id,
                &mut enumerator.0,
            ))?;
            if enumerator.0.is_null() {
                return Err(());
            }
            let mut device = Interface(null_mut());
            ok((enumerator.table::<Enumerator>().default_endpoint)(
                enumerator.0,
                0,
                1,
                &mut device.0,
            ))?;
            if device.0.is_null() {
                return Err(());
            }
            let mut volume = Interface(null_mut());
            ok((device.table::<Device>().activate)(
                device.0,
                &volume_id,
                CLSCTX_ALL,
                null_mut(),
                &mut volume.0,
            ))?;
            if volume.0.is_null() {
                return Err(());
            }
            let table = volume.table::<Volume>();
            if !check() {
                return Err(());
            }
            match op {
                0 => (),
                1 => ok((table.set_scalar)(volume.0, value as f32 / 100.0, null()))?,
                2 => ok((table.set_mute)(volume.0, value, null()))?,
                _ => return Err(()),
            }
            let mut scalar = 0.0;
            let mut mute = 0;
            if !check() {
                return Err(());
            }
            ok((table.get_scalar)(volume.0, &mut scalar))?;
            if !check() {
                return Err(());
            }
            ok((table.get_mute)(volume.0, &mut mute))?;
            if !scalar.is_finite() || !(0.0..=1.0).contains(&scalar) || !matches!(mute, 0 | 1) {
                return Err(());
            }
            Ok((scalar * 100.0).round() as i32 | (mute << 8))
        }
    }
}
