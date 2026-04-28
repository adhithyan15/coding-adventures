use std::ffi::{c_long, c_void};
use std::io;
use std::ptr;

use embeddable_tcp_server::{
    EmbeddableTcpServer, EmbeddableTcpServerOptions, StdioJobSubmitter, TcpMailboxFrame,
    WorkerCommand, WorkerError,
};
use ruby_bridge::VALUE;
use serde::{Deserialize, Serialize};
use tcp_runtime::{TcpConnectionInfo, TcpHandlerResult};

const MAX_TCP_CHUNK_BYTES: usize = 1024 * 1024;

extern "C" {
    fn rb_num2long(val: VALUE) -> c_long;
    fn rb_thread_call_without_gvl(
        func: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
        data1: *mut c_void,
        ubf: Option<unsafe extern "C" fn(*mut c_void)>,
        data2: *mut c_void,
    ) -> *mut c_void;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TcpInputJob {
    stream_id: String,
    bytes_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TcpOutputFrame {
    writes_hex: Vec<String>,
    close: bool,
}

struct RubyMiniRedisServer {
    server: Option<EmbeddableTcpServer<()>>,
}

struct ServeCall {
    server: *const EmbeddableTcpServer<()>,
    result: Option<io::Result<()>>,
}

static mut NATIVE_SERVER_CLASS: VALUE = 0;
static mut SERVER_ERROR: VALUE = 0;

unsafe extern "C" fn server_alloc(klass: VALUE) -> VALUE {
    ruby_bridge::wrap_data(klass, RubyMiniRedisServer { server: None })
}

extern "C" fn server_initialize(
    self_val: VALUE,
    host_val: VALUE,
    port_val: VALUE,
    max_connections_val: VALUE,
    worker_processes_val: VALUE,
    worker_queue_depth_val: VALUE,
    worker_program_val: VALUE,
    worker_args_val: VALUE,
) -> VALUE {
    let host = string_from_rb(host_val, "host must be a String");
    let port = u16_from_rb(port_val, "port must be between 0 and 65535");
    let max_connections =
        usize_from_rb(max_connections_val, "max_connections must be non-negative");
    let worker_processes = usize_from_rb(
        worker_processes_val,
        "worker_processes must be non-negative",
    );
    let worker_queue_depth = usize_from_rb(
        worker_queue_depth_val,
        "worker_queue_depth must be non-negative",
    );
    let worker_program = string_from_rb(worker_program_val, "worker_program must be a String");
    let worker_args = ruby_bridge::vec_str_from_rb(worker_args_val);

    let options = EmbeddableTcpServerOptions {
        host,
        port,
        max_connections,
        worker_processes,
        worker_queue_depth,
        worker: WorkerCommand::new(worker_program, worker_args),
        ..EmbeddableTcpServerOptions::default()
    };

    let server = match EmbeddableTcpServer::new_mailbox(
        options,
        |_| (),
        handle_tcp_bytes,
        |_, _| {},
        map_tcp_output_frame,
    ) {
        Ok(server) => server,
        Err(error) => {
            raise_server_error(&format!("failed to start Mini Redis TCP runtime: {error}"))
        }
    };

    let slot = unsafe { ruby_bridge::unwrap_data_mut::<RubyMiniRedisServer>(self_val) };
    slot.server = Some(server);
    self_val
}

extern "C" fn server_serve(self_val: VALUE) -> VALUE {
    let server = get_server(self_val);
    let mut call = ServeCall {
        server: server as *const EmbeddableTcpServer<()>,
        result: None,
    };

    unsafe {
        rb_thread_call_without_gvl(
            serve_without_gvl,
            &mut call as *mut ServeCall as *mut c_void,
            None,
            ptr::null_mut(),
        );
    }

    match call.result.take().expect("serve result should be set") {
        Ok(()) => ruby_bridge::QNIL,
        Err(error) => raise_server_error(&format!("Mini Redis TCP runtime failed: {error}")),
    }
}

extern "C" fn server_stop(self_val: VALUE) -> VALUE {
    get_server(self_val).stop();
    ruby_bridge::QNIL
}

extern "C" fn server_dispose(self_val: VALUE) -> VALUE {
    let slot = unsafe { ruby_bridge::unwrap_data_mut::<RubyMiniRedisServer>(self_val) };
    if let Some(server) = slot.server.as_ref() {
        if server.is_running() {
            raise_server_error("cannot dispose a running server; stop and wait first");
        }
    }
    slot.server.take();
    ruby_bridge::QNIL
}

extern "C" fn server_running(self_val: VALUE) -> VALUE {
    ruby_bridge::bool_to_rb(get_server(self_val).is_running())
}

extern "C" fn server_local_host(self_val: VALUE) -> VALUE {
    ruby_bridge::str_to_rb(&get_server(self_val).local_addr().ip().to_string())
}

extern "C" fn server_local_port(self_val: VALUE) -> VALUE {
    ruby_bridge::usize_to_rb(get_server(self_val).local_addr().port() as usize)
}

unsafe extern "C" fn serve_without_gvl(data: *mut c_void) -> *mut c_void {
    let call = &mut *(data as *mut ServeCall);
    call.result = Some((*call.server).serve());
    ptr::null_mut()
}

fn get_server(self_val: VALUE) -> &'static EmbeddableTcpServer<()> {
    let slot = unsafe { ruby_bridge::unwrap_data::<RubyMiniRedisServer>(self_val) };
    match slot.server.as_ref() {
        Some(server) => server,
        None => raise_server_error("server is closed"),
    }
}

fn handle_tcp_bytes(
    info: TcpConnectionInfo,
    _state: &mut (),
    data: &[u8],
    submitter: &StdioJobSubmitter<TcpInputJob, TcpOutputFrame>,
) -> TcpHandlerResult {
    if data.len() > MAX_TCP_CHUNK_BYTES {
        return TcpHandlerResult::close();
    }

    match submitter.submit(
        info.id,
        TcpInputJob {
            stream_id: info.id.0.to_string(),
            bytes_hex: hex_encode(data),
        },
    ) {
        Ok(_) => TcpHandlerResult::default(),
        Err(_) => TcpHandlerResult::defer_read(),
    }
}

fn map_tcp_output_frame(frame: TcpOutputFrame) -> Result<TcpMailboxFrame, WorkerError> {
    let mut writes = Vec::with_capacity(frame.writes_hex.len());
    for item in frame.writes_hex {
        writes.push(hex_decode(&item).map_err(WorkerError::Protocol)?);
    }
    Ok(TcpMailboxFrame {
        writes,
        close: frame.close,
    })
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn hex_decode(input: &str) -> Result<Vec<u8>, String> {
    if input.len() % 2 != 0 {
        return Err("hex input has odd length".to_string());
    }
    let mut output = Vec::with_capacity(input.len() / 2);
    for pair in input.as_bytes().chunks_exact(2) {
        let high = hex_value(pair[0])?;
        let low = hex_value(pair[1])?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn hex_value(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid hex byte: {byte}")),
    }
}

fn string_from_rb(value: VALUE, message: &str) -> String {
    match ruby_bridge::str_from_rb(value) {
        Some(value) => value,
        None => raise_arg_error(message),
    }
}

fn usize_from_rb(value: VALUE, message: &str) -> usize {
    let number = unsafe { rb_num2long(value) };
    if number < 0 {
        raise_arg_error(message);
    }
    number as usize
}

fn u16_from_rb(value: VALUE, message: &str) -> u16 {
    let number = usize_from_rb(value, message);
    if number > u16::MAX as usize {
        raise_arg_error(message);
    }
    number as u16
}

fn raise_arg_error(message: &str) -> ! {
    ruby_bridge::raise_error(ruby_bridge::path2class("ArgumentError"), message)
}

fn raise_server_error(message: &str) -> ! {
    ruby_bridge::raise_error(unsafe { SERVER_ERROR }, message)
}

#[no_mangle]
pub extern "C" fn Init_mini_redis_native() {
    let coding_adventures = ruby_bridge::define_module("CodingAdventures");
    let mini_redis_native = ruby_bridge::define_module_under(coding_adventures, "MiniRedisNative");

    let error_class = ruby_bridge::define_class_under(
        mini_redis_native,
        "ServerError",
        ruby_bridge::standard_error_class(),
    );
    unsafe { SERVER_ERROR = error_class };

    let server_class = ruby_bridge::define_class_under(
        mini_redis_native,
        "NativeServer",
        ruby_bridge::object_class(),
    );
    unsafe { NATIVE_SERVER_CLASS = server_class };

    ruby_bridge::define_alloc_func(server_class, server_alloc);
    ruby_bridge::define_method_raw(
        server_class,
        "initialize",
        server_initialize as *const c_void,
        7,
    );
    ruby_bridge::define_method_raw(server_class, "serve", server_serve as *const c_void, 0);
    ruby_bridge::define_method_raw(server_class, "stop", server_stop as *const c_void, 0);
    ruby_bridge::define_method_raw(server_class, "dispose", server_dispose as *const c_void, 0);
    ruby_bridge::define_method_raw(server_class, "running?", server_running as *const c_void, 0);
    ruby_bridge::define_method_raw(
        server_class,
        "local_host",
        server_local_host as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        server_class,
        "local_port",
        server_local_port as *const c_void,
        0,
    );
}
