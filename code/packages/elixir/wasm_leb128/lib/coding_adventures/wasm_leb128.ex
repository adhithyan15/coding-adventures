defmodule CodingAdventures.WasmLeb128 do
  @moduledoc """
  LEB128 (Little-Endian Base-128) variable-length integer encoding for the
  WebAssembly binary format.

  ## What is LEB128?

  Imagine you need to store the number 3 in a binary file. You *could* always
  use 8 bytes (a 64-bit integer), but that wastes 7 bytes when the value is
  small. LEB128 is a compression trick: pack 7 bits of data into each byte,
  and use the **high bit** (bit 7) as a "more bytes follow" flag.

  ```
  Byte layout:
    bit 7  (MSB): continuation flag — 1 means "more bytes follow"
    bits 0–6    : 7 bits of actual data
  ```

  Small numbers fit in one byte; large numbers use more bytes. Most integers
  in a WASM module are small (function counts, local counts, instruction
  immediates), so LEB128 keeps the binary format compact.

  ## Encoding Example: 624485 (unsigned)

  ```
  624485 in binary: 10011000011101100101
  Split into 7-bit groups (LSB first):
    group 0: 1100101  → 0x65  → set continuation: 0xE5
    group 1: 0001110  → 0x0E  → set continuation: 0x8E
    group 2: 0100110  → 0x26  → last byte, no continuation
  Result bytes: <<0xE5, 0x8E, 0x26>>
  ```

  ## Unsigned vs Signed

  **Unsigned** encoding stores non-negative integers. The 7-bit groups are
  concatenated LSB-first and the high bit of each byte (except the last)
  signals that more bytes follow.

  **Signed** encoding uses two's complement representation with sign
  extension. When decoding, if the highest *data* bit of the final byte
  is 1 the value is negative and all remaining high bits are filled with 1s
  (sign extension).

  ## WASM Context

  Every integer in a WASM binary file — section lengths, function counts,
  local variable counts, branch depths, and instruction immediates — is
  encoded in LEB128. This module provides the primitives needed by a WASM
  parser.

  This module is part of the coding-adventures monorepo — a ground-up
  implementation of the computing stack from transistors to operating systems.
  """

  # ── Unsigned Decoding ────────────────────────────────────────────────────

  @doc """
  Decode an **unsigned** LEB128 integer from a binary, starting at `offset`.

  Returns `{:ok, {value, bytes_consumed}}` on success, or `{:error, message}`
  if the data is malformed.

  ## Algorithm

  We consume one byte at a time. For each byte:

  1. Extract the low 7 bits: `Bitwise.band(byte, 0x7F)`.
  2. Shift them into place: `Bitwise.bsl(bits, shift)` where `shift`
     starts at 0 and increases by 7 each step.
  3. Accumulate: `value = Bitwise.bor(value, shifted_bits)`.
  4. Check bit 7: if it is 0, this was the last byte.

  We use Elixir binary pattern matching to read bytes efficiently.

  ## Examples

      iex> CodingAdventures.WasmLeb128.decode_unsigned(<<0x00>>, 0)
      {:ok, {0, 1}}

      iex> CodingAdventures.WasmLeb128.decode_unsigned(<<0x03>>, 0)
      {:ok, {3, 1}}

      iex> CodingAdventures.WasmLeb128.decode_unsigned(<<0xE5, 0x8E, 0x26>>, 0)
      {:ok, {624485, 3}}

  ## Errors

  - Offset out of bounds.
  - The continuation flag is set on the last byte available (unterminated).
  """
  @spec decode_unsigned(binary(), non_neg_integer()) ::
          {:ok, {non_neg_integer(), pos_integer()}} | {:error, String.t()}
  def decode_unsigned(data, offset \\ 0) do
    byte_size_data = byte_size(data)

    cond do
      offset >= byte_size_data ->
        {:error,
         "offset #{offset} is out of bounds for data of length #{byte_size_data}"}

      true ->
        # Skip to the starting offset, then decode.
        <<_skip::binary-size(offset), rest::binary>> = data
        decode_unsigned_loop(rest, 0, 0, 0)
    end
  end

  # Internal recursive loop for unsigned decoding.
  #
  # Parameters:
  #   rest           — remaining binary to read from
  #   accumulator    — value accumulated so far
  #   shift          — how many bits we've already placed (multiple of 7)
  #   bytes_consumed — how many bytes we've read so far
  #
  # We use binary pattern matching to pull one byte at a time.
  defp decode_unsigned_loop(<<>>, _acc, _shift, _consumed) do
    # Ran out of bytes while continuation flag was still set (otherwise we
    # would have returned from the branch below).
    {:error, "unexpected end of data: LEB128 sequence is unterminated"}
  end

  defp decode_unsigned_loop(<<byte::8, rest::binary>>, acc, shift, consumed) do
    # Extract the 7 data bits and shift them into position.
    data_bits = Bitwise.band(byte, 0x7F)
    new_acc = Bitwise.bor(acc, Bitwise.bsl(data_bits, shift))
    new_consumed = consumed + 1

    if Bitwise.band(byte, 0x80) == 0 do
      # High bit is clear — this is the last byte.
      {:ok, {new_acc, new_consumed}}
    else
      # High bit is set — more bytes follow.
      decode_unsigned_loop(rest, new_acc, shift + 7, new_consumed)
    end
  end

  # ── Signed Decoding ──────────────────────────────────────────────────────

  @doc """
  Decode a **signed** LEB128 integer from a binary, starting at `offset`.

  Returns `{:ok, {value, bytes_consumed}}` on success, or `{:error, message}`
  if the data is malformed.

  ## Sign Extension

  The loop is identical to unsigned decoding. The difference is the final
  step: **sign extension**. After we read the last byte, we check whether the
  highest *data* bit (bit 6) is set. If so, the number is negative and we must
  fill the upper bits with 1s using a bitmask.

  ```
  Sign extension mask (for 64-bit arithmetic):
    If bit 6 of the last byte is 1 and shift < 64:
      mask = -(1 <<< shift)   (all bits above 'shift' are 1)
      value = value ||| mask
  ```

  In Elixir, integers have arbitrary precision, so we compute the mask as
  `-(1 <<< shift)` which gives an infinitely sign-extended negative number.
  We then AND with `0xFFFFFFFFFFFFFFFF` (max u64) to keep only 64 bits.

  ## Examples

      iex> CodingAdventures.WasmLeb128.decode_signed(<<0x00>>, 0)
      {:ok, {0, 1}}

      iex> CodingAdventures.WasmLeb128.decode_signed(<<0x7E>>, 0)
      {:ok, {-2, 1}}

      iex> CodingAdventures.WasmLeb128.decode_signed(<<0x80, 0x80, 0x80, 0x80, 0x78>>, 0)
      {:ok, {-2147483648, 5}}

  ## Errors

  Same conditions as `decode_unsigned/2`.
  """
  @spec decode_signed(binary(), non_neg_integer()) ::
          {:ok, {integer(), pos_integer()}} | {:error, String.t()}
  def decode_signed(data, offset \\ 0) do
    byte_size_data = byte_size(data)

    cond do
      offset >= byte_size_data ->
        {:error,
         "offset #{offset} is out of bounds for data of length #{byte_size_data}"}

      true ->
        <<_skip::binary-size(offset), rest::binary>> = data
        decode_signed_loop(rest, 0, 0, 0)
    end
  end

  # Internal recursive loop for signed decoding.
  # Uses `Integer.floor_div/2` for arithmetic right shift (sign-preserving).
  defp decode_signed_loop(<<>>, _acc, _shift, _consumed) do
    {:error, "unexpected end of data: LEB128 sequence is unterminated"}
  end

  defp decode_signed_loop(<<byte::8, rest::binary>>, acc, shift, consumed) do
    data_bits = Bitwise.band(byte, 0x7F)
    new_acc = Bitwise.bor(acc, Bitwise.bsl(data_bits, shift))
    new_shift = shift + 7
    new_consumed = consumed + 1

    if Bitwise.band(byte, 0x80) == 0 do
      # Last byte — check for sign extension.
      #
      # If bit 6 of the last byte (the highest data bit) is set, the encoded
      # number is negative. We sign-extend by ORing with a mask that has 1s in
      # all bit positions above `new_shift`.
      #
      # In Elixir, `-(1 <<< new_shift)` gives us exactly that mask:
      #   shift=7  → -(128) = 0xFFFF...FF80 in two's complement
      #   shift=14 → -(16384) = 0xFFFF...FC00, etc.
      #
      # We then keep only 64 bits by ANDing with the max u64 value, then
      # re-interpret as a signed integer using the sign-bit trick.
      value =
        if new_shift < 64 and Bitwise.band(byte, 0x40) != 0 do
          mask = -Bitwise.bsl(1, new_shift)
          raw = Bitwise.bor(new_acc, mask)
          # Re-interpret the lower 64 bits as a signed integer.
          to_signed_64(raw)
        else
          new_acc
        end

      {:ok, {value, new_consumed}}
    else
      decode_signed_loop(rest, new_acc, new_shift, new_consumed)
    end
  end

  # Convert an arbitrary-precision integer to a 64-bit signed integer.
  #
  # Elixir integers are unbounded, so `-1` stored in `new_acc` after the mask
  # OR might be a large negative number. We mask to 64 bits, then check if the
  # sign bit (bit 63) is set to decide whether the result is negative.
  #
  # Two's complement sign conversion:
  #   If bit 63 is 1: value = (raw & 0xFFFFFFFFFFFFFFFF) - 2^64
  #   Otherwise:      value = raw & 0xFFFFFFFFFFFFFFFF
  defp to_signed_64(raw) do
    masked = Bitwise.band(raw, 0xFFFFFFFFFFFFFFFF)

    if masked >= 0x8000000000000000 do
      masked - 0x10000000000000000
    else
      masked
    end
  end

  # ── Unsigned Encoding ────────────────────────────────────────────────────

  @doc """
  Encode an **unsigned** integer as LEB128.

  Returns a binary. Zero encodes as `<<0x00>>` (always at least 1 byte).

  ## Algorithm

  Loop:
  1. Take the low 7 bits: `byte = value &&& 0x7F`.
  2. Shift value right by 7: `value = value >>> 7`.
  3. If `value != 0`, set the continuation flag: `byte = byte ||| 0x80`.
  4. Accumulate byte.
  5. Repeat until `value == 0`.

  ## Examples

      iex> CodingAdventures.WasmLeb128.encode_unsigned(0)
      <<0x00>>

      iex> CodingAdventures.WasmLeb128.encode_unsigned(3)
      <<0x03>>

      iex> CodingAdventures.WasmLeb128.encode_unsigned(624485)
      <<0xE5, 0x8E, 0x26>>

  """
  @spec encode_unsigned(non_neg_integer()) :: binary()
  def encode_unsigned(value) do
    encode_unsigned_loop(value, <<>>)
  end

  defp encode_unsigned_loop(value, acc) do
    # Grab the lowest 7 bits.
    byte = Bitwise.band(value, 0x7F)
    # Shift those bits out.
    remaining = Bitwise.bsr(value, 7)

    if remaining == 0 do
      # No more data — emit this byte without the continuation flag.
      acc <> <<byte>>
    else
      # More bytes will follow — set the continuation flag.
      acc <> <<Bitwise.bor(byte, 0x80)>> |> then(&encode_unsigned_loop(remaining, &1))
    end
  end

  # ── Signed Encoding ──────────────────────────────────────────────────────

  @doc """
  Encode a **signed** integer as LEB128.

  Returns a binary. Negative numbers are stored in two's complement and
  sign-extended so that decoding with `decode_signed/2` recovers the original.

  ## Termination Condition

  Unlike unsigned encoding, the termination check must account for sign
  extension. We stop when no more *meaningful* bits remain:

  ```
  done if:
    remaining == 0  AND  (byte &&& 0x40) == 0   # positive, sign bit is clear
    remaining == -1 AND  (byte &&& 0x40) != 0   # negative, sign bit is set
  ```

  The second condition ensures that decoding will sign-extend correctly.

  ## Examples

      iex> CodingAdventures.WasmLeb128.encode_signed(0)
      <<0x00>>

      iex> CodingAdventures.WasmLeb128.encode_signed(-2)
      <<0x7E>>

      iex> CodingAdventures.WasmLeb128.encode_signed(-2147483648)
      <<0x80, 0x80, 0x80, 0x80, 0x78>>

  """
  @spec encode_signed(integer()) :: binary()
  def encode_signed(value) do
    encode_signed_loop(value, <<>>)
  end

  defp encode_signed_loop(value, acc) do
    # Take the low 7 bits.
    byte = Bitwise.band(value, 0x7F)
    # Arithmetic right shift by 7: preserves sign.
    #
    # Elixir's `Bitwise.bsr` is a *logical* (unsigned) shift for large
    # positive integers, but for negative numbers in Elixir's arbitrary-
    # precision representation we need *floor* division by 2^7 = 128.
    # `Integer.floor_div/2` always rounds toward negative infinity, which
    # matches arithmetic right-shift semantics.
    remaining = Integer.floor_div(value, 128)

    # Termination:
    #   - Positive: no more set bits above, and the current byte's top data
    #     bit (bit 6) is clear (so decode won't sign-extend spuriously).
    #   - Negative: all remaining bits are 1 (-1 in two's complement), and
    #     the current byte's top data bit is set (so decode sign-extends OK).
    done =
      (remaining == 0 and Bitwise.band(byte, 0x40) == 0) or
        (remaining == -1 and Bitwise.band(byte, 0x40) != 0)

    if done do
      acc <> <<byte>>
    else
      acc <> <<Bitwise.bor(byte, 0x80)>> |> then(&encode_signed_loop(remaining, &1))
    end
  end
end
