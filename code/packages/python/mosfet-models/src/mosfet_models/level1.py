"""SPICE Level-1 (Shockley) MOSFET I-V model.

The classical square-law model. ~10 parameters, simple equations, exact
analytical Jacobian. Pedagogy-grade — for hand calculations and the canonical
4-bit adder smoke test.
"""

from __future__ import annotations

from dataclasses import dataclass
from math import exp, sqrt

from device_physics import thermal_voltage


@dataclass(frozen=True, slots=True)
class Level1Params:
    """SPICE Level-1 parameter set. Defaults are typical for a 130 nm-style
    NMOS device."""

    VT0: float = 0.42  # threshold at V_BS=0 (V)
    KP: float = 220e-6  # transconductance, mu*C_ox (A/V^2)
    LAMBDA: float = 0.05  # channel-length modulation (1/V)
    GAMMA: float = 0.27  # body-effect coefficient (sqrt(V))
    PHI: float = 0.84  # surface potential at threshold, 2*phi_F (V)
    W: float = 1e-6  # channel width (m)
    L: float = 130e-9  # channel length (m)
    IS: float = 1e-15  # saturation current (A)
    N_SUB: float = 1.4  # subthreshold slope factor
    T_NOM: float = 300.15  # nominal temperature (K)
    subthreshold_enable: bool = True


@dataclass(frozen=True, slots=True)
class MosResult:
    """One operating-point evaluation."""

    Id: float
    gm: float
    gds: float
    gmb: float
    Cgs: float
    Cgd: float
    Cgb: float
    Cbs: float
    Cbd: float
    region: str  # 'cutoff', 'subthreshold', 'triode', 'saturation'


def evaluate_level1(
    params: Level1Params,
    V_GS: float,
    V_DS: float,
    V_BS: float = 0.0,
    T: float = 300.15,
) -> MosResult:
    """Compute Id and small-signal parameters at the given operating point.

    Returns positive Id for NMOS-style equations. PMOS callers must invert
    sign of inputs/outputs externally.
    """
    p = params
    beta = p.KP * (p.W / p.L)

    # Threshold with body effect. The formula is well-defined whenever
    # PHI - V_BS >= 0 (sqrt domain). If V_BS rises above PHI (heavy forward
    # body bias), clamp V_t to V_T0 since the model is invalid there.
    if p.PHI - V_BS >= 0:
        V_t = p.VT0 + p.GAMMA * (sqrt(p.PHI - V_BS) - sqrt(p.PHI))
    else:
        V_t = p.VT0

    V_OV = V_GS - V_t
    V_T = thermal_voltage(T)

    Cgs_off = (2.0 / 3.0) * p.W * p.L * p.KP / 1.0  # placeholder; Meyer model
    Cgd_off = 0.0
    Cgb_off = 0.0
    Cbs_off = 0.0
    Cbd_off = 0.0

    if V_OV <= 0:
        # Cutoff — optionally subthreshold.
        if p.subthreshold_enable:
            n = p.N_SUB
            Id_sub = (
                beta * n * V_T * V_T
                * exp(V_OV / (n * V_T))
                * (1.0 - exp(-V_DS / V_T))
            )
            gm_sub = Id_sub / (n * V_T)
            gds_sub = (beta * n * V_T) * exp(V_OV / (n * V_T)) * exp(-V_DS / V_T)
            return MosResult(
                Id=Id_sub, gm=gm_sub, gds=gds_sub, gmb=0.0,
                Cgs=Cgs_off, Cgd=Cgd_off, Cgb=Cgb_off, Cbs=Cbs_off, Cbd=Cbd_off,
                region="subthreshold",
            )
        return MosResult(
            Id=0.0, gm=0.0, gds=0.0, gmb=0.0,
            Cgs=Cgs_off, Cgd=Cgd_off, Cgb=Cgb_off, Cbs=Cbs_off, Cbd=Cbd_off,
            region="cutoff",
        )

    if V_DS < V_OV:
        # Triode (linear) region.
        Id = beta * (V_OV * V_DS - V_DS * V_DS / 2.0) * (1.0 + p.LAMBDA * V_DS)
        gm = beta * V_DS * (1.0 + p.LAMBDA * V_DS)
        gds = (
            beta * (V_OV - V_DS) * (1.0 + p.LAMBDA * V_DS)
            + beta * (V_OV * V_DS - V_DS * V_DS / 2.0) * p.LAMBDA
        )
        # Body transconductance via chain rule on V_t.
        if p.PHI - V_BS > 0:
            dVt_dVbs = -p.GAMMA / (2.0 * sqrt(p.PHI - V_BS))
            gmb = -gm * dVt_dVbs
        else:
            gmb = 0.0
        return MosResult(
            Id=Id, gm=gm, gds=gds, gmb=gmb,
            Cgs=Cgs_off / 2.0, Cgd=Cgd_off / 2.0, Cgb=Cgb_off,
            Cbs=Cbs_off, Cbd=Cbd_off,
            region="triode",
        )

    # Saturation.
    Id = (beta / 2.0) * V_OV * V_OV * (1.0 + p.LAMBDA * V_DS)
    gm = beta * V_OV * (1.0 + p.LAMBDA * V_DS)
    gds = (beta / 2.0) * V_OV * V_OV * p.LAMBDA
    if p.PHI - V_BS > 0:
        dVt_dVbs = -p.GAMMA / (2.0 * sqrt(p.PHI - V_BS))
        gmb = -gm * dVt_dVbs
    else:
        gmb = 0.0
    return MosResult(
        Id=Id, gm=gm, gds=gds, gmb=gmb,
        Cgs=(2.0 / 3.0) * Cgs_off, Cgd=0.0, Cgb=0.0, Cbs=0.0, Cbd=0.0,
        region="saturation",
    )
