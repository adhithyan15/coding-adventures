# frozen_string_literal: true

require "io/wait"
require "rbconfig"

module CodingAdventures
  module BoardVM
    SessionResult = Struct.new(:frames, :responses, keyword_init: true)

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
