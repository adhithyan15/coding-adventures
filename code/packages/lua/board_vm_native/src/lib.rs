#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, CString};
use std::ptr;

use board_vm_host::{
    BlinkProgram, BLINK_MODULE_LEN, DEFAULT_INSTRUCTION_BUDGET, DEFAULT_PROGRAM_ID,
    DEFAULT_RUN_FLAGS,
};
use board_vm_language_core::{
    build_blink_module, build_caps_query_wire_frame, build_hello_wire_frame,
    build_program_begin_wire_frame, build_program_chunk_wire_frame, build_program_end_wire_frame,
    build_run_wire_frame, BoardVmLanguageSession, BuiltWireFrame, LanguageCoreError,
};
use lua_bridge::{
    get_str, luaL_Reg, luaL_checkinteger, lua_Integer, lua_State, lua_gettop, lua_pushinteger,
    lua_pushlstring, lua_setfield, lua_tolstring, raise_error, register_lib,
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

unsafe fn set_bytes(L: *mut lua_State, key: &str, value: &[u8]) {
    let key = CString::new(key).unwrap();
    push_bytes(L, value);
    lua_setfield(L, -2, key.as_ptr());
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

struct FuncTable([luaL_Reg; 9]);
unsafe impl Sync for FuncTable {}

static FUNCS: FuncTable = FuncTable([
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
