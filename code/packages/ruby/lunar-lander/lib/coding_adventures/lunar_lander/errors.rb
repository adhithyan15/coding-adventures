# frozen_string_literal: true

module CodingAdventures
  module LunarLander
    # Base class for all Lunar Lander flight simulation errors.
    class LanderError < StandardError
      attr_reader :code

      def initialize(msg = nil, code = nil)
        @code = code || self.class.default_code
        super(msg || self.class.default_message)
      end

      def self.default_code
        "LanderError"
      end

      def self.default_message
        "Lunar Lander simulation error"
      end

      def self.symbolic_name
        :lander_error
      end

      def ==(other)
        if other.is_a?(String)
          @code == other || to_s == other
        elsif other.is_a?(Symbol)
          @code.to_sym == other || self.class.symbolic_name == other
        elsif other.is_a?(Class)
          is_a?(other)
        else
          super
        end
      end
    end

    # Arm 1: Attempted to execute burn command when flight has already terminated.
    class FlightAlreadyEndedError < LanderError
      def self.default_code; "FlightAlreadyEnded"; end
      def self.default_message; "Flight has already ended"; end
      def self.symbolic_name; :flight_already_ended; end
    end
    FlightAlreadyEnded = FlightAlreadyEndedError

    # Arm 2: Commanded burn rate exceeds engine maximum (B > 200 lbs/s).
    class ExcessiveBurnRateError < LanderError
      def self.default_code; "ExcessiveBurnRate"; end
      def self.default_message; "Burn rate exceeds maximum engine capacity"; end
      def self.symbolic_name; :excessive_burn_rate; end
    end
    ExcessiveBurnRate = ExcessiveBurnRateError

    # Arm 3: Duration is invalid (duration == 0 || duration > 10).
    class InvalidDurationError < LanderError
      def self.default_code; "InvalidDuration"; end
      def self.default_message; "Duration is invalid (must be between 1 and 10 seconds)"; end
      def self.symbolic_name; :invalid_duration; end
    end
    InvalidDuration = InvalidDurationError

    # Arm 4: Arithmetic overflow or state/kinematic coordinate boundary violation.
    class ArithmeticOverflowError < LanderError
      def self.default_code; "ArithmeticOverflow"; end
      def self.default_message; "Arithmetic overflow or coordinate bounds violation"; end
      def self.symbolic_name; :arithmetic_overflow; end
    end
    ArithmeticOverflow = ArithmeticOverflowError
  end
end
