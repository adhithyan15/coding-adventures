# frozen_string_literal: true

# ==========================================================================
# WasiStub --- Minimal WASI Host Implementation
# ==========================================================================
#
# Provides fd_write (stdout/stderr capture) and proc_exit. All other
# WASI functions return ENOSYS (52 = not implemented).
# ==========================================================================

module CodingAdventures
  module WasmRuntime
    # Thrown when a WASM program calls proc_exit.
    class ProcExitError < StandardError
      attr_reader :exit_code

      def initialize(exit_code)
        @exit_code = exit_code
        super("proc_exit(#{exit_code})")
      end
    end

    # A minimal WASI host.
    class WasiStub
      include WasmExecution::HostInterface

      ENOSYS = 52
      ESUCCESS = 0

      def initialize(stdout_callback: nil, stderr_callback: nil)
        @stdout_callback = stdout_callback || ->(_text) {}
        @stderr_callback = stderr_callback || ->(_text) {}
        @instance_memory = nil
      end

      def set_memory(memory)
        @instance_memory = memory
      end

      def resolve_function(module_name, name)
        return nil unless module_name == "wasi_snapshot_preview1"

        case name
        when "fd_write" then make_fd_write
        when "proc_exit" then make_proc_exit
        else make_stub(name)
        end
      end

      private

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

      def make_stub(_name)
        WasmExecution::HostFunction.new(
          func_type: WasmTypes::FuncType.new([], [WasmTypes::VALUE_TYPE[:i32]]),
          implementation: ->(_args) { [WasmExecution.i32(ENOSYS)] }
        )
      end
    end
  end
end
