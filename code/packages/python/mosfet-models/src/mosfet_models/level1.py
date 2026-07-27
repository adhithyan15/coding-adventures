"""SPICE Level-1 (Shockley) MOSFET I-V model.

The classical square-law model. ~10 parameters, simple equations, exact
analytical Jacobian. Pedagogy-grade — for hand calculations and the canonical
4-bit adder smoke test.
"""

from __future__ import annotations

from dataclasses import dataclass
from math import exp, isfinite, sqrt

from device_physics import thermal_voltage

OXIDE_PERMITTIVITY = 3.453133e-11


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
    LD: float = 0.0  # source/drain lateral diffusion length (m)
    TOX: float = 1e-7  # gate oxide thickness (m)
    RD: float = 0.0  # external drain resistance (ohm)
    IS: float = 1e-15  # saturation current (A)
    N_SUB: float = 1.4  # subthreshold slope factor
    T_NOM: float = 300.15  # nominal temperature (K)
    CGSO: float = 0.0  # gate-source overlap capacitance per width (F/m)
    CGDO: float = 0.0  # gate-drain overlap capacitance per width (F/m)
    CGBO: float = 0.0  # gate-bulk overlap capacitance per width (F/m)
    CBS: float = 0.0  # source-bulk zero-bias junction capacitance (F)
    CBD: float = 0.0  # drain-bulk zero-bias junction capacitance (F)
    PB: float = 0.8  # bulk junction potential (V)
    MJ: float = 0.5  # bulk junction grading coefficient
    FC: float = 0.5  # forward-bias depletion transition coefficient
    KF: float = 0.0  # flicker-noise coefficient
    AF: float = 1.0  # flicker-noise drain-current exponent
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


def bulk_junction_capacitance(
    zero_bias_capacitance: float,
    junction_voltage: float,
    junction_potential: float,
    grading_coefficient: float,
    forward_bias_coefficient: float = 0.5,
) -> float:
    """Return the Level-1 bulk-junction depletion capacitance.

    ``junction_voltage`` follows the diode convention: positive is forward
    body-to-terminal bias, negative is reverse bias.
    """

    if zero_bias_capacitance <= 0.0:
        return zero_bias_capacitance
    if (
        not isfinite(forward_bias_coefficient)
        or forward_bias_coefficient < 0.0
        or forward_bias_coefficient >= 1.0
    ):
        raise ValueError("MOSFET FC must be finite and in [0, 1)")
    if junction_potential <= 0.0 or grading_coefficient == 0.0:
        return zero_bias_capacitance
    normalized_voltage = junction_voltage / junction_potential
    if normalized_voltage < forward_bias_coefficient:
        return zero_bias_capacitance / (
            (1.0 - normalized_voltage) ** grading_coefficient
        )
    denominator = (1.0 - forward_bias_coefficient) ** (
        1.0 + grading_coefficient
    )
    continuation = (
        1.0
        - forward_bias_coefficient * (1.0 + grading_coefficient)
        + grading_coefficient * normalized_voltage
    )
    return zero_bias_capacitance * continuation / denominator


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
    if not isfinite(p.FC) or p.FC < 0.0 or p.FC >= 1.0:
        raise ValueError("MOSFET FC must be finite and in [0, 1)")
    effective_length = p.L - 2.0 * p.LD
    if not isfinite(p.LD) or p.LD < 0.0 or effective_length <= 0.0:
        raise ValueError(
            "MOSFET LD must be finite and non-negative with L - 2*LD > 0"
        )
    if not isfinite(p.TOX) or p.TOX <= 0.0:
        raise ValueError("MOSFET TOX must be finite and positive")
    if not isfinite(p.RD) or p.RD < 0.0:
        raise ValueError("MOSFET RD must be finite and non-negative")
    beta = p.KP * (p.W / effective_length)

    # Threshold with body effect. The formula is well-defined whenever
    # PHI - V_BS >= 0 (sqrt domain). If V_BS rises above PHI (heavy forward
    # body bias), clamp V_t to V_T0 since the model is invalid there.
    if p.PHI - V_BS >= 0:
        V_t = p.VT0 + p.GAMMA * (sqrt(p.PHI - V_BS) - sqrt(p.PHI))
    else:
        V_t = p.VT0

    V_OV = V_GS - V_t
    V_T = thermal_voltage(T)

    Cgs_overlap = p.CGSO * p.W
    Cgd_overlap = p.CGDO * p.W
    Cgb_overlap = p.CGBO * effective_length
    channel_capacitance = (
        p.W * effective_length * OXIDE_PERMITTIVITY / p.TOX
    )
    Cgd_intrinsic = 0.0
    Cgb_intrinsic = 0.0
    Cbs_bulk = bulk_junction_capacitance(p.CBS, V_BS, p.PB, p.MJ, p.FC)
    Cbd_bulk = bulk_junction_capacitance(p.CBD, V_BS - V_DS, p.PB, p.MJ, p.FC)

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
                Cgs=Cgs_overlap + channel_capacitance,
                Cgd=Cgd_overlap + Cgd_intrinsic,
                Cgb=Cgb_overlap + Cgb_intrinsic,
                Cbs=Cbs_bulk,
                Cbd=Cbd_bulk,
                region="subthreshold",
            )
        return MosResult(
            Id=0.0, gm=0.0, gds=0.0, gmb=0.0,
            Cgs=Cgs_overlap + channel_capacitance,
            Cgd=Cgd_overlap + Cgd_intrinsic,
            Cgb=Cgb_overlap + Cgb_intrinsic,
            Cbs=Cbs_bulk,
            Cbd=Cbd_bulk,
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
            Cgs=Cgs_overlap + channel_capacitance / 2.0,
            Cgd=Cgd_overlap + channel_capacitance / 2.0,
            Cgb=Cgb_overlap + Cgb_intrinsic,
            Cbs=Cbs_bulk,
            Cbd=Cbd_bulk,
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
        Cgs=Cgs_overlap + (2.0 / 3.0) * channel_capacitance,
        Cgd=Cgd_overlap,
        Cgb=Cgb_overlap,
        Cbs=Cbs_bulk,
        Cbd=Cbd_bulk,
        region="saturation",
    )
