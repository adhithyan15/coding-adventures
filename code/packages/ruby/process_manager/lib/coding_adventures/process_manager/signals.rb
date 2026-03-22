# frozen_string_literal: true

# = Signals: Software Interrupts Between Processes
#
# Signals are the Unix mechanism for asynchronous communication between
# processes (and between the kernel and processes). They are "software
# interrupts" -- a way to notify a process that something has happened.
#
# == Real-World Analogy
#
# Think of signals like a tap on the shoulder. You're working at your desk
# (running a process), and someone taps your shoulder (sends a signal):
#
#   - A polite tap (SIGTERM): "Hey, please finish up and leave."
#     You can ignore it, or you can clean up and go.
#
#   - A forceful shove (SIGKILL): "GET OUT NOW."
#     You cannot ignore this. You're dragged out immediately.
#
#   - A freeze command (SIGSTOP): "Don't move."
#     You freeze in place. You can't ignore this either.
#
#   - A resume command (SIGCONT): "Okay, you can move again."
#     You unfreeze and continue where you left off.
#
# == Why These Specific Numbers?
#
# The signal numbers (SIGINT=2, SIGKILL=9, etc.) come from POSIX, the
# standard that defines how Unix-like systems behave. Every Unix system
# uses the same numbering. We implement only the 6 most essential signals;
# real systems define about 31 standard signals.
#
# == Signal Delivery Flow
#
#   Process A calls kill(pid_B, SIGTERM)
#         |
#         v
#   Kernel adds SIGTERM to B's pending_signals list
#         |
#         v
#   When B is next scheduled (context switch to B):
#         |
#         v
#   Kernel checks B's pending_signals:
#         |
#         +-- Is the signal masked? --> Keep in pending, skip
#         |
#         +-- Does B have a custom handler?
#         |     |
#         |     +-- YES: Set PC = handler address. B runs handler.
#         |     |
#         |     +-- NO:  Apply default action (usually terminate).
#         |
#         +-- Special cases:
#               SIGKILL: always terminates, cannot be caught or masked.
#               SIGSTOP: always stops, cannot be caught or masked.
#               SIGCONT: always resumes a stopped process.

module CodingAdventures
  module ProcessManager
    # Signal constants matching POSIX signal numbers.
    #
    # We implement only the 6 most important signals. Real systems
    # define about 31, but these 6 cover all the fundamental concepts:
    # user interrupts, forced/polite termination, child notification,
    # and process control (stop/continue).
    module Signal
      # SIGINT (2) -- Interrupt signal.
      # Sent when the user presses Ctrl+C in the terminal.
      # Default action: terminate the process.
      # Can be caught: YES. Programs often catch it to save work.
      SIGINT  = 2

      # SIGKILL (9) -- Kill signal.
      # Unconditionally terminates the process. This is the "nuclear option."
      # Default action: terminate immediately.
      # Can be caught: NO. Cannot be caught, blocked, or ignored.
      # Always try SIGTERM first; only use SIGKILL as a last resort.
      SIGKILL = 9

      # SIGTERM (15) -- Terminate signal.
      # Polite request to exit. This is what `kill <pid>` sends by default.
      # Default action: terminate the process.
      # Can be caught: YES. Servers catch it for graceful shutdown.
      SIGTERM = 15

      # SIGCHLD (17) -- Child status changed.
      # Sent to the parent when a child process exits, stops, or continues.
      # Default action: ignore (the parent must explicitly call wait()).
      # Can be caught: YES. Shells catch it to know when background jobs finish.
      SIGCHLD = 17

      # SIGCONT (18) -- Continue signal.
      # Resumes a process that was stopped by SIGSTOP.
      # Sent by `fg` in the shell or `kill -CONT <pid>`.
      # Can be caught: YES, but the process always resumes regardless.
      SIGCONT = 18

      # SIGSTOP (19) -- Stop signal.
      # Suspends the process (freezes it in place).
      # Default action: stop the process.
      # Can be caught: NO. Like SIGKILL, this cannot be caught or ignored.
      # Sent by Ctrl+Z in the shell.
      SIGSTOP = 19

      # All valid signal values.
      ALL = [SIGINT, SIGKILL, SIGTERM, SIGCHLD, SIGCONT, SIGSTOP].freeze

      # Signals that cannot be caught, blocked, or ignored.
      # SIGKILL and SIGSTOP are special -- they always take effect.
      # This is a security feature: it guarantees that the kernel can
      # always terminate or stop a misbehaving process.
      UNCATCHABLE = [SIGKILL, SIGSTOP].freeze

      # Signals whose default action is to terminate the process.
      FATAL_BY_DEFAULT = [SIGINT, SIGKILL, SIGTERM].freeze

      # Human-readable names for each signal.
      NAMES = {
        SIGINT  => "SIGINT",
        SIGKILL => "SIGKILL",
        SIGTERM => "SIGTERM",
        SIGCHLD => "SIGCHLD",
        SIGCONT => "SIGCONT",
        SIGSTOP => "SIGSTOP"
      }.freeze

      # Returns true if the given value is a valid signal number.
      def self.valid?(value)
        ALL.include?(value)
      end

      # Returns true if the signal cannot be caught or masked.
      #
      # SIGKILL and SIGSTOP are the two uncatchable signals in Unix.
      # No matter what a process does, it cannot prevent these from
      # taking effect. This is a deliberate design choice -- the system
      # administrator must always be able to kill or stop any process.
      def self.uncatchable?(value)
        UNCATCHABLE.include?(value)
      end

      # Returns true if the signal's default action is to terminate.
      def self.fatal_by_default?(value)
        FATAL_BY_DEFAULT.include?(value)
      end

      # Returns the name of a signal, or "UNKNOWN" for invalid values.
      def self.name_for(value)
        NAMES.fetch(value, "UNKNOWN(#{value})")
      end
    end

    # SignalManager handles signal delivery, masking, and handler registration.
    #
    # Each process has its own signal state (pending signals, handlers, mask),
    # but the SignalManager provides the logic for manipulating that state.
    # It operates on ProcessControlBlock instances directly.
    #
    # == Usage Example
    #
    #   manager = SignalManager.new
    #   pcb = ProcessControlBlock.new(pid: 1, name: "shell")
    #
    #   # Register a handler for SIGTERM at address 0x1000
    #   manager.register_handler(pcb, Signal::SIGTERM, 0x1000)
    #
    #   # Send SIGTERM to the process
    #   manager.send_signal(pcb, Signal::SIGTERM)
    #   pcb.pending_signals  #=> [Signal::SIGTERM]
    #
    #   # Deliver pending signals (returns actions to take)
    #   actions = manager.deliver_pending(pcb)
    #   actions  #=> [{signal: SIGTERM, action: :handler, address: 0x1000}]
    class SignalManager
      # Sends a signal to a process by adding it to the pending list.
      #
      # This does NOT immediately deliver the signal. Signals are delivered
      # when the process is next scheduled (see deliver_pending).
      #
      # Special cases:
      #   - SIGKILL is always added (cannot be masked).
      #   - SIGSTOP is always added (cannot be masked).
      #
      # @param pcb [ProcessControlBlock] the target process
      # @param signal [Integer] the signal number to send
      # @return [Boolean] true if the signal was successfully queued
      def send_signal(pcb, signal)
        return false unless Signal.valid?(signal)

        pcb.pending_signals << signal
        true
      end

      # Delivers all pending signals to a process.
      #
      # This is called by the kernel when a process is about to run.
      # It processes each pending signal and determines the action:
      #
      #   1. If the signal is masked (and not uncatchable), skip it.
      #   2. If the signal is SIGKILL, action = :kill (terminate).
      #   3. If the signal is SIGSTOP, action = :stop.
      #   4. If the signal is SIGCONT, action = :continue.
      #   5. If the process has a custom handler, action = :handler.
      #   6. Otherwise, apply the default action.
      #
      # @param pcb [ProcessControlBlock] the process whose signals to deliver
      # @return [Array<Hash>] list of actions, each with :signal, :action, and optional :address
      def deliver_pending(pcb)
        actions = []
        remaining = []

        pcb.pending_signals.each do |signal|
          # Masked signals stay pending (but SIGKILL/SIGSTOP bypass the mask).
          if pcb.signal_mask.include?(signal) && !Signal.uncatchable?(signal)
            remaining << signal
            next
          end

          action = determine_action(pcb, signal)
          actions << action
        end

        pcb.pending_signals = remaining
        actions
      end

      # Registers a custom signal handler for a process.
      #
      # When the signal is delivered, the kernel will redirect the process's
      # program counter to the handler address. The handler function runs
      # in the process's context, then returns to where the process was
      # interrupted.
      #
      # SIGKILL and SIGSTOP cannot have custom handlers. Attempting to
      # register one is silently ignored (this matches Unix behavior).
      #
      # @param pcb [ProcessControlBlock] the process
      # @param signal [Integer] the signal number
      # @param handler_address [Integer] memory address of the handler function
      # @return [Boolean] true if the handler was registered
      def register_handler(pcb, signal, handler_address)
        return false unless Signal.valid?(signal)

        # SIGKILL and SIGSTOP cannot be caught. The kernel enforces this.
        return false if Signal.uncatchable?(signal)

        pcb.signal_handlers[signal] = handler_address
        true
      end

      # Adds a signal to the process's signal mask (blocks it).
      #
      # While masked, the signal accumulates in pending_signals but is
      # not delivered. Useful during critical sections where the process
      # must not be interrupted.
      #
      # SIGKILL and SIGSTOP cannot be masked. They always take effect.
      #
      # @param pcb [ProcessControlBlock] the process
      # @param signal [Integer] the signal number to mask
      # @return [Boolean] true if the signal was masked
      def mask(pcb, signal)
        return false unless Signal.valid?(signal)
        return false if Signal.uncatchable?(signal)

        pcb.signal_mask.add(signal)
        true
      end

      # Removes a signal from the process's signal mask (unblocks it).
      #
      # After unmasking, any pending instances of this signal will be
      # delivered on the next call to deliver_pending.
      #
      # @param pcb [ProcessControlBlock] the process
      # @param signal [Integer] the signal number to unmask
      # @return [Boolean] true if the signal was unmasked
      def unmask(pcb, signal)
        return false unless Signal.valid?(signal)

        pcb.signal_mask.delete(signal)
        true
      end

      # Returns true if a signal would be fatal for this process.
      #
      # A signal is fatal if:
      #   - It is SIGKILL (always fatal, no handler can prevent it), OR
      #   - It is fatal by default AND the process has no custom handler.
      #
      # @param pcb [ProcessControlBlock] the process
      # @param signal [Integer] the signal number
      # @return [Boolean] true if the signal would terminate the process
      def fatal?(pcb, signal)
        return true if signal == Signal::SIGKILL

        Signal.fatal_by_default?(signal) && !pcb.signal_handlers.key?(signal)
      end

      private

      # Determines what action to take for a given signal.
      #
      # This implements the Unix signal delivery rules:
      #   - SIGKILL always kills (cannot be overridden).
      #   - SIGSTOP always stops (cannot be overridden).
      #   - SIGCONT always continues (handler runs too, if registered).
      #   - Custom handler redirects execution to the handler address.
      #   - Default action depends on the signal type.
      def determine_action(pcb, signal)
        case signal
        when Signal::SIGKILL
          {signal: signal, action: :kill}
        when Signal::SIGSTOP
          {signal: signal, action: :stop}
        when Signal::SIGCONT
          {signal: signal, action: :continue}
        else
          if pcb.signal_handlers.key?(signal)
            {signal: signal, action: :handler, address: pcb.signal_handlers[signal]}
          elsif Signal.fatal_by_default?(signal)
            {signal: signal, action: :kill}
          else
            # Non-fatal signals with no handler are ignored (e.g., SIGCHLD).
            {signal: signal, action: :ignore}
          end
        end
      end
    end
  end
end
