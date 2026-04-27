defmodule CodingAdventures.MicroQR do
  @moduledoc """
  Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.

  Micro QR Code is the compact variant of standard QR Code, designed for
  applications where even the smallest standard QR (21×21) is too large.
  Common uses include surface-mount component labels, circuit board markings,
  and miniature industrial tags.

  ## Symbol sizes

  ```
  M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
  formula: size = 2 × version_number + 9
  ```

  ## Key differences from regular QR Code

  - **Single finder pattern** at top-left only (one 7×7 square, not three).
  - **Timing at row 0 / col 0** (not row 6 / col 6).
  - **Only 4 mask patterns** (not 8).
  - **Format XOR mask 0x4445** (not 0x5412).
  - **Single copy of format info** (not two).
  - **2-module quiet zone** (not 4).
  - **Narrower mode indicators** (0–3 bits instead of 4).
  - **Single block** RS error correction (no interleaving).

  ## Encoding pipeline

  ```
  input string
    → auto-select smallest symbol (M1..M4) and mode
    → build bit stream (mode indicator + char count + data + terminator + padding)
    → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
    → initialize grid (finder, L-shaped separator, timing at row0/col0, format reserved)
    → zigzag data placement (two-column snake from bottom-right)
    → evaluate 4 mask patterns, pick lowest penalty
    → write format information (15 bits, single copy, XOR 0x4445)
    → ModuleGrid
  ```

  ## IMPORTANT Elixir reserved words

  Elixir reserves `after`, `rescue`, `catch`, `else`, `end`, `do`, `fn`,
  `true`, `false`, `nil`. These cannot be used as variable names. In this
  module we use `ecc_cw` instead of bare `ecc`, `ecc_word` for computed
  values, and descriptive alternatives for all reserved words.
  """

  import Bitwise

  alias CodingAdventures.GF256
  alias CodingAdventures.Barcode2D
  alias CodingAdventures.Barcode2D.{ModuleGrid, Barcode2DLayoutConfig}

  @version "0.1.0"
  def version, do: @version

  # ============================================================================
  # Public types
  # ============================================================================

  @typedoc """
  Micro QR symbol designator. Size = 2 × version_number + 9 modules per side.
  """
  @type micro_qr_version :: :m1 | :m2 | :m3 | :m4

  @typedoc """
  Error correction level.
  - `:detection` — M1 only, detect errors but cannot correct.
  - `:l` — Low (~7% recovery), available in M2/M3/M4.
  - `:m` — Medium (~15% recovery), available in M2/M3/M4.
  - `:q` — Quartile (~25% recovery), M4 only.
  Level H is not available in any Micro QR symbol.
  """
  @type micro_qr_ecc :: :detection | :l | :m | :q

  # ============================================================================
  # Symbol configuration table
  # ============================================================================

  # All the compile-time constants for one (version, ECC) combination.
  # There are exactly 8 valid combinations:
  # M1/detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q.
  #
  # Fields:
  #   version          — :m1 | :m2 | :m3 | :m4
  #   ecc              — :detection | :l | :m | :q
  #   symbol_indicator — 3-bit value placed in format information (0..7)
  #   size             — symbol side length in modules (11, 13, 15, or 17)
  #   data_cw          — number of data codewords (bytes, except M1 uses 2.5)
  #   ecc_cw           — number of ECC codewords
  #   numeric_cap      — max numeric characters (0 = not supported)
  #   alpha_cap        — max alphanumeric characters (0 = not supported)
  #   byte_cap         — max byte characters (0 = not supported)
  #   terminator_bits  — number of zero bits in terminator (3/5/7/9)
  #   mode_bits        — mode indicator bit width (0=M1, 1=M2, 2=M3, 3=M4)
  #   cc_numeric       — character count field width for numeric mode
  #   cc_alpha         — character count field width for alphanumeric mode
  #   cc_byte          — character count field width for byte mode
  #   m1_half_cw       — true for M1: last data "codeword" is 4 bits
  defp symbol_configs do
    [
      # ── M1 / Detection ──────────────────────────────────────────────────
      # M1 is the smallest: 11×11 modules. Only numeric. No mode indicator
      # (there is only one mode, so no indicator is needed — it's implicit).
      # The "3 data codewords" are unusual: 2 full bytes + 4-bit nibble = 20 bits.
      %{
        version: :m1, ecc: :detection, symbol_indicator: 0, size: 11,
        data_cw: 3, ecc_cw: 2,
        numeric_cap: 5, alpha_cap: 0, byte_cap: 0,
        terminator_bits: 3, mode_bits: 0,
        cc_numeric: 3, cc_alpha: 0, cc_byte: 0,
        m1_half_cw: true
      },
      # ── M2 / L (Low ECC) ────────────────────────────────────────────────
      # First symbol to support alphanumeric and byte modes.
      # Mode indicator is 1 bit: 0=numeric, 1=alphanumeric.
      %{
        version: :m2, ecc: :l, symbol_indicator: 1, size: 13,
        data_cw: 5, ecc_cw: 5,
        numeric_cap: 10, alpha_cap: 6, byte_cap: 4,
        terminator_bits: 5, mode_bits: 1,
        cc_numeric: 4, cc_alpha: 3, cc_byte: 4,
        m1_half_cw: false
      },
      # ── M2 / M (Medium ECC) ─────────────────────────────────────────────
      %{
        version: :m2, ecc: :m, symbol_indicator: 2, size: 13,
        data_cw: 4, ecc_cw: 6,
        numeric_cap: 8, alpha_cap: 5, byte_cap: 3,
        terminator_bits: 5, mode_bits: 1,
        cc_numeric: 4, cc_alpha: 3, cc_byte: 4,
        m1_half_cw: false
      },
      # ── M3 / L ──────────────────────────────────────────────────────────
      # 2-bit mode indicator: 00=numeric, 01=alpha, 10=byte.
      %{
        version: :m3, ecc: :l, symbol_indicator: 3, size: 15,
        data_cw: 11, ecc_cw: 6,
        numeric_cap: 23, alpha_cap: 14, byte_cap: 9,
        terminator_bits: 7, mode_bits: 2,
        cc_numeric: 5, cc_alpha: 4, cc_byte: 4,
        m1_half_cw: false
      },
      # ── M3 / M ──────────────────────────────────────────────────────────
      %{
        version: :m3, ecc: :m, symbol_indicator: 4, size: 15,
        data_cw: 9, ecc_cw: 8,
        numeric_cap: 18, alpha_cap: 11, byte_cap: 7,
        terminator_bits: 7, mode_bits: 2,
        cc_numeric: 5, cc_alpha: 4, cc_byte: 4,
        m1_half_cw: false
      },
      # ── M4 / L ──────────────────────────────────────────────────────────
      # 3-bit mode indicator: 000=numeric, 001=alpha, 010=byte, 011=kanji.
      %{
        version: :m4, ecc: :l, symbol_indicator: 5, size: 17,
        data_cw: 16, ecc_cw: 8,
        numeric_cap: 35, alpha_cap: 21, byte_cap: 15,
        terminator_bits: 9, mode_bits: 3,
        cc_numeric: 6, cc_alpha: 5, cc_byte: 5,
        m1_half_cw: false
      },
      # ── M4 / M ──────────────────────────────────────────────────────────
      %{
        version: :m4, ecc: :m, symbol_indicator: 6, size: 17,
        data_cw: 14, ecc_cw: 10,
        numeric_cap: 30, alpha_cap: 18, byte_cap: 13,
        terminator_bits: 9, mode_bits: 3,
        cc_numeric: 6, cc_alpha: 5, cc_byte: 5,
        m1_half_cw: false
      },
      # ── M4 / Q (Quartile ECC) ───────────────────────────────────────────
      # The highest ECC level available in Micro QR. Only M4 can afford this
      # because Q uses 14 ECC codewords out of the total 24, leaving only
      # 10 data codewords.
      %{
        version: :m4, ecc: :q, symbol_indicator: 7, size: 17,
        data_cw: 10, ecc_cw: 14,
        numeric_cap: 21, alpha_cap: 13, byte_cap: 9,
        terminator_bits: 9, mode_bits: 3,
        cc_numeric: 6, cc_alpha: 5, cc_byte: 5,
        m1_half_cw: false
      }
    ]
  end

  # ============================================================================
  # RS generator polynomials
  # ============================================================================

  # Monic RS generator polynomials for GF(256)/0x11D with b=0 convention.
  # g(x) = (x+α⁰)(x+α¹)···(x+α^{n-1})
  # Array has n+1 entries (including leading monic coefficient 1).
  #
  # Only the ECC codeword counts {2, 5, 6, 8, 10, 14} are used in Micro QR.
  #
  # These are compile-time constants derived from the GF(256) field. They were
  # verified against the Rust and TypeScript reference implementations.
  defp get_generator(ecc_count) do
    case ecc_count do
      2  -> [0x01, 0x03, 0x02]
      5  -> [0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68]
      6  -> [0x01, 0x3f, 0x4e, 0x17, 0x9b, 0x05, 0x37]
      8  -> [0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, 0xa2, 0xa3]
      10 -> [0x01, 0xf6, 0x75, 0xa8, 0xd0, 0xc3, 0xe3, 0x36, 0xe1, 0x3c, 0x45]
      14 -> [0x01, 0xf6, 0x9a, 0x60, 0x97, 0x8a, 0xf1, 0xa4, 0xa1, 0x8e, 0xfc, 0x7a, 0x52, 0xad, 0xac]
      _  -> raise ArgumentError, "micro_qr: no generator for ecc_count=#{ecc_count}"
    end
  end

  # ============================================================================
  # Pre-computed format information table
  # ============================================================================

  # All 32 pre-computed format words (after XOR with Micro QR mask 0x4445).
  #
  # Indexed as FORMAT_TABLE[symbol_indicator][mask_pattern].
  #
  # The 15-bit format word structure:
  #   [symbol_indicator (3b)] [mask_pattern (2b)] [BCH-10 remainder]
  # XOR-masked with 0x4445 (Micro QR specific — NOT the regular QR 0x5412).
  #
  # Symbol indicators:
  #   0 = M1,  1 = M2-L,  2 = M2-M,  3 = M3-L,
  #   4 = M3-M, 5 = M4-L,  6 = M4-M,  7 = M4-Q
  @format_table [
    # mask:  0       1       2       3
    [0x4445, 0x4172, 0x4E2B, 0x4B1C],  # M1      (symbol_indicator = 0)
    [0x5528, 0x501F, 0x5F46, 0x5A71],  # M2-L    (symbol_indicator = 1)
    [0x6649, 0x637E, 0x6C27, 0x6910],  # M2-M    (symbol_indicator = 2)
    [0x7764, 0x7253, 0x7D0A, 0x783D],  # M3-L    (symbol_indicator = 3)
    [0x06DE, 0x03E9, 0x0CB0, 0x0987],  # M3-M    (symbol_indicator = 4)
    [0x17F3, 0x12C4, 0x1D9D, 0x18AA],  # M4-L    (symbol_indicator = 5)
    [0x24B2, 0x2185, 0x2EDC, 0x2BEB],  # M4-M    (symbol_indicator = 6)
    [0x359F, 0x30A8, 0x3FF1, 0x3AC6]   # M4-Q    (symbol_indicator = 7)
  ]

  # ============================================================================
  # Alphanumeric character set
  # ============================================================================

  # The 45-character set shared with regular QR Code.
  # Index in this string = alphanumeric value.
  # This encodes: digits 0-9, uppercase A-Z, space, $, %, *, +, -, ., /, :
  @alphanum_chars "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./"
  # Note: the last char is `:` — index 44.
  # We compute the full string with colon appended here:
  @alphanum_chars_with_colon @alphanum_chars <> ":"

  # ============================================================================
  # Public API
  # ============================================================================

  @doc """
  Encode a string to a Micro QR Code `ModuleGrid`.

  Automatically selects the smallest symbol (M1..M4) and encoding mode that
  can hold the input. Pass `version` and/or `ecc` to override the selection.

  ## Parameters

  - `input` — the string to encode.
  - `version` — optional `:m1 | :m2 | :m3 | :m4`. If omitted, auto-selects.
  - `ecc` — optional `:detection | :l | :m | :q`. If omitted, auto-selects
    the best available level for the chosen symbol.

  ## Returns

  - `{:ok, %ModuleGrid{}}` on success.
  - `{:error, reason}` if the input is too long, uses unsupported characters,
    or an invalid version/ECC combination is requested.

  ## Examples

      iex> {:ok, grid} = CodingAdventures.MicroQR.encode("1")
      iex> grid.rows
      11

      iex> {:ok, grid} = CodingAdventures.MicroQR.encode("HELLO")
      iex> grid.rows
      13

      iex> {:ok, grid} = CodingAdventures.MicroQR.encode("https://a.b")
      iex> grid.rows
      17
  """
  @spec encode(String.t(), micro_qr_version() | nil, micro_qr_ecc() | nil) ::
          {:ok, ModuleGrid.t()} | {:error, String.t()}
  def encode(input, version \\ nil, ecc \\ nil) do
    with {:ok, cfg} <- select_config(input, version, ecc),
         {:ok, mode} <- select_mode(input, cfg),
         {:ok, grid} <- do_encode(input, cfg, mode) do
      {:ok, grid}
    end
  end

  @doc """
  Encode a string to a `ModuleGrid`, raising on error.

  Same as `encode/3` but raises `RuntimeError` on failure.
  """
  @spec encode!(String.t(), micro_qr_version() | nil, micro_qr_ecc() | nil) :: ModuleGrid.t()
  def encode!(input, version \\ nil, ecc \\ nil) do
    case encode(input, version, ecc) do
      {:ok, grid} -> grid
      {:error, reason} -> raise RuntimeError, message: reason
    end
  end

  @doc """
  Convert a `ModuleGrid` to a `PaintScene` for rendering.

  Uses a 2-module quiet zone (half the regular QR minimum of 4) as required
  by ISO/IEC 18004:2015 Annex E for Micro QR Code symbols.

  Pass an optional `%Barcode2DLayoutConfig{}` to override defaults.
  The `quiet_zone_modules` field defaults to `2` here (not the barcode_2d
  default of 4).
  """
  @spec layout_grid(ModuleGrid.t(), Barcode2DLayoutConfig.t() | nil) ::
          {:ok, term()} | {:error, String.t()}
  def layout_grid(%ModuleGrid{} = grid, config \\ nil) do
    cfg =
      case config do
        nil -> %Barcode2DLayoutConfig{quiet_zone_modules: 2}
        c   -> c
      end

    Barcode2D.layout(grid, cfg)
  end

  @doc """
  Encode and layout in one step, returning a `PaintScene`.

  Convenience wrapper over `encode/3` + `layout_grid/2`.
  """
  @spec encode_and_layout(String.t(), micro_qr_version() | nil, micro_qr_ecc() | nil, Barcode2DLayoutConfig.t() | nil) ::
          {:ok, term()} | {:error, String.t()}
  def encode_and_layout(input, version \\ nil, ecc \\ nil, config \\ nil) do
    with {:ok, grid} <- encode(input, version, ecc),
         {:ok, scene} <- layout_grid(grid, config) do
      {:ok, scene}
    end
  end

  # ============================================================================
  # Symbol selection
  # ============================================================================

  # Find the smallest symbol configuration that fits the input.
  # Filters by version and/or ECC if specified.
  # Iterates configs in order (M1..M4, L before M before Q) and returns
  # the first one that has capacity for the input in a supported mode.
  defp select_config(input, version, ecc_level) do
    candidates =
      symbol_configs()
      |> Enum.filter(fn cfg ->
        (is_nil(version) or cfg.version == version) and
        (is_nil(ecc_level) or cfg.ecc == ecc_level)
      end)

    if Enum.empty?(candidates) do
      {:error, "ECCNotAvailable: No symbol configuration for version=#{inspect(version)} ecc=#{inspect(ecc_level)}"}
    else
      # Try each candidate in order (smallest first).
      # Return the first one where a supported mode fits the input.
      result =
        Enum.find(candidates, fn cfg ->
          case select_mode(input, cfg) do
            {:ok, mode} ->
              len = input_length(input, mode)
              cap = mode_capacity(cfg, mode)
              cap > 0 and len <= cap
            {:error, _} ->
              false
          end
        end)

      case result do
        nil ->
          {:error, "InputTooLong: input (#{String.length(input)} chars) does not fit in any Micro QR symbol (version=#{inspect(version)}, ecc=#{inspect(ecc_level)}). Maximum is 35 numeric chars in M4-L."}
        cfg ->
          {:ok, cfg}
      end
    end
  end

  # ============================================================================
  # Mode selection
  # ============================================================================

  # Select the most compact encoding mode supported by the given config.
  # Priority: numeric > alphanumeric > byte.
  defp select_mode(input, cfg) do
    cond do
      is_numeric(input) and cfg.cc_numeric > 0 ->
        {:ok, :numeric}
      is_alphanumeric(input) and cfg.alpha_cap > 0 ->
        {:ok, :alphanumeric}
      cfg.byte_cap > 0 ->
        {:ok, :byte}
      true ->
        {:error, "UnsupportedMode: no mode available for this input in #{inspect(cfg.version)}-#{inspect(cfg.ecc)}"}
    end
  end

  # True if all characters are ASCII digits 0-9.
  defp is_numeric(""), do: true
  defp is_numeric(input) do
    String.to_charlist(input)
    |> Enum.all?(fn c -> c >= ?0 and c <= ?9 end)
  end

  # True if all characters are in the 45-character QR alphanumeric set.
  defp is_alphanumeric(""), do: true
  defp is_alphanumeric(input) do
    String.to_charlist(input)
    |> Enum.all?(fn c -> String.contains?(@alphanum_chars_with_colon, <<c>>) end)
  end

  # Return the input length in the given mode.
  # For byte mode, count UTF-8 bytes (not codepoints).
  # For other modes, count characters.
  defp input_length(input, :byte), do: byte_size(input)
  defp input_length(input, _mode), do: String.length(input)

  # Return the capacity of the config for a given mode.
  defp mode_capacity(cfg, :numeric), do: cfg.numeric_cap
  defp mode_capacity(cfg, :alphanumeric), do: cfg.alpha_cap
  defp mode_capacity(cfg, :byte), do: cfg.byte_cap

  # ============================================================================
  # Core encoding pipeline
  # ============================================================================

  defp do_encode(input, cfg, mode) do
    # Step 1: Build data codewords
    data_cw = build_data_codewords(input, cfg, mode)

    # Step 2: Reed-Solomon ECC
    generator = get_generator(cfg.ecc_cw)
    ecc_bytes = rs_encode(data_cw, generator)

    # Step 3: Flatten to bit stream
    # For M1: the third data codeword only contributes its upper 4 bits
    # (the lower nibble was zeroed in build_data_codewords to hold the RS
    # encoder's input steady; we emit only 4 bits to avoid placing phantom zeros).
    final_cw = data_cw ++ ecc_bytes

    bits =
      final_cw
      |> Enum.with_index()
      |> Enum.flat_map(fn {cw, cw_idx} ->
        bits_in_cw =
          if cfg.m1_half_cw and cw_idx == cfg.data_cw - 1 do
            4  # M1 last data codeword: upper nibble only
          else
            8
          end
        # Emit bits MSB-first, skipping the lower (8 - bits_in_cw) bits
        shift_start = bits_in_cw - 1
        Enum.map(shift_start..0//-1, fn bit_pos ->
          ((cw >>> (bit_pos + (8 - bits_in_cw))) &&& 1) == 1
        end)
      end)

    # Step 4: Initialize grid with structural modules
    work_grid = build_work_grid(cfg)

    # Step 5: Place data bits via two-column zigzag
    work_grid = place_bits(work_grid, bits)

    # Step 6: Evaluate all 4 masks, pick the one with lowest penalty
    {best_mask, _best_penalty} =
      Enum.reduce(0..3, {0, :infinity}, fn mask_idx, {best_m, best_p} ->
        masked = apply_mask(work_grid, mask_idx)
        fmt = get_format_word(cfg.symbol_indicator, mask_idx)
        with_fmt = write_format_info(masked, fmt)
        penalty = compute_penalty(with_fmt)
        if penalty < best_p do
          {mask_idx, penalty}
        else
          {best_m, best_p}
        end
      end)

    # Step 7: Apply best mask and write final format info
    final_modules = apply_mask(work_grid, best_mask)
    final_fmt = get_format_word(cfg.symbol_indicator, best_mask)
    final_modules = write_format_info(final_modules, final_fmt)

    grid =
      Barcode2D.make_module_grid(cfg.size, cfg.size)
      |> copy_modules(final_modules, cfg.size)

    {:ok, grid}
  end

  # ============================================================================
  # Data codeword assembly
  # ============================================================================

  # Build the complete data codeword byte sequence.
  #
  # For all symbols except M1:
  #   [mode indicator (0/1/2/3 bits)] [char count] [data bits]
  #   [terminator] [byte-align zeros] [0xEC/0x11 padding]
  #   → exactly cfg.data_cw bytes.
  #
  # For M1 (m1_half_cw = true):
  #   Total capacity = 20 bits = 2 full bytes + 4-bit nibble.
  #   The RS encoder receives 3 bytes where byte[2] has data in its upper 4 bits
  #   and zeros in the lower 4 bits.
  defp build_data_codewords(input, cfg, mode) do
    # Total usable data bit capacity.
    # M1 is special: 3 "codewords" where the last is only 4 bits → 20 bits total.
    total_bits =
      if cfg.m1_half_cw do
        cfg.data_cw * 8 - 4  # 3*8 - 4 = 20 bits for M1
      else
        cfg.data_cw * 8
      end

    # Start building the bit list (MSB-first).
    bits = []

    # Mode indicator (0 bits for M1, 1 for M2, 2 for M3, 3 for M4)
    bits =
      if cfg.mode_bits > 0 do
        indicator = mode_indicator_value(mode, cfg)
        bits ++ int_to_bits(indicator, cfg.mode_bits)
      else
        bits
      end

    # Character count field.
    char_count = input_length(input, mode)
    cc_width = mode_cc_width(cfg, mode)
    bits = bits ++ int_to_bits(char_count, cc_width)

    # Encoded data bits.
    bits = bits ++ encode_data(input, mode)

    # Terminator: up to terminator_bits zero bits, truncated if capacity is full.
    remaining = total_bits - length(bits)
    term_bits = if remaining > 0, do: min(cfg.terminator_bits, remaining), else: 0
    bits = bits ++ List.duplicate(false, term_bits)

    if cfg.m1_half_cw do
      # M1: pad to exactly 20 bits, then pack into 3 bytes.
      bits = (bits ++ List.duplicate(false, 20)) |> Enum.take(20)

      b0 = pack_byte(bits, 0)
      b1 = pack_byte(bits, 8)
      # Third byte: upper nibble = data bits 16-19, lower nibble = 0.
      b2 = pack_nibble(bits, 16) <<< 4
      [b0, b1, b2]
    else
      # Pad to byte boundary with zeros.
      rem = rem(length(bits), 8)
      bits = if rem != 0, do: bits ++ List.duplicate(false, 8 - rem), else: bits

      # Convert to bytes.
      bytes = bits_to_bytes(bits)

      # Fill remaining data codewords with alternating 0xEC / 0x11.
      fill_padding(bytes, cfg.data_cw)
    end
  end

  # Mode indicator value: the small integer prepended before char count.
  # Depends on the symbol (number of mode indicator bits).
  defp mode_indicator_value(mode, cfg) do
    case cfg.mode_bits do
      0 -> 0   # M1: no indicator
      1 -> if mode == :numeric, do: 0, else: 1
      2 -> case mode do
             :numeric      -> 0b00
             :alphanumeric -> 0b01
             :byte         -> 0b10
           end
      3 -> case mode do
             :numeric      -> 0b000
             :alphanumeric -> 0b001
             :byte         -> 0b010
           end
      _ -> 0
    end
  end

  # Return the character count field width for the given mode.
  defp mode_cc_width(cfg, :numeric), do: cfg.cc_numeric
  defp mode_cc_width(cfg, :alphanumeric), do: cfg.cc_alpha
  defp mode_cc_width(cfg, :byte), do: cfg.cc_byte

  # ============================================================================
  # Data encoding helpers
  # ============================================================================

  # Dispatch to mode-specific encoder. Returns a list of booleans (bits).
  defp encode_data(input, :numeric), do: encode_numeric(input)
  defp encode_data(input, :alphanumeric), do: encode_alphanumeric(input)
  defp encode_data(input, :byte), do: encode_byte_mode(input)

  # Numeric mode: groups of 3 digits → 10 bits, pair → 7 bits, single → 4 bits.
  #
  # Example: "12345" → "123" → 0b0001111011 (10 bits), "45" → 0b0101101 (7 bits)
  #
  # This is the most efficient mode for purely numeric data. A group of 3 digits
  # has a maximum value of 999 (needs ceil(log2(1000)) = 10 bits), whereas
  # storing each digit individually as 4 bits would use 12 bits per group.
  defp encode_numeric(input) do
    digits = String.to_charlist(input) |> Enum.map(fn c -> c - ?0 end)
    encode_numeric_digits(digits, [])
  end

  defp encode_numeric_digits([], acc), do: acc
  defp encode_numeric_digits([d1, d2, d3 | rest], acc) do
    value = d1 * 100 + d2 * 10 + d3
    encode_numeric_digits(rest, acc ++ int_to_bits(value, 10))
  end
  defp encode_numeric_digits([d1, d2], acc) do
    value = d1 * 10 + d2
    acc ++ int_to_bits(value, 7)
  end
  defp encode_numeric_digits([d1], acc) do
    acc ++ int_to_bits(d1, 4)
  end

  # Alphanumeric mode: pairs → 11 bits, single → 6 bits.
  #
  # The 45-character set maps letters, digits, and common symbols to indices.
  # Pairs are packed as: first_index * 45 + second_index (max 44*45+44 = 2024,
  # which fits in 11 bits since 2^11 = 2048).
  defp encode_alphanumeric(input) do
    indices =
      String.to_charlist(input)
      |> Enum.map(fn c ->
        case :binary.match(@alphanum_chars_with_colon, <<c>>) do
          {pos, 1} -> pos
          :nomatch  -> 0
        end
      end)
    encode_alpha_indices(indices, [])
  end

  defp encode_alpha_indices([], acc), do: acc
  defp encode_alpha_indices([i1, i2 | rest], acc) do
    value = i1 * 45 + i2
    encode_alpha_indices(rest, acc ++ int_to_bits(value, 11))
  end
  defp encode_alpha_indices([i1], acc) do
    acc ++ int_to_bits(i1, 6)
  end

  # Byte mode: each byte → 8 bits. UTF-8 strings are treated as raw bytes,
  # so a multi-byte codepoint contributes multiple bytes to the count.
  defp encode_byte_mode(input) do
    :binary.bin_to_list(input)
    |> Enum.flat_map(fn b -> int_to_bits(b, 8) end)
  end

  # ============================================================================
  # Reed-Solomon encoder
  # ============================================================================

  # Compute ECC bytes via LFSR polynomial division over GF(256)/0x11D.
  #
  # Returns the remainder of D(x)·x^n mod G(x).
  # Uses the b=0 convention (first root is α^0 = 1), same as regular QR.
  #
  # The algorithm:
  #   1. Initialize a remainder register (list of n zeros).
  #   2. For each data byte b:
  #      a. feedback = b XOR rem[0]
  #      b. Shift the register left by one (drop rem[0], append 0)
  #      c. For each position i: rem[i] XOR= GF_mul(generator[i+1], feedback)
  #   3. The register contains the ECC bytes.
  defp rs_encode(data_bytes, generator) do
    n = length(generator) - 1
    initial_rem = List.duplicate(0, n)

    Enum.reduce(data_bytes, initial_rem, fn data_byte, rem ->
      feedback = bxor(data_byte, hd(rem))
      # Shift register left: drop first element, append zero.
      shifted = tl(rem) ++ [0]

      if feedback == 0 do
        shifted
      else
        # XOR each position with gf_mul(generator[i+1], feedback).
        # generator has n+1 entries; indices 1..n are the non-monic coefficients.
        gen_tail = Enum.drop(generator, 1)
        Enum.zip(shifted, gen_tail)
        |> Enum.map(fn {rem_val, gen_val} ->
          bxor(rem_val, GF256.multiply(gen_val, feedback))
        end)
      end
    end)
  end

  # ============================================================================
  # Working grid (internal mutable-style representation)
  # ============================================================================

  # The working grid is a map with:
  #   :size     — symbol side length (integer)
  #   :modules  — list of `size` lists, each `size` booleans (dark=true)
  #   :reserved — list of `size` lists, each `size` booleans (reserved=true)
  #
  # We build it using a flat list approach indexed by row*size+col for the
  # hot paths, but expose the list-of-lists interface for final conversion.

  defp make_work_grid(size) do
    row = List.duplicate(false, size)
    modules = List.duplicate(row, size)
    reserved = List.duplicate(row, size)
    %{size: size, modules: modules, reserved: reserved}
  end

  # Set a module value and optionally reserve it.
  defp wg_set(grid, row, col, dark, reserve) do
    mod_row = Enum.at(grid.modules, row)
    mod_row = List.replace_at(mod_row, col, dark)
    modules = List.replace_at(grid.modules, row, mod_row)
    grid = %{grid | modules: modules}

    if reserve do
      res_row = Enum.at(grid.reserved, row)
      res_row = List.replace_at(res_row, col, true)
      reserved = List.replace_at(grid.reserved, row, res_row)
      %{grid | reserved: reserved}
    else
      grid
    end
  end

  # Get module value at (row, col).
  defp wg_get_module(grid, row, col) do
    grid.modules |> Enum.at(row) |> Enum.at(col)
  end

  # Check if module at (row, col) is reserved.
  defp wg_reserved?(grid, row, col) do
    grid.reserved |> Enum.at(row) |> Enum.at(col)
  end

  # ============================================================================
  # Grid initialization — structural modules
  # ============================================================================

  # Build the working grid with all structural modules placed and reserved.
  defp build_work_grid(cfg) do
    grid = make_work_grid(cfg.size)
    grid = place_finder(grid)
    grid = place_separator(grid)
    grid = place_timing(grid)
    reserve_format_info(grid)
  end

  # Place the 7×7 finder pattern at the top-left corner (rows 0–6, cols 0–6).
  #
  # The finder pattern is the same 1:1:3:1:1 concentric-square pattern used
  # in regular QR Code. Because Micro QR has only one finder (not three),
  # orientation is inferred from its single-corner placement.
  #
  # Pattern (■ = dark, □ = light):
  #   ■ ■ ■ ■ ■ ■ ■
  #   ■ □ □ □ □ □ ■
  #   ■ □ ■ ■ ■ □ ■
  #   ■ □ ■ ■ ■ □ ■
  #   ■ □ ■ ■ ■ □ ■
  #   ■ □ □ □ □ □ ■
  #   ■ ■ ■ ■ ■ ■ ■
  defp place_finder(grid) do
    Enum.reduce(0..6, grid, fn dr, g ->
      Enum.reduce(0..6, g, fn dc, gg ->
        on_border = dr == 0 or dr == 6 or dc == 0 or dc == 6
        in_core   = dr >= 2 and dr <= 4 and dc >= 2 and dc <= 4
        dark = on_border or in_core
        wg_set(gg, dr, dc, dark, true)
      end)
    end)
  end

  # Place the L-shaped separator (light modules at row 7 cols 0–7, col 7 rows 0–7).
  #
  # In regular QR Code, each of the three finder patterns has a full rectangular
  # border of light "separator" modules on all four sides. In Micro QR, the single
  # finder sits in the top-left corner with the top and left sides of the finder
  # ON the symbol boundary — there is nothing to separate there. Only the bottom
  # and right sides need separators, producing an L-shape.
  defp place_separator(grid) do
    # Bottom edge of finder: row 7, cols 0–7
    grid =
      Enum.reduce(0..7, grid, fn c, g ->
        wg_set(g, 7, c, false, true)
      end)

    # Right edge of finder: col 7, rows 0–7
    Enum.reduce(0..7, grid, fn r, g ->
      wg_set(g, r, 7, false, true)
    end)
  end

  # Place timing pattern extensions along row 0 and col 0.
  #
  # In Micro QR, timing patterns run along the OUTER EDGES of the symbol
  # (row 0 and col 0), unlike regular QR where they run along row 6 / col 6.
  #
  # Positions 0–6 are already determined by the finder pattern (all dark,
  # matching the timing pattern's even-index dark rule since indices 0,2,4,6
  # are even). Position 7 is the separator (light). Positions 8 and beyond
  # are placed here, alternating: dark at even index, light at odd.
  defp place_timing(grid) do
    sz = grid.size

    grid =
      Enum.reduce(8..(sz - 1), grid, fn col, g ->
        dark = rem(col, 2) == 0
        wg_set(g, 0, col, dark, true)
      end)

    Enum.reduce(8..(sz - 1), grid, fn row, g ->
      dark = rem(row, 2) == 0
      wg_set(g, row, 0, dark, true)
    end)
  end

  # Reserve the 15 format information module positions (initially light/false).
  #
  # Format info layout:
  #   Row 8, cols 1–8 → bits f14..f7 (MSB first, placed later)
  #   Col 8, rows 1–7 → bits f6..f0 (f6 at row 7, f0 at row 1)
  #
  # Note: the format modules at (8,1)..(8,8) and (1,8)..(7,8) are within the
  # 9×9 top-left corner where the finder, separator, and timing all meet.
  defp reserve_format_info(grid) do
    # Row 8, cols 1–8
    grid =
      Enum.reduce(1..8, grid, fn col, g ->
        wg_set(g, 8, col, false, true)
      end)

    # Col 8, rows 1–7
    Enum.reduce(1..7, grid, fn row, g ->
      wg_set(g, row, 8, false, true)
    end)
  end

  # Write the 15-bit format word into the reserved positions.
  #
  # Bit f14 (MSB) → row 8 col 1, f13 → row 8 col 2, ..., f7 → row 8 col 8.
  # f6 → col 8 row 7, f5 → col 8 row 6, ..., f0 (LSB) → col 8 row 1.
  #
  # The MSB-first ordering in row 8 is consistent with the lesson learned from
  # the QR Code format info bug: always MSB-first in the horizontal strip.
  defp write_format_info(grid, fmt) do
    # Row 8, cols 1–8: bits f14 down to f7
    grid =
      Enum.reduce(0..7, grid, fn i, g ->
        bit = (fmt >>> (14 - i)) &&& 1
        dark = bit == 1
        mod_row = Enum.at(g.modules, 8)
        mod_row = List.replace_at(mod_row, 1 + i, dark)
        modules = List.replace_at(g.modules, 8, mod_row)
        %{g | modules: modules}
      end)

    # Col 8, rows 7 down to 1: bits f6 down to f0
    Enum.reduce(0..6, grid, fn i, g ->
      row_idx = 7 - i
      bit = (fmt >>> (6 - i)) &&& 1
      dark = bit == 1
      mod_row = Enum.at(g.modules, row_idx)
      mod_row = List.replace_at(mod_row, 8, dark)
      modules = List.replace_at(g.modules, row_idx, mod_row)
      %{g | modules: modules}
    end)
  end

  # Get a pre-computed format word from the table.
  defp get_format_word(symbol_indicator, mask_idx) do
    @format_table
    |> Enum.at(symbol_indicator)
    |> Enum.at(mask_idx)
  end

  # ============================================================================
  # Data placement — two-column zigzag
  # ============================================================================

  # Place bits from the final codeword stream into the grid.
  #
  # The algorithm scans the symbol in a two-column zigzag pattern, starting
  # from the bottom-right corner and moving left two columns at a time,
  # alternating up and down direction each pass.
  #
  # Why two-column zigzag? The algorithm naturally fills the grid in a compact
  # serpentine pattern that distributes data bits evenly across the symbol,
  # which helps masking produce balanced dark/light proportions.
  #
  # Unlike regular QR, there is NO timing column at col 6 to skip around.
  # Micro QR's timing is at col 0, which is reserved and auto-skipped.
  # The zigzag stops at col >= 1 (not >= 0) because when col=1, the pair
  # scanned is cols 1 and 0. Col 0 is reserved (timing), so it is skipped.
  defp place_bits(grid, bits) do
    sz = grid.size
    bits_array = bits  # list of booleans

    {grid, _idx, _up} =
      # Start col at size-1, going up first.
      # Step 2 each iteration (two-column pair per pass).
      Stream.iterate(sz - 1, fn col -> col - 2 end)
      |> Stream.take_while(fn col -> col >= 1 end)
      |> Enum.reduce({grid, 0, true}, fn col, {g, bit_idx, going_up} ->
        row_order = if going_up, do: (sz - 1)..0//-1, else: 0..(sz - 1)

        {g, bit_idx} =
          Enum.reduce(row_order, {g, bit_idx}, fn row, {gg, bidx} ->
            # Try both columns in the pair: right column first, then left.
            Enum.reduce([col, col - 1], {gg, bidx}, fn c, {ggg, bix} ->
              if wg_reserved?(ggg, row, c) do
                {ggg, bix}
              else
                dark =
                  if bix < length(bits_array) do
                    Enum.at(bits_array, bix)
                  else
                    false  # remainder bits
                  end
                {wg_set(ggg, row, c, dark, false), bix + 1}
              end
            end)
          end)

        {g, bit_idx, not going_up}
      end)

    grid
  end

  # ============================================================================
  # Masking
  # ============================================================================

  # Micro QR uses only 4 of the 8 mask patterns from regular QR Code.
  # The mask conditions flip non-reserved module values.
  #
  # | Pattern | Condition (flip if true) |
  # |---------|-------------------------|
  # | 0       | (row + col) mod 2 == 0  |
  # | 1       | row mod 2 == 0          |
  # | 2       | col mod 3 == 0          |
  # | 3       | (row + col) mod 3 == 0  |
  defp mask_condition(mask_idx, row, col) do
    case mask_idx do
      0 -> rem(row + col, 2) == 0
      1 -> rem(row, 2) == 0
      2 -> rem(col, 3) == 0
      3 -> rem(row + col, 3) == 0
      _ -> false
    end
  end

  # Apply mask pattern to all non-reserved modules.
  # Returns a new working grid (reserved map unchanged).
  defp apply_mask(grid, mask_idx) do
    new_modules =
      Enum.with_index(grid.modules)
      |> Enum.map(fn {row_data, row} ->
        Enum.with_index(row_data)
        |> Enum.map(fn {dark, col} ->
          if wg_reserved?(grid, row, col) do
            dark
          else
            dark != mask_condition(mask_idx, row, col)
          end
        end)
      end)

    %{grid | modules: new_modules}
  end

  # ============================================================================
  # Penalty scoring
  # ============================================================================

  # Compute the 4-rule penalty score (same rules as regular QR Code).
  # Lower penalty = better mask selection.
  #
  # Rule 1: runs of ≥5 same-color modules in any row/column
  # Rule 2: 2×2 same-color blocks anywhere in the grid
  # Rule 3: finder-pattern-like 11-module sequences in rows/columns
  # Rule 4: dark proportion deviation from 50%
  defp compute_penalty(grid) do
    sz = grid.size
    modules = grid.modules

    penalty_r1 = penalty_rule1(modules, sz)
    penalty_r2 = penalty_rule2(modules, sz)
    penalty_r3 = penalty_rule3(modules, sz)
    penalty_r4 = penalty_rule4(modules, sz)

    penalty_r1 + penalty_r2 + penalty_r3 + penalty_r4
  end

  # Rule 1: adjacent same-color runs of ≥ 5 modules.
  # Score += (run_length - 2) for each qualifying run.
  # Run of 5 → +3, run of 6 → +4, run of 7 → +5, etc.
  defp penalty_rule1(modules, sz) do
    # Helper: score a single run in one direction.
    score_runs = fn seq ->
      {score, run, _prev} =
        Enum.reduce(seq, {0, 1, Enum.at(seq, 0)}, fn cur, {s, r, prev} ->
          if cur == prev do
            {s, r + 1, cur}
          else
            add = if r >= 5, do: r - 2, else: 0
            {s + add, 1, cur}
          end
        end)
      # Don't forget the last run
      add = if run >= 5, do: run - 2, else: 0
      score + add
    end

    row_penalty =
      Enum.sum(
        Enum.map(modules, fn row ->
          score_runs.(row)
        end)
      )

    col_penalty =
      Enum.sum(
        Enum.map(0..(sz - 1), fn col ->
          seq = Enum.map(modules, fn row -> Enum.at(row, col) end)
          score_runs.(seq)
        end)
      )

    row_penalty + col_penalty
  end

  # Rule 2: 2×2 same-color blocks → +3 per block.
  # A 2×2 block exists when all four corners have the same value.
  defp penalty_rule2(modules, sz) do
    Enum.reduce(0..(sz - 2), 0, fn r, acc ->
      Enum.reduce(0..(sz - 2), acc, fn c, a ->
        d = modules |> Enum.at(r) |> Enum.at(c)
        d2 = modules |> Enum.at(r) |> Enum.at(c + 1)
        d3 = modules |> Enum.at(r + 1) |> Enum.at(c)
        d4 = modules |> Enum.at(r + 1) |> Enum.at(c + 1)
        if d == d2 and d == d3 and d == d4, do: a + 3, else: a
      end)
    end)
  end

  # Rule 3: finder-pattern-like sequences → +40 each.
  # The two patterns to detect:
  #   P1: 1 0 1 1 1 0 1 0 0 0 0
  #   P2: 0 0 0 0 1 0 1 1 1 0 1
  # (P2 is the reverse of P1.)
  # Checked in both rows and columns.
  @p1 [true, false, true, true, true, false, true, false, false, false, false]
  @p2 [false, false, false, false, true, false, true, true, true, false, true]

  defp penalty_rule3(modules, sz) do
    limit = max(0, sz - 11)

    row_penalty =
      Enum.sum(
        Enum.map(0..(sz - 1), fn r ->
          Enum.sum(
            Enum.map(0..limit, fn c ->
              seq = Enum.map(0..10, fn k -> Enum.at(Enum.at(modules, r), c + k) end)
              p1_match = if seq == @p1, do: 40, else: 0
              p2_match = if seq == @p2, do: 40, else: 0
              p1_match + p2_match
            end)
          )
        end)
      )

    col_penalty =
      Enum.sum(
        Enum.map(0..(sz - 1), fn c ->
          Enum.sum(
            Enum.map(0..limit, fn r ->
              seq = Enum.map(0..10, fn k -> Enum.at(Enum.at(modules, r + k), c) end)
              p1_match = if seq == @p1, do: 40, else: 0
              p2_match = if seq == @p2, do: 40, else: 0
              p1_match + p2_match
            end)
          )
        end)
      )

    row_penalty + col_penalty
  end

  # Rule 4: dark module proportion deviation from 50%.
  # dark_pct = dark_count * 100 / total
  # penalty = min(|prev5 - 50|, |next5 - 50|) / 5 * 10
  # where prev5 = largest multiple of 5 ≤ dark_pct.
  defp penalty_rule4(modules, sz) do
    dark = modules |> Enum.concat() |> Enum.count(fn d -> d end)
    total = sz * sz
    dark_pct = div(dark * 100, total)
    prev5 = div(dark_pct, 5) * 5
    next5 = prev5 + 5
    r4 = min(abs(prev5 - 50), abs(next5 - 50))
    div(r4, 5) * 10
  end

  # ============================================================================
  # ModuleGrid conversion
  # ============================================================================

  # Copy the final module values from the working grid into a ModuleGrid.
  # The ModuleGrid is the standard output type for all barcode_2d encoders.
  defp copy_modules(%ModuleGrid{} = grid, work_grid, size) do
    Enum.reduce(0..(size - 1), grid, fn row, g ->
      Enum.reduce(0..(size - 1), g, fn col, gg ->
        dark = wg_get_module(work_grid, row, col)
        case Barcode2D.set_module(gg, row, col, dark) do
          {:ok, updated} -> updated
          {:error, _}    -> gg
        end
      end)
    end)
  end

  # ============================================================================
  # Bit manipulation helpers
  # ============================================================================

  # Convert an integer to a list of `width` booleans, MSB first.
  # int_to_bits(0b1010, 4) → [true, false, true, false]
  defp int_to_bits(value, width) do
    Enum.map((width - 1)..0//-1, fn bit_pos ->
      ((value >>> bit_pos) &&& 1) == 1
    end)
  end

  # Extract 8 bits from a bit list starting at offset and pack them into a byte.
  defp pack_byte(bits, offset) do
    Enum.reduce(0..7, 0, fn i, acc ->
      bit = if Enum.at(bits, offset + i), do: 1, else: 0
      (acc <<< 1) ||| bit
    end)
  end

  # Extract 4 bits from a bit list starting at offset and pack into a nibble.
  defp pack_nibble(bits, offset) do
    Enum.reduce(0..3, 0, fn i, acc ->
      bit = if Enum.at(bits, offset + i), do: 1, else: 0
      (acc <<< 1) ||| bit
    end)
  end

  # Convert a list of booleans (MSB-first groups of 8) to a byte list.
  defp bits_to_bytes(bits) do
    bits
    |> Enum.chunk_every(8)
    |> Enum.map(fn chunk ->
      chunk
      |> Enum.reduce(0, fn bit, acc ->
        b = if bit, do: 1, else: 0
        (acc <<< 1) ||| b
      end)
    end)
  end

  # Pad a byte list to exactly `target_len` bytes using alternating 0xEC/0x11.
  defp fill_padding(bytes, target_len) do
    pad = if rem(length(bytes), 2) == 0, do: 0xEC, else: 0x11
    fill_padding(bytes, target_len, pad)
  end

  defp fill_padding(bytes, target_len, _pad) when length(bytes) >= target_len do
    Enum.take(bytes, target_len)
  end
  defp fill_padding(bytes, target_len, pad) do
    next_pad = if pad == 0xEC, do: 0x11, else: 0xEC
    fill_padding(bytes ++ [pad], target_len, next_pad)
  end
end
