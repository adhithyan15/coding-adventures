# frozen_string_literal: true

module CodingAdventures
  module LunarLander
    # Terminal landing outcome classified by touchdown impact speed magnitude.
    module LandingOutcome
      PERFECT = :perfect
      HARD = :hard
      CRASH = :crash
      FATAL = :fatal
      CRATER = :crater

      ALL = [PERFECT, HARD, CRASH, FATAL, CRATER].freeze

      def self.from_speed(speed_mft_s)
        speed = speed_mft_s.abs
        if speed <= 5_000
          PERFECT
        elsif speed <= 15_000
          HARD
        elsif speed <= 30_000
          CRASH
        elsif speed <= 60_000
          FATAL
        else
          CRATER
        end
      end

      def self.name_for(outcome)
        case outcome
        when PERFECT then "Perfect"
        when HARD then "Hard"
        when CRASH then "Crash"
        when FATAL then "Fatal"
        when CRATER then "Crater"
        else outcome.to_s
        end
      end
    end

    # Lifecycle flight status of the Lunar Module.
    class FlightStatus
      attr_reader :kind, :outcome

      def initialize(kind, outcome = nil)
        @kind = kind.to_sym
        @outcome = outcome
        freeze
      end

      def self.in_flight
        @in_flight ||= new(:in_flight).freeze
      end

      def self.landed(outcome)
        new(:landed, outcome)
      end

      def self.parse(status)
        return status if status.is_a?(FlightStatus)
        str = status.to_s
        if str == "InFlight" || str == "in_flight"
          in_flight
        elsif str =~ /\ALanded\((.*)\)\z/i
          out_str = $1.downcase
          outcome = case out_str
          when "perfect" then LandingOutcome::PERFECT
          when "hard" then LandingOutcome::HARD
          when "crash" then LandingOutcome::CRASH
          when "fatal" then LandingOutcome::FATAL
          when "crater" then LandingOutcome::CRATER
          else raise ArgumentError, "Unknown landing outcome: #{$1}"
          end
          landed(outcome)
        else
          raise ArgumentError, "Unknown flight status: #{str}"
        end
      end

      def in_flight?
        @kind == :in_flight
      end

      def landed?
        @kind == :landed
      end

      def to_s
        if in_flight?
          "InFlight"
        else
          "Landed(#{LandingOutcome.name_for(@outcome)})"
        end
      end

      def inspect
        "#<FlightStatus #{to_s}>"
      end

      def ==(other)
        if other.is_a?(FlightStatus)
          @kind == other.kind && @outcome == other.outcome
        elsif other.is_a?(String)
          to_s == other
        elsif other.is_a?(Symbol)
          (in_flight? && other == :in_flight) || (landed? && other == :landed) || to_s.to_sym == other
        else
          false
        end
      end
      alias eql? ==

      def hash
        [@kind, @outcome].hash
      end
    end
  end
end
