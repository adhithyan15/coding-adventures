# frozen_string_literal: true

module CodingAdventures
  module HistoricalWeb
    # A request address owns an immutable snapshot of printable ASCII octets.
    class Http09Address
      attr_reader :value

      def self.create(value)
        case value
        when String
          # A base String copy below removes subclass method overrides.
        else
          return Result.failure(:invalid_address)
        end

        snapshot = String.new(value, encoding: Encoding::BINARY)
        return Result.failure(:invalid_address) unless printable_ascii?(snapshot)

        Result.success(new(snapshot.freeze))
      rescue ArgumentError, EncodingError, TypeError
        Result.failure(:invalid_address)
      end

      def self.safe_snapshot(address)
        case address
        when Http09Address
          # Continue only for this address type.
        else
          return nil
        end
        return nil unless Object.instance_method(:frozen?).bind_call(address)

        stored = Object.instance_method(:instance_variable_get).bind_call(address, :@value)
        case stored
        when String
          # The copied base String below removes subclass method overrides.
        else
          return nil
        end
        return nil unless Object.instance_method(:frozen?).bind_call(stored)

        snapshot = String.new(stored, encoding: Encoding::BINARY)
        printable_ascii?(snapshot) ? snapshot.freeze : nil
      rescue ArgumentError, EncodingError, TypeError
        nil
      end

      def initialize(value)
        @value = String.new(value, encoding: Encoding::BINARY).freeze
        freeze
      end
      private_class_method :new

      def self.printable_ascii?(value)
        return false if value.empty?

        index = 0
        while index < value.bytesize
          byte = value.getbyte(index)
          return false unless byte&.between?(0x21, 0x7e)

          index += 1
        end
        true
      end
      private_class_method :printable_ascii?
    end

    # Encode a single `GET address\r\n` line. The input address's original
    # String is never consulted; only a fresh checked snapshot can reach the wire.
    def self.encode_http09_request(address)
      snapshot = Http09Address.safe_snapshot(address)
      return Result.failure(:invalid_address) unless snapshot

      wire = String.new("GET ", encoding: Encoding::BINARY)
      wire << snapshot << "\r\n".b
      Result.success(wire.freeze)
    end

    # Parse exactly one GET line, allowing the profile's CRLF and LF endings.
    # A Ruby String is treated as octets, regardless of its encoding tag.
    def self.parse_http09_request(wire)
      case wire
      when String
        # Work from a concrete binary copy below.
      else
        return Result.failure(:invalid_request)
      end

      bytes = String.new(wire, encoding: Encoding::BINARY)
      return Result.failure(:invalid_request) if bytes.bytesize < 6
      return Result.failure(:invalid_request) unless bytes.byteslice(0, 4) == "GET ".b
      return Result.failure(:invalid_request) unless bytes.getbyte(bytes.bytesize - 1) == 0x0a

      finish = bytes.bytesize - 1
      finish -= 1 if finish.positive? && bytes.getbyte(finish - 1) == 0x0d
      return Result.failure(:invalid_request) if finish <= 4

      address = bytes.byteslice(4, finish - 4)
      result = Http09Address.create(address)
      result.ok? ? result : Result.failure(:invalid_request)
    rescue ArgumentError, EncodingError, TypeError
      Result.failure(:invalid_request)
    end

    # Decode the complete response octet stream as ASCII text. The returned
    # String is fresh and US-ASCII encoded; input encoding labels do not alter
    # the meaning of the HTTP bytes.
    def self.decode_http09_response(body)
      case body
      when String
        # Work from a concrete binary copy below.
      else
        return Result.failure(:non_ascii)
      end

      bytes = String.new(body, encoding: Encoding::BINARY)
      index = 0
      while index < bytes.bytesize
        byte = bytes.getbyte(index)
        return Result.failure(:non_ascii) unless byte && byte <= 0x7f

        index += 1
      end
      Result.success(String.new(bytes, encoding: Encoding::US_ASCII))
    rescue ArgumentError, EncodingError, TypeError
      Result.failure(:non_ascii)
    end
  end
end
