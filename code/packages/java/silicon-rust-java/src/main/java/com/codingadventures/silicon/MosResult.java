// ============================================================================
// MosResult.java — Level-1 MOSFET DC operating-point result
// ============================================================================
//
// This class is constructed from the Rust side via JNI NewObjectA.  The
// constructor signature used is:
//
//   (DDDDDDDDDLjava/lang/String;)V
//
// — nine `double` arguments in the order declared below, then the region
// string.  The Rust code looks this class up at runtime with FindClass
// and GetMethodID, so the constructor signature here must match exactly.

package com.codingadventures.silicon;

/**
 * DC operating-point result from the Level-1 MOSFET model.
 *
 * <p>All currents in Amperes, all conductances in Siemens, all capacitances
 * in Farads.
 *
 * <p>Instances are created by the Rust native library via JNI and are
 * immutable.
 */
public final class MosResult {

    // ------------------------------------------------------------------ fields

    /** Drain current I_D [A]. Positive for NMOS in saturation. */
    public final double id;
    /** Transconductance g_m = ∂I_D/∂V_GS [A/V]. */
    public final double gm;
    /** Output conductance g_ds = ∂I_D/∂V_DS [A/V]. */
    public final double gds;
    /** Body transconductance g_mb = ∂I_D/∂V_BS [A/V]. */
    public final double gmb;
    /** Gate–source capacitance C_GS [F]. */
    public final double cgs;
    /** Gate–drain capacitance C_GD [F]. */
    public final double cgd;
    /** Gate–bulk capacitance C_GB [F]. */
    public final double cgb;
    /** Source–bulk capacitance C_BS [F]. */
    public final double cbs;
    /** Drain–bulk capacitance C_BD [F]. */
    public final double cbd;
    /** Operating region: "cutoff", "subthreshold", "triode", or "saturation". */
    public final String region;

    // --------------------------------------------------------------- constructor
    //
    // This constructor is called by the Rust JNI code via NewObjectA.
    // The order of arguments must not change without updating the JNI
    // signature in silicon-rust-jni/src/lib.rs.

    /** Construct a {@code MosResult} (called from JNI). */
    public MosResult(double id, double gm, double gds, double gmb,
                     double cgs, double cgd, double cgb, double cbs, double cbd,
                     String region) {
        this.id     = id;
        this.gm     = gm;
        this.gds    = gds;
        this.gmb    = gmb;
        this.cgs    = cgs;
        this.cgd    = cgd;
        this.cgb    = cgb;
        this.cbs    = cbs;
        this.cbd    = cbd;
        this.region = region;
    }

    // --------------------------------------------------------------- overrides

    @Override
    public String toString() {
        return "MosResult{region=" + region +
               ", id=" + id + " A" +
               ", gm=" + gm + " S" +
               ", gds=" + gds + " S}";
    }
}
