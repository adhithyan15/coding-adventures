defmodule CodingAdventures.BrainfuckIrCompiler.BuildConfigTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BrainfuckIrCompiler.BuildConfig

  # ── debug_config/0 ───────────────────────────────────────────────────────────

  describe "debug_config/0" do
    test "has bounds checks enabled" do
      cfg = BuildConfig.debug_config()
      assert cfg.insert_bounds_checks == true
    end

    test "has debug locs enabled" do
      cfg = BuildConfig.debug_config()
      assert cfg.insert_debug_locs == true
    end

    test "has byte masking enabled" do
      cfg = BuildConfig.debug_config()
      assert cfg.mask_byte_arithmetic == true
    end

    test "has canonical tape size 30000" do
      cfg = BuildConfig.debug_config()
      assert cfg.tape_size == 30_000
    end
  end

  # ── release_config/0 ─────────────────────────────────────────────────────────

  describe "release_config/0" do
    test "has bounds checks disabled" do
      cfg = BuildConfig.release_config()
      assert cfg.insert_bounds_checks == false
    end

    test "has debug locs disabled" do
      cfg = BuildConfig.release_config()
      assert cfg.insert_debug_locs == false
    end

    test "has byte masking enabled (correctness requirement)" do
      cfg = BuildConfig.release_config()
      assert cfg.mask_byte_arithmetic == true
    end

    test "has canonical tape size 30000" do
      cfg = BuildConfig.release_config()
      assert cfg.tape_size == 30_000
    end
  end

  # ── Custom config ─────────────────────────────────────────────────────────────

  describe "custom BuildConfig" do
    test "can override tape_size" do
      cfg = %{BuildConfig.release_config() | tape_size: 1000}
      assert cfg.tape_size == 1000
    end

    test "can disable mask_byte_arithmetic" do
      cfg = %{BuildConfig.release_config() | mask_byte_arithmetic: false}
      assert cfg.mask_byte_arithmetic == false
    end
  end
end
