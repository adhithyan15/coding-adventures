use std::ffi::{c_char, c_long};
use std::ptr;

use board_vm_host::{
    BlinkProgram, GpioReadProgram, TimeNowProgram, BLINK_MODULE_LEN, GPIO_READ_MODULE_LEN,
    TIME_NOW_MODULE_LEN,
};
use board_vm_language_core::{
    build_blink_module, build_caps_query_wire_frame, build_gpio_read_module,
    build_hello_wire_frame, build_program_begin_wire_frame, build_program_chunk_wire_frame,
    build_program_end_wire_frame, build_run_background_wire_frame, build_stop_wire_frame,
    build_time_now_module, capability_board_metadata, capability_bytecode_callable,
    capability_flag_names, capability_protocol_feature, decode_wire_response, program_format_name,
    run_status_name, BoardVmLanguageSession, DecodedLanguageResponse, DecodedLanguageResponseBody,
    LanguageCoreError, LanguageValue,
};
use python_bridge::*;

#[allow(non_snake_case)]
extern "C" {
    fn PyLong_AsLong(obj: PyObjectPtr) -> c_long;
    fn PyErr_Occurred() -> PyObjectPtr;
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
    let methods: &'static mut [PyMethodDef; 12] = Box::leak(Box::new([
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
            ml_name: b"time_now_module\0".as_ptr() as *const c_char,
            ml_meth: Some(py_time_now_module),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM time.now_ms BVM module in Rust.\0".as_ptr() as *const c_char,
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
            ml_name: b"run_background_wire\0".as_ptr() as *const c_char,
            ml_meth: Some(py_run_background_wire),
            ml_flags: METH_VARARGS,
            ml_doc: b"Build a Board VM background RUN wire frame in Rust.\0".as_ptr()
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
