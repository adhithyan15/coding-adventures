# frozen_string_literal: true

# Pipe -- a unidirectional byte stream between two endpoints.
#
# A pipe is the simplest IPC mechanism. Think of a pneumatic tube in an old
# bank: you stuff a message capsule in one end, and it arrives at the other end.
# The tube only goes one direction, and messages arrive in the order they were
# sent (FIFO).
#
# Under the hood, a pipe is a **circular buffer** -- a fixed-size array where
# the write position wraps around to the beginning when it reaches the end.
# This avoids the need to shift data forward after each read.
#
# === Circular Buffer Mechanics ===
#
# Imagine a buffer of 8 bytes (we use 4096 in practice, but 8 is easier to
# draw):
#
#   Initial state (empty):
#     +-+-+-+-+-+-+-+-+
#     | | | | | | | | |    read_pos = 0, write_pos = 0
#     +-+-+-+-+-+-+-+-+
#      ^R              ^W  (R and W at same position = empty)
#
#   After writing "hello" (5 bytes):
#     +-+-+-+-+-+-+-+-+
#     |h|e|l|l|o| | | |    read_pos = 0, write_pos = 5
#     +-+-+-+-+-+-+-+-+
#      ^R          ^W
#
#   After reading 3 bytes ("hel"):
#     +-+-+-+-+-+-+-+-+
#     | | | |l|o| | | |    read_pos = 3, write_pos = 5
#     +-+-+-+-+-+-+-+-+
#            ^R    ^W
#
#   Wrapping -- write "fghij" (wraps around the end):
#     +-+-+-+-+-+-+-+-+
#     |i|j| |l|o|f|g|h|    read_pos = 3, write_pos = 2
#     +-+-+-+-+-+-+-+-+
#        ^W  ^R             write_pos BEHIND read_pos = wrapped
#
# The key formula for how many bytes are available to read:
#
#   bytes_used = (write_pos - read_pos + capacity) % capacity
#
# And for how many bytes can be written:
#
#   bytes_free = capacity - bytes_used - 1
#
# Why "-1"? We reserve one slot to distinguish "full" from "empty". If
# write_pos == read_pos, the buffer is empty. If write_pos is one slot
# behind read_pos, the buffer is full. Without this convention, both states
# would look the same (write_pos == read_pos).
#
# === EOF and Broken Pipe ===
#
# Two important signals arise from reference counting:
#
#   +---------------------+------------------------------------------+
#   | Condition           | Result                                   |
#   +---------------------+------------------------------------------+
#   | writer_count == 0   | Read returns empty (EOF). No more data   |
#   | AND buffer empty    | will ever arrive -- the pipe is done.    |
#   +---------------------+------------------------------------------+
#   | reader_count == 0   | Write raises BrokenPipeError. There is   |
#   |                     | nobody to read the data, so writing is   |
#   |                     | pointless.                               |
#   +---------------------+------------------------------------------+
#
# In a shell pipeline like `ls | grep foo`, when `ls` finishes and closes its
# write end, `grep` sees EOF on the read end and knows there is no more input.
# Conversely, if `grep` exits early (e.g., `head -1`), `ls` gets SIGPIPE
# (broken pipe) because nobody is reading.

module CodingAdventures
  module Ipc
    # Error raised when writing to a pipe whose read end has been closed.
    # Named after the Unix EPIPE error / SIGPIPE signal.
    class BrokenPipeError < StandardError; end

    # Default pipe buffer size: 4096 bytes, matching one memory page.
    # This is the same default as Linux pipes (though Linux actually uses
    # 16 pages = 65536 bytes since kernel 2.6.11; we use one page for
    # simplicity and to make wrap-around easier to test).
    DEFAULT_PIPE_CAPACITY = 4096

    class Pipe
      attr_reader :capacity, :reader_count, :writer_count

      # Create a new pipe with an empty circular buffer.
      #
      # The pipe starts with one reader and one writer -- the two file
      # descriptors returned by the pipe() system call. Additional readers
      # or writers are created by dup() or fork().
      def initialize(capacity: DEFAULT_PIPE_CAPACITY)
        @capacity = capacity

        # The buffer is one slot larger than the usable capacity. We need
        # the extra slot to distinguish "full" from "empty" -- both would
        # otherwise have write_pos == read_pos.
        @buffer = Array.new(capacity + 1, 0)
        @read_pos = 0
        @write_pos = 0

        # Reference counts for the read and write endpoints.
        @reader_count = 1
        @writer_count = 1
      end

      # Write data (an array of byte values 0-255) into the pipe.
      #
      # Returns the number of bytes actually written. This may be less than
      # data.length if the buffer fills up (partial write). In a real OS,
      # the process would block until space is available; here we return
      # the partial count so the caller can retry.
      #
      # Raises BrokenPipeError if no readers remain -- writing data that
      # nobody will ever read is an error.
      def write(data)
        raise BrokenPipeError, "write to pipe with no readers (EPIPE)" if @reader_count == 0

        bytes_written = 0
        data.each do |byte|
          break if full?

          @buffer[@write_pos] = byte & 0xFF
          @write_pos = (@write_pos + 1) % @buffer.length
          bytes_written += 1
        end
        bytes_written
      end

      # Read up to `count` bytes from the pipe.
      #
      # Returns an array of byte values. The array may be shorter than
      # `count` if fewer bytes are available (partial read). Returns an
      # empty array if the buffer is empty AND all writers have closed
      # (EOF condition).
      def read(count)
        result = []
        count.times do
          break if empty?

          result << @buffer[@read_pos]
          @read_pos = (@read_pos + 1) % @buffer.length
        end
        result
      end

      # Close the read end of the pipe (decrement reader count).
      #
      # When reader_count drops to 0, any subsequent write will raise
      # BrokenPipeError. This mimics the kernel behavior when all file
      # descriptors pointing to the read end are closed.
      def close_read
        @reader_count -= 1 if @reader_count > 0
      end

      # Close the write end of the pipe (decrement writer count).
      #
      # When writer_count drops to 0 AND the buffer is empty, reads return
      # an empty array (EOF). This tells the reader that no more data will
      # ever arrive.
      def close_write
        @writer_count -= 1 if @writer_count > 0
      end

      # Is the buffer empty? (No data available to read.)
      #
      # Empty when read_pos == write_pos. This is why we need the buffer
      # to be capacity+1 in size -- if we allowed the buffer to fill
      # completely, write_pos would equal read_pos and we couldn't tell
      # "full" from "empty."
      def empty?
        @read_pos == @write_pos
      end

      # Is the buffer full? (No space available to write.)
      #
      # Full when the next write position would equal the read position.
      # We leave one slot unused to distinguish full from empty.
      def full?
        (@write_pos + 1) % @buffer.length == @read_pos
      end

      # Number of bytes available to read.
      def available
        (@write_pos - @read_pos + @buffer.length) % @buffer.length
      end

      # Number of bytes that can be written before the buffer is full.
      def space
        @capacity - available
      end

      # Is the pipe at EOF?
      #
      # EOF means: all writers have closed AND the buffer is empty. This is
      # the signal that tells the reader "the pipe is done, no more data
      # will ever come." In a shell pipeline, this is how `grep` knows that
      # `ls` has finished producing output.
      def eof?
        @writer_count == 0 && empty?
      end
    end
  end
end
