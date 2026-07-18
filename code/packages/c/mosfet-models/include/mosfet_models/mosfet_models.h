/*
 * mosfet_models/mosfet_models.h — SPICE Level-1 (Shockley) MOSFET I-V model.
 * ===========================================================================
 *
 * The C port of the Rust `mosfet-models` crate, a bucket-A port of the CCPP02
 * campaign: a pure-ISO crate that needs no OS, so it rides the `iso-harness`
 * (links nothing, strict-conformance flags on).
 *
 * The classical square-law MOSFET model — the one you use for hand calculations
 * and canonical CMOS smoke tests. Given a bias point (V_GS, V_DS, V_BS, T) it
 * returns the drain current, the small-signal conductances (gm / gds / gmb), and
 * the Meyer intrinsic + overlap capacitances, along with the operating region.
 *
 *   Region       Condition            Id
 *   ----------   ------------------   ------------------------------------------
 *   Cutoff       V_OV <= 0            0 (or the subthreshold exp if enabled)
 *   Triode       0 < V_DS < V_OV      beta (V_OV V_DS - V_DS^2/2)(1 + lambda V_DS)
 *   Saturation   V_DS >= V_OV         (beta/2) V_OV^2 (1 + lambda V_DS)
 *
 * where beta = KP (W/L) and V_OV = V_GS - V_t.
 *
 * COMPOSES `c/device-physics` (the thermal voltage kT/q) and `c/float-math` (the
 * from-scratch sqrt / exp). Everything is by value — no allocation, no OS, no
 * libm; the math is computed from scratch.
 */
#ifndef MOSFET_MODELS_MOSFET_MODELS_H
#define MOSFET_MODELS_MOSFET_MODELS_H

#ifdef __cplusplus
extern "C" {
#endif

/*
 * SPICE Level-1 parameter set. Defaults (via mos_level1_params_default) are
 * typical for a 130 nm NMOS device at room temperature.
 */
typedef struct {
    double vt0;    /* threshold voltage at V_BS = 0 [V] */
    double kp;     /* transconductance parameter mu*Cox [A/V^2] */
    double lambda; /* channel-length modulation [1/V] */
    double gamma;  /* body-effect coefficient [sqrt(V)] */
    double phi;    /* surface potential 2*phi_F [V] */
    double w;      /* channel width [m] */
    double l;      /* channel length [m] */
    double is;     /* drain-body saturation current [A] (subthreshold floor) */
    double n_sub;  /* subthreshold slope factor n */
    double t_nom;  /* nominal temperature [K] */
    double cgso;   /* gate-source overlap cap per width [F/m] */
    double cgdo;   /* gate-drain overlap cap per width [F/m] */
    double cgbo;   /* gate-bulk overlap cap per length [F/m] */
    double cbs;    /* source-bulk zero-bias junction cap [F] */
    double cbd;    /* drain-bulk zero-bias junction cap [F] */
    int subthreshold_enable; /* nonzero → model subthreshold current below V_t */
} MosLevel1Params;

/* Fill *out with the default parameter set (130 nm NMOS, room temperature). */
void mos_level1_params_default(MosLevel1Params *out);

/* MOSFET operating region. */
typedef enum {
    MOS_REGION_CUTOFF,
    MOS_REGION_SUBTHRESHOLD,
    MOS_REGION_TRIODE,
    MOS_REGION_SATURATION
} MosRegion;

/* ASCII region name ("cutoff" / "subthreshold" / "triode" / "saturation"). */
const char *mos_region_str(MosRegion r);

/* One operating-point evaluation. Conductances in A/V, capacitances in F. */
typedef struct {
    double id;  /* drain current [A] */
    double gm;  /* transconductance dId/dVgs [A/V] */
    double gds; /* output conductance dId/dVds [A/V] */
    double gmb; /* body transconductance dId/dVbs [A/V] */
    double cgs; /* gate-source capacitance [F] */
    double cgd; /* gate-drain capacitance [F] */
    double cgb; /* gate-bulk capacitance [F] */
    double cbs; /* source-bulk capacitance [F] */
    double cbd; /* drain-bulk capacitance [F] */
    MosRegion region;
} MosResult;

/*
 * Evaluate the Level-1 model at a bias point in NMOS convention (positive V_GS
 * for inversion). For PMOS, use mosfet_dc, which handles the sign flips.
 */
MosResult mos_evaluate_level1(const MosLevel1Params *p, double v_gs, double v_ds,
                              double v_bs, double t);

/* Whether a device is n-channel or p-channel. */
typedef enum { MOS_NMOS, MOS_PMOS } MosfetType;

/*
 * Evaluate a MOSFET of the given type. NMOS uses the natural polarity; PMOS
 * negates the input voltages before evaluation and negates the resulting Id, so
 * the caller sees conventional PMOS convention. (A Level-1 "model" that only
 * holds parameters, like the Rust `Level1Model`, is simply
 * mosfet_dc(MOS_NMOS, ...) / mos_evaluate_level1.)
 */
MosResult mosfet_dc(MosfetType type, const MosLevel1Params *p, double v_gs,
                    double v_ds, double v_bs, double t);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* MOSFET_MODELS_MOSFET_MODELS_H */
