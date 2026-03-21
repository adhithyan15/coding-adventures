defmodule CodingAdventures.InterruptHandler do
  @moduledoc "Interrupt handler: IDT, ISR registry, and interrupt controller."

  defmodule IDT do
    @moduledoc "Interrupt Descriptor Table -- 256 entries mapping interrupt numbers to ISR addresses."
    defstruct entries: %{}

    def new, do: %__MODULE__{}
    def set_entry(%__MODULE__{entries: e} = idt, number, entry) when number >= 0 and number <= 255, do: %{idt | entries: Map.put(e, number, entry)}
    def get_entry(%__MODULE__{entries: e}, number) when number >= 0 and number <= 255, do: Map.get(e, number, %{isr_address: 0, present: false, privilege_level: 0})
  end

  defmodule ISRRegistry do
    @moduledoc "Maps interrupt numbers to handler functions."
    defstruct handlers: %{}
    def new, do: %__MODULE__{}
    def register(%__MODULE__{handlers: h} = r, number, handler), do: %{r | handlers: Map.put(h, number, handler)}
    def dispatch(%__MODULE__{handlers: h}, number, frame, kernel) do
      case Map.get(h, number) do
        nil -> raise "no ISR handler registered for interrupt"
        handler -> handler.(frame, kernel)
      end
    end
    def has_handler?(%__MODULE__{handlers: h}, number), do: Map.has_key?(h, number)
  end

  defmodule Controller do
    @moduledoc "Interrupt controller managing pending queue, masking, and dispatch."
    defstruct idt: IDT.new(), registry: ISRRegistry.new(), pending: [], mask_register: 0, enabled: true

    def new, do: %__MODULE__{}

    def raise_interrupt(%__MODULE__{pending: p} = c, number) do
      if number in p, do: c, else: %{c | pending: Enum.sort([number | p])}
    end

    def has_pending?(%__MODULE__{enabled: false}), do: false
    def has_pending?(%__MODULE__{pending: p} = c), do: Enum.any?(p, &(!masked?(c, &1)))

    def next_pending(%__MODULE__{enabled: false}), do: -1
    def next_pending(%__MODULE__{pending: p} = c), do: Enum.find(p, -1, &(!masked?(c, &1)))

    def acknowledge(%__MODULE__{pending: p} = c, number), do: %{c | pending: Enum.filter(p, &(&1 != number))}

    def set_mask(%__MODULE__{} = c, number, true) when number >= 0 and number <= 31 do
      import Bitwise
      %{c | mask_register: c.mask_register ||| (1 <<< number)}
    end
    def set_mask(%__MODULE__{} = c, number, false) when number >= 0 and number <= 31 do
      import Bitwise
      %{c | mask_register: c.mask_register &&& ~~~(1 <<< number)}
    end
    def set_mask(c, _, _), do: c

    def masked?(%__MODULE__{mask_register: m}, number) when number >= 0 and number <= 31 do
      import Bitwise
      (m &&& (1 <<< number)) != 0
    end
    def masked?(_, _), do: false

    def enable(c), do: %{c | enabled: true}
    def disable(c), do: %{c | enabled: false}
    def pending_count(%__MODULE__{pending: p}), do: length(p)
    def clear_all(c), do: %{c | pending: []}
  end

  defmodule Frame do
    @moduledoc "Interrupt frame -- saved CPU context."
    defstruct [:pc, :registers, :mstatus, :mcause]

    def save_context(registers, pc, mstatus, mcause) do
      %__MODULE__{pc: pc, registers: registers, mstatus: mstatus, mcause: mcause}
    end

    def restore_context(%__MODULE__{} = frame) do
      {frame.registers, frame.pc, frame.mstatus}
    end
  end
end
