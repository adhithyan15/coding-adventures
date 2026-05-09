#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, CString};
use std::ptr;

use board_vm_host::{
    BlinkProgram, BLINK_MODULE_LEN, DEFAULT_INSTRUCTION_BUDGET, DEFAULT_PROGRAM_ID,
    DEFAULT_RUN_FLAGS,
};
use board_vm_language_core::{
    bluetooth_endpoint_candidates_from_devices, board_family_name, build_blink_module,
    build_caps_query_wire_frame, build_hello_wire_frame, build_program_begin_wire_frame,
    build_program_chunk_wire_frame, build_program_end_wire_frame, build_run_wire_frame,
    connection_options_for_target, connection_transport_name, detect_target, discover_devices,
    discover_devices_from_paths, esp_upload_options_for_target, host_endpoint_transport_name,
    known_targets, onboard_led_kind, parse_bluetooth_endpoint as core_parse_bluetooth_endpoint,
    pico_uf2_upload_options_for_target, wireless_transport_name, BoardVmLanguageSession,
    BuiltWireFrame, LanguageBluetoothDiscoveredDevice, LanguageBluetoothEndpoint,
    LanguageBluetoothEndpointCandidate, LanguageConnectionOption, LanguageCoreError,
    LanguageEspUploadOptions, LanguageHostDevice, LanguageOnboardLed, LanguagePicoUf2UploadOptions,
    LanguageTargetInfo, LanguageWirelessInterface,
};
use lua_bridge::{
    get_str, luaL_Reg, luaL_checkinteger, lua_Integer, lua_State, lua_createtable, lua_getfield,
    lua_gettop, lua_pop, lua_pushboolean, lua_pushinteger, lua_pushlstring, lua_pushnil,
    lua_rawgeti, lua_rawlen, lua_rawseti, lua_setfield, lua_toboolean, lua_tointegerx,
    lua_tolstring, lua_type, push_str, raise_error, register_lib, LUA_TNIL, LUA_TTABLE,
};

unsafe fn check_u8(L: *mut lua_State, index: c_int, name: &str) -> u8 {
    let value = luaL_checkinteger(L, index);
    if !(0..=u8::MAX as lua_Integer).contains(&value) {
        raise_error(L, &format!("{name} must fit in u8"));
    }
    value as u8
}

unsafe fn check_u16(L: *mut lua_State, index: c_int, name: &str) -> u16 {
    let value = luaL_checkinteger(L, index);
    if !(0..=u16::MAX as lua_Integer).contains(&value) {
        raise_error(L, &format!("{name} must fit in u16"));
    }
    value as u16
}

unsafe fn check_u32(L: *mut lua_State, index: c_int, name: &str) -> u32 {
    let value = luaL_checkinteger(L, index);
    if !(0..=u32::MAX as lua_Integer).contains(&value) {
        raise_error(L, &format!("{name} must fit in u32"));
    }
    value as u32
}

unsafe fn check_bytes(L: *mut lua_State, index: c_int, name: &str) -> Vec<u8> {
    let mut len = 0usize;
    let ptr = lua_tolstring(L, index, &mut len);
    if ptr.is_null() {
        raise_error(L, &format!("{name} must be a string"));
    }
    std::slice::from_raw_parts(ptr as *const u8, len).to_vec()
}

unsafe fn push_bytes(L: *mut lua_State, bytes: &[u8]) {
    lua_pushlstring(L, bytes.as_ptr() as *const c_char, bytes.len());
}

unsafe fn set_int(L: *mut lua_State, key: &str, value: u64) {
    let key = CString::new(key).unwrap();
    lua_pushinteger(L, value as lua_Integer);
    lua_setfield(L, -2, key.as_ptr());
}

unsafe fn set_str(L: *mut lua_State, key: &str, value: &str) {
    let key = CString::new(key).unwrap();
    push_str(L, value);
    lua_setfield(L, -2, key.as_ptr());
}

unsafe fn set_bool(L: *mut lua_State, key: &str, value: bool) {
    let key = CString::new(key).unwrap();
    lua_pushboolean(L, i32::from(value));
    lua_setfield(L, -2, key.as_ptr());
}

unsafe fn set_bytes(L: *mut lua_State, key: &str, value: &[u8]) {
    let key = CString::new(key).unwrap();
    push_bytes(L, value);
    lua_setfield(L, -2, key.as_ptr());
}

unsafe fn set_nil(L: *mut lua_State, key: &str) {
    let key = CString::new(key).unwrap();
    lua_pushnil(L);
    lua_setfield(L, -2, key.as_ptr());
}

unsafe fn read_string_table(L: *mut lua_State, index: c_int, name: &str) -> Vec<String> {
    if lua_type(L, index) != LUA_TTABLE {
        raise_error(L, &format!("{name} must be a table of strings"));
    }
    let len = lua_rawlen(L, index);
    let mut values = Vec::with_capacity(len as usize);
    for i in 1..=len {
        lua_rawgeti(L, index, i);
        let value = get_str(L, -1)
            .unwrap_or_else(|| raise_error(L, &format!("{name}[{i}] must be a string")));
        values.push(value);
        lua_pop(L, 1);
    }
    values
}

unsafe fn absolute_index(L: *mut lua_State, index: c_int) -> c_int {
    if index < 0 {
        lua_gettop(L) + index + 1
    } else {
        index
    }
}

unsafe fn read_optional_string_field(
    L: *mut lua_State,
    table_index: c_int,
    key: &str,
) -> Option<String> {
    let table_index = absolute_index(L, table_index);
    let c_key = CString::new(key).unwrap();
    lua_getfield(L, table_index, c_key.as_ptr());
    let value = if lua_type(L, -1) == LUA_TNIL {
        None
    } else {
        Some(get_str(L, -1).unwrap_or_else(|| raise_error(L, &format!("{key} must be a string"))))
    };
    lua_pop(L, 1);
    value
}

unsafe fn read_bool_field(L: *mut lua_State, table_index: c_int, key: &str) -> bool {
    let table_index = absolute_index(L, table_index);
    let c_key = CString::new(key).unwrap();
    lua_getfield(L, table_index, c_key.as_ptr());
    let value = lua_type(L, -1) != LUA_TNIL && lua_toboolean(L, -1) != 0;
    lua_pop(L, 1);
    value
}

unsafe fn read_string_array_field(L: *mut lua_State, table_index: c_int, key: &str) -> Vec<String> {
    let table_index = absolute_index(L, table_index);
    let c_key = CString::new(key).unwrap();
    lua_getfield(L, table_index, c_key.as_ptr());
    let value = if lua_type(L, -1) == LUA_TNIL {
        Vec::new()
    } else {
        read_string_table(L, -1, key)
    };
    lua_pop(L, 1);
    value
}

unsafe fn read_u8_array_field(L: *mut lua_State, table_index: c_int, key: &str) -> Vec<u8> {
    let table_index = absolute_index(L, table_index);
    let c_key = CString::new(key).unwrap();
    lua_getfield(L, table_index, c_key.as_ptr());
    if lua_type(L, -1) == LUA_TNIL {
        lua_pop(L, 1);
        return Vec::new();
    }
    if lua_type(L, -1) != LUA_TTABLE {
        raise_error(L, &format!("{key} must be a table of integers"));
    }

    let len = lua_rawlen(L, -1);
    let mut values = Vec::with_capacity(len as usize);
    for i in 1..=len {
        lua_rawgeti(L, -1, i);
        let mut isnum = 0;
        let value = lua_tointegerx(L, -1, &mut isnum);
        if isnum == 0 || !(0..=u8::MAX as lua_Integer).contains(&value) {
            raise_error(L, &format!("{key}[{i}] must fit in u8"));
        }
        values.push(value as u8);
        lua_pop(L, 1);
    }
    lua_pop(L, 1);
    values
}

unsafe fn read_bluetooth_discovered_devices(
    L: *mut lua_State,
    index: c_int,
) -> Vec<LanguageBluetoothDiscoveredDevice> {
    if lua_type(L, index) != LUA_TTABLE {
        raise_error(L, "devices must be a table");
    }

    let table_index = absolute_index(L, index);
    let len = lua_rawlen(L, table_index);
    let mut devices = Vec::with_capacity(len as usize);
    for i in 1..=len {
        lua_rawgeti(L, table_index, i);
        if lua_type(L, -1) != LUA_TTABLE {
            raise_error(L, &format!("devices[{i}] must be a table"));
        }
        let id = read_optional_string_field(L, -1, "id")
            .unwrap_or_else(|| raise_error(L, &format!("devices[{i}].id must be a string")));
        devices.push(LanguageBluetoothDiscoveredDevice {
            id,
            name: read_optional_string_field(L, -1, "name"),
            address: read_optional_string_field(L, -1, "address"),
            paired: read_bool_field(L, -1, "paired"),
            service_uuids: read_string_array_field(L, -1, "service_uuids"),
            characteristic_uuids: read_string_array_field(L, -1, "characteristic_uuids"),
            board_vm_rfcomm_channels: read_u8_array_field(L, -1, "board_vm_rfcomm_channels"),
        });
        lua_pop(L, 1);
    }
    devices
}

fn core_result<T>(L: *mut lua_State, result: Result<T, LanguageCoreError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => unsafe {
            raise_error(L, &format!("Board VM language core error: {error:?}"))
        },
    }
}

unsafe fn push_wire_result(
    L: *mut lua_State,
    session: &BoardVmLanguageSession,
    built: BuiltWireFrame,
    wire: &[u8],
) -> c_int {
    lua_bridge::lua_newtable(L);
    set_bytes(L, "frame", &wire[..built.len]);
    set_int(L, "request_id", built.request_id as u64);
    set_int(L, "next_request_id", session.next_request_id() as u64);
    1
}

unsafe fn push_string_array(L: *mut lua_State, values: &[String]) {
    lua_createtable(L, values.len() as c_int, 0);
    for (index, value) in values.iter().enumerate() {
        push_str(L, value);
        lua_rawseti(L, -2, (index + 1) as lua_Integer);
    }
}

unsafe fn push_onboard_led(L: *mut lua_State, led: Option<LanguageOnboardLed>) {
    match led {
        Some(led) => {
            lua_bridge::lua_newtable(L);
            set_str(L, "kind", onboard_led_kind(led));
            let pin = match led {
                LanguageOnboardLed::Gpio(pin) | LanguageOnboardLed::WirelessChipGpio(pin) => pin,
            };
            set_int(L, "pin", pin as u64);
        }
        None => lua_pushnil(L),
    }
}

unsafe fn push_wireless_interface(L: *mut lua_State, interface: &LanguageWirelessInterface) {
    lua_bridge::lua_newtable(L);
    set_str(L, "transport", wireless_transport_name(interface.transport));
    set_str(L, "chip", &interface.chip);
    set_bool(L, "command_transport", interface.command_transport);
    set_bool(L, "ota_update", interface.ota_update);
}

unsafe fn push_wireless_interfaces(L: *mut lua_State, interfaces: &[LanguageWirelessInterface]) {
    lua_createtable(L, interfaces.len() as c_int, 0);
    for (index, interface) in interfaces.iter().enumerate() {
        push_wireless_interface(L, interface);
        lua_rawseti(L, -2, (index + 1) as lua_Integer);
    }
}

unsafe fn push_connection_option(L: *mut lua_State, option: &LanguageConnectionOption) {
    lua_bridge::lua_newtable(L);
    set_str(L, "transport", connection_transport_name(option.transport));
    set_str(L, "display_name", &option.display_name);
    set_bool(L, "command_transport", option.command_transport);
    set_bool(L, "ota_update", option.ota_update);
    set_str(L, "requires", &option.requires);
    set_str(
        L,
        "endpoint_transport",
        host_endpoint_transport_name(option.endpoint_transport),
    );
    set_str(L, "endpoint_scheme", &option.endpoint_scheme);
    set_str(L, "wire_protocol", &option.wire_protocol);
}

unsafe fn push_connection_options(L: *mut lua_State, options: &[LanguageConnectionOption]) {
    lua_createtable(L, options.len() as c_int, 0);
    for (index, option) in options.iter().enumerate() {
        push_connection_option(L, option);
        lua_rawseti(L, -2, (index + 1) as lua_Integer);
    }
}

unsafe fn push_optional_str(L: *mut lua_State, key: &str, value: Option<&str>) {
    let key = CString::new(key).unwrap();
    match value {
        Some(value) => push_str(L, value),
        None => lua_pushnil(L),
    }
    lua_setfield(L, -2, key.as_ptr());
}

unsafe fn push_optional_int(L: *mut lua_State, key: &str, value: Option<u8>) {
    let key = CString::new(key).unwrap();
    match value {
        Some(value) => lua_pushinteger(L, value as lua_Integer),
        None => lua_pushnil(L),
    }
    lua_setfield(L, -2, key.as_ptr());
}

unsafe fn push_bluetooth_endpoint(L: *mut lua_State, endpoint: &LanguageBluetoothEndpoint) {
    lua_bridge::lua_newtable(L);
    set_str(L, "endpoint", &endpoint.endpoint);
    set_str(
        L,
        "transport",
        connection_transport_name(endpoint.transport),
    );
    set_str(
        L,
        "endpoint_transport",
        host_endpoint_transport_name(endpoint.endpoint_transport),
    );
    set_str(L, "endpoint_scheme", &endpoint.endpoint_scheme);
    set_str(L, "device", &endpoint.device);
    push_optional_str(L, "service_uuid", endpoint.service_uuid.as_deref());
    push_optional_str(
        L,
        "write_characteristic_uuid",
        endpoint.write_characteristic_uuid.as_deref(),
    );
    push_optional_str(
        L,
        "notify_characteristic_uuid",
        endpoint.notify_characteristic_uuid.as_deref(),
    );
    push_optional_int(L, "channel", endpoint.channel);
}

unsafe fn push_bluetooth_endpoint_candidate(
    L: *mut lua_State,
    candidate: &LanguageBluetoothEndpointCandidate,
) {
    lua_bridge::lua_newtable(L);
    let key = CString::new("endpoint").unwrap();
    push_bluetooth_endpoint(L, &candidate.endpoint);
    lua_setfield(L, -2, key.as_ptr());
    set_str(L, "device", &candidate.device);
    set_str(L, "display_name", &candidate.display_name);
    set_bool(L, "paired", candidate.paired);
    set_bool(L, "requires_pairing", candidate.requires_pairing);
}

unsafe fn push_bluetooth_endpoint_candidates(
    L: *mut lua_State,
    candidates: &[LanguageBluetoothEndpointCandidate],
) {
    lua_createtable(L, candidates.len() as c_int, 0);
    for (index, candidate) in candidates.iter().enumerate() {
        push_bluetooth_endpoint_candidate(L, candidate);
        lua_rawseti(L, -2, (index + 1) as lua_Integer);
    }
}

unsafe fn push_target(L: *mut lua_State, target: &LanguageTargetInfo) {
    lua_bridge::lua_newtable(L);
    set_str(L, "board_id", &target.board_id);
    set_str(L, "display_name", &target.display_name);
    set_str(L, "family", board_family_name(target.family));
    set_str(L, "runtime_id", &target.runtime_id);
    set_str(L, "mcu", &target.mcu);
    set_str(L, "core", &target.core);
    set_str(L, "rust_target", &target.rust_target);
    set_int(L, "clock_hz", target.clock_hz as u64);
    set_int(
        L,
        "operating_voltage_mv",
        target.operating_voltage_mv as u64,
    );
    set_int(L, "digital_pin_count", target.digital_pin_count as u64);

    let key = CString::new("onboard_led").unwrap();
    push_onboard_led(L, target.onboard_led);
    lua_setfield(L, -2, key.as_ptr());

    let key = CString::new("wireless").unwrap();
    push_wireless_interfaces(L, &target.wireless);
    lua_setfield(L, -2, key.as_ptr());

    let key = CString::new("connection_options").unwrap();
    push_connection_options(L, &target.connection_options);
    lua_setfield(L, -2, key.as_ptr());

    let key = CString::new("capabilities").unwrap();
    push_string_array(L, &target.capabilities);
    lua_setfield(L, -2, key.as_ptr());
}

unsafe fn push_targets(L: *mut lua_State, targets: &[LanguageTargetInfo]) {
    lua_createtable(L, targets.len() as c_int, 0);
    for (index, target) in targets.iter().enumerate() {
        push_target(L, target);
        lua_rawseti(L, -2, (index + 1) as lua_Integer);
    }
}

unsafe fn push_esp_upload_options(L: *mut lua_State, options: &LanguageEspUploadOptions) {
    lua_bridge::lua_newtable(L);
    set_str(L, "board_id", &options.board_id);
    set_int(L, "baud_rate", options.baud_rate as u64);
    set_int(L, "timeout_ms", options.timeout_ms);
    set_bool(L, "reset_into_bootloader", options.reset_into_bootloader);
    set_int(L, "offset", options.offset as u64);
    set_int(L, "block_size", options.block_size as u64);
    match options.flash_size {
        Some(size) => set_int(L, "flash_size", size as u64),
        None => set_nil(L, "flash_size"),
    }
    set_bool(L, "verify_md5", options.verify_md5);
    set_bool(L, "stay_in_bootloader", options.stay_in_bootloader);
}

unsafe fn push_pico_uf2_upload_options(L: *mut lua_State, options: &LanguagePicoUf2UploadOptions) {
    lua_bridge::lua_newtable(L);
    set_str(L, "board_id", &options.board_id);
    set_str(L, "command", &options.command);
    set_str(L, "volume_label", &options.volume_label);
    set_str(L, "image_extension", &options.image_extension);
    set_bool(L, "auto_detect_mount", options.auto_detect_mount);
}

unsafe fn push_device(L: *mut lua_State, device: &LanguageHostDevice) {
    lua_bridge::lua_newtable(L);
    set_str(L, "id", &device.id);
    set_str(L, "port", &device.port);
    set_str(L, "transport", &device.transport);
    set_str(L, "display_name", &device.display_name);
    set_int(L, "target_confidence", device.target_confidence as u64);
    set_bool(L, "bootloader", device.bootloader);

    let key = CString::new("target").unwrap();
    if let Some(target) = &device.target {
        push_target(L, target);
    } else {
        lua_pushnil(L);
    }
    lua_setfield(L, -2, key.as_ptr());

    let key = CString::new("tags").unwrap();
    push_string_array(L, &device.tags);
    lua_setfield(L, -2, key.as_ptr());
}

unsafe fn push_devices(L: *mut lua_State, devices: &[LanguageHostDevice]) {
    lua_createtable(L, devices.len() as c_int, 0);
    for (index, device) in devices.iter().enumerate() {
        push_device(L, device);
        lua_rawseti(L, -2, (index + 1) as lua_Integer);
    }
}

unsafe extern "C" fn lua_known_targets(L: *mut lua_State) -> c_int {
    let targets = known_targets();
    push_targets(L, &targets);
    1
}

unsafe extern "C" fn lua_detect_target(L: *mut lua_State) -> c_int {
    let selector = get_str(L, 1).unwrap_or_else(|| raise_error(L, "selector must be a string"));
    match detect_target(&selector) {
        Some(target) => push_target(L, &target),
        None => lua_pushnil(L),
    }
    1
}

unsafe extern "C" fn lua_connection_options(L: *mut lua_State) -> c_int {
    let selector = get_str(L, 1).unwrap_or_else(|| raise_error(L, "selector must be a string"));
    match connection_options_for_target(&selector) {
        Some(options) => push_connection_options(L, &options),
        None => raise_error(L, &format!("unsupported board: {selector}")),
    }
    1
}

unsafe extern "C" fn lua_bluetooth_endpoint(L: *mut lua_State) -> c_int {
    let endpoint = get_str(L, 1).unwrap_or_else(|| raise_error(L, "endpoint must be a string"));
    match core_parse_bluetooth_endpoint(&endpoint) {
        Some(endpoint) => push_bluetooth_endpoint(L, &endpoint),
        None => lua_pushnil(L),
    }
    1
}

unsafe extern "C" fn lua_bluetooth_endpoint_candidates(L: *mut lua_State) -> c_int {
    let devices = read_bluetooth_discovered_devices(L, 1);
    let candidates = bluetooth_endpoint_candidates_from_devices(&devices);
    push_bluetooth_endpoint_candidates(L, &candidates);
    1
}

unsafe extern "C" fn lua_esp_upload_options(L: *mut lua_State) -> c_int {
    let selector = get_str(L, 1).unwrap_or_else(|| raise_error(L, "selector must be a string"));
    match esp_upload_options_for_target(&selector) {
        Some(options) => push_esp_upload_options(L, &options),
        None => lua_pushnil(L),
    }
    1
}

unsafe extern "C" fn lua_pico_uf2_upload_options(L: *mut lua_State) -> c_int {
    let selector = get_str(L, 1).unwrap_or_else(|| raise_error(L, "selector must be a string"));
    match pico_uf2_upload_options_for_target(&selector) {
        Some(options) => push_pico_uf2_upload_options(L, &options),
        None => lua_pushnil(L),
    }
    1
}

unsafe extern "C" fn lua_devices(L: *mut lua_State) -> c_int {
    let _ = lua_gettop(L);
    let devices = discover_devices();
    push_devices(L, &devices);
    1
}

unsafe extern "C" fn lua_classify_devices(L: *mut lua_State) -> c_int {
    let paths = read_string_table(L, 1, "paths");
    let devices = discover_devices_from_paths(paths);
    push_devices(L, &devices);
    1
}

unsafe extern "C" fn lua_hello_wire(L: *mut lua_State) -> c_int {
    let next_request_id = check_u16(L, 1, "next_request_id");
    let host_name = get_str(L, 2).unwrap_or_else(|| raise_error(L, "host_name must be a string"));
    let host_nonce = check_u32(L, 3, "host_nonce");
    let mut session = BoardVmLanguageSession::with_next_request_id(next_request_id);
    let mut wire = vec![0; host_name.len().saturating_add(96)];
    let built = core_result(
        L,
        build_hello_wire_frame(&mut session, &host_name, host_nonce, &mut wire),
    );
    push_wire_result(L, &session, built, &wire)
}

unsafe extern "C" fn lua_caps_query_wire(L: *mut lua_State) -> c_int {
    let next_request_id = check_u16(L, 1, "next_request_id");
    let mut session = BoardVmLanguageSession::with_next_request_id(next_request_id);
    let mut wire = [0u8; 64];
    let built = core_result(L, build_caps_query_wire_frame(&mut session, &mut wire));
    push_wire_result(L, &session, built, &wire)
}

unsafe extern "C" fn lua_blink_module(L: *mut lua_State) -> c_int {
    let pin = check_u8(L, 1, "pin");
    let high_ms = check_u16(L, 2, "high_ms");
    let low_ms = check_u16(L, 3, "low_ms");
    let max_stack = check_u8(L, 4, "max_stack");
    let mut module = vec![0; BLINK_MODULE_LEN];
    let len = core_result(
        L,
        build_blink_module(
            BlinkProgram {
                pin,
                high_ms,
                low_ms,
                max_stack,
            },
            &mut module,
        ),
    );
    push_bytes(L, &module[..len]);
    1
}

unsafe extern "C" fn lua_program_begin_wire(L: *mut lua_State) -> c_int {
    let next_request_id = check_u16(L, 1, "next_request_id");
    let program_id = check_u16(L, 2, "program_id");
    let module = check_bytes(L, 3, "module");
    let mut session = BoardVmLanguageSession::with_next_request_id(next_request_id);
    let mut wire = [0u8; 96];
    let built = core_result(
        L,
        build_program_begin_wire_frame(&mut session, program_id, &module, &mut wire),
    );
    push_wire_result(L, &session, built, &wire)
}

unsafe extern "C" fn lua_program_chunk_wire(L: *mut lua_State) -> c_int {
    let next_request_id = check_u16(L, 1, "next_request_id");
    let program_id = check_u16(L, 2, "program_id");
    let offset = check_u32(L, 3, "offset");
    let chunk = check_bytes(L, 4, "chunk");
    let mut session = BoardVmLanguageSession::with_next_request_id(next_request_id);
    let mut wire = vec![0; chunk.len().saturating_add(96)];
    let built = core_result(
        L,
        build_program_chunk_wire_frame(&mut session, program_id, offset, &chunk, &mut wire),
    );
    push_wire_result(L, &session, built, &wire)
}

unsafe extern "C" fn lua_program_end_wire(L: *mut lua_State) -> c_int {
    let next_request_id = check_u16(L, 1, "next_request_id");
    let program_id = check_u16(L, 2, "program_id");
    let mut session = BoardVmLanguageSession::with_next_request_id(next_request_id);
    let mut wire = [0u8; 64];
    let built = core_result(
        L,
        build_program_end_wire_frame(&mut session, program_id, &mut wire),
    );
    push_wire_result(L, &session, built, &wire)
}

unsafe extern "C" fn lua_run_wire(L: *mut lua_State) -> c_int {
    let next_request_id = check_u16(L, 1, "next_request_id");
    let program_id = check_u16(L, 2, "program_id");
    let flags = check_u8(L, 3, "flags");
    let instruction_budget = check_u32(L, 4, "instruction_budget");
    let time_budget_ms = check_u32(L, 5, "time_budget_ms");
    let mut session = BoardVmLanguageSession::with_next_request_id(next_request_id);
    let mut wire = [0u8; 96];
    let built = core_result(
        L,
        build_run_wire_frame(
            &mut session,
            program_id,
            flags,
            instruction_budget,
            time_budget_ms,
            &mut wire,
        ),
    );
    push_wire_result(L, &session, built, &wire)
}

unsafe extern "C" fn lua_defaults(L: *mut lua_State) -> c_int {
    let _ = lua_gettop(L);
    lua_bridge::lua_newtable(L);
    set_int(L, "program_id", DEFAULT_PROGRAM_ID as u64);
    set_int(L, "run_flags", DEFAULT_RUN_FLAGS as u64);
    set_int(L, "instruction_budget", DEFAULT_INSTRUCTION_BUDGET as u64);
    1
}

struct FuncTable([luaL_Reg; 18]);
unsafe impl Sync for FuncTable {}

static FUNCS: FuncTable = FuncTable([
    luaL_Reg {
        name: b"known_targets\0".as_ptr() as *const _,
        func: Some(lua_known_targets),
    },
    luaL_Reg {
        name: b"detect_target\0".as_ptr() as *const _,
        func: Some(lua_detect_target),
    },
    luaL_Reg {
        name: b"connection_options\0".as_ptr() as *const _,
        func: Some(lua_connection_options),
    },
    luaL_Reg {
        name: b"bluetooth_endpoint\0".as_ptr() as *const _,
        func: Some(lua_bluetooth_endpoint),
    },
    luaL_Reg {
        name: b"bluetooth_endpoint_candidates\0".as_ptr() as *const _,
        func: Some(lua_bluetooth_endpoint_candidates),
    },
    luaL_Reg {
        name: b"esp_upload_options\0".as_ptr() as *const _,
        func: Some(lua_esp_upload_options),
    },
    luaL_Reg {
        name: b"pico_uf2_upload_options\0".as_ptr() as *const _,
        func: Some(lua_pico_uf2_upload_options),
    },
    luaL_Reg {
        name: b"devices\0".as_ptr() as *const _,
        func: Some(lua_devices),
    },
    luaL_Reg {
        name: b"classify_devices\0".as_ptr() as *const _,
        func: Some(lua_classify_devices),
    },
    luaL_Reg {
        name: b"hello_wire\0".as_ptr() as *const _,
        func: Some(lua_hello_wire),
    },
    luaL_Reg {
        name: b"caps_query_wire\0".as_ptr() as *const _,
        func: Some(lua_caps_query_wire),
    },
    luaL_Reg {
        name: b"blink_module\0".as_ptr() as *const _,
        func: Some(lua_blink_module),
    },
    luaL_Reg {
        name: b"program_begin_wire\0".as_ptr() as *const _,
        func: Some(lua_program_begin_wire),
    },
    luaL_Reg {
        name: b"program_chunk_wire\0".as_ptr() as *const _,
        func: Some(lua_program_chunk_wire),
    },
    luaL_Reg {
        name: b"program_end_wire\0".as_ptr() as *const _,
        func: Some(lua_program_end_wire),
    },
    luaL_Reg {
        name: b"run_wire\0".as_ptr() as *const _,
        func: Some(lua_run_wire),
    },
    luaL_Reg {
        name: b"defaults\0".as_ptr() as *const _,
        func: Some(lua_defaults),
    },
    luaL_Reg {
        name: ptr::null(),
        func: None,
    },
]);

#[no_mangle]
pub unsafe extern "C" fn luaopen_board_vm_native(L: *mut lua_State) -> c_int {
    register_lib(L, &FUNCS.0);
    1
}
