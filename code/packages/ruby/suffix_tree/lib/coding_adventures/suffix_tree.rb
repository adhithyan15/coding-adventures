# frozen_string_literal: true

module CodingAdventures
  module SuffixTree
    SuffixTreeNode = Struct.new(:suffix_index, :children, keyword_init: true)

    class SuffixTree
      attr_reader :text, :root

      def self.build(text)
        new(text)
      end

      def self.build_ukkonen(text)
        build(text)
      end

      def self.longest_common_substring(left, right)
        a = left.to_s.chars
        b = right.to_s.chars
        return "" if a.empty? || b.empty?

        dp = Array.new(a.length + 1) { Array.new(b.length + 1, 0) }
        best_len = 0
        best_end = 0

        (1..a.length).each do |i|
          (1..b.length).each do |j|
            next unless a[i - 1] == b[j - 1]

            dp[i][j] = dp[i - 1][j - 1] + 1
            if dp[i][j] > best_len
              best_len = dp[i][j]
              best_end = i
            end
          end
        end

        a[(best_end - best_len)...best_end].join
      end

      def initialize(text)
        @text = text.to_s
        @root = SuffixTreeNode.new(suffix_index: nil, children: suffix_indices)
      end

      def search(pattern)
        search_positions(@text, pattern)
      end

      def count_occurrences(pattern)
        search(pattern).length
      end

      def longest_repeated_substring
        longest_repeated_substring_in(@text)
      end

      def all_suffixes
        all_suffixes_in(@text)
      end

      def node_count
        1 + @root.children.length
      end

      private

      def suffix_indices
        (0...@text.length).map { |index| SuffixTreeNode.new(suffix_index: index, children: []) }
      end

      def search_positions(text, pattern)
        return (0..text.length).to_a if pattern.to_s.empty?

        positions = []
        text.length.times do |start|
          positions << start if text[start, pattern.length] == pattern
        end
        positions
      end

      def longest_repeated_substring_in(text)
        suffixes = all_suffixes_in(text)
        best = ""
        suffixes.each_with_index do |left, index|
          suffixes[(index + 1)..].to_a.each do |right|
            prefix = common_prefix(left, right)
            best = prefix if prefix.length > best.length
          end
        end
        best
      end

      def common_prefix(left, right)
        out = +""
        left.chars.zip(right.chars).each do |a, b|
          break if a != b

          out << a
        end
        out
      end

      def all_suffixes_in(text)
        (0...text.length).map { |index| text[index..] }
      end
    end

    def self.build(text)
      SuffixTree.build(text)
    end

    def self.build_ukkonen(text)
      SuffixTree.build_ukkonen(text)
    end
  end
end
