defmodule CodingAdventures.MicroQRTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.MicroQR
  alias CodingAdventures.Barcode2D.ModuleGrid

  # ============================================================================
  # Version module (versioning smoke test)
  # ============================================================================

  test "version/0 returns a version string" do
    assert is_binary(MicroQR.version())
    assert MicroQR.version() == "0.1.0"
  end

  # ============================================================================
  # Symbol dimensions — each version must produce the correct grid size
  # ============================================================================

  describe "grid dimensions" do
    test "M1 encodes to 11×11" do
      {:ok, grid} = MicroQR.encode("1")
      assert grid.rows == 11
      assert grid.cols == 11
      assert length(grid.modules) == 11
      assert length(List.first(grid.modules)) == 11
    end

    test "M2 encodes to 13×13" do
      {:ok, grid} = MicroQR.encode("HELLO")
      assert grid.rows == 13
      assert grid.cols == 13
    end

    test "M3 encodes to 15×15" do
      {:ok, grid} = MicroQR.encode("MICRO QR TEST")
      assert grid.rows == 15
      assert grid.cols == 15
    end

    test "M4 encodes to 17×17" do
      {:ok, grid} = MicroQR.encode("https://a.b")
      assert grid.rows == 17
      assert grid.cols == 17
    end

    test "forced M4 for single digit gives 17×17" do
      {:ok, grid} = MicroQR.encode("1", :m4, nil)
      assert grid.rows == 17
    end
  end

  # ============================================================================
  # Auto-version selection
  # ============================================================================

  describe "auto-version selection" do
    test "single digit auto-selects M1" do
      {:ok, grid} = MicroQR.encode("1")
      assert grid.rows == 11
    end

    test "12345 fits in M1 (max 5 numeric)" do
      {:ok, grid} = MicroQR.encode("12345")
      assert grid.rows == 11
    end

    test "6 digits falls through to M2" do
      {:ok, grid} = MicroQR.encode("123456")
      assert grid.rows == 13
    end

    test "HELLO is alphanumeric → M2" do
      {:ok, grid} = MicroQR.encode("HELLO")
      assert grid.rows == 13
    end

    test "lowercase hello is byte mode → M3-L (byte cap 9)" do
      {:ok, grid} = MicroQR.encode("hello")
      assert grid.rows >= 15
    end

    test "URL auto-selects M4" do
      {:ok, grid} = MicroQR.encode("https://a.b")
      assert grid.rows == 17
    end

    test "8 numeric digits → M2" do
      {:ok, grid} = MicroQR.encode("01234567")
      assert grid.rows == 13
    end

    test "6-char alphanumeric A1B2C3 → M2-L" do
      {:ok, grid} = MicroQR.encode("A1B2C3")
      assert grid.rows == 13
    end

    test "13 alphanumeric chars MICRO QR TEST → M3" do
      {:ok, grid} = MicroQR.encode("MICRO QR TEST")
      assert grid.rows == 15
    end
  end

  # ============================================================================
  # Numeric encoding in all versions
  # ============================================================================

  describe "numeric encoding" do
    test "encodes single digit in M1" do
      assert {:ok, grid} = MicroQR.encode("5")
      assert grid.rows == 11
    end

    test "encodes at M1 limit (5 digits)" do
      assert {:ok, grid} = MicroQR.encode("12345")
      assert grid.rows == 11
    end

    test "encodes numeric at M2 version explicitly" do
      assert {:ok, grid} = MicroQR.encode("12345678", :m2, :l)
      assert grid.rows == 13
    end

    test "encodes numeric at M3 version explicitly" do
      assert {:ok, grid} = MicroQR.encode("1234567890123", :m3, :l)
      assert grid.rows == 15
    end

    test "encodes numeric at M4 version explicitly" do
      assert {:ok, grid} = MicroQR.encode("12345678901234567", :m4, :l)
      assert grid.rows == 17
    end
  end

  # ============================================================================
  # Alphanumeric encoding in M2-M4
  # ============================================================================

  describe "alphanumeric encoding" do
    test "HELLO encodes in M2" do
      assert {:ok, grid} = MicroQR.encode("HELLO", :m2, :l)
      assert grid.rows == 13
    end

    test "alphanumeric forced to M3-L" do
      assert {:ok, grid} = MicroQR.encode("HELLO WORLD", :m3, :l)
      assert grid.rows == 15
    end

    test "alphanumeric forced to M4-L" do
      assert {:ok, grid} = MicroQR.encode("MICRO QR", :m4, :l)
      assert grid.rows == 17
    end

    test "alphanumeric fails for M1" do
      # M1 only supports numeric; HELLO cannot fit
      assert {:error, reason} = MicroQR.encode("HELLO", :m1, nil)
      assert String.contains?(reason, "does not fit")
    end
  end

  # ============================================================================
  # Byte encoding in M3-M4
  # ============================================================================

  describe "byte encoding" do
    test "lowercase string in M3-L" do
      assert {:ok, grid} = MicroQR.encode("hello", :m3, :l)
      assert grid.rows == 15
    end

    test "URL in M4-L" do
      assert {:ok, grid} = MicroQR.encode("https://a.b", :m4, :l)
      assert grid.rows == 17
    end

    test "byte mode forced for non-alphanumeric in M4" do
      assert {:ok, grid} = MicroQR.encode("hello!", :m4, :l)
      assert grid.rows == 17
    end

    test "byte mode for mixed lowercase in M4-M" do
      assert {:ok, grid} = MicroQR.encode("Hello", :m4, :m)
      assert grid.rows == 17
    end
  end

  # ============================================================================
  # ECC level selection
  # ============================================================================

  describe "ECC levels" do
    test "M1 detection-only ECC" do
      assert {:ok, grid} = MicroQR.encode("1", :m1, :detection)
      assert grid.rows == 11
    end

    test "M2-L and M2-M produce different grids" do
      {:ok, grid_l} = MicroQR.encode("HELLO", nil, :l)
      {:ok, grid_m} = MicroQR.encode("HELLO", nil, :m)
      assert grid_l.modules != grid_m.modules
    end

    test "M4-L, M4-M, M4-Q all produce 17×17" do
      {:ok, gl} = MicroQR.encode("MICRO QR", :m4, :l)
      {:ok, gm} = MicroQR.encode("MICRO QR", :m4, :m)
      {:ok, gq} = MicroQR.encode("MICRO QR", :m4, :q)
      assert gl.rows == 17
      assert gm.rows == 17
      assert gq.rows == 17
    end

    test "M4-L, M4-M, M4-Q produce different grids (different format info)" do
      {:ok, gl} = MicroQR.encode("MICRO QR", :m4, :l)
      {:ok, gm} = MicroQR.encode("MICRO QR", :m4, :m)
      {:ok, gq} = MicroQR.encode("MICRO QR", :m4, :q)
      assert gl.modules != gm.modules
      assert gm.modules != gq.modules
    end

    test "Q level not available for M2 → falls back to M4-Q or error" do
      # Requesting M2-Q should fail since Q is only for M4
      assert {:error, _reason} = MicroQR.encode("HI", :m2, :q)
    end

    test "Q level not available for M3" do
      assert {:error, _reason} = MicroQR.encode("HI", :m3, :q)
    end
  end

  # ============================================================================
  # Grid structure — structural module checks
  # ============================================================================

  describe "grid structure" do
    test "top-left 7x7 is the finder pattern" do
      # Finder pattern rows 0-6, cols 0-6.
      # Outer border (row/col 0 or 6) = dark, inner ring = light, core 3x3 = dark.
      {:ok, grid} = MicroQR.encode("1")

      # Check outer border: row 0 cols 0-6 all dark
      row0 = Enum.take(List.first(grid.modules), 7)
      assert Enum.all?(row0, fn m -> m == true end)

      # Check row 1, cols 0 and 6 are dark (borders)
      row1 = Enum.at(grid.modules, 1)
      assert Enum.at(row1, 0) == true
      assert Enum.at(row1, 6) == true

      # Check row 1, cols 1-5 are light (inner ring)
      assert Enum.at(row1, 1) == false
      assert Enum.at(row1, 5) == false

      # Check core: row 2, col 2 is dark
      row2 = Enum.at(grid.modules, 2)
      assert Enum.at(row2, 2) == true
    end

    test "separator row 7 cols 0-7 are light" do
      {:ok, grid} = MicroQR.encode("1")
      row7 = Enum.at(grid.modules, 7)
      sep_modules = Enum.take(row7, 8)
      assert Enum.all?(sep_modules, fn m -> m == false end)
    end

    test "separator col 7 rows 0-7 are light" do
      {:ok, grid} = MicroQR.encode("HELLO")
      col7 = Enum.map(Enum.take(grid.modules, 8), fn row -> Enum.at(row, 7) end)
      assert Enum.all?(col7, fn m -> m == false end)
    end

    test "timing row 0 beyond separator alternates dark/light" do
      {:ok, grid} = MicroQR.encode("HELLO")  # 13×13
      row0 = List.first(grid.modules)
      # Position 8 should be dark (even index), position 9 should be light
      assert Enum.at(row0, 8) == true   # col 8: even, dark
      assert Enum.at(row0, 9) == false  # col 9: odd, light
      assert Enum.at(row0, 10) == true  # col 10: even, dark
    end

    test "timing col 0 beyond separator alternates dark/light" do
      {:ok, grid} = MicroQR.encode("HELLO")  # 13×13
      col0 = Enum.map(grid.modules, fn row -> List.first(row) end)
      assert Enum.at(col0, 8) == true   # row 8: even, dark
      assert Enum.at(col0, 9) == false  # row 9: odd, light
      assert Enum.at(col0, 10) == true  # row 10: even, dark
    end

    test "module_shape is :square" do
      {:ok, grid} = MicroQR.encode("1")
      assert grid.module_shape == :square
    end
  end

  # ============================================================================
  # Error cases
  # ============================================================================

  describe "error cases" do
    test "input too long for any symbol returns error" do
      # M4-L numeric max is 35 chars; 36 digits should fail
      long_input = String.duplicate("1", 36)
      assert {:error, reason} = MicroQR.encode(long_input)
      assert String.contains?(reason, "does not fit")
    end

    test "alphanumeric input too long returns error" do
      # M4-L alpha max is 21; 22 uppercase chars fail
      long_alpha = String.duplicate("A", 22)
      assert {:error, reason} = MicroQR.encode(long_alpha)
      assert String.contains?(reason, "does not fit")
    end

    test "invalid version/ECC combo returns error" do
      assert {:error, reason} = MicroQR.encode("1", :m1, :l)
      # M1 only supports :detection
      assert String.contains?(reason, "does not fit") or
             String.contains?(reason, "ECCNotAvailable")
    end
  end

  # ============================================================================
  # encode!/3 — bang variant
  # ============================================================================

  describe "encode!/3" do
    test "returns grid on success" do
      grid = MicroQR.encode!("1")
      assert %ModuleGrid{} = grid
      assert grid.rows == 11
    end

    test "raises RuntimeError on failure" do
      long_input = String.duplicate("1", 36)
      assert_raise RuntimeError, fn ->
        MicroQR.encode!(long_input)
      end
    end
  end

  # ============================================================================
  # layout_grid/2
  # ============================================================================

  describe "layout_grid/2" do
    test "returns a PaintScene with 2-module quiet zone" do
      {:ok, grid} = MicroQR.encode("1")  # 11×11
      {:ok, scene} = MicroQR.layout_grid(grid)
      # Total size: (11 + 2*2) * 10 = 150 pixels
      assert scene.width == 150.0
      assert scene.height == 150.0
    end

    test "scene has at least 2 instructions (background + dark modules)" do
      {:ok, grid} = MicroQR.encode("1")
      {:ok, scene} = MicroQR.layout_grid(grid)
      assert length(scene.instructions) >= 2
    end
  end

  # ============================================================================
  # encode_and_layout/4
  # ============================================================================

  describe "encode_and_layout/4" do
    test "encodes and lays out in one step" do
      assert {:ok, scene} = MicroQR.encode_and_layout("HELLO")
      assert scene.width > 0
      assert scene.height > 0
    end

    test "returns error for too-long input" do
      long_input = String.duplicate("1", 36)
      assert {:error, _reason} = MicroQR.encode_and_layout(long_input)
    end
  end

  # ============================================================================
  # Cross-version consistency — same input produces same grid regardless of path
  # ============================================================================

  describe "determinism" do
    test "same input always produces same grid" do
      {:ok, g1} = MicroQR.encode("HELLO")
      {:ok, g2} = MicroQR.encode("HELLO")
      assert g1.modules == g2.modules
    end

    test "different inputs produce different grids" do
      {:ok, g1} = MicroQR.encode("12345")
      {:ok, g2} = MicroQR.encode("12346")
      assert g1.modules != g2.modules
    end
  end

  # ============================================================================
  # RS encoder correctness
  # ============================================================================

  describe "RS encoder" do
    # This test exercises the internal RS encoder indirectly by encoding a
    # known input at M1 and verifying the grid is valid (non-trivial).
    test "M1 encodes 123 to a valid non-empty grid" do
      {:ok, grid} = MicroQR.encode("123")
      dark_count = grid.modules |> Enum.concat() |> Enum.count(fn d -> d end)
      assert dark_count > 0
      assert dark_count < 11 * 11
    end
  end

  # ============================================================================
  # Test corpus — cross-language reference inputs
  # ============================================================================

  describe "test corpus (cross-language reference)" do
    test "corpus: 1 → M1 (11×11)" do
      {:ok, g} = MicroQR.encode("1")
      assert g.rows == 11
    end

    test "corpus: 12345 → M1 (11×11)" do
      {:ok, g} = MicroQR.encode("12345")
      assert g.rows == 11
    end

    test "corpus: HELLO → M2 (13×13)" do
      {:ok, g} = MicroQR.encode("HELLO")
      assert g.rows == 13
    end

    test "corpus: 01234567 → M2 (13×13)" do
      {:ok, g} = MicroQR.encode("01234567")
      assert g.rows == 13
    end

    test "corpus: https://a.b → M4 (17×17)" do
      {:ok, g} = MicroQR.encode("https://a.b")
      assert g.rows == 17
    end

    test "corpus: MICRO QR TEST → M3 (15×15)" do
      {:ok, g} = MicroQR.encode("MICRO QR TEST")
      assert g.rows == 15
    end
  end
end
