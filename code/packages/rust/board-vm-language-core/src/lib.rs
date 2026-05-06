//! Rust-owned Board VM host protocol boundary for language frontends.
//!
//! Ruby, Python, Lua, Java, and similar frontends should be thin syntax layers
//! over this crate. The binary protocol, request ids, BVM module shape, COBS
//! framing, and CRC checks stay in Rust, where the board firmware and host CLI
//! already share the same implementation.

use std::cell::{Cell, RefCell};
use std::ffi::CString;
use std::panic::{self, AssertUnwindSafe};
use std::ptr;
use std::slice;
use std::str;

use board_vm_host::{
    write_blink_module, BlinkProgram, HostError, HostSession, BLINK_MODULE_LEN,
    DEFAULT_INSTRUCTION_BUDGET, DEFAULT_PROGRAM_ID,
};
use board_vm_protocol::{decode_frame, decode_wire_frame, encode_wire_frame, Frame, ProtocolError};

pub const LANGUAGE_CORE_VERSION_MAJOR: u16 = 0;
pub const LANGUAGE_CORE_VERSION_MINOR: u16 = 1;
pub const LANGUAGE_CORE_VERSION_PATCH: u16 = 0;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardVmLanguageStatusCode {
    Ok = 0,
    NullPointer = 1,
    InvalidUtf8 = 2,
    ValueTooLarge = 3,
    OutputTooSmall = 4,
    ProtocolError = 5,
    HostError = 6,
    Panic = 7,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardVmLanguageStatus {
    pub code: u32,
    pub len: u64,
    pub request_id: u16,
    pub message_type: u8,
    pub flags: u8,
    pub payload_offset: u64,
    pub payload_len: u64,
}

impl BoardVmLanguageStatus {
    pub const fn ok() -> Self {
        Self {
            code: BoardVmLanguageStatusCode::Ok as u32,
            len: 0,
            request_id: 0,
            message_type: 0,
            flags: 0,
            payload_offset: 0,
            payload_len: 0,
        }
    }

    pub const fn err(code: BoardVmLanguageStatusCode) -> Self {
        Self {
            code: code as u32,
            len: 0,
            request_id: 0,
            message_type: 0,
            flags: 0,
            payload_offset: 0,
            payload_len: 0,
        }
    }

    fn written(request_id: u16, len: usize) -> Self {
        Self {
            len: len as u64,
            request_id,
            ..Self::ok()
        }
    }

    fn decoded(frame: &Frame<'_>, raw_base: *const u8, raw_len: usize) -> Self {
        let payload_offset = frame.payload.as_ptr() as usize - raw_base as usize;
        Self {
            len: raw_len as u64,
            request_id: frame.request_id,
            message_type: frame.message_type.0,
            flags: frame.flags,
            payload_offset: payload_offset as u64,
            payload_len: frame.payload.len() as u64,
            code: BoardVmLanguageStatusCode::Ok as u32,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardVmLanguageSession {
    next_request_id: u16,
}

impl BoardVmLanguageSession {
    pub const fn new() -> Self {
        Self { next_request_id: 1 }
    }

    pub const fn with_next_request_id(next_request_id: u16) -> Self {
        Self {
            next_request_id: if next_request_id == 0 {
                1
            } else {
                next_request_id
            },
        }
    }

    pub const fn next_request_id(&self) -> u16 {
        self.next_request_id
    }

    fn host_session(&self) -> HostSession {
        HostSession::with_next_request_id(self.next_request_id)
    }

    fn update_from_host_session(&mut self, host: &HostSession) {
        self.next_request_id = host.next_request_id();
    }
}

impl Default for BoardVmLanguageSession {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltWireFrame {
    pub request_id: u16,
    pub len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageCoreError {
    NullPointer(&'static str),
    InvalidUtf8,
    ValueTooLarge,
    OutputTooSmall,
    Protocol(ProtocolError),
    Host(HostError),
}

impl From<ProtocolError> for LanguageCoreError {
    fn from(value: ProtocolError) -> Self {
        match value {
            ProtocolError::OutputTooSmall | ProtocolError::PayloadTooLarge => Self::OutputTooSmall,
            other => Self::Protocol(other),
        }
    }
}

impl From<HostError> for LanguageCoreError {
    fn from(value: HostError) -> Self {
        match value {
            HostError::OutputTooSmall => Self::OutputTooSmall,
            other => Self::Host(other),
        }
    }
}

pub fn build_hello_wire_frame(
    session: &mut BoardVmLanguageSession,
    host_name: &str,
    host_nonce: u32,
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    let mut payload = vec![0; host_name.len().saturating_add(16)];
    let mut raw = vec![0; host_name.len().saturating_add(32)];
    let mut host = session.host_session();
    let written = host.hello_frame(host_name, host_nonce, &mut payload, &mut raw)?;
    let wire_len = encode_wire_frame(&raw[..written.len], wire_out)?;
    session.update_from_host_session(&host);
    Ok(BuiltWireFrame {
        request_id: written.request_id,
        len: wire_len,
    })
}

pub fn build_caps_query_wire_frame(
    session: &mut BoardVmLanguageSession,
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    let mut raw = [0u8; 16];
    let mut host = session.host_session();
    let written = host.caps_query_frame(&mut raw)?;
    let wire_len = encode_wire_frame(&raw[..written.len], wire_out)?;
    session.update_from_host_session(&host);
    Ok(BuiltWireFrame {
        request_id: written.request_id,
        len: wire_len,
    })
}

pub fn build_blink_module(
    program: BlinkProgram,
    out: &mut [u8],
) -> Result<usize, LanguageCoreError> {
    Ok(write_blink_module(program, out)?)
}

pub fn build_program_begin_wire_frame(
    session: &mut BoardVmLanguageSession,
    program_id: u16,
    module: &[u8],
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    let mut payload = [0u8; 16];
    let mut raw = [0u8; 32];
    let mut host = session.host_session();
    let written = host.program_begin_frame(program_id, module, &mut payload, &mut raw)?;
    let wire_len = encode_wire_frame(&raw[..written.len], wire_out)?;
    session.update_from_host_session(&host);
    Ok(BuiltWireFrame {
        request_id: written.request_id,
        len: wire_len,
    })
}

pub fn build_program_chunk_wire_frame(
    session: &mut BoardVmLanguageSession,
    program_id: u16,
    offset: u32,
    chunk: &[u8],
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    let mut payload = vec![0; chunk.len().saturating_add(16)];
    let mut raw = vec![0; chunk.len().saturating_add(32)];
    let mut host = session.host_session();
    let written = host.program_chunk_frame(program_id, offset, chunk, &mut payload, &mut raw)?;
    let wire_len = encode_wire_frame(&raw[..written.len], wire_out)?;
    session.update_from_host_session(&host);
    Ok(BuiltWireFrame {
        request_id: written.request_id,
        len: wire_len,
    })
}

pub fn build_program_end_wire_frame(
    session: &mut BoardVmLanguageSession,
    program_id: u16,
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    let mut payload = [0u8; 8];
    let mut raw = [0u8; 16];
    let mut host = session.host_session();
    let written = host.program_end_frame(program_id, &mut payload, &mut raw)?;
    let wire_len = encode_wire_frame(&raw[..written.len], wire_out)?;
    session.update_from_host_session(&host);
    Ok(BuiltWireFrame {
        request_id: written.request_id,
        len: wire_len,
    })
}

pub fn build_run_background_wire_frame(
    session: &mut BoardVmLanguageSession,
    program_id: u16,
    instruction_budget: u32,
    wire_out: &mut [u8],
) -> Result<BuiltWireFrame, LanguageCoreError> {
    let mut payload = [0u8; 16];
    let mut raw = [0u8; 32];
    let mut host = session.host_session();
    let written =
        host.run_background_frame(program_id, instruction_budget, &mut payload, &mut raw)?;
    let wire_len = encode_wire_frame(&raw[..written.len], wire_out)?;
    session.update_from_host_session(&host);
    Ok(BuiltWireFrame {
        request_id: written.request_id,
        len: wire_len,
    })
}

pub fn decode_wire_frame_into_raw(
    wire_frame: &[u8],
    raw_out: &mut [u8],
) -> Result<BoardVmLanguageStatus, LanguageCoreError> {
    let raw_len = decode_wire_frame(wire_frame, raw_out)?;
    let frame = decode_frame(&raw_out[..raw_len])?;
    Ok(BoardVmLanguageStatus::decoded(
        &frame,
        raw_out.as_ptr(),
        raw_len,
    ))
}

thread_local! {
    static LAST_ERROR_CODE: Cell<u32> = const { Cell::new(BoardVmLanguageStatusCode::Ok as u32) };
    static LAST_ERROR_MESSAGE: RefCell<Option<CString>> = const { RefCell::new(None) };
}

#[no_mangle]
pub extern "C" fn board_vm_language_core_version_major() -> u16 {
    LANGUAGE_CORE_VERSION_MAJOR
}

#[no_mangle]
pub extern "C" fn board_vm_language_core_version_minor() -> u16 {
    LANGUAGE_CORE_VERSION_MINOR
}

#[no_mangle]
pub extern "C" fn board_vm_language_core_version_patch() -> u16 {
    LANGUAGE_CORE_VERSION_PATCH
}

#[no_mangle]
pub extern "C" fn board_vm_language_last_error_code() -> u32 {
    LAST_ERROR_CODE.with(Cell::get)
}

#[no_mangle]
pub extern "C" fn board_vm_language_last_error_message() -> *const std::ffi::c_char {
    LAST_ERROR_MESSAGE.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|message| message.as_ptr())
            .unwrap_or(ptr::null())
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_session_init(
    session: *mut BoardVmLanguageSession,
    next_request_id: u16,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_session_init session") }?;
        *session = BoardVmLanguageSession::with_next_request_id(next_request_id);
        Ok(BoardVmLanguageStatus::ok())
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_session_next_request_id(
    session: *const BoardVmLanguageSession,
) -> u16 {
    clear_error();
    match panic::catch_unwind(AssertUnwindSafe(|| {
        unsafe { ref_from_ptr(session, "board_vm_language_session_next_request_id session") }
            .map(BoardVmLanguageSession::next_request_id)
            .unwrap_or(0)
    })) {
        Ok(value) => value,
        Err(_) => {
            set_error(
                BoardVmLanguageStatusCode::Panic,
                "board_vm_language_session_next_request_id caught a Rust panic.",
            );
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_hello_wire(
    session: *mut BoardVmLanguageSession,
    host_name: *const u8,
    host_name_len: u64,
    host_nonce: u32,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_hello_wire session") }?;
        let host_name = unsafe { utf8_from_ptr(host_name, host_name_len, "host_name") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written = build_hello_wire_frame(session, host_name, host_nonce, wire_out)?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_caps_query_wire(
    session: *mut BoardVmLanguageSession,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_caps_query_wire session") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written = build_caps_query_wire_frame(session, wire_out)?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_blink_module(
    pin: u8,
    high_ms: u16,
    low_ms: u16,
    max_stack: u8,
    module_out: *mut u8,
    module_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let module_out = unsafe { out_slice(module_out, module_cap, "module_out") }?;
        let len = build_blink_module(
            BlinkProgram {
                pin,
                high_ms,
                low_ms,
                max_stack,
            },
            module_out,
        )?;
        Ok(BoardVmLanguageStatus {
            len: len as u64,
            ..BoardVmLanguageStatus::ok()
        })
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_program_begin_wire(
    session: *mut BoardVmLanguageSession,
    program_id: u16,
    module: *const u8,
    module_len: u64,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_program_begin_wire session") }?;
        let module = unsafe { in_slice(module, module_len, "module") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written = build_program_begin_wire_frame(session, program_id, module, wire_out)?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_program_chunk_wire(
    session: *mut BoardVmLanguageSession,
    program_id: u16,
    offset: u32,
    chunk: *const u8,
    chunk_len: u64,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_program_chunk_wire session") }?;
        let chunk = unsafe { in_slice(chunk, chunk_len, "chunk") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written = build_program_chunk_wire_frame(session, program_id, offset, chunk, wire_out)?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_program_end_wire(
    session: *mut BoardVmLanguageSession,
    program_id: u16,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_program_end_wire session") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written = build_program_end_wire_frame(session, program_id, wire_out)?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_run_background_wire(
    session: *mut BoardVmLanguageSession,
    program_id: u16,
    instruction_budget: u32,
    wire_out: *mut u8,
    wire_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let session = unsafe { mut_ref(session, "board_vm_language_run_background_wire session") }?;
        let wire_out = unsafe { out_slice(wire_out, wire_cap, "wire_out") }?;
        let written =
            build_run_background_wire_frame(session, program_id, instruction_budget, wire_out)?;
        Ok(BoardVmLanguageStatus::written(
            written.request_id,
            written.len,
        ))
    })
}

#[no_mangle]
pub unsafe extern "C" fn board_vm_language_decode_wire_frame(
    wire_frame: *const u8,
    wire_frame_len: u64,
    raw_out: *mut u8,
    raw_cap: u64,
) -> BoardVmLanguageStatus {
    catch_status(|| {
        let wire_frame = unsafe { in_slice(wire_frame, wire_frame_len, "wire_frame") }?;
        let raw_out = unsafe { out_slice(raw_out, raw_cap, "raw_out") }?;
        decode_wire_frame_into_raw(wire_frame, raw_out)
    })
}

#[no_mangle]
pub extern "C" fn board_vm_language_default_program_id() -> u16 {
    DEFAULT_PROGRAM_ID
}

#[no_mangle]
pub extern "C" fn board_vm_language_default_instruction_budget() -> u32 {
    DEFAULT_INSTRUCTION_BUDGET
}

#[no_mangle]
pub extern "C" fn board_vm_language_blink_module_len() -> u64 {
    BLINK_MODULE_LEN as u64
}

fn catch_status(
    operation: impl FnOnce() -> Result<BoardVmLanguageStatus, LanguageCoreError>,
) -> BoardVmLanguageStatus {
    clear_error();
    match panic::catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(status)) => status,
        Ok(Err(error)) => {
            let code = status_code_for_error(&error);
            set_error(code, error_message(&error));
            BoardVmLanguageStatus::err(code)
        }
        Err(_) => {
            set_error(
                BoardVmLanguageStatusCode::Panic,
                "board-vm-language-core caught a Rust panic before it crossed the C ABI boundary.",
            );
            BoardVmLanguageStatus::err(BoardVmLanguageStatusCode::Panic)
        }
    }
}

fn status_code_for_error(error: &LanguageCoreError) -> BoardVmLanguageStatusCode {
    match error {
        LanguageCoreError::NullPointer(_) => BoardVmLanguageStatusCode::NullPointer,
        LanguageCoreError::InvalidUtf8 => BoardVmLanguageStatusCode::InvalidUtf8,
        LanguageCoreError::ValueTooLarge => BoardVmLanguageStatusCode::ValueTooLarge,
        LanguageCoreError::OutputTooSmall => BoardVmLanguageStatusCode::OutputTooSmall,
        LanguageCoreError::Protocol(_) => BoardVmLanguageStatusCode::ProtocolError,
        LanguageCoreError::Host(_) => BoardVmLanguageStatusCode::HostError,
    }
}

fn error_message(error: &LanguageCoreError) -> String {
    match error {
        LanguageCoreError::NullPointer(name) => format!("{name} must not be null."),
        other => format!("{other:?}"),
    }
}

fn clear_error() {
    LAST_ERROR_CODE.with(|slot| slot.set(BoardVmLanguageStatusCode::Ok as u32));
    LAST_ERROR_MESSAGE.with(|slot| *slot.borrow_mut() = None);
}

fn set_error(code: BoardVmLanguageStatusCode, message: impl AsRef<str>) {
    LAST_ERROR_CODE.with(|slot| slot.set(code as u32));
    LAST_ERROR_MESSAGE.with(|slot| *slot.borrow_mut() = Some(sanitize_message(message.as_ref())));
}

fn sanitize_message(message: &str) -> CString {
    match CString::new(message) {
        Ok(message) => message,
        Err(_) => CString::new(message.replace('\0', " "))
            .expect("nul-stripped error message must be a valid CString"),
    }
}

unsafe fn ref_from_ptr<'a, T>(
    ptr: *const T,
    name: &'static str,
) -> Result<&'a T, LanguageCoreError> {
    unsafe { ptr.as_ref() }.ok_or_else(|| {
        set_error(
            BoardVmLanguageStatusCode::NullPointer,
            format!("{name} must not be null."),
        );
        LanguageCoreError::NullPointer(name)
    })
}

unsafe fn mut_ref<'a, T>(ptr: *mut T, name: &'static str) -> Result<&'a mut T, LanguageCoreError> {
    unsafe { ptr.as_mut() }.ok_or_else(|| {
        set_error(
            BoardVmLanguageStatusCode::NullPointer,
            format!("{name} must not be null."),
        );
        LanguageCoreError::NullPointer(name)
    })
}

unsafe fn in_slice<'a>(
    ptr: *const u8,
    len: u64,
    name: &'static str,
) -> Result<&'a [u8], LanguageCoreError> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        set_error(
            BoardVmLanguageStatusCode::NullPointer,
            format!("{name} must not be null when len is non-zero."),
        );
        return Err(LanguageCoreError::NullPointer(name));
    }
    let len = usize::try_from(len).map_err(|_| LanguageCoreError::ValueTooLarge)?;
    Ok(unsafe { slice::from_raw_parts(ptr, len) })
}

unsafe fn out_slice<'a>(
    ptr: *mut u8,
    len: u64,
    name: &'static str,
) -> Result<&'a mut [u8], LanguageCoreError> {
    if ptr.is_null() {
        set_error(
            BoardVmLanguageStatusCode::NullPointer,
            format!("{name} must not be null."),
        );
        return Err(LanguageCoreError::NullPointer(name));
    }
    let len = usize::try_from(len).map_err(|_| LanguageCoreError::ValueTooLarge)?;
    Ok(unsafe { slice::from_raw_parts_mut(ptr, len) })
}

unsafe fn utf8_from_ptr<'a>(
    ptr: *const u8,
    len: u64,
    name: &'static str,
) -> Result<&'a str, LanguageCoreError> {
    let bytes = unsafe { in_slice(ptr, len, name) }?;
    str::from_utf8(bytes).map_err(|_| LanguageCoreError::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use board_vm_protocol::{
        decode_program_begin, decode_program_chunk, decode_program_end, decode_run_request,
        encode_frame, encode_wire_frame, Frame, MessageType, RunReportHeader, RunStatus,
        FLAG_IS_RESPONSE, GOLDEN_HELLO_WIRE_FRAME_BVM_V1, RUN_FLAG_BACKGROUND_RUN,
        RUN_FLAG_RESET_VM_BEFORE_RUN,
    };

    #[test]
    fn hello_wire_frame_matches_protocol_golden_vector() {
        let mut session = BoardVmLanguageSession::with_next_request_id(0x1234);
        let mut wire = [0u8; 64];

        let written = build_hello_wire_frame(&mut session, "bvm", 0x1234_ABCD, &mut wire).unwrap();

        assert_eq!(written.request_id, 0x1234);
        assert_eq!(written.len, GOLDEN_HELLO_WIRE_FRAME_BVM_V1.len());
        assert_eq!(&wire[..written.len], GOLDEN_HELLO_WIRE_FRAME_BVM_V1);
        assert_eq!(session.next_request_id(), 0x1235);
    }

    #[test]
    fn c_abi_builds_blink_upload_run_wire_frames_from_rust_core() {
        let mut session = BoardVmLanguageSession::new();
        let mut module = [0u8; BLINK_MODULE_LEN];
        let mut wire = [0u8; 256];
        let mut raw = [0u8; 256];

        let module_status = unsafe {
            board_vm_language_blink_module(
                13,
                250,
                250,
                4,
                module.as_mut_ptr(),
                module.len() as u64,
            )
        };
        assert_eq!(module_status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(module_status.len, BLINK_MODULE_LEN as u64);

        let begin = unsafe {
            board_vm_language_program_begin_wire(
                &mut session,
                7,
                module.as_ptr(),
                module.len() as u64,
                wire.as_mut_ptr(),
                wire.len() as u64,
            )
        };
        assert_eq!(begin.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(begin.request_id, 1);
        let decoded = decode_wire_frame_into_raw(&wire[..begin.len as usize], &mut raw).unwrap();
        assert_eq!(decoded.message_type, MessageType::PROGRAM_BEGIN.0);
        let frame = decode_frame(&raw[..decoded.len as usize]).unwrap();
        assert_eq!(decode_program_begin(frame.payload).unwrap().program_id, 7);

        let chunk = unsafe {
            board_vm_language_program_chunk_wire(
                &mut session,
                7,
                0,
                module.as_ptr(),
                module.len() as u64,
                wire.as_mut_ptr(),
                wire.len() as u64,
            )
        };
        assert_eq!(chunk.request_id, 2);
        let decoded = decode_wire_frame_into_raw(&wire[..chunk.len as usize], &mut raw).unwrap();
        assert_eq!(decoded.message_type, MessageType::PROGRAM_CHUNK.0);
        let frame = decode_frame(&raw[..decoded.len as usize]).unwrap();
        let chunk_payload = decode_program_chunk(frame.payload).unwrap();
        assert_eq!(chunk_payload.offset, 0);
        assert_eq!(chunk_payload.bytes, &module);

        let end = unsafe {
            board_vm_language_program_end_wire(
                &mut session,
                7,
                wire.as_mut_ptr(),
                wire.len() as u64,
            )
        };
        assert_eq!(end.request_id, 3);
        let decoded = decode_wire_frame_into_raw(&wire[..end.len as usize], &mut raw).unwrap();
        assert_eq!(decoded.message_type, MessageType::PROGRAM_END.0);
        let frame = decode_frame(&raw[..decoded.len as usize]).unwrap();
        assert_eq!(decode_program_end(frame.payload).unwrap().program_id, 7);

        let run = unsafe {
            board_vm_language_run_background_wire(
                &mut session,
                7,
                123,
                wire.as_mut_ptr(),
                wire.len() as u64,
            )
        };
        assert_eq!(run.request_id, 4);
        let decoded = decode_wire_frame_into_raw(&wire[..run.len as usize], &mut raw).unwrap();
        assert_eq!(decoded.message_type, MessageType::RUN.0);
        let frame = decode_frame(&raw[..decoded.len as usize]).unwrap();
        let run_payload = decode_run_request(frame.payload).unwrap();
        assert_eq!(run_payload.instruction_budget, 123);
        assert_eq!(
            run_payload.flags,
            RUN_FLAG_RESET_VM_BEFORE_RUN | RUN_FLAG_BACKGROUND_RUN
        );
    }

    #[test]
    fn c_abi_decode_wire_frame_reports_payload_offset_and_len() {
        let report = RunReportHeader {
            program_id: 1,
            status: RunStatus::BudgetExceeded,
            instructions_executed: 12,
            elapsed_ms: 20,
            stack_depth: 1,
            open_handles: 1,
            return_count: 0,
        };
        let mut payload = [0u8; 32];
        let payload_len =
            board_vm_protocol::encode_run_report_header(&report, &mut payload).unwrap();
        let mut raw = [0u8; 64];
        let raw_len = encode_frame(
            &Frame {
                flags: FLAG_IS_RESPONSE,
                message_type: MessageType::RUN_REPORT,
                request_id: 9,
                payload: &payload[..payload_len],
            },
            &mut raw,
        )
        .unwrap();
        let mut wire = [0u8; 96];
        let wire_len = encode_wire_frame(&raw[..raw_len], &mut wire).unwrap();
        let mut decoded_raw = [0u8; 64];

        let status = unsafe {
            board_vm_language_decode_wire_frame(
                wire.as_ptr(),
                wire_len as u64,
                decoded_raw.as_mut_ptr(),
                decoded_raw.len() as u64,
            )
        };

        assert_eq!(status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(status.len, raw_len as u64);
        assert_eq!(status.message_type, MessageType::RUN_REPORT.0);
        assert_eq!(status.request_id, 9);
        assert_eq!(status.payload_len, payload_len as u64);
        assert!(status.payload_offset > 0);
    }

    #[test]
    fn c_abi_reports_null_output_buffers_without_unwinding() {
        let status = unsafe { board_vm_language_blink_module(13, 250, 250, 4, ptr::null_mut(), 0) };

        assert_ne!(status.code, BoardVmLanguageStatusCode::Ok as u32);
        assert_eq!(
            board_vm_language_last_error_code(),
            BoardVmLanguageStatusCode::NullPointer as u32
        );
        let message = unsafe {
            std::ffi::CStr::from_ptr(board_vm_language_last_error_message())
                .to_string_lossy()
                .into_owned()
        };
        assert!(message.contains("module_out"));
    }
}
