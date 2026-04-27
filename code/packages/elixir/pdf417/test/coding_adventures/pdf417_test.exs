defmodule CodingAdventures.PDF417Test do
  use ExUnit.Case, async: true

  alias CodingAdventures.PDF417
  alias CodingAdventures.PDF417.ModuleGrid

  # ===========================================================================
  # Version
  # ===========================================================================

  describe "version/0" do
    test "returns the package version string" do
      assert PDF417.version() == "0.1.0"
    end
  end

  # ===========================================================================
  # Basic encode/1 and encode/2 API
  # ===========================================================================

  describe "encode/1 basic success" do
    test "encode Hello returns {:ok, %ModuleGrid{}}" do
      assert {:ok, %ModuleGrid{} = grid} = PDF417.encode("Hello")
      assert grid.rows >= 3
      assert grid.cols >= 1
    end

    test "encode empty string succeeds" do
      assert {:ok, %ModuleGrid{} = grid} = PDF417.encode("")
      assert grid.rows >= 3
      assert grid.cols >= 1
    end

    test "modules is a list of rows" do
      {:ok, grid} = PDF417.encode("Hello")
      assert is_list(grid.modules)
    end

    test "each row is a list of booleans" do
      {:ok, grid} = PDF417.encode("Hello")

      Enum.each(grid.modules, fn row ->
        assert is_list(row)
        Enum.each(row, fn m -> assert is_boolean(m) end)
      end)
    end

    test "number of module rows matches grid.rows" do
      {:ok, grid} = PDF417.encode("Hello")
      assert length(grid.modules) == grid.rows
    end

    test "first module row length matches grid.cols" do
      {:ok, grid} = PDF417.encode("Hello, World!")
      assert length(hd(grid.modules)) == grid.cols
    end

    test "all rows have the same width" do
      {:ok, grid} = PDF417.encode("Hello, World!")
      widths = Enum.map(grid.modules, &length/1)
      assert Enum.uniq(widths) == [grid.cols]
    end
  end

  # ===========================================================================
  # Struct fields
  # ===========================================================================

  describe "ModuleGrid struct fields" do
    test "rows and cols are positive integers" do
      {:ok, grid} = PDF417.encode("test")
      assert is_integer(grid.rows) and grid.rows > 0
      assert is_integer(grid.cols) and grid.cols > 0
    end

    test "rows is multiple of row_height (default 3)" do
      # With default row_height=3, rows must be divisible by 3 because we have
      # at least 3 logical rows (min_rows=3) each repeated 3 times = 9.
      {:ok, grid} = PDF417.encode("short")
      assert rem(grid.rows, 3) == 0
    end
  end

  # ===========================================================================
  # Determinism
  # ===========================================================================

  describe "determinism" do
    test "same input produces identical output" do
      {:ok, grid1} = PDF417.encode("Hello, World!")
      {:ok, grid2} = PDF417.encode("Hello, World!")
      assert grid1 == grid2
    end

    test "deterministic with options" do
      opts = [ecc_level: 3, columns: 5, row_height: 4]
      {:ok, grid1} = PDF417.encode("abc123", opts)
      {:ok, grid2} = PDF417.encode("abc123", opts)
      assert grid1 == grid2
    end
  end

  # ===========================================================================
  # Input size scaling
  # ===========================================================================

  describe "input size scaling" do
    test "larger input produces larger or equal area" do
      {:ok, small} = PDF417.encode("A")
      {:ok, large} = PDF417.encode(String.duplicate("A", 200))

      small_area = small.rows * small.cols
      large_area = large.rows * large.cols
      assert large_area >= small_area
    end

    test "long binary input encodes successfully" do
      data = :binary.copy(<<0xAB>>, 500)
      assert {:ok, %ModuleGrid{}} = PDF417.encode(data)
    end
  end

  # ===========================================================================
  # ECC level option
  # ===========================================================================

  describe "ecc_level option" do
    test "all ECC levels 0..8 produce valid grids" do
      for level <- 0..8 do
        result = PDF417.encode("test data", ecc_level: level)
        assert {:ok, %ModuleGrid{} = grid} = result,
               "Expected success for ecc_level #{level}, got #{inspect(result)}"

        assert grid.rows >= 3
        assert grid.cols >= 1
      end
    end

    test "higher ECC level produces larger or equal area for same input" do
      {:ok, low} = PDF417.encode("Hello World", ecc_level: 0)
      {:ok, high} = PDF417.encode("Hello World", ecc_level: 5)

      low_area = low.rows * low.cols
      high_area = high.rows * high.cols
      assert high_area >= low_area
    end

    test "ecc_level :auto succeeds" do
      assert {:ok, %ModuleGrid{}} = PDF417.encode("auto ecc test", ecc_level: :auto)
    end

    test "ecc_level out of range returns error" do
      assert {:error, :invalid_ecc_level} = PDF417.encode("Hi", ecc_level: 9)
      assert {:error, :invalid_ecc_level} = PDF417.encode("Hi", ecc_level: -1)
    end

    test "ecc_level non-integer returns error" do
      assert {:error, :invalid_ecc_level} = PDF417.encode("Hi", ecc_level: "high")
    end
  end

  # ===========================================================================
  # Columns option
  # ===========================================================================

  describe "columns option" do
    test "specifying columns=1 succeeds" do
      assert {:ok, %ModuleGrid{}} = PDF417.encode("abc", columns: 1)
    end

    test "specifying columns=10 succeeds" do
      assert {:ok, %ModuleGrid{} = grid} = PDF417.encode("Hello World", columns: 10)
      # Width = 69 + 17*10 = 239 modules
      assert grid.cols == 239
    end

    test "specifying columns=30 succeeds" do
      assert {:ok, %ModuleGrid{}} = PDF417.encode("test", columns: 30)
    end

    test "columns out of range returns error" do
      assert {:error, :invalid_columns} = PDF417.encode("Hi", columns: 0)
      assert {:error, :invalid_columns} = PDF417.encode("Hi", columns: 31)
    end

    test "symbol width formula is 69 + 17*cols" do
      for cols <- [1, 5, 10, 20, 30] do
        {:ok, grid} = PDF417.encode("hello world test", columns: cols)
        expected_width = 69 + 17 * cols
        assert grid.cols == expected_width,
               "Expected width #{expected_width} for cols=#{cols}, got #{grid.cols}"
      end
    end
  end

  # ===========================================================================
  # Row height option
  # ===========================================================================

  describe "row_height option" do
    test "row_height=1 produces minimal height" do
      {:ok, grid1} = PDF417.encode("test", row_height: 1)
      {:ok, grid3} = PDF417.encode("test", row_height: 3)
      # row_height=3 should be exactly 3x taller in terms of module rows
      assert grid3.rows == grid1.rows * 3
    end

    test "row_height=5 produces correct height" do
      {:ok, grid1} = PDF417.encode("hi", row_height: 1)
      {:ok, grid5} = PDF417.encode("hi", row_height: 5)
      assert grid5.rows == grid1.rows * 5
    end

    test "invalid row_height falls back to default 3" do
      # Invalid row_height is silently clamped to 3 in our validation.
      {:ok, grid_default} = PDF417.encode("test")
      {:ok, grid_invalid} = PDF417.encode("test", row_height: -1)
      assert grid_default.rows == grid_invalid.rows
    end
  end

  # ===========================================================================
  # Module content (basic correctness checks)
  # ===========================================================================

  describe "module content" do
    test "first row starts with a dark module (start pattern begins with a bar)" do
      {:ok, grid} = PDF417.encode("A")
      first_row = hd(grid.modules)
      assert hd(first_row) == true
    end

    test "symbol contains both dark and light modules" do
      {:ok, grid} = PDF417.encode("Hello World")
      all_modules = List.flatten(grid.modules)
      assert Enum.member?(all_modules, true)
      assert Enum.member?(all_modules, false)
    end
  end

  # ===========================================================================
  # GF(929) arithmetic
  # ===========================================================================

  describe "gf_mul/2" do
    test "zero times anything is zero" do
      assert PDF417.gf_mul(0, 500) == 0
      assert PDF417.gf_mul(300, 0) == 0
    end

    test "one is the multiplicative identity" do
      assert PDF417.gf_mul(1, 42) == 42
      assert PDF417.gf_mul(42, 1) == 42
    end

    test "multiplication is commutative" do
      assert PDF417.gf_mul(3, 5) == PDF417.gf_mul(5, 3)
      assert PDF417.gf_mul(100, 200) == PDF417.gf_mul(200, 100)
    end

    test "alpha^1 is 3 and alpha^2 is 9" do
      # alpha = 3, so alpha^1 = 3, alpha^2 = 3*3 = 9
      assert PDF417.gf_mul(3, 1) == 3
      assert PDF417.gf_mul(3, 3) == 9
    end

    test "field closure: result is always in 0..928" do
      for a <- [1, 7, 42, 100, 500, 928] do
        for b <- [1, 3, 928] do
          result = PDF417.gf_mul(a, b)
          assert result >= 0 and result <= 928
        end
      end
    end
  end

  describe "gf_add/2" do
    test "zero is the additive identity" do
      assert PDF417.gf_add(0, 42) == 42
      assert PDF417.gf_add(42, 0) == 42
    end

    test "addition wraps at 929" do
      assert PDF417.gf_add(928, 1) == 0
      assert PDF417.gf_add(900, 100) == 71
    end

    test "commutativity" do
      assert PDF417.gf_add(3, 100) == PDF417.gf_add(100, 3)
    end
  end

  # ===========================================================================
  # Reed-Solomon generator polynomial
  # ===========================================================================

  describe "build_generator/1" do
    test "degree of generator for ECC level L is 2^(L+1)" do
      for level <- 0..4 do
        k = Integer.pow(2, level + 1)
        gen = PDF417.build_generator(level)
        # A degree-k polynomial has k+1 coefficients.
        assert length(gen) == k + 1,
               "Expected #{k + 1} coefficients for level #{level}, got #{length(gen)}"
      end
    end

    test "leading coefficient is always 1 (monic)" do
      for level <- 0..4 do
        gen = PDF417.build_generator(level)
        assert hd(gen) == 1
      end
    end

    test "all coefficients are in 0..928" do
      gen = PDF417.build_generator(2)

      Enum.each(gen, fn c ->
        assert c >= 0 and c <= 928
      end)
    end
  end

  # ===========================================================================
  # Reed-Solomon encoder
  # ===========================================================================

  describe "rs_encode/2" do
    test "returns the correct number of ECC codewords" do
      for level <- 0..4 do
        k = Integer.pow(2, level + 1)
        ecc = PDF417.rs_encode([1, 924, 72, 101, 108], level)
        assert length(ecc) == k
      end
    end

    test "ECC codewords are all in 0..928" do
      ecc = PDF417.rs_encode([5, 924, 65], 2)

      Enum.each(ecc, fn c ->
        assert c >= 0 and c <= 928
      end)
    end

    test "different data produces different ECC" do
      ecc1 = PDF417.rs_encode([1, 924, 72], 1)
      ecc2 = PDF417.rs_encode([1, 924, 99], 1)
      refute ecc1 == ecc2
    end
  end

  # ===========================================================================
  # Byte compaction
  # ===========================================================================

  describe "byte_compact/1" do
    test "always starts with the latch codeword 924" do
      cws = PDF417.byte_compact([72, 101, 108, 108, 111])
      assert hd(cws) == 924
    end

    test "6 bytes -> 5 codewords (plus latch)" do
      cws = PDF417.byte_compact([0, 1, 2, 3, 4, 5])
      # 1 latch + 5 data codewords
      assert length(cws) == 6
    end

    test "12 bytes -> 10 codewords (plus latch)" do
      cws = PDF417.byte_compact(List.duplicate(65, 12))
      assert length(cws) == 11
    end

    test "1 tail byte -> 1 codeword (plus latch)" do
      cws = PDF417.byte_compact([42])
      assert length(cws) == 2
      assert Enum.at(cws, 1) == 42
    end

    test "all codewords except latch are in 0..928" do
      cws = PDF417.byte_compact(Enum.to_list(0..255))
      [_latch | rest] = cws

      Enum.each(rest, fn c ->
        assert c >= 0 and c <= 928
      end)
    end
  end

  # ===========================================================================
  # Auto ECC level
  # ===========================================================================

  describe "auto_ecc_level/1" do
    test "small data (<=40) gets level 2" do
      assert PDF417.auto_ecc_level(10) == 2
      assert PDF417.auto_ecc_level(40) == 2
    end

    test "medium data (41..160) gets level 3" do
      assert PDF417.auto_ecc_level(41) == 3
      assert PDF417.auto_ecc_level(160) == 3
    end

    test "larger data (161..320) gets level 4" do
      assert PDF417.auto_ecc_level(161) == 4
      assert PDF417.auto_ecc_level(320) == 4
    end

    test "large data (321..863) gets level 5" do
      assert PDF417.auto_ecc_level(321) == 5
      assert PDF417.auto_ecc_level(863) == 5
    end

    test "very large data (>863) gets level 6" do
      assert PDF417.auto_ecc_level(864) == 6
      assert PDF417.auto_ecc_level(9999) == 6
    end
  end

  # ===========================================================================
  # Dimension selection
  # ===========================================================================

  describe "choose_dimensions/1" do
    test "result rows is in 3..90" do
      for total <- [1, 10, 50, 200, 500] do
        {_cols, rows} = PDF417.choose_dimensions(total)
        assert rows >= 3 and rows <= 90
      end
    end

    test "result cols is in 1..30" do
      for total <- [1, 10, 50, 200, 500] do
        {cols, _rows} = PDF417.choose_dimensions(total)
        assert cols >= 1 and cols <= 30
      end
    end

    test "capacity is at least total" do
      for total <- [1, 5, 20, 100, 300] do
        {cols, rows} = PDF417.choose_dimensions(total)
        assert cols * rows >= total
      end
    end
  end

  # ===========================================================================
  # Row indicators
  # ===========================================================================

  describe "compute_lri/4 and compute_rri/4" do
    test "values are non-negative integers" do
      for r <- 0..8 do
        lri = PDF417.compute_lri(r, 9, 3, 2)
        rri = PDF417.compute_rri(r, 9, 3, 2)
        assert is_integer(lri) and lri >= 0
        assert is_integer(rri) and rri >= 0
      end
    end

    test "values are within codeword range 0..928" do
      for r <- 0..89 do
        lri = PDF417.compute_lri(r, 90, 30, 8)
        rri = PDF417.compute_rri(r, 90, 30, 8)
        assert lri <= 928
        assert rri <= 928
      end
    end

    test "cluster 0 LRI encodes row info" do
      # Cluster 0 (r=0): LRI = 30*0 + r_info = (rows-1) div 3
      lri = PDF417.compute_lri(0, 9, 3, 2)
      r_info = div(9 - 1, 3)
      assert lri == 30 * 0 + r_info
    end

    test "cluster 0 RRI encodes column info" do
      # Cluster 0 (r=0): RRI = 30*0 + c_info = cols - 1
      rri = PDF417.compute_rri(0, 9, 3, 2)
      c_info = 3 - 1
      assert rri == 30 * 0 + c_info
    end
  end

  # ===========================================================================
  # Pattern expansion
  # ===========================================================================

  describe "expand_widths/1" do
    test "starts with dark (bar)" do
      modules = PDF417.expand_widths([2, 1, 1])
      assert hd(modules) == true
    end

    test "total module count equals sum of widths" do
      widths = [8, 1, 1, 1, 1, 1, 1, 3]
      modules = PDF417.expand_widths(widths)
      assert length(modules) == Enum.sum(widths)
    end

    test "alternates dark and light by width" do
      # [2, 3] -> [true, true, false, false, false]
      modules = PDF417.expand_widths([2, 3])
      assert modules == [true, true, false, false, false]
    end

    test "start pattern expands to 17 modules" do
      # Start pattern: [8, 1, 1, 1, 1, 1, 1, 3] = 17
      alias CodingAdventures.PDF417.ClusterTables
      modules = PDF417.expand_widths(ClusterTables.start_pattern())
      assert length(modules) == 17
    end

    test "stop pattern expands to 18 modules" do
      # Stop pattern: [7, 1, 1, 3, 1, 1, 1, 2, 1] = 18
      alias CodingAdventures.PDF417.ClusterTables
      modules = PDF417.expand_widths(ClusterTables.stop_pattern())
      assert length(modules) == 18
    end
  end

  # ===========================================================================
  # Binary byte list input
  # ===========================================================================

  describe "encode with byte list input" do
    test "byte list input works like string input" do
      str = "Hello"
      bytes = :binary.bin_to_list(str)
      {:ok, grid_str} = PDF417.encode(str)
      {:ok, grid_bytes} = PDF417.encode(bytes)
      assert grid_str == grid_bytes
    end

    test "byte list with values 0..255 succeeds" do
      bytes = Enum.to_list(0..255)
      assert {:ok, %ModuleGrid{}} = PDF417.encode(bytes)
    end

    test "invalid byte value in list returns error" do
      assert {:error, :invalid_data} = PDF417.encode([0, 300, 42])
      assert {:error, :invalid_data} = PDF417.encode([0, -1, 42])
    end

    test "non-list non-binary returns error" do
      assert {:error, :invalid_data} = PDF417.encode(42)
      assert {:error, :invalid_data} = PDF417.encode(:atom)
    end
  end

  # ===========================================================================
  # Edge cases
  # ===========================================================================

  describe "edge cases" do
    test "single null byte succeeds" do
      assert {:ok, %ModuleGrid{}} = PDF417.encode(<<0>>)
    end

    test "binary with all 0xFF bytes succeeds" do
      data = :binary.copy(<<0xFF>>, 20)
      assert {:ok, %ModuleGrid{}} = PDF417.encode(data)
    end

    test "columns=1 produces a very tall symbol" do
      {:ok, grid} = PDF417.encode("Hello World", columns: 1)
      # With 1 column the symbol must have many more logical rows than with auto.
      {:ok, grid_auto} = PDF417.encode("Hello World")
      # Area should be similar but grid.rows should be larger for 1-column layout.
      assert grid.rows >= grid_auto.rows
    end
  end
end
