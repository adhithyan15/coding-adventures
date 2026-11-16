# frozen_string_literal: true

module CodingAdventures
  module LunarLander
    # Immutable value record capturing the complete flight simulation state.
    class LanderState
      attr_reader :time_seconds, :altitude_mft, :velocity_mft_s, :fuel_lbs,
                  :dry_mass_lbs, :status, :impact_velocity_mft_s

      def initialize(
        time_seconds:,
        altitude_mft:,
        velocity_mft_s:,
        fuel_lbs:,
        dry_mass_lbs:,
        status:,
        impact_velocity_mft_s: nil
      )
        @time_seconds = time_seconds
        @altitude_mft = altitude_mft
        @velocity_mft_s = velocity_mft_s
        @fuel_lbs = fuel_lbs
        @dry_mass_lbs = dry_mass_lbs
        @status = status.is_a?(FlightStatus) ? status : FlightStatus.parse(status)
        @impact_velocity_mft_s = impact_velocity_mft_s
        freeze
      end

      def [](key)
        case key.to_s
        when "time_seconds" then @time_seconds
        when "altitude_mft" then @altitude_mft
        when "velocity_mft_s" then @velocity_mft_s
        when "fuel_lbs" then @fuel_lbs
        when "dry_mass_lbs" then @dry_mass_lbs
        when "status" then @status
        when "impact_velocity_mft_s" then @impact_velocity_mft_s
        end
      end

      def to_h
        {
          time_seconds: @time_seconds,
          altitude_mft: @altitude_mft,
          velocity_mft_s: @velocity_mft_s,
          fuel_lbs: @fuel_lbs,
          dry_mass_lbs: @dry_mass_lbs,
          status: @status.to_s,
          impact_velocity_mft_s: @impact_velocity_mft_s
        }
      end

      def self.from_h(hash)
        return hash if hash.is_a?(LanderState)
        h = hash.transform_keys(&:to_s)
        new(
          time_seconds: h["time_seconds"],
          altitude_mft: h["altitude_mft"],
          velocity_mft_s: h["velocity_mft_s"],
          fuel_lbs: h["fuel_lbs"],
          dry_mass_lbs: h["dry_mass_lbs"],
          status: FlightStatus.parse(h["status"]),
          impact_velocity_mft_s: h["impact_velocity_mft_s"]
        )
      end

      def ==(other)
        return false unless other.is_a?(LanderState)
        @time_seconds == other.time_seconds &&
          @altitude_mft == other.altitude_mft &&
          @velocity_mft_s == other.velocity_mft_s &&
          @fuel_lbs == other.fuel_lbs &&
          @dry_mass_lbs == other.dry_mass_lbs &&
          @status == other.status &&
          @impact_velocity_mft_s == other.impact_velocity_mft_s
      end
      alias eql? ==

      def hash
        [
          @time_seconds,
          @altitude_mft,
          @velocity_mft_s,
          @fuel_lbs,
          @dry_mass_lbs,
          @status,
          @impact_velocity_mft_s
        ].hash
      end
    end
  end
end
