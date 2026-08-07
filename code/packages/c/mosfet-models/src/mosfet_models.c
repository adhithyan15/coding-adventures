/*
 * mosfet_models.c — SPICE Level-1 (Shockley) MOSFET model (implementation).
 * ===========================================================================
 *
 * A faithful C port of the Rust `mosfet-models` crate. The evaluation is a direct
 * transcription of the square-law equations, with a smooth subthreshold branch
 * below threshold, channel-length modulation, the body effect (and its
 * derivative for gmb), and the piecewise Meyer capacitance model.
 *
 * The two non-arithmetic operations — the thermal voltage kT/q and the sqrt/exp
 * calls — come from the composed pure-ISO packages `c/device-physics`
 * (dp_thermal_voltage) and `c/float-math` (fm_sqrt / fm_exp), so nothing here
 * links a math library.
 */
#include "mosfet_models/mosfet_models.h"

#include "device_physics.h" /* dp_thermal_voltage */
#include "float_math.h"     /* fm_sqrt, fm_exp */

void mos_level1_params_default(MosLevel1Params *out) {
    if (!out) {
        return;
    }
    out->vt0 = 0.42;
    out->kp = 220e-6;
    out->lambda = 0.05;
    out->gamma = 0.27;
    out->phi = 0.84;
    out->w = 1e-6;
    out->l = 130e-9;
    out->is = 1e-15;
    out->n_sub = 1.4;
    out->t_nom = 300.15;
    out->cgso = 0.0;
    out->cgdo = 0.0;
    out->cgbo = 0.0;
    out->cbs = 0.0;
    out->cbd = 0.0;
    out->subthreshold_enable = 1;
}

const char *mos_region_str(MosRegion r) {
    switch (r) {
    case MOS_REGION_CUTOFF:
        return "cutoff";
    case MOS_REGION_SUBTHRESHOLD:
        return "subthreshold";
    case MOS_REGION_TRIODE:
        return "triode";
    case MOS_REGION_SATURATION:
        return "saturation";
    }
    return "cutoff"; /* unreachable; keeps the function total */
}

MosResult mos_evaluate_level1(const MosLevel1Params *p, double v_gs, double v_ds,
                              double v_bs, double t) {
    MosResult res;
    double beta = p->kp * (p->w / p->l);
    double v_t;
    double v_ov;
    double vth;
    double cgs_overlap, cgd_overlap, cgb_overlap, cgs_intrinsic;
    double dvt_dvbs;

    /* Threshold with the body effect; sqrt(PHI - V_BS) is valid only for
     * PHI >= V_BS. Under strong forward body bias, clamp V_t to VT0. */
    if (p->phi - v_bs >= 0.0) {
        v_t = p->vt0 + p->gamma * (fm_sqrt(p->phi - v_bs) - fm_sqrt(p->phi));
    } else {
        v_t = p->vt0;
    }

    v_ov = v_gs - v_t;
    vth = dp_thermal_voltage(t); /* kT/q at the operating temperature */

    cgs_overlap = p->cgso * p->w;
    cgd_overlap = p->cgdo * p->w;
    cgb_overlap = p->cgbo * p->l;
    /* Meyer intrinsic gate cap, saturation reference: (2/3) W L Cox (Cox~KP). */
    cgs_intrinsic = (2.0 / 3.0) * p->w * p->l * p->kp;

    /* --- Cutoff / subthreshold --- */
    if (v_ov <= 0.0) {
        if (p->subthreshold_enable) {
            double n = p->n_sub;
            double e_ov = fm_exp(v_ov / (n * vth));
            double id_sub = beta * n * vth * vth * e_ov * (1.0 - fm_exp(-v_ds / vth));
            double gm_sub = id_sub / (n * vth);
            double gds_sub = (beta * n * vth) * e_ov * fm_exp(-v_ds / vth);
            res.id = id_sub;
            res.gm = gm_sub;
            res.gds = gds_sub;
            res.gmb = 0.0;
            res.cgs = cgs_overlap + cgs_intrinsic;
            res.cgd = cgd_overlap;
            res.cgb = cgb_overlap;
            res.cbs = p->cbs;
            res.cbd = p->cbd;
            res.region = MOS_REGION_SUBTHRESHOLD;
            return res;
        }
        /* Hard cutoff. */
        res.id = 0.0;
        res.gm = 0.0;
        res.gds = 0.0;
        res.gmb = 0.0;
        res.cgs = cgs_overlap + cgs_intrinsic;
        res.cgd = cgd_overlap;
        res.cgb = cgb_overlap;
        res.cbs = p->cbs;
        res.cbd = p->cbd;
        res.region = MOS_REGION_CUTOFF;
        return res;
    }

    /* Body-effect derivative for gmb: dV_t/dV_BS = -gamma / (2 sqrt(PHI - V_BS)). */
    if (p->phi - v_bs > 0.0) {
        dvt_dvbs = -p->gamma / (2.0 * fm_sqrt(p->phi - v_bs));
    } else {
        dvt_dvbs = 0.0;
    }

    /* --- Triode: 0 < V_DS < V_OV --- */
    if (v_ds < v_ov) {
        double clm = 1.0 + p->lambda * v_ds;
        double id = beta * (v_ov * v_ds - v_ds * v_ds / 2.0) * clm;
        double gm = beta * v_ds * clm;
        double gds = beta * (v_ov - v_ds) * clm +
                     beta * (v_ov * v_ds - v_ds * v_ds / 2.0) * p->lambda;
        res.id = id;
        res.gm = gm;
        res.gds = gds;
        res.gmb = -gm * dvt_dvbs;
        res.cgs = cgs_overlap + cgs_intrinsic / 2.0;
        res.cgd = cgd_overlap + cgs_intrinsic / 2.0;
        res.cgb = cgb_overlap;
        res.cbs = p->cbs;
        res.cbd = p->cbd;
        res.region = MOS_REGION_TRIODE;
        return res;
    }

    /* --- Saturation: V_DS >= V_OV --- */
    {
        double clm = 1.0 + p->lambda * v_ds;
        double id = (beta / 2.0) * v_ov * v_ov * clm;
        double gm = beta * v_ov * clm;
        double gds = (beta / 2.0) * v_ov * v_ov * p->lambda;
        res.id = id;
        res.gm = gm;
        res.gds = gds;
        res.gmb = -gm * dvt_dvbs;
        res.cgs = cgs_overlap + (2.0 / 3.0) * cgs_intrinsic;
        res.cgd = cgd_overlap;
        res.cgb = cgb_overlap;
        res.cbs = p->cbs;
        res.cbd = p->cbd;
        res.region = MOS_REGION_SATURATION;
        return res;
    }
}

MosResult mosfet_dc(MosfetType type, const MosLevel1Params *p, double v_gs,
                    double v_ds, double v_bs, double t) {
    if (type == MOS_PMOS) {
        /* Negate the inputs, evaluate as NMOS, then negate the drain current so
         * the caller sees PMOS convention (negative Id for drain-out current). */
        MosResult r = mos_evaluate_level1(p, -v_gs, -v_ds, -v_bs, t);
        r.id = -r.id;
        return r;
    }
    return mos_evaluate_level1(p, v_gs, v_ds, v_bs, t);
}
