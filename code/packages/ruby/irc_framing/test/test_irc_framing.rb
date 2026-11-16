# frozen_string_literal: true

# Tests for CodingAdventures::IrcFraming::Framer
#
# Coverage strategy:
#   - CRLF lines extracted correctly
#   - Bare LF lines extracted correctly
#   - Partial lines buffered until complete
#   - Multiple lines in one feed
#   - Overlong lines discarded
#   - buffer_size tracks bytes correctly
#   - reset clears buffer
#   - empty feed is a no-op

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  minimum_coverage 95
end

require "minitest/autorun"
require "coding_adventures/irc_framing"

Lib = CodingAdventures::IrcFraming

class TestFramer < Minitest::Test
  def setup
    @framer = Lib::Framer.new
  end

  # ── Basic extraction ───────────────────────────────────────────────────

  def test_crlf_line_extracted
    @framer.feed("NICK alice\r\n")
    assert_equal ["NICK alice"], @framer.frames
  end

  def test_bare_lf_line_extracted
    @framer.feed("NICK alice\n")
    assert_equal ["NICK alice"], @framer.frames
  end

  def test_frames_empty_when_no_newline
    @framer.feed("NICK ali")
    assert_equal [], @framer.frames
  end

  def test_partial_line_buffered
    @framer.feed("NICK ali")
    @framer.feed("ce\r\n")
    assert_equal ["NICK alice"], @framer.frames
  end

  def test_multiple_lines_in_one_feed
    @framer.feed("NICK alice\r\nUSER alice 0 * :Alice\r\n")
    assert_equal ["NICK alice", "USER alice 0 * :Alice"], @framer.frames
  end

  def test_second_frames_call_is_empty_after_extraction
    @framer.feed("PING irc.test\r\n")
    @framer.frames
    assert_equal [], @framer.frames
  end

  # ── Overlong line disccard ─────────────────────────────────────────────

  def test_overlong_line_discarded
    long = "A" * 511
    @framer.feed(long + "\r\n")
    assert_equal [], @framer.frames
  end

  def test_max_length_line_kept
    # Exactly 510 bytes of content should be kept.
    line = "A" * 510
    @framer.feed(line + "\r\n")
    assert_equal [line], @framer.frames
  end

  # ── buffer_size ────────────────────────────────────────────────────────

  def test_buffer_size_zero_initially
    assert_equal 0, @framer.buffer_size
  end

  def test_buffer_size_reflects_partial_data
    @framer.feed("NICK ali")
    assert_equal 8, @framer.buffer_size
  end

  def test_buffer_size_decreases_after_frames
    @framer.feed("NICK alice\r\n")
    @framer.frames
    assert_equal 0, @framer.buffer_size
  end

  # ── reset ──────────────────────────────────────────────────────────────

  def test_reset_clears_buffer
    @framer.feed("NICK ali")
    @framer.reset
    assert_equal 0, @framer.buffer_size
    assert_equal [], @framer.frames
  end

  # ── Edge cases ─────────────────────────────────────────────────────────

  def test_empty_feed_is_noop
    @framer.feed("")
    assert_equal 0, @framer.buffer_size
  end

  def test_multiple_feeds_then_frames
    @framer.feed("NICK")
    @framer.feed(" alice")
    @framer.feed("\r\n")
    assert_equal ["NICK alice"], @framer.frames
  end

  # -------------------------------------------------------------------------
  # SEC-011: a peer that never sends LF must not grow the buffer.
  #
  # The cap on an unterminated line is 511 bytes: 510 content bytes plus a CR
  # whose LF has not arrived yet (RFC 1459 / RFC 2812 section 2.3: 512 bytes
  # including CRLF).  These tests mirror the Rust crate's SEC-011 tests.
  # -------------------------------------------------------------------------

  SIXTEEN_MIB = 16 * 1024 * 1024

  def test_no_lf_in_chunks_is_bounded
    chunk = "A" * 4096
    worst = 0
    (SIXTEEN_MIB / chunk.bytesize).times do
      @framer.feed(chunk)
      worst = [worst, @framer.buffer_size].max
    end
    assert worst <= 512, "buffer grew to #{worst} bytes with no LF"
    assert_equal [], @framer.frames
  end

  def test_no_lf_in_one_feed_is_bounded
    @framer.feed("A" * SIXTEEN_MIB)
    assert @framer.buffer_size <= 512, "buffer grew to #{@framer.buffer_size}"
    assert_equal [], @framer.frames
  end

  def test_framer_recovers_after_flood
    # The flooded line is dropped whole, up to its LF; the next line frames.
    @framer.feed("A" * 4096)
    @framer.feed("B" * 4096)
    @framer.feed("tail\r\nNICK alice\r\n")
    assert_equal ["NICK alice"], @framer.frames
    assert_equal 0, @framer.buffer_size
  end

  def test_pending_line_at_limit_is_kept
    # 510 content bytes plus a CR is a legal line still waiting for its LF.
    @framer.feed(("A" * 510) + "\r")
    assert_equal 511, @framer.buffer_size
    @framer.feed("\n")
    assert_equal ["A" * 510], @framer.frames
  end

  def test_pending_line_over_limit_is_dropped
    @framer.feed("A" * 512)
    assert_equal 0, @framer.buffer_size
    @framer.feed("\r\nPING x\r\n")
    assert_equal ["PING x"], @framer.frames
  end

  def test_complete_lines_before_flood_survive
    @framer.feed("NICK a\r\n" + ("A" * 2000))
    assert_equal ["NICK a"], @framer.frames
    assert_equal 0, @framer.buffer_size
  end

  def test_reset_clears_discard_state
    # Reset must end the skip-to-LF state, or the next line is swallowed.
    @framer.feed("A" * 600)
    @framer.reset
    @framer.feed("NICK a\r\n")
    assert_equal ["NICK a"], @framer.frames
  end
end
