"""SPICE model-card normalization helpers.

The engine still uses programmatic circuit construction.  These helpers provide
the shared `.model` alias surface that the deck parser can target without
duplicating diode, BJT, JFET, and Level-1 MOS parameter mapping logic.
"""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, replace

from mosfet_models import MOSFET, Level1Model, Level1Params, MosfetType

from spice_engine.elements import BJT, JFET, Diode, Mosfet, Resistor, VoltageSource
from spice_engine.engine import Circuit


@dataclass(frozen=True, slots=True)
class NormalizedModelCard:
    """A normalized SPICE `.model` card with stable cross-language keys."""

    name: str
    kind: str
    parameters: dict[str, float]
    unsupported_parameters: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class DeviceModelBehaviorFixture:
    """A runnable device-model reference fixture with a stable DC probe window."""

    name: str
    kind: str
    model: NormalizedModelCard
    circuit: Circuit
    probe_node: str
    expected_min: float
    expected_max: float
    deck_lines: tuple[str, ...]


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


def _model_card_by_name() -> dict[str, NormalizedModelCard]:
    return {model.name: model for model in device_model_audit_fixtures()}


def device_model_behavior_audit_fixtures() -> tuple[DeviceModelBehaviorFixture, ...]:
    """Return runnable one-device bias fixtures for model-depth audits.

    The circuits are intentionally small and DC-only.  They make the current
    diode, BJT, JFET, and Level-1 MOS behavior executable while carrying deck
    lines that future parser-backed reference-deck tests can consume.
    """

    models = _model_card_by_name()

    diode_circuit = Circuit()
    diode_circuit.add(VoltageSource("Vbias", "vin", "0", 0.8))
    diode_circuit.add(Resistor("Rlimit", "vin", "out", 1_000.0))
    diode_circuit.add(diode_from_model_card("D1", "out", "0", models["Dfast"]))

    bjt_circuit = Circuit()
    bjt_circuit.add(VoltageSource("Vcc", "vcc", "0", 5.0))
    bjt_circuit.add(VoltageSource("Vbase", "base", "0", 0.72))
    bjt_circuit.add(Resistor("Rload", "out", "0", 1_000.0))
    bjt_circuit.add(bjt_from_model_card("Q1", "vcc", "base", "out", models["Qsmall"]))

    jfet_circuit = Circuit()
    jfet_circuit.add(VoltageSource("Vdd", "vdd", "0", 10.0))
    jfet_circuit.add(VoltageSource("Vg", "gate", "0", 0.0))
    jfet_circuit.add(Resistor("Rd", "vdd", "drain", 2_000.0))
    jfet_circuit.add(Resistor("Rs", "source", "0", 1_000.0))
    jfet_circuit.add(jfet_from_model_card("J1", "drain", "gate", "source", models["Jn"]))

    mos_circuit = Circuit()
    mos_circuit.add(VoltageSource("Vdd", "vdd", "0", 1.8))
    mos_circuit.add(VoltageSource("Vgate", "gate", "0", 1.8))
    mos_circuit.add(Resistor("Rload", "vdd", "out", 1_000.0))
    mos_circuit.add(mosfet_from_model_card("M1", "out", "gate", "0", "0", models["Mn"]))

    return (
        DeviceModelBehaviorFixture(
            name="diode-forward-bias",
            kind=models["Dfast"].kind,
            model=models["Dfast"],
            circuit=diode_circuit,
            probe_node="out",
            expected_min=0.55,
            expected_max=0.65,
            deck_lines=(
                "* device-model behavior fixture: diode-forward-bias",
                ".model Dfast D(IS=2e-14 CJO=1.5e-12 TT=4e-9)",
                "Vbias vin 0 0.8",
                "Rlimit vin out 1k",
                "D1 out 0 Dfast",
                ".op",
                ".save V(out)",
                ".end",
            ),
        ),
        DeviceModelBehaviorFixture(
            name="bjt-emitter-follower",
            kind=models["Qsmall"].kind,
            model=models["Qsmall"],
            circuit=bjt_circuit,
            probe_node="out",
            expected_min=0.08,
            expected_max=0.18,
            deck_lines=(
                "* device-model behavior fixture: bjt-emitter-follower",
                ".model Qsmall NPN(BF=125 CJE=2e-12 TF=1e-10)",
                "Vcc vcc 0 5",
                "Vbase base 0 0.72",
                "Q1 vcc base out Qsmall",
                "Rload out 0 1k",
                ".op",
                ".save V(out)",
                ".end",
            ),
        ),
        DeviceModelBehaviorFixture(
            name="jfet-source-bias",
            kind=models["Jn"].kind,
            model=models["Jn"],
            circuit=jfet_circuit,
            probe_node="source",
            expected_min=0.80,
            expected_max=0.95,
            deck_lines=(
                "* device-model behavior fixture: jfet-source-bias",
                ".model Jn NJF(BETA=9e-4 VTO=-1.8 LAMBDA=0.02)",
                "Vdd vdd 0 10",
                "Vg gate 0 0",
                "Rd vdd drain 2k",
                "Rs source 0 1k",
                "J1 drain gate source Jn",
                ".op",
                ".save V(source)",
                ".end",
            ),
        ),
        DeviceModelBehaviorFixture(
            name="mos-level1-common-source",
            kind=models["Mn"].kind,
            model=models["Mn"],
            circuit=mos_circuit,
            probe_node="out",
            expected_min=0.55,
            expected_max=0.85,
            deck_lines=(
                "* device-model behavior fixture: mos-level1-common-source",
                ".model Mn NMOS(LEVEL=1 VTO=0.55 LAMBDA=0.04 NSUB=1.6 CBD=3e-13)",
                "Vdd vdd 0 1.8",
                "Vgate gate 0 1.8",
                "Rload vdd out 1k",
                "M1 out gate 0 0 Mn",
                ".op",
                ".save V(out)",
                ".end",
            ),
        ),
    )
