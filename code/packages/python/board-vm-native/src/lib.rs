use std::ffi::{c_char, c_int, c_long, CString};
use std::ptr;

use board_vm_host::{
    BlinkProgram, GpioHandleCloseProgram, GpioHandleReadProgram, GpioHandleWriteProgram,
    GpioOpenProgram, GpioReadProgram, GpioWriteProgram, TimeNowProgram, TimeSleepMsProgram,
    BLINK_MODULE_LEN, GPIO_HANDLE_CLOSE_MODULE_LEN, GPIO_HANDLE_READ_MODULE_LEN,
    GPIO_HANDLE_WRITE_MODULE_LEN, GPIO_OPEN_MODULE_LEN, GPIO_READ_MODULE_LEN,
    GPIO_WRITE_MODULE_LEN, TIME_NOW_MODULE_LEN, TIME_SLEEP_MS_MODULE_LEN,
};
use board_vm_language_core::{
    bluetooth_endpoint_candidates_from_devices, board_family_name, build_blink_module,
    build_caps_query_wire_frame, build_gpio_handle_close_module, build_gpio_handle_read_module,
    build_gpio_handle_write_module, build_gpio_open_module, build_gpio_read_module,
    build_gpio_write_module, build_hello_wire_frame, build_program_begin_wire_frame,
    build_program_chunk_wire_frame, build_program_end_wire_frame, build_raw_module,
    build_run_background_wire_frame, build_run_wire_frame, build_stop_wire_frame,
    build_store_program_wire_frame, build_time_now_module, build_time_sleep_ms_module,
    capability_board_metadata, capability_bytecode_callable, capability_flag_names,
    capability_protocol_feature, connection_transport_name, decode_wire_response,
    detect_target as core_detect_target,
    discover_bluetooth_devices as core_discover_bluetooth_devices,
    discover_devices as core_discover_devices, discover_devices_from_paths,
    discover_pico_bootsel_mounts as core_discover_pico_bootsel_mounts,
    discover_pico_bootsel_mounts_in_roots, esp_upload_options_for_target,
    host_endpoint_transport_name, known_targets, onboard_led_kind,
    parse_bluetooth_endpoint as core_parse_bluetooth_endpoint, pico_uf2_upload_options_for_target,
    program_format_name, raw_module_len, run_status_name, wireless_transport_name,
    BoardVmLanguageSession, DecodedLanguageResponse, DecodedLanguageResponseBody,
    LanguageBluetoothDiscoveredDevice, LanguageBluetoothEndpoint,
    LanguageBluetoothEndpointCandidate, LanguageConnectionOption, LanguageCoreError,
    LanguageEspUploadOptions, LanguageHostDevice, LanguageOnboardLed, LanguagePicoUf2UploadOptions,
    LanguageTargetInfo, LanguageValue, LanguageWirelessInterface,
};
use python_bridge::*;

#[allow(non_snake_case)]
extern "C" {
    fn PyLong_AsLong(obj: PyObjectPtr) -> c_long;
    fn PyErr_Occurred() -> PyObjectPtr;
    fn PyDict_GetItemString(p: PyObjectPtr, key: *const c_char) -> PyObjectPtr;
    fn PyObject_IsTrue(o: PyObjectPtr) -> c_int;
}

unsafe extern "C" fn py_hello_wire(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let next_request_id = match parse_arg_u16(args, 0, "next_request_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let host_name = match parse_arg_str(args, 1) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "hello_wire() requires host_name as str");
            return ptr::null_mut();
        }
    };
    let host_nonce = match parse_arg_u32(args, 2, "host_nonce") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    with_session(next_request_id, |session| {
        let mut wire = vec![0; host_name.len().saturating_add(64).max(128)];
        let written = build_hello_wire_frame(session, &host_name, host_nonce, &mut wire)?;
        Ok(wire_result(&wire, written.len, session))
    })
}

unsafe extern "C" fn py_caps_query_wire(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let next_request_id = match parse_arg_u16(args, 0, "next_request_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    with_session(next_request_id, |session| {
        let mut wire = [0u8; 64];
        let written = build_caps_query_wire_frame(session, &mut wire)?;
        Ok(wire_result(&wire, written.len, session))
    })
}

unsafe extern "C" fn py_blink_module(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let pin = match parse_arg_u8(args, 0, "pin") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let high_ms = match parse_arg_u16(args, 1, "high_ms") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let low_ms = match parse_arg_u16(args, 2, "low_ms") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let max_stack = match parse_arg_u8(args, 3, "max_stack") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    let module = match build_blink_module_value(pin, high_ms, low_ms, max_stack) {
        Ok(module) => module,
        Err(error) => return raise_core_error("blink_module", error),
    };
    bytes_to_py(&module)
}

unsafe extern "C" fn py_gpio_read_module(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let pin = match parse_arg_u8(args, 0, "pin") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let mode = match parse_arg_u8(args, 1, "mode") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let max_stack = match parse_arg_u8(args, 2, "max_stack") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    let module = match build_gpio_read_module_value(pin, mode, max_stack) {
        Ok(module) => module,
        Err(error) => return raise_core_error("gpio_read_module", error),
    };
    bytes_to_py(&module)
}

unsafe extern "C" fn py_gpio_write_module(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let pin = match parse_arg_u8(args, 0, "pin") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let value = match parse_arg_u8(args, 1, "value") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let max_stack = match parse_arg_u8(args, 2, "max_stack") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    let module = match build_gpio_write_module_value(pin, value != 0, max_stack) {
        Ok(module) => module,
        Err(error) => return raise_core_error("gpio_write_module", error),
    };
    bytes_to_py(&module)
}

unsafe extern "C" fn py_gpio_open_module(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let pin = match parse_arg_u8(args, 0, "pin") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let mode = match parse_arg_u8(args, 1, "mode") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let max_stack = match parse_arg_u8(args, 2, "max_stack") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    let module = match build_gpio_open_module_value(pin, mode, max_stack) {
        Ok(module) => module,
        Err(error) => return raise_core_error("gpio_open_module", error),
    };
    bytes_to_py(&module)
}

unsafe extern "C" fn py_gpio_handle_read_module(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let max_stack = match parse_arg_u8(args, 0, "max_stack") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    let module = match build_gpio_handle_read_module_value(max_stack) {
        Ok(module) => module,
        Err(error) => return raise_core_error("gpio_handle_read_module", error),
    };
    bytes_to_py(&module)
}

unsafe extern "C" fn py_gpio_handle_write_module(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let value = match parse_arg_u8(args, 0, "value") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let max_stack = match parse_arg_u8(args, 1, "max_stack") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    let module = match build_gpio_handle_write_module_value(value != 0, max_stack) {
        Ok(module) => module,
        Err(error) => return raise_core_error("gpio_handle_write_module", error),
    };
    bytes_to_py(&module)
}

unsafe extern "C" fn py_gpio_handle_close_module(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let max_stack = match parse_arg_u8(args, 0, "max_stack") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    let module = match build_gpio_handle_close_module_value(max_stack) {
        Ok(module) => module,
        Err(error) => return raise_core_error("gpio_handle_close_module", error),
    };
    bytes_to_py(&module)
}

unsafe extern "C" fn py_time_now_module(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let max_stack = match parse_arg_u8(args, 0, "max_stack") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    let module = match build_time_now_module_value(max_stack) {
        Ok(module) => module,
        Err(error) => return raise_core_error("time_now_module", error),
    };
    bytes_to_py(&module)
}

unsafe extern "C" fn py_time_sleep_ms_module(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let duration_ms = match parse_arg_u16(args, 0, "duration_ms") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let max_stack = match parse_arg_u8(args, 1, "max_stack") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    let module = match build_time_sleep_ms_module_value(duration_ms, max_stack) {
        Ok(module) => module,
        Err(error) => return raise_core_error("time_sleep_ms_module", error),
    };
    bytes_to_py(&module)
}

unsafe extern "C" fn py_raw_module(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let flags = match parse_arg_u8(args, 0, "flags") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let max_stack = match parse_arg_u8(args, 1, "max_stack") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let code = match parse_arg_bytes(args, 2, "code") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let const_pool = match parse_arg_bytes(args, 3, "const_pool") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    let module = match build_raw_module_value(flags, max_stack, &code, &const_pool) {
        Ok(module) => module,
        Err(error) => return raise_core_error("raw_module", error),
    };
    bytes_to_py(&module)
}

unsafe extern "C" fn py_program_begin_wire(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let next_request_id = match parse_arg_u16(args, 0, "next_request_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let program_id = match parse_arg_u16(args, 1, "program_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let module = match parse_arg_bytes(args, 2, "module_bytes") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    with_session(next_request_id, |session| {
        let mut wire = [0u8; 96];
        let written = build_program_begin_wire_frame(session, program_id, &module, &mut wire)?;
        Ok(wire_result(&wire, written.len, session))
    })
}

unsafe extern "C" fn py_program_chunk_wire(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let next_request_id = match parse_arg_u16(args, 0, "next_request_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let program_id = match parse_arg_u16(args, 1, "program_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let offset = match parse_arg_u32(args, 2, "offset") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let chunk = match parse_arg_bytes(args, 3, "chunk") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    with_session(next_request_id, |session| {
        let mut wire = vec![0; chunk.len().saturating_add(64).max(128)];
        let written =
            build_program_chunk_wire_frame(session, program_id, offset, &chunk, &mut wire)?;
        Ok(wire_result(&wire, written.len, session))
    })
}

unsafe extern "C" fn py_program_end_wire(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let next_request_id = match parse_arg_u16(args, 0, "next_request_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let program_id = match parse_arg_u16(args, 1, "program_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    with_session(next_request_id, |session| {
        let mut wire = [0u8; 64];
        let written = build_program_end_wire_frame(session, program_id, &mut wire)?;
        Ok(wire_result(&wire, written.len, session))
    })
}

unsafe extern "C" fn py_store_program_wire(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let next_request_id = match parse_arg_u16(args, 0, "next_request_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let program_id = match parse_arg_u16(args, 1, "program_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let slot = match parse_arg_u8(args, 2, "slot") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let boot_policy = match parse_arg_u8(args, 3, "boot_policy") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    with_session(next_request_id, |session| {
        let mut wire = [0u8; 64];
        let written =
            build_store_program_wire_frame(session, program_id, slot, boot_policy, &mut wire)?;
        Ok(wire_result(&wire, written.len, session))
    })
}

unsafe extern "C" fn py_run_background_wire(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let next_request_id = match parse_arg_u16(args, 0, "next_request_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let program_id = match parse_arg_u16(args, 1, "program_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let instruction_budget = match parse_arg_u32(args, 2, "instruction_budget") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    with_session(next_request_id, |session| {
        let mut wire = [0u8; 96];
        let written =
            build_run_background_wire_frame(session, program_id, instruction_budget, &mut wire)?;
        Ok(wire_result(&wire, written.len, session))
    })
}

unsafe extern "C" fn py_run_wire(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let next_request_id = match parse_arg_u16(args, 0, "next_request_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let program_id = match parse_arg_u16(args, 1, "program_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let flags = match parse_arg_u8(args, 2, "flags") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let instruction_budget = match parse_arg_u32(args, 3, "instruction_budget") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let time_budget_ms = match parse_arg_u32(args, 4, "time_budget_ms") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    with_session(next_request_id, |session| {
        let mut wire = [0u8; 96];
        let written = build_run_wire_frame(
            session,
            program_id,
            flags,
            instruction_budget,
            time_budget_ms,
            &mut wire,
        )?;
        Ok(wire_result(&wire, written.len, session))
    })
}

unsafe extern "C" fn py_stop_wire(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let next_request_id = match parse_arg_u16(args, 0, "next_request_id") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };

    with_session(next_request_id, |session| {
        let mut wire = [0u8; 64];
        let written = build_stop_wire_frame(session, &mut wire)?;
        Ok(wire_result(&wire, written.len, session))
    })
}

unsafe extern "C" fn py_decode_response(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let wire = match parse_arg_bytes(args, 0, "wire response") {
        Some(value) => value,
        None => return ptr::null_mut(),
    };
    let mut raw = vec![0; wire.len().max(64)];
    let decoded = match decode_wire_response(&wire, &mut raw) {
        Ok(decoded) => decoded,
        Err(error) => return raise_core_error("decode_response", error),
    };
    decoded_response_to_py(&decoded)
}

unsafe extern "C" fn py_known_targets(_module: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    language_targets_to_py(&known_targets())
}

unsafe extern "C" fn py_detect_target(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let selector = match parse_arg_str(args, 0) {
        Some(value) => value,
        None => {
            set_error(
                type_error_class(),
                "detect_target() requires selector as str",
            );
            return ptr::null_mut();
        }
    };

    match core_detect_target(&selector) {
        Some(target) => language_target_to_py(&target),
        None => py_none(),
    }
}

unsafe extern "C" fn py_bluetooth_endpoint(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let endpoint = match parse_arg_str(args, 0) {
        Some(value) => value,
        None => {
            set_error(
                type_error_class(),
                "bluetooth_endpoint() requires endpoint as str",
            );
            return ptr::null_mut();
        }
    };

    match core_parse_bluetooth_endpoint(&endpoint) {
        Some(endpoint) => language_bluetooth_endpoint_to_py(&endpoint),
        None => py_none(),
    }
}

unsafe extern "C" fn py_bluetooth_endpoint_candidates(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let devices_arg = PyTuple_GetItem(args, 0);
    if devices_arg.is_null() {
        PyErr_Clear();
        let devices = core_discover_bluetooth_devices();
        return language_bluetooth_endpoint_candidates_to_py(
            &bluetooth_endpoint_candidates_from_devices(&devices),
        );
    }

    let devices = match language_bluetooth_devices_from_py(devices_arg) {
        Some(devices) => devices,
        None => return ptr::null_mut(),
    };

    language_bluetooth_endpoint_candidates_to_py(&bluetooth_endpoint_candidates_from_devices(
        &devices,
    ))
}

unsafe extern "C" fn py_bluetooth_devices(_module: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    language_bluetooth_devices_to_py(&core_discover_bluetooth_devices())
}

unsafe extern "C" fn py_esp_upload_options(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let selector = match parse_arg_str(args, 0) {
        Some(value) => value,
        None => {
            set_error(
                type_error_class(),
                "esp_upload_options() requires selector as str",
            );
            return ptr::null_mut();
        }
    };

    match esp_upload_options_for_target(&selector) {
        Some(options) => esp_upload_options_to_py(&options),
        None => py_none(),
    }
}

unsafe extern "C" fn py_pico_uf2_upload_options(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let selector = match parse_arg_str(args, 0) {
        Some(value) => value,
        None => {
            set_error(
                type_error_class(),
                "pico_uf2_upload_options() requires selector as str",
            );
            return ptr::null_mut();
        }
    };

    match pico_uf2_upload_options_for_target(&selector) {
        Some(options) => pico_uf2_upload_options_to_py(&options),
        None => py_none(),
    }
}

unsafe extern "C" fn py_discover_devices(_module: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    host_devices_to_py(&core_discover_devices())
}

unsafe extern "C" fn py_classify_devices(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let paths_arg = PyTuple_GetItem(args, 0);
    if paths_arg.is_null() {
        PyErr_Clear();
        set_error(
            type_error_class(),
            "classify_devices() requires a list of device paths",
        );
        return ptr::null_mut();
    }
    let paths = match vec_str_from_py(paths_arg) {
        Some(value) => value,
        None => {
            set_error(
                type_error_class(),
                "classify_devices() requires a list of device paths",
            );
            return ptr::null_mut();
        }
    };
    host_devices_to_py(&discover_devices_from_paths(paths))
}

unsafe extern "C" fn py_pico_uf2_mounts(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let roots_arg = PyTuple_GetItem(args, 0);
    if roots_arg.is_null() {
        PyErr_Clear();
        return strings_to_py(&core_discover_pico_bootsel_mounts());
    }
    let roots = match vec_str_from_py(roots_arg) {
        Some(value) => value,
        None => {
            set_error(
                type_error_class(),
                "pico_uf2_mounts() requires a list of mount roots",
            );
            return ptr::null_mut();
        }
    };
    strings_to_py(&discover_pico_bootsel_mounts_in_roots(roots))
}

fn with_session(
    next_request_id: u16,
    operation: impl FnOnce(&mut BoardVmLanguageSession) -> Result<PyObjectPtr, LanguageCoreError>,
) -> PyObjectPtr {
    let mut session = BoardVmLanguageSession::with_next_request_id(next_request_id);
    match operation(&mut session) {
        Ok(value) => value,
        Err(error) => unsafe { raise_core_error("Board VM language core", error) },
    }
}

unsafe fn wire_result(buffer: &[u8], len: usize, session: &BoardVmLanguageSession) -> PyObjectPtr {
    let result = PyDict_New();
    dict_set(result, "frame", bytes_to_py(&buffer[..len]));
    dict_set(
        result,
        "next_request_id",
        usize_to_py(session.next_request_id() as usize),
    );
    result
}

unsafe fn decoded_response_to_py(decoded: &DecodedLanguageResponse) -> PyObjectPtr {
    let dict = PyDict_New();
    dict_set(dict, "request_id", usize_to_py(decoded.request_id as usize));
    dict_set(
        dict,
        "message_type",
        str_to_py(message_type_name(decoded.message_type.0)),
    );
    dict_set(
        dict,
        "message_type_code",
        usize_to_py(decoded.message_type.0 as usize),
    );
    dict_set(dict, "flags", usize_to_py(decoded.flags as usize));
    dict_set(dict, "response", bool_to_py(decoded.is_response()));
    dict_set(dict, "error", bool_to_py(decoded.is_error_response()));
    dict_set(dict, "kind", str_to_py(decoded.body.kind()));
    dict_set(dict, "payload_len", usize_to_py(decoded.payload_len));
    dict_set(
        dict,
        "payload",
        response_body_to_py(&decoded.body, decoded.payload_len),
    );
    dict
}

unsafe fn response_body_to_py(
    body: &DecodedLanguageResponseBody,
    payload_len: usize,
) -> PyObjectPtr {
    let dict = PyDict_New();
    match body {
        DecodedLanguageResponseBody::HelloAck(ack) => {
            dict_set(
                dict,
                "selected_version",
                usize_to_py(ack.selected_version as usize),
            );
            dict_set(dict, "board_name", str_to_py(&ack.board_name));
            dict_set(dict, "runtime_name", str_to_py(&ack.runtime_name));
            dict_set(dict, "host_nonce", usize_to_py(ack.host_nonce as usize));
            dict_set(dict, "board_nonce", usize_to_py(ack.board_nonce as usize));
            dict_set(
                dict,
                "max_frame_payload",
                usize_to_py(ack.max_frame_payload as usize),
            );
        }
        DecodedLanguageResponseBody::CapsReport(report) => {
            dict_set(dict, "board_id", str_to_py(&report.board_id));
            dict_set(dict, "runtime_id", str_to_py(&report.runtime_id));
            dict_set(
                dict,
                "max_program_bytes",
                usize_to_py(report.max_program_bytes as usize),
            );
            dict_set(
                dict,
                "max_stack_values",
                usize_to_py(report.max_stack_values as usize),
            );
            dict_set(
                dict,
                "max_handles",
                usize_to_py(report.max_handles as usize),
            );
            dict_set(
                dict,
                "supports_store_program",
                bool_to_py(report.supports_store_program),
            );
            let capabilities = PyList_New(report.capabilities.len() as isize);
            for (index, capability) in report.capabilities.iter().enumerate() {
                let item = PyDict_New();
                dict_set(item, "id", usize_to_py(capability.id as usize));
                dict_set(item, "version", usize_to_py(capability.version as usize));
                dict_set(item, "flags", usize_to_py(capability.flags as usize));
                dict_set(item, "name", str_to_py(&capability.name));
                dict_set(
                    item,
                    "bytecode_callable",
                    bool_to_py(capability_bytecode_callable(capability.flags)),
                );
                dict_set(
                    item,
                    "protocol_feature",
                    bool_to_py(capability_protocol_feature(capability.flags)),
                );
                dict_set(
                    item,
                    "board_metadata",
                    bool_to_py(capability_board_metadata(capability.flags)),
                );
                dict_set(
                    item,
                    "flag_names",
                    capability_flag_names_to_py(capability.flags),
                );
                PyList_SetItem(capabilities, index as isize, item);
            }
            dict_set(dict, "capabilities", capabilities);
        }
        DecodedLanguageResponseBody::ProgramBegin(begin) => {
            dict_set(dict, "program_id", usize_to_py(begin.program_id as usize));
            dict_set(dict, "format", str_to_py(program_format_name(begin.format)));
            dict_set(dict, "total_len", usize_to_py(begin.total_len as usize));
            dict_set(
                dict,
                "program_crc32",
                usize_to_py(begin.program_crc32 as usize),
            );
        }
        DecodedLanguageResponseBody::ProgramChunk(chunk) => {
            dict_set(dict, "program_id", usize_to_py(chunk.program_id as usize));
            dict_set(dict, "offset", usize_to_py(chunk.offset as usize));
            dict_set(dict, "len", usize_to_py(chunk.len));
        }
        DecodedLanguageResponseBody::ProgramEnd(end) => {
            dict_set(dict, "program_id", usize_to_py(end.program_id as usize));
        }
        DecodedLanguageResponseBody::RunReport(report) => {
            dict_set(dict, "program_id", usize_to_py(report.program_id as usize));
            dict_set(dict, "status", str_to_py(run_status_name(report.status)));
            dict_set(
                dict,
                "status_code",
                usize_to_py(report.status.as_u8() as usize),
            );
            dict_set(
                dict,
                "instructions_executed",
                usize_to_py(report.instructions_executed as usize),
            );
            dict_set(dict, "elapsed_ms", usize_to_py(report.elapsed_ms as usize));
            dict_set(
                dict,
                "stack_depth",
                usize_to_py(report.stack_depth as usize),
            );
            dict_set(
                dict,
                "open_handles",
                usize_to_py(report.open_handles as usize),
            );
            dict_set(
                dict,
                "return_count",
                usize_to_py(report.return_count as usize),
            );
            dict_set(dict, "returns", language_values_to_py(&report.returns));
        }
        DecodedLanguageResponseBody::Error(error) => {
            dict_set(dict, "code", usize_to_py(error.code as usize));
            dict_set(dict, "request_id", usize_to_py(error.request_id as usize));
            dict_set(dict, "program_id", usize_to_py(error.program_id as usize));
            dict_set(
                dict,
                "bytecode_offset",
                usize_to_py(error.bytecode_offset as usize),
            );
            dict_set(dict, "message", str_to_py(&error.message));
        }
        DecodedLanguageResponseBody::Raw => {
            dict_set(dict, "payload_len", usize_to_py(payload_len));
        }
    }
    dict
}

unsafe fn capability_flag_names_to_py(flags: u16) -> PyObjectPtr {
    let mut names = [""; 3];
    let count = capability_flag_names(flags, &mut names);
    let list = PyList_New(count as isize);
    for (index, name) in names[..count].iter().enumerate() {
        PyList_SetItem(list, index as isize, str_to_py(name));
    }
    list
}

unsafe fn language_targets_to_py(targets: &[LanguageTargetInfo]) -> PyObjectPtr {
    let list = PyList_New(targets.len() as isize);
    for (index, target) in targets.iter().enumerate() {
        PyList_SetItem(list, index as isize, language_target_to_py(target));
    }
    list
}

unsafe fn language_target_to_py(target: &LanguageTargetInfo) -> PyObjectPtr {
    let dict = PyDict_New();
    dict_set(dict, "board_id", str_to_py(&target.board_id));
    dict_set(dict, "display_name", str_to_py(&target.display_name));
    dict_set(dict, "family", str_to_py(board_family_name(target.family)));
    dict_set(dict, "runtime_id", str_to_py(&target.runtime_id));
    dict_set(dict, "mcu", str_to_py(&target.mcu));
    dict_set(dict, "core", str_to_py(&target.core));
    dict_set(dict, "rust_target", str_to_py(&target.rust_target));
    dict_set(dict, "clock_hz", usize_to_py(target.clock_hz as usize));
    dict_set(
        dict,
        "operating_voltage_mv",
        usize_to_py(target.operating_voltage_mv as usize),
    );
    dict_set(
        dict,
        "onboard_led",
        language_onboard_led_to_py(target.onboard_led),
    );
    dict_set(
        dict,
        "digital_pin_count",
        usize_to_py(target.digital_pin_count),
    );
    dict_set(dict, "wireless", language_wireless_to_py(&target.wireless));
    dict_set(
        dict,
        "connection_options",
        language_connection_options_to_py(&target.connection_options),
    );
    let capabilities = PyList_New(target.capabilities.len() as isize);
    for (index, capability) in target.capabilities.iter().enumerate() {
        PyList_SetItem(capabilities, index as isize, str_to_py(capability));
    }
    dict_set(dict, "capabilities", capabilities);
    dict
}

unsafe fn language_wireless_to_py(interfaces: &[LanguageWirelessInterface]) -> PyObjectPtr {
    let list = PyList_New(interfaces.len() as isize);
    for (index, interface) in interfaces.iter().enumerate() {
        let dict = PyDict_New();
        dict_set(
            dict,
            "transport",
            str_to_py(wireless_transport_name(interface.transport)),
        );
        dict_set(dict, "chip", str_to_py(&interface.chip));
        dict_set(
            dict,
            "command_transport",
            bool_to_py(interface.command_transport),
        );
        dict_set(dict, "ota_update", bool_to_py(interface.ota_update));
        PyList_SetItem(list, index as isize, dict);
    }
    list
}

unsafe fn language_connection_options_to_py(options: &[LanguageConnectionOption]) -> PyObjectPtr {
    let list = PyList_New(options.len() as isize);
    for (index, option) in options.iter().enumerate() {
        let dict = PyDict_New();
        dict_set(
            dict,
            "transport",
            str_to_py(connection_transport_name(option.transport)),
        );
        dict_set(dict, "display_name", str_to_py(&option.display_name));
        dict_set(
            dict,
            "command_transport",
            bool_to_py(option.command_transport),
        );
        dict_set(dict, "ota_update", bool_to_py(option.ota_update));
        dict_set(dict, "requires", str_to_py(&option.requires));
        dict_set(
            dict,
            "endpoint_transport",
            str_to_py(host_endpoint_transport_name(option.endpoint_transport)),
        );
        dict_set(dict, "endpoint_scheme", str_to_py(&option.endpoint_scheme));
        dict_set(dict, "wire_protocol", str_to_py(&option.wire_protocol));
        PyList_SetItem(list, index as isize, dict);
    }
    list
}

unsafe fn language_bluetooth_endpoint_to_py(endpoint: &LanguageBluetoothEndpoint) -> PyObjectPtr {
    let dict = PyDict_New();
    dict_set(dict, "endpoint", str_to_py(&endpoint.endpoint));
    dict_set(
        dict,
        "transport",
        str_to_py(connection_transport_name(endpoint.transport)),
    );
    dict_set(
        dict,
        "endpoint_transport",
        str_to_py(host_endpoint_transport_name(endpoint.endpoint_transport)),
    );
    dict_set(
        dict,
        "endpoint_scheme",
        str_to_py(&endpoint.endpoint_scheme),
    );
    dict_set(dict, "device", str_to_py(&endpoint.device));
    dict_set(
        dict,
        "service_uuid",
        endpoint
            .service_uuid
            .as_ref()
            .map(|value| str_to_py(value))
            .unwrap_or_else(|| py_none()),
    );
    dict_set(
        dict,
        "write_characteristic_uuid",
        endpoint
            .write_characteristic_uuid
            .as_ref()
            .map(|value| str_to_py(value))
            .unwrap_or_else(|| py_none()),
    );
    dict_set(
        dict,
        "notify_characteristic_uuid",
        endpoint
            .notify_characteristic_uuid
            .as_ref()
            .map(|value| str_to_py(value))
            .unwrap_or_else(|| py_none()),
    );
    dict_set(
        dict,
        "channel",
        endpoint
            .channel
            .map(|value| usize_to_py(value as usize))
            .unwrap_or_else(|| py_none()),
    );
    dict
}

unsafe fn language_bluetooth_endpoint_candidates_to_py(
    candidates: &[LanguageBluetoothEndpointCandidate],
) -> PyObjectPtr {
    let list = PyList_New(candidates.len() as isize);
    for (index, candidate) in candidates.iter().enumerate() {
        let dict = PyDict_New();
        dict_set(
            dict,
            "endpoint",
            language_bluetooth_endpoint_to_py(&candidate.endpoint),
        );
        dict_set(dict, "device", str_to_py(&candidate.device));
        dict_set(dict, "display_name", str_to_py(&candidate.display_name));
        dict_set(dict, "paired", bool_to_py(candidate.paired));
        dict_set(
            dict,
            "requires_pairing",
            bool_to_py(candidate.requires_pairing),
        );
        PyList_SetItem(list, index as isize, dict);
    }
    list
}

unsafe fn language_bluetooth_devices_to_py(
    devices: &[LanguageBluetoothDiscoveredDevice],
) -> PyObjectPtr {
    let list = PyList_New(devices.len() as isize);
    for (index, device) in devices.iter().enumerate() {
        let dict = PyDict_New();
        dict_set(dict, "id", str_to_py(&device.id));
        dict_set(
            dict,
            "name",
            device
                .name
                .as_ref()
                .map(|value| str_to_py(value))
                .unwrap_or_else(|| py_none()),
        );
        dict_set(
            dict,
            "address",
            device
                .address
                .as_ref()
                .map(|value| str_to_py(value))
                .unwrap_or_else(|| py_none()),
        );
        dict_set(dict, "paired", bool_to_py(device.paired));
        dict_set(dict, "service_uuids", strings_to_py(&device.service_uuids));
        dict_set(
            dict,
            "characteristic_uuids",
            strings_to_py(&device.characteristic_uuids),
        );
        let channels = PyList_New(device.board_vm_rfcomm_channels.len() as isize);
        for (channel_index, channel) in device.board_vm_rfcomm_channels.iter().enumerate() {
            PyList_SetItem(
                channels,
                channel_index as isize,
                usize_to_py(*channel as usize),
            );
        }
        dict_set(dict, "board_vm_rfcomm_channels", channels);
        PyList_SetItem(list, index as isize, dict);
    }
    list
}

unsafe fn language_bluetooth_devices_from_py(
    devices: PyObjectPtr,
) -> Option<Vec<LanguageBluetoothDiscoveredDevice>> {
    let len = PyList_Size(devices);
    if len < 0 {
        PyErr_Clear();
        set_error(
            type_error_class(),
            "bluetooth_endpoint_candidates() requires devices as list",
        );
        return None;
    }

    let mut parsed = Vec::with_capacity(len as usize);
    for index in 0..len {
        let device = PyList_GetItem(devices, index);
        let Some(id) = py_dict_optional_str(device, "id") else {
            set_error(
                type_error_class(),
                "Bluetooth discovered device id must be a str",
            );
            return None;
        };

        parsed.push(LanguageBluetoothDiscoveredDevice {
            id,
            name: py_dict_optional_str(device, "name"),
            address: py_dict_optional_str(device, "address"),
            paired: py_dict_bool(device, "paired"),
            service_uuids: py_dict_vec_str(device, "service_uuids")?,
            characteristic_uuids: py_dict_vec_str(device, "characteristic_uuids")?,
            board_vm_rfcomm_channels: py_dict_vec_u8(device, "board_vm_rfcomm_channels")?,
        });
    }
    Some(parsed)
}

unsafe fn py_dict_get(dict: PyObjectPtr, key: &str) -> PyObjectPtr {
    let key = CString::new(key).unwrap();
    PyDict_GetItemString(dict, key.as_ptr())
}

unsafe fn py_dict_optional_str(dict: PyObjectPtr, key: &str) -> Option<String> {
    let value = py_dict_get(dict, key);
    if value.is_null() {
        None
    } else {
        str_from_py(value)
    }
}

unsafe fn py_dict_bool(dict: PyObjectPtr, key: &str) -> bool {
    let value = py_dict_get(dict, key);
    if value.is_null() {
        return false;
    }
    let truthy = PyObject_IsTrue(value);
    if truthy < 0 {
        PyErr_Clear();
        false
    } else {
        truthy != 0
    }
}

unsafe fn py_dict_vec_str(dict: PyObjectPtr, key: &str) -> Option<Vec<String>> {
    let value = py_dict_get(dict, key);
    if value.is_null() {
        Some(Vec::new())
    } else {
        vec_str_from_py(value).or_else(|| {
            set_error(
                type_error_class(),
                &format!("Bluetooth discovered device {key} must be a list of str"),
            );
            None
        })
    }
}

unsafe fn py_dict_vec_u8(dict: PyObjectPtr, key: &str) -> Option<Vec<u8>> {
    let value = py_dict_get(dict, key);
    if value.is_null() {
        return Some(Vec::new());
    }

    let len = PyList_Size(value);
    if len < 0 {
        PyErr_Clear();
        set_error(
            type_error_class(),
            &format!("Bluetooth discovered device {key} must be a list of int"),
        );
        return None;
    }

    let mut values = Vec::with_capacity(len as usize);
    for index in 0..len {
        let item = PyList_GetItem(value, index);
        PyErr_Clear();
        let channel = PyLong_AsLong(item);
        if channel == -1 && !PyErr_Occurred().is_null() {
            PyErr_Clear();
            set_error(
                type_error_class(),
                &format!("Bluetooth discovered device {key}[{index}] must be an int"),
            );
            return None;
        }
        if !(0..=u8::MAX as c_long).contains(&channel) {
            set_error(
                value_error_class(),
                &format!("Bluetooth discovered device {key}[{index}] must fit in u8"),
            );
            return None;
        }
        values.push(channel as u8);
    }
    Some(values)
}

unsafe fn esp_upload_options_to_py(options: &LanguageEspUploadOptions) -> PyObjectPtr {
    let dict = PyDict_New();
    dict_set(dict, "board_id", str_to_py(&options.board_id));
    dict_set(dict, "baud_rate", usize_to_py(options.baud_rate as usize));
    dict_set(dict, "timeout_ms", usize_to_py(options.timeout_ms as usize));
    dict_set(
        dict,
        "reset_into_bootloader",
        bool_to_py(options.reset_into_bootloader),
    );
    dict_set(dict, "offset", usize_to_py(options.offset as usize));
    dict_set(dict, "block_size", usize_to_py(options.block_size as usize));
    dict_set(
        dict,
        "flash_size",
        options
            .flash_size
            .map(|value| usize_to_py(value as usize))
            .unwrap_or_else(|| py_none()),
    );
    dict_set(dict, "verify_md5", bool_to_py(options.verify_md5));
    dict_set(
        dict,
        "stay_in_bootloader",
        bool_to_py(options.stay_in_bootloader),
    );
    dict
}

unsafe fn pico_uf2_upload_options_to_py(options: &LanguagePicoUf2UploadOptions) -> PyObjectPtr {
    let dict = PyDict_New();
    dict_set(dict, "board_id", str_to_py(&options.board_id));
    dict_set(dict, "command", str_to_py(&options.command));
    dict_set(dict, "volume_label", str_to_py(&options.volume_label));
    dict_set(dict, "image_extension", str_to_py(&options.image_extension));
    dict_set(
        dict,
        "auto_detect_mount",
        bool_to_py(options.auto_detect_mount),
    );
    dict
}

unsafe fn host_devices_to_py(devices: &[LanguageHostDevice]) -> PyObjectPtr {
    let list = PyList_New(devices.len() as isize);
    for (index, device) in devices.iter().enumerate() {
        PyList_SetItem(list, index as isize, host_device_to_py(device));
    }
    list
}

unsafe fn strings_to_py(strings: &[String]) -> PyObjectPtr {
    let list = PyList_New(strings.len() as isize);
    for (index, string) in strings.iter().enumerate() {
        PyList_SetItem(list, index as isize, str_to_py(string));
    }
    list
}

unsafe fn host_device_to_py(device: &LanguageHostDevice) -> PyObjectPtr {
    let dict = PyDict_New();
    dict_set(dict, "id", str_to_py(&device.id));
    dict_set(dict, "port", str_to_py(&device.port));
    dict_set(dict, "transport", str_to_py(&device.transport));
    dict_set(dict, "display_name", str_to_py(&device.display_name));
    dict_set(
        dict,
        "target",
        device
            .target
            .as_ref()
            .map(|target| language_target_to_py(target))
            .unwrap_or_else(|| py_none()),
    );
    dict_set(
        dict,
        "target_confidence",
        usize_to_py(device.target_confidence as usize),
    );
    dict_set(dict, "bootloader", bool_to_py(device.bootloader));
    let tags = PyList_New(device.tags.len() as isize);
    for (index, tag) in device.tags.iter().enumerate() {
        PyList_SetItem(tags, index as isize, str_to_py(tag));
    }
    dict_set(dict, "tags", tags);
    dict
}

unsafe fn language_onboard_led_to_py(led: Option<LanguageOnboardLed>) -> PyObjectPtr {
    let Some(led) = led else {
        return py_none();
    };
    let dict = PyDict_New();
    dict_set(dict, "kind", str_to_py(onboard_led_kind(led)));
    let pin = match led {
        LanguageOnboardLed::Gpio(pin) | LanguageOnboardLed::WirelessChipGpio(pin) => pin,
    };
    dict_set(dict, "pin", usize_to_py(pin as usize));
    dict
}

unsafe fn language_values_to_py(values: &[LanguageValue]) -> PyObjectPtr {
    let list = PyList_New(values.len() as isize);
    for (index, value) in values.iter().enumerate() {
        PyList_SetItem(list, index as isize, language_value_to_py(value));
    }
    list
}

unsafe fn language_value_to_py(value: &LanguageValue) -> PyObjectPtr {
    let dict = PyDict_New();
    dict_set(dict, "kind", str_to_py(value.kind()));
    let value_py = match value {
        LanguageValue::Unit => py_none(),
        LanguageValue::Bool(value) => bool_to_py(*value),
        LanguageValue::U8(value) => usize_to_py(*value as usize),
        LanguageValue::U16(value) => usize_to_py(*value as usize),
        LanguageValue::U32(value) => usize_to_py(*value as usize),
        LanguageValue::I16(value) => PyLong_FromLong(*value as c_long),
        LanguageValue::Handle(value) => usize_to_py(*value as usize),
        LanguageValue::Bytes(value) => bytes_to_py(value),
        LanguageValue::String(value) => str_to_py(value),
    };
    dict_set(dict, "value", value_py);
    dict
}

fn build_blink_module_value(
    pin: u8,
    high_ms: u16,
    low_ms: u16,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; BLINK_MODULE_LEN];
    let len = build_blink_module(
        BlinkProgram {
            pin,
            high_ms,
            low_ms,
            max_stack,
        },
        &mut module,
    )?;
    module.truncate(len);
    Ok(module)
}

fn build_time_now_module_value(max_stack: u8) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; TIME_NOW_MODULE_LEN];
    let len = build_time_now_module(TimeNowProgram { max_stack }, &mut module)?;
    module.truncate(len);
    Ok(module)
}

fn build_time_sleep_ms_module_value(
    duration_ms: u16,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; TIME_SLEEP_MS_MODULE_LEN];
    let len = build_time_sleep_ms_module(
        TimeSleepMsProgram {
            duration_ms,
            max_stack,
        },
        &mut module,
    )?;
    module.truncate(len);
    Ok(module)
}

fn build_raw_module_value(
    flags: u8,
    max_stack: u8,
    code: &[u8],
    const_pool: &[u8],
) -> Result<Vec<u8>, LanguageCoreError> {
    let module_len = raw_module_len(code.len() as u64, const_pool.len() as u64)?;
    let mut module = vec![0; module_len];
    let len = build_raw_module(flags, max_stack, code, const_pool, &mut module)?;
    module.truncate(len);
    Ok(module)
}

fn build_gpio_read_module_value(
    pin: u8,
    mode: u8,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; GPIO_READ_MODULE_LEN];
    let len = build_gpio_read_module(
        GpioReadProgram {
            pin,
            mode,
            max_stack,
        },
        &mut module,
    )?;
    module.truncate(len);
    Ok(module)
}

fn build_gpio_write_module_value(
    pin: u8,
    value: bool,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; GPIO_WRITE_MODULE_LEN];
    let len = build_gpio_write_module(
        GpioWriteProgram {
            pin,
            value,
            max_stack,
        },
        &mut module,
    )?;
    module.truncate(len);
    Ok(module)
}

fn build_gpio_open_module_value(
    pin: u8,
    mode: u8,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; GPIO_OPEN_MODULE_LEN];
    let len = build_gpio_open_module(
        GpioOpenProgram {
            pin,
            mode,
            max_stack,
        },
        &mut module,
    )?;
    module.truncate(len);
    Ok(module)
}

fn build_gpio_handle_read_module_value(max_stack: u8) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; GPIO_HANDLE_READ_MODULE_LEN];
    let len = build_gpio_handle_read_module(GpioHandleReadProgram { max_stack }, &mut module)?;
    module.truncate(len);
    Ok(module)
}

fn build_gpio_handle_write_module_value(
    value: bool,
    max_stack: u8,
) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; GPIO_HANDLE_WRITE_MODULE_LEN];
    let len =
        build_gpio_handle_write_module(GpioHandleWriteProgram { value, max_stack }, &mut module)?;
    module.truncate(len);
    Ok(module)
}

fn build_gpio_handle_close_module_value(max_stack: u8) -> Result<Vec<u8>, LanguageCoreError> {
    let mut module = vec![0; GPIO_HANDLE_CLOSE_MODULE_LEN];
    let len = build_gpio_handle_close_module(GpioHandleCloseProgram { max_stack }, &mut module)?;
    module.truncate(len);
    Ok(module)
}

unsafe fn parse_arg_bytes(args: PyObjectPtr, index: isize, name: &str) -> Option<Vec<u8>> {
    let arg = PyTuple_GetItem(args, index);
    if arg.is_null() {
        PyErr_Clear();
        set_error(type_error_class(), &format!("missing {name} argument"));
        return None;
    }
    match bytes_from_py(arg) {
        Some(value) => Some(value),
        None => {
            set_error(type_error_class(), &format!("{name} must be bytes"));
            None
        }
    }
}

unsafe fn parse_arg_u8(args: PyObjectPtr, index: isize, name: &str) -> Option<u8> {
    parse_arg_unsigned(args, index, name, u8::MAX as u64).map(|value| value as u8)
}

unsafe fn parse_arg_u16(args: PyObjectPtr, index: isize, name: &str) -> Option<u16> {
    parse_arg_unsigned(args, index, name, u16::MAX as u64).map(|value| value as u16)
}

unsafe fn parse_arg_u32(args: PyObjectPtr, index: isize, name: &str) -> Option<u32> {
    parse_arg_unsigned(args, index, name, u32::MAX as u64).map(|value| value as u32)
}

unsafe fn parse_arg_unsigned(args: PyObjectPtr, index: isize, name: &str, max: u64) -> Option<u64> {
    let arg = PyTuple_GetItem(args, index);
    if arg.is_null() {
        PyErr_Clear();
        set_error(type_error_class(), &format!("missing {name} argument"));
        return None;
    }
    PyErr_Clear();
    let value = PyLong_AsLong(arg);
    if value == -1 && !PyErr_Occurred().is_null() {
        PyErr_Clear();
        set_error(type_error_class(), &format!("{name} must be an integer"));
        return None;
    }
    if value < 0 {
        set_error(value_error_class(), &format!("{name} must be non-negative"));
        return None;
    }
    let value = value as u64;
    if value > max {
        set_error(
            value_error_class(),
            &format!("{name} must be less than or equal to {max}"),
        );
        return None;
    }
    Some(value)
}

unsafe fn dict_set(dict: PyObjectPtr, key: &str, value: PyObjectPtr) {
    let key = str_to_py(key);
    PyDict_SetItem(dict, key, value);
    Py_DecRef(key);
    Py_DecRef(value);
}

fn message_type_name(code: u8) -> &'static str {
    match code {
        0x01 => "hello",
        0x02 => "hello_ack",
        0x03 => "caps_query",
        0x04 => "caps_report",
        0x05 => "program_begin",
        0x06 => "program_chunk",
        0x07 => "program_end",
        0x08 => "run",
        0x09 => "run_report",
        0x0A => "stop",
        0x0B => "reset_vm",
        0x0C => "store_program",
        0x0D => "run_stored",
        0x0E => "read_state",
        0x0F => "state_report",
        0x10 => "subscribe",
        0x11 => "event",
        0x12 => "log",
        0x13 => "error",
        0x14 => "ping",
        0x15 => "pong",
        _ => "unknown",
    }
}

unsafe fn raise_core_error(context: &str, error: LanguageCoreError) -> PyObjectPtr {
    set_error(
        runtime_error_class(),
        &format!("{context} failed in Rust language core: {error:?}"),
    );
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn PyInit_board_vm_native() -> PyObjectPtr {
    let methods: &'static mut [PyMethodDef; 31] = Box::leak(Box::new([
        PyMethodDef {
            ml_name: b"hello_wire\0".as_ptr() as *const c_char,
            ml_meth: Some(py_hello_wire),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM HELLO wire frame in Rust.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"caps_query_wire\0".as_ptr() as *const c_char,
            ml_meth: Some(py_caps_query_wire),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM CAPS_QUERY wire frame in Rust.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"blink_module\0".as_ptr() as *const c_char,
            ml_meth: Some(py_blink_module),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM blink BVM module in Rust.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"gpio_read_module\0".as_ptr() as *const c_char,
            ml_meth: Some(py_gpio_read_module),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM GPIO read BVM module in Rust.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"gpio_write_module\0".as_ptr() as *const c_char,
            ml_meth: Some(py_gpio_write_module),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM GPIO write BVM module in Rust.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"gpio_open_module\0".as_ptr() as *const c_char,
            ml_meth: Some(py_gpio_open_module),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM GPIO open-handle BVM module in Rust.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"gpio_handle_read_module\0".as_ptr() as *const c_char,
            ml_meth: Some(py_gpio_handle_read_module),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM GPIO read-top-handle BVM module in Rust.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"gpio_handle_write_module\0".as_ptr() as *const c_char,
            ml_meth: Some(py_gpio_handle_write_module),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM GPIO write-top-handle BVM module in Rust.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"gpio_handle_close_module\0".as_ptr() as *const c_char,
            ml_meth: Some(py_gpio_handle_close_module),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM GPIO close-top-handle BVM module in Rust.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"time_now_module\0".as_ptr() as *const c_char,
            ml_meth: Some(py_time_now_module),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM time.now_ms BVM module in Rust.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"time_sleep_ms_module\0".as_ptr() as *const c_char,
            ml_meth: Some(py_time_sleep_ms_module),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM time.sleep_ms BVM module in Rust.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"raw_module\0".as_ptr() as *const c_char,
            ml_meth: Some(py_raw_module),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a generic BVM module from raw bytecode in Rust.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"program_begin_wire\0".as_ptr() as *const c_char,
            ml_meth: Some(py_program_begin_wire),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM PROGRAM_BEGIN wire frame in Rust.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"program_chunk_wire\0".as_ptr() as *const c_char,
            ml_meth: Some(py_program_chunk_wire),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM PROGRAM_CHUNK wire frame in Rust.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"program_end_wire\0".as_ptr() as *const c_char,
            ml_meth: Some(py_program_end_wire),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM PROGRAM_END wire frame in Rust.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"store_program_wire\0".as_ptr() as *const c_char,
            ml_meth: Some(py_store_program_wire),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM STORE_PROGRAM wire frame in Rust.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"run_background_wire\0".as_ptr() as *const c_char,
            ml_meth: Some(py_run_background_wire),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM background RUN wire frame in Rust.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"run_wire\0".as_ptr() as *const c_char,
            ml_meth: Some(py_run_wire),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a configurable Board VM RUN wire frame in Rust.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"stop_wire\0".as_ptr() as *const c_char,
            ml_meth: Some(py_stop_wire),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM STOP wire frame in Rust.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"decode_response\0".as_ptr() as *const c_char,
            ml_meth: Some(py_decode_response),
            ml_flags: METH_VARARGS,
            ml_doc: b"Decode a Board VM wire response in Rust.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"known_targets\0".as_ptr() as *const c_char,
            ml_meth: Some(py_known_targets),
            ml_flags: METH_VARARGS,
            ml_doc: b"Return known Board VM targets from Rust.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"detect_target\0".as_ptr() as *const c_char,
            ml_meth: Some(py_detect_target),
            ml_flags: METH_VARARGS,
            ml_doc: b"Resolve a Board VM target selector using Rust-owned aliases.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"bluetooth_endpoint\0".as_ptr() as *const c_char,
            ml_meth: Some(py_bluetooth_endpoint),
            ml_flags: METH_VARARGS,
            ml_doc: b"Parse a Board VM Bluetooth endpoint in Rust.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"bluetooth_endpoint_candidates\0".as_ptr() as *const c_char,
            ml_meth: Some(py_bluetooth_endpoint_candidates),
            ml_flags: METH_VARARGS,
            ml_doc: b"Plan Board VM Bluetooth endpoint candidates in Rust.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"bluetooth_devices\0".as_ptr() as *const c_char,
            ml_meth: Some(py_bluetooth_devices),
            ml_flags: METH_VARARGS,
            ml_doc: b"Discover host Bluetooth Board VM device metadata in Rust.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"esp_upload_options\0".as_ptr() as *const c_char,
            ml_meth: Some(py_esp_upload_options),
            ml_flags: METH_VARARGS,
            ml_doc: b"Return Rust-owned ESP ROM upload defaults for a target.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"pico_uf2_upload_options\0".as_ptr() as *const c_char,
            ml_meth: Some(py_pico_uf2_upload_options),
            ml_flags: METH_VARARGS,
            ml_doc: b"Return Rust-owned Pico UF2 upload defaults for a target.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"discover_devices\0".as_ptr() as *const c_char,
            ml_meth: Some(py_discover_devices),
            ml_flags: METH_VARARGS,
            ml_doc: b"Discover host Board VM device candidates in Rust.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"classify_devices\0".as_ptr() as *const c_char,
            ml_meth: Some(py_classify_devices),
            ml_flags: METH_VARARGS,
            ml_doc: b"Classify host Board VM device paths in Rust.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"pico_uf2_mounts\0".as_ptr() as *const c_char,
            ml_meth: Some(py_pico_uf2_mounts),
            ml_flags: METH_VARARGS,
            ml_doc: b"Discover Pico BOOTSEL UF2 mount candidates in Rust.\0".as_ptr()
                as *const c_char,
        },
        method_def_sentinel(),
    ]));

    let def: &'static mut PyModuleDef = Box::leak(Box::new(PyModuleDef {
        m_base: PyModuleDef_Base {
            ob_base: [0u8; std::mem::size_of::<usize>() * 2],
            m_init: None,
            m_index: 0,
            m_copy: ptr::null_mut(),
        },
        m_name: b"board_vm_native\0".as_ptr() as *const c_char,
        m_doc: b"Rust-owned Board VM protocol framing and decoding for Python sugar.\0".as_ptr()
            as *const c_char,
        m_size: -1,
        m_methods: methods.as_mut_ptr(),
        m_slots: ptr::null_mut(),
        m_traverse: ptr::null_mut(),
        m_clear: ptr::null_mut(),
        m_free: ptr::null_mut(),
    }));

    PyModule_Create2(def as *mut PyModuleDef, PYTHON_API_VERSION)
}
