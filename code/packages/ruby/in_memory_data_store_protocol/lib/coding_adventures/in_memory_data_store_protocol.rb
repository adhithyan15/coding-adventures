# frozen_string_literal: true

require "coding_adventures_resp_protocol"

module CodingAdventures
  module InMemoryDataStoreProtocol
    Command = Struct.new(:name, :argv, keyword_init: true)

    class ProtocolError < StandardError; end

    class Translator
      def decode(frame)
        raise ProtocolError, "expected RESP array command" unless frame.is_a?(RespProtocol::RespArray)
        raise ProtocolError, "null RESP arrays are not valid commands" if frame.values.nil? || frame.values.empty?

        name = frame_to_string(frame.values.first).upcase
        argv = frame.values.drop(1).map { |value| frame_to_string(value) }
        Command.new(name: name, argv: argv)
      end

      def encode(value)
        case value
        when RespProtocol::SimpleString, RespProtocol::Error, RespProtocol::RespInteger, RespProtocol::BulkString, RespProtocol::RespArray
          value
        when ::String
          RespProtocol::BulkString.new(value: value)
        when ::Integer
          RespProtocol::RespInteger.new(value: value)
        when nil
          RespProtocol::BulkString.new(value: nil)
        when ::Array
          RespProtocol::RespArray.new(values: value.map { |element| encode(element) })
        when true
          RespProtocol::RespInteger.new(value: 1)
        when false
          RespProtocol::RespInteger.new(value: 0)
        else
          if value.respond_to?(:to_a)
            encode(value.to_a)
          else
            RespProtocol::BulkString.new(value: value.to_s)
          end
        end
      end

      private

      def frame_to_string(frame)
        case frame
        when RespProtocol::BulkString, RespProtocol::SimpleString
          raise ProtocolError, "null bulk strings are not valid command arguments" if frame.value.nil?
          frame.value.to_s
        when RespProtocol::RespInteger
          frame.value.to_s
        else
          raise ProtocolError, "unsupported RESP frame in command: #{frame.class}"
        end
      end
    end
  end
end
