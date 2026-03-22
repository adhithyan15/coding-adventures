defmodule CodingAdventures.FPGA.SwitchMatrix do
  @moduledoc """
  SwitchMatrix — programmable routing crossbar in an FPGA.

  ## What is a Switch Matrix?

  A switch matrix (also called a routing switch box) is the component
  that connects CLBs, I/O blocks, and other resources together. It sits
  at the intersection of horizontal and vertical routing channels.

  In a real FPGA, the routing network consumes 60-80% of the chip area
  and contributes significantly to signal delay. The switch matrix
  contains programmable pass transistors that can be turned on or off
  to create connections between wires.

  ## Switch Matrix Architecture

  A switch matrix has N input ports and M output ports. Each output
  port can be connected to at most one input port (to avoid driver
  conflicts). The connections are defined by a routing map.

      Input 0 ──┐
      Input 1 ──┤──── Switch ──── Output 0
      Input 2 ──┤     Matrix ──── Output 1
      Input 3 ──┘            ──── Output 2

  ## Configuration

  The routing map is a map from output port names to input port names:

      %{"out_0" => "in_2", "out_1" => "in_0"}

  This means output 0 is connected to input 2, and output 1 is
  connected to input 0. Unconnected outputs produce nil (high-Z).

  ## Why This Matters

  The switch matrix is what makes an FPGA "field-programmable" — by
  changing the routing configuration, you change how the logic blocks
  are connected, which changes the overall circuit behavior. Two
  identical FPGAs with the same logic but different routing implement
  completely different designs.
  """

  defstruct [:num_inputs, :num_outputs, :connections, :input_names, :output_names]

  @type t :: %__MODULE__{
          num_inputs: pos_integer(),
          num_outputs: pos_integer(),
          connections: %{String.t() => String.t()},
          input_names: [String.t()],
          output_names: [String.t()]
        }

  @doc """
  Creates a new switch matrix with the given number of input and output ports.

  Port names are automatically generated as "in_0", "in_1", ... and
  "out_0", "out_1", ...

  ## Examples

      iex> sm = CodingAdventures.FPGA.SwitchMatrix.new(4, 4)
      iex> sm.num_inputs
      4
  """
  @spec new(pos_integer(), pos_integer()) :: t()
  def new(num_inputs, num_outputs)
      when is_integer(num_inputs) and num_inputs > 0 and
             is_integer(num_outputs) and num_outputs > 0 do
    %__MODULE__{
      num_inputs: num_inputs,
      num_outputs: num_outputs,
      connections: %{},
      input_names: Enum.map(0..(num_inputs - 1), &"in_#{&1}"),
      output_names: Enum.map(0..(num_outputs - 1), &"out_#{&1}")
    }
  end

  @doc """
  Configures the switch matrix routing.

  Takes a map from output port names to input port names. Each output
  can be connected to at most one input. Multiple outputs can connect
  to the same input (fan-out is allowed).

  Raises ArgumentError if a port name is invalid.

  ## Examples

      iex> sm = CodingAdventures.FPGA.SwitchMatrix.new(4, 4)
      iex> sm = CodingAdventures.FPGA.SwitchMatrix.configure(sm, %{"out_0" => "in_2", "out_1" => "in_0"})
      iex> sm.connections
      %{"out_0" => "in_2", "out_1" => "in_0"}
  """
  @spec configure(t(), %{String.t() => String.t()}) :: t()
  def configure(%__MODULE__{} = sm, connections) when is_map(connections) do
    # Validate all port names
    Enum.each(connections, fn {out_name, in_name} ->
      if out_name not in sm.output_names do
        raise ArgumentError, "invalid output port: #{inspect(out_name)}"
      end

      if in_name not in sm.input_names do
        raise ArgumentError, "invalid input port: #{inspect(in_name)}"
      end
    end)

    %{sm | connections: connections}
  end

  @doc """
  Routes signals through the switch matrix.

  Takes a map of input port names to signal values. Returns a map of
  output port names to signal values. Unconnected outputs have nil values.

  ## Examples

      iex> sm = CodingAdventures.FPGA.SwitchMatrix.new(4, 4)
      iex> sm = CodingAdventures.FPGA.SwitchMatrix.configure(sm, %{"out_0" => "in_2"})
      iex> result = CodingAdventures.FPGA.SwitchMatrix.route(sm, %{"in_0" => 0, "in_1" => 0, "in_2" => 1, "in_3" => 0})
      iex> result["out_0"]
      1
  """
  @spec route(t(), %{String.t() => 0 | 1 | nil}) :: %{String.t() => 0 | 1 | nil}
  def route(%__MODULE__{} = sm, input_signals) when is_map(input_signals) do
    Map.new(sm.output_names, fn out_name ->
      case Map.get(sm.connections, out_name) do
        nil ->
          # Unconnected output — high impedance
          {out_name, nil}

        in_name ->
          # Connected output — pass through the input signal
          {out_name, Map.get(input_signals, in_name)}
      end
    end)
  end
end
