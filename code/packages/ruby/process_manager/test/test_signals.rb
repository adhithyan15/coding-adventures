# frozen_string_literal: true

require_relative "test_helper"

# = Tests for Signal Constants and SignalManager
#
# Signals are software interrupts -- the Unix way for processes to
# communicate and for the kernel to control processes. These tests
# verify that signal numbers match POSIX, that masking works correctly,
# and that SIGKILL/SIGSTOP are truly uncatchable.

class TestSignal < Minitest::Test
  include CodingAdventures::ProcessManager

  # -- Signal Constants --

  def test_signal_numbers_match_posix
    # These numbers are standardized by POSIX. Every Unix system uses them.
    assert_equal 2,  Signal::SIGINT
    assert_equal 9,  Signal::SIGKILL
    assert_equal 15, Signal::SIGTERM
    assert_equal 17, Signal::SIGCHLD
    assert_equal 18, Signal::SIGCONT
    assert_equal 19, Signal::SIGSTOP
  end

  def test_all_signals_list
    assert_equal [2, 9, 15, 17, 18, 19], Signal::ALL
  end

  def test_valid_signal_check
    Signal::ALL.each do |sig|
      assert Signal.valid?(sig), "Signal #{sig} should be valid"
    end
    refute Signal.valid?(0)
    refute Signal.valid?(99)
    refute Signal.valid?(-1)
  end

  def test_uncatchable_signals
    assert Signal.uncatchable?(Signal::SIGKILL)
    assert Signal.uncatchable?(Signal::SIGSTOP)
    refute Signal.uncatchable?(Signal::SIGTERM)
    refute Signal.uncatchable?(Signal::SIGINT)
    refute Signal.uncatchable?(Signal::SIGCHLD)
    refute Signal.uncatchable?(Signal::SIGCONT)
  end

  def test_fatal_by_default
    assert Signal.fatal_by_default?(Signal::SIGINT)
    assert Signal.fatal_by_default?(Signal::SIGKILL)
    assert Signal.fatal_by_default?(Signal::SIGTERM)
    refute Signal.fatal_by_default?(Signal::SIGCHLD)
    refute Signal.fatal_by_default?(Signal::SIGCONT)
    refute Signal.fatal_by_default?(Signal::SIGSTOP)
  end

  def test_signal_names
    assert_equal "SIGINT",  Signal.name_for(Signal::SIGINT)
    assert_equal "SIGKILL", Signal.name_for(Signal::SIGKILL)
    assert_equal "SIGTERM", Signal.name_for(Signal::SIGTERM)
    assert_equal "SIGCHLD", Signal.name_for(Signal::SIGCHLD)
    assert_equal "SIGCONT", Signal.name_for(Signal::SIGCONT)
    assert_equal "SIGSTOP", Signal.name_for(Signal::SIGSTOP)
    assert_match(/UNKNOWN/, Signal.name_for(99))
  end
end

class TestSignalManager < Minitest::Test
  include CodingAdventures::ProcessManager

  def setup
    @manager = SignalManager.new
    @pcb = ProcessControlBlock.new(pid: 1, name: "test")
  end

  # -- send_signal --

  def test_send_signal_adds_to_pending
    @manager.send_signal(@pcb, Signal::SIGTERM)
    assert_equal [Signal::SIGTERM], @pcb.pending_signals
  end

  def test_send_multiple_signals
    @manager.send_signal(@pcb, Signal::SIGTERM)
    @manager.send_signal(@pcb, Signal::SIGINT)
    assert_equal [Signal::SIGTERM, Signal::SIGINT], @pcb.pending_signals
  end

  def test_send_invalid_signal_returns_false
    refute @manager.send_signal(@pcb, 99)
    assert_empty @pcb.pending_signals
  end

  # -- deliver_pending --

  def test_deliver_fatal_signal_without_handler
    @manager.send_signal(@pcb, Signal::SIGTERM)
    actions = @manager.deliver_pending(@pcb)

    assert_equal 1, actions.size
    assert_equal Signal::SIGTERM, actions[0][:signal]
    assert_equal :kill, actions[0][:action]
    assert_empty @pcb.pending_signals
  end

  def test_deliver_signal_with_custom_handler
    @manager.register_handler(@pcb, Signal::SIGTERM, 0x1000)
    @manager.send_signal(@pcb, Signal::SIGTERM)
    actions = @manager.deliver_pending(@pcb)

    assert_equal 1, actions.size
    assert_equal :handler, actions[0][:action]
    assert_equal 0x1000, actions[0][:address]
    assert_empty @pcb.pending_signals
  end

  def test_deliver_sigkill_always_kills
    # Even if a handler is registered (which it can't be), SIGKILL always kills.
    @manager.send_signal(@pcb, Signal::SIGKILL)
    actions = @manager.deliver_pending(@pcb)

    assert_equal 1, actions.size
    assert_equal :kill, actions[0][:action]
  end

  def test_deliver_sigstop_always_stops
    @manager.send_signal(@pcb, Signal::SIGSTOP)
    actions = @manager.deliver_pending(@pcb)

    assert_equal 1, actions.size
    assert_equal :stop, actions[0][:action]
  end

  def test_deliver_sigcont_continues
    @manager.send_signal(@pcb, Signal::SIGCONT)
    actions = @manager.deliver_pending(@pcb)

    assert_equal 1, actions.size
    assert_equal :continue, actions[0][:action]
  end

  def test_deliver_sigchld_ignored_by_default
    @manager.send_signal(@pcb, Signal::SIGCHLD)
    actions = @manager.deliver_pending(@pcb)

    assert_equal 1, actions.size
    assert_equal :ignore, actions[0][:action]
  end

  def test_deliver_with_no_pending_signals
    actions = @manager.deliver_pending(@pcb)
    assert_empty actions
  end

  # -- register_handler --

  def test_register_handler_for_catchable_signal
    result = @manager.register_handler(@pcb, Signal::SIGTERM, 0x1000)
    assert result
    assert_equal 0x1000, @pcb.signal_handlers[Signal::SIGTERM]
  end

  def test_cannot_register_handler_for_sigkill
    result = @manager.register_handler(@pcb, Signal::SIGKILL, 0x1000)
    refute result
    refute @pcb.signal_handlers.key?(Signal::SIGKILL)
  end

  def test_cannot_register_handler_for_sigstop
    result = @manager.register_handler(@pcb, Signal::SIGSTOP, 0x1000)
    refute result
    refute @pcb.signal_handlers.key?(Signal::SIGSTOP)
  end

  def test_register_handler_for_invalid_signal
    result = @manager.register_handler(@pcb, 99, 0x1000)
    refute result
  end

  # -- mask/unmask --

  def test_mask_signal
    result = @manager.mask(@pcb, Signal::SIGTERM)
    assert result
    assert_includes @pcb.signal_mask, Signal::SIGTERM
  end

  def test_unmask_signal
    @manager.mask(@pcb, Signal::SIGTERM)
    result = @manager.unmask(@pcb, Signal::SIGTERM)
    assert result
    refute_includes @pcb.signal_mask, Signal::SIGTERM
  end

  def test_cannot_mask_sigkill
    result = @manager.mask(@pcb, Signal::SIGKILL)
    refute result
    refute_includes @pcb.signal_mask, Signal::SIGKILL
  end

  def test_cannot_mask_sigstop
    result = @manager.mask(@pcb, Signal::SIGSTOP)
    refute result
    refute_includes @pcb.signal_mask, Signal::SIGSTOP
  end

  def test_mask_invalid_signal
    refute @manager.mask(@pcb, 99)
  end

  def test_unmask_invalid_signal
    refute @manager.unmask(@pcb, 99)
  end

  def test_masked_signal_stays_pending
    @manager.mask(@pcb, Signal::SIGTERM)
    @manager.send_signal(@pcb, Signal::SIGTERM)
    actions = @manager.deliver_pending(@pcb)

    # Signal was masked, so it stays pending and no action is taken.
    assert_empty actions
    assert_equal [Signal::SIGTERM], @pcb.pending_signals
  end

  def test_unmask_then_deliver
    @manager.mask(@pcb, Signal::SIGTERM)
    @manager.send_signal(@pcb, Signal::SIGTERM)
    @manager.deliver_pending(@pcb)  # stays pending

    @manager.unmask(@pcb, Signal::SIGTERM)
    actions = @manager.deliver_pending(@pcb)

    # Now it should be delivered.
    assert_equal 1, actions.size
    assert_equal :kill, actions[0][:action]
    assert_empty @pcb.pending_signals
  end

  def test_sigkill_bypasses_mask
    # Even though we try to mask SIGKILL, it can't be masked.
    # But send_signal still adds it to pending.
    @manager.send_signal(@pcb, Signal::SIGKILL)
    # Even if signal_mask somehow contains SIGKILL (it shouldn't via mask()),
    # deliver_pending treats it as uncatchable.
    actions = @manager.deliver_pending(@pcb)

    assert_equal 1, actions.size
    assert_equal :kill, actions[0][:action]
  end

  # -- fatal? --

  def test_sigkill_always_fatal
    assert @manager.fatal?(@pcb, Signal::SIGKILL)
  end

  def test_sigterm_fatal_without_handler
    assert @manager.fatal?(@pcb, Signal::SIGTERM)
  end

  def test_sigterm_not_fatal_with_handler
    @manager.register_handler(@pcb, Signal::SIGTERM, 0x1000)
    refute @manager.fatal?(@pcb, Signal::SIGTERM)
  end

  def test_sigchld_not_fatal
    refute @manager.fatal?(@pcb, Signal::SIGCHLD)
  end

  def test_sigint_fatal_without_handler
    assert @manager.fatal?(@pcb, Signal::SIGINT)
  end

  def test_sigint_not_fatal_with_handler
    @manager.register_handler(@pcb, Signal::SIGINT, 0x2000)
    refute @manager.fatal?(@pcb, Signal::SIGINT)
  end
end
