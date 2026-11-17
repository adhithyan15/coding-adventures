# frozen_string_literal: true

require_relative "constants"
require_relative "messages"

module CodingAdventures
  module CanonicalCbor
    def self.normalize_bytes(data, error_id)
      if data.is_a?(String)
        data.b
      elsif data.is_a?(Array) && data.all? { |b| b.is_a?(Integer) && b >= 0 && b <= Constants::BYTE_MASK }
        data.pack("C*").b
      else
        raise CborError.new(error_id)
      end
    end

    class CborError < StandardError
      attr_reader :id

      def initialize(id)
        @id = id
        super(Messages.message_for(id))
      end

      def to_s
        Messages.message_for(@id)
      end

      def inspect
        "#<#{self.class.name}: #{message}>"
      end

      def ==(other)
        other.is_a?(CborError) && other.id == @id
      end
      alias_method :eql?, :==

      def hash
        [self.class, @id].hash
      end
    end

    class Span
      attr_reader :start, :end

      def initialize(start_pos, end_pos)
        @start = start_pos
        @end = end_pos
        freeze
      end

      def length
        @end - @start
      end

      def ==(other)
        other.is_a?(Span) && other.start == @start && other.end == @end
      end
      alias_method :eql?, :==

      def hash
        [self.class, @start, @end].hash
      end

      def to_s
        "[#{@start}, #{@end})"
      end
      alias_method :inspect, :to_s
    end

    class SpannedMapEntry
      attr_reader :key, :value, :value_span

      def initialize(key:, value:, value_span:)
        @key = key
        @value = value
        @value_span = value_span
        freeze
      end

      def ==(other)
        other.is_a?(SpannedMapEntry) &&
          other.key == @key &&
          other.value == @value &&
          other.value_span == @value_span
      end
      alias_method :eql?, :==

      def hash
        [self.class, @key, @value, @value_span].hash
      end

      def to_s
        "SpannedMapEntry(key: #{Messages::REDACTED_PLACEHOLDER}, value: #{Messages::REDACTED_PLACEHOLDER}, value_span: #{@value_span})"
      end
      alias_method :inspect, :to_s
    end

    class DecodeMapSpansResult
      attr_reader :entries

      private_class_method :new

      def self.map(entries)
        unless entries.is_a?(Array) && entries.all? { |e| e.is_a?(SpannedMapEntry) }
          raise CborError.new(:unsupported_simple)
        end
        new(entries.dup.freeze)
      end

      def self.non_map
        new(nil)
      end

      def initialize(entries)
        @entries = entries
        freeze
      end

      def map?
        !@entries.nil?
      end

      def ==(other)
        other.is_a?(DecodeMapSpansResult) &&
          other.entries == @entries
      end
      alias_method :eql?, :==

      def hash
        [self.class, @entries].hash
      end
    end

    class CborValue
      def self.validate_u64!(val)
        unless val.is_a?(Integer) && val >= 0 && val <= Constants::MAX_UINT64
          raise CborError.new(:length_too_large)
        end
        val
      end
    end

    class CborUnsigned < CborValue
      attr_reader :value

      def initialize(value)
        CborValue.validate_u64!(value)
        @value = value
        freeze
      end

      def kind
        :unsigned
      end

      def ==(other)
        other.is_a?(CborUnsigned) && other.value == @value
      end
      alias_method :eql?, :==

      def hash
        [self.class, @value].hash
      end

      def to_s
        "CborUnsigned(#{@value})"
      end
      alias_method :inspect, :to_s
    end

    class CborNegative < CborValue
      attr_reader :value

      def initialize(value)
        CborValue.validate_u64!(value)
        @value = value
        freeze
      end

      def to_i
        -1 - @value
      end

      def kind
        :negative
      end

      def ==(other)
        other.is_a?(CborNegative) && other.value == @value
      end
      alias_method :eql?, :==

      def hash
        [self.class, @value].hash
      end

      def to_s
        "CborNegative(#{@value})"
      end
      alias_method :inspect, :to_s
    end

    class CborBytes < CborValue
      attr_reader :bytes

      def initialize(bytes)
        @bytes = CanonicalCbor.normalize_bytes(bytes, :length_too_large).freeze
        freeze
      end

      def bytesize
        @bytes.bytesize
      end
      alias_method :length, :bytesize

      def kind
        :bytes
      end

      def ==(other)
        other.is_a?(CborBytes) && other.bytes == @bytes
      end
      alias_method :eql?, :==

      def hash
        [self.class, @bytes].hash
      end

      def to_s
        "CborBytes(#{@bytes.inspect})"
      end
      alias_method :inspect, :to_s
    end

    class CborText < CborValue
      attr_reader :text

      def initialize(text)
        unless text.is_a?(String) && text.encoding == Encoding::UTF_8 && text.valid_encoding?
          raise CborError.new(:invalid_utf8)
        end
        @text = text.dup.freeze
        freeze
      end

      def bytesize
        @text.bytesize
      end

      def length
        @text.length
      end

      def kind
        :text
      end

      def ==(other)
        other.is_a?(CborText) && other.text == @text
      end
      alias_method :eql?, :==

      def hash
        [self.class, @text].hash
      end

      def to_s
        "CborText(#{@text.inspect})"
      end
      alias_method :inspect, :to_s
    end

    class CborArray < CborValue
      attr_reader :items

      def initialize(items)
        unless items.is_a?(Array)
          raise CborError.new(:length_too_large)
        end
        unless items.all? { |elem| elem.is_a?(CborValue) }
          raise CborError.new(:unsupported_simple)
        end
        @items = items.dup.freeze
        freeze
      end

      def length
        @items.length
      end

      def kind
        :array
      end

      def ==(other)
        other.is_a?(CborArray) && other.items == @items
      end
      alias_method :eql?, :==

      def hash
        [self.class, @items].hash
      end

      def to_s
        "CborArray(#{@items.inspect})"
      end
      alias_method :inspect, :to_s
    end

    class CborMapEntry
      attr_reader :key, :value

      def initialize(key, value)
        unless key.is_a?(CborValue) && value.is_a?(CborValue)
          raise CborError.new(:unsupported_simple)
        end
        @key = key
        @value = value
        freeze
      end

      def ==(other)
        other.is_a?(CborMapEntry) && other.key == @key && other.value == @value
      end
      alias_method :eql?, :==

      def hash
        [self.class, @key, @value].hash
      end

      def to_s
        "CborMapEntry(#{@key.inspect}, #{@value.inspect})"
      end
      alias_method :inspect, :to_s
    end

    class CborMap < CborValue
      attr_reader :entries

      def initialize(entries)
        unless entries.is_a?(Array)
          raise CborError.new(:length_too_large)
        end
        unless entries.all? { |e| e.is_a?(CborMapEntry) }
          raise CborError.new(:unsupported_simple)
        end
        @entries = entries.dup.freeze
        freeze
      end

      def length
        @entries.length
      end

      def kind
        :map
      end

      def ==(other)
        other.is_a?(CborMap) && other.entries == @entries
      end
      alias_method :eql?, :==

      def hash
        [self.class, @entries].hash
      end

      def to_s
        "CborMap(#{@entries.inspect})"
      end
      alias_method :inspect, :to_s
    end

    class CborTag < CborValue
      attr_reader :tag, :value

      def initialize(tag, value)
        CborValue.validate_u64!(tag)
        unless value.is_a?(CborValue)
          raise CborError.new(:unsupported_simple)
        end
        @tag = tag
        @value = value
        freeze
      end

      def kind
        :tag
      end

      def ==(other)
        other.is_a?(CborTag) && other.tag == @tag && other.value == @value
      end
      alias_method :eql?, :==

      def hash
        [self.class, @tag, @value].hash
      end

      def to_s
        "CborTag(#{@tag}, #{@value.inspect})"
      end
      alias_method :inspect, :to_s
    end

    class CborBool < CborValue
      attr_reader :value

      def initialize(value)
        unless value == true || value == false
          raise CborError.new(:unsupported_simple)
        end
        @value = value
        freeze
      end

      def kind
        :boolean
      end

      def ==(other)
        other.is_a?(CborBool) && other.value == @value
      end
      alias_method :eql?, :==

      def hash
        [self.class, @value].hash
      end

      def to_s
        "CborBool(#{@value})"
      end
      alias_method :inspect, :to_s
    end

    class CborNull < CborValue
      def initialize
        freeze
      end

      def kind
        :null
      end

      def ==(other)
        other.is_a?(CborNull)
      end
      alias_method :eql?, :==

      def hash
        self.class.hash
      end

      def to_s
        "CborNull()"
      end
      alias_method :inspect, :to_s
    end
  end
end
