// ============================================================================
// SiliconSim.java — JNI wrapper for the Rust silicon simulation stack
// ============================================================================
//
// This class provides static native methods backed by the Rust cdylib
// `silicon_rust_jni` (libsilicon_rust_jni.so / .dylib / .dll).
//
// The static initializer loads the library.  Tests must pass
// -Djava.library.path pointing to the directory containing the compiled
// cdylib; see build.gradle.kts.
//
// ## Cross-section wire format
//
// A CrossSection is transported across the JNI boundary as a
// pipe-separated string:
//
//   ""                               empty cross-section
//   "Si:500.0"                       bare silicon substrate
//   "SiO2:4.8|Si:500.0"             gate oxide on silicon
//   "Poly:50.0|SiO2:4.8|Si:500.0"  poly gate on gate oxide
//
// Passing null for the cs argument is equivalent to passing "".
// Material and species names must not contain '|' or ':'.

package com.codingadventures.silicon;

/**
 * Static gateway to the Rust silicon simulation stack via JNI.
 *
 * <p>All methods are {@code static} — no instantiation needed.
 *
 * <p>Methods that can fail throw {@link SiliconException} (a
 * {@link RuntimeException}) on error.
 */
public class SiliconSim {

    static {
        System.loadLibrary("silicon_rust_jni");
    }

    private SiliconSim() {}  // utility class

    // ---------------------------------------------------------------- constants

    /** Boltzmann constant k_B [J/K]. */
    public static native double kBoltzmann();
    /** Elementary charge q [C]. */
    public static native double qElectron();
    /** Vacuum permittivity ε₀ [F/m]. */
    public static native double eps0();
    /** Silicon permittivity ε_Si = 11.7 × ε₀ [F/m]. */
    public static native double epsSi();
    /** SiO₂ permittivity ε_ox = 3.9 × ε₀ [F/m]. */
    public static native double epsOx();
    /** Silicon intrinsic carrier concentration at 300 K [/m³]. */
    public static native double niAt300K();
    /** Silicon bandgap at 300 K [eV]. */
    public static native double egSiAt300K();
    /** Electron drift mobility at 300 K [m²/V·s]. */
    public static native double muN300K();
    /** Hole drift mobility at 300 K [m²/V·s]. */
    public static native double muP300K();

    // --------------------------------------------------------- device-physics

    /**
     * Thermal voltage V_T = kT/q [V].
     *
     * <p>At 300 K this is approximately 25.85 mV.
     *
     * @param tKelvin temperature [K]
     * @return thermal voltage [V]
     */
    public static native double thermalVoltage(double tKelvin);

    /**
     * Intrinsic carrier concentration n_i(T) [/m³].
     *
     * @param tKelvin temperature [K]; must be in [1, 1000]
     * @throws SiliconException if temperature is out of range
     */
    public static native double intrinsicConcentration(double tKelvin)
            throws SiliconException;

    /**
     * Fermi potential φ_F [V].
     *
     * @param nDoping doping concentration [/m³] (must be positive)
     * @param kind    {@code "p"} or {@code "n"}
     * @param tKelvin temperature [K]
     * @throws SiliconException on invalid kind or non-positive doping
     */
    public static native double fermiPotential(
            double nDoping, String kind, double tKelvin)
            throws SiliconException;

    /**
     * PN junction built-in (contact) voltage V_bi [V].
     *
     * @param na doping concentration of the p-side [/m³] (must be positive)
     * @param nd doping concentration of the n-side [/m³] (must be positive)
     * @param t  temperature [K]
     * @throws SiliconException on invalid parameters
     */
    public static native double pnJunctionBuiltInVoltage(
            double na, double nd, double t) throws SiliconException;

    /**
     * Depletion-region total width W [m] under applied bias.
     *
     * @param na       p-side doping [/m³]
     * @param nd       n-side doping [/m³]
     * @param t        temperature [K]
     * @param vApplied applied voltage [V]; positive = forward bias
     * @throws SiliconException on invalid parameters
     */
    public static native double pnJunctionDepletionWidth(
            double na, double nd, double t, double vApplied)
            throws SiliconException;

    /**
     * Shockley diode saturation current I_S [A].
     *
     * @param na   p-side doping [/m³]
     * @param nd   n-side doping [/m³]
     * @param a    junction area [m²]
     * @param t    temperature [K]
     * @param tauN minority-carrier electron lifetime [s]
     * @param tauP minority-carrier hole lifetime [s]
     * @throws SiliconException on invalid parameters
     */
    public static native double pnJunctionSaturationCurrent(
            double na, double nd, double a, double t,
            double tauN, double tauP) throws SiliconException;

    /**
     * Shockley diode current I = I_S (e^(V/V_T) − 1) [A].
     *
     * @param na   p-side doping [/m³]
     * @param nd   n-side doping [/m³]
     * @param a    junction area [m²]
     * @param t    temperature [K]
     * @param tauN minority-carrier electron lifetime [s]
     * @param tauP minority-carrier hole lifetime [s]
     * @param v    applied junction voltage [V]
     * @throws SiliconException on invalid parameters
     */
    public static native double pnJunctionCurrent(
            double na, double nd, double a, double t,
            double tauN, double tauP, double v) throws SiliconException;

    /**
     * MOSFET threshold voltage V_t [V] with body effect.
     *
     * @param deviceType {@code "NMOS"} or {@code "PMOS"}
     * @param l          channel length [m]
     * @param w          channel width [m]
     * @param tOx        gate-oxide thickness [m]
     * @param nBody      body doping [/m³]
     * @param phiMs      gate–body work-function difference [V]
     * @param qOx        oxide trapped charge per area [C/m²]
     * @param t          temperature [K]
     * @param vSb        source–body reverse bias [V]
     * @throws SiliconException on invalid parameters
     */
    public static native double mosfetThresholdVoltage(
            String deviceType, double l, double w, double tOx,
            double nBody, double phiMs, double qOx, double t, double vSb)
            throws SiliconException;

    // ---------------------------------------------------------- mosfet-models

    /**
     * Evaluate the Level-1 (Shichman–Hodges) MOSFET model at the given
     * operating point with explicit model parameters.
     *
     * @param vt0    threshold voltage at zero source-body bias [V]
     * @param kp     process transconductance μₙCₒₓ [A/V²]
     * @param lambda channel-length modulation [1/V]
     * @param gamma  body-effect coefficient γ [√V]
     * @param phi    surface potential at threshold 2φ_F [V]
     * @param w      channel width [m]
     * @param l      channel length [m]
     * @param nSub   subthreshold slope factor n
     * @param vGs    gate–source voltage [V]
     * @param vDs    drain–source voltage [V]
     * @param vBs    body–source voltage [V]
     * @param t      temperature [K]
     * @throws SiliconException on allocation failure (extremely rare)
     */
    public static native MosResult evaluateLevel1(
            double vt0, double kp, double lambda, double gamma, double phi,
            double w, double l, double nSub,
            double vGs, double vDs, double vBs, double t)
            throws SiliconException;

    /**
     * Evaluate the Level-1 MOSFET model using the default 130 nm NMOS
     * parameters (Sky130-calibrated).
     *
     * @param vGs gate–source voltage [V]
     * @param vDs drain–source voltage [V]
     * @param vBs body–source voltage [V]
     * @param t   temperature [K]
     * @throws SiliconException on allocation failure (extremely rare)
     */
    public static native MosResult evaluateLevel1Defaults(
            double vGs, double vDs, double vBs, double t)
            throws SiliconException;

    // ------------------------------------------------ fab-process-simulation

    /**
     * Deposit a new layer on top of the cross-section.
     *
     * @param cs          cross-section wire string (null treated as "")
     * @param material    material name (must not contain '|' or ':')
     * @param thicknessNm layer thickness [nm]
     * @return updated cross-section wire string
     * @throws SiliconException on invalid material name or malformed wire
     */
    public static native String deposit(
            String cs, String material, double thicknessNm)
            throws SiliconException;

    /**
     * Etch material from the cross-section to a given depth.
     *
     * @param cs      cross-section wire string
     * @param target  material to etch (must not contain '|' or ':')
     * @param depthNm etch depth [nm]
     * @return updated cross-section wire string
     * @throws SiliconException on invalid target or malformed wire
     */
    public static native String etch(
            String cs, String target, double depthNm)
            throws SiliconException;

    /**
     * Ion implant into the cross-section.
     *
     * @param cs        cross-section wire string
     * @param species   ion species (e.g. "B", "P", "As"; must not contain '|' or ':')
     * @param energyKev implant energy [keV]
     * @param doseCm2   dose [ions/cm²]
     * @return updated cross-section wire string
     * @throws SiliconException on invalid species or malformed wire
     */
    public static native String implant(
            String cs, String species, double energyKev, double doseCm2)
            throws SiliconException;

    /**
     * Diffusion anneal at the default temperature (1000 °C).
     *
     * @param cs      cross-section wire string
     * @param timeMin anneal duration [minutes]
     * @return updated cross-section wire string
     * @throws SiliconException on malformed wire
     */
    public static native String diffuse(String cs, double timeMin)
            throws SiliconException;

    /**
     * Diffusion anneal at an explicit temperature.
     *
     * @param cs           cross-section wire string
     * @param timeMin      anneal duration [minutes]
     * @param temperatureC anneal temperature [°C]
     * @return updated cross-section wire string
     * @throws SiliconException on malformed wire
     */
    public static native String diffuseWithTemp(
            String cs, double timeMin, double temperatureC)
            throws SiliconException;

    /**
     * Thermal oxidation using the Deal-Grove model with default dry-O₂
     * parameters at 1000 °C.
     *
     * @param cs      cross-section wire string (top layer must be Si)
     * @param timeMin oxidation duration [minutes]
     * @return updated cross-section wire string with new SiO₂ top layer
     * @throws SiliconException on non-Si surface or malformed wire
     */
    public static native String dealGroveOxidation(String cs, double timeMin)
            throws SiliconException;

    /**
     * Thermal oxidation using the Deal-Grove model with custom rate constants.
     *
     * @param cs          cross-section wire string
     * @param timeMin     oxidation duration [minutes]
     * @param aUm         Deal-Grove A parameter [µm]
     * @param bUm2PerHr   Deal-Grove B parameter [µm²/hr]
     * @return updated cross-section wire string
     * @throws SiliconException on non-Si surface or malformed wire
     */
    public static native String dealGroveOxidationCustom(
            String cs, double timeMin, double aUm, double bUm2PerHr)
            throws SiliconException;

    /**
     * Look up the ion-implant projected range and straggle from SRIM tables.
     *
     * @param species   ion species (e.g. "B", "P", "As")
     * @param energyKev implant energy [keV]
     * @return {@code double[2]} = {R_p [nm], ΔR_p [nm] (straggle)}
     * @throws SiliconException if the species/energy combination is not in
     *         the table
     */
    public static native double[] implantRange(String species, double energyKev)
            throws SiliconException;

    /**
     * Diffusivity of an ion species in silicon at a given temperature.
     *
     * <p>Infallible — returns 0.0 for unknown species.
     *
     * @param species     ion species (e.g. "B", "P")
     * @param temperatureC temperature [°C]
     * @return diffusivity [cm²/s]
     */
    public static native double diffusivityCm2PerS(
            String species, double temperatureC);
}
