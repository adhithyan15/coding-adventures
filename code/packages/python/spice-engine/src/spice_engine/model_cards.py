"""SPICE model-card normalization helpers.

The engine still uses programmatic circuit construction.  These helpers provide
the shared `.model` alias surface that the deck parser can target without
duplicating diode, BJT, JFET, and Level-1 MOS parameter mapping logic.
"""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, replace

from mosfet_models import MOSFET, Level1Model, Level1Params, MosfetType

from spice_engine.elements import BJT, JFET, Diode, Mosfet


@dataclass(frozen=True, slots=True)
class NormalizedModelCard:
    """A normalized SPICE `.model` card with stable cross-language keys."""

    name: str
    kind: str
    parameters: dict[str, float]
    unsupported_parameters: tuple[str, ...] = ()


_MODEL_TYPE_ALIASES: dict[str, str] = {
    "D": "D",
    "DIODE": "D",
    "NPN": "NPN",
    "PNP": "PNP",
    "NJF": "NJF",
    "NJFET": "NJF",
    "NJ": "NJF",
    "PJF": "PJF",
    "PJFET": "PJF",
    "PJ": "PJF",
    "NMOS": "NMOS",
    "NCH": "NMOS",
    "PMOS": "PMOS",
    "PCH": "PMOS",
}

_DIODE_PARAMETER_ALIASES: dict[str, str] = {
    "IS": "IS",
    "JS": "IS",
    "VT": "VT",
    "V_T": "VT",
    "N": "N",
    "BV": "BV",
    "IBV": "IBV",
    "CJO": "CJO",
    "CJ": "CJO",
    "CJ0": "CJO",
    "TT": "TT",
}

_BJT_PARAMETER_ALIASES: dict[str, str] = {
    "IS": "IS",
    "BF": "BF",
    "BETA": "BF",
    "BETA_F": "BF",
    "HFE": "BF",
    "VT": "VT",
    "V_T": "VT",
    "CJE": "CJE",
    "CJE0": "CJE",
    "CBE": "CJE",
    "CJC": "CJC",
    "CJC0": "CJC",
    "CBC": "CJC",
    "TF": "TF",
    "TR": "TR",
}

_JFET_PARAMETER_ALIASES: dict[str, str] = {
    "BETA": "BETA",
    "BET": "BETA",
    "VTO": "VTO",
    "VT0": "VTO",
    "VTH": "VTO",
    "LAMBDA": "LAMBDA",
    "LAM": "LAMBDA",
}

_MOS_LEVEL1_PARAMETER_ALIASES: dict[str, str] = {
    "LEVEL": "LEVEL",
    "VT0": "VT0",
    "VTO": "VT0",
    "VTH": "VT0",
    "KP": "KP",
    "LAMBDA": "LAMBDA",
    "LAM": "LAMBDA",
    "GAMMA": "GAMMA",
    "PHI": "PHI",
    "W": "W",
    "L": "L",
    "IS": "IS",
    "NSUB": "N_SUB",
    "N_SUB": "N_SUB",
    "TNOM": "T_NOM",
    "T_NOM": "T_NOM",
    "CGSO": "CGSO",
    "CGDO": "CGDO",
    "CGBO": "CGBO",
    "CBS": "CBS",
    "CJS": "CBS",
    "CBD": "CBD",
    "CJD": "CBD",
}


def _type_key(text: str) -> str:
    return text.strip().upper().replace("-", "").replace("_", "")


def _parameter_key(text: str) -> str:
    return text.strip().upper().replace("-", "_")


def normalize_model_card_type(model_type: str) -> str:
    """Return the supported canonical model kind for a `.model` card."""

    key = _type_key(model_type)
    try:
        return _MODEL_TYPE_ALIASES[key]
    except KeyError as exc:
        raise ValueError(f"unsupported SPICE model type {model_type!r}") from exc


def _parameter_aliases(kind: str) -> dict[str, str]:
    if kind == "D":
        return _DIODE_PARAMETER_ALIASES
    if kind in {"NPN", "PNP"}:
        return _BJT_PARAMETER_ALIASES
    if kind in {"NJF", "PJF"}:
        return _JFET_PARAMETER_ALIASES
    if kind in {"NMOS", "PMOS"}:
        return _MOS_LEVEL1_PARAMETER_ALIASES
    raise ValueError(f"unsupported SPICE model kind {kind!r}")


def normalize_model_card(
    name: str,
    model_type: str,
    parameters: Mapping[str, float] | None = None,
) -> NormalizedModelCard:
    """Normalize a SPICE `.model` card to stable kind and parameter keys.

    Unknown parameter names are retained in ``unsupported_parameters`` so deck
    parsing can produce explicit diagnostics without losing the supported
    portion of a model card.  MOS cards accept only ``LEVEL=1`` in this slice.
    """

    kind = normalize_model_card_type(model_type)
    aliases = _parameter_aliases(kind)
    normalized: dict[str, float] = {}
    unsupported: list[str] = []
    for raw_name, raw_value in (parameters or {}).items():
        key = _parameter_key(raw_name)
        canonical = aliases.get(key)
        if canonical is None:
            if key not in unsupported:
                unsupported.append(key)
            continue
        value = float(raw_value)
        if canonical == "LEVEL":
            if abs(value - 1.0) > 1.0e-12:
                raise ValueError(f"{name}: only MOS LEVEL=1 model cards are supported")
            normalized[canonical] = 1.0
        else:
            normalized[canonical] = value
    return NormalizedModelCard(
        name=name,
        kind=kind,
        parameters=normalized,
        unsupported_parameters=tuple(unsupported),
    )


def diode_from_model_card(
    name: str,
    anode: str,
    cathode: str,
    model: NormalizedModelCard,
) -> Diode:
    """Build a diode instance from a normalized diode model card."""

    if model.kind != "D":
        raise ValueError(f"{name}: expected diode model card, got {model.kind}")
    p = model.parameters
    return Diode(
        name,
        anode,
        cathode,
        Is=p.get("IS", 1.0e-15),
        Vt=p.get("VT", 0.02585),
        N=p.get("N", 1.0),
        BV=p.get("BV"),
        IBV=p.get("IBV", 1.0e-3),
        Cjo=p.get("CJO", 0.0),
        Tt=p.get("TT", 0.0),
    )


def bjt_from_model_card(
    name: str,
    collector: str,
    base: str,
    emitter: str,
    model: NormalizedModelCard,
) -> BJT:
    """Build a BJT instance from a normalized NPN or PNP model card."""

    if model.kind not in {"NPN", "PNP"}:
        raise ValueError(f"{name}: expected BJT model card, got {model.kind}")
    p = model.parameters
    return BJT(
        name,
        collector,
        base,
        emitter,
        polarity=model.kind,
        Is=p.get("IS", 1.0e-14),
        beta_f=p.get("BF", 100.0),
        Vt=p.get("VT", 0.02585),
        Cje=p.get("CJE", 0.0),
        Cjc=p.get("CJC", 0.0),
        Tf=p.get("TF", 0.0),
        Tr=p.get("TR", 0.0),
    )


def jfet_from_model_card(
    name: str,
    drain: str,
    gate: str,
    source: str,
    model: NormalizedModelCard,
) -> JFET:
    """Build a JFET instance from a normalized NJF or PJF model card."""

    if model.kind not in {"NJF", "PJF"}:
        raise ValueError(f"{name}: expected JFET model card, got {model.kind}")
    p = model.parameters
    return JFET(
        name,
        drain,
        gate,
        source,
        polarity=model.kind,
        beta=p.get("BETA", 1.0e-4),
        vto=p.get("VTO", -2.0 if model.kind == "NJF" else 2.0),
        lambda_=p.get("LAMBDA", 0.0),
    )


def mosfet_from_model_card(
    name: str,
    drain: str,
    gate: str,
    source: str,
    body: str,
    model: NormalizedModelCard,
) -> Mosfet:
    """Build a Level-1 MOSFET instance from a normalized NMOS or PMOS card."""

    if model.kind not in {"NMOS", "PMOS"}:
        raise ValueError(f"{name}: expected MOSFET model card, got {model.kind}")
    p = model.parameters
    defaults = Level1Params()
    params = replace(
        defaults,
        VT0=p.get("VT0", defaults.VT0),
        KP=p.get("KP", defaults.KP),
        LAMBDA=p.get("LAMBDA", defaults.LAMBDA),
        GAMMA=p.get("GAMMA", defaults.GAMMA),
        PHI=p.get("PHI", defaults.PHI),
        W=p.get("W", defaults.W),
        L=p.get("L", defaults.L),
        IS=p.get("IS", defaults.IS),
        N_SUB=p.get("N_SUB", defaults.N_SUB),
        T_NOM=p.get("T_NOM", defaults.T_NOM),
        CGSO=p.get("CGSO", defaults.CGSO),
        CGDO=p.get("CGDO", defaults.CGDO),
        CGBO=p.get("CGBO", defaults.CGBO),
        CBS=p.get("CBS", defaults.CBS),
        CBD=p.get("CBD", defaults.CBD),
    )
    mos_type = MosfetType.NMOS if model.kind == "NMOS" else MosfetType.PMOS
    return Mosfet(name, drain, gate, source, body, MOSFET(mos_type, Level1Model(params)))


def device_model_audit_fixtures() -> tuple[NormalizedModelCard, ...]:
    """Return canonical audit fixtures shared by Python, Rust, and TypeScript."""

    return (
        normalize_model_card(
            "Dfast",
            "diode",
            {"JS": 2.0e-14, "CJ": 1.5e-12, "TT": 4.0e-9},
        ),
        normalize_model_card(
            "Qsmall",
            "npn",
            {"BETA": 125.0, "CBE": 2.0e-12, "TF": 1.0e-10},
        ),
        normalize_model_card("Jn", "njfet", {"BET": 9.0e-4, "VT0": -1.8, "LAM": 0.02}),
        normalize_model_card(
            "Mn",
            "nmos",
            {"LEVEL": 1.0, "VTO": 0.55, "LAM": 0.04, "NSUB": 1.6, "CJD": 3.0e-13},
        ),
    )
