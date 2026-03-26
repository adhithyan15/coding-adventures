# frozen_string_literal: true

module CodingAdventures
  module SystemBoard
    PHASE_POWER_ON    = 0
    PHASE_BIOS        = 1
    PHASE_BOOTLOADER  = 2
    PHASE_KERNEL_INIT = 3
    PHASE_USER_PROGRAM = 4
    PHASE_IDLE        = 5

    PHASE_NAMES = {
      0 => "PowerOn", 1 => "BIOS", 2 => "Bootloader",
      3 => "KernelInit", 4 => "UserProgram", 5 => "Idle"
    }.freeze

    BootEvent = Data.define(:phase, :cycle, :description)

    class BootTrace
      attr_reader :events

      def initialize
        @events = []
      end

      def add_event(phase, cycle, description)
        @events << BootEvent.new(phase: phase, cycle: cycle, description: description)
      end

      def phases
        seen = {}
        result = []
        @events.each do |e|
          unless seen[e.phase]
            seen[e.phase] = true
            result << e.phase
          end
        end
        result
      end

      def events_in_phase(phase)
        @events.select { |e| e.phase == phase }
      end

      def total_cycles
        return 0 if @events.empty?
        @events.last.cycle
      end

      def phase_start_cycle(phase)
        e = @events.find { |ev| ev.phase == phase }
        e ? e.cycle : -1
      end
    end
  end
end
