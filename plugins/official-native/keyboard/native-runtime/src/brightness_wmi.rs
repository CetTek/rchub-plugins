//! WMI fallback for the internal (laptop) panel.
//!
//! DDC/CI is still preferred. Some internal panels do not expose DDC/CI but do
//! expose `WmiMonitorBrightnessMethods`. WMI is only used when the active
//! display topology has exactly one internal target and that target is the
//! primary source, and when both `WmiMonitorBrightness` and the internal
//! `WmiMonitorConnectionParams` instance agree on the same instance name.
//! Anything ambiguous fails closed: no write is attempted.
use super::Cancel;
use std::time::{Duration, Instant};
use windows::core::{w, BSTR, PCWSTR, VARIANT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoSetProxyBlanket, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_MULTITHREADED, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Rpc::{RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE};
use windows::Win32::System::Wmi::{
    IWbemClassObject, IWbemLocator, IWbemServices, WbemLocator, WBEM_E_NOT_FOUND,
    WBEM_FLAG_FORWARD_ONLY, WBEM_GENERIC_FLAG_TYPE, WBEM_S_TIMEDOUT,
};
use windows_sys::Win32::Devices::Display::{
    GetDisplayConfigBufferSizes, QueryDisplayConfig, DISPLAYCONFIG_MODE_INFO,
    DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_LVDS, DISPLAYCONFIG_PATH_INFO, QDC_ONLY_ACTIVE_PATHS,
};

/// WMI result. `NoTarget` means the topology/instances are not an unambiguous
/// single internal panel; callers must not try another provider after a write.
pub(super) enum Outcome {
    Brightness(u32),
    NoTarget,
}

struct Com;

impl Com {
    fn init() -> Result<Self, ()> {
        // A fresh helper thread is expected to enter the MTA cleanly. If COM
        // was already initialized in an incompatible mode, fail instead of
        // guessing or touching another thread's apartment.
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
            .ok()
            .map_err(|_| ())?;
        Ok(Self)
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

pub(super) fn read(cancel: &Cancel) -> Result<Outcome, ()> {
    if cancel.is_cancelled() {
        return Err(());
    }
    if !primary_is_single_internal() {
        return Ok(Outcome::NoTarget);
    }
    let _com = Com::init()?;
    let services = connect()?;
    if cancel.is_cancelled() {
        return Err(());
    }
    let Some(target) = target(&services, cancel)? else {
        return Ok(Outcome::NoTarget);
    };
    if cancel.is_cancelled() {
        return Err(());
    }
    Ok(Outcome::Brightness(target.value))
}

pub(super) fn write(level: u32, cancel: &Cancel) -> Result<Outcome, ()> {
    if cancel.is_cancelled() {
        return Err(());
    }
    // A write is denied unless the topology is unambiguously one internal
    // primary target. Denials are errors, never a silent provider switch.
    if !primary_is_single_internal() {
        return Err(());
    }
    let _com = Com::init()?;
    let services = connect()?;
    if cancel.is_cancelled() {
        return Err(());
    }
    let Some(expected) = target(&services, cancel)? else {
        return Err(());
    };
    let path = method_path(&services, &expected, cancel)?;

    // Long COM preparation (GetObject/GetMethod/SpawnInstance/Put) happens
    // inside `prepare_write`, before the side-effect gate. Cancellation during
    // any preparation stage aborts without starting WmiSetBrightness.
    gated_write(
        cancel,
        |cancel| prepare_write(&services, &path, level, cancel),
        |cancel| revalidate_target(&services, cancel, &expected),
        |prepared| unsafe { execute_write(&services, prepared) },
    )?;

    // Poll only observations, never reissue the write. The outer WMI deadline
    // and cancellation still apply if a provider blocks during an observation.
    let started = Instant::now();
    settle_readback(
        cancel,
        &expected.instance_name,
        level,
        primary_is_single_internal,
        || target(&services, cancel),
        || started.elapsed(),
        std::thread::sleep,
    )
    .map(Outcome::Brightness)
}

const READBACK_WINDOW: Duration = Duration::from_millis(1500);
const READBACK_POLL: Duration = Duration::from_millis(50);

/// Native completion is necessary but not enough: acknowledge a fresh matching
/// observation only while the same internal-primary topology remains valid.
fn settle_readback(
    cancel: &Cancel,
    expected_name: &str,
    level: u32,
    mut topology_valid: impl FnMut() -> bool,
    mut observe: impl FnMut() -> Result<Option<Target>, ()>,
    mut elapsed: impl FnMut() -> Duration,
    mut wait: impl FnMut(Duration),
) -> Result<u32, ()> {
    loop {
        if cancel.is_cancelled() || elapsed() > READBACK_WINDOW {
            return Err(());
        }
        if !topology_valid() || cancel.is_cancelled() || elapsed() > READBACK_WINDOW {
            return Err(());
        }
        let observed = observe()?.ok_or(())?;
        if !topology_valid()
            || cancel.is_cancelled()
            || elapsed() > READBACK_WINDOW
            || observed.instance_name != expected_name
            || observed.value > 100
        {
            return Err(());
        }
        if observed.value.abs_diff(level) <= 1 {
            return Ok(observed.value);
        }
        let remaining = READBACK_WINDOW.saturating_sub(elapsed());
        if remaining.is_zero() {
            return Err(());
        }
        wait(remaining.min(READBACK_POLL));
    }
}

/// Runs preparation, then the last fail-closed revalidation, then the
/// side-effect gate, and only then the native call. `execute` starts with the
/// WMI `ExecMethod`; the gate is the last check before it. A revocation that
/// lands between the gate and the syscall is an unavoidable tiny race; once
/// the gate passes, the native call is allowed to complete and a later
/// revocation is reported as failure/unknown, never as success.
fn gated_write<P, R>(
    cancel: &Cancel,
    prepare: impl FnOnce(&Cancel) -> Result<P, ()>,
    revalidate: impl FnOnce(&Cancel) -> Result<(), ()>,
    execute: impl FnOnce(&P) -> Result<R, ()>,
) -> Result<R, ()> {
    if cancel.is_cancelled() {
        return Err(());
    }
    let prepared = prepare(cancel)?;
    if cancel.is_cancelled() {
        return Err(());
    }
    revalidate(cancel)?;
    if cancel.is_cancelled() {
        return Err(());
    }
    if !cancel.begin_side_effect() {
        return Err(());
    }
    let result = execute(&prepared)?;
    if cancel.is_cancelled() {
        return Err(());
    }
    Ok(result)
}

/// Minimal fail-closed revalidation before the gate: the primary topology must
/// still be one internal target, and the WMI instance must still name the same
/// panel that was discovered and prepared.
fn revalidate_target(
    services: &IWbemServices,
    cancel: &Cancel,
    expected: &Target,
) -> Result<(), ()> {
    if cancel.is_cancelled() {
        return Err(());
    }
    if !primary_is_single_internal() {
        return Err(());
    }
    let Some(current) = target(services, cancel)? else {
        return Err(());
    };
    if current.instance_name != expected.instance_name {
        return Err(());
    }
    Ok(())
}

/// Resolves the single method instance and requires it to name the same panel
/// as the brightness instance. Ambiguity is a write error.
fn method_path(services: &IWbemServices, expected: &Target, cancel: &Cancel) -> Result<BSTR, ()> {
    if cancel.is_cancelled() {
        return Err(());
    }
    let methods = query(
        services,
        "SELECT * FROM WmiMonitorBrightnessMethods",
        cancel,
    )?;
    if cancel.is_cancelled() {
        return Err(());
    }
    if methods.len() != 1 {
        return Err(());
    }
    let method_name = unsafe { bstr_prop(&methods[0], w!("InstanceName")) }?.to_string();
    if method_name != expected.instance_name {
        return Err(());
    }
    let path = unsafe { bstr_prop(&methods[0], w!("__PATH")) }?;
    if cancel.is_cancelled() {
        return Err(());
    }
    Ok(path)
}

struct Target {
    value: u32,
    instance_name: String,
}

fn connect() -> Result<IWbemServices, ()> {
    let locator: IWbemLocator =
        unsafe { CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER) }.map_err(|_| ())?;
    let namespace = BSTR::from("ROOT\\WMI");
    let empty = BSTR::from("");
    let services =
        unsafe { locator.ConnectServer(&namespace, &empty, &empty, &empty, 0, &empty, None) }
            .map_err(|_| ())?;
    unsafe {
        CoSetProxyBlanket(
            &services,
            RPC_C_AUTHN_WINNT,
            RPC_C_AUTHZ_NONE,
            PCWSTR::null(),
            RPC_C_AUTHN_LEVEL_CALL,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            None,
            EOAC_NONE,
        )
    }
    .map_err(|_| ())?;
    Ok(services)
}

fn target(services: &IWbemServices, cancel: &Cancel) -> Result<Option<Target>, ()> {
    let brightness = query(
        services,
        "SELECT InstanceName, CurrentBrightness FROM WmiMonitorBrightness",
        cancel,
    )?;
    if cancel.is_cancelled() {
        return Err(());
    }
    let connections = query(
        services,
        "SELECT InstanceName, VideoOutputTechnology FROM WmiMonitorConnectionParams",
        cancel,
    )?;
    if cancel.is_cancelled() {
        return Err(());
    }
    let brightness = brightness
        .iter()
        .map(|object| unsafe {
            Ok((
                bstr_prop(object, w!("InstanceName"))?.to_string(),
                u32_prop(object, w!("CurrentBrightness"))?,
            ))
        })
        .collect::<Result<Vec<_>, ()>>()?;
    let connections = connections
        .iter()
        .map(|object| unsafe {
            Ok((
                bstr_prop(object, w!("InstanceName"))?.to_string(),
                i32_prop(object, w!("VideoOutputTechnology"))? as u32,
            ))
        })
        .collect::<Result<Vec<_>, ()>>()?;
    let Some(value) = select_internal_value(&brightness, &connections) else {
        return Ok(None);
    };
    // CurrentBrightness is a percentage. An out-of-range value means the WMI
    // data cannot be trusted; fail rather than clamp or mirror a bad number.
    let value = checked_percent(value)?;
    Ok(Some(Target {
        value,
        instance_name: brightness[0].0.clone(),
    }))
}

fn query(
    services: &IWbemServices,
    wql: &str,
    cancel: &Cancel,
) -> Result<Vec<IWbemClassObject>, ()> {
    if cancel.is_cancelled() {
        return Err(());
    }
    let enumerator = unsafe {
        services.ExecQuery(
            &BSTR::from("WQL"),
            &BSTR::from(wql),
            WBEM_FLAG_FORWARD_ONLY,
            None,
        )
    }
    .map_err(|_| ())?;
    let mut results = Vec::new();
    loop {
        if cancel.is_cancelled() {
            return Err(());
        }
        let mut batch: [Option<IWbemClassObject>; 8] = std::array::from_fn(|_| None);
        let mut returned = 0u32;
        let result = unsafe { enumerator.Next(2000, &mut batch, &mut returned) };
        if result.is_err() || result.0 == WBEM_S_TIMEDOUT.0 {
            return Err(());
        }
        if returned == 0 {
            break;
        }
        for object in batch.into_iter().take(returned as usize) {
            if let Some(object) = object {
                results.push(object);
            }
        }
        // These monitor classes have a handful of instances at most. More than
        // one page is already enough to prove ambiguity for our matching rules.
        if results.len() > 8 {
            break;
        }
    }
    Ok(results)
}

unsafe fn bstr_prop(object: &IWbemClassObject, name: PCWSTR) -> Result<BSTR, ()> {
    let mut value = VARIANT::default();
    object
        .Get(name, 0, &mut value, None, None)
        .map_err(|_| ())?;
    BSTR::try_from(&value).map_err(|_| ())
}

unsafe fn u32_prop(object: &IWbemClassObject, name: PCWSTR) -> Result<u32, ()> {
    let mut value = VARIANT::default();
    let mut cim_type = -1;
    object
        .Get(name, 0, &mut value, Some(&mut cim_type), None)
        .map_err(|_| ())?;
    brightness_value(cim_type, &value)
}

fn brightness_value(cim_type: i32, value: &VARIANT) -> Result<u32, ()> {
    let raw = unsafe { &value.as_raw().Anonymous.Anonymous };
    // CurrentBrightness is CIM_UINT8 / VT_UI1. Do not coerce missing/null/false
    // to zero and accidentally acknowledge an unobserved write to 0%.
    if cim_type != 17 || raw.vt != 17 {
        return Err(());
    }
    checked_percent(unsafe { u32::from(raw.Anonymous.bVal) })
}

unsafe fn i32_prop(object: &IWbemClassObject, name: PCWSTR) -> Result<i32, ()> {
    let mut value = VARIANT::default();
    object
        .Get(name, 0, &mut value, None, None)
        .map_err(|_| ())?;
    i32::try_from(&value).map_err(|_| ())
}

unsafe fn cim_type(object: &IWbemClassObject, name: PCWSTR) -> Result<i32, ()> {
    let mut value = VARIANT::default();
    let mut cim_type = -1i32;
    object
        .Get(name, 0, &mut value, Some(&mut cim_type), None)
        .map_err(|_| ())?;
    if cim_type < 0 {
        return Err(());
    }
    Ok(cim_type)
}

struct PreparedWrite {
    path: BSTR,
    method: BSTR,
    parameters: IWbemClassObject,
    return_contract: ReturnContract,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReturnContract {
    Void,
    Uint32,
}

/// Learn documents uint32/zero success, but the local ROOT/WMI class declares
/// void (GetMethod's output signature is absent). Use the actual COM signature,
/// not Get-CimClass's disagreeing Boolean metadata. Boolean success
/// semantics are undocumented here; reject those declarations before writing.
/// References: Microsoft Learn's WmiSetBrightness and
/// IWbemClassObject::GetMethod / IWbemServices::ExecMethod contracts.
fn return_contract(cim_type: Option<i32>) -> Result<ReturnContract, ()> {
    match cim_type {
        None => Ok(ReturnContract::Void),
        Some(19) => Ok(ReturnContract::Uint32),
        _ => Err(()),
    }
}

unsafe fn return_property(object: &IWbemClassObject) -> Result<Option<(i32, VARIANT)>, ()> {
    let mut value = VARIANT::default();
    let mut cim_type = -1;
    match object.Get(w!("ReturnValue"), 0, &mut value, Some(&mut cim_type), None) {
        Ok(()) => Ok(Some((cim_type, value))),
        Err(error) if error.code().0 == WBEM_E_NOT_FOUND.0 => Ok(None),
        Err(_) => Err(()),
    }
}

fn validate_return(contract: ReturnContract, output: Option<(i32, &VARIANT)>) -> Result<(), ()> {
    match (contract, output) {
        (ReturnContract::Void, None) => Ok(()),
        (ReturnContract::Uint32, Some((19, value))) => {
            // WMI represents CIM_UINT32 as VT_I4; also accept VT_UI4. Check the
            // tag first: VariantToInt32 otherwise coerces null/false/text to 0.
            let raw = unsafe { &value.as_raw().Anonymous.Anonymous };
            let zero = unsafe {
                match raw.vt {
                    3 => raw.Anonymous.lVal == 0,   // VT_I4
                    19 => raw.Anonymous.ulVal == 0, // VT_UI4
                    _ => false,
                }
            };
            if zero {
                Ok(())
            } else {
                Err(())
            }
        }
        _ => Err(()),
    }
}

/// All potentially blocking COM preparation, with cancellation checks between
/// stages. This never calls `ExecMethod`; the caller gates the actual write
/// immediately before `execute_write`.
fn prepare_write(
    services: &IWbemServices,
    path: &BSTR,
    level: u32,
    cancel: &Cancel,
) -> Result<PreparedWrite, ()> {
    if cancel.is_cancelled() {
        return Err(());
    }
    if level > 100 {
        return Err(());
    }

    // GetMethod is only valid on a class object, so fetch the class
    // definition; `path` remains the instance path for ExecMethod.
    let class = BSTR::from("WmiMonitorBrightnessMethods");
    let mut definition: Option<IWbemClassObject> = None;
    unsafe {
        services.GetObject(
            &class,
            WBEM_GENERIC_FLAG_TYPE(0),
            None,
            Some(&mut definition),
            None,
        )
    }
    .map_err(|_| ())?;
    if cancel.is_cancelled() {
        return Err(());
    }
    let definition = definition.ok_or(())?;

    let mut signature: Option<IWbemClassObject> = None;
    let mut output_signature: Option<IWbemClassObject> = None;
    unsafe {
        definition.GetMethod(
            w!("WmiSetBrightness"),
            0,
            &mut signature,
            &mut output_signature,
        )
    }
    .map_err(|_| ())?;
    if cancel.is_cancelled() {
        return Err(());
    }
    let signature = signature.ok_or(())?;
    let return_contract = match output_signature.as_ref() {
        None => return_contract(None)?,
        Some(signature) => {
            let (kind, _) = unsafe { return_property(signature) }?.ok_or(())?;
            return_contract(Some(kind))?
        }
    };

    // The provider contract must match the VARIANT types below; fail closed
    // instead of sending a mismatched parameter to a different provider.
    // WMI maps CIM uint32 to VT_I4 and CIM uint8 to VT_UI1.
    if unsafe { cim_type(&signature, w!("Timeout"))? } != 19
        || unsafe { cim_type(&signature, w!("Brightness"))? } != 17
    {
        return Err(());
    }
    if cancel.is_cancelled() {
        return Err(());
    }

    let parameters = unsafe { signature.SpawnInstance(0) }.map_err(|_| ())?;
    if cancel.is_cancelled() {
        return Err(());
    }

    let timeout = VARIANT::from(1i32);
    unsafe { parameters.Put(w!("Timeout"), 0, &timeout, 0) }.map_err(|_| ())?;
    if cancel.is_cancelled() {
        return Err(());
    }

    let brightness = VARIANT::from(level as u8);
    unsafe { parameters.Put(w!("Brightness"), 0, &brightness, 0) }.map_err(|_| ())?;
    if cancel.is_cancelled() {
        return Err(());
    }

    Ok(PreparedWrite {
        path: path.clone(),
        method: BSTR::from("WmiSetBrightness"),
        parameters,
        return_contract,
    })
}

/// The actual native side effect. The caller must have passed the
/// `begin_side_effect` gate immediately before this call.
unsafe fn execute_write(services: &IWbemServices, prepared: &PreparedWrite) -> Result<(), ()> {
    let mut output: Option<IWbemClassObject> = None;
    services
        .ExecMethod(
            &prepared.path,
            &prepared.method,
            WBEM_GENERIC_FLAG_TYPE(0),
            None,
            &prepared.parameters,
            Some(&mut output),
            None,
        )
        .map_err(|_| ())?;
    let returned = output
        .as_ref()
        .map(|object| return_property(object))
        .transpose()?
        .flatten();
    validate_return(
        prepared.return_contract,
        returned.as_ref().map(|(kind, value)| (*kind, value)),
    )
}

fn checked_percent(value: u32) -> Result<u32, ()> {
    if value <= 100 {
        Ok(value)
    } else {
        Err(())
    }
}

fn is_internal(technology: u32) -> bool {
    technology == DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL as u32
        || technology == DISPLAYCONFIG_OUTPUT_TECHNOLOGY_LVDS as u32
}

/// Requires exactly one brightness instance, exactly one internal connection
/// instance, and identical instance names. External monitor entries are
/// ignored, but a second internal entry makes the match ambiguous.
fn select_internal_value(
    brightness: &[(String, u32)],
    connections: &[(String, u32)],
) -> Option<u32> {
    if brightness.len() != 1 {
        return None;
    }
    let internal: Vec<&(String, u32)> = connections
        .iter()
        .filter(|(_, technology)| is_internal(*technology))
        .collect();
    if internal.len() != 1 {
        return None;
    }
    if internal[0].0 != brightness[0].0 {
        return None;
    }
    Some(brightness[0].1)
}

fn primary_is_single_internal() -> bool {
    let mut path_count = 0u32;
    let mut mode_count = 0u32;
    if unsafe {
        GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
    } != 0
        || path_count == 0
        || path_count > 64
        || mode_count == 0
        || mode_count > 256
    {
        return false;
    }
    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
    if unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            std::ptr::null_mut(),
        )
    } != 0
    {
        return false;
    }

    let mut summaries = Vec::with_capacity(path_count as usize);
    for path in paths.iter().take(path_count as usize) {
        let index = unsafe { path.sourceInfo.Anonymous.modeInfoIdx };
        if index == u32::MAX || index as usize >= mode_count as usize {
            return false;
        }
        let mode = &modes[index as usize];
        if mode.infoType != DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
            return false;
        }
        let position = unsafe { mode.Anonymous.sourceMode.position };
        summaries.push(((position.x, position.y), path.targetInfo.outputTechnology));
    }
    primary_is_single_internal_paths(&summaries)
}

/// A path at source position (0,0) is the Windows primary source. Clone mode
/// produces more than one path at that position, which is rejected.
fn primary_is_single_internal_paths(paths: &[((i32, i32), i32)]) -> bool {
    let mut primary = 0usize;
    let mut internal = 0usize;
    let mut primary_is_internal = false;
    for (position, technology) in paths {
        let technology_internal = *technology == DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL
            || *technology == DISPLAYCONFIG_OUTPUT_TECHNOLOGY_LVDS;
        if *position == (0, 0) {
            primary += 1;
            primary_is_internal = technology_internal;
        }
        if technology_internal {
            internal += 1;
        }
    }
    primary == 1 && internal == 1 && primary_is_internal
}
