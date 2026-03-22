defmodule CodingAdventures.SystemBoardTest do
  use ExUnit.Case, async: true
  alias CodingAdventures.SystemBoard
  alias CodingAdventures.SystemBoard.BootTrace
  alias CodingAdventures.Display.Snapshot

  test "powers on successfully" do
    board = SystemBoard.new() |> SystemBoard.power_on()
    assert board.powered == true
    assert board.current_phase == :kernel_init
  end

  test "boots to hello-world and displays output" do
    board = SystemBoard.new() |> SystemBoard.power_on() |> SystemBoard.run(100_000)
    snap = SystemBoard.display_snapshot(board)
    assert snap != nil
    assert Snapshot.contains(snap, "Hello World")
  end

  test "reaches idle phase after hello-world" do
    board = SystemBoard.new() |> SystemBoard.power_on() |> SystemBoard.run(100_000)
    assert SystemBoard.idle?(board)
    assert board.current_phase == :idle
  end

  test "boot trace records all phases" do
    board = SystemBoard.new() |> SystemBoard.power_on() |> SystemBoard.run(100_000)
    phases = BootTrace.phases(board.trace)
    assert :power_on in phases
    assert :bios in phases
    assert :bootloader in phases
    assert :kernel_init in phases
    assert :user_program in phases
    assert :idle in phases
  end

  test "power_on is idempotent" do
    board = SystemBoard.new() |> SystemBoard.power_on() |> SystemBoard.power_on()
    assert board.powered == true
  end

  test "cycle count increases during execution" do
    board = SystemBoard.new() |> SystemBoard.power_on() |> SystemBoard.run(100_000)
    assert board.cycle > 0
  end
end
