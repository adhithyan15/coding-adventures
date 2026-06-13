# frozen_string_literal: true

# silicon_rust_ruby_test.rb — minitest suite for the silicon_rust_ruby gem.
# =========================================================================
#
# These tests dlopen the silicon_rust_ruby_native shared library (built by
# `rake compile`) and verify the Ruby FFI boundary.  They test "did the
# call cross the FFI and come back with a physically reasonable value?",
# not the underlying Rust math (which the crate unit tests and the
# silicon-rust-python tests already cover).
#
# Layering principle:
#   Each language binding tests "did the FFI work?", not "did the physics
#   compute correctly?"
#
# Coverage targets from the spec:
#   * All 9 constant accessors return Numeric
#   * thermal_voltage(300) ≈ 0.025852
#   * pn_junction_built_in_voltage returns a positive Float
#   * mosfet_threshold_voltage returns a Float
#   * evaluate_level1_defaults returns a Hash with all 10 keys including :region
#   * evaluate_level1 saturation path returns { region: "saturation" }
#   * deposit builds a wire string with the new layer on top
#   * deal_grove_oxidation returns a String starting with "SiO2:"
#   * etch removes a layer
#   * implant_range returns a Hash with :rp and :straggle
#   * diffusivity_cm2_per_s returns a positive Float
#   * deposit raises RuntimeError when material name contains "|"
#   * deal_grove_oxidation raises RuntimeError for non-positive time_min
#   * Version constant is defined

require "minitest/autorun"
require "coding_adventures/silicon_rust_ruby"

class SiliconRustRubyTest < Minitest::Test
  # Tolerance for floating-point comparisons.
  EPSILON = 1e-6

  # ---------------------------------------------------------------------------
  # Physical constants — all should return Numeric (Float)
  # ---------------------------------------------------------------------------

  def test_k_boltzmann_is_numeric
    assert_kind_of Numeric, SiliconRustRuby.k_boltzmann
  end

  def test_q_electron_is_numeric
    assert_kind_of Numeric, SiliconRustRuby.q_electron
  end

  def test_eps0_is_numeric
    assert_kind_of Numeric, SiliconRustRuby.eps0
  end

  def test_eps_si_is_numeric
    assert_kind_of Numeric, SiliconRustRuby.eps_si
  end

  def test_eps_ox_is_numeric
    assert_kind_of Numeric, SiliconRustRuby.eps_ox
  end

  def test_ni_at_300k_is_numeric
    assert_kind_of Numeric, SiliconRustRuby.ni_at_300k
  end

  def test_eg_si_at_300k_is_numeric
    assert_kind_of Numeric, SiliconRustRuby.eg_si_at_300k
  end

  def test_mu_n_300k_is_numeric
    assert_kind_of Numeric, SiliconRustRuby.mu_n_300k
  end

  def test_mu_p_300k_is_numeric
    assert_kind_of Numeric, SiliconRustRuby.mu_p_300k
  end

  def test_k_boltzmann_value
    # 1.380649e-23 J/K
    assert_in_delta 1.380649e-23, SiliconRustRuby.k_boltzmann, 1e-30
  end

  # ---------------------------------------------------------------------------
  # device-physics
  # ---------------------------------------------------------------------------

  def test_thermal_voltage_at_300k
    # kT/q at 300 K ≈ 0.025852 V
    vt = SiliconRustRuby.thermal_voltage(300.0)
    assert_in_delta 0.025852, vt, 1e-5
  end

  def test_intrinsic_concentration_at_300k
    # Should be N_I_300K = 1e16 /m³ at 300 K
    ni = SiliconRustRuby.intrinsic_concentration(300.0)
    assert_in_delta 1.0e16, ni, 1e9
  end

  def test_intrinsic_concentration_below_100k_raises
    assert_raises(RuntimeError) do
      SiliconRustRuby.intrinsic_concentration(50.0)
    end
  end

  def test_fermi_potential_p_type
    # p-type, N=1e23, T=300 → positive value
    phi = SiliconRustRuby.fermi_potential(1e23, "p", 300.0)
    assert phi > 0.0, "p-type fermi potential should be positive"
  end

  def test_fermi_potential_n_type
    phi = SiliconRustRuby.fermi_potential(1e23, "n", 300.0)
    assert phi < 0.0, "n-type fermi potential should be negative"
  end

  def test_pn_junction_built_in_voltage_positive
    vbi = SiliconRustRuby.pn_junction_built_in_voltage(1e23, 1e22, 300.0)
    assert vbi > 0.5, "built-in voltage should be > 0.5 V for typical junction"
    assert vbi < 1.5, "built-in voltage should be < 1.5 V"
  end

  def test_pn_junction_depletion_width_positive
    w = SiliconRustRuby.pn_junction_depletion_width(1e23, 1e22, 300.0, 0.0)
    assert w > 0.0, "depletion width at zero bias should be positive"
  end

  def test_pn_junction_saturation_current_positive
    is_ = SiliconRustRuby.pn_junction_saturation_current(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6)
    assert is_ > 0.0, "saturation current should be positive"
  end

  def test_pn_junction_current_forward_bias
    i = SiliconRustRuby.pn_junction_current(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6, 0.6)
    assert i > 0.0, "forward-bias current should be positive"
  end

  def test_mosfet_threshold_voltage_nmos
    # 130 nm NMOS: t_ox=2e-9, n_body=1e24, phi_ms=-0.05
    vt = SiliconRustRuby.mosfet_threshold_voltage("NMOS", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0, 300.0, 0.0)
    assert_kind_of Float, vt
    # High body doping → V_t > 0.5 V
    assert vt > 0.5
  end

  # ---------------------------------------------------------------------------
  # mosfet-models
  # ---------------------------------------------------------------------------

  def test_evaluate_level1_defaults_returns_hash_with_all_keys
    r = SiliconRustRuby.evaluate_level1_defaults(1.8, 1.8, 0.0, 300.15)
    assert_kind_of Hash, r
    %i[id gm gds gmb cgs cgd cgb cbs cbd region].each do |key|
      assert r.key?(key), "result hash should contain key :#{key}"
    end
  end

  def test_evaluate_level1_defaults_saturation_region
    r = SiliconRustRuby.evaluate_level1_defaults(1.8, 1.8, 0.0, 300.15)
    assert_equal "saturation", r[:region]
  end

  def test_evaluate_level1_defaults_id_positive
    r = SiliconRustRuby.evaluate_level1_defaults(1.8, 1.8, 0.0, 300.15)
    assert r[:id] > 0.0, "drain current in saturation should be positive"
  end

  def test_evaluate_level1_cutoff
    # V_GS = 0 < V_t ≈ 0.42 V → cutoff
    r = SiliconRustRuby.evaluate_level1_defaults(0.0, 1.8, 0.0, 300.15)
    assert_includes ["cutoff", "subthreshold"], r[:region]
  end

  def test_evaluate_level1_explicit_params_saturation
    r = SiliconRustRuby.evaluate_level1(
      0.42,   # vt0
      220e-6, # kp
      0.05,   # lambda
      0.27,   # gamma
      0.84,   # phi
      1e-6,   # w
      130e-9, # l
      1.4,    # n_sub
      1.8,    # v_gs
      1.8,    # v_ds
      0.0,    # v_bs
      300.15  # t
    )
    assert_equal "saturation", r[:region]
    assert r[:id] > 0.0
  end

  # ---------------------------------------------------------------------------
  # fab-process-simulation — CrossSection wire format
  # ---------------------------------------------------------------------------

  def test_deposit_builds_wire_string
    cs = SiliconRustRuby.deposit("", "Si", 500.0)
    assert cs.start_with?("Si:"), "deposit on empty cs should start with Si:"
    assert cs.include?("500"), "thickness 500 nm should appear in wire string"
  end

  def test_deposit_prepends_new_layer
    cs = SiliconRustRuby.deposit("Si:500.0", "Poly", 50.0)
    assert cs.start_with?("Poly:"), "deposited layer should be on top"
    assert cs.include?("|Si:500.0"), "original Si layer should remain"
  end

  def test_deal_grove_oxidation_returns_sio2_wire
    cs = SiliconRustRuby.deal_grove_oxidation("Si:500.0", 5.0)
    assert_kind_of String, cs
    assert cs.start_with?("SiO2:"), "oxidation should produce an SiO2 top layer"
  end

  def test_deal_grove_oxidation_with_custom_a_b
    cs = SiliconRustRuby.deal_grove_oxidation("Si:500.0", 5.0, 0.165, 0.0117)
    assert cs.start_with?("SiO2:"), "custom A/B oxidation should also produce SiO2"
  end

  def test_etch_removes_top_layer
    cs = SiliconRustRuby.deposit("Si:500.0", "Poly", 50.0)
    # cs = "Poly:50.0|Si:500.0"
    cs = SiliconRustRuby.etch(cs, "Poly", 50.0)
    refute cs.include?("Poly:"), "etch should remove the Poly layer completely"
    assert cs.start_with?("Si:"), "Si substrate should be exposed after etch"
  end

  def test_implant_adds_doping_profile
    cs = SiliconRustRuby.deposit("", "Si", 500.0)
    # implant returns same layers (doping is internal); wire format unchanged for
    # structure but the call must not raise
    cs2 = SiliconRustRuby.implant(cs, "B", 30.0, 1e13)
    assert_kind_of String, cs2
  end

  def test_diffuse_returns_string
    cs = SiliconRustRuby.deposit("", "Si", 500.0)
    cs = SiliconRustRuby.implant(cs, "B", 30.0, 1e13)
    cs2 = SiliconRustRuby.diffuse(cs, 30.0)
    assert_kind_of String, cs2
  end

  def test_diffuse_with_custom_temperature
    cs = SiliconRustRuby.deposit("", "Si", 500.0)
    cs = SiliconRustRuby.implant(cs, "B", 30.0, 1e13)
    cs2 = SiliconRustRuby.diffuse(cs, 30.0, 1000.0)
    assert_kind_of String, cs2
  end

  def test_implant_range_returns_hash_with_rp_and_straggle
    result = SiliconRustRuby.implant_range("B", 30.0)
    assert_kind_of Hash, result
    assert result.key?(:rp),       "implant_range should return :rp"
    assert result.key?(:straggle), "implant_range should return :straggle"
    assert result[:rp] > 0.0,       ":rp should be positive"
    assert result[:straggle] > 0.0, ":straggle should be positive"
  end

  def test_implant_range_boron_30kev_approximate
    # SRIM table value: B at 30 keV → Rp ≈ 92 nm, straggle ≈ 38 nm
    result = SiliconRustRuby.implant_range("B", 30.0)
    assert_in_delta 92.0, result[:rp],       5.0
    assert_in_delta 38.0, result[:straggle], 5.0
  end

  def test_diffusivity_cm2_per_s_positive
    d = SiliconRustRuby.diffusivity_cm2_per_s("B", 1000.0)
    assert d > 0.0, "diffusivity should be positive"
  end

  def test_diffusivity_cm2_per_s_boron_1000c
    # Reference: B in Si at 1000 °C ≈ 1e-14 cm²/s
    d = SiliconRustRuby.diffusivity_cm2_per_s("B", 1000.0)
    assert_in_delta 1e-14, d, 1e-16
  end

  # ---------------------------------------------------------------------------
  # Error cases
  # ---------------------------------------------------------------------------

  def test_deposit_raises_for_pipe_in_material_name
    error = assert_raises(RuntimeError) do
      SiliconRustRuby.deposit("Si:500.0", "Bad|Material", 10.0)
    end
    assert_match(/\|/, error.message)
  end

  def test_deposit_raises_for_colon_in_material_name
    error = assert_raises(RuntimeError) do
      SiliconRustRuby.deposit("Si:500.0", "Bad:Name", 10.0)
    end
    assert_match(/:/, error.message)
  end

  def test_deal_grove_oxidation_raises_for_non_positive_time
    assert_raises(RuntimeError) do
      SiliconRustRuby.deal_grove_oxidation("Si:500.0", -1.0)
    end
  end

  def test_wrong_arg_count_raises
    assert_raises(RuntimeError) do
      SiliconRustRuby.thermal_voltage(300.0, 1.0)
    end
  end

  # ---------------------------------------------------------------------------
  # Namespace alias
  # ---------------------------------------------------------------------------

  def test_namespaced_alias_thermal_voltage
    direct  = SiliconRustRuby.thermal_voltage(300.0)
    aliased = CodingAdventures::SiliconRustRuby.thermal_voltage(300.0)
    assert_in_delta direct, aliased, EPSILON
  end

  def test_namespaced_alias_evaluate_level1_defaults
    direct  = SiliconRustRuby.evaluate_level1_defaults(1.8, 1.8, 0.0, 300.15)
    aliased = CodingAdventures::SiliconRustRuby.evaluate_level1_defaults(1.8, 1.8, 0.0, 300.15)
    assert_equal direct[:region], aliased[:region]
  end

  # ---------------------------------------------------------------------------
  # Version
  # ---------------------------------------------------------------------------

  def test_version_constant_defined
    assert_kind_of String, CodingAdventures::SiliconRustRuby::VERSION
    assert_match(/\A\d+\.\d+\.\d+\z/, CodingAdventures::SiliconRustRuby::VERSION)
  end
end
