"""SPICE model-card normalization helpers.

The engine still uses programmatic circuit construction.  These helpers provide
the shared `.model` alias surface that the deck parser can target without
duplicating diode, BJT, JFET, and Level-1 MOS parameter mapping logic.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass, replace

from mosfet_models import MOSFET, Level1Model, Level1Params, MosfetType

from spice_engine.elements import (
    BJT,
    JFET,
    AcSource,
    Capacitor,
    Diode,
    Mosfet,
    Resistor,
    VoltageSource,
)
from spice_engine.engine import (
    Circuit,
    deck_table_records,
    format_deck_table_csv,
    format_deck_table_json,
)


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


@dataclass(frozen=True, slots=True)
class DeviceModelTemperaturePoint:
    """Expected probe-voltage window for one model-fixture temperature."""

    temperature_kelvin: float
    expected_min: float
    expected_max: float


@dataclass(frozen=True, slots=True)
class DeviceModelTemperatureBehaviorFixture:
    """A runnable model-card temperature fixture with stable sweep windows."""

    name: str
    kind: str
    model: NormalizedModelCard
    circuit: Circuit
    probe_node: str
    nominal_temperature_kelvin: float
    energy_gap_ev: float
    temperature_behavior: str
    temperature_points: tuple[DeviceModelTemperaturePoint, ...]
    deck_lines: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class DeviceModelCapacitanceBehaviorFixture:
    """A runnable model-card AC fixture with a stable capacitance probe window."""

    name: str
    kind: str
    model: NormalizedModelCard
    circuit: Circuit
    probe_node: str
    frequency_hz: float
    expected_magnitude_min: float
    expected_magnitude_max: float
    capacitance_behavior: str
    deck_lines: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class DeviceModelNoiseBehaviorFixture:
    """A runnable model-card noise fixture with a stable source PSD window."""

    name: str
    kind: str
    model: NormalizedModelCard
    circuit: Circuit
    output_node: str
    input_source: str
    frequency_hz: float
    expected_noise_element: str
    expected_noise_type: str
    expected_source_psd_min: float
    expected_source_psd_max: float
    expected_output_psd_min: float
    expected_output_psd_max: float
    noise_behavior: str
    deck_lines: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class DeviceModelChargeBehaviorFixture:
    """A runnable model-card transient storage fixture with stable probe windows."""

    name: str
    kind: str
    model: NormalizedModelCard
    circuit: Circuit
    probe_node: str
    time_step_s: float
    stop_time_s: float
    storage_capacitance_f: float
    expected_initial_min: float
    expected_initial_max: float
    expected_final_min: float
    expected_final_max: float
    charge_behavior: str
    deck_lines: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class DeviceModelReferenceDeckAuditFixture:
    """A reference-deck audit row for one model family and analysis kind."""

    name: str
    kind: str
    model: NormalizedModelCard
    analysis: str
    reference: str
    expected_behavior: str
    deck_lines: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class DeviceModelReferenceDeckAuditIssue:
    """A release-gate issue for the device-model reference-deck audit matrix."""

    fixture_name: str
    field: str
    message: str


@dataclass(frozen=True, slots=True)
class DeviceModelReferenceDeckAuditGateReport:
    """Release-gate summary for device-model reference-deck audit coverage."""

    passed: bool
    fixture_count: int
    expected_kinds: tuple[str, ...]
    expected_analyses: tuple[str, ...]
    issues: tuple[DeviceModelReferenceDeckAuditIssue, ...]


@dataclass(frozen=True, slots=True)
class DeviceModelReferenceDeckAuditSummary:
    """A compact per-model-family summary of reference-deck audit coverage."""

    kind: str
    fixture_count: int
    analyses: tuple[str, ...]
    missing_analyses: tuple[str, ...]
    deck_line_count: int
    references: tuple[str, ...]


_REFERENCE_DECK_AUDIT_EXPECTED_KINDS = ("D", "NPN", "NJF", "NMOS")
_REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES = (
    "op",
    "temperature",
    "ac",
    "noise",
    "tran",
)


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
    "CGS": "CGS",
    "CGS0": "CGS",
    "CGD": "CGD",
    "CGD0": "CGD",
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
    "PB": "PB",
    "MJ": "MJ",
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
        Cgs=p.get("CGS", 0.0),
        Cgd=p.get("CGD", 0.0),
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
        PB=p.get("PB", defaults.PB),
        MJ=p.get("MJ", defaults.MJ),
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


def _temperature_deck_lines(fixture: DeviceModelBehaviorFixture) -> tuple[str, ...]:
    lines = [*fixture.deck_lines]
    lines[0] = f"* device-model temperature fixture: {fixture.name}"
    op_index = lines.index(".op")
    lines.insert(op_index, ".temp 260.15 300.15 340.15")
    return tuple(lines)


def _temperature_points_for_fixture(name: str) -> tuple[DeviceModelTemperaturePoint, ...]:
    windows = {
        "diode-forward-bias": (
            (260.15, 0.63, 0.70),
            (300.15, 0.55, 0.65),
            (340.15, 0.49, 0.56),
        ),
        "bjt-emitter-follower": (
            (260.15, 0.03, 0.09),
            (300.15, 0.08, 0.18),
            (340.15, 0.15, 0.22),
        ),
        "jfet-source-bias": (
            (260.15, 0.86, 0.90),
            (300.15, 0.86, 0.90),
            (340.15, 0.86, 0.90),
        ),
        "mos-level1-common-source": (
            (260.15, 0.58, 0.68),
            (300.15, 0.55, 0.85),
            (340.15, 0.70, 0.82),
        ),
    }[name]
    return tuple(
        DeviceModelTemperaturePoint(
            temperature_kelvin=temperature_kelvin,
            expected_min=expected_min,
            expected_max=expected_max,
        )
        for temperature_kelvin, expected_min, expected_max in windows
    )


def device_model_temperature_audit_fixtures() -> tuple[DeviceModelTemperatureBehaviorFixture, ...]:
    """Return runnable model-card temperature sweep fixtures for model-depth audits."""

    behavior_by_name = {
        "diode-forward-bias": "diode saturation current and thermal voltage scale with temperature",
        "bjt-emitter-follower": "BJT saturation current and thermal voltage scale with temperature",
        "jfet-source-bias": "JFET temperature scaling is intentionally invariant until a policy lands",
        "mos-level1-common-source": "Level-1 MOS threshold and transconductance scale with temperature",
    }
    return tuple(
        DeviceModelTemperatureBehaviorFixture(
            name=fixture.name,
            kind=fixture.kind,
            model=fixture.model,
            circuit=fixture.circuit,
            probe_node=fixture.probe_node,
            nominal_temperature_kelvin=300.15,
            energy_gap_ev=1.11,
            temperature_behavior=behavior_by_name[fixture.name],
            temperature_points=_temperature_points_for_fixture(fixture.name),
            deck_lines=_temperature_deck_lines(fixture),
        )
        for fixture in device_model_behavior_audit_fixtures()
    )


def device_model_capacitance_audit_fixtures() -> tuple[DeviceModelCapacitanceBehaviorFixture, ...]:
    """Return runnable model-card AC fixtures for capacitance model-depth audits."""

    models = _model_card_by_name()
    frequency_hz = 100_000.0

    diode_circuit = Circuit()
    diode_circuit.add(VoltageSource("Vdrive", "in", "0", 0.0, ac=AcSource(1.0)))
    diode_circuit.add(Resistor("Rin", "in", "out", 1_000_000.0))
    diode_circuit.add(diode_from_model_card("D1", "out", "0", models["Dfast"]))

    bjt_circuit = Circuit()
    bjt_circuit.add(VoltageSource("Vdrive", "in", "0", 0.0, ac=AcSource(1.0)))
    bjt_circuit.add(Resistor("Rin", "in", "base", 1_000_000.0))
    bjt_circuit.add(Resistor("Rc", "col", "0", 1_000.0))
    bjt_circuit.add(bjt_from_model_card("Q1", "col", "base", "0", models["Qsmall"]))

    jfet_model = normalize_model_card(
        "Jn",
        "NJF",
        {"BETA": 9.0e-4, "VTO": -1.8, "LAMBDA": 0.02, "CGS": 2.0e-9, "CGD": 1.0e-10},
    )
    jfet_circuit = Circuit()
    jfet_circuit.add(VoltageSource("Vdrive", "in", "0", 0.0, ac=AcSource(1.0)))
    jfet_circuit.add(Resistor("Rin", "in", "source", 1_000.0))
    jfet_circuit.add(Resistor("Rd", "drain", "0", 2_000.0))
    jfet_circuit.add(VoltageSource("Vgate", "gate", "0", 0.0))
    jfet_circuit.add(jfet_from_model_card("J1", "drain", "gate", "source", jfet_model))

    mos_circuit = Circuit()
    mos_circuit.add(VoltageSource("Vdrive", "in", "0", 0.0, ac=AcSource(1.0)))
    mos_circuit.add(Resistor("Rin", "in", "drain", 5_000_000.0))
    mos_circuit.add(VoltageSource("Vgate", "gate", "0", 0.0))
    mos_circuit.add(mosfet_from_model_card("M1", "drain", "gate", "0", "0", models["Mn"]))

    return (
        DeviceModelCapacitanceBehaviorFixture(
            name="diode-capacitance-ac",
            kind=models["Dfast"].kind,
            model=models["Dfast"],
            circuit=diode_circuit,
            probe_node="out",
            frequency_hz=frequency_hz,
            expected_magnitude_min=0.72,
            expected_magnitude_max=0.74,
            capacitance_behavior="diode CJO and TT contribute high-frequency shunt capacitance",
            deck_lines=(
                "* device-model capacitance fixture: diode-capacitance-ac",
                ".model Dfast D(IS=2e-14 CJO=1.5e-12 TT=4e-9)",
                "Vdrive in 0 0 AC 1",
                "Rin in out 1meg",
                "D1 out 0 Dfast",
                ".ac lin 1 100k 100k",
                ".save V(out)",
                ".end",
            ),
        ),
        DeviceModelCapacitanceBehaviorFixture(
            name="bjt-capacitance-ac",
            kind=models["Qsmall"].kind,
            model=models["Qsmall"],
            circuit=bjt_circuit,
            probe_node="base",
            frequency_hz=frequency_hz,
            expected_magnitude_min=0.61,
            expected_magnitude_max=0.64,
            capacitance_behavior="BJT CJE and TF contribute base-emitter AC capacitance",
            deck_lines=(
                "* device-model capacitance fixture: bjt-capacitance-ac",
                ".model Qsmall NPN(BF=125 CJE=2e-12 TF=1e-10)",
                "Vdrive in 0 0 AC 1",
                "Rin in base 1meg",
                "Rc col 0 1k",
                "Q1 col base 0 Qsmall",
                ".ac lin 1 100k 100k",
                ".save V(base)",
                ".end",
            ),
        ),
        DeviceModelCapacitanceBehaviorFixture(
            name="jfet-capacitance-ac",
            kind=jfet_model.kind,
            model=jfet_model,
            circuit=jfet_circuit,
            probe_node="source",
            frequency_hz=frequency_hz,
            expected_magnitude_min=0.50,
            expected_magnitude_max=0.54,
            capacitance_behavior="JFET CGS/CGD contribute high-frequency gate-channel capacitance",
            deck_lines=(
                "* device-model capacitance fixture: jfet-capacitance-ac",
                ".model Jn NJF(BETA=9e-4 VTO=-1.8 LAMBDA=0.02 CGS=2n CGD=100p)",
                "Vdrive in 0 0 AC 1",
                "Rin in source 1k",
                "Rd drain 0 2k",
                "Vgate gate 0 0",
                "J1 drain gate source Jn",
                ".ac lin 1 100k 100k",
                ".save V(source)",
                ".end",
            ),
        ),
        DeviceModelCapacitanceBehaviorFixture(
            name="mos-level1-capacitance-ac",
            kind=models["Mn"].kind,
            model=models["Mn"],
            circuit=mos_circuit,
            probe_node="drain",
            frequency_hz=frequency_hz,
            expected_magnitude_min=0.72,
            expected_magnitude_max=0.74,
            capacitance_behavior="Level-1 MOS CBD contributes drain-bulk AC capacitance",
            deck_lines=(
                "* device-model capacitance fixture: mos-level1-capacitance-ac",
                ".model Mn NMOS(LEVEL=1 VTO=0.55 LAMBDA=0.04 NSUB=1.6 CBD=3e-13)",
                "Vdrive in 0 0 AC 1",
                "Rin in drain 5meg",
                "Vgate gate 0 0",
                "M1 drain gate 0 0 Mn",
                ".ac lin 1 100k 100k",
                ".save V(drain)",
                ".end",
            ),
        ),
    )


def device_model_noise_audit_fixtures() -> tuple[DeviceModelNoiseBehaviorFixture, ...]:
    """Return runnable model-card .noise fixtures for model-depth audits."""

    models = _model_card_by_name()
    frequency_hz = 1_000.0

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
        DeviceModelNoiseBehaviorFixture(
            name="diode-shot-noise",
            kind=models["Dfast"].kind,
            model=models["Dfast"],
            circuit=diode_circuit,
            output_node="out",
            input_source="Vbias",
            frequency_hz=frequency_hz,
            expected_noise_element="D1",
            expected_noise_type="shot",
            expected_source_psd_min=6.4e-23,
            expected_source_psd_max=6.7e-23,
            expected_output_psd_min=8.0e-19,
            expected_output_psd_max=8.5e-19,
            noise_behavior="diode forward current contributes junction shot noise",
            deck_lines=(
                "* device-model noise fixture: diode-shot-noise",
                ".model Dfast D(IS=2e-14 CJO=1.5e-12 TT=4e-9)",
                "Vbias vin 0 0.8",
                "Rlimit vin out 1k",
                "D1 out 0 Dfast",
                ".noise V(out) Vbias lin 1 1k 1k",
                ".save V(out)",
                ".end",
            ),
        ),
        DeviceModelNoiseBehaviorFixture(
            name="bjt-shot-noise",
            kind=models["Qsmall"].kind,
            model=models["Qsmall"],
            circuit=bjt_circuit,
            output_node="out",
            input_source="Vbase",
            frequency_hz=frequency_hz,
            expected_noise_element="Q1",
            expected_noise_type="shot",
            expected_source_psd_min=3.7e-23,
            expected_source_psd_max=3.9e-23,
            expected_output_psd_min=1.1e-18,
            expected_output_psd_max=1.3e-18,
            noise_behavior="BJT forward-active collector current contributes shot noise",
            deck_lines=(
                "* device-model noise fixture: bjt-shot-noise",
                ".model Qsmall NPN(BF=125 CJE=2e-12 TF=1e-10)",
                "Vcc vcc 0 5",
                "Vbase base 0 0.72",
                "Q1 vcc base out Qsmall",
                "Rload out 0 1k",
                ".noise V(out) Vbase lin 1 1k 1k",
                ".save V(out)",
                ".end",
            ),
        ),
        DeviceModelNoiseBehaviorFixture(
            name="jfet-channel-noise",
            kind=models["Jn"].kind,
            model=models["Jn"],
            circuit=jfet_circuit,
            output_node="source",
            input_source="Vdd",
            frequency_hz=frequency_hz,
            expected_noise_element="J1",
            expected_noise_type="thermal",
            expected_source_psd_min=2.0e-23,
            expected_source_psd_max=2.2e-23,
            expected_output_psd_min=2.3e-18,
            expected_output_psd_max=2.5e-18,
            noise_behavior="JFET transconductance contributes long-channel channel thermal noise",
            deck_lines=(
                "* device-model noise fixture: jfet-channel-noise",
                ".model Jn NJF(BETA=9e-4 VTO=-1.8 LAMBDA=0.02)",
                "Vdd vdd 0 10",
                "Vg gate 0 0",
                "Rd vdd drain 2k",
                "Rs source 0 1k",
                "J1 drain gate source Jn",
                ".noise V(source) Vdd lin 1 1k 1k",
                ".save V(source)",
                ".end",
            ),
        ),
        DeviceModelNoiseBehaviorFixture(
            name="mos-level1-channel-noise",
            kind=models["Mn"].kind,
            model=models["Mn"],
            circuit=mos_circuit,
            output_node="out",
            input_source="Vgate",
            frequency_hz=frequency_hz,
            expected_noise_element="M1",
            expected_noise_type="thermal",
            expected_source_psd_min=1.3e-23,
            expected_source_psd_max=1.4e-23,
            expected_output_psd_min=3.3e-18,
            expected_output_psd_max=3.5e-18,
            noise_behavior="Level-1 MOS gm contributes long-channel channel thermal noise",
            deck_lines=(
                "* device-model noise fixture: mos-level1-channel-noise",
                ".model Mn NMOS(LEVEL=1 VTO=0.55 LAMBDA=0.04 NSUB=1.6 CBD=3e-13)",
                "Vdd vdd 0 1.8",
                "Vgate gate 0 1.8",
                "Rload vdd out 1k",
                "M1 out gate 0 0 Mn",
                ".noise V(out) Vgate lin 1 1k 1k",
                ".save V(out)",
                ".end",
            ),
        ),
    )


def device_model_charge_audit_fixtures() -> tuple[DeviceModelChargeBehaviorFixture, ...]:
    """Return runnable transient storage fixtures for charge model-depth audits.

    Diode CJO/TT, BJT CJE/CJC/TF/TR, JFET fixed gate-source/gate-drain, and
    Level-1 MOS fixed gate-overlap plus zero-bias bulk-junction storage are
    transient-stamped by the simulator.
    """

    models = _model_card_by_name()
    time_step_s = 2.0e-8
    stop_time_s = 2.0e-6
    storage_capacitance_f = 1.0e-10

    diode_circuit = Circuit()
    diode_circuit.add(VoltageSource("Vbias", "vin", "0", 0.8))
    diode_circuit.add(Resistor("Rlimit", "vin", "out", 1_000.0))
    diode_circuit.add(diode_from_model_card("D1", "out", "0", models["Dfast"]))
    diode_circuit.add(Capacitor("Cstore", "out", "0", storage_capacitance_f))

    bjt_circuit = Circuit()
    bjt_circuit.add(VoltageSource("Vcc", "vcc", "0", 5.0))
    bjt_circuit.add(VoltageSource("Vbase", "base", "0", 0.72))
    bjt_circuit.add(Resistor("Rload", "out", "0", 1_000.0))
    bjt_circuit.add(bjt_from_model_card("Q1", "vcc", "base", "out", models["Qsmall"]))
    bjt_circuit.add(Capacitor("Cstore", "out", "0", storage_capacitance_f))

    jfet_model = normalize_model_card(
        "Jn",
        "NJF",
        {"BETA": 9.0e-4, "VTO": -1.8, "LAMBDA": 0.02, "CGS": 2.0e-11, "CGD": 5.0e-12},
    )
    jfet_circuit = Circuit()
    jfet_circuit.add(VoltageSource("Vdd", "vdd", "0", 10.0))
    jfet_circuit.add(VoltageSource("Vg", "gate", "0", 0.0))
    jfet_circuit.add(Resistor("Rd", "vdd", "drain", 2_000.0))
    jfet_circuit.add(Resistor("Rs", "source", "0", 1_000.0))
    jfet_circuit.add(jfet_from_model_card("J1", "drain", "gate", "source", jfet_model))
    jfet_circuit.add(Capacitor("Cstore", "source", "0", storage_capacitance_f))

    mos_model = normalize_model_card(
        "Mn",
        "NMOS",
        {
            "LEVEL": 1.0,
            "VTO": 0.55,
            "LAMBDA": 0.04,
            "NSUB": 1.6,
            "CGSO": 2.0e-11,
            "CGDO": 5.0e-12,
            "CGBO": 1.0e-12,
            "CBS": 4.0e-13,
            "CBD": 3.0e-13,
            "PB": 0.9,
            "MJ": 0.45,
        },
    )
    mos_circuit = Circuit()
    mos_circuit.add(VoltageSource("Vdd", "vdd", "0", 1.8))
    mos_circuit.add(VoltageSource("Vgate", "gate", "0", 1.8))
    mos_circuit.add(Resistor("Rload", "vdd", "out", 1_000.0))
    mos_circuit.add(mosfet_from_model_card("M1", "out", "gate", "0", "0", mos_model))
    mos_circuit.add(Capacitor("Cstore", "out", "0", storage_capacitance_f))

    return (
        DeviceModelChargeBehaviorFixture(
            name="diode-storage-charge",
            kind=models["Dfast"].kind,
            model=models["Dfast"],
            circuit=diode_circuit,
            probe_node="out",
            time_step_s=time_step_s,
            stop_time_s=stop_time_s,
            storage_capacitance_f=storage_capacitance_f,
            expected_initial_min=-1.0e-9,
            expected_initial_max=1.0,
            expected_final_min=0.58,
            expected_final_max=0.61,
            charge_behavior=(
                "diode CJO/TT contribute transient anode-cathode storage; "
                "explicit Cstore keeps the fixture comparable with other charge audits"
            ),
            deck_lines=(
                "* device-model charge fixture: diode-storage-charge",
                ".model Dfast D(IS=2e-14 CJO=1.5e-12 TT=4e-9)",
                "Vbias vin 0 0.8",
                "Rlimit vin out 1k",
                "D1 out 0 Dfast",
                "Cstore out 0 100p",
                ".tran 20n 2u",
                ".save V(out)",
                ".end",
            ),
        ),
        DeviceModelChargeBehaviorFixture(
            name="bjt-storage-charge",
            kind=models["Qsmall"].kind,
            model=models["Qsmall"],
            circuit=bjt_circuit,
            probe_node="out",
            time_step_s=time_step_s,
            stop_time_s=stop_time_s,
            storage_capacitance_f=storage_capacitance_f,
            expected_initial_min=-1.0e-9,
            expected_initial_max=1.0,
            expected_final_min=0.10,
            expected_final_max=0.14,
            charge_behavior=(
                "BJT CJE/CJC/TF/TR contribute transient base-emitter and "
                "base-collector storage; explicit Cstore keeps the fixture "
                "comparable with other charge audits"
            ),
            deck_lines=(
                "* device-model charge fixture: bjt-storage-charge",
                ".model Qsmall NPN(BF=125 CJE=2e-12 TF=1e-10)",
                "Vcc vcc 0 5",
                "Vbase base 0 0.72",
                "Q1 vcc base out Qsmall",
                "Rload out 0 1k",
                "Cstore out 0 100p",
                ".tran 20n 2u",
                ".save V(out)",
                ".end",
            ),
        ),
        DeviceModelChargeBehaviorFixture(
            name="jfet-storage-charge",
            kind=jfet_model.kind,
            model=jfet_model,
            circuit=jfet_circuit,
            probe_node="source",
            time_step_s=time_step_s,
            stop_time_s=stop_time_s,
            storage_capacitance_f=storage_capacitance_f,
            expected_initial_min=-1.0e-9,
            expected_initial_max=1.0,
            expected_final_min=0.86,
            expected_final_max=0.90,
            charge_behavior=(
                "JFET CGS/CGD contribute transient gate-source and gate-drain storage; "
                "explicit Cstore keeps the fixture comparable with other charge audits"
            ),
            deck_lines=(
                "* device-model charge fixture: jfet-storage-charge",
                ".model Jn NJF(BETA=9e-4 VTO=-1.8 LAMBDA=0.02 CGS=20p CGD=5p)",
                "Vdd vdd 0 10",
                "Vg gate 0 0",
                "Rd vdd drain 2k",
                "Rs source 0 1k",
                "J1 drain gate source Jn",
                "Cstore source 0 100p",
                ".tran 20n 2u",
                ".save V(source)",
                ".end",
            ),
        ),
        DeviceModelChargeBehaviorFixture(
            name="mos-level1-storage-charge",
            kind=mos_model.kind,
            model=mos_model,
            circuit=mos_circuit,
            probe_node="out",
            time_step_s=time_step_s,
            stop_time_s=stop_time_s,
            storage_capacitance_f=storage_capacitance_f,
            expected_initial_min=-1.0e-9,
            expected_initial_max=1.0,
            expected_final_min=0.68,
            expected_final_max=0.73,
            charge_behavior=(
                "Level-1 MOS CGSO/CGDO/CGBO plus CBS/CBD contribute transient "
                "gate-overlap and depletion-shaped bulk-junction storage; explicit "
                "Cstore keeps the fixture comparable with other charge audits"
            ),
            deck_lines=(
                "* device-model charge fixture: mos-level1-storage-charge",
                ".model Mn NMOS(LEVEL=1 VTO=0.55 LAMBDA=0.04 NSUB=1.6 CGSO=20p CGDO=5p CGBO=1p CBS=4e-13 CBD=3e-13 PB=0.9 MJ=0.45)",
                "Vdd vdd 0 1.8",
                "Vgate gate 0 1.8",
                "Rload vdd out 1k",
                "M1 out gate 0 0 Mn",
                "Cstore out 0 100p",
                ".tran 20n 2u",
                ".save V(out)",
                ".end",
            ),
        ),
    )


def device_model_reference_deck_audit_fixtures() -> tuple[DeviceModelReferenceDeckAuditFixture, ...]:
    """Return flattened reference-deck coverage rows for device-model audits."""

    reference = "SPICE2/SPICE3-style local model-depth fixture"
    rows: list[DeviceModelReferenceDeckAuditFixture] = []
    for fixture in device_model_behavior_audit_fixtures():
        rows.append(
            DeviceModelReferenceDeckAuditFixture(
                name=f"{fixture.name}:op",
                kind=fixture.kind,
                model=fixture.model,
                analysis="op",
                reference=reference,
                expected_behavior=(
                    f"DC probe {fixture.probe_node} remains in "
                    f"[{fixture.expected_min:g}, {fixture.expected_max:g}] V"
                ),
                deck_lines=fixture.deck_lines,
            )
        )
    for fixture in device_model_temperature_audit_fixtures():
        rows.append(
            DeviceModelReferenceDeckAuditFixture(
                name=f"{fixture.name}:temperature",
                kind=fixture.kind,
                model=fixture.model,
                analysis="temperature",
                reference=reference,
                expected_behavior=fixture.temperature_behavior,
                deck_lines=fixture.deck_lines,
            )
        )
    for fixture in device_model_capacitance_audit_fixtures():
        rows.append(
            DeviceModelReferenceDeckAuditFixture(
                name=f"{fixture.name}:ac",
                kind=fixture.kind,
                model=fixture.model,
                analysis="ac",
                reference=reference,
                expected_behavior=fixture.capacitance_behavior,
                deck_lines=fixture.deck_lines,
            )
        )
    for fixture in device_model_noise_audit_fixtures():
        rows.append(
            DeviceModelReferenceDeckAuditFixture(
                name=f"{fixture.name}:noise",
                kind=fixture.kind,
                model=fixture.model,
                analysis="noise",
                reference=reference,
                expected_behavior=fixture.noise_behavior,
                deck_lines=fixture.deck_lines,
            )
        )
    for fixture in device_model_charge_audit_fixtures():
        rows.append(
            DeviceModelReferenceDeckAuditFixture(
                name=f"{fixture.name}:tran",
                kind=fixture.kind,
                model=fixture.model,
                analysis="tran",
                reference=reference,
                expected_behavior=fixture.charge_behavior,
                deck_lines=fixture.deck_lines,
            )
        )
    return tuple(rows)


def format_device_model_reference_deck_audit_table(
    fixtures: Sequence[DeviceModelReferenceDeckAuditFixture] | None = None,
) -> str:
    """Return a stable tab-separated summary of reference-deck audit coverage."""

    rows = (
        device_model_reference_deck_audit_fixtures()
        if fixtures is None
        else tuple(fixtures)
    )
    lines = [
        "name\tkind\tanalysis\tmodel\treference\texpected_behavior\tdeck_lines",
    ]
    for fixture in rows:
        lines.append(
            "\t".join(
                [
                    fixture.name,
                    fixture.kind,
                    fixture.analysis,
                    fixture.model.name,
                    fixture.reference,
                    fixture.expected_behavior,
                    str(len(fixture.deck_lines)),
                ]
            )
        )
    return "\n".join(lines)


def device_model_reference_deck_audit_records(
    fixtures: Sequence[DeviceModelReferenceDeckAuditFixture] | None = None,
) -> list[dict[str, str]]:
    """Return header-keyed records for the reference-deck audit matrix."""

    return deck_table_records(format_device_model_reference_deck_audit_table(fixtures))


def format_device_model_reference_deck_audit_csv(
    fixtures: Sequence[DeviceModelReferenceDeckAuditFixture] | None = None,
) -> str:
    """Return the reference-deck audit matrix as RFC 4180-style CSV."""

    return format_deck_table_csv(format_device_model_reference_deck_audit_table(fixtures))


def format_device_model_reference_deck_audit_json(
    fixtures: Sequence[DeviceModelReferenceDeckAuditFixture] | None = None,
) -> str:
    """Return compact JSON records for the reference-deck audit matrix."""

    return format_deck_table_json(format_device_model_reference_deck_audit_table(fixtures))


def device_model_reference_deck_audit_summary(
    fixtures: Sequence[DeviceModelReferenceDeckAuditFixture] | None = None,
) -> tuple[DeviceModelReferenceDeckAuditSummary, ...]:
    """Return per-model-family coverage summaries for the reference-deck audit."""

    rows = (
        device_model_reference_deck_audit_fixtures()
        if fixtures is None
        else tuple(fixtures)
    )
    expected_kinds = _REFERENCE_DECK_AUDIT_EXPECTED_KINDS
    expected_analyses = _REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES
    extra_kinds = tuple(
        sorted({fixture.kind for fixture in rows if fixture.kind not in expected_kinds})
    )
    summaries: list[DeviceModelReferenceDeckAuditSummary] = []

    for kind in (*expected_kinds, *extra_kinds):
        kind_rows = tuple(fixture for fixture in rows if fixture.kind == kind)
        row_analyses = {fixture.analysis for fixture in kind_rows}
        analyses = tuple(
            analysis for analysis in expected_analyses if analysis in row_analyses
        ) + tuple(
            sorted(analysis for analysis in row_analyses if analysis not in expected_analyses)
        )
        missing_analyses = (
            tuple(analysis for analysis in expected_analyses if analysis not in row_analyses)
            if kind in expected_kinds
            else ()
        )
        references: list[str] = []
        for fixture in kind_rows:
            if fixture.reference and fixture.reference not in references:
                references.append(fixture.reference)
        summaries.append(
            DeviceModelReferenceDeckAuditSummary(
                kind=kind,
                fixture_count=len(kind_rows),
                analyses=analyses,
                missing_analyses=missing_analyses,
                deck_line_count=sum(len(fixture.deck_lines) for fixture in kind_rows),
                references=tuple(references),
            )
        )
    return tuple(summaries)


def format_device_model_reference_deck_audit_summary_table(
    fixtures: Sequence[DeviceModelReferenceDeckAuditFixture] | None = None,
) -> str:
    """Return a stable tab-separated reference-deck audit coverage summary."""

    lines = [
        "kind\tfixture_count\tanalyses\tmissing_analyses\tdeck_lines\treferences",
    ]
    for summary in device_model_reference_deck_audit_summary(fixtures):
        lines.append(
            "\t".join(
                [
                    summary.kind,
                    str(summary.fixture_count),
                    ",".join(summary.analyses),
                    ",".join(summary.missing_analyses),
                    str(summary.deck_line_count),
                    ",".join(summary.references),
                ]
            )
        )
    return "\n".join(lines)


def device_model_reference_deck_audit_summary_records(
    fixtures: Sequence[DeviceModelReferenceDeckAuditFixture] | None = None,
) -> list[dict[str, str]]:
    """Return header-keyed records for the reference-deck audit summary."""

    return deck_table_records(format_device_model_reference_deck_audit_summary_table(fixtures))


def format_device_model_reference_deck_audit_summary_csv(
    fixtures: Sequence[DeviceModelReferenceDeckAuditFixture] | None = None,
) -> str:
    """Return the reference-deck audit summary as RFC 4180-style CSV."""

    return format_deck_table_csv(format_device_model_reference_deck_audit_summary_table(fixtures))


def format_device_model_reference_deck_audit_summary_json(
    fixtures: Sequence[DeviceModelReferenceDeckAuditFixture] | None = None,
) -> str:
    """Return compact JSON records for the reference-deck audit summary."""

    return format_deck_table_json(format_device_model_reference_deck_audit_summary_table(fixtures))


def device_model_reference_deck_audit_gate(
    fixtures: Sequence[DeviceModelReferenceDeckAuditFixture] | None = None,
) -> DeviceModelReferenceDeckAuditGateReport:
    """Validate that reference-deck audit rows cover the release matrix."""

    rows = (
        device_model_reference_deck_audit_fixtures()
        if fixtures is None
        else tuple(fixtures)
    )
    issues: list[DeviceModelReferenceDeckAuditIssue] = []
    seen_names: set[str] = set()
    seen_pairs: set[tuple[str, str]] = set()
    expected_kinds = _REFERENCE_DECK_AUDIT_EXPECTED_KINDS
    expected_analyses = _REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES

    if not rows:
        issues.append(
            DeviceModelReferenceDeckAuditIssue(
                "audit_matrix",
                "fixture_count",
                "audit matrix must contain at least one reference-deck row",
            )
        )

    for fixture in rows:
        fixture_name = fixture.name or "<missing>"
        if fixture.name in seen_names:
            issues.append(
                DeviceModelReferenceDeckAuditIssue(
                    fixture_name,
                    "name",
                    "reference-deck audit fixture names must be unique",
                )
            )
        seen_names.add(fixture.name)
        if not fixture.name.strip():
            issues.append(
                DeviceModelReferenceDeckAuditIssue(
                    fixture_name,
                    "name",
                    "field must be documented and non-empty",
                )
            )
        if fixture.kind not in expected_kinds:
            issues.append(
                DeviceModelReferenceDeckAuditIssue(
                    fixture_name,
                    "kind",
                    f"unsupported reference-deck audit kind {fixture.kind!r}",
                )
            )
        if fixture.analysis not in expected_analyses:
            issues.append(
                DeviceModelReferenceDeckAuditIssue(
                    fixture_name,
                    "analysis",
                    f"unsupported reference-deck audit analysis {fixture.analysis!r}",
                )
            )
        seen_pairs.add((fixture.kind, fixture.analysis))
        if not fixture.model.name.strip():
            issues.append(
                DeviceModelReferenceDeckAuditIssue(
                    fixture_name,
                    "model.name",
                    "field must be documented and non-empty",
                )
            )
        if not fixture.reference.strip():
            issues.append(
                DeviceModelReferenceDeckAuditIssue(
                    fixture_name,
                    "reference",
                    "field must be documented and non-empty",
                )
            )
        if not fixture.expected_behavior.strip():
            issues.append(
                DeviceModelReferenceDeckAuditIssue(
                    fixture_name,
                    "expected_behavior",
                    "field must be documented and non-empty",
                )
            )
        if not fixture.deck_lines:
            issues.append(
                DeviceModelReferenceDeckAuditIssue(
                    fixture_name,
                    "deck_lines",
                    "reference deck must contain active deck lines",
                )
            )
        else:
            if not fixture.deck_lines[0].startswith("* device-model "):
                issues.append(
                    DeviceModelReferenceDeckAuditIssue(
                        fixture_name,
                        "deck_lines[0]",
                        "reference deck must start with a device-model comment",
                    )
                )
            if not any(line.startswith(".model ") for line in fixture.deck_lines):
                issues.append(
                    DeviceModelReferenceDeckAuditIssue(
                        fixture_name,
                        "deck_lines",
                        "reference deck must include a .model card",
                    )
                )
            if fixture.deck_lines[-1] != ".end":
                issues.append(
                    DeviceModelReferenceDeckAuditIssue(
                        fixture_name,
                        "deck_lines[-1]",
                        "reference deck must end with .end",
                    )
                )

    for kind in expected_kinds:
        for analysis in expected_analyses:
            if (kind, analysis) not in seen_pairs:
                issues.append(
                    DeviceModelReferenceDeckAuditIssue(
                        f"{kind}:{analysis}",
                        "coverage",
                        f"missing required {kind} {analysis} reference-deck audit row",
                    )
                )

    return DeviceModelReferenceDeckAuditGateReport(
        passed=not issues,
        fixture_count=len(rows),
        expected_kinds=expected_kinds,
        expected_analyses=expected_analyses,
        issues=tuple(issues),
    )


def format_device_model_reference_deck_audit_gate_report(
    report: DeviceModelReferenceDeckAuditGateReport,
) -> str:
    """Return a stable tab-separated device-model audit gate report."""

    lines = [
        "passed\tfixture_count\texpected_kinds\texpected_analyses\tissue_count",
        (
            f"{str(report.passed).lower()}\t{report.fixture_count}\t"
            f"{','.join(report.expected_kinds)}\t"
            f"{','.join(report.expected_analyses)}\t{len(report.issues)}"
        ),
    ]
    if report.issues:
        lines.append("fixture_name\tfield\tmessage")
        lines.extend(
            f"{issue.fixture_name}\t{issue.field}\t{issue.message}"
            for issue in report.issues
        )
    return "\n".join(lines)
