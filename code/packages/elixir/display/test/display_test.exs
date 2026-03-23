defmodule CodingAdventures.DisplayTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Display
  alias CodingAdventures.Display.{Config, Snapshot}

  defp make_driver(cols \\ 40, rows \\ 10) do
    Display.new(%Config{columns: cols, rows: rows})
  end

  test "starts blank" do
    d = make_driver()
    snap = Display.snapshot(d)
    assert Enum.all?(snap.lines, &(&1 == ""))
  end

  test "puts writes text" do
    d = make_driver() |> Display.puts("Hello")
    snap = Display.snapshot(d)
    assert Snapshot.line_at(snap, 0) == "Hello"
  end

  test "newline advances row" do
    d = make_driver() |> Display.puts("AB\nCD")
    snap = Display.snapshot(d)
    assert Snapshot.line_at(snap, 0) == "AB"
    assert Snapshot.line_at(snap, 1) == "CD"
  end

  test "wraps at end of line" do
    d = make_driver(10, 5) |> Display.puts("1234567890X")
    snap = Display.snapshot(d)
    assert Snapshot.line_at(snap, 0) == "1234567890"
    assert Snapshot.line_at(snap, 1) == "X"
  end

  test "scrolls when past last row" do
    d = make_driver(10, 3) |> Display.puts("Line1\nLine2\nLine3\nLine4")
    snap = Display.snapshot(d)
    assert Snapshot.line_at(snap, 0) == "Line2"
    assert Snapshot.line_at(snap, 1) == "Line3"
    assert Snapshot.line_at(snap, 2) == "Line4"
  end

  test "clear resets screen" do
    d = make_driver() |> Display.puts("text") |> Display.clear()
    snap = Display.snapshot(d)
    assert Snapshot.line_at(snap, 0) == ""
    assert snap.cursor == {0, 0}
  end

  test "contains finds text" do
    d = make_driver() |> Display.puts("Hello World")
    snap = Display.snapshot(d)
    assert Snapshot.contains(snap, "World")
    refute Snapshot.contains(snap, "Nope")
  end

  test "set_cursor clamps bounds" do
    d = make_driver(10, 5) |> Display.set_cursor(-1, -1)
    assert d.cursor == {0, 0}
    d = make_driver(10, 5) |> Display.set_cursor(100, 100)
    assert d.cursor == {4, 9}
  end

  test "get_cell reads character" do
    d = make_driver() |> Display.puts("A")
    {ch, attr} = Display.get_cell(d, 0, 0)
    assert ch == ?A
    assert attr == 0x07
  end
end
