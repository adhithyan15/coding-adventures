defmodule CodingAdventures.QrCode do
  @moduledoc """
  QR Code encoder — ISO/IEC 18004:2015 compliant.

  QR Code (Quick Response) was invented by Masahiro Hara at Denso Wave in
  1994 to track automotive parts. It is now the most widely deployed 2D
  barcode on earth. This encoder produces valid, scannable QR Codes from
  any UTF-8 string.

  ## Encoding pipeline

  ```
  input string
    → mode selection    (numeric / alphanumeric / byte)
    → version selection (smallest version that fits at the chosen ECC level)
    → bit stream        (mode indicator + char count + data + padding)
    → blocks + RS ECC   (GF(256) b=0 convention, poly 0x11D)
    → interleave        (data CWs interleaved, then ECC CWs)
    → grid init         (finder, separator, timing, alignment, format, dark)
    → zigzag placement  (two-column snake from bottom-right corner)
    → mask evaluation   (8 patterns, lowest 4-rule penalty wins)
    → finalize          (format info + version info v7+)
    → ModuleGrid        (abstract boolean grid, true = dark)
  ```

  ## Error correction levels

  | Level | Recovery | Use case                         |
  |-------|----------|----------------------------------|
  | :l    | ~7%      | Maximum data density             |
  | :m    | ~15%     | General-purpose (common default) |
  | :q    | ~25%     | Moderate noise/damage expected   |
  | :h    | ~30%     | High damage risk, logo overlaid  |

  ## Quick example

  ```elixir
  {:ok, grid} = CodingAdventures.QrCode.encode("HELLO WORLD", :m)
  # grid.rows == grid.cols == 21  (version 1)
  # grid.modules is a list of lists of booleans
  ```

  ## Elixir reserved word note

  Elixir reserves words like `after`, `rescue`, `catch`, `else`, `end`.
  These cannot be used as variable names. In this module we avoid all of
  them — for example, using `going_up` instead of `after` or `remainder_val`
  instead of `rescue`.
  """

  import Bitwise

  alias CodingAdventures.Barcode2D.ModuleGrid
  alias CodingAdventures.QrCode.Tables
  alias CodingAdventures.QrCode.Encoder
  alias CodingAdventures.QrCode.RS

  @version_str "0.1.0"
  def version, do: @version_str

  # ============================================================================
  # Public API
  # ============================================================================

  @doc """
  Encode a UTF-8 string into a QR Code `ModuleGrid`.

  Returns `{:ok, grid}` on success, or `{:error, reason}` on failure.

  The returned grid is a `%CodingAdventures.Barcode2D.ModuleGrid{}` with:
  - `rows` = `cols` = `4 * version + 17`
  - `modules` — list of rows, each a list of booleans (`true` = dark module)
  - `module_shape` = `:square`

  ## Parameters

  - `input` — the string to encode. Any UTF-8 content is valid.
  - `ecc_level` — one of `:l`, `:m`, `:q`, `:h`. Defaults to `:m`.

  ## Errors

  - `{:error, :input_too_long}` — input exceeds version-40 capacity at the
    chosen ECC level.

  ## Example

  ```elixir
  {:ok, grid} = CodingAdventures.QrCode.encode("https://example.com")
  # grid.rows == 29  (version 3 at :m)
  ```
  """
  @spec encode(String.t(), :l | :m | :q | :h) ::
          {:ok, ModuleGrid.t()} | {:error, atom()}
  def encode(input, ecc_level \\ :m) do
    # Guard: QR Code v40 holds at most 7089 numeric characters (~2953 bytes
    # in byte mode). Without this guard, select_version/2 would try all 40
    # versions before returning an error, allocating repeatedly for huge
    # inputs — a cheap DoS amplifier in a server context.
    if byte_size(input) > 7089 do
      {:error, :input_too_long}
    else
      case Encoder.select_version(input, ecc_level) do
        {:error, :input_too_long} ->
          {:error, :input_too_long}

        {:ok, ver} ->
          grid = build_qr(input, ver, ecc_level)
          {:ok, grid}
      end
    end
  end

  # ============================================================================
  # Main encoding pipeline
  # ============================================================================

  # Runs the full QR Code pipeline for the given input, version, and ECC level.
  # Returns a fully-finalized ModuleGrid.
  defp build_qr(input, ver, ecc_level) do
    sz = symbol_size(ver)

    # Step 1: Build data codewords (mode + char count + payload + padding).
    data_cw = Encoder.build_data_codewords(input, ver, ecc_level)

    # Step 2: Split into blocks and compute RS ECC for each block.
    blocks = RS.compute_blocks(data_cw, ver, ecc_level)

    # Step 3: Interleave all block codewords (data then ECC, round-robin).
    interleaved = RS.interleave_blocks(blocks)

    # Step 4: Initialize the work grid with all structural patterns.
    {modules, reserved} = build_grid(ver, sz)

    # Step 5: Place interleaved codewords into the data area (zigzag scan).
    modules = place_bits(modules, reserved, interleaved, ver, sz)

    # Step 6: Evaluate all 8 masks, pick the one with the lowest penalty score.
    best_mask = select_best_mask(modules, reserved, sz, ecc_level)

    # Step 7: Apply the best mask and write final format + version information.
    final_modules = apply_mask(modules, reserved, sz, best_mask)
    fmt_bits = compute_format_bits(ecc_level, best_mask)
    final_modules = write_format_info(final_modules, fmt_bits, sz)
    final_modules = write_version_info(final_modules, ver, sz)

    %ModuleGrid{
      rows: sz,
      cols: sz,
      modules: final_modules,
      module_shape: :square
    }
  end

  # ============================================================================
  # Grid geometry
  # ============================================================================

  # Symbol size: (4 × version + 17). Version 1 = 21×21, Version 40 = 177×177.
  defp symbol_size(ver), do: 4 * ver + 17

  # ============================================================================
  # Grid initialization — structural patterns
  # ============================================================================

  # Build the work grid: two 2D boolean arrays, each sz×sz.
  # Both are represented as a list of sz rows, each row a list of sz booleans.
  #
  # `modules`  — true = dark module.
  # `reserved` — true = this position is structural (skip during data
  #              placement and masking).
  #
  # Returns {modules, reserved}.
  defp build_grid(ver, sz) do
    # Start with all-false grids.
    modules = List.duplicate(List.duplicate(false, sz), sz)
    reserved = List.duplicate(List.duplicate(false, sz), sz)

    # Place three finder patterns at the three corners.
    {modules, reserved} = place_finder(modules, reserved, 0, 0)        # top-left
    {modules, reserved} = place_finder(modules, reserved, 0, sz - 7)   # top-right
    {modules, reserved} = place_finder(modules, reserved, sz - 7, 0)   # bottom-left

    # Separators — 1-module light border just outside each finder pattern.
    {modules, reserved} = place_separators(modules, reserved, sz)

    # Timing strips — alternating dark/light on row 6 and col 6.
    {modules, reserved} = place_timing_strips(modules, reserved, sz)

    # Alignment patterns (version 2+).
    {modules, reserved} = place_all_alignments(modules, reserved, ver)

    # Reserve format information positions (two copies, 15 modules each).
    reserved = reserve_format_info(reserved, sz)

    # Reserve version information positions (v7+: two 6×3 blocks).
    reserved = reserve_version_info(reserved, ver, sz)

    # Always-dark module at (4V+9, 8).
    {modules, reserved} = place_dark_module(modules, reserved, ver)

    {modules, reserved}
  end

  # ---------------------------------------------------------------------------
  # Finder pattern
  # ---------------------------------------------------------------------------

  # 7×7 finder pattern centred at (top_row, top_col).
  #
  # The pattern looks like:
  #   ■ ■ ■ ■ ■ ■ ■
  #   ■ □ □ □ □ □ ■
  #   ■ □ ■ ■ ■ □ ■
  #   ■ □ ■ ■ ■ □ ■
  #   ■ □ ■ ■ ■ □ ■
  #   ■ □ □ □ □ □ ■
  #   ■ ■ ■ ■ ■ ■ ■
  #
  # The 1:1:3:1:1 dark:light ratio in every scan direction lets any decoder
  # locate and orient the symbol even under partial occlusion or rotation.
  defp place_finder(modules, reserved, top_row, top_col) do
    Enum.reduce(0..6, {modules, reserved}, fn dr, {m_acc, r_acc} ->
      Enum.reduce(0..6, {m_acc, r_acc}, fn dc, {m, r} ->
        on_border = dr == 0 or dr == 6 or dc == 0 or dc == 6
        in_core = dr >= 2 and dr <= 4 and dc >= 2 and dc <= 4
        dark = on_border or in_core
        row = top_row + dr
        col = top_col + dc
        m = set_mod(m, row, col, dark)
        r = set_reserved(r, row, col)
        {m, r}
      end)
    end)
  end

  # ---------------------------------------------------------------------------
  # Separators
  # ---------------------------------------------------------------------------

  # 1-module light separator border just outside each finder pattern.
  #
  # Top-left finder at (0,0): row 7 (cols 0..7) and col 7 (rows 0..7).
  # Top-right finder at (0,sz-7): row 7 (cols sz-8..sz-1) and col sz-8 (rows 0..7).
  # Bottom-left finder at (sz-7,0): row sz-8 (cols 0..7) and col 7 (rows sz-8..sz-1).
  defp place_separators(modules, reserved, sz) do
    # Top-left separator
    {modules, reserved} =
      Enum.reduce(0..7, {modules, reserved}, fn i, {m, r} ->
        m = set_mod(m, 7, i, false)
        r = set_reserved(r, 7, i)
        m = set_mod(m, i, 7, false)
        r = set_reserved(r, i, 7)
        {m, r}
      end)

    # Top-right separator
    {modules, reserved} =
      Enum.reduce(0..7, {modules, reserved}, fn i, {m, r} ->
        m = set_mod(m, 7, sz - 1 - i, false)
        r = set_reserved(r, 7, sz - 1 - i)
        m = set_mod(m, i, sz - 8, false)
        r = set_reserved(r, i, sz - 8)
        {m, r}
      end)

    # Bottom-left separator
    Enum.reduce(0..7, {modules, reserved}, fn i, {m, r} ->
      m = set_mod(m, sz - 8, i, false)
      r = set_reserved(r, sz - 8, i)
      m = set_mod(m, sz - 1 - i, 7, false)
      r = set_reserved(r, sz - 1 - i, 7)
      {m, r}
    end)
  end

  # ---------------------------------------------------------------------------
  # Timing strips
  # ---------------------------------------------------------------------------

  # Alternating dark/light timing patterns.
  # Row 6, columns 8..sz-9 (horizontal).
  # Col 6, rows 8..sz-9 (vertical).
  # Dark when index is even (starts and ends dark).
  defp place_timing_strips(modules, reserved, sz) do
    # Horizontal timing strip: row 6, cols 8..sz-9
    {modules, reserved} =
      Enum.reduce(8..(sz - 9), {modules, reserved}, fn c, {m, r} ->
        m = set_mod(m, 6, c, rem(c, 2) == 0)
        r = set_reserved(r, 6, c)
        {m, r}
      end)

    # Vertical timing strip: col 6, rows 8..sz-9
    Enum.reduce(8..(sz - 9), {modules, reserved}, fn row_idx, {m, r} ->
      m = set_mod(m, row_idx, 6, rem(row_idx, 2) == 0)
      r = set_reserved(r, row_idx, 6)
      {m, r}
    end)
  end

  # ---------------------------------------------------------------------------
  # Alignment patterns
  # ---------------------------------------------------------------------------

  # 5×5 alignment pattern centered at (row, col).
  #
  # Pattern:
  #   ■ ■ ■ ■ ■
  #   ■ □ □ □ ■
  #   ■ □ ■ □ ■
  #   ■ □ □ □ ■
  #   ■ ■ ■ ■ ■
  #
  # Appears in versions 2+ at tabulated positions, helping decoders correct
  # for perspective distortion and barrel/pincushion warping.
  defp place_alignment(modules, reserved, row, col) do
    Enum.reduce(-2..2, {modules, reserved}, fn dr, {m_acc, r_acc} ->
      Enum.reduce(-2..2, {m_acc, r_acc}, fn dc, {m, r} ->
        on_border = abs(dr) == 2 or abs(dc) == 2
        is_center = dr == 0 and dc == 0
        dark = on_border or is_center
        m = set_mod(m, row + dr, col + dc, dark)
        r = set_reserved(r, row + dr, col + dc)
        {m, r}
      end)
    end)
  end

  # Place all alignment patterns for the version.
  # All pairwise combinations of alignment_positions are considered.
  # Any whose center falls on an already-reserved module is skipped —
  # this naturally excludes the three finder-overlap positions.
  defp place_all_alignments(modules, reserved, ver) do
    positions = Tables.alignment_positions(ver)

    Enum.reduce(positions, {modules, reserved}, fn row, {m_acc, r_acc} ->
      Enum.reduce(positions, {m_acc, r_acc}, fn col, {m, r} ->
        # Check if center is already reserved (overlaps finder/timing).
        if get_reserved(r, row, col) do
          {m, r}
        else
          place_alignment(m, r, row, col)
        end
      end)
    end)
  end

  # ---------------------------------------------------------------------------
  # Format information reservation
  # ---------------------------------------------------------------------------

  # Reserve the 15 format information module positions (×2 copies).
  # Placeholder (false) values are written; actual bits come after mask selection.
  #
  # Copy 1 — adjacent to top-left finder:
  #   (8, 0..5), (8, 7), (8, 8), (7, 8), (5..0, 8)
  # Copy 2:
  #   (sz-1..sz-7, 8) and (8, sz-8..sz-1)
  defp reserve_format_info(reserved, sz) do
    # Copy 1 horizontal strip: row 8, cols 0..8 (skip col 6 = timing)
    reserved =
      Enum.reduce(0..8, reserved, fn c, r ->
        if c != 6, do: set_reserved(r, 8, c), else: r
      end)

    # Copy 1 vertical strip: col 8, rows 0..8 (skip row 6 = timing)
    reserved =
      Enum.reduce(0..8, reserved, fn row_idx, r ->
        if row_idx != 6, do: set_reserved(r, row_idx, 8), else: r
      end)

    # Copy 2 bottom-left: col 8, rows sz-7..sz-1
    reserved =
      Enum.reduce((sz - 7)..(sz - 1), reserved, fn row_idx, r ->
        set_reserved(r, row_idx, 8)
      end)

    # Copy 2 top-right: row 8, cols sz-8..sz-1
    Enum.reduce((sz - 8)..(sz - 1), reserved, fn c, r ->
      set_reserved(r, 8, c)
    end)
  end

  # ---------------------------------------------------------------------------
  # Version information reservation
  # ---------------------------------------------------------------------------

  # Reserve version information positions for versions 7+: two 6×3 blocks.
  # Near top-right: rows 0..5, cols sz-11..sz-9.
  # Near bottom-left: rows sz-11..sz-9, cols 0..5.
  defp reserve_version_info(reserved, ver, _sz) when ver < 7, do: reserved

  defp reserve_version_info(reserved, _ver, sz) do
    reserved =
      Enum.reduce(0..5, reserved, fn row_idx, r ->
        Enum.reduce(0..2, r, fn dc, r2 ->
          set_reserved(r2, row_idx, sz - 11 + dc)
        end)
      end)

    Enum.reduce(0..2, reserved, fn dr, r ->
      Enum.reduce(0..5, r, fn col, r2 ->
        set_reserved(r2, sz - 11 + dr, col)
      end)
    end)
  end

  # ---------------------------------------------------------------------------
  # Always-dark module
  # ---------------------------------------------------------------------------

  # The always-dark module at row (4V+9), col 8.
  # This is a fixed dark module not part of any data, not masked.
  defp place_dark_module(modules, reserved, ver) do
    row_idx = 4 * ver + 9
    modules = set_mod(modules, row_idx, 8, true)
    reserved = set_reserved(reserved, row_idx, 8)
    {modules, reserved}
  end

  # ============================================================================
  # Data bit placement — zigzag scan
  # ============================================================================

  # Place the interleaved codeword stream using the two-column zigzag scan.
  #
  # Scans from column sz-1 leftward in 2-column strips, alternating
  # upward/downward:
  #   - Column 6 (vertical timing strip) is always skipped.
  #   - After decrementing from col=8 to col=7, the next step goes to col=5
  #     (skipping col=6 which is the timing strip).
  #   - Reserved modules are skipped; data bits fill the rest.
  defp place_bits(modules, reserved, codewords, ver, sz) do
    # Flatten codewords to a bit array (MSB first), then append remainder bits.
    bits =
      Enum.flat_map(codewords, fn cw ->
        for b <- 7..0//-1, do: (cw >>> b) &&& 1
      end)

    remainder = Tables.remainder_bits(ver)
    all_bits = bits ++ List.duplicate(0, remainder)

    # Run the zigzag scan, starting at the rightmost column pair, going up.
    {modules, _final_idx} = do_zigzag(modules, reserved, all_bits, sz, sz - 1, true, 0)
    modules
  end

  # Iterative zigzag column-pair scanner using Enum.reduce over column positions.
  # Each iteration processes one 2-column strip.
  defp do_zigzag(modules, reserved, bits, sz, _start_col, going_up_init, bit_idx_init) do
    # Build the list of leading columns for each 2-column strip.
    # Start at sz-1, step down by 2, skipping col 6.
    col_list = build_col_list(sz - 1, [])

    Enum.reduce(col_list, {modules, bit_idx_init, going_up_init}, fn col, {m, idx, going_up} ->
      {m2, idx2} = scan_col_pair(m, reserved, bits, sz, col, going_up, idx)
      {m2, idx2, not going_up}
    end)
    |> then(fn {m, idx, _dir} -> {m, idx} end)
  end

  # Build the list of leading columns for the zigzag scan (right to left, skip 6).
  defp build_col_list(col, acc) when col < 1, do: Enum.reverse(acc)

  defp build_col_list(col, acc) do
    next_col = col - 2
    next_col = if next_col == 6, do: 5, else: next_col
    build_col_list(next_col, [col | acc])
  end

  # Scan one 2-column strip (col and col-1), going up or down.
  defp scan_col_pair(modules, reserved, bits, sz, col, going_up, bit_idx) do
    rows = if going_up, do: (sz - 1)..0//-1, else: 0..(sz - 1)

    Enum.reduce(rows, {modules, bit_idx}, fn row_idx, {m, idx} ->
      # Process the two columns in this strip: col and col-1.
      Enum.reduce([0, 1], {m, idx}, fn dc, {m2, idx2} ->
        c = col - dc

        # Skip column 6 (vertical timing strip).
        if c == 6 do
          {m2, idx2}
        else
          if get_reserved(reserved, row_idx, c) do
            # Structural module — skip (do not consume a bit).
            {m2, idx2}
          else
            # Data bit: place it and advance the bit index.
            bit = idx2 < length(bits) and Enum.at(bits, idx2) == 1
            {set_mod(m2, row_idx, c, bit), idx2 + 1}
          end
        end
      end)
    end)
  end

  # ============================================================================
  # Masking
  # ============================================================================

  # The 8 mask conditions from ISO 18004 Table 10.
  # Returns true if the module at (row, col) should be flipped (dark ↔ light).
  # Applied only to non-reserved (data/ECC) modules.
  defp mask_cond(0, r, c), do: rem(r + c, 2) == 0
  defp mask_cond(1, r, _c), do: rem(r, 2) == 0
  defp mask_cond(2, _r, c), do: rem(c, 3) == 0
  defp mask_cond(3, r, c), do: rem(r + c, 3) == 0
  defp mask_cond(4, r, c), do: rem(div(r, 2) + div(c, 3), 2) == 0
  defp mask_cond(5, r, c), do: rem(r * c, 2) + rem(r * c, 3) == 0
  defp mask_cond(6, r, c), do: rem(rem(r * c, 2) + rem(r * c, 3), 2) == 0
  defp mask_cond(7, r, c), do: rem(rem(r + c, 2) + rem(r * c, 3), 2) == 0

  # Apply mask pattern `mask_idx` (0–7) to non-reserved modules.
  # Returns a new modules grid with the mask applied.
  # Masking XORs (flips) the module if the condition is true.
  defp apply_mask(modules, reserved, _sz, mask_idx) do
    modules
    |> Enum.with_index()
    |> Enum.map(fn {row_data, row_idx} ->
      row_data
      |> Enum.with_index()
      |> Enum.map(fn {dark, col_idx} ->
        if get_reserved(reserved, row_idx, col_idx) do
          dark
        else
          # XOR the module with the mask condition result.
          dark != mask_cond(mask_idx, row_idx, col_idx)
        end
      end)
    end)
  end

  # Evaluate all 8 masks and return the index of the one with the lowest penalty.
  # We write format info before evaluating the penalty because the format modules
  # contribute to the penalty score.
  defp select_best_mask(modules, reserved, sz, ecc_level) do
    {best_mask, _best_penalty} =
      Enum.reduce(0..7, {0, :infinity}, fn mask_idx, {best_m, best_p} ->
        masked = apply_mask(modules, reserved, sz, mask_idx)
        fmt_bits = compute_format_bits(ecc_level, mask_idx)
        test_modules = write_format_info(masked, fmt_bits, sz)
        penalty = compute_penalty(test_modules, sz)

        if penalty < best_p do
          {mask_idx, penalty}
        else
          {best_m, best_p}
        end
      end)

    best_mask
  end

  # ============================================================================
  # Penalty scoring — ISO 18004 Section 7.8.3
  # ============================================================================

  # Compute the 4-rule penalty score for a (masked) module array.
  #
  # Rule 1: runs of ≥5 same-color modules in a row/col → score += run − 2
  # Rule 2: 2×2 same-color blocks → score += 3 per block
  # Rule 3: finder-like patterns → score += 40 per match
  # Rule 4: dark proportion deviation from 50% → score based on 5% steps
  defp compute_penalty(modules, sz) do
    rule1 = penalty_rule1(modules, sz)
    rule2 = penalty_rule2(modules, sz)
    rule3 = penalty_rule3(modules, sz)
    rule4 = penalty_rule4(modules, sz)
    rule1 + rule2 + rule3 + rule4
  end

  # Rule 1: Adjacent same-color runs of 5 or more in rows and columns.
  # For each run of length n ≥ 5, add (n − 2) to the penalty.
  defp penalty_rule1(modules, sz) do
    # Horizontal runs — scan each row.
    horiz =
      Enum.reduce(modules, 0, fn row_data, acc ->
        acc + run_penalty(row_data)
      end)

    # Vertical runs — scan each column (need to transpose).
    vert =
      Enum.reduce(0..(sz - 1), 0, fn col_idx, acc ->
        col_data = Enum.map(modules, &Enum.at(&1, col_idx))
        acc + run_penalty(col_data)
      end)

    horiz + vert
  end

  # Calculate the run-length penalty for one row or column.
  # Walk through the sequence, tracking current run length and value.
  # For each run of length n ≥ 5 in a row, the penalty is n − 2.
  defp run_penalty([]), do: 0

  defp run_penalty([first | rest]) do
    {penalty, run_len, _prev} =
      Enum.reduce(rest, {0, 1, first}, fn val, {pen, run_len, prev} ->
        if val == prev do
          {pen, run_len + 1, prev}
        else
          pen = if run_len >= 5, do: pen + run_len - 2, else: pen
          {pen, 1, val}
        end
      end)

    # Account for the final run at the end of the row/column.
    if run_len >= 5, do: penalty + run_len - 2, else: penalty
  end

  # Rule 2: 2×2 same-color blocks — each adds 3 to the penalty.
  defp penalty_rule2(modules, sz) do
    Enum.reduce(0..(sz - 2), 0, fn row_idx, acc ->
      row_data = Enum.at(modules, row_idx)
      next_row = Enum.at(modules, row_idx + 1)

      Enum.reduce(0..(sz - 2), acc, fn col_idx, acc2 ->
        d = Enum.at(row_data, col_idx)

        if d == Enum.at(row_data, col_idx + 1) and
             d == Enum.at(next_row, col_idx) and
             d == Enum.at(next_row, col_idx + 1) do
          acc2 + 3
        else
          acc2
        end
      end)
    end)
  end

  # Rule 3: Finder-pattern-like sequences.
  # Pattern 1: dark,light,dark,dark,dark,light,dark,light,light,light,light
  # Pattern 2: the reverse
  # Each match adds 40 to the penalty.
  @rule3_p1 [true, false, true, true, true, false, true, false, false, false, false]
  @rule3_p2 [false, false, false, false, true, false, true, true, true, false, true]

  defp penalty_rule3(modules, sz) do
    p1 = @rule3_p1
    p2 = @rule3_p2

    Enum.reduce(0..(sz - 1), 0, fn a, acc ->
      Enum.reduce(0..(sz - 11), acc, fn b, acc2 ->
        row_a = Enum.at(modules, a)

        horiz = Enum.slice(row_a, b, 11)
        vert = Enum.map(b..(b + 10), fn k -> Enum.at(Enum.at(modules, k), a) end)

        acc2 = if horiz == p1 or horiz == p2, do: acc2 + 40, else: acc2
        if vert == p1 or vert == p2, do: acc2 + 40, else: acc2
      end)
    end)
  end

  # Rule 4: Dark module ratio deviation from 50%.
  # Penalty = (steps away from 50% in 5% increments) × 10.
  defp penalty_rule4(modules, sz) do
    dark_count =
      Enum.reduce(modules, 0, fn row_data, acc ->
        acc + Enum.count(row_data, & &1)
      end)

    total = sz * sz
    ratio = dark_count / total * 100
    prev5 = trunc(ratio / 5) * 5
    min(abs(prev5 - 50), abs(prev5 + 5 - 50)) |> div(5) |> Kernel.*(10)
  end

  # ============================================================================
  # Format information
  # ============================================================================

  # Compute the 15-bit format information string.
  #
  # 1. 5-bit data = [ECC level (2b)] [mask pattern (3b)]
  # 2. BCH(15,5): remainder of (data × x^10) mod G(x), G(x) = 0x537
  # 3. Concatenate data and 10-bit remainder
  # 4. XOR with 0x5412 to prevent all-zero format info
  #
  # G(x) = x^10 + x^8 + x^5 + x^4 + x^2 + x + 1 = 0x537.
  defp compute_format_bits(ecc_level, mask) do
    data = (Tables.ecc_indicator(ecc_level) <<< 3) ||| mask
    rem_val = bch_remainder_format(data <<< 10)
    bxor((data <<< 10) ||| (rem_val &&& 0x3FF), 0x5412)
  end

  # Compute BCH(15,5) remainder: divide `val` by 0x537 via long division.
  defp bch_remainder_format(val) do
    Enum.reduce(14..10//-1, val, fn i, acc ->
      if ((acc >>> i) &&& 1) == 1 do
        bxor(acc, 0x537 <<< (i - 10))
      else
        acc
      end
    end)
  end

  # Write 15-bit format information into both copy locations.
  #
  # CRITICAL (from lessons.md 2026-04-23): The bit ordering must be MSB-first
  # in row 8 (f14 at col 0) and specific per-position ordering elsewhere.
  # The wrong bit ordering causes the format info BCH check to fail, making
  # the QR code completely unscannable.
  #
  # Copy 1:
  #   row 8, cols 0–5:  f14 at col 0, f13 at col 1, ..., f9 at col 5
  #   (8,7) = f8,  (8,8) = f7
  #   (7,8) = f6
  #   col 8, rows 5–0:  f5 at row 5, ..., f0 at row 0
  #
  # Copy 2:
  #   col 8, rows sz-1..sz-7: f0 at row sz-1, ..., f6 at row sz-7
  #   row 8, cols sz-8..sz-1: f7 at col sz-8, ..., f14 at col sz-1
  defp write_format_info(modules, fmt_bits, sz) do
    # Copy 1 — row 8, cols 0..5 (f14 → f9, MSB-first)
    modules =
      Enum.reduce(0..5, modules, fn i, m ->
        set_mod(m, 8, i, ((fmt_bits >>> (14 - i)) &&& 1) == 1)
      end)

    # Copy 1 — (8,7) = f8, (8,8) = f7, (7,8) = f6
    modules = set_mod(modules, 8, 7, ((fmt_bits >>> 8) &&& 1) == 1)
    modules = set_mod(modules, 8, 8, ((fmt_bits >>> 7) &&& 1) == 1)
    modules = set_mod(modules, 7, 8, ((fmt_bits >>> 6) &&& 1) == 1)

    # Copy 1 — col 8, rows 5..0 (f5 at row 5, f0 at row 0)
    modules =
      Enum.reduce(0..5, modules, fn i, m ->
        set_mod(m, 5 - i, 8, ((fmt_bits >>> i) &&& 1) == 1)
      end)

    # Copy 2 — col 8, rows sz-1..sz-7 (f0 at row sz-1, ..., f6 at row sz-7)
    modules =
      Enum.reduce(0..6, modules, fn i, m ->
        set_mod(m, sz - 1 - i, 8, ((fmt_bits >>> i) &&& 1) == 1)
      end)

    # Copy 2 — row 8, cols sz-8..sz-1 (f7 at col sz-8, ..., f14 at col sz-1)
    Enum.reduce(7..14, modules, fn i, m ->
      set_mod(m, 8, sz - 15 + i, ((fmt_bits >>> i) &&& 1) == 1)
    end)
  end

  # ============================================================================
  # Version information (v7+)
  # ============================================================================

  # Compute 18-bit version information (v7+).
  #
  # 1. 6-bit version number
  # 2. BCH(18,6): remainder of (version × x^12) mod G(x), G(x) = 0x1F25
  # 3. Concatenate for 18 bits
  #
  # G(x) = x^12+x^11+x^10+x^9+x^8+x^5+x^2+1 = 0x1F25.
  defp compute_version_bits(ver) do
    initial = ver <<< 12

    rem_val =
      Enum.reduce(17..12//-1, initial, fn i, acc ->
        if ((acc >>> i) &&& 1) == 1 do
          bxor(acc, 0x1F25 <<< (i - 12))
        else
          acc
        end
      end)

    (ver <<< 12) ||| (rem_val &&& 0xFFF)
  end

  # Write version information into both 6×3 blocks (v7+).
  #
  # Top-right block:   bit i → (5 − ⌊i/3⌋, sz−9−(i rem 3))
  # Bottom-left block: bit i → (sz−9−(i rem 3), 5−⌊i/3⌋)
  defp write_version_info(modules, ver, _sz) when ver < 7, do: modules

  defp write_version_info(modules, ver, sz) do
    bits = compute_version_bits(ver)

    Enum.reduce(0..17, modules, fn i, m ->
      dark = ((bits >>> i) &&& 1) == 1
      a = 5 - div(i, 3)
      b = sz - 9 - rem(i, 3)
      m = set_mod(m, a, b, dark)
      set_mod(m, b, a, dark)
    end)
  end

  # ============================================================================
  # Grid utility helpers
  # ============================================================================

  # Set the boolean value of module at (row, col) in the modules grid.
  # Returns the updated grid (a new list-of-lists, immutable).
  defp set_mod(modules, row, col, dark) do
    row_data = Enum.at(modules, row)
    new_row = List.replace_at(row_data, col, dark)
    List.replace_at(modules, row, new_row)
  end

  # Mark position (row, col) as reserved in the reserved grid.
  defp set_reserved(reserved, row, col) do
    row_data = Enum.at(reserved, row)
    new_row = List.replace_at(row_data, col, true)
    List.replace_at(reserved, row, new_row)
  end

  # Check whether position (row, col) is reserved.
  defp get_reserved(reserved, row, col) do
    reserved
    |> Enum.at(row)
    |> Enum.at(col)
  end
end
