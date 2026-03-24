defmodule CodingAdventures.ClockEdge do
  @moduledoc """
  One transition of a clock signal.
  """

  @enforce_keys [:cycle, :value, :is_rising, :is_falling]
  defstruct [:cycle, :value, :is_rising, :is_falling]

  @type t :: %__MODULE__{
          cycle: non_neg_integer(),
          value: 0 | 1,
          is_rising: boolean(),
          is_falling: boolean()
        }
end

defmodule CodingAdventures.Clock do
  @moduledoc """
  System clock generator.

  The clock starts low (`0`) and toggles on every tick. Rising edges start new
  cycles, while falling edges complete them.
  """

  alias CodingAdventures.ClockEdge

  @default_frequency_hz 1_000_000

  @enforce_keys [:frequency_hz]
  defstruct frequency_hz: @default_frequency_hz,
            cycle: 0,
            value: 0,
            total_ticks: 0,
            listeners: []

  @type listener :: (ClockEdge.t() -> any())

  @type t :: %__MODULE__{
          frequency_hz: pos_integer(),
          cycle: non_neg_integer(),
          value: 0 | 1,
          total_ticks: non_neg_integer(),
          listeners: [listener()]
        }

  @spec new(pos_integer()) :: t()
  def new(frequency_hz \\ @default_frequency_hz) when is_integer(frequency_hz) and frequency_hz > 0 do
    %__MODULE__{frequency_hz: frequency_hz}
  end

  @spec tick(t()) :: {t(), ClockEdge.t()}
  def tick(%__MODULE__{} = clock) do
    old_value = clock.value
    new_value = 1 - old_value
    is_rising = old_value == 0 and new_value == 1
    is_falling = old_value == 1 and new_value == 0
    new_cycle = if is_rising, do: clock.cycle + 1, else: clock.cycle

    edge = %ClockEdge{
      cycle: new_cycle,
      value: new_value,
      is_rising: is_rising,
      is_falling: is_falling
    }

    Enum.each(clock.listeners, fn listener -> listener.(edge) end)

    updated = %__MODULE__{
      clock
      | value: new_value,
        cycle: new_cycle,
        total_ticks: clock.total_ticks + 1
    }

    {updated, edge}
  end

  @spec full_cycle(t()) :: {t(), ClockEdge.t(), ClockEdge.t()}
  def full_cycle(%__MODULE__{} = clock) do
    {clock, rising} = tick(clock)
    {clock, falling} = tick(clock)
    {clock, rising, falling}
  end

  @spec run(t(), non_neg_integer()) :: {t(), [ClockEdge.t()]}
  def run(%__MODULE__{} = clock, cycles) when is_integer(cycles) and cycles >= 0 do
    Enum.reduce(1..cycles, {clock, []}, fn _, {acc_clock, acc_edges} ->
      {next_clock, rising, falling} = full_cycle(acc_clock)
      {next_clock, acc_edges ++ [rising, falling]}
    end)
  end

  def run(%__MODULE__{} = clock, 0), do: {clock, []}

  @spec register_listener(t(), listener()) :: t()
  def register_listener(%__MODULE__{} = clock, listener) when is_function(listener, 1) do
    %{clock | listeners: clock.listeners ++ [listener]}
  end

  @spec unregister_listener(t(), non_neg_integer()) :: {:ok, t()} | {:error, String.t()}
  def unregister_listener(%__MODULE__{} = clock, index)
      when is_integer(index) and index >= 0 do
    if index < length(clock.listeners) do
      listeners = List.delete_at(clock.listeners, index)
      {:ok, %{clock | listeners: listeners}}
    else
      {:error, "listener index #{index} out of range"}
    end
  end

  @spec listener_count(t()) :: non_neg_integer()
  def listener_count(%__MODULE__{} = clock), do: length(clock.listeners)

  @spec reset(t()) :: t()
  def reset(%__MODULE__{} = clock) do
    %{clock | cycle: 0, value: 0, total_ticks: 0}
  end

  @spec period_ns(t()) :: float()
  def period_ns(%__MODULE__{} = clock), do: 1.0e9 / clock.frequency_hz
end

defmodule CodingAdventures.ClockDivider do
  @moduledoc """
  Divides a source clock by an integer factor.
  """

  alias CodingAdventures.Clock
  alias CodingAdventures.ClockEdge

  @enforce_keys [:source, :divisor, :output]
  defstruct [:source, :divisor, :output, counter: 0]

  @type t :: %__MODULE__{
          source: Clock.t(),
          divisor: pos_integer(),
          output: Clock.t(),
          counter: non_neg_integer()
        }

  @spec new(Clock.t(), pos_integer()) :: {:ok, t()} | {:error, String.t()}
  def new(%Clock{} = source, divisor) when is_integer(divisor) and divisor >= 2 do
    {:ok,
     %__MODULE__{
       source: source,
       divisor: divisor,
       output: Clock.new(div(source.frequency_hz, divisor))
     }}
  end

  def new(%Clock{}, divisor), do: {:error, "divisor must be >= 2, got #{divisor}"}

  @spec on_edge(t(), ClockEdge.t()) :: t()
  def on_edge(%__MODULE__{} = divider, %ClockEdge{is_rising: true}) do
    counter = divider.counter + 1

    if counter >= divider.divisor do
      {output, _} = Clock.tick(divider.output)
      {output, _} = Clock.tick(output)
      %{divider | counter: 0, output: output}
    else
      %{divider | counter: counter}
    end
  end

  def on_edge(%__MODULE__{} = divider, %ClockEdge{}), do: divider
end

defmodule CodingAdventures.MultiPhaseClock do
  @moduledoc """
  Generates multiple non-overlapping phases from a source clock.
  """

  alias CodingAdventures.Clock
  alias CodingAdventures.ClockEdge

  @enforce_keys [:source, :phases]
  defstruct [:source, :phases, active_phase: 0, phase_values: []]

  @type t :: %__MODULE__{
          source: Clock.t(),
          phases: pos_integer(),
          active_phase: non_neg_integer(),
          phase_values: [0 | 1]
        }

  @spec new(Clock.t(), pos_integer()) :: {:ok, t()} | {:error, String.t()}
  def new(%Clock{} = source, phases) when is_integer(phases) and phases >= 2 do
    {:ok, %__MODULE__{source: source, phases: phases, phase_values: List.duplicate(0, phases)}}
  end

  def new(%Clock{}, phases), do: {:error, "phases must be >= 2, got #{phases}"}

  @spec get_phase(t(), non_neg_integer()) :: 0 | 1
  def get_phase(%__MODULE__{} = clock, index), do: Enum.at(clock.phase_values, index, 0)

  @spec on_edge(t(), ClockEdge.t()) :: t()
  def on_edge(%__MODULE__{} = clock, %ClockEdge{is_rising: true}) do
    phase_values =
      List.duplicate(0, clock.phases)
      |> List.replace_at(clock.active_phase, 1)

    %{clock | phase_values: phase_values, active_phase: rem(clock.active_phase + 1, clock.phases)}
  end

  def on_edge(%__MODULE__{} = clock, %ClockEdge{}), do: clock
end
