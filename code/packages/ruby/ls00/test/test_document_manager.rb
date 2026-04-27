# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# DocumentManager Tests
# ================================================================
#
# Tests for the DocumentManager which tracks open files, applies
# incremental changes, and handles open/close lifecycle.
#
# ================================================================

class TestDocumentManager < Minitest::Test
  def test_open
    dm = CodingAdventures::Ls00::DocumentManager.new
    dm.open("file:///test.txt", "hello world", 1)

    doc = dm.get("file:///test.txt")
    refute_nil doc
    assert_equal "hello world", doc.text
    assert_equal 1, doc.version
  end

  def test_get_missing
    dm = CodingAdventures::Ls00::DocumentManager.new
    doc = dm.get("file:///nonexistent.txt")
    assert_nil doc
  end

  def test_close
    dm = CodingAdventures::Ls00::DocumentManager.new
    dm.open("file:///test.txt", "hello", 1)
    dm.close("file:///test.txt")

    doc = dm.get("file:///test.txt")
    assert_nil doc
  end

  def test_apply_changes_full_replacement
    dm = CodingAdventures::Ls00::DocumentManager.new
    dm.open("file:///test.txt", "hello world", 1)

    dm.apply_changes("file:///test.txt", [
      CodingAdventures::Ls00::TextChange.new(range: nil, new_text: "goodbye world")
    ], 2)

    doc = dm.get("file:///test.txt")
    assert_equal "goodbye world", doc.text
    assert_equal 2, doc.version
  end

  def test_apply_changes_incremental
    dm = CodingAdventures::Ls00::DocumentManager.new
    dm.open("file:///test.txt", "hello world", 1)

    # Replace "world" with "Go" -- range covers chars 6-11 on line 0
    dm.apply_changes("file:///test.txt", [
      CodingAdventures::Ls00::TextChange.new(
        range: CodingAdventures::Ls00::LspRange.new(
          start: CodingAdventures::Ls00::Position.new(line: 0, character: 6),
          end_pos: CodingAdventures::Ls00::Position.new(line: 0, character: 11)
        ),
        new_text: "Go"
      )
    ], 2)

    doc = dm.get("file:///test.txt")
    assert_equal "hello Go", doc.text
  end

  def test_apply_changes_not_open
    dm = CodingAdventures::Ls00::DocumentManager.new
    assert_raises(RuntimeError) do
      dm.apply_changes("file:///notopen.txt", [
        CodingAdventures::Ls00::TextChange.new(range: nil, new_text: "x")
      ], 1)
    end
  end

  # Incremental change with emoji.
  # "A\u{1F3B8}B" -- emoji is 4 UTF-8 bytes, 2 UTF-16 code units
  # Replace "B" (UTF-16 char 3, byte offset 5) with "X"
  def test_incremental_with_emoji
    dm = CodingAdventures::Ls00::DocumentManager.new
    dm.open("file:///test.txt", "A\u{1F3B8}B", 1)

    dm.apply_changes("file:///test.txt", [
      CodingAdventures::Ls00::TextChange.new(
        range: CodingAdventures::Ls00::LspRange.new(
          start: CodingAdventures::Ls00::Position.new(line: 0, character: 3),
          end_pos: CodingAdventures::Ls00::Position.new(line: 0, character: 4)
        ),
        new_text: "X"
      )
    ], 2)

    doc = dm.get("file:///test.txt")
    assert_equal "A\u{1F3B8}X", doc.text
  end

  # Two incremental changes applied in sequence.
  def test_incremental_multi_change
    dm = CodingAdventures::Ls00::DocumentManager.new
    dm.open("uri", "hello world", 1)

    # Change "hello" to "hi"
    dm.apply_changes("uri", [
      CodingAdventures::Ls00::TextChange.new(
        range: CodingAdventures::Ls00::LspRange.new(
          start: CodingAdventures::Ls00::Position.new(line: 0, character: 0),
          end_pos: CodingAdventures::Ls00::Position.new(line: 0, character: 5)
        ),
        new_text: "hi"
      )
    ], 2)

    doc = dm.get("uri")
    assert_equal "hi world", doc.text
  end
end
