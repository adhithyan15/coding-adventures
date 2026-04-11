# frozen_string_literal: true

module CodingAdventures
  module LZ77
    # Token is a single LZ77 token: (offset, length, next_char).
    #
    # Represents one unit of the compressed stream.
    #
    # - +offset+:    Distance back the match starts (1..window_size), or 0.
    # - +length+:    Number of bytes the match covers (0 = no match).
    # - +next_char+: Literal byte immediately after the match (0..255).
    Token = Struct.new(:offset, :length, :next_char) do
      # Two tokens are equal if all three fields match.
      def ==(other)
        other.is_a?(Token) &&
          offset == other.offset &&
          length == other.length &&
          next_char == other.next_char
      end

      def to_s
        "(#{offset}, #{length}, #{next_char})"
      end
    end
  end
end
