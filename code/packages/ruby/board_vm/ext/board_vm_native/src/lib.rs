use std::ffi::{c_char, c_int, c_void};
use std::ptr;
use std::slice;

use board_vm_host::{BlinkProgram, BLINK_MODULE_LEN};
use board_vm_language_core::{
    build_blink_module, build_caps_query_wire_frame, build_hello_wire_frame,
    build_program_begin_wire_frame, build_program_chunk_wire_frame, build_program_end_wire_frame,
    build_run_background_wire_frame, BoardVmLanguageSession, LanguageCoreError,
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
        "blink_upload_run_frames",
        session_blink_upload_run_frames as *const c_void,
        -1,
    );
}
