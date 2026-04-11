# frozen_string_literal: true

module CodingAdventures
  module Rope
    class LeafNode
      attr_accessor :chunk

      def initialize(chunk)
        @chunk = chunk.to_s
      end
    end

    class InternalNode
      attr_accessor :weight, :left, :right

      def initialize(weight, left, right)
        @weight = weight
        @left = left
        @right = right
      end
    end

    class RopeNode
      attr_accessor :value

      def initialize(value)
        @value = value
      end

      def leaf?
        value.is_a?(LeafNode)
      end

      def internal?
        value.is_a?(InternalNode)
      end

      def depth
        case value
        when LeafNode
          0
        when InternalNode
          1 + [RopeNode.new(value.left).depth, RopeNode.new(value.right).depth].max
        end
      end

      def balanced?
        case value
        when LeafNode
          true
        when InternalNode
          left_depth = RopeNode.new(value.left).depth
          right_depth = RopeNode.new(value.right).depth
          (left_depth - right_depth).abs <= 1 &&
            RopeNode.new(value.left).balanced? &&
            RopeNode.new(value.right).balanced?
        end
      end

      def to_s(out = +"")
        case value
        when LeafNode
          out << value.chunk
        when InternalNode
          RopeNode.new(value.left).to_s(out)
          RopeNode.new(value.right).to_s(out)
        end
        out
      end
    end

    class Rope
      attr_reader :root, :len

      def self.empty
        new
      end

      def self.from_string(string)
        new(string)
      end

      def initialize(string = nil)
        if string.nil? || string.to_s.empty?
          @root = nil
          @len = 0
        else
          string = string.to_s
          @root = RopeNode.new(LeafNode.new(string))
          @len = string.length
        end
      end

      def empty?
        @len.zero?
      end

      def to_string
        return "" if @root.nil?

        @root.to_s
      end

      def index(i)
        to_string[i]
      end

      def substring(start, finish)
        text = to_string
        start = [start, text.length].min
        finish = [finish, text.length].min
        return "" if start >= finish

        text[start...finish]
      end

      def depth
        @root&.depth || 0
      end

      def balanced?
        @root&.balanced? || true
      end

      def to_s
        to_string
      end
    end

    def self.rope_from_string(string)
      Rope.from_string(string)
    end

    def self.rope_empty
      Rope.empty
    end

    def self.length(rope)
      rope.len
    end

    def self.index(rope, i)
      rope.index(i)
    end

    def self.rope_index(rope, i)
      rope.index(i)
    end

    def self.to_string(rope)
      rope.to_string
    end

    def self.concat(left, right)
      return Rope.new(right.to_string) if left.empty?
      return Rope.new(left.to_string) if right.empty?

      Rope.new(left.to_string + right.to_string)
    end

    def self.split(rope, index)
      text = rope.to_string
      index = [[index, 0].max, text.length].min
      [Rope.new(text[0...index]), Rope.new(text[index..] || "")]
    end

    def self.insert(rope, index, string)
      left, right = split(rope, index)
      concat(concat(left, Rope.new(string)), right)
    end

    def self.delete(rope, start, length)
      text = rope.to_string
      start = [[start, 0].max, text.length].min
      finish = [[start + length, 0].max, text.length].min
      Rope.new(text[0...start].to_s + text[finish..].to_s)
    end

    def self.substring(rope, start, finish)
      rope.substring(start, finish)
    end

    def self.depth(rope)
      rope.depth
    end

    def self.is_balanced(rope)
      rope.balanced?
    end

    def self.rebalance(rope)
      Rope.new(rope.to_string)
    end
  end
end
