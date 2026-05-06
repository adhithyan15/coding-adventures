# frozen_string_literal: true

require "io/wait"
require "rbconfig"

module CodingAdventures
  module BoardVM
    ProtocolResult = Struct.new(
      :command,
      :frame,
      :response,
      :decoded_response,
      keyword_init: true
    ) do
      def kind
        decoded_response && decoded_response["kind"]
      end

      def payload
        decoded_response && decoded_response["payload"]
      end

      def board_descriptor
        BoardDescriptor.from_decoded_response(decoded_response)
      end

      def error?
        !!(decoded_response && decoded_response["error"])
      end
    end

    SessionResult = Struct.new(
      :results,
      :frames,
      :responses,
      :decoded_responses,
      keyword_init: true
    ) do
      def initialize(results: nil, frames: nil, responses: nil, decoded_responses: nil)
        results ||= []
        frames ||= results.map(&:frame)
        responses ||= results.map(&:response)
        decoded_responses ||= results.map(&:decoded_response)
        super(
          results: results,
          frames: frames,
          responses: responses,
          decoded_responses: decoded_responses
        )
      end

      def error?
        results.any?(&:error?)
      end

      def board_descriptor
        results.each do |result|
          descriptor = result.board_descriptor
          return descriptor if descriptor
        end
        nil
      end
    end

    class TransportError < StandardError; end

    class SerialTransport
      FRAME_DELIMITER = "\x00".b

      def initialize(port:, baud_rate:, timeout_ms:)
        @port = port
        @baud_rate = baud_rate
        @timeout_ms = timeout_ms
        @io = nil
      end

      def transact(frame, timeout_ms: @timeout_ms)
        write(frame)
        read_frame(timeout_ms: timeout_ms)
      end

      def write(frame)
        io.write(frame.b)
        io.flush
      rescue SystemCallError, IOError => e
        raise TransportError, "failed to write Board VM frame to #{@port}: #{e.message}"
      end

      def close
        @io&.close
      ensure
        @io = nil
      end

      private

      def io
        @io ||= begin
          configure_port
          File.open(@port, "r+b")
        rescue SystemCallError => e
          raise TransportError, "failed to open Board VM serial port #{@port}: #{e.message}"
        end
      end

      def configure_port
        flag = RbConfig::CONFIG.fetch("host_os").match?(/darwin|bsd/) ? "-f" : "-F"
        return if system("stty", flag, @port, @baud_rate.to_s, "raw", "-echo")

        raise TransportError, "failed to configure Board VM serial port #{@port}"
      end

      def read_frame(timeout_ms:)
        deadline = monotonic_now + (timeout_ms.to_f / 1000.0)
        response = +"".b

        loop do
          remaining = deadline - monotonic_now
          raise TransportError, "timed out waiting for Board VM response on #{@port}" if remaining <= 0

          readable = IO.select([io], nil, nil, remaining)
          next if readable.nil?

          byte = io.read_nonblock(1, exception: false)
          next if byte == :wait_readable
          raise TransportError, "Board VM serial port #{@port} closed" if byte.nil? || byte.empty?

          response << byte
          return response if byte == FRAME_DELIMITER
        end
      rescue SystemCallError, IOError => e
        raise TransportError, "failed to read Board VM response from #{@port}: #{e.message}"
      end

      def monotonic_now
        Process.clock_gettime(Process::CLOCK_MONOTONIC)
      end
    end
  end
end
