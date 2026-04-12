defmodule Ls00.DocumentManagerTest do
  use ExUnit.Case, async: true

  @moduledoc """
  Tests for DocumentManager: open, change, close, and incremental editing.
  """

  alias Ls00.DocumentManager
  alias Ls00.Types
  alias Ls00.Types.{Position, TextChange}

  test "open and get a document" do
    docs =
      DocumentManager.new()
      |> DocumentManager.open("file:///test.txt", "hello world", 1)

    assert {:ok, doc} = DocumentManager.get(docs, "file:///test.txt")
    assert doc.text == "hello world"
    assert doc.version == 1
  end

  test "get returns :error for non-open file" do
    docs = DocumentManager.new()
    assert :error = DocumentManager.get(docs, "file:///nonexistent.txt")
  end

  test "close removes a document" do
    docs =
      DocumentManager.new()
      |> DocumentManager.open("file:///test.txt", "hello", 1)
      |> DocumentManager.close("file:///test.txt")

    assert :error = DocumentManager.get(docs, "file:///test.txt")
  end

  test "apply_changes with full replacement" do
    docs =
      DocumentManager.new()
      |> DocumentManager.open("file:///test.txt", "hello world", 1)

    {:ok, docs} =
      DocumentManager.apply_changes(docs, "file:///test.txt", [
        %TextChange{range: nil, new_text: "goodbye world"}
      ], 2)

    {:ok, doc} = DocumentManager.get(docs, "file:///test.txt")
    assert doc.text == "goodbye world"
    assert doc.version == 2
  end

  test "apply_changes with incremental change" do
    docs =
      DocumentManager.new()
      |> DocumentManager.open("file:///test.txt", "hello world", 1)

    # Replace "world" (chars 6-11 on line 0) with "Go"
    {:ok, docs} =
      DocumentManager.apply_changes(docs, "file:///test.txt", [
        %TextChange{
          range: %Types.Range{
            start: %Position{line: 0, character: 6},
            end_pos: %Position{line: 0, character: 11}
          },
          new_text: "Go"
        }
      ], 2)

    {:ok, doc} = DocumentManager.get(docs, "file:///test.txt")
    assert doc.text == "hello Go"
  end

  test "apply_changes to non-open document returns error" do
    docs = DocumentManager.new()

    assert {:error, "document not open: file:///notopen.txt"} =
             DocumentManager.apply_changes(docs, "file:///notopen.txt", [
               %TextChange{range: nil, new_text: "x"}
             ], 1)
  end

  test "incremental change with emoji (UTF-16 surrogate pair)" do
    # "A🎸B" -- emoji is 4 UTF-8 bytes, 2 UTF-16 code units
    # Replace "B" (UTF-16 char 3, byte offset 5) with "X"
    docs =
      DocumentManager.new()
      |> DocumentManager.open("file:///test.txt", "A\u{1F3B8}B", 1)

    {:ok, docs} =
      DocumentManager.apply_changes(docs, "file:///test.txt", [
        %TextChange{
          range: %Types.Range{
            start: %Position{line: 0, character: 3},
            end_pos: %Position{line: 0, character: 4}
          },
          new_text: "X"
        }
      ], 2)

    {:ok, doc} = DocumentManager.get(docs, "file:///test.txt")
    assert doc.text == "A\u{1F3B8}X"
  end

  test "incremental multi-change" do
    docs =
      DocumentManager.new()
      |> DocumentManager.open("uri", "hello world", 1)

    # Change "hello" to "hi"
    {:ok, docs} =
      DocumentManager.apply_changes(docs, "uri", [
        %TextChange{
          range: %Types.Range{
            start: %Position{line: 0, character: 0},
            end_pos: %Position{line: 0, character: 5}
          },
          new_text: "hi"
        }
      ], 2)

    {:ok, doc} = DocumentManager.get(docs, "uri")
    assert doc.text == "hi world"
  end
end
