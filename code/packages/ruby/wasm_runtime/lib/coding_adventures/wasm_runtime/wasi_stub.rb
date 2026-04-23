# frozen_string_literal: true

require "securerandom"

# ==========================================================================
# WasiStub --- WASI Host Implementation (Tier 1–3)
# ==========================================================================
#
# WASI (WebAssembly System Interface) is the standard ABI that connects
# WASM programs to the host environment. It is modelled after POSIX but
# intentionally capability-based: the host grants only the resources it
# chooses to expose.
#
# Tier breakdown for this implementation:
#
#   Tier 1 (fd_write, proc_exit)     — I/O capture, clean exit
#   Tier 3 (args, environ, clock, random, sched_yield) — new in this file
#
# All functions live under the module name "wasi_snapshot_preview1",
# which is the single WASI version that WASM 1.0 toolchains target.
#
# ==========================================================================
# DESIGN: Injectable Clock and Random
# ==========================================================================
#
# Clock and random behaviour must be injectable so tests can use
# deterministic fakes without touching system resources:
#
#   WasiClock duck-type interface:
#     realtime_ns         → Integer  (nanoseconds since Unix epoch)
#     monotonic_ns        → Integer  (nanoseconds since arbitrary start)
#     resolution_ns(id)   → Integer  (nanoseconds; 1_000_000 = 1 ms)
#
#   WasiRandom duck-type interface:
#     fill_bytes(n)       → Array<Integer>  (n bytes in 0..255)
#
# The defaults (SystemClock, SystemRandom) delegate to the OS via Ruby's
# Process.clock_gettime and SecureRandom. Tests inject FakeClock/FakeRandom.
#
# ==========================================================================

module CodingAdventures
  module WasmRuntime
    # ── SystemClock ─────────────────────────────────────────────────────────
    #
    # Production clock backed by the operating system.
    #
    # CLOCK_REALTIME is the wall-clock time; it can jump forward or backward
    # when the administrator adjusts the system clock.
    #
    # CLOCK_MONOTONIC is guaranteed to only move forward, making it suitable
    # for measuring elapsed time (timeouts, benchmarks). The epoch is arbitrary
    # — typically system boot.
    #
    # resolution_ns returns a conservative 1 ms, which is correct for most
    # modern operating systems (Linux CLOCK_REALTIME resolution ≈ 1 ns, but
    # macOS CLOCK_REALTIME can be coarser).
    class SystemClock
      # Wall-clock time in nanoseconds since the Unix epoch (1 Jan 1970 UTC).
      def realtime_ns
        Process.clock_gettime(Process::CLOCK_REALTIME, :nanosecond)
      end

      # Monotonic time in nanoseconds since an arbitrary fixed start.
      def monotonic_ns
        Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)
      end

      # Clock resolution in nanoseconds for clock_id +id+.
      # We return 1 ms (1_000_000 ns) as a conservative but universally
      # correct value. A WASM program that needs higher precision should
      # query the actual clock and compare consecutive readings.
      def resolution_ns(_clock_id)
        1_000_000 # 1 millisecond
      end
    end

    # ── SystemRandom ─────────────────────────────────────────────────────────
    #
    # Production CSPRNG backed by SecureRandom (which uses the OS entropy
    # pool: /dev/urandom on Linux/macOS, CryptGenRandom on Windows).
    class SystemRandom
      # Return +n+ cryptographically random bytes as an Array of Integers
      # each in 0..255.
      def fill_bytes(n)
        SecureRandom.random_bytes(n).bytes
      end
    end

    # ── ProcExitError ───────────────────────────────────────────────────────
    #
    # Thrown when a WASM program calls proc_exit. The exit_code mirrors
    # the integer the WASM program passed to proc_exit.
    #
    # Callers that embed WasmRuntime should rescue ProcExitError and
    # decide what to do with the code (log it, re-raise, etc.). This is
    # analogous to catching SystemExit in Python.
    class ProcExitError < StandardError
      attr_reader :exit_code

      def initialize(exit_code)
        @exit_code = exit_code
        super("proc_exit(#{exit_code})")
      end
    end

    # ── WasiStub ─────────────────────────────────────────────────────────────
    #
    # Implements the WASI snapshot_preview1 ABI up to Tier 3.
    #
    # Constructor keyword arguments:
    #
    #   args:    Array<String>   argv passed to the WASM program (default [])
    #   env:     Hash<String,String> environment variables (default {})
    #   stdout:  Proc | nil     called with each stdout chunk (default nil → discard)
    #   stderr:  Proc | nil     called with each stderr chunk (default nil → discard)
    #   clock:   SystemClock    duck-typed clock object (default SystemClock.new)
    #   random:  SystemRandom   duck-typed random object (default SystemRandom.new)
    #
    # The object is connected to a LinearMemory via set_memory, which the
    # WasmRuntime instantiation step calls automatically.
    class WasiStub
      include WasmExecution::HostInterface

      # WASI errno values used in this file.
      ESUCCESS = 0   # No error
      EBADF = 8    # Bad file descriptor
      EINVAL = 28  # Invalid argument (e.g. unknown clock id)
      ENOSYS = 52  # Function not implemented

      # WASI clock IDs (from the WASI spec):
      #   0 = REALTIME   — wall clock
      #   1 = MONOTONIC  — monotonic clock
      #   2 = PROCESS_CPUTIME_ID — per-process CPU time (we map to realtime)
      #   3 = THREAD_CPUTIME_ID  — per-thread CPU time  (we map to realtime)
      CLOCK_REALTIME = 0
      CLOCK_MONOTONIC = 1
      CLOCK_PROCESS_CPU = 2
      CLOCK_THREAD_CPU = 3

      def initialize(args: [], env: {}, stdin: nil, stdout: nil, stderr: nil,
        clock: SystemClock.new, random: SystemRandom.new,
        # Legacy keyword aliases kept for backward compatibility:
        stdout_callback: nil, stderr_callback: nil)
        @args = args
        @env = env
        @stdin_callback = stdin || ->(_count) { "".b }
        # Prefer the new-style keyword; fall back to legacy if given.
        @stdout_callback = stdout || stdout_callback || ->(_text) {}
        @stderr_callback = stderr || stderr_callback || ->(_text) {}
        @clock = clock
        @random = random
        @instance_memory = nil
      end

      # Called by the runtime to attach linear memory to the stub.
      def set_memory(memory)
        @instance_memory = memory
      end

      # Resolve a WASI host function by module + name.
      # Returns a WasmExecution::HostFunction or nil if the module is unknown.
      def resolve_function(module_name, name)
        return nil unless module_name == "wasi_snapshot_preview1"

        case name
        when "fd_write" then make_fd_write
        when "fd_read" then make_fd_read
        when "proc_exit" then make_proc_exit
        when "args_sizes_get" then make_args_sizes_get
        when "args_get" then make_args_get
        when "environ_sizes_get" then make_environ_sizes_get
        when "environ_get" then make_environ_get
        when "clock_res_get" then make_clock_res_get
        when "clock_time_get" then make_clock_time_get
        when "random_get" then make_random_get
        when "sched_yield" then make_sched_yield
        else make_stub(name)
        end
      end

      private

      # ── Helpers ──────────────────────────────────────────────────────────

      # Build a HostFunction from params, results, and a lambda.
      #
      # We use a lambda (not a Proc block) because inside a lambda `return`
      # exits only the lambda. Inside a bare Proc/block passed to a method,
      # `return` tries to exit the DEFINING method, which causes LocalJumpError
      # when the block is later called from a different context (HostFunction#call).
      def host_fn(params, results, impl)
        WasmExecution::HostFunction.new(
          func_type: WasmTypes::FuncType.new(params, results),
          implementation: impl
        )
      end

      # Convert the args array to a list of null-terminated UTF-8 byte arrays.
      # Used by both args_sizes_get and args_get.
      def encoded_args
        @args.map { |a| a.encode("UTF-8").bytes + [0] }
      end

      # Convert the env hash to a list of null-terminated "KEY=VALUE" byte arrays.
      # Used by both environ_sizes_get and environ_get.
      def encoded_env
        @env.map { |k, v| "#{k}=#{v}".encode("UTF-8").bytes + [0] }
      end

      # ── Tier 1: fd_write ─────────────────────────────────────────────────
      #
      # fd_write(fd, iovs_ptr, iovs_len, nwritten_ptr) → errno
      #
      # Writes scatter/gather buffers described by an iovec array to the given
      # file descriptor. WASI defines:
      #
      #   struct ciovec { buf: i32, buf_len: i32 }
      #
      # Each entry is 8 bytes: a 4-byte pointer followed by a 4-byte length.
      # fd 1 = stdout, fd 2 = stderr.
      def make_fd_write
        stub = self
        WasmExecution::HostFunction.new(
          func_type: WasmTypes::FuncType.new(
            [WasmTypes::VALUE_TYPE[:i32]] * 4,
            [WasmTypes::VALUE_TYPE[:i32]]
          ),
          implementation: ->(args) {
            fd = args[0].value
            iovs_ptr = args[1].value
            iovs_len = args[2].value
            nwritten_ptr = args[3].value

            return [WasmExecution.i32(ENOSYS)] unless stub.instance_variable_get(:@instance_memory)
            memory = stub.instance_variable_get(:@instance_memory)

            total_written = 0
            iovs_len.times do |i|
              buf_ptr = WasmExecution.to_u32(memory.load_i32(iovs_ptr + i * 8))
              buf_len = WasmExecution.to_u32(memory.load_i32(iovs_ptr + i * 8 + 4))

              bytes = buf_len.times.map { |j| memory.load_i32_8u(buf_ptr + j) }
              text = bytes.pack("C*").force_encoding("UTF-8")
              total_written += buf_len

              case fd
              when 1 then stub.instance_variable_get(:@stdout_callback).call(text)
              when 2 then stub.instance_variable_get(:@stderr_callback).call(text)
              end
            end

            memory.store_i32(nwritten_ptr, total_written)
            [WasmExecution.i32(ESUCCESS)]
          }
        )
      end

      # fd_read(fd, iovs_ptr, iovs_len, nread_ptr) → errno
      #
      # Reads bytes from stdin into the guest buffers described by the iovec
      # array. Only fd 0 is supported.
      def make_fd_read
        stub = self
        WasmExecution::HostFunction.new(
          func_type: WasmTypes::FuncType.new(
            [WasmTypes::VALUE_TYPE[:i32]] * 4,
            [WasmTypes::VALUE_TYPE[:i32]]
          ),
          implementation: ->(args) {
            fd = args[0].value
            iovs_ptr = args[1].value
            iovs_len = args[2].value
            nread_ptr = args[3].value

            return [WasmExecution.i32(ENOSYS)] unless stub.instance_variable_get(:@instance_memory)
            return [WasmExecution.i32(EBADF)] unless fd == 0

            memory = stub.instance_variable_get(:@instance_memory)
            total_read = 0

            iovs_len.times do |i|
              buf_ptr = WasmExecution.to_u32(memory.load_i32(iovs_ptr + i * 8))
              buf_len = WasmExecution.to_u32(memory.load_i32(iovs_ptr + i * 8 + 4))

              raw_chunk = stub.instance_variable_get(:@stdin_callback).call(buf_len)
              chunk =
                case raw_chunk
                when nil then +""
                when String then raw_chunk.b
                else raw_chunk.pack("C*")
                end
              chunk = chunk.byteslice(0, buf_len) || +""

              chunk.bytes.each_with_index do |byte, offset|
                memory.store_i32_8(buf_ptr + offset, byte)
              end

              total_read += chunk.bytesize
              break if chunk.bytesize < buf_len
            end

            memory.store_i32(nread_ptr, total_read)
            [WasmExecution.i32(ESUCCESS)]
          }
        )
      end

      # ── Tier 1: proc_exit ────────────────────────────────────────────────
      #
      # proc_exit(code) — no return value
      #
      # Terminates the WASM program with the given exit code. We model this
      # as a Ruby exception (ProcExitError) so the caller can catch it cleanly
      # without crashing the host process.
      def make_proc_exit
        WasmExecution::HostFunction.new(
          func_type: WasmTypes::FuncType.new(
            [WasmTypes::VALUE_TYPE[:i32]], []
          ),
          implementation: ->(args) {
            exit_code = args[0].value
            raise ProcExitError.new(exit_code)
          }
        )
      end

      # ── Tier 3: args_sizes_get ───────────────────────────────────────────
      #
      # args_sizes_get(argc_ptr, argv_buf_size_ptr) → errno
      #
      # Returns two sizes needed to allocate the argv buffers:
      #   argc          — number of arguments (written as i32 at argc_ptr)
      #   argv_buf_size — total bytes for all null-terminated argument strings
      #                   (written as i32 at argv_buf_size_ptr)
      #
      # A WASM program typically calls args_sizes_get first to allocate space,
      # then calls args_get to fill in the actual strings. This two-phase pattern
      # avoids fixed-size buffers and buffer overflow.
      #
      # Example with args = ["myapp", "hello"]:
      #   argc          = 2
      #   argv_buf_size = len("myapp\0") + len("hello\0") = 6 + 6 = 12
      def make_args_sizes_get
        stub = self
        host_fn(
          [WasmTypes::VALUE_TYPE[:i32], WasmTypes::VALUE_TYPE[:i32]],
          [WasmTypes::VALUE_TYPE[:i32]],
          ->(args) {
            argc_ptr = args[0].value & 0xFFFFFFFF
            argv_buf_size_ptr = args[1].value & 0xFFFFFFFF
            memory = stub.instance_variable_get(:@instance_memory)
            return [WasmExecution.i32(ENOSYS)] unless memory

            enc = stub.send(:encoded_args)
            argc = enc.length
            buf_size = enc.sum(&:length)

            memory.store_i32(argc_ptr, argc)
            memory.store_i32(argv_buf_size_ptr, buf_size)
            [WasmExecution.i32(ESUCCESS)]
          }
        )
      end

      # ── Tier 3: args_get ─────────────────────────────────────────────────
      #
      # args_get(argv_ptr, argv_buf_ptr) → errno
      #
      # Fills two pre-allocated regions:
      #   argv_ptr     — an array of i32 pointers, one per argument
      #   argv_buf_ptr — the packed null-terminated argument strings
      #
      # After this call the WASM program can read argv[i] to get a pointer
      # into argv_buf that contains the i-th argument string.
      #
      # Memory layout after args_get(["myapp", "hello"], argv_ptr=0, buf_ptr=100):
      #
      #   argv[0] at 0   → 100   (pointer to "myapp\0")
      #   argv[1] at 4   → 106   (pointer to "hello\0")
      #   buf[0..5]      = 'm','y','a','p','p',0
      #   buf[6..11]     = 'h','e','l','l','o',0
      def make_args_get
        stub = self
        host_fn(
          [WasmTypes::VALUE_TYPE[:i32], WasmTypes::VALUE_TYPE[:i32]],
          [WasmTypes::VALUE_TYPE[:i32]],
          ->(args) {
            argv_ptr = args[0].value & 0xFFFFFFFF
            argv_buf_ptr = args[1].value & 0xFFFFFFFF
            memory = stub.instance_variable_get(:@instance_memory)
            return [WasmExecution.i32(ENOSYS)] unless memory

            offset = argv_buf_ptr
            stub.send(:encoded_args).each_with_index do |bytes, i|
              # Write the pointer to this string into the argv array.
              memory.store_i32(argv_ptr + i * 4, offset)
              # Write the null-terminated string bytes into the buffer.
              bytes.each_with_index do |b, j|
                memory.store_i64_8(offset + j, b)
              end
              offset += bytes.length
            end
            [WasmExecution.i32(ESUCCESS)]
          }
        )
      end

      # ── Tier 3: environ_sizes_get ─────────────────────────────────────────
      #
      # environ_sizes_get(count_ptr, buf_size_ptr) → errno
      #
      # Analogous to args_sizes_get but for environment variables.
      # Each variable is serialised as "KEY=VALUE\0".
      #
      # Example with env = {"HOME" => "/home/user"}:
      #   count    = 1
      #   buf_size = len("HOME=/home/user\0") = 16 bytes
      def make_environ_sizes_get
        stub = self
        host_fn(
          [WasmTypes::VALUE_TYPE[:i32], WasmTypes::VALUE_TYPE[:i32]],
          [WasmTypes::VALUE_TYPE[:i32]],
          ->(args) {
            count_ptr = args[0].value & 0xFFFFFFFF
            buf_size_ptr = args[1].value & 0xFFFFFFFF
            memory = stub.instance_variable_get(:@instance_memory)
            return [WasmExecution.i32(ENOSYS)] unless memory

            enc = stub.send(:encoded_env)
            count = enc.length
            buf_size = enc.sum(&:length)

            memory.store_i32(count_ptr, count)
            memory.store_i32(buf_size_ptr, buf_size)
            [WasmExecution.i32(ESUCCESS)]
          }
        )
      end

      # ── Tier 3: environ_get ───────────────────────────────────────────────
      #
      # environ_get(environ_ptr, environ_buf_ptr) → errno
      #
      # Fills two pre-allocated regions, same pattern as args_get:
      #   environ_ptr     — array of i32 pointers, one per variable
      #   environ_buf_ptr — packed null-terminated "KEY=VALUE\0" strings
      def make_environ_get
        stub = self
        host_fn(
          [WasmTypes::VALUE_TYPE[:i32], WasmTypes::VALUE_TYPE[:i32]],
          [WasmTypes::VALUE_TYPE[:i32]],
          ->(args) {
            environ_ptr = args[0].value & 0xFFFFFFFF
            environ_buf_ptr = args[1].value & 0xFFFFFFFF
            memory = stub.instance_variable_get(:@instance_memory)
            return [WasmExecution.i32(ENOSYS)] unless memory

            offset = environ_buf_ptr
            stub.send(:encoded_env).each_with_index do |bytes, i|
              memory.store_i32(environ_ptr + i * 4, offset)
              bytes.each_with_index do |b, j|
                memory.store_i64_8(offset + j, b)
              end
              offset += bytes.length
            end
            [WasmExecution.i32(ESUCCESS)]
          }
        )
      end

      # ── Tier 3: clock_res_get ─────────────────────────────────────────────
      #
      # clock_res_get(id, resolution_ptr) → errno
      #
      # Writes the clock resolution (in nanoseconds) as an i64 to
      # resolution_ptr. A resolution of 1_000_000 ns means the clock
      # ticks at most every 1 ms.
      #
      # The i64 write uses store_i64, which is available on LinearMemory.
      def make_clock_res_get
        stub = self
        host_fn(
          [WasmTypes::VALUE_TYPE[:i32], WasmTypes::VALUE_TYPE[:i32]],
          [WasmTypes::VALUE_TYPE[:i32]],
          ->(args) {
            clock_id = args[0].value
            resolution_ptr = args[1].value & 0xFFFFFFFF
            memory = stub.instance_variable_get(:@instance_memory)
            return [WasmExecution.i32(ENOSYS)] unless memory

            res_ns = stub.instance_variable_get(:@clock).resolution_ns(clock_id)
            memory.store_i64(resolution_ptr, res_ns)
            [WasmExecution.i32(ESUCCESS)]
          }
        )
      end

      # ── Tier 3: clock_time_get ────────────────────────────────────────────
      #
      # clock_time_get(id, precision, time_ptr) → errno
      #
      # Returns the current time for the given clock as an i64 nanosecond
      # count written to time_ptr. The +precision+ argument is an i64 hint
      # (requested precision in ns) but we ignore it — our clocks always
      # return at their natural resolution.
      #
      # WASI clock IDs:
      #   0 CLOCK_REALTIME          → wall clock nanoseconds since epoch
      #   1 CLOCK_MONOTONIC         → monotonic nanoseconds since boot
      #   2 CLOCK_PROCESS_CPUTIME   → we map to realtime (conservative)
      #   3 CLOCK_THREAD_CPUTIME    → we map to realtime (conservative)
      #   other → EINVAL
      #
      # Note: clock_time_get's +precision+ param is i64 (64-bit), so the
      # FuncType must declare it as VALUE_TYPE[:i64].
      def make_clock_time_get
        stub = self
        host_fn(
          [
            WasmTypes::VALUE_TYPE[:i32],  # id
            WasmTypes::VALUE_TYPE[:i64],  # precision (hint, ignored)
            WasmTypes::VALUE_TYPE[:i32]   # time_ptr
          ],
          [WasmTypes::VALUE_TYPE[:i32]],
          ->(args) {
            clock_id = args[0].value
            # args[1] is precision — we intentionally ignore it
            time_ptr = args[2].value & 0xFFFFFFFF
            memory = stub.instance_variable_get(:@instance_memory)
            return [WasmExecution.i32(ENOSYS)] unless memory

            clk = stub.instance_variable_get(:@clock)
            ns = case clock_id
            when CLOCK_REALTIME, CLOCK_PROCESS_CPU, CLOCK_THREAD_CPU
              clk.realtime_ns
            when CLOCK_MONOTONIC
              clk.monotonic_ns
            else
              return [WasmExecution.i32(EINVAL)]
            end

            memory.store_i64(time_ptr, ns)
            [WasmExecution.i32(ESUCCESS)]
          }
        )
      end

      # ── Tier 3: random_get ────────────────────────────────────────────────
      #
      # random_get(buf_ptr, buf_len) → errno
      #
      # Fills buf_len bytes at buf_ptr with cryptographically random data.
      # We delegate to @random.fill_bytes(n) which returns an Array of
      # Integers in 0..255. Each byte is written with store_i64_8.
      #
      # WASM programs use random_get for:
      #   - Seeding their own PRNG
      #   - Generating UUIDs
      #   - Hash table seed randomisation (HashDoS mitigation)
      def make_random_get
        stub = self
        host_fn(
          [WasmTypes::VALUE_TYPE[:i32], WasmTypes::VALUE_TYPE[:i32]],
          [WasmTypes::VALUE_TYPE[:i32]],
          ->(args) {
            buf_ptr = args[0].value & 0xFFFFFFFF
            buf_len = args[1].value & 0x7FFFFFFF
            memory = stub.instance_variable_get(:@instance_memory)
            return [WasmExecution.i32(ENOSYS)] unless memory

            bytes = stub.instance_variable_get(:@random).fill_bytes(buf_len)
            bytes.each_with_index do |b, i|
              memory.store_i64_8(buf_ptr + i, b)
            end
            [WasmExecution.i32(ESUCCESS)]
          }
        )
      end

      # ── Tier 3: sched_yield ───────────────────────────────────────────────
      #
      # sched_yield() → errno
      #
      # Voluntarily yields the CPU to another thread or process. In a
      # single-threaded WASM host like ours there is nothing to yield to,
      # so we return success immediately. This keeps WASM programs that
      # call sched_yield in spin-wait loops from getting ENOSYS errors.
      def make_sched_yield
        host_fn(
          [],
          [WasmTypes::VALUE_TYPE[:i32]],
          ->(_args) { [WasmExecution.i32(ESUCCESS)] }
        )
      end

      # ── Fallback stub ─────────────────────────────────────────────────────
      #
      # Any WASI function we haven't implemented returns ENOSYS (52).
      # This is the correct POSIX errno for "Function not implemented".
      def make_stub(_name)
        WasmExecution::HostFunction.new(
          func_type: WasmTypes::FuncType.new([], [WasmTypes::VALUE_TYPE[:i32]]),
          implementation: ->(_args) { [WasmExecution.i32(ENOSYS)] }
        )
      end
    end

    WasiHost = WasiStub
  end
end
