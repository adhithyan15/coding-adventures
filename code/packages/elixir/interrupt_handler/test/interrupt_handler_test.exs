defmodule CodingAdventures.InterruptHandlerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.InterruptHandler.{IDT, ISRRegistry, Controller, Frame}

  test "IDT set/get entry" do
    idt = IDT.new() |> IDT.set_entry(32, %{isr_address: 0x800, present: true, privilege_level: 0})
    entry = IDT.get_entry(idt, 32)
    assert entry.isr_address == 0x800
    assert entry.present == true
  end

  test "ISR registry dispatches handler" do
    reg = ISRRegistry.new() |> ISRRegistry.register(32, fn _frame, _kernel -> :ok end)
    assert ISRRegistry.has_handler?(reg, 32)
    assert ISRRegistry.dispatch(reg, 32, nil, nil) == :ok
  end

  test "ISR registry raises on missing handler" do
    reg = ISRRegistry.new()
    assert_raise RuntimeError, ~r/no ISR handler/, fn -> ISRRegistry.dispatch(reg, 99, nil, nil) end
  end

  test "Controller raise/acknowledge" do
    c = Controller.new() |> Controller.raise_interrupt(32)
    assert Controller.pending_count(c) == 1
    assert Controller.has_pending?(c)
    assert Controller.next_pending(c) == 32
    c = Controller.acknowledge(c, 32)
    assert Controller.pending_count(c) == 0
  end

  test "Controller no duplicates" do
    c = Controller.new() |> Controller.raise_interrupt(32) |> Controller.raise_interrupt(32)
    assert Controller.pending_count(c) == 1
  end

  test "Controller masking" do
    c = Controller.new() |> Controller.raise_interrupt(5) |> Controller.set_mask(5, true)
    refute Controller.has_pending?(c)
    c = Controller.set_mask(c, 5, false)
    assert Controller.has_pending?(c)
  end

  test "Controller global disable" do
    c = Controller.new() |> Controller.raise_interrupt(32) |> Controller.disable()
    refute Controller.has_pending?(c)
    c = Controller.enable(c)
    assert Controller.has_pending?(c)
  end

  test "Frame save/restore" do
    regs = Enum.to_list(0..31)
    frame = Frame.save_context(regs, 0x1000, 0x08, 11)
    {restored_regs, pc, mstatus} = Frame.restore_context(frame)
    assert pc == 0x1000
    assert mstatus == 0x08
    assert Enum.at(restored_regs, 5) == 5
  end
end
