# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_wasm_runtime"

# ==========================================================================
# Tests for WasiStub --- Minimal WASI Host Implementation
# ==========================================================================
#
# WasiStub provides a minimal WASI implementation for WASM modules:
#   - fd_write: captures stdout/stderr output
#   - proc_exit: raises ProcExitError with exit code
#   - Other WASI functions: return ENOSYS (52)
#
# Tests cover:
#   1. resolve_function dispatching
#   2. fd_write with memory
#   3. proc_exit raises ProcExitError
#   4. Stub functions return ENOSYS
#   5. Non-WASI modules return nil
#   6. set_memory
#   7. ProcExitError attributes
# ==========================================================================

class TestWasi < Minitest::Test
  WE = CodingAdventures::WasmExecution
  WT = CodingAdventures::WasmTypes
  WR = CodingAdventures::WasmRuntime

  # ── resolve_function ───────────────────────────────────────────────

  def test_resolve_fd_write
    wasi = WR::WasiStub.new
    result = wasi.resolve_function("wasi_snapshot_preview1", "fd_write")
    refute_nil result
    assert_kind_of WE::HostFunction, result
  end

  def test_wasi_host_alias_matches_stub
    assert_equal WR::WasiHost, WR::WasiStub
  end

  def test_resolve_fd_read
    wasi = WR::WasiHost.new
    result = wasi.resolve_function("wasi_snapshot_preview1", "fd_read")
    refute_nil result
    assert_kind_of WE::HostFunction, result
  end

  def test_resolve_proc_exit
    wasi = WR::WasiStub.new
    result = wasi.resolve_function("wasi_snapshot_preview1", "proc_exit")
    refute_nil result
    assert_kind_of WE::HostFunction, result
  end

  def test_resolve_unknown_wasi_function_returns_stub
    wasi = WR::WasiStub.new
    result = wasi.resolve_function("wasi_snapshot_preview1", "random_get")
    refute_nil result
    assert_kind_of WE::HostFunction, result
  end

  def test_resolve_non_wasi_module_returns_nil
    wasi = WR::WasiStub.new
    result = wasi.resolve_function("env", "some_function")
    assert_nil result
  end

  # ── Stub functions return ENOSYS ───────────────────────────────────

  def test_stub_function_returns_enosys
    # Use a WASI function that is not yet implemented (Tier 3 functions such as
    # args_sizes_get are now real implementations; use an obscure unimplemented
    # one like path_open instead).
    wasi = WR::WasiStub.new
    stub = wasi.resolve_function("wasi_snapshot_preview1", "path_open")
    result = stub.call([])
    assert_equal 1, result.length
    assert_equal 52, result[0].value # ENOSYS
  end

  # ── proc_exit ──────────────────────────────────────────────────────

  def test_proc_exit_raises_with_code_0
    wasi = WR::WasiStub.new
    proc_exit = wasi.resolve_function("wasi_snapshot_preview1", "proc_exit")
    err = assert_raises(WR::ProcExitError) do
      proc_exit.call([WE.i32(0)])
    end
    assert_equal 0, err.exit_code
  end

  def test_proc_exit_raises_with_code_1
    wasi = WR::WasiStub.new
    proc_exit = wasi.resolve_function("wasi_snapshot_preview1", "proc_exit")
    err = assert_raises(WR::ProcExitError) do
      proc_exit.call([WE.i32(1)])
    end
    assert_equal 1, err.exit_code
  end

  def test_proc_exit_message_includes_code
    err = WR::ProcExitError.new(42)
    assert_equal 42, err.exit_code
    assert_includes err.message, "42"
  end

  # ── ProcExitError ──────────────────────────────────────────────────

  def test_proc_exit_error_is_standard_error
    assert WR::ProcExitError < StandardError
  end

  # ── fd_write ───────────────────────────────────────────────────────

  def test_fd_write_without_memory_returns_enosys
    wasi = WR::WasiStub.new
    fd_write = wasi.resolve_function("wasi_snapshot_preview1", "fd_write")
    result = fd_write.call([WE.i32(1), WE.i32(0), WE.i32(1), WE.i32(0)])
    assert_equal 1, result.length
    assert_equal 52, result[0].value # ENOSYS
  end

  def test_fd_write_captures_stdout
    captured = []
    wasi = WR::WasiStub.new(stdout_callback: ->(text) { captured << text })
    mem = WE::LinearMemory.new(1)

    # Set up an iov: buf_ptr=100, buf_len=5
    # Write "Hello" at offset 100
    mem.write_bytes(100, "Hello".b)
    # iov at offset 0: buf_ptr (i32 LE) = 100, buf_len (i32 LE) = 5
    mem.store_i32(0, 100)
    mem.store_i32(4, 5)

    wasi.set_memory(mem)

    fd_write = wasi.resolve_function("wasi_snapshot_preview1", "fd_write")
    # fd=1 (stdout), iovs_ptr=0, iovs_len=1, nwritten_ptr=16
    result = fd_write.call([WE.i32(1), WE.i32(0), WE.i32(1), WE.i32(16)])

    assert_equal 0, result[0].value # ESUCCESS
    assert_equal ["Hello"], captured
    # nwritten should be 5
    assert_equal 5, mem.load_i32(16)
  end

  def test_fd_read_reads_stdin
    wasi = WR::WasiHost.new(stdin: ->(_count) { "hi" })
    mem = WE::LinearMemory.new(1)
    wasi.set_memory(mem)
    mem.store_i32(0, 0x0200)
    mem.store_i32(4, 2)

    fd_read = wasi.resolve_function("wasi_snapshot_preview1", "fd_read")
    result = fd_read.call([WE.i32(0), WE.i32(0), WE.i32(1), WE.i32(0x0100)])

    assert_equal 0, result[0].value
    assert_equal 2, mem.load_i32(0x0100)
    assert_equal "h".ord, mem.load_i32_8u(0x0200)
    assert_equal "i".ord, mem.load_i32_8u(0x0201)
  end

  def test_fd_write_captures_stderr
    captured = []
    wasi = WR::WasiStub.new(stderr_callback: ->(text) { captured << text })
    mem = WE::LinearMemory.new(1)

    mem.write_bytes(100, "Error".b)
    mem.store_i32(0, 100)
    mem.store_i32(4, 5)

    wasi.set_memory(mem)

    fd_write = wasi.resolve_function("wasi_snapshot_preview1", "fd_write")
    # fd=2 (stderr)
    result = fd_write.call([WE.i32(2), WE.i32(0), WE.i32(1), WE.i32(16)])

    assert_equal 0, result[0].value
    assert_equal ["Error"], captured
  end

  def test_fd_write_multiple_iovs
    captured = []
    wasi = WR::WasiStub.new(stdout_callback: ->(text) { captured << text })
    mem = WE::LinearMemory.new(1)

    # First buffer: "Hi" at 200
    mem.write_bytes(200, "Hi".b)
    # Second buffer: " World" at 300
    mem.write_bytes(300, " World".b)

    # iovs at offset 0: two entries
    mem.store_i32(0, 200)   # iov[0].buf_ptr
    mem.store_i32(4, 2)     # iov[0].buf_len
    mem.store_i32(8, 300)   # iov[1].buf_ptr
    mem.store_i32(12, 6)    # iov[1].buf_len

    wasi.set_memory(mem)

    fd_write = wasi.resolve_function("wasi_snapshot_preview1", "fd_write")
    result = fd_write.call([WE.i32(1), WE.i32(0), WE.i32(2), WE.i32(20)])

    assert_equal 0, result[0].value
    assert_equal ["Hi", " World"], captured
    assert_equal 8, mem.load_i32(20) # total written
  end

  # ── set_memory ─────────────────────────────────────────────────────

  def test_set_memory
    wasi = WR::WasiStub.new
    mem = WE::LinearMemory.new(1)
    wasi.set_memory(mem)
    # No error means success; internal state is set
    assert true
  end

  # ── HostInterface inclusion ────────────────────────────────────────

  def test_wasi_stub_includes_host_interface
    assert WR::WasiStub.ancestors.include?(WE::HostInterface)
  end

  def test_wasi_stub_resolve_global_returns_nil
    wasi = WR::WasiStub.new
    assert_nil wasi.resolve_global("wasi_snapshot_preview1", "g")
  end

  def test_wasi_stub_resolve_memory_returns_nil
    wasi = WR::WasiStub.new
    assert_nil wasi.resolve_memory("wasi_snapshot_preview1", "m")
  end

  def test_wasi_stub_resolve_table_returns_nil
    wasi = WR::WasiStub.new
    assert_nil wasi.resolve_table("wasi_snapshot_preview1", "t")
  end
end
