# frozen_string_literal: true

require_relative "test_helper"

# Tests for MemoryController -- memory access serialization.
class TestMemoryControllerReadWrite < Minitest::Test
  def test_read_write_word
    mem = Array.new(4096, 0)
    mc = CodingAdventures::Core::MemoryController.new(mem, 10)

    mc.write_word(100, 0x1234ABCD)
    val = mc.read_word(100)
    assert_equal 0x1234ABCD, val
  end

  def test_load_program
    mem = Array.new(4096, 0)
    mc = CodingAdventures::Core::MemoryController.new(mem, 10)

    program = [0x01, 0x02, 0x03, 0x04]
    mc.load_program(program, 0)

    word = mc.read_word(0)
    expected = 0x04030201 # little-endian
    assert_equal expected, word
  end

  def test_pending_requests
    mem = Array.new(4096, 0)
    mc = CodingAdventures::Core::MemoryController.new(mem, 3) # 3-cycle latency

    # Write data directly first.
    mc.write_word(0, 42)

    # Submit an async read request.
    mc.request_read(0, 4, 0)
    assert_equal 1, mc.pending_count

    # Tick 1 and 2: not ready yet.
    result1 = mc.tick
    assert_equal 0, result1.length

    result2 = mc.tick
    assert_equal 0, result2.length

    # Tick 3: ready.
    result3 = mc.tick
    assert_equal 1, result3.length
    assert_equal 0, result3[0].requester_id
  end

  def test_async_write
    mem = Array.new(4096, 0)
    mc = CodingAdventures::Core::MemoryController.new(mem, 2) # 2-cycle latency

    mc.request_write(100, [0xAB, 0xCD], 0)
    assert_equal 1, mc.pending_count

    mc.tick # cycle 1: not done yet
    # Value not yet written
    assert_equal 0, mc.read_word(100)

    mc.tick # cycle 2: done
    assert_equal 0, mc.pending_count
  end

  def test_bounds_check
    mem = Array.new(64, 0)
    mc = CodingAdventures::Core::MemoryController.new(mem, 1)

    # These should not raise.
    assert_equal 0, mc.read_word(1000)
    mc.write_word(1000, 42)
    mc.load_program([1, 2, 3, 4], 1000)
  end

  def test_memory_size
    mem = Array.new(1024, 0)
    mc = CodingAdventures::Core::MemoryController.new(mem, 1)
    assert_equal 1024, mc.memory_size
  end
end
