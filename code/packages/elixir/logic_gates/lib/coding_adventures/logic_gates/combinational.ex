defmodule CodingAdventures.LogicGates.Combinational do
  @moduledoc """
  Combinational Logic Circuits — complex decision-making from simple gates.

  ## What is a combinational circuit?

  A combinational circuit is a network of logic gates whose output depends
  ONLY on the current inputs — there is no memory, no feedback, no state.
  Given the same inputs, a combinational circuit always produces the same
  outputs. This is in contrast to sequential circuits (like latches and
  flip-flops), which have memory and whose outputs depend on both current
  inputs and previous state.

  ## The Circuits in This Module

  These circuits appear everywhere in digital hardware:

      Multiplexer (MUX)     → selects one of N inputs to pass through
      Demultiplexer (DEMUX) → routes one input to one of N outputs
      Decoder               → activates one of 2^N outputs based on N-bit input
      Encoder               → converts a one-hot input to an N-bit binary code
      Priority Encoder      → like encoder, but handles multiple active inputs
      Tri-State Buffer      → conditionally connects or disconnects a signal

  ## Why These Matter

  A CPU's ALU uses multiplexers to select which operation result to output.
  Memory address decoders use decoders to activate the correct memory row.
  Interrupt controllers use priority encoders to pick the highest-priority
  pending interrupt. Tri-state buffers allow multiple devices to share a
  single data bus without electrical conflicts.

  Every one of these circuits is built from the same AND, OR, and NOT gates
  we defined in the Gates module — just wired together in specific patterns.
  """

  alias CodingAdventures.LogicGates.Gates

  # ===========================================================================
  # MULTIPLEXER (MUX) — "Data Selector"
  # ===========================================================================
  #
  # A multiplexer is like a railway switch: it has multiple input tracks
  # but only one output track. The "select" signal determines which input
  # track connects to the output.
  #
  # In a CPU, multiplexers are everywhere:
  #   - The ALU uses a MUX to select which operation result to output
  #   - The register file uses a MUX to select which register to read
  #   - The program counter uses a MUX to choose between PC+4 and branch target

  @doc """
  2-to-1 Multiplexer — the simplest data selector.

  Takes two data inputs (d0 and d1) and a select signal (sel).
  When sel=0, the output is d0. When sel=1, the output is d1.

  Implementation using AND/OR gates:

      output = (d0 AND NOT(sel)) OR (d1 AND sel)

  Truth table:

      sel │ output
      ────┼───────
       0  │  d0
       1  │  d1

  Think of it as an if/else in hardware:
    if sel == 0 then d0 else d1

  ## Examples

      iex> CodingAdventures.LogicGates.Combinational.mux2(0, 1, 0)
      0
      iex> CodingAdventures.LogicGates.Combinational.mux2(0, 1, 1)
      1
  """
  @spec mux2(0 | 1, 0 | 1, 0 | 1) :: 0 | 1
  def mux2(d0, d1, sel) do
    Gates.validate_bit!(d0, "d0")
    Gates.validate_bit!(d1, "d1")
    Gates.validate_bit!(sel, "sel")

    # When sel=0: NOT(sel)=1, so d0 AND 1 = d0, and d1 AND 0 = 0 → output = d0
    # When sel=1: NOT(sel)=0, so d0 AND 0 = 0, and d1 AND 1 = d1 → output = d1
    not_sel = Gates.not_gate(sel)
    Gates.or_gate(Gates.and_gate(d0, not_sel), Gates.and_gate(d1, sel))
  end

  @doc """
  4-to-1 Multiplexer — selects one of four inputs.

  Takes four data inputs and a 2-bit select signal (as a list [s1, s0]).
  The select bits form a binary number that indexes the input:

      sel = [0, 0] → d0    (binary 00 = decimal 0)
      sel = [0, 1] → d1    (binary 01 = decimal 1)
      sel = [1, 0] → d2    (binary 10 = decimal 2)
      sel = [1, 1] → d3    (binary 11 = decimal 3)

  Built from three 2:1 MUXes in a tree structure:

      d0 ──┐
            MUX (sel[1]) ──┐
      d1 ──┘               │
                            MUX (sel[0]) ── output
      d2 ──┐               │
            MUX (sel[1]) ──┘
      d3 ──┘

  ## Examples

      iex> CodingAdventures.LogicGates.Combinational.mux4(0, 1, 0, 1, [0, 1])
      1
      iex> CodingAdventures.LogicGates.Combinational.mux4(0, 1, 0, 1, [1, 0])
      0
  """
  @spec mux4(0 | 1, 0 | 1, 0 | 1, 0 | 1, [0 | 1]) :: 0 | 1
  def mux4(d0, d1, d2, d3, [s1, s0]) do
    Gates.validate_bit!(d0, "d0")
    Gates.validate_bit!(d1, "d1")
    Gates.validate_bit!(d2, "d2")
    Gates.validate_bit!(d3, "d3")
    Gates.validate_bit!(s1, "s1")
    Gates.validate_bit!(s0, "s0")

    # First level: two 2:1 MUXes select between pairs using the high bit (s1)
    # Second level: one 2:1 MUX selects between the two results using low bit (s0)
    low_pair = mux2(d0, d1, s0)
    high_pair = mux2(d2, d3, s0)
    mux2(low_pair, high_pair, s1)
  end

  def mux4(_d0, _d1, _d2, _d3, sel) do
    raise ArgumentError, "sel must be a list of 2 bits, got #{inspect(sel)}"
  end

  @doc """
  N-to-1 Multiplexer — selects one of N inputs using recursive decomposition.

  Takes a list of N inputs (where N must be a power of 2) and a list of
  select bits. The number of select bits must equal log2(N).

  This is built recursively: an N:1 MUX is two (N/2):1 MUXes feeding
  into a final 2:1 MUX. The recursion bottoms out at the 2:1 MUX.

  This recursive structure mirrors how real hardware MUXes are built —
  as trees of smaller MUXes, which keeps the gate delay logarithmic
  in the number of inputs (log2(N) gate delays) rather than linear.

  ## Examples

      iex> CodingAdventures.LogicGates.Combinational.mux_n([0, 1], [0])
      0
      iex> CodingAdventures.LogicGates.Combinational.mux_n([0, 1], [1])
      1
      iex> CodingAdventures.LogicGates.Combinational.mux_n([0, 0, 1, 0], [1, 0])
      1
  """
  @spec mux_n([0 | 1], [0 | 1]) :: 0 | 1
  def mux_n(inputs, sel) when is_list(inputs) and is_list(sel) do
    n = length(inputs)
    sel_len = length(sel)

    if n < 2 do
      raise ArgumentError, "mux_n requires at least 2 inputs, got #{n}"
    end

    expected_sel = ceil_log2(n)

    if sel_len != expected_sel do
      raise ArgumentError,
            "#{n} inputs require #{expected_sel} select bits, got #{sel_len}"
    end

    Enum.each(inputs, &Gates.validate_bit!(&1, "input"))
    Enum.each(sel, &Gates.validate_bit!(&1, "sel"))

    do_mux_n(inputs, sel)
  end

  # Base case: 2 inputs, 1 select bit
  defp do_mux_n([d0, d1], [sel_bit]) do
    mux2(d0, d1, sel_bit)
  end

  # Recursive case: split inputs in half, use highest select bit for final MUX
  defp do_mux_n(inputs, sel) do
    half = div(length(inputs), 2)
    {low_half, high_half} = Enum.split(inputs, half)
    # The most significant select bit (head) chooses between the two halves
    # The remaining bits (tail) select within each half
    [msb | rest] = sel
    low_result = do_mux_n(low_half, rest)
    high_result = do_mux_n(high_half, rest)
    mux2(low_result, high_result, msb)
  end

  # Integer log2, rounded up — used to compute the number of select bits.
  # We cannot use Bitwise operators in guard clauses, so we use a regular
  # function body with a conditional instead.
  defp ceil_log2(n) when n <= 1, do: 0
  defp ceil_log2(n), do: ceil_log2(n, 0)

  defp ceil_log2(n, acc) do
    if Bitwise.bsl(1, acc) >= n do
      acc
    else
      ceil_log2(n, acc + 1)
    end
  end

  # ===========================================================================
  # DEMULTIPLEXER (DEMUX) — "Data Router"
  # ===========================================================================
  #
  # A demultiplexer is the opposite of a multiplexer: it takes ONE input
  # and routes it to one of N outputs. The select signal determines which
  # output receives the data; all other outputs are 0.
  #
  # Think of it as a mail sorter: one letter comes in, and based on the
  # address (select), it goes into one of many mailboxes.

  @doc """
  1-to-N Demultiplexer — routes one input to one of N outputs.

  Takes a data bit, a list of select bits, and the number of outputs.
  The select bits form a binary address; the data appears on the
  addressed output, and all other outputs are 0.

  n_outputs must be a power of 2 and match 2^(length of sel).

  Returns a list of n_outputs bits.

  ## Examples

      iex> CodingAdventures.LogicGates.Combinational.demux(1, [0], 2)
      [1, 0]
      iex> CodingAdventures.LogicGates.Combinational.demux(1, [1], 2)
      [0, 1]
      iex> CodingAdventures.LogicGates.Combinational.demux(1, [1, 0], 4)
      [0, 0, 1, 0]
  """
  @spec demux(0 | 1, [0 | 1], pos_integer()) :: [0 | 1]
  def demux(data, sel, n_outputs) when is_list(sel) and is_integer(n_outputs) do
    Gates.validate_bit!(data, "data")
    Enum.each(sel, &Gates.validate_bit!(&1, "sel"))

    expected_sel = ceil_log2(n_outputs)

    if length(sel) != expected_sel do
      raise ArgumentError,
            "#{n_outputs} outputs require #{expected_sel} select bits, got #{length(sel)}"
    end

    # For each output position, AND the data with the product of select bits
    # that address that position. Output i is active when the select bits
    # spell out the binary representation of i.
    #
    # Example for 4 outputs with sel = [s1, s0]:
    #   output[0] = data AND NOT(s1) AND NOT(s0)   (address 00)
    #   output[1] = data AND NOT(s1) AND s0         (address 01)
    #   output[2] = data AND s1     AND NOT(s0)     (address 10)
    #   output[3] = data AND s1     AND s0           (address 11)
    Enum.map(0..(n_outputs - 1), fn index ->
      # Check if each select bit matches the corresponding bit of `index`
      match_bits =
        sel
        |> Enum.with_index()
        |> Enum.map(fn {sel_bit, bit_pos} ->
          # Extract the bit at position (sel_len - 1 - bit_pos) from index
          # MSB first: bit_pos 0 is the highest bit
          shift = length(sel) - 1 - bit_pos
          index_bit = Bitwise.band(Bitwise.bsr(index, shift), 1)

          if index_bit == 1 do
            sel_bit
          else
            Gates.not_gate(sel_bit)
          end
        end)

      # AND all match bits together with the data
      Enum.reduce(match_bits, data, &Gates.and_gate/2)
    end)
  end

  # ===========================================================================
  # DECODER — "Address to One-Hot Converter"
  # ===========================================================================
  #
  # A decoder converts an N-bit binary input into a 2^N-bit one-hot output.
  # Exactly one output bit is 1, and its position corresponds to the
  # binary value of the input.
  #
  # Decoders are essential in memory systems: the address decoder in a RAM
  # chip takes the row address bits and activates exactly one word line.
  # In a CPU, the instruction decoder converts the opcode bits into
  # control signals that activate the correct functional unit.

  @doc """
  N-to-2^N Decoder — converts binary input to one-hot output.

  Takes a list of N input bits and produces a list of 2^N output bits
  where exactly one output is 1 (the one whose index matches the
  binary value of the input).

  A decoder is essentially a DEMUX with data permanently set to 1.

  Truth table for 2-to-4 decoder:

      A1  A0 │ Y3  Y2  Y1  Y0
      ───────┼────────────────
       0   0 │  0   0   0   1
       0   1 │  0   0   1   0
       1   0 │  0   1   0   0
       1   1 │  1   0   0   0

  ## Examples

      iex> CodingAdventures.LogicGates.Combinational.decoder([0, 0])
      [1, 0, 0, 0]
      iex> CodingAdventures.LogicGates.Combinational.decoder([1, 0])
      [0, 0, 1, 0]
      iex> CodingAdventures.LogicGates.Combinational.decoder([1, 1])
      [0, 0, 0, 1]
  """
  @spec decoder([0 | 1]) :: [0 | 1]
  def decoder(inputs) when is_list(inputs) do
    if inputs == [] do
      raise ArgumentError, "decoder requires at least 1 input bit"
    end

    Enum.each(inputs, &Gates.validate_bit!(&1, "input"))

    n_outputs = Bitwise.bsl(1, length(inputs))
    # A decoder is a DEMUX with data=1
    demux(1, inputs, n_outputs)
  end

  # ===========================================================================
  # ENCODER — "One-Hot to Binary Converter"
  # ===========================================================================
  #
  # An encoder is the inverse of a decoder: it takes a one-hot input
  # (exactly one bit is 1) and produces the binary index of that bit.
  #
  # Encoders are used in interrupt controllers: when multiple devices
  # can raise interrupts, the encoder converts the one-hot interrupt
  # signals into a binary interrupt number that the CPU can process.

  @doc """
  2^N-to-N Encoder — converts one-hot input to binary output.

  Takes a list of 2^N input bits where EXACTLY one bit is 1, and
  returns the binary representation of that bit's index as a list
  of N bits (MSB first).

  Raises ArgumentError if the input is not one-hot (i.e., if zero
  or more than one bit is set).

  Truth table for 4-to-2 encoder:

      Y3  Y2  Y1  Y0 │ A1  A0
      ────────────────┼───────
       0   0   0   1  │  0   0
       0   0   1   0  │  0   1
       0   1   0   0  │  1   0
       1   0   0   0  │  1   1

  ## Examples

      iex> CodingAdventures.LogicGates.Combinational.encoder([1, 0, 0, 0])
      [0, 0]
      iex> CodingAdventures.LogicGates.Combinational.encoder([0, 0, 1, 0])
      [1, 0]
      iex> CodingAdventures.LogicGates.Combinational.encoder([0, 0, 0, 1])
      [1, 1]
  """
  @spec encoder([0 | 1]) :: [0 | 1]
  def encoder(inputs) when is_list(inputs) do
    n = length(inputs)

    if n < 2 do
      raise ArgumentError, "encoder requires at least 2 inputs, got #{n}"
    end

    Enum.each(inputs, &Gates.validate_bit!(&1, "input"))

    # Verify exactly one bit is set (one-hot)
    active_count = Enum.count(inputs, &(&1 == 1))

    if active_count != 1 do
      raise ArgumentError,
            "encoder requires exactly one active input (one-hot), got #{active_count} active bits"
    end

    # Find the index of the active bit
    active_index = Enum.find_index(inputs, &(&1 == 1))

    # Convert the index to binary, MSB first
    output_bits = ceil_log2(n)

    Enum.map((output_bits - 1)..0//-1, fn bit_pos ->
      Bitwise.band(Bitwise.bsr(active_index, bit_pos), 1)
    end)
  end

  # ===========================================================================
  # PRIORITY ENCODER
  # ===========================================================================
  #
  # A priority encoder is like a regular encoder but handles the case
  # where multiple input bits are set. It returns the index of the
  # HIGHEST-priority (leftmost / most significant) active input.
  #
  # This is exactly how hardware interrupt controllers work: when
  # multiple interrupts fire simultaneously, the priority encoder
  # picks the highest-priority one for the CPU to handle first.

  @doc """
  Priority Encoder — encodes the highest-priority active input.

  Takes a list of N input bits (where index 0 is highest priority)
  and returns `{encoded_bits, valid}` where:
    - encoded_bits is the binary index of the highest-priority active input
    - valid is 1 if any input is active, 0 if all inputs are 0

  Unlike a regular encoder, the priority encoder does NOT require
  one-hot input — multiple bits can be active simultaneously.

  Example with 4 inputs (index 0 = highest priority):

      I0  I1  I2  I3 │ A1  A0  Valid
      ────────────────┼──────────────
       0   0   0   0  │  0   0    0     (no active input)
       0   0   0   1  │  1   1    1     (only I3 active → index 3)
       0   0   1   0  │  1   0    1     (only I2 active → index 2)
       0   0   1   1  │  1   0    1     (I2 wins over I3 → index 2)
       1   0   1   1  │  0   0    1     (I0 wins over all → index 0)

  ## Examples

      iex> CodingAdventures.LogicGates.Combinational.priority_encoder([0, 0, 0, 0])
      {[0, 0], 0}
      iex> CodingAdventures.LogicGates.Combinational.priority_encoder([0, 0, 1, 1])
      {[1, 0], 1}
      iex> CodingAdventures.LogicGates.Combinational.priority_encoder([1, 0, 1, 1])
      {[0, 0], 1}
  """
  @spec priority_encoder([0 | 1]) :: {[0 | 1], 0 | 1}
  def priority_encoder(inputs) when is_list(inputs) do
    n = length(inputs)

    if n < 2 do
      raise ArgumentError, "priority_encoder requires at least 2 inputs, got #{n}"
    end

    Enum.each(inputs, &Gates.validate_bit!(&1, "input"))

    # Check if any input is active
    valid = if Enum.any?(inputs, &(&1 == 1)), do: 1, else: 0

    if valid == 0 do
      # No active input — return all zeros
      output_bits = ceil_log2(n)
      {List.duplicate(0, output_bits), 0}
    else
      # Find the first (highest-priority) active input
      active_index = Enum.find_index(inputs, &(&1 == 1))
      output_bits = ceil_log2(n)

      encoded =
        Enum.map((output_bits - 1)..0//-1, fn bit_pos ->
          Bitwise.band(Bitwise.bsr(active_index, bit_pos), 1)
        end)

      {encoded, 1}
    end
  end

  # ===========================================================================
  # TRI-STATE BUFFER
  # ===========================================================================
  #
  # A tri-state buffer has three possible output states: 0, 1, or
  # "high-impedance" (disconnected). This is crucial for bus architectures
  # where multiple devices share the same wires.
  #
  # Without tri-state buffers, connecting two outputs to the same wire
  # would cause a short circuit if one outputs 0 (connected to ground)
  # and the other outputs 1 (connected to power). The tri-state buffer
  # solves this by allowing a device to completely disconnect from the
  # bus when it's not the one "talking."
  #
  # We represent high-impedance as nil in Elixir (since there's no
  # electrical "Z" value in software).

  @doc """
  Tri-State Buffer — conditionally connects or disconnects a signal.

  When enable=1, the buffer is "active" and passes the data through.
  When enable=0, the buffer is "high-impedance" and returns nil,
  meaning the output is electrically disconnected from the bus.

  Truth table:

      Data  Enable │ Output
      ────────────┼───────
        0      0   │  nil    (disconnected / high-Z)
        1      0   │  nil    (disconnected / high-Z)
        0      1   │   0     (active, passes data)
        1      1   │   1     (active, passes data)

  ## Examples

      iex> CodingAdventures.LogicGates.Combinational.tri_state(1, 1)
      1
      iex> CodingAdventures.LogicGates.Combinational.tri_state(1, 0)
      nil
      iex> CodingAdventures.LogicGates.Combinational.tri_state(0, 1)
      0
  """
  @spec tri_state(0 | 1, 0 | 1) :: 0 | 1 | nil
  def tri_state(data, enable) do
    Gates.validate_bit!(data, "data")
    Gates.validate_bit!(enable, "enable")

    case enable do
      1 -> data
      0 -> nil
    end
  end
end
