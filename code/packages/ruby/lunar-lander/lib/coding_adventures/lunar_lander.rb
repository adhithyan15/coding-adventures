# frozen_string_literal: true

require_relative "lunar_lander/version"
require_relative "lunar_lander/errors"
require_relative "lunar_lander/status"
require_relative "lunar_lander/command"
require_relative "lunar_lander/state"
require_relative "lunar_lander/paint"
require_relative "lunar_lander/sim"

module CodingAdventures
  # Pure, deterministic simulation core and ASCII telemetry for Lunar Lander (GAME02).
  module LunarLander
    extend self

    def to_paint_scene(state, last_burn_rate_lbs_s = 0)
      Telemetry.to_paint_scene(state, last_burn_rate_lbs_s)
    end

    def telemetry_scene(state)
      Telemetry.telemetry_scene(state)
    end

    def render_telemetry_ascii(state, last_burn_rate_lbs_s = 0)
      Telemetry.render_telemetry_ascii(state, last_burn_rate_lbs_s)
    end

    def render_telemetry(state)
      Telemetry.render_telemetry(state)
    end
  end
end
