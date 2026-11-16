# frozen_string_literal: true

module CodingAdventures
  module LunarLander
    # Pilot burn order for a discrete simulation turn.
    class BurnCommand
      attr_reader :burn_rate_lbs_s, :duration_seconds

      def initialize(*args, **kwargs)
        if args.length == 2
          @burn_rate_lbs_s = args[0]
          @duration_seconds = args[1]
        elsif kwargs.key?(:burn_rate_lbs_s) && kwargs.key?(:duration_seconds)
          @burn_rate_lbs_s = kwargs[:burn_rate_lbs_s]
          @duration_seconds = kwargs[:duration_seconds]
        else
          raise ArgumentError, "BurnCommand requires burn_rate_lbs_s and duration_seconds"
        end
        freeze
      end

      def self.from(cmd)
        return cmd if cmd.is_a?(BurnCommand)
        if cmd.is_a?(Hash)
          h = cmd.transform_keys(&:to_s)
          new(burn_rate_lbs_s: h["burn_rate_lbs_s"], duration_seconds: h["duration_seconds"])
        elsif cmd.is_a?(Array) && cmd.length == 2
          new(cmd[0], cmd[1])
        else
          raise ArgumentError, "Cannot convert #{cmd.class} to BurnCommand"
        end
      end

      def [](key)
        case key.to_s
        when "burn_rate_lbs_s" then @burn_rate_lbs_s
        when "duration_seconds" then @duration_seconds
        end
      end

      def to_h
        {
          burn_rate_lbs_s: @burn_rate_lbs_s,
          duration_seconds: @duration_seconds
        }
      end

      def ==(other)
        return false unless other.is_a?(BurnCommand)
        @burn_rate_lbs_s == other.burn_rate_lbs_s &&
          @duration_seconds == other.duration_seconds
      end
      alias eql? ==

      def hash
        [@burn_rate_lbs_s, @duration_seconds].hash
      end
    end
  end
end
