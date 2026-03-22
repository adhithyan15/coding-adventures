# frozen_string_literal: true

require_relative "test_helper"

# Tests for InterruptController -- interrupt routing.
class TestInterruptControllerBasic < Minitest::Test
  def test_raise_and_acknowledge
    ic = CodingAdventures::Core::InterruptController.new(4)

    ic.raise_interrupt(1, 2) # interrupt 1 to core 2
    assert_equal 1, ic.pending_count

    pending = ic.pending_for_core(2)
    assert_equal 1, pending.length
    assert_equal 1, pending[0].interrupt_id

    # Acknowledge.
    ic.acknowledge(2, 1)
    assert_equal 0, ic.pending_count
    assert_equal 1, ic.acknowledged_count
  end

  def test_default_routing
    ic = CodingAdventures::Core::InterruptController.new(4)

    ic.raise_interrupt(5, -1) # should route to core 0
    pending = ic.pending_for_core(0)
    assert_equal 1, pending.length
  end

  def test_overflow_routing
    ic = CodingAdventures::Core::InterruptController.new(4)
    ic.raise_interrupt(1, 10) # core 10 doesn't exist, should go to 0
    pending = ic.pending_for_core(0)
    assert_equal 1, pending.length
  end

  def test_reset
    ic = CodingAdventures::Core::InterruptController.new(4)
    ic.raise_interrupt(1, 0)
    ic.acknowledge(0, 1)
    ic.reset

    assert_equal 0, ic.pending_count
    assert_equal 0, ic.acknowledged_count
  end

  def test_pending_for_core_empty
    ic = CodingAdventures::Core::InterruptController.new(4)
    pending = ic.pending_for_core(3)
    assert_equal 0, pending.length
  end
end
