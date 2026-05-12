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
      LED_MATRIX_ROWS = 8
      LED_MATRIX_COLUMNS = 12
      LED_MATRIX_PRESETS = {
        clear: [0x0000_0000, 0x0000_0000, 0x0000_0000],
        heart: [0x3184_A444, 0x4404_2081, 0x100A_0040],
        happy: [0x0198_0019, 0x8000_0001, 0x081F_8000],
        danger: [0x0400_A015, 0x1502_0820, 0x4840_47FC]
      }.freeze

      def self.led_matrix_words(words: nil, pattern: nil, preset: nil)
        return normalize_led_matrix_words(words) unless words.nil?
        return led_matrix_preset_words(preset) unless preset.nil?
        return pack_led_matrix_pattern(pattern) unless pattern.nil?

        raise ArgumentError, "led_matrix_frame requires words:, pattern:, or preset:"
      end

      def self.normalize_led_matrix_words(words)
        values = Array(words)
        raise ArgumentError, "led_matrix words must contain exactly 3 integers" unless values.length == 3

        values.map.with_index do |word, index|
          value = word.is_a?(Integer) ? word : Integer(word, 0)
          if value.negative? || value > 0xFFFF_FFFF
            raise ArgumentError, "led_matrix word#{index} must fit in u32"
          end
          value
        end
      end
      private_class_method :normalize_led_matrix_words

      def self.led_matrix_preset_words(preset)
        LED_MATRIX_PRESETS.fetch(preset.to_s.tr("-", "_").to_sym) do
          raise ArgumentError, "unknown LED matrix preset: #{preset.inspect}"
        end
      end
      private_class_method :led_matrix_preset_words

      def self.pack_led_matrix_pattern(pattern)
        rows = if pattern.is_a?(String)
          pattern.lines.map(&:chomp).reject { |row| row.strip.empty? }
        else
          Array(pattern)
        end
        unless rows.length == LED_MATRIX_ROWS
          raise ArgumentError, "led_matrix pattern must have #{LED_MATRIX_ROWS} rows"
        end

        words = [0, 0, 0]
        rows.each_with_index do |row, y|
          pixels = row.is_a?(String) ? row.gsub(/\s+/, "").chars : Array(row)
          unless pixels.length == LED_MATRIX_COLUMNS
            raise ArgumentError, "led_matrix pattern row #{y} must have #{LED_MATRIX_COLUMNS} columns"
          end

          pixels.each_with_index do |pixel, x|
            next unless led_matrix_pixel_on?(pixel)

            index = y * LED_MATRIX_COLUMNS + x
            words[index / 32] |= 1 << (31 - (index % 32))
          end
        end
        words
      end
      private_class_method :pack_led_matrix_pattern

      def self.led_matrix_pixel_on?(pixel)
        return pixel if pixel == true || pixel == false
        return !pixel.zero? if pixel.is_a?(Integer)

        case pixel.to_s
        when "", ".", "0", "_", "-"
          false
        else
          true
        end
      end
      private_class_method :led_matrix_pixel_on?

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

      def led_matrix_frame_module(words: nil, pattern: nil, preset: nil, max_stack: 3)
        frame_words = self.class.led_matrix_words(words: words, pattern: pattern, preset: preset)
        native_session.led_matrix_frame_module(frame_words[0], frame_words[1], frame_words[2], max_stack)
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

      def upload_led_matrix_frame(
        program_id: @program_id,
        words: nil,
        pattern: nil,
        preset: nil,
        max_stack: 3
      )
        upload(
          program_id: program_id,
          module_bytes: led_matrix_frame_module(
            words: words,
            pattern: pattern,
            preset: preset,
            max_stack: max_stack
          )
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
        time_budget_ms: 0,
        timeout_ms: nil,
        allow_timeout: false
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
          ),
          timeout_ms: timeout_ms,
          allow_timeout: allow_timeout
        )
      end

      def stop(timeout_ms: nil, allow_timeout: false)
        dispatch(:stop, native_session.stop_wire, timeout_ms: timeout_ms, allow_timeout: allow_timeout)
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
        host_nonce: @host_nonce,
        run_response_timeout_ms: nil,
        allow_run_timeout: true
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
          instruction_budget: instruction_budget || budget,
          timeout_ms: run_response_timeout_ms || blink_run_response_timeout_ms(high_ms, low_ms),
          allow_timeout: allow_run_timeout
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

      def led_matrix_frame(
        program_id: @program_id,
        budget: @instruction_budget,
        instruction_budget: nil,
        words: nil,
        pattern: nil,
        preset: nil,
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
          upload_led_matrix_frame(
            program_id: program_id,
            words: words,
            pattern: pattern,
            preset: preset,
            max_stack: max_stack
          ).results
        )
        results << run(
          program_id: program_id,
          instruction_budget: instruction_budget || budget,
          background: false
        )
        SessionResult.new(results: results)
      end
      alias matrix_frame led_matrix_frame

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
        when "upload-led-matrix-frame", "upload-led-matrix.frame", "upload-matrix-frame"
          upload_led_matrix_frame(**led_matrix_command_options(words, command, options, require_budget: false))
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
        when "led-matrix-frame", "led-matrix.frame", "matrix-frame"
          led_matrix_frame(**led_matrix_command_options(words, command, options))
        else
          raise UnknownSessionCommandError, "unknown Board VM session command: #{command}"
        end
      end

      private

      def dispatch(command, frame, timeout_ms: nil, allow_timeout: false)
        response, decoded_response = connection.dispatch_protocol_frame(
          frame,
          native_session: native_session,
          timeout_ms: timeout_ms,
          allow_timeout: allow_timeout,
          expected_request_id: current_request_id,
          expected_response_kind: expected_response_kind(command)
        )
        ProtocolResult.new(
          command: command,
          frame: frame,
          response: response,
          decoded_response: decoded_response
        )
      end

      def blink_run_response_timeout_ms(high_ms, low_ms)
        [Integer(high_ms) + Integer(low_ms) + 500, 250].max
      end

      def current_request_id
        native_session.next_request_id - 1
      end

      def expected_response_kind(command)
        case command
        when :hello
          "hello_ack"
        when :capabilities
          "caps_report"
        when :run, :stop
          "run_report"
        else
          command.to_s
        end
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

      def led_matrix_command_options(words, command, options, require_budget: true)
        merged = options.dup
        if words.length >= 3 && words.first(3).all? { |word| integer_literal?(word) || hex_literal?(word) }
          merged[:words] = [
            numeric_argument(words.shift, "#{command} word0"),
            numeric_argument(words.shift, "#{command} word1"),
            numeric_argument(words.shift, "#{command} word2")
          ]
        elsif !words.empty?
          merged[:preset] = words.shift
        else
          merged[:preset] ||= :heart
        end

        if require_budget && !words.empty?
          merged[:instruction_budget] = integer_argument(words.shift, "#{command} budget")
        end

        ensure_no_extra_arguments!(words, command)
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

      def numeric_argument(value, name)
        Integer(value, 0)
      rescue ArgumentError
        raise ArgumentError, "#{name} must be an integer: #{value}"
      end

      def integer_literal?(value)
        /\A\d+\z/.match?(value.to_s)
      end

      def hex_literal?(value)
        /\A0x[[:xdigit:]]+\z/i.match?(value.to_s)
      end

      def ensure_no_extra_arguments!(words, command)
        return if words.empty?

        raise ArgumentError, "#{command} got unexpected argument: #{words.first}"
      end
    end
  end
end
