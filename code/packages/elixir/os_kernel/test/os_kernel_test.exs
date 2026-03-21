defmodule CodingAdventures.OsKernelTest do
  use ExUnit.Case, async: true
  alias CodingAdventures.OsKernel
  alias CodingAdventures.OsKernel.{Scheduler, MemoryManager, OSKernel, Process}

  test "Scheduler round-robin selects next ready" do
    pt = [
      %Process{pid: 0, state: :ready, saved_registers: [], saved_pc: 0, stack_pointer: 0, memory_base: 0, memory_size: 0, name: "idle"},
      %Process{pid: 1, state: :ready, saved_registers: [], saved_pc: 0, stack_pointer: 0, memory_base: 0, memory_size: 0, name: "hw"}
    ]
    sched = %Scheduler{process_table: pt, current: 0}
    assert Scheduler.schedule(sched) == 1
  end

  test "Scheduler falls back to idle" do
    pt = [
      %Process{pid: 0, state: :ready, saved_registers: [], saved_pc: 0, stack_pointer: 0, memory_base: 0, memory_size: 0, name: "idle"},
      %Process{pid: 1, state: :terminated, saved_registers: [], saved_pc: 0, stack_pointer: 0, memory_base: 0, memory_size: 0, name: "hw"}
    ]
    sched = %Scheduler{process_table: pt, current: 1}
    assert Scheduler.schedule(sched) == 0
  end

  test "Kernel boot creates processes" do
    k = OSKernel.new(%{}, nil) |> OSKernel.boot()
    assert OSKernel.process_count(k) == 2
    assert k.booted == true
  end

  test "Kernel is_idle? when all non-idle terminated" do
    k = OSKernel.new(%{}, nil) |> OSKernel.boot()
    pt = Enum.map(k.process_table, fn p ->
      if p.pid != 0, do: %{p | state: :terminated}, else: p
    end)
    k = %{k | process_table: pt}
    assert OSKernel.is_idle?(k)
  end

  test "MemoryManager find_region" do
    mm = MemoryManager.new([%{base: 0x1000, size: 0x1000, name: "test"}])
    assert MemoryManager.find_region(mm, 0x1500) != nil
    assert MemoryManager.find_region(mm, 0x3000) == nil
  end

  test "generate_idle_program produces bytes" do
    idle = OsKernel.generate_idle_program()
    assert length(idle) == 12
  end

  test "generate_hello_world_program includes data" do
    hw = OsKernel.generate_hello_world_program(0x00040000)
    assert length(hw) > 0x100
    data = Enum.slice(hw, 0x100, 12) |> List.to_string()
    assert data == "Hello World\n"
  end
end
