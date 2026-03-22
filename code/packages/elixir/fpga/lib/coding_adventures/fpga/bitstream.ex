defmodule CodingAdventures.FPGA.Bitstream do
  @moduledoc """
  Bitstream — FPGA configuration data parser.

  ## What is a Bitstream?

  A bitstream is the configuration file that programs an FPGA. It contains
  all the information needed to configure every LUT, flip-flop, routing
  switch, I/O block, and Block RAM in the device. When you "synthesize"
  and "place & route" a hardware design (written in VHDL or Verilog), the
  tool chain produces a bitstream file.

  In real FPGAs, bitstreams are binary files with vendor-specific formats
  (e.g., Xilinx .bit files, Intel .sof files). They are typically loaded
  into the FPGA at power-up from an external flash memory chip.

  ## Our Format

  For this educational implementation, we use plain Elixir maps as our
  "bitstream" format. The map has this structure:

      %{
        "clbs" => %{
          "0_0" => %{
            "slice_0" => %{
              "lut_a" => [0, 0, 0, 1, ...],
              "lut_b" => [0, 1, 1, 0, ...],
              "use_ff_a" => false,
              "use_ff_b" => true,
              "carry_enable" => false
            },
            "slice_1" => %{...}
          },
          ...
        },
        "routing" => %{
          "0_0" => %{"out_0" => "in_2", ...},
          ...
        },
        "io" => %{
          "pin_0" => %{"direction" => "input"},
          "pin_1" => %{"direction" => "output"},
          ...
        }
      }

  This map-based format is easy to construct and inspect, making it
  ideal for learning and testing.
  """

  defstruct [:clb_configs, :routing_configs, :io_configs]

  @type t :: %__MODULE__{
          clb_configs: %{String.t() => map()},
          routing_configs: %{String.t() => map()},
          io_configs: %{String.t() => map()}
        }

  @doc """
  Parses a bitstream from a plain Elixir map.

  The map should have top-level keys "clbs", "routing", and "io".
  Missing keys default to empty maps.

  ## Examples

      iex> config = %{
      ...>   "clbs" => %{"0_0" => %{"slice_0" => %{"lut_a" => [0,0,0,1]}}},
      ...>   "routing" => %{},
      ...>   "io" => %{}
      ...> }
      iex> bs = CodingAdventures.FPGA.Bitstream.from_map(config)
      iex> Map.has_key?(bs.clb_configs, "0_0")
      true
  """
  @spec from_map(map()) :: t()
  def from_map(config) when is_map(config) do
    %__MODULE__{
      clb_configs: Map.get(config, "clbs", %{}),
      routing_configs: Map.get(config, "routing", %{}),
      io_configs: Map.get(config, "io", %{})
    }
  end

  @doc """
  Returns the CLB configuration for the given position key (e.g., "0_0").

  Returns nil if no configuration exists for that position.

  ## Examples

      iex> bs = CodingAdventures.FPGA.Bitstream.from_map(%{"clbs" => %{"0_0" => %{"slice_0" => %{}}}})
      iex> CodingAdventures.FPGA.Bitstream.clb_config(bs, "0_0")
      %{"slice_0" => %{}}
  """
  @spec clb_config(t(), String.t()) :: map() | nil
  def clb_config(%__MODULE__{clb_configs: configs}, key) do
    Map.get(configs, key)
  end

  @doc """
  Returns the routing configuration for the given position key.

  ## Examples

      iex> bs = CodingAdventures.FPGA.Bitstream.from_map(%{"routing" => %{"0_0" => %{"out_0" => "in_1"}}})
      iex> CodingAdventures.FPGA.Bitstream.routing_config(bs, "0_0")
      %{"out_0" => "in_1"}
  """
  @spec routing_config(t(), String.t()) :: map() | nil
  def routing_config(%__MODULE__{routing_configs: configs}, key) do
    Map.get(configs, key)
  end

  @doc """
  Returns the I/O configuration for the given pin name.

  ## Examples

      iex> bs = CodingAdventures.FPGA.Bitstream.from_map(%{"io" => %{"pin_0" => %{"direction" => "input"}}})
      iex> CodingAdventures.FPGA.Bitstream.io_config(bs, "pin_0")
      %{"direction" => "input"}
  """
  @spec io_config(t(), String.t()) :: map() | nil
  def io_config(%__MODULE__{io_configs: configs}, pin_name) do
    Map.get(configs, pin_name)
  end
end
