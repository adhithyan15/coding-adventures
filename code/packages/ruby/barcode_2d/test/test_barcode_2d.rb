# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_barcode_2d"

# =============================================================================
# TestBarcode2D — comprehensive unit tests for the barcode_2d package
# =============================================================================
#
# Tests mirror the TypeScript test suite and cover:
#   - VERSION constant
#   - make_module_grid (dimensions, default values, module_shape)
#   - set_module (immutability, out-of-bounds, dark/light toggle)
#   - layout() validation errors (bad module_size_px, quiet_zone_modules, shape mismatch)
#   - layout() square-module path (background rect, dark rects, dimensions)
#   - layout() hex-module path (background rect, path commands, dimensions)
#   - ModuleRole constants
#   - layout() with zero quiet zone
#   - layout() with all-dark grid
#   - layout() with all-light grid
class TestBarcode2D < Minitest::Test
  # Convenience alias so tests stay readable.
  B2D = CodingAdventures::Barcode2D
  PI  = CodingAdventures::PaintInstructions

  # ---------------------------------------------------------------------------
  # VERSION
  # ---------------------------------------------------------------------------

  def test_version_is_semver
    # VERSION should be a non-empty string following x.y.z semver format.
    assert_match(/\A\d+\.\d+\.\d+\z/, B2D::VERSION)
  end

  def test_version_value
    assert_equal "0.1.0", B2D::VERSION
  end

  # ---------------------------------------------------------------------------
  # make_module_grid
  # ---------------------------------------------------------------------------

  def test_make_module_grid_dimensions
    grid = B2D.make_module_grid(5, 7)
    assert_equal 5, grid.rows
    assert_equal 7, grid.cols
  end

  def test_make_module_grid_all_false
    grid = B2D.make_module_grid(3, 4)
    grid.rows.times do |row|
      grid.cols.times do |col|
        refute grid.modules[row][col],
          "Expected modules[#{row}][#{col}] to be false"
      end
    end
  end

  def test_make_module_grid_default_shape_is_square
    grid = B2D.make_module_grid(2, 2)
    assert_equal "square", grid.module_shape
  end

  def test_make_module_grid_hex_shape
    grid = B2D.make_module_grid(33, 30, module_shape: "hex")
    assert_equal "hex", grid.module_shape
    assert_equal 33, grid.rows
    assert_equal 30, grid.cols
  end

  def test_make_module_grid_is_frozen
    grid = B2D.make_module_grid(3, 3)
    assert_predicate grid, :frozen?
  end

  def test_make_module_grid_rows_frozen
    grid = B2D.make_module_grid(3, 3)
    grid.modules.each do |row|
      assert_predicate row, :frozen?
    end
  end

  def test_make_module_grid_1x1
    grid = B2D.make_module_grid(1, 1)
    assert_equal 1, grid.rows
    assert_equal 1, grid.cols
    refute grid.modules[0][0]
  end

  # ---------------------------------------------------------------------------
  # set_module
  # ---------------------------------------------------------------------------

  def test_set_module_returns_new_grid
    g  = B2D.make_module_grid(3, 3)
    g2 = B2D.set_module(g, 1, 1, true)
    refute_same g, g2
  end

  def test_set_module_original_unchanged
    g  = B2D.make_module_grid(3, 3)
    B2D.set_module(g, 1, 1, true)
    refute g.modules[1][1], "Original grid must not be mutated"
  end

  def test_set_module_dark_true
    g  = B2D.make_module_grid(3, 3)
    g2 = B2D.set_module(g, 0, 0, true)
    assert g2.modules[0][0]
  end

  def test_set_module_dark_false
    g  = B2D.make_module_grid(3, 3)
    g2 = B2D.set_module(g, 2, 2, true)
    g3 = B2D.set_module(g2, 2, 2, false)
    refute g3.modules[2][2]
  end

  def test_set_module_only_target_row_changed
    g  = B2D.make_module_grid(3, 3)
    g2 = B2D.set_module(g, 1, 1, true)
    # Unchanged rows should be identical Ruby objects (shared, not copied).
    assert_same g.modules[0], g2.modules[0]
    assert_same g.modules[2], g2.modules[2]
    # Changed row should be a different object.
    refute_same g.modules[1], g2.modules[1]
  end

  def test_set_module_result_is_frozen
    g  = B2D.make_module_grid(3, 3)
    g2 = B2D.set_module(g, 1, 1, true)
    assert_predicate g2, :frozen?
  end

  def test_set_module_out_of_bounds_row_too_low
    g = B2D.make_module_grid(3, 3)
    assert_raises(RangeError) { B2D.set_module(g, -1, 0, true) }
  end

  def test_set_module_out_of_bounds_row_too_high
    g = B2D.make_module_grid(3, 3)
    assert_raises(RangeError) { B2D.set_module(g, 3, 0, true) }
  end

  def test_set_module_out_of_bounds_col_too_low
    g = B2D.make_module_grid(3, 3)
    assert_raises(RangeError) { B2D.set_module(g, 0, -1, true) }
  end

  def test_set_module_out_of_bounds_col_too_high
    g = B2D.make_module_grid(3, 3)
    assert_raises(RangeError) { B2D.set_module(g, 0, 3, true) }
  end

  def test_set_module_range_error_message_row
    g = B2D.make_module_grid(4, 4)
    err = assert_raises(RangeError) { B2D.set_module(g, 99, 0, true) }
    assert_match(/row/, err.message)
    assert_match(/99/, err.message)
  end

  def test_set_module_range_error_message_col
    g = B2D.make_module_grid(4, 4)
    err = assert_raises(RangeError) { B2D.set_module(g, 0, 99, true) }
    assert_match(/col/, err.message)
    assert_match(/99/, err.message)
  end

  def test_set_module_last_valid_position
    g  = B2D.make_module_grid(3, 4)
    g2 = B2D.set_module(g, 2, 3, true)  # bottom-right corner
    assert g2.modules[2][3]
  end

  def test_set_module_multiple_times
    g = B2D.make_module_grid(3, 3)
    g = B2D.set_module(g, 0, 0, true)
    g = B2D.set_module(g, 1, 1, true)
    g = B2D.set_module(g, 2, 2, true)
    # All three modules are dark.
    assert g.modules[0][0]
    assert g.modules[1][1]
    assert g.modules[2][2]
    # Others remain light.
    refute g.modules[0][1]
    refute g.modules[0][2]
  end

  # ---------------------------------------------------------------------------
  # layout() — validation errors
  # ---------------------------------------------------------------------------

  def test_layout_raises_for_zero_module_size
    g = B2D.make_module_grid(5, 5)
    err = assert_raises(B2D::InvalidBarcode2DConfigError) do
      B2D.layout(g, module_size_px: 0)
    end
    assert_match(/module_size_px/, err.message)
  end

  def test_layout_raises_for_negative_module_size
    g = B2D.make_module_grid(5, 5)
    assert_raises(B2D::InvalidBarcode2DConfigError) do
      B2D.layout(g, module_size_px: -1)
    end
  end

  def test_layout_raises_for_negative_quiet_zone
    g = B2D.make_module_grid(5, 5)
    err = assert_raises(B2D::InvalidBarcode2DConfigError) do
      B2D.layout(g, quiet_zone_modules: -1)
    end
    assert_match(/quiet_zone_modules/, err.message)
  end

  def test_layout_raises_for_module_shape_mismatch_square_vs_hex
    g = B2D.make_module_grid(5, 5, module_shape: "square")
    err = assert_raises(B2D::InvalidBarcode2DConfigError) do
      B2D.layout(g, module_shape: "hex")
    end
    assert_match(/module_shape/, err.message)
  end

  def test_layout_raises_for_module_shape_mismatch_hex_vs_square
    g = B2D.make_module_grid(5, 5, module_shape: "hex")
    err = assert_raises(B2D::InvalidBarcode2DConfigError) do
      B2D.layout(g, module_shape: "square")
    end
    assert_match(/module_shape/, err.message)
  end

  def test_layout_invalid_config_error_is_barcode_2d_error
    g = B2D.make_module_grid(5, 5)
    err = assert_raises(B2D::InvalidBarcode2DConfigError) do
      B2D.layout(g, module_size_px: 0)
    end
    assert_kind_of B2D::Barcode2DError, err
    assert_kind_of StandardError, err
  end

  # ---------------------------------------------------------------------------
  # layout() — square module path
  # ---------------------------------------------------------------------------

  def test_layout_square_returns_paint_scene
    g = B2D.make_module_grid(3, 3)
    scene = B2D.layout(g)
    # PaintScene is an OpenStruct with width, height, background, instructions.
    assert_respond_to scene, :width
    assert_respond_to scene, :height
    assert_respond_to scene, :instructions
    assert_respond_to scene, :background
  end

  def test_layout_square_total_dimensions_with_default_config
    # Default: module_size_px=10, quiet_zone_modules=4
    # Grid: 21×21 (QR Code v1)
    # total_width  = (21 + 2*4) * 10 = 290
    # total_height = (21 + 2*4) * 10 = 290
    g = B2D.make_module_grid(21, 21)
    scene = B2D.layout(g)
    assert_equal 290, scene.width
    assert_equal 290, scene.height
  end

  def test_layout_square_dimensions_custom_config
    # Grid: 5×7, module_size_px=5, quiet_zone_modules=2
    # total_width  = (7 + 2*2) * 5 = 55
    # total_height = (5 + 2*2) * 5 = 45
    g = B2D.make_module_grid(5, 7)
    scene = B2D.layout(g, module_size_px: 5, quiet_zone_modules: 2)
    assert_equal 55, scene.width
    assert_equal 45, scene.height
  end

  def test_layout_square_all_light_grid_has_one_instruction
    # An all-light grid should produce only the background rect.
    g = B2D.make_module_grid(5, 5)
    scene = B2D.layout(g)
    assert_equal 1, scene.instructions.length
    assert_equal "rect", scene.instructions.first.kind
  end

  def test_layout_square_background_rect_is_first
    g = B2D.make_module_grid(5, 5)
    scene = B2D.layout(g, module_size_px: 10, quiet_zone_modules: 4)
    bg_rect = scene.instructions.first
    assert_equal 0, bg_rect.x
    assert_equal 0, bg_rect.y
    assert_equal 130, bg_rect.width   # (5 + 2*4) * 10 = 130
    assert_equal 130, bg_rect.height
    assert_equal "#ffffff", bg_rect.fill
  end

  def test_layout_square_background_color_in_scene
    g = B2D.make_module_grid(5, 5)
    scene = B2D.layout(g, background: "#ff0000")
    assert_equal "#ff0000", scene.background
  end

  def test_layout_square_dark_module_count
    # Set exactly 3 modules dark → background + 3 dark rects = 4 instructions.
    g = B2D.make_module_grid(5, 5)
    g = B2D.set_module(g, 0, 0, true)
    g = B2D.set_module(g, 2, 2, true)
    g = B2D.set_module(g, 4, 4, true)
    scene = B2D.layout(g)
    assert_equal 4, scene.instructions.length  # 1 bg + 3 dark rects
  end

  def test_layout_square_dark_rect_position
    # Grid: 3×3, module_size_px=10, quiet_zone_modules=4
    # Dark module at (0, 0):
    #   x = 4*10 + 0*10 = 40
    #   y = 4*10 + 0*10 = 40
    g = B2D.make_module_grid(3, 3)
    g = B2D.set_module(g, 0, 0, true)
    scene = B2D.layout(g, module_size_px: 10, quiet_zone_modules: 4)
    dark_rect = scene.instructions[1]  # second instruction; first is bg
    assert_equal "rect", dark_rect.kind
    assert_equal 40, dark_rect.x
    assert_equal 40, dark_rect.y
    assert_equal 10, dark_rect.width
    assert_equal 10, dark_rect.height
  end

  def test_layout_square_dark_rect_position_mid_grid
    # Dark module at (1, 2) with module_size_px=10, quiet_zone_modules=2:
    #   x = 2*10 + 2*10 = 40
    #   y = 2*10 + 1*10 = 30
    g = B2D.make_module_grid(5, 5)
    g = B2D.set_module(g, 1, 2, true)
    scene = B2D.layout(g, module_size_px: 10, quiet_zone_modules: 2)
    dark_rect = scene.instructions[1]
    assert_equal 40, dark_rect.x
    assert_equal 30, dark_rect.y
  end

  def test_layout_square_dark_rect_fill_color
    g = B2D.make_module_grid(3, 3)
    g = B2D.set_module(g, 0, 0, true)
    scene = B2D.layout(g, foreground: "#111111")
    dark_rect = scene.instructions[1]
    assert_equal "#111111", dark_rect.fill
  end

  def test_layout_square_all_dark_grid
    # 2×2 all-dark → 1 background + 4 dark rects = 5 instructions.
    g = B2D.make_module_grid(2, 2)
    g = B2D.set_module(g, 0, 0, true)
    g = B2D.set_module(g, 0, 1, true)
    g = B2D.set_module(g, 1, 0, true)
    g = B2D.set_module(g, 1, 1, true)
    scene = B2D.layout(g)
    assert_equal 5, scene.instructions.length
  end

  def test_layout_square_zero_quiet_zone
    # quiet_zone_modules=0 means no quiet zone at all.
    g = B2D.make_module_grid(5, 5)
    scene = B2D.layout(g, module_size_px: 10, quiet_zone_modules: 0)
    assert_equal 50, scene.width
    assert_equal 50, scene.height
  end

  def test_layout_square_zero_quiet_zone_dark_at_origin
    # With quiet_zone_modules=0, a dark module at (0,0) starts at pixel (0,0).
    g = B2D.make_module_grid(3, 3)
    g = B2D.set_module(g, 0, 0, true)
    scene = B2D.layout(g, module_size_px: 10, quiet_zone_modules: 0)
    dark_rect = scene.instructions[1]
    assert_equal 0, dark_rect.x
    assert_equal 0, dark_rect.y
  end

  def test_layout_square_all_dark_1x1
    # 1×1 all-dark, module_size_px=8, quiet_zone_modules=1
    # total_width = (1 + 2) * 8 = 24
    # dark rect x = 1*8 = 8, y = 8
    g = B2D.make_module_grid(1, 1)
    g = B2D.set_module(g, 0, 0, true)
    scene = B2D.layout(g, module_size_px: 8, quiet_zone_modules: 1)
    assert_equal 24, scene.width
    assert_equal 24, scene.height
    assert_equal 2, scene.instructions.length
    dark_rect = scene.instructions[1]
    assert_equal 8, dark_rect.x
    assert_equal 8, dark_rect.y
    assert_equal 8, dark_rect.width
    assert_equal 8, dark_rect.height
  end

  def test_layout_square_rect_instructions_are_kind_rect
    g = B2D.make_module_grid(3, 3)
    g = B2D.set_module(g, 0, 0, true)
    scene = B2D.layout(g)
    scene.instructions.each do |instr|
      assert_equal "rect", instr.kind
    end
  end

  def test_layout_square_default_config_nil_uses_defaults
    # Passing nil config should use the same defaults as passing no config.
    g = B2D.make_module_grid(3, 3)
    scene_default = B2D.layout(g)
    scene_nil     = B2D.layout(g, nil)
    assert_equal scene_default.width,  scene_nil.width
    assert_equal scene_default.height, scene_nil.height
    assert_equal scene_default.background, scene_nil.background
  end

  def test_layout_square_non_square_grid
    # 3 rows × 10 cols. Total = (10 + 2*4)*10 wide, (3 + 2*4)*10 tall.
    g = B2D.make_module_grid(3, 10)
    scene = B2D.layout(g, module_size_px: 10, quiet_zone_modules: 4)
    assert_equal 180, scene.width   # (10 + 8) * 10
    assert_equal 110, scene.height  # (3  + 8) * 10
  end

  # ---------------------------------------------------------------------------
  # layout() — hex module path
  # ---------------------------------------------------------------------------

  def test_layout_hex_returns_paint_scene
    g = B2D.make_module_grid(3, 3, module_shape: "hex")
    scene = B2D.layout(g, module_shape: "hex")
    assert_respond_to scene, :width
    assert_respond_to scene, :height
    assert_respond_to scene, :instructions
  end

  def test_layout_hex_all_light_has_one_instruction
    g = B2D.make_module_grid(3, 3, module_shape: "hex")
    scene = B2D.layout(g, module_shape: "hex")
    assert_equal 1, scene.instructions.length
  end

  def test_layout_hex_dark_module_produces_path
    g = B2D.make_module_grid(3, 3, module_shape: "hex")
    g = B2D.set_module(g, 0, 0, true)
    scene = B2D.layout(g, module_shape: "hex")
    path_instr = scene.instructions[1]
    assert_equal "path", path_instr.kind
  end

  def test_layout_hex_path_has_seven_commands
    # 1 move_to + 5 line_to + 1 close = 7 commands per hexagon.
    # (vertices 1..5 are drawn with line_to; vertex 0 uses move_to;
    #  the final close returns to vertex 0)
    g = B2D.make_module_grid(3, 3, module_shape: "hex")
    g = B2D.set_module(g, 0, 0, true)
    scene = B2D.layout(g, module_shape: "hex")
    path_instr = scene.instructions[1]
    assert_equal 7, path_instr.commands.length
  end

  def test_layout_hex_path_first_command_is_move_to
    g = B2D.make_module_grid(3, 3, module_shape: "hex")
    g = B2D.set_module(g, 0, 0, true)
    scene = B2D.layout(g, module_shape: "hex")
    path_instr = scene.instructions[1]
    assert_equal "move_to", path_instr.commands.first[:kind]
  end

  def test_layout_hex_path_last_command_is_close
    g = B2D.make_module_grid(3, 3, module_shape: "hex")
    g = B2D.set_module(g, 0, 0, true)
    scene = B2D.layout(g, module_shape: "hex")
    path_instr = scene.instructions[1]
    assert_equal "close", path_instr.commands.last[:kind]
  end

  def test_layout_hex_path_middle_commands_are_line_to
    # Commands 1..5 (indices 1 through 5) should all be line_to.
    # Index 0 is move_to, index 6 is close.
    g = B2D.make_module_grid(3, 3, module_shape: "hex")
    g = B2D.set_module(g, 0, 0, true)
    scene = B2D.layout(g, module_shape: "hex")
    path_instr = scene.instructions[1]
    path_instr.commands[1..5].each do |cmd|
      assert_equal "line_to", cmd[:kind]
    end
  end

  def test_layout_hex_dark_rect_count
    # 2 dark modules → background + 2 paths = 3 instructions.
    g = B2D.make_module_grid(3, 3, module_shape: "hex")
    g = B2D.set_module(g, 0, 0, true)
    g = B2D.set_module(g, 1, 1, true)
    scene = B2D.layout(g, module_shape: "hex")
    assert_equal 3, scene.instructions.length
  end

  def test_layout_hex_dimensions_shape
    # Total dimensions for hex:
    #   hex_width  = module_size_px
    #   hex_height = module_size_px * (√3 / 2)
    #   total_width  = (cols + 2*qz) * hex_width + hex_width/2
    #   total_height = (rows + 2*qz) * hex_height
    module_size_px = 10.0
    qz = 4
    hex_height = module_size_px * (Math.sqrt(3) / 2.0)
    expected_width  = (5 + 2 * qz) * module_size_px + module_size_px / 2.0
    expected_height = (3 + 2 * qz) * hex_height

    g = B2D.make_module_grid(3, 5, module_shape: "hex")
    scene = B2D.layout(g,
      module_shape: "hex",
      module_size_px: 10,
      quiet_zone_modules: 4,)
    assert_in_delta expected_width,  scene.width,  0.0001
    assert_in_delta expected_height, scene.height, 0.0001
  end

  def test_layout_hex_path_fill_color
    g = B2D.make_module_grid(3, 3, module_shape: "hex")
    g = B2D.set_module(g, 0, 0, true)
    scene = B2D.layout(g, module_shape: "hex", foreground: "#cc0000")
    path_instr = scene.instructions[1]
    assert_equal "#cc0000", path_instr.fill
  end

  def test_layout_hex_odd_row_offset
    # A dark module on row 1 should have a larger x than the same col on row 0,
    # because odd rows are offset right by hex_width/2.
    module_size_px = 10.0
    qz = 0
    hex_width = module_size_px
    circum_r  = module_size_px / Math.sqrt(3)
    # Row 0, col 0: cx = 0 + 0*hex_width + 0*(hex_width/2) = 0
    # Row 1, col 0: cx = 0 + 0*hex_width + 1*(hex_width/2) = 5
    # First vertex (vertex 0, angle 0°):
    #   row 0: x = cx_row0 + circum_r * cos(0) = 0 + circum_r
    #   row 1: x = cx_row1 + circum_r * cos(0) = 5 + circum_r
    g = B2D.make_module_grid(3, 3, module_shape: "hex")
    g = B2D.set_module(g, 0, 0, true)
    g = B2D.set_module(g, 1, 0, true)
    scene = B2D.layout(g,
      module_shape: "hex",
      module_size_px: module_size_px,
      quiet_zone_modules: qz,)
    path_row0 = scene.instructions[1]
    path_row1 = scene.instructions[2]
    x0 = path_row0.commands.first[:x]  # move_to x for row 0
    x1 = path_row1.commands.first[:x]  # move_to x for row 1
    # Row 1 should be shifted right by hex_width/2 = 5.
    assert_in_delta hex_width / 2.0, x1 - x0, 0.0001
  end

  # ---------------------------------------------------------------------------
  # ModuleRole constants
  # ---------------------------------------------------------------------------

  def test_module_role_finder
    assert_equal "finder", B2D::ModuleRole::FINDER
  end

  def test_module_role_separator
    assert_equal "separator", B2D::ModuleRole::SEPARATOR
  end

  def test_module_role_timing
    assert_equal "timing", B2D::ModuleRole::TIMING
  end

  def test_module_role_alignment
    assert_equal "alignment", B2D::ModuleRole::ALIGNMENT
  end

  def test_module_role_format
    assert_equal "format", B2D::ModuleRole::FORMAT
  end

  def test_module_role_data
    assert_equal "data", B2D::ModuleRole::DATA
  end

  def test_module_role_ecc
    assert_equal "ecc", B2D::ModuleRole::ECC
  end

  def test_module_role_padding
    assert_equal "padding", B2D::ModuleRole::PADDING
  end

  def test_module_role_all_contains_eight_roles
    assert_equal 8, B2D::ModuleRole::ALL.length
  end

  def test_module_role_all_is_frozen
    assert_predicate B2D::ModuleRole::ALL, :frozen?
  end

  # ---------------------------------------------------------------------------
  # Default layout config
  # ---------------------------------------------------------------------------

  def test_default_layout_config_values
    cfg = B2D::DEFAULT_BARCODE_2D_LAYOUT_CONFIG
    assert_equal 10,       cfg[:module_size_px]
    assert_equal 4,        cfg[:quiet_zone_modules]
    assert_equal "#000000", cfg[:foreground]
    assert_equal "#ffffff", cfg[:background]
    assert_equal false,    cfg[:show_annotations]
    assert_equal "square", cfg[:module_shape]
  end

  def test_default_layout_config_is_frozen
    assert_predicate B2D::DEFAULT_BARCODE_2D_LAYOUT_CONFIG, :frozen?
  end

  # ---------------------------------------------------------------------------
  # Error classes
  # ---------------------------------------------------------------------------

  def test_barcode_2d_error_is_standard_error
    e = B2D::Barcode2DError.new("test")
    assert_kind_of StandardError, e
  end

  def test_invalid_barcode_2d_config_error_inherits_barcode_2d_error
    e = B2D::InvalidBarcode2DConfigError.new("test")
    assert_kind_of B2D::Barcode2DError, e
  end

  # ---------------------------------------------------------------------------
  # Edge cases: rectangular (non-square) grids
  # ---------------------------------------------------------------------------

  def test_layout_wide_grid
    # 1 row × 100 cols
    g = B2D.make_module_grid(1, 100)
    scene = B2D.layout(g, module_size_px: 2, quiet_zone_modules: 0)
    assert_equal 200, scene.width
    assert_equal 2,   scene.height
  end

  def test_layout_tall_grid
    # 100 rows × 1 col
    g = B2D.make_module_grid(100, 1)
    scene = B2D.layout(g, module_size_px: 2, quiet_zone_modules: 0)
    assert_equal 2,   scene.width
    assert_equal 200, scene.height
  end

  # ---------------------------------------------------------------------------
  # Integration: set_module + layout together
  # ---------------------------------------------------------------------------

  def test_full_pipeline_qr_v1_size
    # QR Code version 1 is 21×21 with 4-module quiet zone and 10px modules.
    g = B2D.make_module_grid(21, 21)
    # Place some finder modules.
    7.times do |r|
      7.times do |c|
        g = B2D.set_module(g, r, c, true)
      end
    end
    scene = B2D.layout(g)
    assert_equal 290, scene.width
    assert_equal 290, scene.height
    # 49 dark modules + 1 background = 50 instructions.
    assert_equal 50, scene.instructions.length
  end
end
