# frozen_string_literal: true

# coding_adventures/silicon_rust_ruby.rb — Ruby façade for the silicon sim stack
# ================================================================================
#
# After `require "coding_adventures/silicon_rust_ruby"`, all 26 functions are
# available on `SiliconRustRuby`:
#
#   SiliconRustRuby.k_boltzmann              # => 1.380649e-23 (J/K)
#   SiliconRustRuby.thermal_voltage(300)     # => 0.025852 (V)
#   SiliconRustRuby.evaluate_level1_defaults(1.8, 1.8, 0.0, 300.15)
#   # => { id: Float, gm: Float, ..., region: "saturation" }
#
# The module is defined by the native extension when it loads.  This file:
#
#   1. Loads the VERSION constant.
#   2. Triggers the native loader, which `require`s the .{so,bundle,dll}.
#   3. Provides `CodingAdventures::SiliconRustRuby.method(...)` as an alias
#      so callers who prefer the namespaced form get it.

require_relative "silicon_rust_ruby/version"
require_relative "silicon_rust_ruby/native_loader"

# `CodingAdventures::SiliconRustRuby` is the namespaced module defined in
# version.rb.  `::SiliconRustRuby` (top level) is the module the native
# extension registered.  We bridge them by delegating every method call.
module CodingAdventures
  module SiliconRustRuby
    class << self
      # Physical constants
      def k_boltzmann   = ::SiliconRustRuby.k_boltzmann
      def q_electron    = ::SiliconRustRuby.q_electron
      def eps0          = ::SiliconRustRuby.eps0
      def eps_si        = ::SiliconRustRuby.eps_si
      def eps_ox        = ::SiliconRustRuby.eps_ox
      def ni_at_300k    = ::SiliconRustRuby.ni_at_300k
      def eg_si_at_300k = ::SiliconRustRuby.eg_si_at_300k
      def mu_n_300k     = ::SiliconRustRuby.mu_n_300k
      def mu_p_300k     = ::SiliconRustRuby.mu_p_300k

      # device-physics
      def thermal_voltage(t)              = ::SiliconRustRuby.thermal_voltage(t)
      def intrinsic_concentration(t)      = ::SiliconRustRuby.intrinsic_concentration(t)
      def fermi_potential(n, kind, t)     = ::SiliconRustRuby.fermi_potential(n, kind, t)

      def pn_junction_built_in_voltage(na, nd, t) =
        ::SiliconRustRuby.pn_junction_built_in_voltage(na, nd, t)

      def pn_junction_depletion_width(na, nd, t, v) =
        ::SiliconRustRuby.pn_junction_depletion_width(na, nd, t, v)

      def pn_junction_saturation_current(na, nd, a, t, tau_n, tau_p) =
        ::SiliconRustRuby.pn_junction_saturation_current(na, nd, a, t, tau_n, tau_p)

      def pn_junction_current(na, nd, a, t, tau_n, tau_p, v) =
        ::SiliconRustRuby.pn_junction_current(na, nd, a, t, tau_n, tau_p, v)

      def mosfet_threshold_voltage(device_type, l, w, t_ox, n_body, phi_ms, q_ox, t, v_sb) =
        ::SiliconRustRuby.mosfet_threshold_voltage(device_type, l, w, t_ox, n_body, phi_ms, q_ox, t, v_sb)

      # mosfet-models
      def evaluate_level1(vt0, kp, lambda_val, gamma, phi, w, l, n_sub, v_gs, v_ds, v_bs, t) =
        ::SiliconRustRuby.evaluate_level1(vt0, kp, lambda_val, gamma, phi, w, l, n_sub, v_gs, v_ds, v_bs, t)

      def evaluate_level1_defaults(v_gs, v_ds, v_bs, t) =
        ::SiliconRustRuby.evaluate_level1_defaults(v_gs, v_ds, v_bs, t)

      # fab-process-simulation
      def deal_grove_oxidation(cs_str, time_min, *rest) =
        ::SiliconRustRuby.deal_grove_oxidation(cs_str, time_min, *rest)

      def deposit(cs_str, material, thickness_nm) =
        ::SiliconRustRuby.deposit(cs_str, material, thickness_nm)

      def etch(cs_str, target_material, depth_nm) =
        ::SiliconRustRuby.etch(cs_str, target_material, depth_nm)

      def implant(cs_str, species, energy_kev, dose_cm2) =
        ::SiliconRustRuby.implant(cs_str, species, energy_kev, dose_cm2)

      def diffuse(cs_str, time_min, *rest) =
        ::SiliconRustRuby.diffuse(cs_str, time_min, *rest)

      def implant_range(species, energy_kev) =
        ::SiliconRustRuby.implant_range(species, energy_kev)

      def diffusivity_cm2_per_s(species, temperature_c) =
        ::SiliconRustRuby.diffusivity_cm2_per_s(species, temperature_c)
    end
  end
end
