# frozen_string_literal: true

module CodingAdventures
  module LunarLander
    # Base class for paint instructions in a PaintScene.
    class PaintInstruction
      attr_reader :kind
    end

    # Text paint instruction.
    class PaintText < PaintInstruction
      attr_reader :x, :y, :text, :font_size, :fill, :text_align

      def initialize(x:, y:, text:, font_size: 1.0, fill: "#ffffff", text_align: :left)
        @kind = :text
        @x = x.to_f
        @y = y.to_f
        @text = text
        @font_size = font_size.to_f
        @fill = fill
        @text_align = text_align.to_sym
        freeze
      end

      def to_h
        {
          kind: "text",
          x: @x,
          y: @y,
          text: @text,
          font_size: @font_size,
          fill: @fill,
          text_align: @text_align.to_s
        }
      end
    end

    # Rectangle paint instruction used for gauge bars.
    class PaintRect < PaintInstruction
      attr_reader :x, :y, :width, :height, :fill

      def initialize(x:, y:, width:, height:, fill: "#00ff00")
        @kind = :rect
        @x = x.to_f
        @y = y.to_f
        @width = width.to_f
        @height = height.to_f
        @fill = fill
        freeze
      end

      def to_h
        {
          kind: "rect",
          x: @x,
          y: @y,
          width: @width,
          height: @height,
          fill: @fill
        }
      end
    end

    # Declarative 2D intermediate scene (P2D00).
    class PaintScene
      attr_reader :width, :height, :background, :instructions

      def initialize(width:, height:, background: "#000000", instructions: [])
        @width = width.to_f
        @height = height.to_f
        @background = background
        @instructions = instructions.dup
      end

      def freeze
        @instructions.each(&:freeze)
        @instructions.freeze
        super
      end

      def to_h
        {
          width: @width,
          height: @height,
          background: @background,
          instructions: @instructions.map(&:to_h)
        }
      end
    end

    module Telemetry
      module_function

      def format_with_commas(n)
        negative = n < 0
        s = n.abs.to_s
        parts = []
        while s.length > 3
          parts.unshift(s[-3..-1])
          s = s[0...-3]
        end
        parts.unshift(s) unless s.empty?
        str = parts.join(",")
        negative ? "-#{str}" : str
      end

      # Translates a flight state into a declarative 76x5 PaintScene.
      def to_paint_scene(state, last_burn_rate_lbs_s = 0)
        state_obj = state.is_a?(LanderState) ? state : LanderState.from_h(state)

        alt_clamped = state_obj.altitude_mft.clamp(0, INITIAL_ALTITUDE_MFT)
        alt_pct = CodingAdventures::LunarLander.trunc_div(alt_clamped * 100, INITIAL_ALTITUDE_MFT)
        alt_bars = CodingAdventures::LunarLander.trunc_div(alt_clamped * 20, INITIAL_ALTITUDE_MFT)

        fuel_clamped = [state_obj.fuel_lbs, INITIAL_FUEL_LBS].min
        fuel_pct = CodingAdventures::LunarLander.trunc_div(fuel_clamped * 100, INITIAL_FUEL_LBS)
        fuel_bars = CodingAdventures::LunarLander.trunc_div(fuel_clamped * 20, INITIAL_FUEL_LBS)

        alt_ft = CodingAdventures::LunarLander.trunc_div(state_obj.altitude_mft, 1000)
        vel_fps = CodingAdventures::LunarLander.trunc_div(state_obj.velocity_mft_s, 1000)
        fuel_lbs = state_obj.fuel_lbs

        alt_str = format_with_commas(alt_ft)
        fuel_str = format_with_commas(fuel_lbs)

        rate_label, rate_text, rate_symbol = if state_obj.status.in_flight?
          if state_obj.velocity_mft_s < 0
            ["RATE:", "DESCENT", "▼"]
          elsif state_obj.velocity_mft_s > 0
            ["RATE:", "ASCENT", "▲"]
          else
            ["RATE:", "HOVER", "◆"]
          end
        else
          desc = case state_obj.status.outcome
          when LandingOutcome::PERFECT then "PERFECT"
          when LandingOutcome::HARD then "HARD"
          when LandingOutcome::CRASH then "CRASH"
          when LandingOutcome::FATAL then "FATAL"
          when LandingOutcome::CRATER then "CRATER"
          else "UNKNOWN"
          end
          ["STATUS:", desc, "★"]
        end

        row0 = "┌───────────────────────── LUNAR LANDER TELEMETRY ─────────────────────────┐"
        left1 = " TIME: #{state_obj.time_seconds.to_s.rjust(6)} SEC    "
        right1 = " ALTITUDE: #{alt_str.rjust(7)} FT [░░░░░░░░░░░░░░░░░░░░] #{alt_pct.to_s.rjust(3)}%   "
        row1 = "│#{left1}│#{right1}│"

        left2 = " VELOCITY: #{vel_fps.to_s.rjust(5)} FT/S"
        right2 = " FUEL:    #{fuel_str.rjust(7)} LBS [░░░░░░░░░░░░░░░░░░░░] #{fuel_pct.to_s.rjust(3)}%   "
        row2 = "│#{left2}│#{right2}│"

        left3 = " BURN:     #{last_burn_rate_lbs_s.to_s.rjust(4)} LB/S "
        rate_col = "#{rate_label.ljust(7)} #{rate_text.ljust(8)} #{rate_symbol}"
        row3 = "│#{left3}│ #{rate_col.ljust(51)}│"
        row4 = "└──────────────────────────────────────────────────────────────────────────┘"

        full_text = "#{row0}\n#{row1}\n#{row2}\n#{row3}\n#{row4}"

        instructions = []
        instructions << PaintText.new(
          x: 0.0,
          y: 0.0,
          text: full_text,
          font_size: 1.0,
          fill: "#ffffff",
          text_align: :left
        )

        if alt_bars > 0
          instructions << PaintRect.new(
            x: 46.0,
            y: 1.0,
            width: (alt_bars - 1).to_f,
            height: 0.0,
            fill: "#00ff00"
          )
        end

        if fuel_bars > 0
          instructions << PaintRect.new(
            x: 46.0,
            y: 2.0,
            width: (fuel_bars - 1).to_f,
            height: 0.0,
            fill: "#00ff00"
          )
        end

        PaintScene.new(
          width: 76.0,
          height: 5.0,
          background: "#000000",
          instructions: instructions
        ).freeze
      end

      # Convenience helper generating a PaintScene with zero burn rate.
      def telemetry_scene(state)
        to_paint_scene(state, 0)
      end

      # Renders a flight state telemetry dashboard into a deterministic ASCII string.
      #
      # Renders the 5 rows x 76 Unicode codepoints character grid, overlaying '█'
      # on row 1 and row 2 over the track characters [col 46..46+bars-1].
      def render_telemetry_ascii(state, last_burn_rate_lbs_s = 0)
        scene = to_paint_scene(state, last_burn_rate_lbs_s)
        text_inst = scene.instructions.find { |i| i.kind == :text }
        lines = text_inst.text.split("\n")
        grid = lines.map { |l| l.chars }

        scene.instructions.each do |inst|
          next unless inst.kind == :rect
          row = inst.y.to_i
          col_start = inst.x.to_i
          col_end = col_start + inst.width.to_i
          (col_start..col_end).each do |col|
            grid[row][col] = "█" if row < grid.length && col < grid[row].length
          end
        end

        grid.map { |row_chars| row_chars.join.rstrip }.join("\n")
      end

      # Convenience helper rendering telemetry with zero burn rate.
      def render_telemetry(state)
        render_telemetry_ascii(state, 0)
      end
    end
  end
end
