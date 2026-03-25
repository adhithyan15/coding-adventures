# frozen_string_literal: true

# coding_adventures_event_loop — A pluggable, generic event loop.
#
# The heartbeat of any interactive application.
#
# == What is an event loop?
#
# An event loop is the outermost structure of any interactive program. It runs
# forever (until told to stop), repeatedly asking "did anything happen?" and
# dispatching whatever happened to registered handlers:
#
#   while running
#     collect events from all sources
#     for each event
#       dispatch to handlers
#       if any handler says :exit → stop
#
# == Why generic and pluggable?
#
# A naïve loop hardcodes what events look like (key_press, mouse_move…). That
# makes the loop untestable. This version accepts any object as an event, and
# any object that responds to #poll as a source. No inheritance required.
#
# == Quick start
#
#   require "coding_adventures_event_loop"
#
#   source = Object.new
#   count = 0
#   source.define_singleton_method(:poll) do
#     count += 1
#     count < 4 ? [count] : [:quit]
#   end
#
#   loop = CodingAdventures::EventLoop::Loop.new
#   loop.add_source(source)
#   loop.on_event { |e| e == :quit ? CodingAdventures::EventLoop::ControlFlow::EXIT : CodingAdventures::EventLoop::ControlFlow::CONTINUE }
#   loop.run

require_relative "coding_adventures/event_loop/version"

module CodingAdventures
  # Pluggable generic event loop — the heartbeat of interactive applications.
  module EventLoop
    # ════════════════════════════════════════════════════════════════════════
    # ControlFlow
    # ════════════════════════════════════════════════════════════════════════

    # Signals whether the event loop should continue running or stop.
    #
    # Using named constants instead of +true+/+false+ makes handler return
    # values self-documenting at the call site:
    #
    #   return ControlFlow::EXIT      # intent is clear
    #   return true                   # ambiguous — true means what?
    module ControlFlow
      # Keep looping — there is more work to do.
      CONTINUE = :continue

      # Stop the loop immediately after this event.
      EXIT = :exit
    end

    # ════════════════════════════════════════════════════════════════════════
    # Loop
    # ════════════════════════════════════════════════════════════════════════

    # A pluggable, generic event loop.
    #
    # Sources must respond to +#poll+ and return an Array (empty if nothing
    # ready). Handlers are blocks/Procs that receive one event and return
    # +ControlFlow::CONTINUE+ or +ControlFlow::EXIT+.
    #
    # Single-threaded by design. All sources and handlers run on the calling
    # thread. Multi-threaded event injection is handled by wrapping a
    # +Queue+ in a source whose +poll+ drains it.
    #
    # @example
    #   loop = CodingAdventures::EventLoop::Loop.new
    #   loop.add_source(my_source)
    #   loop.on_event { |e| e == :quit ? ControlFlow::EXIT : ControlFlow::CONTINUE }
    #   loop.run
    class Loop
      def initialize
        @sources  = []
        @handlers = []
        @stopped  = false
      end

      # Register an event source. Sources are polled in registration order.
      #
      # Any object that responds to +#poll → Array+ qualifies as a source.
      # Duck typing — no inheritance required.
      #
      # @param source [#poll] an object whose +#poll+ returns an Array of events
      # @return [self] for chaining
      def add_source(source)
        @sources << source
        self
      end

      # Register an event handler block.
      #
      # Handlers receive each event in registration order. If any handler
      # returns +ControlFlow::EXIT+, the loop stops immediately — subsequent
      # handlers for the same event are not called.
      #
      # @yield [event] receives one event per call
      # @yieldreturn [Symbol] +ControlFlow::CONTINUE+ or +ControlFlow::EXIT+
      # @return [self] for chaining
      def on_event(&handler)
        @handlers << handler
        self
      end

      # Signal the loop to stop on the next iteration.
      #
      # Safe to call from outside a handler (e.g., from another thread that
      # sets a flag the loop checks via a source).
      def stop
        @stopped = true
      end

      # Start the event loop. Blocks until a handler returns EXIT or #stop is called.
      #
      # Each iteration performs three phases:
      #
      # 1. *Collect* — call +#poll+ on every source; concat results to a queue.
      # 2. *Dispatch* — deliver each event to every handler in order.
      #    Stop if any handler returns +EXIT+.
      # 3. *Idle* — if the queue was empty, call +Thread.pass+ to yield the
      #    scheduler. Without this an idle loop would spin at 100 % CPU.
      def run
        @stopped = false

        until @stopped
          # ── Phase 1: Collect ────────────────────────────────────────────
          #
          # Ask every source for events. Concat whatever each returns.
          # Sources return empty arrays when nothing is ready — that is normal.
          queue = []
          @sources.each { |src| queue.concat(src.poll) }

          # ── Phase 2: Dispatch ────────────────────────────────────────────
          #
          # Deliver each event to all handlers in registration order.
          # Stop the moment any handler returns EXIT.
          should_exit = false
          queue.each do |event|
            @handlers.each do |handler|
              if handler.call(event) == ControlFlow::EXIT
                should_exit = true
                break
              end
            end
            break if should_exit
          end
          return if should_exit

          # ── Phase 3: Idle ────────────────────────────────────────────────
          #
          # If nothing happened, yield the Ruby thread scheduler. Thread.pass
          # says "I have nothing to do right now; let other threads run."
          Thread.pass if queue.empty?
        end
      end
    end
  end
end
