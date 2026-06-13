// ============================================================================
// SiliconSimTest.java — JUnit 5 tests for the silicon JNI bindings
// ============================================================================
//
// These tests require the Rust cdylib to be built before running:
//
//   cargo build -p silicon-rust-jni --release
//
// The library path is configured in build.gradle.kts via jvmArgs.

package com.codingadventures.silicon;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class SiliconSimTest {

    // ---------------------------------------------------------------- constants

    @Test
    void testKBoltzmann() {
        double k = SiliconSim.kBoltzmann();
        // Exact CODATA 2018 value
        assertEquals(1.380649e-23, k, 1e-30);
    }

    @Test
    void testQElectron() {
        double q = SiliconSim.qElectron();
        assertEquals(1.602176634e-19, q, 1e-27);
    }

    @Test
    void testEps0() {
        double e0 = SiliconSim.eps0();
        assertTrue(e0 > 8.8e-12 && e0 < 8.9e-12, "ε₀ should be ~8.85e-12 F/m");
    }

    @Test
    void testEpsSi() {
        double eSi = SiliconSim.epsSi();
        assertTrue(eSi > 1.0e-10 && eSi < 1.1e-10, "ε_Si should be ~1.04e-10 F/m");
    }

    @Test
    void testEpsOx() {
        double eOx = SiliconSim.epsOx();
        assertTrue(eOx > 3.4e-11 && eOx < 3.5e-11, "ε_ox should be ~3.45e-11 F/m");
    }

    @Test
    void testNiAt300K() {
        double ni = SiliconSim.niAt300K();
        assertEquals(1.0e16, ni, 1.0);
    }

    @Test
    void testEgSiAt300K() {
        double eg = SiliconSim.egSiAt300K();
        assertEquals(1.12, eg, 1e-6);
    }

    @Test
    void testMuN300K() {
        double muN = SiliconSim.muN300K();
        // 1350 cm²/V·s = 0.135 m²/V·s
        assertEquals(0.135, muN, 1e-4);
    }

    @Test
    void testMuP300K() {
        double muP = SiliconSim.muP300K();
        // 480 cm²/V·s = 0.048 m²/V·s
        assertEquals(0.048, muP, 1e-4);
    }

    // --------------------------------------------------------- device-physics

    @Test
    void testThermalVoltageAt300K() {
        double vt = SiliconSim.thermalVoltage(300.0);
        // kT/q at 300 K ≈ 25.85 mV
        assertEquals(0.025852, vt, 1e-4);
    }

    @Test
    void testIntrinsicConcentrationAt300K() {
        double ni = SiliconSim.intrinsicConcentration(300.0);
        assertTrue(ni > 1e14 && ni < 1e18, "n_i(300 K) should be reasonable");
    }

    @Test
    void testIntrinsicConcentrationInvalidTemp() {
        assertThrows(SiliconException.class, () ->
                SiliconSim.intrinsicConcentration(-1.0));
    }

    @Test
    void testFermiPotentialP() {
        // p-type silicon, typical doping
        double phi = SiliconSim.fermiPotential(1e23, "p", 300.0);
        assertTrue(phi > 0.0 && phi < 1.0, "φ_F for p-type should be small positive");
    }

    @Test
    void testFermiPotentialN() {
        double phi = SiliconSim.fermiPotential(1e23, "n", 300.0);
        // Rust device-physics convention: φ_F for n-type = −V_T·ln(N/nᵢ) < 0
        assertTrue(phi < 0.0, "φ_F for n-type should be negative");
    }

    @Test
    void testFermiPotentialInvalidKind() {
        assertThrows(SiliconException.class, () ->
                SiliconSim.fermiPotential(1e23, "x", 300.0));
    }

    @Test
    void testPnJunctionBuiltInVoltage() {
        double vbi = SiliconSim.pnJunctionBuiltInVoltage(1e23, 1e22, 300.0);
        // Built-in voltage should be in the range 0.5–1.2 V for Si at room temp
        assertTrue(vbi > 0.5 && vbi < 1.2,
                "V_bi should be ~0.7 V, got " + vbi);
    }

    @Test
    void testPnJunctionDepletionWidth() {
        double w = SiliconSim.pnJunctionDepletionWidth(1e23, 1e22, 300.0, 0.0);
        assertTrue(w > 0.0, "depletion width should be positive");
    }

    @Test
    void testPnJunctionSaturationCurrent() {
        double is = SiliconSim.pnJunctionSaturationCurrent(
                1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6);
        assertTrue(is > 0.0 && is < 1.0, "I_S should be small but positive");
    }

    @Test
    void testPnJunctionCurrent() {
        // Forward-biased junction at 0.6 V
        double i = SiliconSim.pnJunctionCurrent(
                1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6, 0.6);
        assertTrue(i > 0.0, "forward-bias current should be positive");
    }

    @Test
    void testMosfetThresholdVoltageNmos() {
        double vt = SiliconSim.mosfetThresholdVoltage(
                "NMOS", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0, 300.0, 0.0);
        // N_body = 1e24 m⁻³ (1e18 cm⁻³) is a heavily-doped body, yielding
        // phi_f ≈ 0.476 V, gamma ≈ 0.334 V^½, and V_t ≈ 1.228 V.
        assertTrue(vt > 1.0 && vt < 1.5,
                "V_t(NMOS) should be ~1.228 V for N_body=1e24 m⁻³, got " + vt);
    }

    @Test
    void testMosfetThresholdVoltageInvalidType() {
        assertThrows(SiliconException.class, () ->
                SiliconSim.mosfetThresholdVoltage(
                        "XMOS", 130e-9, 1e-6, 2e-9, 1e24, 0.0, 0.0, 300.0, 0.0));
    }

    // ---------------------------------------------------------- mosfet-models

    @Test
    void testEvaluateLevel1DefaultsInSaturation() {
        // Default 130 nm NMOS, V_GS = 1.8 V (well above V_t), V_DS = 1.8 V
        MosResult r = SiliconSim.evaluateLevel1Defaults(1.8, 1.8, 0.0, 300.15);
        assertNotNull(r);
        assertEquals("saturation", r.region);
        assertTrue(r.id > 0.0, "Id should be positive in saturation");
        assertTrue(r.gm > 0.0, "gm should be positive");
    }

    @Test
    void testEvaluateLevel1DefaultsCutoff() {
        // V_GS = 0 V (below V_t = 0.42 V), V_DS = 1.8 V
        MosResult r = SiliconSim.evaluateLevel1Defaults(0.0, 1.8, 0.0, 300.15);
        assertNotNull(r);
        // Should be cutoff or subthreshold
        assertTrue(r.region.equals("cutoff") || r.region.equals("subthreshold"),
                "expected cutoff/subthreshold, got " + r.region);
    }

    @Test
    void testEvaluateLevel1FieldsAreNumbers() {
        MosResult r = SiliconSim.evaluateLevel1(
                0.42, 220e-6, 0.05, 0.27, 0.84, 1e-6, 130e-9, 1.4,
                1.8, 1.8, 0.0, 300.15);
        assertNotNull(r);
        assertFalse(Double.isNaN(r.id),  "id should not be NaN");
        assertFalse(Double.isNaN(r.gm),  "gm should not be NaN");
        assertFalse(Double.isNaN(r.gds), "gds should not be NaN");
        assertFalse(Double.isNaN(r.gmb), "gmb should not be NaN");
        assertFalse(Double.isNaN(r.cgs), "cgs should not be NaN");
        assertFalse(Double.isNaN(r.cgd), "cgd should not be NaN");
        assertFalse(Double.isNaN(r.cgb), "cgb should not be NaN");
        assertFalse(Double.isNaN(r.cbs), "cbs should not be NaN");
        assertFalse(Double.isNaN(r.cbd), "cbd should not be NaN");
        assertNotNull(r.region);
    }

    // ------------------------------------------------ fab-process-simulation

    @Test
    void testDepositOnEmpty() {
        String cs = SiliconSim.deposit("", "Si", 500.0);
        assertEquals("Si:500.0", cs);
    }

    @Test
    void testDepositNullCsIsEmpty() {
        // null cs should be treated as "" (empty cross-section)
        String cs = SiliconSim.deposit(null, "Si", 500.0);
        assertEquals("Si:500.0", cs);
    }

    @Test
    void testDepositPrependsLayer() {
        String cs = SiliconSim.deposit("Si:500.0", "SiO2", 4.8);
        assertTrue(cs.startsWith("SiO2:"), "new layer should be on top");
        assertTrue(cs.contains("|Si:"), "original layer should remain");
    }

    @Test
    void testDepositRejectsInjectedMaterial() {
        assertThrows(SiliconException.class, () ->
                SiliconSim.deposit("", "Si|Evil", 10.0));
    }

    @Test
    void testDepositRejectsColonInMaterial() {
        assertThrows(SiliconException.class, () ->
                SiliconSim.deposit("", "Si:100", 10.0));
    }

    @Test
    void testEtch() {
        String cs = SiliconSim.etch("SiO2:4.8|Si:500.0", "SiO2", 2.0);
        assertNotNull(cs);
        // Partial etch: SiO2 layer should be thinner or removed
    }

    @Test
    void testImplant() {
        String cs = SiliconSim.implant("Si:500.0", "B", 30.0, 1e13);
        assertNotNull(cs);
        // Implant doesn't change the stack geometry in the v0.1 model
        assertTrue(cs.contains("Si:"), "Si layer should still be there");
    }

    @Test
    void testDiffuse() {
        String cs = SiliconSim.diffuse("Si:500.0", 30.0);
        assertNotNull(cs);
    }

    @Test
    void testDiffuseWithTemp() {
        String cs = SiliconSim.diffuseWithTemp("Si:500.0", 30.0, 1000.0);
        assertNotNull(cs);
    }

    @Test
    void testDealGroveOxidation() {
        // Start with a Si substrate and grow oxide for 5 minutes
        String cs = SiliconSim.dealGroveOxidation("Si:500.0", 5.0);
        assertNotNull(cs);
        assertTrue(cs.startsWith("SiO2:"), "oxide should be on top");
        assertTrue(cs.contains("|Si:"), "Si substrate should remain");
    }

    @Test
    void testDealGroveOxidationCustom() {
        double a = 0.165;   // µm (dry O₂, 1000 °C)
        double b = 0.0117;  // µm²/hr
        String cs = SiliconSim.dealGroveOxidationCustom("Si:500.0", 5.0, a, b);
        assertNotNull(cs);
        assertTrue(cs.startsWith("SiO2:"), "oxide should be on top");
    }

    @Test
    void testImplantRange() {
        double[] rng = SiliconSim.implantRange("B", 30.0);
        assertNotNull(rng);
        assertEquals(2, rng.length);
        // Boron at 30 keV: Rp ≈ 92 nm, straggle ≈ 38 nm
        assertTrue(rng[0] > 0, "Rp should be positive");
        assertTrue(rng[1] > 0, "straggle should be positive");
        assertEquals(92.0, rng[0], 1.0);
        assertEquals(38.0, rng[1], 1.0);
    }

    @Test
    void testImplantRangeUnknown() {
        assertThrows(SiliconException.class, () ->
                SiliconSim.implantRange("Xe", 30.0));
    }

    @Test
    void testDiffusivityCm2PerS() {
        // Boron diffusivity at 1000 °C should be ~1e-14 cm²/s
        double d = SiliconSim.diffusivityCm2PerS("B", 1000.0);
        assertTrue(d > 0.0, "diffusivity should be positive for known species");
        assertEquals(1e-14, d, 1e-16);
    }

    @Test
    void testDiffusivityUnknownSpecies() {
        // Unknown species → conservative fallback of 1e-14 cm²/s (same as Boron at 1000 °C).
        // The function is infallible; it never throws even for unknown species.
        double d = SiliconSim.diffusivityCm2PerS("Xe", 1000.0);
        assertEquals(1e-14, d, 1e-16);
    }

    // ------------------------------------------------------- full process flow

    @Test
    void testProcessFlowBuildsCrossSection() {
        // Simulate a minimal NMOS gate stack
        String cs = SiliconSim.deposit("", "Si", 500.0);            // Si substrate
        cs = SiliconSim.dealGroveOxidation(cs, 5.0);                // grow gate oxide
        cs = SiliconSim.deposit(cs, "Poly", 50.0);                  // poly gate

        assertTrue(cs.startsWith("Poly:"), "poly should be on top");
        assertTrue(cs.contains("|SiO2:"), "oxide should be in middle");
        assertTrue(cs.contains("|Si:"), "substrate should be at bottom");
    }
}
