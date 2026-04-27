//! # WASI Tier 3 integration tests
//!
//! Tests for the eight new WASI host functions added in Tier 3:
//!
//! | Function           | What it tests                                    |
//! |--------------------|--------------------------------------------------|
//! | `args_sizes_get`   | argc and buf_size written to memory correctly    |
//! | `args_get`         | argv pointers and null-terminated strings        |
//! | `environ_sizes_get`| envc and buf_size for env vars                   |
//! | `environ_get`      | environ pointers and null-terminated strings     |
//! | `clock_time_get`   | realtime and monotonic written as i64 LE         |
//! | `clock_res_get`    | resolution written as i64 LE                     |
//! | `random_get`       | buf filled with expected bytes                   |
//! | `sched_yield`      | returns WasmValue::I32(0)                        |
//!
//! All tests use deterministic fakes for the clock and RNG so results
//! are reproducible regardless of system state.

use wasm_execution::{LinearMemory, WasmValue};
use wasm_runtime::{WasiClock, WasiConfig, WasiEnv, WasiRandom};

// ══════════════════════════════════════════════════════════════════════════════
// Deterministic fakes
// ══════════════════════════════════════════════════════════════════════════════

/// Fake clock with fixed timestamps for deterministic tests.
///
/// Returns:
/// - realtime_ns  = 1_700_000_000_000_000_001  (approx 2023-11-14 22:13:20 UTC)
/// - monotonic_ns = 42_000_000_000             (42 seconds elapsed)
/// - resolution   = 1_000_000                  (1 ms)
struct FakeClock;

impl WasiClock for FakeClock {
    fn realtime_ns(&self) -> i64 {
        1_700_000_000_000_000_001
    }

    fn monotonic_ns(&self) -> i64 {
        42_000_000_000
    }

    fn resolution_ns(&self, _clock_id: i32) -> i64 {
        1_000_000
    }
}

/// Fake RNG that fills every byte with 0xAB.
///
/// Using a fixed pattern makes memory assertions exact — no probabilistic
/// reasoning required.
struct FakeRandom;

impl WasiRandom for FakeRandom {
    fn fill_bytes(&self, buf: &mut [u8]) {
        buf.fill(0xAB);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Helper: build a WasiEnv with fake clock/random and 1 page of memory
// ══════════════════════════════════════════════════════════════════════════════

/// Build a `WasiEnv` with the given args and env vars, attach 1 page of
/// fresh linear memory, and return both the env and the shared memory handle
/// so tests can read back what was written.
fn make_env(args: Vec<String>, env: Vec<String>) -> WasiEnv {
    let cfg = WasiConfig {
        args,
        env,
        clock: Box::new(FakeClock),
        random: Box::new(FakeRandom),
        ..Default::default()
    };
    let wasi = WasiEnv::new(cfg);

    // Attach 1 page (64 KiB) of zeroed linear memory.
    let mem = LinearMemory::new(1, None);
    wasi.attach_memory(mem);

    wasi
}

/// Read a little-endian i32 from memory at the given offset.
fn read_i32_le(mem: &LinearMemory, offset: usize) -> i32 {
    mem.load_i32(offset).expect("read_i32_le: out of bounds")
}

/// Read a little-endian i64 from memory at the given offset.
fn read_i64_le(mem: &LinearMemory, offset: usize) -> i64 {
    mem.load_i64(offset).expect("read_i64_le: out of bounds")
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 1: args_sizes_get — ["myapp", "hello"]
// ══════════════════════════════════════════════════════════════════════════════

/// `args_sizes_get` with two arguments should write:
///   argc     = 2
///   buf_size = len("myapp") + 1 + len("hello") + 1 = 6 + 6 = 12
#[test]
fn test_args_sizes_get() {
    let wasi = make_env(vec!["myapp".to_string(), "hello".to_string()], vec![]);

    // Memory layout:
    //   offset 0 → argc (i32)
    //   offset 4 → buf_size (i32)
    let func = wasi
        .resolve_function_for_test("args_sizes_get")
        .expect("args_sizes_get should be registered");

    let result = func
        .call(&[WasmValue::I32(0), WasmValue::I32(4)], None)
        .expect("args_sizes_get should succeed");

    assert_eq!(result, vec![WasmValue::I32(0)], "errno should be 0");

    let mem_guard = wasi.memory.lock().unwrap();
    let mem = mem_guard.as_ref().unwrap();
    assert_eq!(read_i32_le(mem, 0), 2, "argc should be 2");
    assert_eq!(read_i32_le(mem, 4), 12, "buf_size should be 12");
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 2: args_get — verify memory layout
// ══════════════════════════════════════════════════════════════════════════════

/// `args_get` should write:
///   argv[0] = pointer to "myapp\0"
///   argv[1] = pointer to "hello\0"
///   strings packed at argv_buf_ptr
///
/// Memory layout example (argv_ptr=0, argv_buf_ptr=100):
///   offset   0: i32 = 100   (pointer to "myapp\0")
///   offset   4: i32 = 106   (pointer to "hello\0")
///   offset 100: b'm' b'y' b'a' b'p' b'p' 0x00
///   offset 106: b'h' b'e' b'l' b'l' b'o' 0x00
#[test]
fn test_args_get() {
    let wasi = make_env(vec!["myapp".to_string(), "hello".to_string()], vec![]);

    let func = wasi
        .resolve_function_for_test("args_get")
        .expect("args_get should be registered");

    // argv pointer array at offset 0 (8 bytes for 2 pointers).
    // argv buffer at offset 100.
    let result = func
        .call(&[WasmValue::I32(0), WasmValue::I32(100)], None)
        .expect("args_get should succeed");

    assert_eq!(result, vec![WasmValue::I32(0)], "errno should be 0");

    let mem_guard = wasi.memory.lock().unwrap();
    let mem = mem_guard.as_ref().unwrap();

    // argv[0] should point to offset 100.
    let ptr0 = read_i32_le(mem, 0);
    assert_eq!(ptr0, 100, "argv[0] should point to offset 100");

    // argv[1] should point to offset 106 (100 + len("myapp") + 1).
    let ptr1 = read_i32_le(mem, 4);
    assert_eq!(ptr1, 106, "argv[1] should point to offset 106");

    // Read back the strings from memory.
    let myapp_bytes = read_bytes(mem, 100, 6); // "myapp\0"
    assert_eq!(&myapp_bytes, b"myapp\0");

    let hello_bytes = read_bytes(mem, 106, 6); // "hello\0"
    assert_eq!(&hello_bytes, b"hello\0");
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 3: environ_sizes_get — ["HOME=/home/user"]
// ══════════════════════════════════════════════════════════════════════════════

/// `environ_sizes_get` with one env var should write:
///   envc     = 1
///   buf_size = len("HOME=/home/user") + 1 = 16
///
/// "HOME=/home/user" has 15 bytes, +1 for '\0' = 16.
#[test]
fn test_environ_sizes_get() {
    let wasi = make_env(vec![], vec!["HOME=/home/user".to_string()]);

    let func = wasi
        .resolve_function_for_test("environ_sizes_get")
        .expect("environ_sizes_get should be registered");

    let result = func
        .call(&[WasmValue::I32(0), WasmValue::I32(4)], None)
        .expect("environ_sizes_get should succeed");

    assert_eq!(result, vec![WasmValue::I32(0)]);

    let mem_guard = wasi.memory.lock().unwrap();
    let mem = mem_guard.as_ref().unwrap();
    assert_eq!(read_i32_le(mem, 0), 1, "envc should be 1");
    // "HOME=/home/user" = 15 bytes + 1 null = 16
    assert_eq!(read_i32_le(mem, 4), 16, "buf_size should be 16");
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 4: environ_get — verify memory
// ══════════════════════════════════════════════════════════════════════════════

/// `environ_get` with `HOME=/home/user` should write the pointer and string.
#[test]
fn test_environ_get() {
    let wasi = make_env(vec![], vec!["HOME=/home/user".to_string()]);

    let func = wasi
        .resolve_function_for_test("environ_get")
        .expect("environ_get should be registered");

    // Pointer array at 0; buffer at 200.
    let result = func
        .call(&[WasmValue::I32(0), WasmValue::I32(200)], None)
        .expect("environ_get should succeed");

    assert_eq!(result, vec![WasmValue::I32(0)]);

    let mem_guard = wasi.memory.lock().unwrap();
    let mem = mem_guard.as_ref().unwrap();

    // environ[0] should point to 200.
    assert_eq!(read_i32_le(mem, 0), 200);

    // "HOME=/home/user\0" = 16 bytes
    let env_bytes = read_bytes(mem, 200, 16);
    assert_eq!(&env_bytes, b"HOME=/home/user\0");
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 5: clock_time_get(0) — CLOCK_REALTIME
// ══════════════════════════════════════════════════════════════════════════════

/// `clock_time_get` with id=0 should write `realtime_ns()` as an i64 LE
/// to the given pointer.
///
/// FakeClock.realtime_ns() = 1_700_000_000_000_000_001
#[test]
fn test_clock_time_get_realtime() {
    let wasi = make_env(vec![], vec![]);

    let func = wasi
        .resolve_function_for_test("clock_time_get")
        .expect("clock_time_get should be registered");

    // id=0 (REALTIME), precision=0, time_ptr=0
    let result = func
        .call(&[WasmValue::I32(0), WasmValue::I64(0), WasmValue::I32(0)], None)
        .expect("clock_time_get should succeed");

    assert_eq!(result, vec![WasmValue::I32(0)]);

    let mem_guard = wasi.memory.lock().unwrap();
    let mem = mem_guard.as_ref().unwrap();
    let ts = read_i64_le(mem, 0);
    assert_eq!(ts, 1_700_000_000_000_000_001i64);
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 6: clock_time_get(1) — CLOCK_MONOTONIC
// ══════════════════════════════════════════════════════════════════════════════

/// `clock_time_get` with id=1 should write `monotonic_ns()`.
///
/// FakeClock.monotonic_ns() = 42_000_000_000
#[test]
fn test_clock_time_get_monotonic() {
    let wasi = make_env(vec![], vec![]);

    let func = wasi
        .resolve_function_for_test("clock_time_get")
        .expect("clock_time_get should be registered");

    let result = func
        .call(&[WasmValue::I32(1), WasmValue::I64(0), WasmValue::I32(0)], None)
        .expect("clock_time_get should succeed");

    assert_eq!(result, vec![WasmValue::I32(0)]);

    let mem_guard = wasi.memory.lock().unwrap();
    let mem = mem_guard.as_ref().unwrap();
    let ts = read_i64_le(mem, 0);
    assert_eq!(ts, 42_000_000_000i64);
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 7: clock_res_get(0) — resolution
// ══════════════════════════════════════════════════════════════════════════════

/// `clock_res_get` should write the resolution as an i64 LE.
///
/// FakeClock.resolution_ns() = 1_000_000 (1 ms).
#[test]
fn test_clock_res_get() {
    let wasi = make_env(vec![], vec![]);

    let func = wasi
        .resolve_function_for_test("clock_res_get")
        .expect("clock_res_get should be registered");

    let result = func
        .call(&[WasmValue::I32(0), WasmValue::I32(0)], None)
        .expect("clock_res_get should succeed");

    assert_eq!(result, vec![WasmValue::I32(0)]);

    let mem_guard = wasi.memory.lock().unwrap();
    let mem = mem_guard.as_ref().unwrap();
    let res = read_i64_le(mem, 0);
    assert_eq!(res, 1_000_000i64);
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 8: random_get — 4 bytes of 0xAB
// ══════════════════════════════════════════════════════════════════════════════

/// `random_get(buf_ptr=0, buf_len=4)` should fill 4 bytes at offset 0 with
/// 0xAB (as determined by FakeRandom).
#[test]
fn test_random_get() {
    let wasi = make_env(vec![], vec![]);

    let func = wasi
        .resolve_function_for_test("random_get")
        .expect("random_get should be registered");

    let result = func
        .call(&[WasmValue::I32(0), WasmValue::I32(4)], None)
        .expect("random_get should succeed");

    assert_eq!(result, vec![WasmValue::I32(0)]);

    let mem_guard = wasi.memory.lock().unwrap();
    let mem = mem_guard.as_ref().unwrap();
    let bytes = read_bytes(mem, 0, 4);
    assert_eq!(bytes, vec![0xAB, 0xAB, 0xAB, 0xAB]);
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 8b: fd_write emits stdout and reports bytes written
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_fd_write_emits_stdout() {
    let output = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let output_clone = output.clone();
    let wasi = WasiEnv::new(WasiConfig {
        stdout_callback: Some(Box::new(move |text| {
            output_clone.lock().unwrap().push_str(text);
        })),
        ..Default::default()
    });

    let mut mem = LinearMemory::new(1, None);
    mem.store_i32(0, 32).unwrap();
    mem.store_i32(4, 2).unwrap();
    mem.write_bytes(32, b"Hi").unwrap();
    wasi.attach_memory(mem);

    let func = wasi
        .resolve_function_for_test("fd_write")
        .expect("fd_write should be registered");
    let result = func
        .call(
            &[
                WasmValue::I32(1),
                WasmValue::I32(0),
                WasmValue::I32(1),
                WasmValue::I32(16),
            ],
            None,
        )
        .expect("fd_write should succeed");

    assert_eq!(result, vec![WasmValue::I32(0)]);
    assert_eq!(output.lock().unwrap().as_str(), "Hi");

    let mem_guard = wasi.memory.lock().unwrap();
    let mem = mem_guard.as_ref().unwrap();
    assert_eq!(read_i32_le(mem, 16), 2);
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 8c: fd_read fills guest memory from stdin
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_fd_read_reads_stdin_into_guest_memory() {
    let wasi = WasiEnv::new(WasiConfig {
        stdin_callback: Some(Box::new(|count| b"Yo"[..count.min(2)].to_vec())),
        ..Default::default()
    });

    let mut mem = LinearMemory::new(1, None);
    mem.store_i32(0, 32).unwrap();
    mem.store_i32(4, 2).unwrap();
    wasi.attach_memory(mem);

    let func = wasi
        .resolve_function_for_test("fd_read")
        .expect("fd_read should be registered");
    let result = func
        .call(
            &[
                WasmValue::I32(0),
                WasmValue::I32(0),
                WasmValue::I32(1),
                WasmValue::I32(16),
            ],
            None,
        )
        .expect("fd_read should succeed");

    assert_eq!(result, vec![WasmValue::I32(0)]);

    let mem_guard = wasi.memory.lock().unwrap();
    let mem = mem_guard.as_ref().unwrap();
    assert_eq!(read_i32_le(mem, 16), 2);
    assert_eq!(read_bytes(mem, 32, 2), b"Yo");
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 8d: fd_read rejects non-stdin file descriptors
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_fd_read_rejects_non_stdin_fd() {
    let wasi = make_env(vec![], vec![]);

    let func = wasi
        .resolve_function_for_test("fd_read")
        .expect("fd_read should be registered");
    let result = func
        .call(
            &[
                WasmValue::I32(3),
                WasmValue::I32(0),
                WasmValue::I32(1),
                WasmValue::I32(16),
            ],
            None,
        )
        .expect("fd_read should not trap");

    assert_eq!(result, vec![WasmValue::I32(8)]);
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 9: sched_yield — returns success
// ══════════════════════════════════════════════════════════════════════════════

/// `sched_yield()` should return errno 0 immediately (no-op).
#[test]
fn test_sched_yield() {
    let wasi = make_env(vec![], vec![]);

    let func = wasi
        .resolve_function_for_test("sched_yield")
        .expect("sched_yield should be registered");

    let result = func.call(&[], None).expect("sched_yield should succeed");
    assert_eq!(result, vec![WasmValue::I32(0)]);
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 10: existing square test passes (regression guard)
// ══════════════════════════════════════════════════════════════════════════════

/// The square(x) = x * x end-to-end test must still pass after our changes.
/// This guards against accidental breakage of the core runtime.
#[test]
fn test_square_regression() {
    use wasm_runtime::WasmRuntime;

    let runtime = WasmRuntime::new();

    // Build the square WASM inline (same bytes as the unit test in lib.rs).
    let mut wasm = Vec::new();
    wasm.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]); // magic
    wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // version 1

    // Type section: (i32) -> i32
    let type_section = vec![0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F];
    wasm.push(0x01);
    wasm.push(type_section.len() as u8);
    wasm.extend_from_slice(&type_section);

    // Function section
    wasm.push(0x03);
    wasm.push(2u8);
    wasm.extend_from_slice(&[0x01, 0x00]);

    // Export section: "square"
    let export_section = vec![0x01, 0x06, b's', b'q', b'u', b'a', b'r', b'e', 0x00, 0x00];
    wasm.push(0x07);
    wasm.push(export_section.len() as u8);
    wasm.extend_from_slice(&export_section);

    // Code section: local.get 0; local.get 0; i32.mul; end
    let body = vec![0x00, 0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B];
    let mut code_section = vec![0x01u8, (body.len() as u8) + 0];
    // body_with_size
    code_section.push(body.len() as u8);
    code_section.extend_from_slice(&body);
    // Rebuild properly
    let body_size = body.len() as u8;
    let mut code_section2 = vec![0x01u8]; // 1 function
    code_section2.push(body_size + 1); // body length (body + 0 locals byte)
                                       // Actually body already includes the locals byte (0x00)
                                       // Let's just build it correctly
    let full_body = body.clone();
    let mut cs = vec![0x01u8];
    cs.push(full_body.len() as u8);
    cs.extend_from_slice(&full_body);

    wasm.push(0x0A);
    wasm.push(cs.len() as u8);
    wasm.extend_from_slice(&cs);

    let result = runtime.load_and_run(&wasm, "square", &[5]);
    assert_eq!(result.unwrap(), vec![25]);
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 11: clock_time_get with invalid clock ID — EINVAL
// ══════════════════════════════════════════════════════════════════════════════

/// An unknown clock ID (e.g. 99) should return EINVAL = 28.
#[test]
fn test_clock_time_get_invalid_id() {
    let wasi = make_env(vec![], vec![]);

    let func = wasi
        .resolve_function_for_test("clock_time_get")
        .expect("clock_time_get should be registered");

    let result = func
        .call(&[WasmValue::I32(99), WasmValue::I64(0), WasmValue::I32(0)], None)
        .expect("call should not trap");

    assert_eq!(result, vec![WasmValue::I32(28)], "should return EINVAL=28");
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 12: args_sizes_get with empty args
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_args_sizes_get_empty() {
    let wasi = make_env(vec![], vec![]);

    let func = wasi
        .resolve_function_for_test("args_sizes_get")
        .expect("args_sizes_get should be registered");

    let result = func
        .call(&[WasmValue::I32(0), WasmValue::I32(4)], None)
        .expect("should succeed");

    assert_eq!(result, vec![WasmValue::I32(0)]);

    let mem_guard = wasi.memory.lock().unwrap();
    let mem = mem_guard.as_ref().unwrap();
    assert_eq!(read_i32_le(mem, 0), 0, "argc should be 0");
    assert_eq!(read_i32_le(mem, 4), 0, "buf_size should be 0");
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 13: WasiEnv resolves proc_exit and unknown → ENOSYS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_wasi_env_proc_exit_and_enosys() {
    let wasi = make_env(vec![], vec![]);

    // proc_exit should trap
    let proc_exit = wasi
        .resolve_function_for_test("proc_exit")
        .expect("proc_exit should be registered");
    let trap = proc_exit.call(&[WasmValue::I32(0)], None);
    assert!(trap.is_err(), "proc_exit should trap");

    // Unknown function should return ENOSYS (52)
    let enosys = wasi
        .resolve_function_for_test("fd_sync")
        .expect("fd_sync should fall back to ENOSYS");
    let result = enosys.call(&[], None).unwrap();
    assert_eq!(result, vec![WasmValue::I32(52)]);
}

// ══════════════════════════════════════════════════════════════════════════════
// Test 14: random_get with zero length — no-op
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_random_get_zero_length() {
    let wasi = make_env(vec![], vec![]);

    let func = wasi
        .resolve_function_for_test("random_get")
        .expect("random_get should be registered");

    let result = func
        .call(&[WasmValue::I32(0), WasmValue::I32(0)], None)
        .expect("random_get(0, 0) should succeed");

    assert_eq!(result, vec![WasmValue::I32(0)]);
}

// ══════════════════════════════════════════════════════════════════════════════
// Helper: read raw bytes from LinearMemory
// ══════════════════════════════════════════════════════════════════════════════

/// Read `len` raw bytes from `mem` starting at `offset`.
fn read_bytes(mem: &LinearMemory, offset: usize, len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| {
            // Load each byte individually using load_i32_8u (zero-extends to i32).
            mem.load_i32_8u(offset + i)
                .expect("read_bytes: out of bounds") as u8
        })
        .collect()
}

// ══════════════════════════════════════════════════════════════════════════════
// Helper trait: WasiEnv::resolve_function_for_test
// ══════════════════════════════════════════════════════════════════════════════

/// Extension trait so tests can resolve functions without the full
/// HostInterface syntax.
trait ResolveForTest {
    fn resolve_function_for_test(
        &self,
        name: &str,
    ) -> Option<Box<dyn wasm_execution::HostFunction>>;
}

impl ResolveForTest for WasiEnv {
    fn resolve_function_for_test(
        &self,
        name: &str,
    ) -> Option<Box<dyn wasm_execution::HostFunction>> {
        use wasm_execution::HostInterface;
        self.resolve_function("wasi_snapshot_preview1", name)
    }
}
