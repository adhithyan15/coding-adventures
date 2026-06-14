use crate::{
    BleGattIo, BluetoothOpenError, BluetoothTransportError, MacosCoreBluetoothBleConnector,
    MacosCoreBluetoothBleOpenRequest,
};
use objc_bridge::{
    class_addIvar, class_addMethod, msg, msg_ptr, msg_usize, objc_allocateClassPair, objc_getClass,
    objc_msgSend, objc_registerClassPair, object_getInstanceVariable, object_setInstanceVariable,
    release, retain, sel, CFRelease, CFStringGetCString, ClassPtr, Id, Sel,
    K_CF_STRING_ENCODING_UTF8, NIL,
};
use std::collections::VecDeque;
use std::ffi::{c_void, CString};
use std::ptr;
use std::time::{Duration, Instant};

#[link(name = "CoreBluetooth", kind = "framework")]
extern "C" {}

#[link(name = "Foundation", kind = "framework")]
extern "C" {}

const CB_MANAGER_STATE_POWERED_ON: usize = 5;
const CB_CHARACTERISTIC_WRITE_WITH_RESPONSE: usize = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacosCoreBluetoothTimeouts {
    pub power_on_ms: u64,
    pub connect_ms: u64,
    pub io_ms: u64,
    pub poll_ms: u64,
}

impl MacosCoreBluetoothTimeouts {
    pub const DEFAULT: Self = Self {
        power_on_ms: 5_000,
        connect_ms: 15_000,
        io_ms: 5_000,
        poll_ms: 10,
    };
}

impl Default for MacosCoreBluetoothTimeouts {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MacosCoreBluetoothRuntimeBleConnector {
    timeouts: MacosCoreBluetoothTimeouts,
}

impl MacosCoreBluetoothRuntimeBleConnector {
    pub const fn new() -> Self {
        Self {
            timeouts: MacosCoreBluetoothTimeouts::DEFAULT,
        }
    }

    pub const fn with_timeouts(timeouts: MacosCoreBluetoothTimeouts) -> Self {
        Self { timeouts }
    }

    pub const fn timeouts(&self) -> MacosCoreBluetoothTimeouts {
        self.timeouts
    }
}

impl MacosCoreBluetoothBleConnector for MacosCoreBluetoothRuntimeBleConnector {
    type BleGattLink = MacosCoreBluetoothBleLink;

    fn open_ble_gatt(
        &mut self,
        request: &MacosCoreBluetoothBleOpenRequest,
    ) -> Result<Self::BleGattLink, BluetoothOpenError> {
        MacosCoreBluetoothBleLink::open(request.clone(), self.timeouts)
    }
}

pub struct MacosCoreBluetoothBleLink {
    delegate: Id,
    state: Box<CoreBluetoothState>,
}

impl MacosCoreBluetoothBleLink {
    fn open(
        request: MacosCoreBluetoothBleOpenRequest,
        timeouts: MacosCoreBluetoothTimeouts,
    ) -> Result<Self, BluetoothOpenError> {
        let mut state = Box::new(unsafe { CoreBluetoothState::new(request, timeouts)? });
        let delegate = match unsafe { create_delegate(state.as_mut() as *mut CoreBluetoothState) } {
            Ok(delegate) => delegate,
            Err(error) => {
                unsafe { state.release_core_bluetooth_objects() };
                return Err(error);
            }
        };
        let manager = match unsafe { create_central_manager(delegate) } {
            Ok(manager) => manager,
            Err(error) => {
                unsafe {
                    release(delegate);
                    state.release_core_bluetooth_objects();
                }
                return Err(error);
            }
        };
        state.manager = manager;

        let mut link = Self { delegate, state };

        link.state.wait_until(
            timeouts.power_on_ms,
            "timed out waiting for CoreBluetooth to power on",
            |state| state.powered_on,
        )?;

        unsafe {
            msg!(
                link.state.manager,
                "scanForPeripheralsWithServices:options:",
                link.state.service_uuid_array,
                NIL
            );
            link.state.scanning = true;
        }

        link.state.wait_until(
            timeouts.connect_ms,
            "timed out waiting for CoreBluetooth BLE GATT connection",
            |state| state.ready,
        )?;

        Ok(link)
    }
}

impl BleGattIo for MacosCoreBluetoothBleLink {
    fn write_characteristic(
        &mut self,
        characteristic_uuid: &str,
        bytes: &[u8],
    ) -> Result<(), BluetoothTransportError> {
        if !characteristic_uuid.eq_ignore_ascii_case(&self.state.request.write_characteristic_uuid)
        {
            return Err(BluetoothTransportError::Link);
        }

        if self.state.peripheral == NIL || self.state.write_characteristic == NIL {
            return Err(BluetoothTransportError::Link);
        }

        unsafe {
            let data = ns_data_with_bytes(bytes)?;
            self.state.pending_write = true;
            msg!(
                self.state.peripheral,
                "writeValue:forCharacteristic:type:",
                data,
                self.state.write_characteristic,
                CB_CHARACTERISTIC_WRITE_WITH_RESPONSE
            );
        }

        self.state.wait_until_io(
            "timed out waiting for CoreBluetooth characteristic write",
            |state| !state.pending_write,
        )
    }

    fn read_notification(
        &mut self,
        characteristic_uuid: &str,
        out: &mut [u8],
    ) -> Result<usize, BluetoothTransportError> {
        if !characteristic_uuid.eq_ignore_ascii_case(&self.state.request.notify_characteristic_uuid)
        {
            return Err(BluetoothTransportError::Link);
        }

        self.state.wait_until_io(
            "timed out waiting for CoreBluetooth notification",
            |state| !state.notifications.is_empty(),
        )?;

        let Some(notification) = self.state.notifications.pop_front() else {
            return Err(BluetoothTransportError::Link);
        };

        if notification.len() > out.len() {
            return Err(BluetoothTransportError::FrameTooLarge);
        }

        out[..notification.len()].copy_from_slice(&notification);
        Ok(notification.len())
    }
}

impl Drop for MacosCoreBluetoothBleLink {
    fn drop(&mut self) {
        unsafe {
            let name = c_string("_state");
            object_setInstanceVariable(self.delegate, name.as_ptr(), ptr::null_mut());
            self.state.release_core_bluetooth_objects();
            release(self.delegate);
        }
    }
}

struct CoreBluetoothState {
    request: MacosCoreBluetoothBleOpenRequest,
    timeouts: MacosCoreBluetoothTimeouts,
    manager: Id,
    peripheral: Id,
    service: Id,
    write_characteristic: Id,
    notify_characteristic: Id,
    service_uuid: Id,
    write_uuid: Id,
    notify_uuid: Id,
    service_uuid_array: Id,
    characteristic_uuid_array: Id,
    powered_on: bool,
    scanning: bool,
    ready: bool,
    pending_write: bool,
    notifications: VecDeque<Vec<u8>>,
    error_message: Option<String>,
}

impl CoreBluetoothState {
    unsafe fn new(
        request: MacosCoreBluetoothBleOpenRequest,
        timeouts: MacosCoreBluetoothTimeouts,
    ) -> Result<Self, BluetoothOpenError> {
        let service_uuid = core_bluetooth_uuid(&request.service_uuid)?;
        let write_uuid = core_bluetooth_uuid(&request.write_characteristic_uuid)?;
        let notify_uuid = core_bluetooth_uuid(&request.notify_characteristic_uuid)?;
        let service_uuid_array = ns_array_with_objects(&[service_uuid])?;
        let characteristic_uuid_array = ns_array_with_objects(&[write_uuid, notify_uuid])?;

        Ok(Self {
            request,
            timeouts,
            manager: NIL,
            peripheral: NIL,
            service: NIL,
            write_characteristic: NIL,
            notify_characteristic: NIL,
            service_uuid,
            write_uuid,
            notify_uuid,
            service_uuid_array,
            characteristic_uuid_array,
            powered_on: false,
            scanning: false,
            ready: false,
            pending_write: false,
            notifications: VecDeque::new(),
            error_message: None,
        })
    }

    fn wait_until<F>(
        &mut self,
        timeout_ms: u64,
        timeout_message: &str,
        ready: F,
    ) -> Result<(), BluetoothOpenError>
    where
        F: Fn(&Self) -> bool,
    {
        let deadline = Deadline::new(timeout_ms);
        while !ready(self) {
            if let Some(message) = self.error_message.take() {
                return Err(BluetoothOpenError::Backend { message });
            }
            if deadline.expired() {
                return Err(BluetoothOpenError::Backend {
                    message: format!(
                        "{timeout_message} for {} service {}",
                        self.request.device, self.request.service_uuid
                    ),
                });
            }
            unsafe { pump_run_loop_once(self.timeouts.poll_ms)? };
        }
        Ok(())
    }

    fn wait_until_io<F>(
        &mut self,
        timeout_message: &str,
        ready: F,
    ) -> Result<(), BluetoothTransportError>
    where
        F: Fn(&Self) -> bool,
    {
        let deadline = Deadline::new(self.timeouts.io_ms);
        while !ready(self) {
            if let Some(message) = self.error_message.take() {
                let _ = message;
                return Err(BluetoothTransportError::Link);
            }
            if deadline.expired() {
                let _ = timeout_message;
                return Err(BluetoothTransportError::Link);
            }
            unsafe {
                pump_run_loop_once(self.timeouts.poll_ms)
                    .map_err(|_error| BluetoothTransportError::Link)?;
            }
        }
        Ok(())
    }

    fn set_error(&mut self, message: impl Into<String>) {
        if self.error_message.is_none() {
            self.error_message = Some(message.into());
        }
    }

    unsafe fn release_core_bluetooth_objects(&mut self) {
        if self.manager != NIL {
            if self.scanning {
                msg!(self.manager, "stopScan");
            }
            if self.peripheral != NIL {
                msg!(self.peripheral, "setDelegate:", NIL);
                msg!(self.manager, "cancelPeripheralConnection:", self.peripheral);
            }
            release(self.manager);
            self.manager = NIL;
        }
        release_if_present(&mut self.peripheral);
        release_if_present(&mut self.service);
        release_if_present(&mut self.write_characteristic);
        release_if_present(&mut self.notify_characteristic);
        release_if_present(&mut self.service_uuid);
        release_if_present(&mut self.write_uuid);
        release_if_present(&mut self.notify_uuid);
        release_if_present(&mut self.service_uuid_array);
        release_if_present(&mut self.characteristic_uuid_array);
    }
}

struct Deadline {
    started: Instant,
    timeout: Duration,
}

impl Deadline {
    fn new(timeout_ms: u64) -> Self {
        Self {
            started: Instant::now(),
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    fn expired(&self) -> bool {
        self.started.elapsed() >= self.timeout
    }
}

unsafe fn create_central_manager(delegate: Id) -> Result<Id, BluetoothOpenError> {
    let cls = objc_class("CBCentralManager")? as Id;
    Ok(msg!(
        msg!(cls, "alloc"),
        "initWithDelegate:queue:options:",
        delegate,
        NIL,
        NIL
    ))
}

unsafe fn create_delegate(state: *mut CoreBluetoothState) -> Result<Id, BluetoothOpenError> {
    let class = ensure_delegate_class()?;
    let delegate = msg!(class as Id, "new");
    let name = c_string("_state");
    object_setInstanceVariable(delegate, name.as_ptr(), state.cast());
    Ok(delegate)
}

unsafe fn ensure_delegate_class() -> Result<ClassPtr, BluetoothOpenError> {
    let class_name = c_string("BoardVmCoreBluetoothBleDelegate");
    let existing = objc_getClass(class_name.as_ptr());
    if !existing.is_null() {
        return Ok(existing);
    }

    let superclass = objc_class("NSObject")?;
    let cls = objc_allocateClassPair(superclass, class_name.as_ptr(), 0);
    if cls.is_null() {
        return Err(BluetoothOpenError::Backend {
            message: "failed to allocate CoreBluetooth delegate class".to_string(),
        });
    }

    let ivar_name = c_string("_state");
    if !class_addIvar(
        cls,
        ivar_name.as_ptr(),
        std::mem::size_of::<*mut CoreBluetoothState>(),
        std::mem::align_of::<*mut CoreBluetoothState>() as u8,
        c_string("^v").as_ptr(),
    ) {
        return Err(BluetoothOpenError::Backend {
            message: "failed to add CoreBluetooth delegate state ivar".to_string(),
        });
    }

    add_method(
        cls,
        "centralManagerDidUpdateState:",
        central_manager_did_update_state as *const c_void,
        "v@:@",
    )?;
    add_method(
        cls,
        "centralManager:didDiscoverPeripheral:advertisementData:RSSI:",
        central_manager_did_discover_peripheral as *const c_void,
        "v@:@@@@",
    )?;
    add_method(
        cls,
        "centralManager:didConnectPeripheral:",
        central_manager_did_connect_peripheral as *const c_void,
        "v@:@@",
    )?;
    add_method(
        cls,
        "centralManager:didFailToConnectPeripheral:error:",
        central_manager_did_fail_to_connect_peripheral as *const c_void,
        "v@:@@@",
    )?;
    add_method(
        cls,
        "centralManager:didDisconnectPeripheral:error:",
        central_manager_did_disconnect_peripheral as *const c_void,
        "v@:@@@",
    )?;
    add_method(
        cls,
        "peripheral:didDiscoverServices:",
        peripheral_did_discover_services as *const c_void,
        "v@:@@",
    )?;
    add_method(
        cls,
        "peripheral:didDiscoverCharacteristicsForService:error:",
        peripheral_did_discover_characteristics as *const c_void,
        "v@:@@@",
    )?;
    add_method(
        cls,
        "peripheral:didUpdateValueForCharacteristic:error:",
        peripheral_did_update_value as *const c_void,
        "v@:@@@",
    )?;
    add_method(
        cls,
        "peripheral:didWriteValueForCharacteristic:error:",
        peripheral_did_write_value as *const c_void,
        "v@:@@@",
    )?;

    objc_registerClassPair(cls);
    Ok(cls)
}

unsafe fn add_method(
    cls: ClassPtr,
    selector: &str,
    implementation: *const c_void,
    types: &str,
) -> Result<(), BluetoothOpenError> {
    if class_addMethod(cls, sel(selector), implementation, c_string(types).as_ptr()) {
        Ok(())
    } else {
        Err(BluetoothOpenError::Backend {
            message: format!("failed to add CoreBluetooth delegate method {selector}"),
        })
    }
}

extern "C" fn central_manager_did_update_state(this: Id, _cmd: Sel, central: Id) {
    unsafe {
        let Some(state) = state_from_delegate(this) else {
            return;
        };
        let manager_state = msg_usize!(central, "state");
        if manager_state == CB_MANAGER_STATE_POWERED_ON {
            state.powered_on = true;
            return;
        }
        if matches!(manager_state, 2 | 3 | 4) {
            state.set_error(format!(
                "CoreBluetooth manager is {} for {}",
                manager_state_name(manager_state),
                state.request.device
            ));
        }
    }
}

extern "C" fn central_manager_did_discover_peripheral(
    this: Id,
    _cmd: Sel,
    central: Id,
    peripheral: Id,
    _advertisement_data: Id,
    _rssi: Id,
) {
    unsafe {
        let Some(state) = state_from_delegate(this) else {
            return;
        };
        if state.peripheral != NIL {
            return;
        }
        state.peripheral = retain(peripheral);
        state.scanning = false;
        msg!(central, "stopScan");
        msg!(state.peripheral, "setDelegate:", this);
        msg!(central, "connectPeripheral:options:", state.peripheral, NIL);
    }
}

extern "C" fn central_manager_did_connect_peripheral(
    this: Id,
    _cmd: Sel,
    _central: Id,
    peripheral: Id,
) {
    unsafe {
        let Some(state) = state_from_delegate(this) else {
            return;
        };
        msg!(peripheral, "discoverServices:", state.service_uuid_array);
    }
}

extern "C" fn central_manager_did_fail_to_connect_peripheral(
    this: Id,
    _cmd: Sel,
    _central: Id,
    _peripheral: Id,
    error: Id,
) {
    unsafe {
        let Some(state) = state_from_delegate(this) else {
            return;
        };
        state.set_error(format!(
            "CoreBluetooth failed to connect to {}: {}",
            state.request.device,
            error_description(error)
        ));
    }
}

extern "C" fn central_manager_did_disconnect_peripheral(
    this: Id,
    _cmd: Sel,
    _central: Id,
    _peripheral: Id,
    error: Id,
) {
    unsafe {
        let Some(state) = state_from_delegate(this) else {
            return;
        };
        if error != NIL {
            state.set_error(format!(
                "CoreBluetooth disconnected from {}: {}",
                state.request.device,
                error_description(error)
            ));
        }
    }
}

extern "C" fn peripheral_did_discover_services(this: Id, _cmd: Sel, peripheral: Id, error: Id) {
    unsafe {
        let Some(state) = state_from_delegate(this) else {
            return;
        };
        if error != NIL {
            state.set_error(format!(
                "CoreBluetooth failed to discover BLE services on {}: {}",
                state.request.device,
                error_description(error)
            ));
            return;
        }

        let services = msg!(peripheral, "services");
        let Some(service) = find_uuid_object(services, &state.request.service_uuid) else {
            state.set_error(format!(
                "CoreBluetooth did not find service {} on {}",
                state.request.service_uuid, state.request.device
            ));
            return;
        };

        state.service = retain(service);
        msg!(
            peripheral,
            "discoverCharacteristics:forService:",
            state.characteristic_uuid_array,
            state.service
        );
    }
}

extern "C" fn peripheral_did_discover_characteristics(
    this: Id,
    _cmd: Sel,
    peripheral: Id,
    service: Id,
    error: Id,
) {
    unsafe {
        let Some(state) = state_from_delegate(this) else {
            return;
        };
        if error != NIL {
            state.set_error(format!(
                "CoreBluetooth failed to discover BLE characteristics on {}: {}",
                state.request.device,
                error_description(error)
            ));
            return;
        }

        let characteristics = msg!(service, "characteristics");
        for index in 0..msg_usize!(characteristics, "count") {
            let characteristic = msg!(characteristics, "objectAtIndex:", index);
            let uuid = msg!(characteristic, "UUID");
            if uuid_matches(uuid, &state.request.write_characteristic_uuid) {
                release_if_present(&mut state.write_characteristic);
                state.write_characteristic = retain(characteristic);
            }
            if uuid_matches(uuid, &state.request.notify_characteristic_uuid) {
                release_if_present(&mut state.notify_characteristic);
                state.notify_characteristic = retain(characteristic);
            }
        }

        if state.write_characteristic == NIL {
            state.set_error(format!(
                "CoreBluetooth did not find write characteristic {} on {}",
                state.request.write_characteristic_uuid, state.request.device
            ));
            return;
        }
        if state.notify_characteristic == NIL {
            state.set_error(format!(
                "CoreBluetooth did not find notify characteristic {} on {}",
                state.request.notify_characteristic_uuid, state.request.device
            ));
            return;
        }

        msg!(
            peripheral,
            "setNotifyValue:forCharacteristic:",
            true,
            state.notify_characteristic
        );
        state.ready = true;
    }
}

extern "C" fn peripheral_did_update_value(
    this: Id,
    _cmd: Sel,
    _peripheral: Id,
    characteristic: Id,
    error: Id,
) {
    unsafe {
        let Some(state) = state_from_delegate(this) else {
            return;
        };
        if error != NIL {
            state.set_error(format!(
                "CoreBluetooth failed to read BLE notification from {}: {}",
                state.request.device,
                error_description(error)
            ));
            return;
        }

        let data = msg!(characteristic, "value");
        if data == NIL {
            return;
        }
        let bytes = msg_ptr!(data, "bytes") as *const u8;
        let len = msg_usize!(data, "length");
        if bytes.is_null() {
            return;
        }
        let notification = std::slice::from_raw_parts(bytes, len).to_vec();
        state.notifications.push_back(notification);
    }
}

extern "C" fn peripheral_did_write_value(
    this: Id,
    _cmd: Sel,
    _peripheral: Id,
    _characteristic: Id,
    error: Id,
) {
    unsafe {
        let Some(state) = state_from_delegate(this) else {
            return;
        };
        if error != NIL {
            state.set_error(format!(
                "CoreBluetooth failed to write BLE characteristic on {}: {}",
                state.request.device,
                error_description(error)
            ));
        }
        state.pending_write = false;
    }
}

unsafe fn state_from_delegate(this: Id) -> Option<&'static mut CoreBluetoothState> {
    let name = c_string("_state");
    let mut pointer: *mut c_void = ptr::null_mut();
    object_getInstanceVariable(this, name.as_ptr(), &mut pointer);
    pointer.cast::<CoreBluetoothState>().as_mut()
}

unsafe fn core_bluetooth_uuid(uuid_string: &str) -> Result<Id, BluetoothOpenError> {
    let class = objc_class("CBUUID")? as Id;
    let string = objc_bridge::cfstring(uuid_string);
    let uuid = msg!(class, "UUIDWithString:", string);
    CFRelease(string);
    if uuid == NIL {
        return Err(BluetoothOpenError::Backend {
            message: format!("CoreBluetooth rejected UUID {uuid_string}"),
        });
    }
    Ok(retain(uuid))
}

unsafe fn ns_array_with_objects(objects: &[Id]) -> Result<Id, BluetoothOpenError> {
    let class = objc_class("NSArray")? as Id;
    let send: unsafe extern "C" fn(Id, Sel, *const Id, usize) -> Id =
        std::mem::transmute(objc_msgSend as *const ());
    let array = send(
        class,
        sel("arrayWithObjects:count:"),
        objects.as_ptr(),
        objects.len(),
    );
    if array == NIL {
        return Err(BluetoothOpenError::Backend {
            message: "failed to allocate CoreBluetooth UUID array".to_string(),
        });
    }
    Ok(retain(array))
}

unsafe fn ns_data_with_bytes(bytes: &[u8]) -> Result<Id, BluetoothTransportError> {
    let class = objc_class("NSData").map_err(|_error| BluetoothTransportError::Link)? as Id;
    let send: unsafe extern "C" fn(Id, Sel, *const c_void, usize) -> Id =
        std::mem::transmute(objc_msgSend as *const ());
    let data = send(
        class,
        sel("dataWithBytes:length:"),
        bytes.as_ptr().cast(),
        bytes.len(),
    );
    if data == NIL {
        return Err(BluetoothTransportError::Link);
    }
    Ok(data)
}

unsafe fn find_uuid_object(objects: Id, wanted_uuid: &str) -> Option<Id> {
    if objects == NIL {
        return None;
    }
    for index in 0..msg_usize!(objects, "count") {
        let object = msg!(objects, "objectAtIndex:", index);
        let uuid = msg!(object, "UUID");
        if uuid_matches(uuid, wanted_uuid) {
            return Some(object);
        }
    }
    None
}

unsafe fn uuid_matches(uuid: Id, wanted_uuid: &str) -> bool {
    uuid_string(uuid)
        .map(|uuid| uuid.eq_ignore_ascii_case(wanted_uuid))
        .unwrap_or(false)
}

unsafe fn uuid_string(uuid: Id) -> Option<String> {
    if uuid == NIL {
        return None;
    }
    ns_string_to_string(msg!(uuid, "UUIDString"))
}

unsafe fn ns_string_to_string(string: Id) -> Option<String> {
    if string == NIL {
        return None;
    }
    let mut buffer = vec![0u8; 4096];
    if !CFStringGetCString(
        string.cast(),
        buffer.as_mut_ptr().cast(),
        buffer.len() as i64,
        K_CF_STRING_ENCODING_UTF8,
    ) {
        return None;
    }
    let nul = buffer
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(buffer.len());
    String::from_utf8(buffer[..nul].to_vec()).ok()
}

unsafe fn error_description(error: Id) -> String {
    if error == NIL {
        return "no error details".to_string();
    }
    let description = msg!(error, "localizedDescription");
    ns_string_to_string(description).unwrap_or_else(|| "unreadable CoreBluetooth error".to_string())
}

unsafe fn pump_run_loop_once(poll_ms: u64) -> Result<(), BluetoothOpenError> {
    let run_loop = msg!(objc_class("NSRunLoop")? as Id, "currentRunLoop");
    let mode = objc_bridge::cfstring("NSDefaultRunLoopMode");
    let date = msg!(
        objc_class("NSDate")? as Id,
        "dateWithTimeIntervalSinceNow:",
        poll_ms as f64 / 1_000.0
    );
    let send: unsafe extern "C" fn(Id, Sel, Id, Id) -> bool =
        std::mem::transmute(objc_msgSend as *const ());
    send(run_loop, sel("runMode:beforeDate:"), mode, date);
    CFRelease(mode);
    Ok(())
}

unsafe fn objc_class(name: &str) -> Result<ClassPtr, BluetoothOpenError> {
    let name = c_string(name);
    let class = objc_getClass(name.as_ptr());
    if class.is_null() {
        Err(BluetoothOpenError::Backend {
            message: format!(
                "Objective-C class {} is unavailable",
                name.to_string_lossy()
            ),
        })
    } else {
        Ok(class)
    }
}

fn manager_state_name(state: usize) -> &'static str {
    match state {
        0 => "unknown",
        1 => "resetting",
        2 => "unsupported",
        3 => "unauthorized",
        4 => "powered off",
        5 => "powered on",
        _ => "unrecognized",
    }
}

unsafe fn release_if_present(value: &mut Id) {
    if *value != NIL {
        release(*value);
        *value = NIL;
    }
}

fn c_string(value: &str) -> CString {
    CString::new(value).expect("Objective-C runtime strings must not contain NUL bytes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_timeouts_keep_event_loop_polling_responsive() {
        let timeouts = MacosCoreBluetoothTimeouts::default();

        assert!(timeouts.power_on_ms >= timeouts.poll_ms);
        assert!(timeouts.connect_ms >= timeouts.poll_ms);
        assert!(timeouts.io_ms >= timeouts.poll_ms);
        assert!(timeouts.poll_ms > 0);
    }
}
