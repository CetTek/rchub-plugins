//! Primary-display DDC/CI with WMI fallback, bounded JPEG screen capture,
//! and native beep. Speech is separately owned by the persistent Speech provider.
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, RecvTimeoutError},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use windows_sys::Win32::{Foundation::POINT, Graphics::Gdi::*};

#[path = "brightness_wmi.rs"]
mod wmi;

const WMI_POLL: Duration = Duration::from_millis(25);
const WMI_DEADLINE: Duration = Duration::from_secs(3);
static WMI_BUSY: AtomicBool = AtomicBool::new(false);

pub(super) fn invoke(cap: &str, input: Value, current: &dyn Fn() -> bool) -> Result<Value, ()> {
    let o = input.as_object().ok_or(())?;
    match cap {
        "display.read" if o.is_empty() => brightness(None, current),
        "display.write" if o.len() == 1 => {
            let n = o
                .get("brightness")
                .and_then(Value::as_u64)
                .filter(|n| *n <= 100)
                .ok_or(())?;
            brightness(Some(n as u32), current)
        }
        "screenshot.read" if o.is_empty() => capture(current),
        "effects.write"
            if o.len() == 1 && o.get("effect").and_then(Value::as_str) == Some("beep") =>
        {
            if !current() {
                return Err(());
            }
            if unsafe { windows_sys::Win32::System::Diagnostics::Debug::MessageBeep(0) } == 0 {
                return Err(());
            }
            Ok(json!({"accepted":true}))
        }
        _ => Err(()),
    }
}
fn brightness(set: Option<u32>, current: &dyn Fn() -> bool) -> Result<Value, ()> {
    brightness_with(&NativeProviders, set, current)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Read {
    Supported(u32),
    Unsupported,
}

trait Providers {
    fn ddc_read(&self, current: &dyn Fn() -> bool) -> Result<Read, ()>;
    fn ddc_write(&self, level: u32, current: &dyn Fn() -> bool) -> Result<u32, ()>;
    fn wmi_read(&self, current: &dyn Fn() -> bool) -> Result<Read, ()>;
    fn wmi_write(&self, level: u32, current: &dyn Fn() -> bool) -> Result<u32, ()>;
}

enum Provider {
    Ddc,
    Wmi,
}

#[derive(Default)]
struct CancelState {
    cancelled: bool,
    side_effect_started: bool,
}

/// Shared cancellation signal between the waiting caller and the WMI helper
/// thread. The caller's `current` closure never crosses the thread boundary.
struct Cancel(Mutex<CancelState>);

impl Cancel {
    fn new() -> Self {
        Self(Mutex::new(CancelState::default()))
    }

    fn with<R>(&self, update: impl FnOnce(&mut CancelState) -> R) -> R {
        let mut state = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        update(&mut state)
    }

    fn is_cancelled(&self) -> bool {
        self.with(|state| state.cancelled)
    }

    /// Returns false when cancellation won the race, meaning the side effect
    /// must not start. If this returns true, the caller can no longer guarantee
    /// cancellation and must report the operation as failed/unknown.
    fn begin_side_effect(&self) -> bool {
        self.with(|state| {
            if state.cancelled {
                false
            } else {
                state.side_effect_started = true;
                true
            }
        })
    }

    fn cancel(&self) -> bool {
        self.with(|state| {
            state.cancelled = true;
            state.side_effect_started
        })
    }
}

enum Flight<T> {
    Done(T),
    Revoked,
    TimedOut,
    Busy,
    Failed,
}

/// Runs at most one WMI helper at a time. The busy flag is cleared by an RAII
/// guard on every exit path, including a panic. A helper that outlives the
/// caller's deadline keeps the flag set and blocks new requests instead of
/// allowing an unbounded number of hung threads.
fn run_wmi<T: Send + 'static>(
    busy: &'static AtomicBool,
    current: &dyn Fn() -> bool,
    deadline: Duration,
    work: impl FnOnce(Arc<Cancel>) -> T + Send + 'static,
) -> Flight<T> {
    if !current() {
        return Flight::Revoked;
    }
    if busy
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Flight::Busy;
    }

    struct BusyGuard(&'static AtomicBool);
    impl Drop for BusyGuard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }

    let cancel = Arc::new(Cancel::new());
    let worker_cancel = Arc::clone(&cancel);
    let (sender, receiver) = sync_channel(1);
    let spawned = std::thread::Builder::new()
        .name("rchub-wmi-brightness".into())
        .spawn(move || {
            let _busy = BusyGuard(busy);
            let result = work(worker_cancel);
            let _ = sender.send(result);
        });
    let handle = match spawned {
        Ok(handle) => handle,
        Err(_) => {
            busy.store(false, Ordering::Release);
            return Flight::Failed;
        }
    };

    let started = Instant::now();
    loop {
        let remaining = deadline.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            cancel.cancel();
            return Flight::TimedOut;
        }
        match receiver.recv_timeout(remaining.min(WMI_POLL)) {
            Ok(value) => {
                let _ = handle.join();
                return Flight::Done(value);
            }
            Err(RecvTimeoutError::Timeout) => {
                if !current() {
                    cancel.cancel();
                    return Flight::Revoked;
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                let _ = handle.join();
                return Flight::Failed;
            }
        }
    }
}

struct NativeProviders;

impl Providers for NativeProviders {
    fn ddc_read(&self, current: &dyn Fn() -> bool) -> Result<Read, ()> {
        Ok(match ddc_brightness(None, current)? {
            Some(value) => Read::Supported(value),
            None => Read::Unsupported,
        })
    }

    fn ddc_write(&self, level: u32, current: &dyn Fn() -> bool) -> Result<u32, ()> {
        ddc_brightness(Some(level), current)?.ok_or(())
    }

    fn wmi_read(&self, current: &dyn Fn() -> bool) -> Result<Read, ()> {
        match run_wmi(&WMI_BUSY, current, WMI_DEADLINE, |cancel| {
            wmi::read(&cancel)
        }) {
            Flight::Done(Ok(wmi::Outcome::Brightness(value))) => Ok(Read::Supported(value)),
            Flight::Done(Ok(wmi::Outcome::NoTarget)) => Ok(Read::Unsupported),
            _ => Err(()),
        }
    }

    fn wmi_write(&self, level: u32, current: &dyn Fn() -> bool) -> Result<u32, ()> {
        match run_wmi(&WMI_BUSY, current, WMI_DEADLINE, move |cancel| {
            wmi::write(level, &cancel)
        }) {
            Flight::Done(Ok(wmi::Outcome::Brightness(value))) => Ok(value),
            _ => Err(()),
        }
    }
}

fn brightness_with(
    providers: &dyn Providers,
    set: Option<u32>,
    current: &dyn Fn() -> bool,
) -> Result<Value, ()> {
    if !current() {
        return Err(());
    }
    let (provider, observed) = match providers.ddc_read(current)? {
        Read::Supported(value) => (Provider::Ddc, value),
        Read::Unsupported => match providers.wmi_read(current)? {
            Read::Supported(value) => (Provider::Wmi, value),
            Read::Unsupported => {
                if !current() {
                    return Err(());
                }
                if set.is_some() {
                    return Err(());
                }
                return Ok(json!({"brightness":0,"available":false}));
            }
        },
    };
    let Some(level) = set else {
        return Ok(json!({"brightness":observed,"available":true}));
    };
    let actual = match provider {
        Provider::Ddc => providers.ddc_write(level, current)?,
        Provider::Wmi => providers.wmi_write(level, current)?,
    };
    if !current() {
        return Err(());
    }
    Ok(json!({"brightness":actual,"available":true}))
}

/// DDC/CI only. `Ok(None)` means the primary monitor does not support DDC/CI;
/// `Err` means revocation or a failed DDC write. A failed write is never
/// retried through another provider.
fn ddc_brightness(set: Option<u32>, current: &dyn Fn() -> bool) -> Result<Option<u32>, ()> {
    use windows_sys::Win32::Devices::Display::*;
    if !current() {
        return Err(());
    }
    let monitor = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
    let mut count = 0;
    if monitor.is_null()
        || unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(monitor, &mut count) } == 0
        || !(1..=16).contains(&count)
    {
        return Ok(None);
    }
    struct Held(Vec<PHYSICAL_MONITOR>);
    impl Drop for Held {
        fn drop(&mut self) {
            for monitor in &self.0 {
                if !monitor.hPhysicalMonitor.is_null() {
                    unsafe { DestroyPhysicalMonitor(monitor.hPhysicalMonitor) };
                }
            }
        }
    }
    let mut monitors = Held((0..count).map(|_| unsafe { std::mem::zeroed() }).collect());
    if !current() {
        return Err(());
    }
    if unsafe { GetPhysicalMonitorsFromHMONITOR(monitor, count, monitors.0.as_mut_ptr()) } == 0 {
        return Ok(None);
    }
    for monitor in &monitors.0 {
        if !current() {
            return Err(());
        }
        let (mut min, mut observed, mut max) = (0, 0, 0);
        if unsafe {
            GetMonitorBrightness(monitor.hPhysicalMonitor, &mut min, &mut observed, &mut max)
        } == 0
            || min >= max
            || observed < min
            || observed > max
        {
            continue;
        }
        if let Some(n) = set {
            let raw = min as u64 + (max - min) as u64 * n as u64 / 100;
            if !current() {
                return Err(());
            }
            if unsafe { SetMonitorBrightness(monitor.hPhysicalMonitor, raw as u32) } == 0 {
                return Err(());
            }
            if !current() {
                return Err(());
            }
            if unsafe {
                GetMonitorBrightness(monitor.hPhysicalMonitor, &mut min, &mut observed, &mut max)
            } == 0
                || min >= max
                || observed < min
                || observed > max
            {
                return Err(());
            }
        }
        if !current() {
            return Err(());
        }
        let value = (observed - min) as u64 * 100 / (max - min) as u64;
        return Ok(Some(value as u32));
    }
    Ok(None)
}

fn capture(current: &dyn Fn() -> bool) -> Result<Value, ()> {
    if !current() {
        return Err(());
    }
    let monitor = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
    if monitor.is_null() {
        return Err(());
    }
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return Err(());
    }
    let rect = info.rcMonitor;
    let sw = rect.right.checked_sub(rect.left).ok_or(())?;
    let sh = rect.bottom.checked_sub(rect.top).ok_or(())?;
    if !(1..=32768).contains(&sw) || !(1..=32768).contains(&sh) {
        return Err(());
    }
    let scale = (1920.0_f64 / sw as f64).min(1080.0 / sh as f64).min(1.0);
    let w = (sw as f64 * scale).round().max(1.0) as u32;
    let h = (sh as f64 * scale).round().max(1.0) as u32;
    struct Desktop(HDC);
    impl Drop for Desktop {
        fn drop(&mut self) {
            unsafe { ReleaseDC(std::ptr::null_mut(), self.0) };
        }
    }
    struct Dc(HDC);
    impl Drop for Dc {
        fn drop(&mut self) {
            unsafe { DeleteDC(self.0) };
        }
    }
    struct Bitmap(HBITMAP);
    impl Drop for Bitmap {
        fn drop(&mut self) {
            unsafe { DeleteObject(self.0) };
        }
    }
    struct Selected {
        dc: HDC,
        old: HGDIOBJ,
    }
    impl Drop for Selected {
        fn drop(&mut self) {
            unsafe { SelectObject(self.dc, self.old) };
        }
    }
    if !current() {
        return Err(());
    }
    let source = Desktop(unsafe { GetDC(std::ptr::null_mut()) });
    if source.0.is_null() {
        return Err(());
    }
    let dc = Dc(unsafe { CreateCompatibleDC(source.0) });
    if dc.0.is_null() {
        return Err(());
    }
    let mut bitmap_info: BITMAPINFO = unsafe { std::mem::zeroed() };
    bitmap_info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bitmap_info.bmiHeader.biWidth = w as i32;
    bitmap_info.bmiHeader.biHeight = -(h as i32);
    bitmap_info.bmiHeader.biPlanes = 1;
    bitmap_info.bmiHeader.biBitCount = 32;
    bitmap_info.bmiHeader.biCompression = BI_RGB;
    let mut bits = std::ptr::null_mut();
    let bitmap = Bitmap(unsafe {
        CreateDIBSection(
            source.0,
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        )
    });
    if bitmap.0.is_null() || bits.is_null() {
        return Err(());
    }
    let old = unsafe { SelectObject(dc.0, bitmap.0) };
    if old.is_null() || old as isize == -1 {
        return Err(());
    }
    let _selected = Selected { dc: dc.0, old };
    unsafe { SetStretchBltMode(dc.0, HALFTONE) };
    if !current() {
        return Err(());
    }
    if unsafe {
        StretchBlt(
            dc.0,
            0,
            0,
            w as i32,
            h as i32,
            source.0,
            rect.left,
            rect.top,
            sw,
            sh,
            SRCCOPY | CAPTUREBLT,
        )
    } == 0
    {
        return Err(());
    }
    unsafe { GdiFlush() };
    if !current() {
        return Err(());
    }
    let raw = unsafe { std::slice::from_raw_parts(bits as *const u8, w as usize * h as usize * 4) };
    let mut rgb = Vec::with_capacity(w as usize * h as usize * 3);
    for b in raw.chunks_exact(4) {
        rgb.extend_from_slice(&[b[2], b[1], b[0]])
    }
    screenshot_image(w, h, rgb, current)
}
fn screenshot_image(w: u32, h: u32, rgb: Vec<u8>, current: &dyn Fn() -> bool) -> Result<Value, ()> {
    use base64::Engine;
    use image::ImageEncoder;
    if w == 0 || h == 0 || w > 1920 || h > 1080 {
        return Err(());
    }
    let mut picture = image::RgbImage::from_raw(w, h, rgb).ok_or(())?;
    for _ in 0..8 {
        if !current() {
            return Err(());
        }
        let mut jpeg = vec![];
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 80)
            .write_image(
                picture.as_raw(),
                picture.width(),
                picture.height(),
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|_| ())?;
        let encoded = format!(
            "data:image/jpeg;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(jpeg)
        );
        if encoded.len() <= 800000 {
            if !current() {
                return Err(());
            }
            return Ok(json!({"image":encoded,"width":picture.width(),"height":picture.height()}));
        }
        let w = (picture.width() * 9 / 10).max(1);
        let h = (picture.height() * 9 / 10).max(1);
        picture = image::imageops::resize(&picture, w, h, image::imageops::FilterType::Triangle);
    }
    Err(())
}
