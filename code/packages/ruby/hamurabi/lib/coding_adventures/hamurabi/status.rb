# frozen_string_literal: true

module CodingAdventures
  module Hamurabi
    # Rating classification awarded to ruler upon completing a 10-year term.
    module Rating
      # Average starvation <= 3% AND acres per person >= 10.
      CHARLEMAGNE = "Charlemagne"
      # Average starvation <= 10% AND acres per person >= 9.
      ACRE = "Acre"
      # Average starvation <= 33% AND acres per person >= 7.
      NERO = "Nero"
      # Average starvation > 33% OR acres per person < 7.
      DISASTER = "Disaster"
    end

    # Simulation lifecycle status.
    class GameStatus
      attr_reader :tag, :rating, :starved_this_turn, :percent

      def initialize(tag, rating: nil, starved_this_turn: nil, percent: nil)
        @tag = tag.to_sym
        @rating = rating
        @starved_this_turn = starved_this_turn
        @percent = percent
        freeze
      end

      def playing?
        @tag == :playing
      end

      def won?
        @tag == :won
      end

      def impeached?
        @tag == :impeached
      end

      def extinct?
        @tag == :extinct
      end

      def to_s
        case @tag
        when :playing then "Playing"
        when :won then "Won(#{@rating})"
        when :impeached then "Impeached(starved=#{@starved_this_turn}, percent=#{@percent})"
        when :extinct then "GameOverExtinction"
        end
      end

      def inspect
        "#<CodingAdventures::Hamurabi::GameStatus #{to_s}>"
      end

      def ==(other)
        if other.is_a?(String)
          to_s == other
        elsif other.is_a?(Symbol)
          @tag == other
        elsif other.is_a?(GameStatus)
          @tag == other.tag && @rating == other.rating &&
            @starved_this_turn == other.starved_this_turn && @percent == other.percent
        else
          false
        end
      end

      def self.parse(val)
        return val if val.is_a?(GameStatus)
        str = val.to_s
        if str == "Playing"
          PLAYING
        elsif str == "GameOverExtinction"
          EXTINCT
        elsif str.start_with?("Won(") && str.end_with?(")")
          rating = str[4..-2]
          new(:won, rating: rating)
        elsif str.start_with?("Impeached(") && str.end_with?(")")
          inner = str[10..-2]
          starved = 0
          percent = 0
          inner.split(",").each do |part|
            k, v = part.split("=").map(&:strip)
            starved = v.to_i if k == "starved"
            percent = v.to_i if k == "percent"
          end
          new(:impeached, starved_this_turn: starved, percent: percent)
        else
          new(str.downcase.to_sym)
        end
      end

      PLAYING = new(:playing)
      EXTINCT = new(:extinct)
    end
  end
end
