# frozen_string_literal: true

require "digest"

module CodingAdventures
  module RespProtocol
    SimpleString = Struct.new(:value, keyword_init: true)
    Error = Struct.new(:value, keyword_init: true)
    RespInteger = Struct.new(:value, keyword_init: true)
    BulkString = Struct.new(:value, keyword_init: true)
    RespArray = Struct.new(:values, keyword_init: true)

    class ParseError < StandardError; end

    class Encoder
      def encode(frame)
        case frame
        when SimpleString
          "+#{frame.value}\r\n"
        when Error
          "-#{frame.value}\r\n"
        when RespInteger
          ":#{frame.value}\r\n"
        when BulkString
          return "$-1\r\n" if frame.value.nil?

          value = frame.value.to_s.dup.force_encoding(Encoding::BINARY)
          "$#{value.bytesize}\r\n#{value}\r\n"
        when RespArray
          return "*-1\r\n" if frame.values.nil?

          values = frame.values.map { |value| encode(value) }.join
          "*#{frame.values.length}\r\n#{values}"
        when ::String
          encode(BulkString.new(value: frame))
        when ::Integer
          encode(RespInteger.new(value: frame))
        when nil
          "$-1\r\n"
        else
          raise TypeError, "unsupported RESP frame: #{frame.class}"
        end
      end
    end

    class Decoder
      DecodeResult = Struct.new(:frame, :rest, keyword_init: true)

      def decode(buffer)
        buffer = buffer.to_s.dup.force_encoding(Encoding::BINARY)
        frame, next_index = parse_frame(buffer, 0)
        DecodeResult.new(frame: frame, rest: buffer.byteslice(next_index, buffer.bytesize - next_index).to_s)
      end

      def decode_all(buffer)
        frames = []
        remaining = buffer.to_s.dup.force_encoding(Encoding::BINARY)

        until remaining.empty?
          result = decode(remaining)
          frames << result.frame
          remaining = result.rest
        end

        frames
      end

      private

      def parse_frame(buffer, index)
        raise ParseError, "unexpected end of RESP buffer" if index >= buffer.bytesize

        case buffer.getbyte(index)
        when "+".ord
          parse_simple(buffer, index + 1, SimpleString)
        when "-".ord
          parse_simple(buffer, index + 1, Error)
        when ":".ord
          parse_simple(buffer, index + 1, RespInteger) { |value| value.to_i }
        when "$".ord
          parse_bulk_string(buffer, index + 1)
        when "*".ord
          parse_array(buffer, index + 1)
        else
          raise ParseError, "unknown RESP type byte: #{buffer.getbyte(index).inspect}"
        end
      end

      def parse_simple(buffer, index, frame_class)
        line, next_index = read_line(buffer, index)
        value = block_given? ? yield(line) : line
        [frame_class.new(value: value), next_index]
      rescue ArgumentError => e
        raise ParseError, e.message
      end

      def parse_bulk_string(buffer, index)
        line, next_index = read_line(buffer, index)
        length = line.to_i
        return [BulkString.new(value: nil), next_index] if length == -1

        payload = buffer.byteslice(next_index, length)
        raise ParseError, "bulk string length out of bounds" if payload.nil? || payload.bytesize != length

        terminator = buffer.byteslice(next_index + length, 2)
        raise ParseError, "bulk string missing CRLF terminator" unless terminator == "\r\n"

        [BulkString.new(value: payload), next_index + length + 2]
      rescue ArgumentError => e
        raise ParseError, e.message
      end

      def parse_array(buffer, index)
        line, next_index = read_line(buffer, index)
        length = line.to_i
        return [RespArray.new(values: nil), next_index] if length == -1

        values = []
        current = next_index

        length.times do
          frame, current = parse_frame(buffer, current)
          values << frame
        end

        [RespArray.new(values: values), current]
      rescue ArgumentError => e
        raise ParseError, e.message
      end

      def read_line(buffer, index)
        terminator = buffer.index("\r\n", index)
        raise ParseError, "unterminated RESP line" if terminator.nil?

        [buffer.byteslice(index, terminator - index), terminator + 2]
      end
    end
  end
end
