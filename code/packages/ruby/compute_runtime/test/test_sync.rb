# frozen_string_literal: true

require_relative "test_helper"

# Tests for synchronization primitives -- Fence, Semaphore, Event.
class TestFence < Minitest::Test
  include CodingAdventures

  def test_default_unsignaled
    fence = ComputeRuntime::Fence.new
    refute fence.signaled
  end

  def test_create_signaled
    fence = ComputeRuntime::Fence.new(signaled: true)
    assert fence.signaled
  end

  def test_signal
    fence = ComputeRuntime::Fence.new
    fence.signal
    assert fence.signaled
  end

  def test_wait_signaled
    fence = ComputeRuntime::Fence.new(signaled: true)
    assert_equal true, fence.wait
  end

  def test_wait_unsignaled
    fence = ComputeRuntime::Fence.new
    assert_equal false, fence.wait
  end

  def test_reset
    fence = ComputeRuntime::Fence.new(signaled: true)
    fence.reset
    refute fence.signaled
  end

  def test_reuse
    fence = ComputeRuntime::Fence.new
    fence.signal
    assert fence.signaled
    fence.reset
    refute fence.signaled
    fence.signal
    assert fence.signaled
  end

  def test_unique_ids
    f1 = ComputeRuntime::Fence.new
    f2 = ComputeRuntime::Fence.new
    refute_equal f1.fence_id, f2.fence_id
  end

  def test_wait_cycles
    fence = ComputeRuntime::Fence.new
    assert_equal 0, fence.wait_cycles
  end

  def test_reset_clears_wait_cycles
    fence = ComputeRuntime::Fence.new
    fence.reset
    assert_equal 0, fence.wait_cycles
  end
end

class TestSemaphore < Minitest::Test
  include CodingAdventures

  def test_default_unsignaled
    sem = ComputeRuntime::Semaphore.new
    refute sem.signaled
  end

  def test_signal
    sem = ComputeRuntime::Semaphore.new
    sem.signal
    assert sem.signaled
  end

  def test_reset
    sem = ComputeRuntime::Semaphore.new
    sem.signal
    sem.reset
    refute sem.signaled
  end

  def test_unique_ids
    s1 = ComputeRuntime::Semaphore.new
    s2 = ComputeRuntime::Semaphore.new
    refute_equal s1.semaphore_id, s2.semaphore_id
  end
end

class TestEvent < Minitest::Test
  include CodingAdventures

  def test_default_unsignaled
    event = ComputeRuntime::Event.new
    refute event.signaled
  end

  def test_set
    event = ComputeRuntime::Event.new
    event.set
    assert event.signaled
  end

  def test_reset
    event = ComputeRuntime::Event.new
    event.set
    event.reset
    refute event.signaled
  end

  def test_status
    event = ComputeRuntime::Event.new
    assert_equal false, event.status
    event.set
    assert_equal true, event.status
  end

  def test_unique_ids
    e1 = ComputeRuntime::Event.new
    e2 = ComputeRuntime::Event.new
    refute_equal e1.event_id, e2.event_id
  end
end
