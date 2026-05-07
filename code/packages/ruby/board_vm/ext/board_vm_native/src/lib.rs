use std::ffi::{c_char, c_int, c_long, c_void};
use std::ptr;
use std::slice;

use board_vm_host::{
    BlinkProgram, GpioReadProgram, GpioWriteProgram, TimeNowProgram, BLINK_MODULE_LEN,
    GPIO_READ_MODULE_LEN, GPIO_WRITE_MODULE_LEN, TIME_NOW_MODULE_LEN,
};
use board_vm_language_core::{
    build_blink_module, build_caps_query_wire_frame, build_gpio_read_module,
    build_gpio_write_module, build_hello_wire_frame, build_program_begin_wire_frame,
    build_program_chunk_wire_frame, build_program_end_wire_frame, build_run_background_wire_frame,
    build_stop_wire_frame, build_time_now_module, capability_board_metadata,
    capability_bytecode_callable, capability_flag_names, capability_protocol_feature,
    decode_wire_response, program_format_name, run_status_name, BoardVmLanguageSession,
    DecodedLanguageResponse, DecodedLanguageResponseBody, LanguageCoreError, LanguageValue,
};
use ruby_bridge::VALUE;

struct RubyBoardVmSession {
    inner: BoardVmLanguageSession,
}

unsafe extern "C" fn session_alloc(klass: VALUE) -> VALUE {
    ruby_bridge::wrap_data(
        klass,
        RubyBoardVmSession {
            inner: BoardVmLanguageSession::new(),
        },
    )
}

extern "C" fn session_initialize(self_val: VALUE) -> VALUE {
    self_val
}

extern "C" fn session_next_request_id(self_val: VALUE) -> VALUE {
    let session = unsafe { ruby_bridge::unwrap_data::<RubyBoardVmSession>(self_val) };
    ruby_bridge::usize_to_rb(session.inner.next_request_id() as usize)
}

extern "C" fn session_hello_wire(
    self_val: VALUE,
    host_name_val: VALUE,
    host_nonce_val: VALUE,
) -> VALUE {
    let host_name = ruby_bridge::str_from_rb(host_name_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("host_name must be a Ruby String"));
    let host_nonce = rb_u32(host_nonce_val, "host_nonce");

    with_session_mut(self_val, |session| {
        let mut wire = vec![0; host_name.len().saturating_add(64).max(128)];
        let written =
            build_hello_wire_frame(&mut session.inner, &host_name, host_nonce, &mut wire)?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_caps_query_wire(self_val: VALUE) -> VALUE {
    with_session_mut(self_val, |session| {
        let mut wire = [0u8; 64];
        let written = build_caps_query_wire_frame(&mut session.inner, &mut wire)?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_blink_module(
    _self_val: VALUE,
    pin_val: VALUE,
    high_ms_val: VALUE,
    low_ms_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let pin = rb_u8(pin_val, "pin");
    let high_ms = rb_u16(high_ms_val, "high_ms");
    let low_ms = rb_u16(low_ms_val, "low_ms");
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_blink_module_value(pin, high_ms, low_ms, max_stack)
        .unwrap_or_else(|error| raise_core_error("blink_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_gpio_read_module(
    _self_val: VALUE,
    pin_val: VALUE,
    mode_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let pin = rb_u8(pin_val, "pin");
    let mode = rb_u8(mode_val, "mode");
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_gpio_read_module_value(pin, mode, max_stack)
        .unwrap_or_else(|error| raise_core_error("gpio_read_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_gpio_write_module(
    _self_val: VALUE,
    pin_val: VALUE,
    value_val: VALUE,
    max_stack_val: VALUE,
) -> VALUE {
    let pin = rb_u8(pin_val, "pin");
    let value = rb_u8(value_val, "value") != 0;
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_gpio_write_module_value(pin, value, max_stack)
        .unwrap_or_else(|error| raise_core_error("gpio_write_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_time_now_module(_self_val: VALUE, max_stack_val: VALUE) -> VALUE {
    let max_stack = rb_u8(max_stack_val, "max_stack");

    let module = build_time_now_module_value(max_stack)
        .unwrap_or_else(|error| raise_core_error("time_now_module", error));
    ruby_bridge::bytes_to_rb(&module)
}

extern "C" fn session_program_begin_wire(
    self_val: VALUE,
    program_id_val: VALUE,
    module_val: VALUE,
) -> VALUE {
    let program_id = rb_u16(program_id_val, "program_id");
    let module = ruby_bridge::bytes_from_rb(module_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("module must be a Ruby binary String"));

    with_session_mut(self_val, |session| {
        let mut wire = [0u8; 96];
        let written =
            build_program_begin_wire_frame(&mut session.inner, program_id, &module, &mut wire)?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_program_chunk_wire(
    self_val: VALUE,
    program_id_val: VALUE,
    offset_val: VALUE,
    chunk_val: VALUE,
) -> VALUE {
    let program_id = rb_u16(program_id_val, "program_id");
    let offset = rb_u32(offset_val, "offset");
    let chunk = ruby_bridge::bytes_from_rb(chunk_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("chunk must be a Ruby binary String"));

    with_session_mut(self_val, |session| {
        let mut wire = vec![0; chunk.len().saturating_add(64).max(128)];
        let written = build_program_chunk_wire_frame(
            &mut session.inner,
            program_id,
            offset,
            &chunk,
            &mut wire,
        )?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_program_end_wire(self_val: VALUE, program_id_val: VALUE) -> VALUE {
    let program_id = rb_u16(program_id_val, "program_id");

    with_session_mut(self_val, |session| {
        let mut wire = [0u8; 64];
        let written = build_program_end_wire_frame(&mut session.inner, program_id, &mut wire)?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_run_background_wire(
    self_val: VALUE,
    program_id_val: VALUE,
    instruction_budget_val: VALUE,
) -> VALUE {
    let program_id = rb_u16(program_id_val, "program_id");
    let instruction_budget = rb_u32(instruction_budget_val, "instruction_budget");

    with_session_mut(self_val, |session| {
        let mut wire = [0u8; 96];
        let written = build_run_background_wire_frame(
            &mut session.inner,
            program_id,
            instruction_budget,
            &mut wire,
        )?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_stop_wire(self_val: VALUE) -> VALUE {
    with_session_mut(self_val, |session| {
        let mut wire = [0u8; 64];
        let written = build_stop_wire_frame(&mut session.inner, &mut wire)?;
        Ok(bytes_result(&wire, written.len))
    })
}

extern "C" fn session_blink_upload_run_frames(
    argc: c_int,
    argv: *const VALUE,
    self_val: VALUE,
) -> VALUE {
    if argc != 6 {
        ruby_bridge::raise_arg_error(
            "blink_upload_run_frames expects program_id, instruction_budget, pin, high_ms, low_ms, max_stack",
        );
    }
    let args = unsafe { slice::from_raw_parts(argv, argc as usize) };
    let program_id = rb_u16(args[0], "program_id");
    let instruction_budget = rb_u32(args[1], "instruction_budget");
    let pin = rb_u8(args[2], "pin");
    let high_ms = rb_u16(args[3], "high_ms");
    let low_ms = rb_u16(args[4], "low_ms");
    let max_stack = rb_u8(args[5], "max_stack");

    let module = build_blink_module_value(pin, high_ms, low_ms, max_stack)
        .unwrap_or_else(|error| raise_core_error("blink_upload_run_frames", error));

    with_session_mut(self_val, |session| {
        let frames = ruby_bridge::array_new();

        let mut begin_wire = [0u8; 96];
        let begin = build_program_begin_wire_frame(
            &mut session.inner,
            program_id,
            &module,
            &mut begin_wire,
        )?;
        ruby_bridge::array_push(frames, bytes_result(&begin_wire, begin.len));

        let mut chunk_wire = vec![0; module.len().saturating_add(64).max(128)];
        let chunk = build_program_chunk_wire_frame(
            &mut session.inner,
            program_id,
            0,
            &module,
            &mut chunk_wire,
        )?;
        ruby_bridge::array_push(frames, bytes_result(&chunk_wire, chunk.len));

        let mut end_wire = [0u8; 64];
        let end = build_program_end_wire_frame(&mut session.inner, program_id, &mut end_wire)?;
        ruby_bridge::array_push(frames, bytes_result(&end_wire, end.len));

        let mut run_wire = [0u8; 96];
        let run = build_run_background_wire_frame(
            &mut session.inner,
            program_id,
            instruction_budget,
            &mut run_wire,
        )?;
        ruby_bridge::array_push(frames, bytes_result(&run_wire, run.len));

        Ok(frames)
    })
}

extern "C" fn session_decode_response(_self_val: VALUE, wire_val: VALUE) -> VALUE {
    let wire = ruby_bridge::bytes_from_rb(wire_val)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("wire response must be a binary String"));
    let mut raw = vec![0; wire.len().max(64)];
    let decoded = decode_wire_response(&wire, &mut raw)
        .unwrap_or_else(|error| raise_core_error("decode_response", error));
    decoded_response_to_rb(&decoded)
}

fn with_session_mut(
    self_val: VALUE,
    operation: impl FnOnce(&mut RubyBoardVmSession) -> Result<VALUE, LanguageCoreError>,
) -> VALUE {
    let session = unsafe { ruby_bridge::unwrap_data_mut::<RubyBoardVmSession>(self_val) };
    match operation(session) {
        Ok(value) => value,
        Err(error) => raise_core_error("BoardVM::Native::Session", error),
    }
}

fn bytes_result(buffer: &[u8], len: usize) -> VALUE {
    ruby_bridge::bytes_to_rb(&buffer[..len])
}

fn decoded_response_to_rb(decoded: &DecodedLanguageResponse) -> VALUE {
    let hash = ruby_bridge::hash_new();
    hash_set(hash, "request_id", rb_usize(decoded.request_id));
    hash_set(
        hash,
        "message_type",
        ruby_bridge::str_to_rb(message_type_name(decoded.message_type.0)),
    );
    hash_set(hash, "message_type_code", rb_usize(decoded.message_type.0));
    hash_set(hash, "flags", rb_usize(decoded.flags));
    hash_set(
        hash,
        "response",
        ruby_bridge::bool_to_rb(decoded.is_response()),
    );
    hash_set(
        hash,
        "error",
        ruby_bridge::bool_to_rb(decoded.is_error_response()),
    );
    hash_set(hash, "kind", ruby_bridge::str_to_rb(decoded.body.kind()));
    hash_set(hash, "payload_len", rb_usize(decoded.payload_len));
    hash_set(
        hash,
        "payload",
        response_body_to_rb(&decoded.body, decoded.payload_len),
    );
    hash
}

fn response_body_to_rb(body: &DecodedLanguageResponseBody, payload_len: usize) -> VALUE {
    let hash = ruby_bridge::hash_new();
    match body {
        DecodedLanguageResponseBody::HelloAck(ack) => {
            hash_set(hash, "selected_version", rb_usize(ack.selected_version));
            hash_set(hash, "board_name", ruby_bridge::str_to_rb(&ack.board_name));
            hash_set(
                hash,
                "runtime_name",
                ruby_bridge::str_to_rb(&ack.runtime_name),
            );
            hash_set(hash, "host_nonce", rb_usize(ack.host_nonce));
            hash_set(hash, "board_nonce", rb_usize(ack.board_nonce));
            hash_set(hash, "max_frame_payload", rb_usize(ack.max_frame_payload));
        }
        DecodedLanguageResponseBody::CapsReport(report) => {
            hash_set(hash, "board_id", ruby_bridge::str_to_rb(&report.board_id));
            hash_set(
                hash,
                "runtime_id",
                ruby_bridge::str_to_rb(&report.runtime_id),
            );
            hash_set(
                hash,
                "max_program_bytes",
                rb_usize(report.max_program_bytes),
            );
            hash_set(hash, "max_stack_values", rb_usize(report.max_stack_values));
            hash_set(hash, "max_handles", rb_usize(report.max_handles));
            hash_set(
                hash,
                "supports_store_program",
                ruby_bridge::bool_to_rb(report.supports_store_program),
            );
            let capabilities = ruby_bridge::array_new();
            for capability in &report.capabilities {
                let item = ruby_bridge::hash_new();
                hash_set(item, "id", rb_usize(capability.id));
                hash_set(item, "version", rb_usize(capability.version));
                hash_set(item, "flags", rb_usize(capability.flags));
                hash_set(item, "name", ruby_bridge::str_to_rb(&capability.name));
                hash_set(
                    item,
                    "bytecode_callable",
                    ruby_bridge::bool_to_rb(capability_bytecode_callable(capability.flags)),
                );
                hash_set(
                    item,
                    "protocol_feature",
                    ruby_bridge::bool_to_rb(capability_protocol_feature(capability.flags)),
                );
                hash_set(
                    item,
                    "board_metadata",
                    ruby_bridge::bool_to_rb(capability_board_metadata(capability.flags)),
                );
                hash_set(
                    item,
                    "flag_names",
                    capability_flag_names_to_rb(capability.flags),
                );
                ruby_bridge::array_push(capabilities, item);
            }
            hash_set(hash, "capabilities", capabilities);
        }
        DecodedLanguageResponseBody::ProgramBegin(begin) => {
            hash_set(hash, "program_id", rb_usize(begin.program_id));
            hash_set(
                hash,
                "format",
                ruby_bridge::str_to_rb(program_format_name(begin.format)),
            );
            hash_set(hash, "total_len", rb_usize(begin.total_len));
            hash_set(hash, "program_crc32", rb_usize(begin.program_crc32));
        }
        DecodedLanguageResponseBody::ProgramChunk(chunk) => {
            hash_set(hash, "program_id", rb_usize(chunk.program_id));
            hash_set(hash, "offset", rb_usize(chunk.offset));
            hash_set(hash, "len", rb_usize(chunk.len));
        }
        DecodedLanguageResponseBody::ProgramEnd(end) => {
            hash_set(hash, "program_id", rb_usize(end.program_id));
        }
        DecodedLanguageResponseBody::RunReport(report) => {
            hash_set(hash, "program_id", rb_usize(report.program_id));
            hash_set(
                hash,
                "status",
                ruby_bridge::str_to_rb(run_status_name(report.status)),
            );
            hash_set(hash, "status_code", rb_usize(report.status.as_u8()));
            hash_set(
                hash,
                "instructions_executed",
                rb_usize(report.instructions_executed),
            );
            hash_set(hash, "elapsed_ms", rb_usize(report.elapsed_ms));
            hash_set(hash, "stack_depth", rb_usize(report.stack_depth));
            hash_set(hash, "open_handles", rb_usize(report.open_handles));
            hash_set(hash, "return_count", rb_usize(report.return_count));
            hash_set(hash, "returns", language_values_to_rb(&report.returns));
        }
        DecodedLanguageResponseBody::Error(error) => {
            hash_set(hash, "code", rb_usize(error.code));
            hash_set(hash, "request_id", rb_usize(error.request_id));
            hash_set(hash, "program_id", rb_usize(error.program_id));
            hash_set(hash, "bytecode_offset", rb_usize(error.bytecode_offset));
            hash_set(hash, "message", ruby_bridge::str_to_rb(&error.message));
        }
        DecodedLanguageResponseBody::Raw => {
            hash_set(hash, "payload_len", rb_usize(payload_len));
        }
    }
    hash
}

fn capability_flag_names_to_rb(flags: u16) -> VALUE {
    let mut names = [""; 3];
    let count = capability_flag_names(flags, &mut names);
    let array = ruby_bridge::array_new();
    for name in &names[..count] {
        ruby_bridge::array_push(array, ruby_bridge::str_to_rb(name));
    }
    array
}

fn language_values_to_rb(values: &[LanguageValue]) -> VALUE {
    let array = ruby_bridge::array_new();
    for value in values {
        ruby_bridge::array_push(array, language_value_to_rb(value));
    }
    array
}

fn language_value_to_rb(value: &LanguageValue) -> VALUE {
    let hash = ruby_bridge::hash_new();
    hash_set(hash, "kind", ruby_bridge::str_to_rb(value.kind()));
    let value_rb = match value {
        LanguageValue::Unit => ruby_bridge::QNIL,
        LanguageValue::Bool(value) => ruby_bridge::bool_to_rb(*value),
        LanguageValue::U8(value) => rb_usize(*value),
        LanguageValue::U16(value) => rb_usize(*value),
        LanguageValue::U32(value) => rb_usize(*value),
        LanguageValue::I16(value) => rb_i64(*value as i64),
        LanguageValue::Handle(value) => rb_usize(*value),
        LanguageValue::Bytes(value) => ruby_bridge::bytes_to_rb(value),
        LanguageValue::String(value) => ruby_bridge::str_to_rb(value),
    };
    hash_set(hash, "value", value_rb);
    hash
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

fn hash_set(hash: VALUE, key: &str, value: VALUE) {
    ruby_bridge::hash_aset(hash, ruby_bridge::str_to_rb(key), value);
}

fn rb_usize(value: impl TryInto<usize>) -> VALUE {
    ruby_bridge::usize_to_rb(value.try_into().unwrap_or(usize::MAX))
}

fn rb_i64(value: i64) -> VALUE {
    unsafe { ruby_bridge::rb_int2inum(value as c_long) }
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

fn rb_u8(value: VALUE, name: &str) -> u8 {
    let value = rb_nonnegative_integer(value, name);
    if value > u8::MAX as u64 {
        ruby_bridge::raise_arg_error(&format!("{name} must fit in u8"));
    }
    value as u8
}

fn rb_u16(value: VALUE, name: &str) -> u16 {
    let value = rb_nonnegative_integer(value, name);
    if value > u16::MAX as u64 {
        ruby_bridge::raise_arg_error(&format!("{name} must fit in u16"));
    }
    value as u16
}

fn rb_u32(value: VALUE, name: &str) -> u32 {
    let value = rb_nonnegative_integer(value, name);
    if value > u32::MAX as u64 {
        ruby_bridge::raise_arg_error(&format!("{name} must fit in u32"));
    }
    value as u32
}

fn rb_nonnegative_integer(value: VALUE, name: &str) -> u64 {
    let to_s = unsafe { ruby_bridge::rb_intern(b"to_s\0".as_ptr() as *const c_char) };
    let string_value = unsafe { ruby_bridge::rb_funcallv(value, to_s, 0, ptr::null()) };
    let text = ruby_bridge::str_from_rb(string_value)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error(&format!("{name} must be an integer")));
    text.parse::<u64>()
        .unwrap_or_else(|_| ruby_bridge::raise_arg_error(&format!("{name} must be non-negative")))
}

fn raise_core_error(context: &str, error: LanguageCoreError) -> ! {
    ruby_bridge::raise_runtime_error(&format!(
        "{context} failed in Rust language core: {error:?}"
    ))
}

#[no_mangle]
pub extern "C" fn Init_board_vm_native() {
    let coding_adventures = ruby_bridge::define_module("CodingAdventures");
    let board_vm = ruby_bridge::define_module_under(coding_adventures, "BoardVM");
    let native = ruby_bridge::define_module_under(board_vm, "Native");
    let session_class =
        ruby_bridge::define_class_under(native, "Session", ruby_bridge::object_class());

    ruby_bridge::define_alloc_func(session_class, session_alloc);
    ruby_bridge::define_method_raw(
        session_class,
        "initialize",
        session_initialize as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "next_request_id",
        session_next_request_id as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "hello_wire",
        session_hello_wire as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "caps_query_wire",
        session_caps_query_wire as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "blink_module",
        session_blink_module as *const c_void,
        4,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "gpio_read_module",
        session_gpio_read_module as *const c_void,
        3,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "gpio_write_module",
        session_gpio_write_module as *const c_void,
        3,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "time_now_module",
        session_time_now_module as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "program_begin_wire",
        session_program_begin_wire as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "program_chunk_wire",
        session_program_chunk_wire as *const c_void,
        3,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "program_end_wire",
        session_program_end_wire as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "run_background_wire",
        session_run_background_wire as *const c_void,
        2,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "stop_wire",
        session_stop_wire as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "blink_upload_run_frames",
        session_blink_upload_run_frames as *const c_void,
        -1,
    );
    ruby_bridge::define_method_raw(
        session_class,
        "decode_response",
        session_decode_response as *const c_void,
        1,
    );
}
