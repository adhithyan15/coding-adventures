# frozen_string_literal: true

module CodingAdventures
  module BoardVM
    class UnknownSessionCommandError < ArgumentError; end

    class Session
      DEFAULT_HOST_NAME = "ruby-board-vm"

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

      def upload(program_id: @program_id, module_bytes:)
        SessionResult.new(results: [
          dispatch(:program_begin, native_session.program_begin_wire(program_id, module_bytes)),
          dispatch(:program_chunk, native_session.program_chunk_wire(program_id, 0, module_bytes)),
          dispatch(:program_end, native_session.program_end_wire(program_id))
        ])
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

      def run(
        program_id: @program_id,
        budget: @instruction_budget,
        instruction_budget: nil
      )
        dispatch(
          :run,
          native_session.run_background_wire(program_id, instruction_budget || budget)
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

      def run_command(line, **options)
        words = line.to_s.split
        command = words.shift
        return SessionResult.new if command.nil?

        case command
        when "hello"
          ensure_no_extra_arguments!(words, command)
          SessionResult.new(results: [hello(**options)])
        when "caps", "capabilities"
          ensure_no_extra_arguments!(words, command)
          SessionResult.new(results: [capabilities])
        when "upload-blink"
          ensure_no_extra_arguments!(words, command)
          upload_blink(**options)
        when "run"
          SessionResult.new(results: [run(**options.merge(optional_budget(words, command)))])
        when "stop"
          ensure_no_extra_arguments!(words, command)
          SessionResult.new(results: [stop])
        when "blink"
          blink(**options.merge(optional_budget(words, command)))
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

      def ensure_no_extra_arguments!(words, command)
        return if words.empty?

        raise ArgumentError, "#{command} got unexpected argument: #{words.first}"
      end
    end
  end
end
