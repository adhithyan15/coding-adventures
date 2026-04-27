defmodule CodingAdventures.Barcode2DTest do
  use ExUnit.Case

  alias CodingAdventures.Barcode2D
  alias CodingAdventures.Barcode2D.ModuleGrid
  alias CodingAdventures.Barcode2D.Barcode2DLayoutConfig

  # ============================================================================
  # version/0
  # ============================================================================

  describe "version/0" do
    test "returns the package version string" do
      assert Barcode2D.version() == "0.1.0"
    end
  end

  # ============================================================================
  # make_module_grid/3
  # ============================================================================

  describe "make_module_grid/3" do
    test "creates a grid with correct dimensions" do
      grid = Barcode2D.make_module_grid(5, 7)

      assert grid.rows == 5
      assert grid.cols == 7
    end

    test "all modules start as false (light)" do
      grid = Barcode2D.make_module_grid(3, 4)

      for row <- grid.modules do
        for module <- row do
          assert module == false
        end
      end
    end

    test "default module_shape is :square" do
      grid = Barcode2D.make_module_grid(3, 3)

      assert grid.module_shape == :square
    end

    test "accepts :hex module_shape" do
      grid = Barcode2D.make_module_grid(33, 30, :hex)

      assert grid.module_shape == :hex
      assert grid.rows == 33
      assert grid.cols == 30
    end

    test "grid is a ModuleGrid struct" do
      grid = Barcode2D.make_module_grid(2, 2)

      assert %ModuleGrid{} = grid
    end

    test "modules list has correct row count" do
      grid = Barcode2D.make_module_grid(6, 4)

      assert length(grid.modules) == 6
    end

    test "each row has correct column count" do
      grid = Barcode2D.make_module_grid(4, 9)

      for row <- grid.modules do
        assert length(row) == 9
      end
    end

    test "creates a 21x21 QR Code v1 grid (smoke test)" do
      # QR Code version 1 is always 21×21 modules.
      grid = Barcode2D.make_module_grid(21, 21)

      assert grid.rows == 21
      assert grid.cols == 21
      assert grid.module_shape == :square
    end
  end

  # ============================================================================
  # set_module/4
  # ============================================================================

  describe "set_module/4" do
    test "sets a module to dark (true)" do
      grid = Barcode2D.make_module_grid(3, 3)

      {:ok, grid2} = Barcode2D.set_module(grid, 1, 1, true)

      assert Enum.at(Enum.at(grid2.modules, 1), 1) == true
    end

    test "sets a module to light (false)" do
      grid = Barcode2D.make_module_grid(3, 3)
      {:ok, grid2} = Barcode2D.set_module(grid, 0, 0, true)

      {:ok, grid3} = Barcode2D.set_module(grid2, 0, 0, false)

      assert Enum.at(Enum.at(grid3.modules, 0), 0) == false
    end

    test "original grid is unchanged after set_module" do
      grid = Barcode2D.make_module_grid(3, 3)

      {:ok, _grid2} = Barcode2D.set_module(grid, 1, 1, true)

      # The original must still be all-light.
      assert Enum.at(Enum.at(grid.modules, 1), 1) == false
    end

    test "only the specified position changes" do
      grid = Barcode2D.make_module_grid(3, 3)

      {:ok, grid2} = Barcode2D.set_module(grid, 0, 2, true)

      # Position (0,2) is now dark.
      assert Enum.at(Enum.at(grid2.modules, 0), 2) == true

      # All other positions remain light.
      assert Enum.at(Enum.at(grid2.modules, 0), 0) == false
      assert Enum.at(Enum.at(grid2.modules, 0), 1) == false
      assert Enum.at(Enum.at(grid2.modules, 1), 0) == false
      assert Enum.at(Enum.at(grid2.modules, 2), 2) == false
    end

    test "returns error for negative row" do
      grid = Barcode2D.make_module_grid(3, 3)

      assert {:error, msg} = Barcode2D.set_module(grid, -1, 0, true)
      assert msg =~ "row"
      assert msg =~ "-1"
    end

    test "returns error for row == rows" do
      grid = Barcode2D.make_module_grid(3, 3)

      assert {:error, msg} = Barcode2D.set_module(grid, 3, 0, true)
      assert msg =~ "row"
    end

    test "returns error for row > rows" do
      grid = Barcode2D.make_module_grid(3, 3)

      assert {:error, _msg} = Barcode2D.set_module(grid, 100, 0, true)
    end

    test "returns error for negative col" do
      grid = Barcode2D.make_module_grid(3, 3)

      assert {:error, msg} = Barcode2D.set_module(grid, 0, -1, true)
      assert msg =~ "col"
      assert msg =~ "-1"
    end

    test "returns error for col == cols" do
      grid = Barcode2D.make_module_grid(3, 3)

      assert {:error, _msg} = Barcode2D.set_module(grid, 0, 3, true)
    end

    test "returns error for col > cols" do
      grid = Barcode2D.make_module_grid(3, 3)

      assert {:error, _msg} = Barcode2D.set_module(grid, 0, 100, true)
    end

    test "set_module on top-left corner (0, 0)" do
      grid = Barcode2D.make_module_grid(3, 3)

      {:ok, grid2} = Barcode2D.set_module(grid, 0, 0, true)

      assert Enum.at(Enum.at(grid2.modules, 0), 0) == true
    end

    test "set_module on bottom-right corner" do
      grid = Barcode2D.make_module_grid(3, 4)

      {:ok, grid2} = Barcode2D.set_module(grid, 2, 3, true)

      assert Enum.at(Enum.at(grid2.modules, 2), 3) == true
    end

    test "chained set_module calls produce correct grid" do
      grid = Barcode2D.make_module_grid(3, 3)

      {:ok, grid2} = Barcode2D.set_module(grid, 0, 0, true)
      {:ok, grid3} = Barcode2D.set_module(grid2, 1, 1, true)
      {:ok, grid4} = Barcode2D.set_module(grid3, 2, 2, true)

      # Diagonal should be dark.
      assert Enum.at(Enum.at(grid4.modules, 0), 0) == true
      assert Enum.at(Enum.at(grid4.modules, 1), 1) == true
      assert Enum.at(Enum.at(grid4.modules, 2), 2) == true

      # Off-diagonal should be light.
      assert Enum.at(Enum.at(grid4.modules, 0), 1) == false
      assert Enum.at(Enum.at(grid4.modules, 1), 0) == false
    end

    test "preserves rows, cols, module_shape" do
      grid = Barcode2D.make_module_grid(5, 7, :hex)

      {:ok, grid2} = Barcode2D.set_module(grid, 2, 3, true)

      assert grid2.rows == 5
      assert grid2.cols == 7
      assert grid2.module_shape == :hex
    end
  end

  # ============================================================================
  # layout/2 — square modules (the common case)
  # ============================================================================

  describe "layout/2 with square modules" do
    test "returns {:ok, scene} for a valid grid and default config" do
      grid = Barcode2D.make_module_grid(5, 5)

      assert {:ok, scene} = Barcode2D.layout(grid)
      assert is_map(scene)
    end

    test "scene has correct total width including quiet zone" do
      # (5 cols + 2*4 quiet) * 10 px = 130.0
      grid = Barcode2D.make_module_grid(5, 5)

      {:ok, scene} = Barcode2D.layout(grid)

      assert scene.width == 130.0
    end

    test "scene has correct total height including quiet zone" do
      grid = Barcode2D.make_module_grid(5, 5)

      {:ok, scene} = Barcode2D.layout(grid)

      assert scene.height == 130.0
    end

    test "scene background matches config" do
      grid = Barcode2D.make_module_grid(3, 3)

      {:ok, scene} = Barcode2D.layout(grid, %Barcode2DLayoutConfig{background: "#ff0000"})

      assert scene.background == "#ff0000"
    end

    test "all-light grid produces 1 instruction (background only)" do
      grid = Barcode2D.make_module_grid(5, 5)

      {:ok, scene} = Barcode2D.layout(grid)

      # Only the background rect; no dark modules.
      assert length(scene.instructions) == 1
    end

    test "grid with one dark module produces 2 instructions" do
      grid = Barcode2D.make_module_grid(5, 5)
      {:ok, grid} = Barcode2D.set_module(grid, 2, 2, true)

      {:ok, scene} = Barcode2D.layout(grid)

      # Background + 1 dark rect.
      assert length(scene.instructions) == 2
    end

    test "background instruction is a rect covering the whole canvas" do
      grid = Barcode2D.make_module_grid(3, 3)

      {:ok, scene} = Barcode2D.layout(grid)

      bg = List.first(scene.instructions)
      assert bg.kind == :rect
      assert bg.x == 0
      assert bg.y == 0
      assert bg.width == scene.width
      assert bg.height == scene.height
    end

    test "dark module rect has correct pixel position (square)" do
      # Grid: 5×5, module_size_px 10, quiet_zone_modules 4
      # Module at (row=1, col=2):
      #   x = 4*10 + 2*10 = 60
      #   y = 4*10 + 1*10 = 50
      grid = Barcode2D.make_module_grid(5, 5)
      {:ok, grid} = Barcode2D.set_module(grid, 1, 2, true)

      {:ok, scene} = Barcode2D.layout(grid)

      [_bg, dark_rect] = scene.instructions
      assert dark_rect.x == 60.0
      assert dark_rect.y == 50.0
      assert dark_rect.width == 10.0
      assert dark_rect.height == 10.0
    end

    test "dark module rect fill matches foreground color" do
      grid = Barcode2D.make_module_grid(3, 3)
      {:ok, grid} = Barcode2D.set_module(grid, 0, 0, true)

      {:ok, scene} =
        Barcode2D.layout(grid, %Barcode2DLayoutConfig{foreground: "#0000ff"})

      [_bg, dark_rect] = scene.instructions
      assert dark_rect.fill == "#0000ff"
    end

    test "custom module_size_px scales the scene correctly" do
      # 3×3 grid, module_size_px=4, quiet_zone_modules=2
      # total = (3 + 2*2) * 4 = 28
      grid = Barcode2D.make_module_grid(3, 3)
      config = %Barcode2DLayoutConfig{module_size_px: 4.0, quiet_zone_modules: 2}

      {:ok, scene} = Barcode2D.layout(grid, config)

      assert scene.width == 28.0
      assert scene.height == 28.0
    end

    test "zero quiet_zone_modules is valid" do
      grid = Barcode2D.make_module_grid(3, 3)
      config = %Barcode2DLayoutConfig{quiet_zone_modules: 0}

      assert {:ok, scene} = Barcode2D.layout(grid, config)
      # (3 + 0) * 10 = 30
      assert scene.width == 30.0
    end

    test "all dark modules produces 1 + rows*cols instructions" do
      grid = Barcode2D.make_module_grid(2, 2)
      {:ok, grid} = Barcode2D.set_module(grid, 0, 0, true)
      {:ok, grid} = Barcode2D.set_module(grid, 0, 1, true)
      {:ok, grid} = Barcode2D.set_module(grid, 1, 0, true)
      {:ok, grid} = Barcode2D.set_module(grid, 1, 1, true)

      {:ok, scene} = Barcode2D.layout(grid)

      # 1 background + 4 dark rects
      assert length(scene.instructions) == 5
    end

    test "21x21 QR v1 empty grid layout" do
      grid = Barcode2D.make_module_grid(21, 21)

      {:ok, scene} = Barcode2D.layout(grid)

      # (21 + 2*4) * 10 = 290
      assert scene.width == 290.0
      assert scene.height == 290.0
      assert length(scene.instructions) == 1
    end
  end

  # ============================================================================
  # layout/2 — validation errors
  # ============================================================================

  describe "layout/2 validation" do
    test "returns error when module_size_px == 0" do
      grid = Barcode2D.make_module_grid(3, 3)
      config = %Barcode2DLayoutConfig{module_size_px: 0.0}

      assert {:error, msg} = Barcode2D.layout(grid, config)
      assert msg =~ "module_size_px"
    end

    test "returns error when module_size_px is negative" do
      grid = Barcode2D.make_module_grid(3, 3)
      config = %Barcode2DLayoutConfig{module_size_px: -5.0}

      assert {:error, msg} = Barcode2D.layout(grid, config)
      assert msg =~ "module_size_px"
    end

    test "returns error when quiet_zone_modules is negative" do
      grid = Barcode2D.make_module_grid(3, 3)
      config = %Barcode2DLayoutConfig{quiet_zone_modules: -1}

      assert {:error, msg} = Barcode2D.layout(grid, config)
      assert msg =~ "quiet_zone_modules"
    end

    test "returns error when config.module_shape does not match grid.module_shape" do
      grid = Barcode2D.make_module_grid(3, 3, :hex)
      config = %Barcode2DLayoutConfig{module_shape: :square}

      assert {:error, msg} = Barcode2D.layout(grid, config)
      assert msg =~ "module_shape"
    end

    test "returns error when config is square but grid is hex" do
      grid = Barcode2D.make_module_grid(3, 3, :hex)

      # Default config has module_shape: :square.
      assert {:error, _msg} = Barcode2D.layout(grid)
    end

    test "returns error when config is hex but grid is square" do
      grid = Barcode2D.make_module_grid(3, 3, :square)
      config = %Barcode2DLayoutConfig{module_shape: :hex}

      assert {:error, _msg} = Barcode2D.layout(grid, config)
    end
  end

  # ============================================================================
  # layout/2 — hex modules (MaxiCode)
  # ============================================================================

  describe "layout/2 with hex modules" do
    test "returns {:ok, scene} for a valid hex grid" do
      grid = Barcode2D.make_module_grid(5, 5, :hex)
      config = %Barcode2DLayoutConfig{module_shape: :hex}

      assert {:ok, scene} = Barcode2D.layout(grid, config)
      assert is_map(scene)
    end

    test "all-light hex grid produces 1 instruction (background only)" do
      grid = Barcode2D.make_module_grid(5, 5, :hex)
      config = %Barcode2DLayoutConfig{module_shape: :hex}

      {:ok, scene} = Barcode2D.layout(grid, config)

      assert length(scene.instructions) == 1
    end

    test "one dark hex module produces 2 instructions" do
      grid = Barcode2D.make_module_grid(5, 5, :hex)
      {:ok, grid} = Barcode2D.set_module(grid, 0, 0, true)
      config = %Barcode2DLayoutConfig{module_shape: :hex}

      {:ok, scene} = Barcode2D.layout(grid, config)

      assert length(scene.instructions) == 2
    end

    test "dark hex module instruction has kind :path" do
      grid = Barcode2D.make_module_grid(3, 3, :hex)
      {:ok, grid} = Barcode2D.set_module(grid, 0, 0, true)
      config = %Barcode2DLayoutConfig{module_shape: :hex}

      {:ok, scene} = Barcode2D.layout(grid, config)

      [_bg, path_instr] = scene.instructions
      assert path_instr.kind == :path
    end

    test "hex path has 7 commands (move_to + 5 line_to + close)" do
      grid = Barcode2D.make_module_grid(3, 3, :hex)
      {:ok, grid} = Barcode2D.set_module(grid, 0, 0, true)
      config = %Barcode2DLayoutConfig{module_shape: :hex}

      {:ok, scene} = Barcode2D.layout(grid, config)

      [_bg, path_instr] = scene.instructions
      # 1 move_to + 5 line_to + 1 close = 7 commands
      assert length(path_instr.commands) == 7
    end

    test "hex path first command is move_to" do
      grid = Barcode2D.make_module_grid(3, 3, :hex)
      {:ok, grid} = Barcode2D.set_module(grid, 0, 0, true)
      config = %Barcode2DLayoutConfig{module_shape: :hex}

      {:ok, scene} = Barcode2D.layout(grid, config)

      [_bg, path_instr] = scene.instructions
      first_cmd = List.first(path_instr.commands)
      assert first_cmd.kind == :move_to
    end

    test "hex path last command is close" do
      grid = Barcode2D.make_module_grid(3, 3, :hex)
      {:ok, grid} = Barcode2D.set_module(grid, 0, 0, true)
      config = %Barcode2DLayoutConfig{module_shape: :hex}

      {:ok, scene} = Barcode2D.layout(grid, config)

      [_bg, path_instr] = scene.instructions
      last_cmd = List.last(path_instr.commands)
      assert last_cmd.kind == :close
    end

    test "hex path commands 1-5 are line_to" do
      grid = Barcode2D.make_module_grid(3, 3, :hex)
      {:ok, grid} = Barcode2D.set_module(grid, 0, 0, true)
      config = %Barcode2DLayoutConfig{module_shape: :hex}

      {:ok, scene} = Barcode2D.layout(grid, config)

      [_bg, path_instr] = scene.instructions
      # Commands index 1..5 (0-based) are line_to.
      line_cmds = Enum.slice(path_instr.commands, 1, 5)
      assert Enum.all?(line_cmds, fn c -> c.kind == :line_to end)
    end

    test "hex path fill matches foreground color" do
      grid = Barcode2D.make_module_grid(3, 3, :hex)
      {:ok, grid} = Barcode2D.set_module(grid, 0, 0, true)
      config = %Barcode2DLayoutConfig{module_shape: :hex, foreground: "#112233"}

      {:ok, scene} = Barcode2D.layout(grid, config)

      [_bg, path_instr] = scene.instructions
      assert path_instr.fill == "#112233"
    end

    test "hex total width includes quiet zone and half-hex offset for odd rows" do
      # hex_width = 10, quiet_zone_modules = 4, cols = 3
      # total_width = (3 + 2*4) * 10 + 10/2 = 110 + 5 = 115
      grid = Barcode2D.make_module_grid(3, 3, :hex)
      config = %Barcode2DLayoutConfig{module_shape: :hex, module_size_px: 10.0, quiet_zone_modules: 4}

      {:ok, scene} = Barcode2D.layout(grid, config)

      assert scene.width == 115.0
    end

    test "hex total height uses sqrt(3)/2 row step" do
      # hex_height = 10 * sqrt(3)/2 per row, rows=3, quiet_zone=4
      # total_height = (3 + 8) * hex_height = 11 * (10 * sqrt(3)/2)
      grid = Barcode2D.make_module_grid(3, 3, :hex)
      config = %Barcode2DLayoutConfig{module_shape: :hex, module_size_px: 10.0, quiet_zone_modules: 4}

      {:ok, scene} = Barcode2D.layout(grid, config)

      expected = 11 * 10.0 * (:math.sqrt(3) / 2.0)
      assert_in_delta scene.height, expected, 1.0e-9
    end

    test "MaxiCode 33x30 hex grid layout smoke test" do
      grid = Barcode2D.make_module_grid(33, 30, :hex)
      config = %Barcode2DLayoutConfig{module_shape: :hex, quiet_zone_modules: 1}

      assert {:ok, scene} = Barcode2D.layout(grid, config)
      assert scene.width > 0
      assert scene.height > 0
    end

    test "odd row hex module is offset right by half hex_width" do
      # Row 1 (odd) at col 0 with module_size_px=10, quiet_zone_modules=0:
      #   cx = 0 + 0*10 + 1*(10/2) = 5
      #   cy = 0 + 1 * (10 * sqrt(3)/2)
      grid = Barcode2D.make_module_grid(3, 3, :hex)
      {:ok, grid} = Barcode2D.set_module(grid, 1, 0, true)

      config = %Barcode2DLayoutConfig{
        module_shape: :hex,
        module_size_px: 10.0,
        quiet_zone_modules: 0
      }

      {:ok, scene} = Barcode2D.layout(grid, config)

      [_bg, path_instr] = scene.instructions
      # First command is move_to vertex 0 at angle 0° (rightmost point).
      # cx = 5.0, circumR = 10/sqrt(3)
      # vertex 0 x = 5.0 + circumR
      circum_r = 10.0 / :math.sqrt(3)
      expected_x0 = 5.0 + circum_r

      move_cmd = List.first(path_instr.commands)
      assert_in_delta move_cmd.x, expected_x0, 1.0e-9
    end
  end

  # ============================================================================
  # Barcode2DLayoutConfig struct defaults
  # ============================================================================

  describe "Barcode2DLayoutConfig defaults" do
    test "default module_size_px is 10.0" do
      config = %Barcode2DLayoutConfig{}
      assert config.module_size_px == 10.0
    end

    test "default quiet_zone_modules is 4" do
      config = %Barcode2DLayoutConfig{}
      assert config.quiet_zone_modules == 4
    end

    test "default foreground is #000000" do
      config = %Barcode2DLayoutConfig{}
      assert config.foreground == "#000000"
    end

    test "default background is #ffffff" do
      config = %Barcode2DLayoutConfig{}
      assert config.background == "#ffffff"
    end

    test "default show_annotations is false" do
      config = %Barcode2DLayoutConfig{}
      assert config.show_annotations == false
    end

    test "default module_shape is :square" do
      config = %Barcode2DLayoutConfig{}
      assert config.module_shape == :square
    end
  end
end
