# frozen_string_literal: true

module CodingAdventures
  module BoardVM
    class UnknownSessionCommandError < ArgumentError; end

    class Session
      DEFAULT_HOST_NAME = "ruby-board-vm"
      GPIO_READ_MODES = {
        input: 0,
        input_pullup: 2,
        pullup: 2,
        input_pulldown: 3,
        pulldown: 3
      }.freeze
      GPIO_MODES = GPIO_READ_MODES.merge(output: 1).freeze
      GPIO_WRITE_VALUES = {
        high: true,
        on: true,
        :"true" => true,
        low: false,
        off: false,
        :"false" => false
      }.freeze
      BOOT_POLICIES = {
        store_only: 0,
        run_at_boot: 1,
        run_if_no_host: 2
      }.freeze
      RUN_FLAG_RESET_VM_BEFORE_RUN = 0x01
      RUN_FLAG_KEEP_HANDLES_AFTER_RUN = 0x02
      RUN_FLAG_BACKGROUND_RUN = 0x04
      DEFAULT_RUN_FLAGS = RUN_FLAG_RESET_VM_BEFORE_RUN | RUN_FLAG_BACKGROUND_RUN
      RUN_FLAGS = {
        reset_vm_before_run: RUN_FLAG_RESET_VM_BEFORE_RUN,
        reset_vm: RUN_FLAG_RESET_VM_BEFORE_RUN,
        keep_handles_after_run: RUN_FLAG_KEEP_HANDLES_AFTER_RUN,
        keep_handles: RUN_FLAG_KEEP_HANDLES_AFTER_RUN,
        background_run: RUN_FLAG_BACKGROUND_RUN,
        background: RUN_FLAG_BACKGROUND_RUN
      }.freeze

      attr_reader :connection, :native_session, :host_name, :host_nonce, :program_id,
        :instruction_budget

      def initialize(
        connection,
        native_session: Native::Session.new,
        host_name: DEFAULT_HOST_NAME,
        host_nonce: DEFAULT_HOST_NONCE,
        program_id: DEFAULT_PROGRAM_ID,
        instruction_budget: DEFAULT_INSTRUCTION_BUDGET
      )
        @connection = connection
        @native_session = native_session
        @host_name = host_name
        @host_nonce = host_nonce
        @program_id = program_id
        @instruction_budget = instruction_budget
      end

      def hello(host_name: @host_name, host_nonce: @host_nonce)
        dispatch(:hello, native_session.hello_wire(host_name, host_nonce))
      end

      def capabilities
        dispatch(:capabilities, native_session.caps_query_wire)
      end
      alias caps capabilities

      def board_descriptor
        capabilities.board_descriptor
      end
      alias describe board_descriptor

      def smoke(host_name: @host_name, host_nonce: @host_nonce, query_caps: true)
        results = [hello(host_name: host_name, host_nonce: host_nonce)]
        results << capabilities if query_caps
        SessionResult.new(results: results)
      end

      def upload(program_id: @program_id, module_bytes:)
        SessionResult.new(results: [
          dispatch(:program_begin, native_session.program_begin_wire(program_id, module_bytes)),
          dispatch(:program_chunk, native_session.program_chunk_wire(program_id, 0, module_bytes)),
          dispatch(:program_end, native_session.program_end_wire(program_id))
        ])
      end

      def store_program(program_id: @program_id, slot: 0, boot_policy: DEFAULT_BOOT_POLICY)
        dispatch(
          :store_program,
          native_session.store_program_wire(program_id, slot, boot_policy_value(boot_policy))
        )
      end

      def upload_blink(
        program_id: @program_id,
        pin: 13,
        high_ms: 250,
        low_ms: 250,
        max_stack: 4
      )
        upload(
          program_id: program_id,
          module_bytes: native_session.blink_module(pin, high_ms, low_ms, max_stack)
        )
      end

      def gpio_read_module(pin:, mode: :input, max_stack: 2)
        native_session.gpio_read_module(pin, gpio_read_mode(mode), max_stack)
      end

      def gpio_write_module(pin:, value:, max_stack: 3)
        native_session.gpio_write_module(pin, gpio_write_value(value) ? 1 : 0, max_stack)
      end

      def gpio_open_module(pin:, mode: :output, max_stack: 2)
        native_session.gpio_open_module(pin, gpio_mode(mode), max_stack)
      end

      def gpio_handle_read_module(max_stack: 2)
        native_session.gpio_handle_read_module(max_stack)
      end

      def gpio_handle_write_module(value:, max_stack: 3)
        native_session.gpio_handle_write_module(gpio_write_value(value) ? 1 : 0, max_stack)
      end

      def gpio_handle_close_module(max_stack: 1)
        native_session.gpio_handle_close_module(max_stack)
      end

      def upload_gpio_read(
        program_id: @program_id,
        pin:,
        mode: :input,
        max_stack: 2
      )
        upload(
          program_id: program_id,
          module_bytes: gpio_read_module(pin: pin, mode: mode, max_stack: max_stack)
        )
      end

      def upload_gpio_write(
        program_id: @program_id,
        pin:,
        value:,
        max_stack: 3
      )
        upload(
          program_id: program_id,
          module_bytes: gpio_write_module(pin: pin, value: value, max_stack: max_stack)
        )
      end

      def upload_gpio_open(
        program_id: @program_id,
        pin:,
        mode: :output,
        max_stack: 2
      )
        upload(
          program_id: program_id,
          module_bytes: gpio_open_module(pin: pin, mode: mode, max_stack: max_stack)
        )
      end

      def upload_gpio_handle_read(program_id: @program_id, max_stack: 2)
        upload(
          program_id: program_id,
          module_bytes: gpio_handle_read_module(max_stack: max_stack)
        )
      end

      def upload_gpio_handle_write(program_id: @program_id, value:, max_stack: 3)
        upload(
          program_id: program_id,
          module_bytes: gpio_handle_write_module(value: value, max_stack: max_stack)
        )
      end

      def upload_gpio_handle_close(program_id: @program_id, max_stack: 1)
        upload(
          program_id: program_id,
          module_bytes: gpio_handle_close_module(max_stack: max_stack)
        )
      end

      def time_now_module(max_stack: 1)
        native_session.time_now_module(max_stack)
      end

      def time_sleep_ms_module(duration_ms:, max_stack: 1)
        native_session.time_sleep_ms_module(duration_ms, max_stack)
      end

      def raw_module(code:, max_stack:, flags: 0, const_pool: +"")
        native_session.raw_module(flags, max_stack, code.b, const_pool.b)
      end
      alias module raw_module

      def upload_time_now(program_id: @program_id, max_stack: 1)
        upload(
          program_id: program_id,
          module_bytes: time_now_module(max_stack: max_stack)
        )
      end

      def upload_time_sleep_ms(program_id: @program_id, duration_ms:, max_stack: 1)
        upload(
          program_id: program_id,
          module_bytes: time_sleep_ms_module(duration_ms: duration_ms, max_stack: max_stack)
        )
      end

      def upload_raw_module(program_id: @program_id, code:, max_stack:, flags: 0, const_pool: +"")
        upload(
          program_id: program_id,
          module_bytes: raw_module(
            code: code,
            max_stack: max_stack,
            flags: flags,
            const_pool: const_pool
          )
        )
      end

      def run(
        program_id: @program_id,
        budget: @instruction_budget,
        instruction_budget: nil,
        flags: nil,
        reset_vm: true,
        keep_handles: false,
        background: true,
        time_budget_ms: 0
      )
        dispatch(
          :run,
          native_session.run_wire(
            program_id,
            run_flags(
              flags: flags,
              reset_vm: reset_vm,
              keep_handles: keep_handles,
              background: background
            ),
            instruction_budget || budget,
            time_budget_ms
          )
        )
      end

      def stop
        dispatch(:stop, native_session.stop_wire)
      end

      def blink(
        program_id: @program_id,
        budget: @instruction_budget,
        instruction_budget: nil,
        pin: 13,
        high_ms: 250,
        low_ms: 250,
        max_stack: 4,
        handshake: false,
        query_caps: false,
        host_name: @host_name,
        host_nonce: @host_nonce
      )
        results = []
        results << hello(host_name: host_name, host_nonce: host_nonce) if handshake
        results << capabilities if query_caps
        results.concat(
          upload_blink(
            program_id: program_id,
            pin: pin,
            high_ms: high_ms,
            low_ms: low_ms,
            max_stack: max_stack
          ).results
        )
        results << run(
          program_id: program_id,
          instruction_budget: instruction_budget || budget
        )
        SessionResult.new(results: results)
      end

      def gpio_read(
        program_id: @program_id,
        budget: @instruction_budget,
        instruction_budget: nil,
        pin:,
        mode: :input,
        max_stack: 2,
        handshake: false,
        query_caps: false,
        host_name: @host_name,
        host_nonce: @host_nonce
      )
        results = []
        results << hello(host_name: host_name, host_nonce: host_nonce) if handshake
        results << capabilities if query_caps
        results.concat(
          upload_gpio_read(
            program_id: program_id,
            pin: pin,
            mode: mode,
            max_stack: max_stack
          ).results
        )
        results << run(
          program_id: program_id,
          instruction_budget: instruction_budget || budget
        )
        SessionResult.new(results: results)
      end

      def gpio_write(
        program_id: @program_id,
        budget: @instruction_budget,
        instruction_budget: nil,
        pin:,
        value:,
        max_stack: 3,
        handshake: false,
        query_caps: false,
        host_name: @host_name,
        host_nonce: @host_nonce
      )
        results = []
        results << hello(host_name: host_name, host_nonce: host_nonce) if handshake
        results << capabilities if query_caps
        results.concat(
          upload_gpio_write(
            program_id: program_id,
            pin: pin,
            value: value,
            max_stack: max_stack
          ).results
        )
        results << run(
          program_id: program_id,
          instruction_budget: instruction_budget || budget
        )
        SessionResult.new(results: results)
      end

      def gpio_high(pin:, **options)
        gpio_write(pin: pin, value: true, **options)
      end

      def gpio_low(pin:, **options)
        gpio_write(pin: pin, value: false, **options)
      end

      def gpio_open(
        program_id: @program_id,
        budget: @instruction_budget,
        instruction_budget: nil,
        pin:,
        mode: :output,
        max_stack: 2,
        handshake: false,
        query_caps: false,
        host_name: @host_name,
        host_nonce: @host_nonce
      )
        results = []
        results << hello(host_name: host_name, host_nonce: host_nonce) if handshake
        results << capabilities if query_caps
        results.concat(
          upload_gpio_open(
            program_id: program_id,
            pin: pin,
            mode: mode,
            max_stack: max_stack
          ).results
        )
        results << run(
          program_id: program_id,
          instruction_budget: instruction_budget || budget,
          keep_handles: true,
          background: false
        )
        SessionResult.new(results: results)
      end

      def gpio_handle_read(
        program_id: @program_id,
        budget: @instruction_budget,
        instruction_budget: nil,
        max_stack: 2
      )
        results = upload_gpio_handle_read(program_id: program_id, max_stack: max_stack).results
        results << run(
          program_id: program_id,
          instruction_budget: instruction_budget || budget,
          reset_vm: false,
          keep_handles: true,
          background: false
        )
        SessionResult.new(results: results)
      end

      def gpio_handle_write(
        program_id: @program_id,
        budget: @instruction_budget,
        instruction_budget: nil,
        value:,
        max_stack: 3
      )
        results = upload_gpio_handle_write(
          program_id: program_id,
          value: value,
          max_stack: max_stack
        ).results
        results << run(
          program_id: program_id,
          instruction_budget: instruction_budget || budget,
          reset_vm: false,
          keep_handles: true,
          background: false
        )
        SessionResult.new(results: results)
      end

      def gpio_handle_close(
        program_id: @program_id,
        budget: @instruction_budget,
        instruction_budget: nil,
        max_stack: 1
      )
        results = upload_gpio_handle_close(program_id: program_id, max_stack: max_stack).results
        results << run(
          program_id: program_id,
          instruction_budget: instruction_budget || budget,
          reset_vm: false,
          background: false
        )
        SessionResult.new(results: results)
      end

      def time_now(
        program_id: @program_id,
        budget: @instruction_budget,
        instruction_budget: nil,
        max_stack: 1,
        handshake: false,
        query_caps: false,
        host_name: @host_name,
        host_nonce: @host_nonce
      )
        results = []
        results << hello(host_name: host_name, host_nonce: host_nonce) if handshake
        results << capabilities if query_caps
        results.concat(upload_time_now(program_id: program_id, max_stack: max_stack).results)
        results << run(
          program_id: program_id,
          instruction_budget: instruction_budget || budget
        )
        SessionResult.new(results: results)
      end

      def time_sleep_ms(
        duration_ms:,
        program_id: @program_id,
        budget: @instruction_budget,
        instruction_budget: nil,
        max_stack: 1,
        handshake: false,
        query_caps: false,
        host_name: @host_name,
        host_nonce: @host_nonce
      )
        results = []
        results << hello(host_name: host_name, host_nonce: host_nonce) if handshake
        results << capabilities if query_caps
        results.concat(
          upload_time_sleep_ms(
            program_id: program_id,
            duration_ms: duration_ms,
            max_stack: max_stack
          ).results
        )
        results << run(
          program_id: program_id,
          instruction_budget: instruction_budget || budget
        )
        SessionResult.new(results: results)
      end
      alias sleep_ms time_sleep_ms

      def run_command(line, **options)
        words = line.to_s.split
        command = words.shift
        return SessionResult.new if command.nil?

        case command
        when "hello"
          ensure_no_extra_arguments!(words, command)
          SessionResult.new(results: [hello(**options)])
        when "smoke"
          ensure_no_extra_arguments!(words, command)
          smoke(**options)
        when "caps", "capabilities"
          ensure_no_extra_arguments!(words, command)
          SessionResult.new(results: [capabilities])
        when "upload-blink"
          ensure_no_extra_arguments!(words, command)
          upload_blink(**options)
        when "upload-gpio-read", "upload-gpio.read"
          upload_gpio_read(**gpio_read_command_options(words, command, options, require_budget: false))
        when "upload-gpio-write", "upload-gpio.write"
          upload_gpio_write(**gpio_write_command_options(words, command, options, require_budget: false))
        when "upload-gpio-open", "upload-gpio.open"
          upload_gpio_open(**gpio_open_command_options(words, command, options, require_budget: false))
        when "upload-gpio-handle-read", "upload-gpio.handle-read"
          ensure_no_extra_arguments!(words, command)
          upload_gpio_handle_read(**options)
        when "upload-gpio-handle-write", "upload-gpio.handle-write"
          upload_gpio_handle_write(**gpio_handle_write_command_options(words, command, options, require_budget: false))
        when "upload-gpio-handle-close", "upload-gpio.handle-close"
          ensure_no_extra_arguments!(words, command)
          upload_gpio_handle_close(**options)
        when "upload-time-now", "upload-time.now"
          ensure_no_extra_arguments!(words, command)
          upload_time_now(**options)
        when "upload-time-sleep-ms", "upload-time.sleep_ms", "upload-sleep-ms"
          upload_time_sleep_ms(**time_sleep_ms_command_options(words, command, options, require_budget: false))
        when "store-program", "store.program"
          SessionResult.new(results: [store_program(**store_program_command_options(words, command, options))])
        when "run"
          SessionResult.new(results: [run(**options.merge(optional_budget(words, command)))])
        when "stop"
          ensure_no_extra_arguments!(words, command)
          SessionResult.new(results: [stop])
        when "blink"
          blink(**options.merge(optional_budget(words, command)))
        when "gpio-read", "gpio.read"
          gpio_read(**gpio_read_command_options(words, command, options))
        when "gpio-write", "gpio.write"
          gpio_write(**gpio_write_command_options(words, command, options))
        when "gpio-high", "gpio.high"
          gpio_write(**gpio_level_command_options(words, command, options, value: true))
        when "gpio-low", "gpio.low"
          gpio_write(**gpio_level_command_options(words, command, options, value: false))
        when "gpio-open", "gpio.open"
          gpio_open(**gpio_open_command_options(words, command, options))
        when "gpio-handle-read", "gpio.handle-read"
          gpio_handle_read(**options.merge(optional_budget(words, command)))
        when "gpio-handle-write", "gpio.handle-write"
          gpio_handle_write(**gpio_handle_write_command_options(words, command, options))
        when "gpio-handle-close", "gpio.handle-close"
          gpio_handle_close(**options.merge(optional_budget(words, command)))
        when "time-now", "time.now", "now"
          time_now(**options.merge(optional_budget(words, command)))
        when "time-sleep-ms", "time.sleep_ms", "sleep-ms"
          time_sleep_ms(**time_sleep_ms_command_options(words, command, options))
        else
          raise UnknownSessionCommandError, "unknown Board VM session command: #{command}"
        end
      end

      private

      def dispatch(command, frame)
        response, decoded_response = connection.dispatch_protocol_frame(
          frame,
          native_session: native_session
        )
        ProtocolResult.new(
          command: command,
          frame: frame,
          response: response,
          decoded_response: decoded_response
        )
      end

      def optional_budget(words, command)
        return {} if words.empty?

        value = words.shift
        budget = begin
          Integer(value, 10)
        rescue ArgumentError
          raise ArgumentError, "#{command} budget must be an integer: #{value}"
        end
        ensure_no_extra_arguments!(words, command)
        {instruction_budget: budget}
      end

      def store_program_command_options(words, command, options)
        merged = options.dup
        merged[:program_id] = integer_argument(words.shift, "#{command} program_id") unless words.empty?
        merged[:slot] = integer_argument(words.shift, "#{command} slot") unless words.empty?
        merged[:boot_policy] = words.shift unless words.empty?
        ensure_no_extra_arguments!(words, command)
        merged
      end

      def gpio_read_command_options(words, command, options, require_budget: true)
        merged = options.dup
        merged[:pin] = integer_argument(words.shift, "#{command} pin") unless words.empty?

        unless words.empty?
          mode_or_budget = words.shift
          if integer_literal?(mode_or_budget) && require_budget
            merged[:instruction_budget] = Integer(mode_or_budget, 10)
          else
            merged[:mode] = mode_or_budget
          end
        end

        if require_budget && !words.empty?
          merged[:instruction_budget] = integer_argument(words.shift, "#{command} budget")
        end

        ensure_no_extra_arguments!(words, command)
        raise ArgumentError, "#{command} requires pin" unless merged.key?(:pin)

        merged
      end

      def time_sleep_ms_command_options(words, command, options, require_budget: true)
        merged = options.dup
        merged[:duration_ms] = integer_argument(words.shift, "#{command} duration_ms") unless words.empty?

        if require_budget && !words.empty?
          merged[:instruction_budget] = integer_argument(words.shift, "#{command} budget")
        end

        ensure_no_extra_arguments!(words, command)
        raise ArgumentError, "#{command} requires duration_ms" unless merged.key?(:duration_ms)

        merged
      end

      def gpio_read_mode(mode)
        return mode if mode.is_a?(Integer)

        text = mode.to_s.tr("-", "_")
        return Integer(text, 10) if integer_literal?(text)

        GPIO_READ_MODES.fetch(text.to_sym) do
          raise ArgumentError, "unsupported GPIO read mode: #{mode.inspect}"
        end
      end

      def gpio_mode(mode)
        return mode if mode.is_a?(Integer)

        text = mode.to_s.tr("-", "_")
        return Integer(text, 10) if integer_literal?(text)

        GPIO_MODES.fetch(text.to_sym) do
          raise ArgumentError, "unsupported GPIO mode: #{mode.inspect}"
        end
      end

      def gpio_write_command_options(words, command, options, require_budget: true)
        merged = options.dup
        merged[:pin] = integer_argument(words.shift, "#{command} pin") unless words.empty?
        merged[:value] = gpio_write_value(words.shift) unless words.empty?

        if require_budget && !words.empty?
          merged[:instruction_budget] = integer_argument(words.shift, "#{command} budget")
        end

        ensure_no_extra_arguments!(words, command)
        raise ArgumentError, "#{command} requires pin" unless merged.key?(:pin)
        raise ArgumentError, "#{command} requires value" unless merged.key?(:value)

        merged
      end

      def gpio_open_command_options(words, command, options, require_budget: true)
        merged = options.dup
        merged[:pin] = integer_argument(words.shift, "#{command} pin") unless words.empty?

        unless words.empty?
          mode_or_budget = words.shift
          if integer_literal?(mode_or_budget) && require_budget
            merged[:instruction_budget] = Integer(mode_or_budget, 10)
          else
            merged[:mode] = mode_or_budget
          end
        end

        if require_budget && !words.empty?
          merged[:instruction_budget] = integer_argument(words.shift, "#{command} budget")
        end

        ensure_no_extra_arguments!(words, command)
        raise ArgumentError, "#{command} requires pin" unless merged.key?(:pin)

        merged
      end

      def gpio_handle_write_command_options(words, command, options, require_budget: true)
        merged = options.dup
        merged[:value] = gpio_write_value(words.shift) unless words.empty?

        if require_budget && !words.empty?
          merged[:instruction_budget] = integer_argument(words.shift, "#{command} budget")
        end

        ensure_no_extra_arguments!(words, command)
        raise ArgumentError, "#{command} requires value" unless merged.key?(:value)

        merged
      end

      def gpio_level_command_options(words, command, options, value:)
        merged = options.dup
        merged[:pin] = integer_argument(words.shift, "#{command} pin") unless words.empty?

        unless words.empty?
          merged[:instruction_budget] = integer_argument(words.shift, "#{command} budget")
        end

        ensure_no_extra_arguments!(words, command)
        raise ArgumentError, "#{command} requires pin" unless merged.key?(:pin)

        merged.merge(value: value)
      end

      def gpio_write_value(value)
        return value if value == true || value == false
        return !value.zero? if value.is_a?(Integer)

        text = value.to_s.tr("-", "_")
        return !Integer(text, 10).zero? if integer_literal?(text)

        GPIO_WRITE_VALUES.fetch(text.to_sym) do
          raise ArgumentError, "unsupported GPIO write value: #{value.inspect}"
        end
      end

      def boot_policy_value(value)
        return value if value.is_a?(Integer)

        text = value.to_s.tr("-", "_")
        return Integer(text, 10) if integer_literal?(text)

        BOOT_POLICIES.fetch(text.to_sym) do
          raise ArgumentError, "unsupported boot policy: #{value.inspect}"
        end
      end

      def run_flags(flags:, reset_vm:, keep_handles:, background:)
        return flags if flags.is_a?(Integer)

        if flags
          return Array(flags).reduce(0) do |mask, flag|
            mask | RUN_FLAGS.fetch(flag.to_s.tr("-", "_").to_sym) do
              raise ArgumentError, "unsupported run flag: #{flag.inspect}"
            end
          end
        end

        value = 0
        value |= RUN_FLAG_RESET_VM_BEFORE_RUN if reset_vm
        value |= RUN_FLAG_KEEP_HANDLES_AFTER_RUN if keep_handles
        value |= RUN_FLAG_BACKGROUND_RUN if background
        value
      end

      def integer_argument(value, name)
        Integer(value, 10)
      rescue ArgumentError
        raise ArgumentError, "#{name} must be an integer: #{value}"
      end

      def integer_literal?(value)
        /\A\d+\z/.match?(value.to_s)
      end

      def ensure_no_extra_arguments!(words, command)
        return if words.empty?

        raise ArgumentError, "#{command} got unexpected argument: #{words.first}"
      end
    end
  end
end
