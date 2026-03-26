# frozen_string_literal: true

require_relative "test_helper"

# Tests for the Pipe class -- a circular buffer byte stream.
#
# We test:
#   1. Basic write/read round-trip
#   2. FIFO ordering (data comes out in the order it went in)
#   3. Circular wrapping (write past the end, wrap to the beginning)
#   4. Partial reads (read fewer bytes than available)
#   5. Partial writes (write more bytes than space allows)
#   6. EOF detection (all writers closed + buffer empty)
#   7. BrokenPipeError (all readers closed, attempt to write)
#   8. Capacity tracking (available/space methods)
#   9. Empty and full state detection
class TestPipe < Minitest::Test
  # -- Basic write/read --

  def test_write_and_read_bytes
    pipe = CodingAdventures::Ipc::Pipe.new
    data = [72, 101, 108, 108, 111] # "Hello"

    bytes_written = pipe.write(data)
    assert_equal 5, bytes_written

    result = pipe.read(5)
    assert_equal data, result
  end

  def test_fifo_ordering
    # Data must come out in the same order it went in.
    # Write "abc" then "def", read 6 bytes, expect "abcdef".
    pipe = CodingAdventures::Ipc::Pipe.new
    pipe.write([97, 98, 99])  # "abc"
    pipe.write([100, 101, 102]) # "def"

    result = pipe.read(6)
    assert_equal [97, 98, 99, 100, 101, 102], result
  end

  # -- Circular wrapping --

  def test_circular_buffer_wrapping
    # Use a small capacity to make wrapping easy to trigger.
    pipe = CodingAdventures::Ipc::Pipe.new(capacity: 8)

    # Write 6 bytes to fill most of the buffer.
    pipe.write([1, 2, 3, 4, 5, 6])

    # Read 4 bytes to free up space at the front.
    result = pipe.read(4)
    assert_equal [1, 2, 3, 4], result

    # Now write 5 more bytes -- this should wrap around.
    # Buffer state before: [_, _, _, _, 5, 6, _, _] (read_pos=4, write_pos=6)
    # After writing [7,8,9,10,11]: wraps around the end.
    pipe.write([7, 8, 9, 10, 11])

    # Read everything back.
    result = pipe.read(7)
    assert_equal [5, 6, 7, 8, 9, 10, 11], result
  end

  # -- Partial reads and writes --

  def test_partial_read_returns_available_bytes
    pipe = CodingAdventures::Ipc::Pipe.new
    pipe.write([1, 2, 3])

    # Ask for 10 bytes but only 3 are available.
    result = pipe.read(10)
    assert_equal [1, 2, 3], result
  end

  def test_read_from_empty_pipe_returns_empty
    pipe = CodingAdventures::Ipc::Pipe.new
    result = pipe.read(5)
    assert_equal [], result
  end

  def test_partial_write_when_buffer_is_nearly_full
    pipe = CodingAdventures::Ipc::Pipe.new(capacity: 4)

    # Write 3 bytes (fills most of a 4-byte buffer).
    pipe.write([1, 2, 3])

    # Try to write 3 more, but only 1 slot is available.
    bytes_written = pipe.write([4, 5, 6])
    assert_equal 1, bytes_written

    result = pipe.read(4)
    assert_equal [1, 2, 3, 4], result
  end

  # -- EOF detection --

  def test_eof_when_all_writers_closed_and_buffer_empty
    pipe = CodingAdventures::Ipc::Pipe.new
    pipe.close_write

    assert pipe.eof?
    result = pipe.read(5)
    assert_equal [], result
  end

  def test_not_eof_when_writers_closed_but_buffer_has_data
    pipe = CodingAdventures::Ipc::Pipe.new
    pipe.write([1, 2, 3])
    pipe.close_write

    # Not EOF yet -- there's still data to read.
    refute pipe.eof?

    # Read the remaining data.
    result = pipe.read(3)
    assert_equal [1, 2, 3], result

    # Now it's EOF.
    assert pipe.eof?
  end

  def test_not_eof_when_writers_still_open
    pipe = CodingAdventures::Ipc::Pipe.new
    refute pipe.eof?
  end

  # -- Broken pipe --

  def test_broken_pipe_when_no_readers
    pipe = CodingAdventures::Ipc::Pipe.new
    pipe.close_read

    assert_raises(CodingAdventures::Ipc::BrokenPipeError) do
      pipe.write([1, 2, 3])
    end
  end

  # -- Capacity tracking --

  def test_available_and_space
    pipe = CodingAdventures::Ipc::Pipe.new(capacity: 10)

    assert_equal 0, pipe.available
    assert_equal 10, pipe.space

    pipe.write([1, 2, 3])
    assert_equal 3, pipe.available
    assert_equal 7, pipe.space

    pipe.read(2)
    assert_equal 1, pipe.available
    assert_equal 9, pipe.space
  end

  def test_empty_and_full
    pipe = CodingAdventures::Ipc::Pipe.new(capacity: 3)

    assert pipe.empty?
    refute pipe.full?

    pipe.write([1, 2, 3])
    refute pipe.empty?
    assert pipe.full?
  end

  # -- Reference counting --

  def test_initial_reader_writer_counts
    pipe = CodingAdventures::Ipc::Pipe.new
    assert_equal 1, pipe.reader_count
    assert_equal 1, pipe.writer_count
  end

  def test_close_read_decrements_reader_count
    pipe = CodingAdventures::Ipc::Pipe.new
    pipe.close_read
    assert_equal 0, pipe.reader_count
  end

  def test_close_write_decrements_writer_count
    pipe = CodingAdventures::Ipc::Pipe.new
    pipe.close_write
    assert_equal 0, pipe.writer_count
  end

  def test_close_read_does_not_go_negative
    pipe = CodingAdventures::Ipc::Pipe.new
    pipe.close_read
    pipe.close_read
    assert_equal 0, pipe.reader_count
  end

  def test_close_write_does_not_go_negative
    pipe = CodingAdventures::Ipc::Pipe.new
    pipe.close_write
    pipe.close_write
    assert_equal 0, pipe.writer_count
  end

  # -- Byte masking --

  def test_write_masks_bytes_to_0_255
    pipe = CodingAdventures::Ipc::Pipe.new
    pipe.write([256, 257, -1])
    result = pipe.read(3)
    # 256 & 0xFF = 0, 257 & 0xFF = 1, -1 & 0xFF = 255
    assert_equal [0, 1, 255], result
  end

  # -- Default capacity --

  def test_default_capacity_is_4096
    pipe = CodingAdventures::Ipc::Pipe.new
    assert_equal 4096, pipe.capacity
  end

  def test_custom_capacity
    pipe = CodingAdventures::Ipc::Pipe.new(capacity: 128)
    assert_equal 128, pipe.capacity
  end
end
