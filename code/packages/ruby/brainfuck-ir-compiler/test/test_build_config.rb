# frozen_string_literal: true

require_relative "test_helper"

# ==========================================================================
# Tests for BuildConfig — compilation flag struct and preset methods
# ==========================================================================

class TestBuildConfig < Minitest::Test
  include CodingAdventures::BrainfuckIrCompiler

  def test_debug_config_bounds_checks
    cfg = BuildConfig.debug_config
    assert cfg.insert_bounds_checks, "debug config should have bounds checks"
  end

  def test_debug_config_debug_locs
    cfg = BuildConfig.debug_config
    assert cfg.insert_debug_locs, "debug config should have debug locs"
  end

  def test_debug_config_mask_byte_arithmetic
    cfg = BuildConfig.debug_config
    assert cfg.mask_byte_arithmetic, "debug config should have byte masking"
  end

  def test_debug_config_tape_size
    cfg = BuildConfig.debug_config
    assert_equal 30_000, cfg.tape_size
  end

  def test_release_config_no_bounds_checks
    cfg = BuildConfig.release_config
    refute cfg.insert_bounds_checks, "release config should NOT have bounds checks"
  end

  def test_release_config_no_debug_locs
    cfg = BuildConfig.release_config
    refute cfg.insert_debug_locs, "release config should NOT have debug locs"
  end

  def test_release_config_mask_byte_arithmetic
    cfg = BuildConfig.release_config
    assert cfg.mask_byte_arithmetic, "release config should have byte masking"
  end

  def test_release_config_tape_size
    cfg = BuildConfig.release_config
    assert_equal 30_000, cfg.tape_size
  end

  def test_custom_config
    cfg = BuildConfig.new(
      insert_bounds_checks: true,
      insert_debug_locs: false,
      mask_byte_arithmetic: false,
      tape_size: 1000
    )
    assert cfg.insert_bounds_checks
    refute cfg.insert_debug_locs
    refute cfg.mask_byte_arithmetic
    assert_equal 1000, cfg.tape_size
  end
end
