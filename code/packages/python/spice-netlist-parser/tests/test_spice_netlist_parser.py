from math import isclose

import pytest
from spice_engine import VCCS, Capacitor, CurrentSource, Inductor, Resistor, VoltageSource, dc_op

from spice_netlist_parser import (
    AcAnalysis,
    DcAnalysis,
    NetlistParseError,
    OpAnalysis,
    TranAnalysis,
    parse_netlist,
)
from spice_netlist_parser.parser import parse_value


def test_parse_linear_operating_point_netlist_into_circuit() -> None:
    parsed = parse_netlist(
        """
* resistor divider
V1 vin 0 DC 10
R1 vin mid 1k
R2 mid 0 1k
.op
.end
"""
    )

    assert parsed.title == "resistor divider"
    assert [type(element) for element in parsed.circuit.elements] == [
        VoltageSource,
        Resistor,
        Resistor,
    ]
    assert isinstance(parsed.analyses[0], OpAnalysis)

    result = dc_op(parsed.circuit)
    assert result.converged
    assert isclose(result.node_voltages["mid"], 5.0, abs_tol=1e-9)


def test_parse_reactive_elements_and_analysis_cards() -> None:
    parsed = parse_netlist(
        """
Vstep in 0 PULSE(0 1 0 1n 1n 10n 20n)
I1 out 0 1m
Rload in out 2.2k
Cload out 0 10p
L1 out 0 1u
G1 out 0 in 0 2m
.tran 1n 20n
.dc Vstep 0 1 0.5
.ac dec 10 1k 1meg
"""
    )

    assert isinstance(parsed.circuit.elements[0], VoltageSource)
    assert parsed.circuit.elements[0].waveform is not None
    assert isinstance(parsed.circuit.elements[1], CurrentSource)
    assert isinstance(parsed.circuit.elements[3], Capacitor)
    assert isinstance(parsed.circuit.elements[4], Inductor)
    assert isinstance(parsed.circuit.elements[5], VCCS)
    assert parsed.analyses == [
        TranAnalysis(t_step=1.0e-9, t_stop=20.0e-9),
        DcAnalysis(source_name="Vstep", start=0.0, stop=1.0, step=0.5),
        AcAnalysis(mode="dec", points=10, start_hz=1.0e3, stop_hz=1.0e6),
    ]


def test_parse_pwl_and_sin_source_waveforms() -> None:
    parsed = parse_netlist(
        """
V1 in 0 PWL(0 0, 1n 1.8, 2n 0)
I1 in 0 SIN(0 2m 1k 10u 5)
"""
    )

    voltage = parsed.circuit.elements[0]
    current = parsed.circuit.elements[1]
    assert isinstance(voltage, VoltageSource)
    assert isinstance(current, CurrentSource)
    assert voltage.waveform is not None
    assert current.waveform is not None
    assert isclose(voltage.waveform(0.5e-9), 0.9, abs_tol=1e-12)
    assert isclose(current.waveform(1.0e-6), 0.0, abs_tol=1e-12)


def test_expands_subcircuit_instances_into_engine_elements() -> None:
    parsed = parse_netlist(
        """
.subckt divider top mid bot
Rtop top mid 1k
Rbot mid bot 1k
.ends divider
V1 vin 0 DC 10
Xdiv vin mid 0 divider
.op
"""
    )

    assert [element.name for element in parsed.circuit.elements] == [
        "V1",
        "Xdiv.Rtop",
        "Xdiv.Rbot",
    ]
    assert isinstance(parsed.circuit.elements[1], Resistor)
    assert parsed.circuit.elements[1].n_plus == "vin"
    assert parsed.circuit.elements[1].n_minus == "mid"

    result = dc_op(parsed.circuit)
    assert result.converged
    assert isclose(result.node_voltages["mid"], 5.0, abs_tol=1e-9)


def test_subcircuit_internal_nodes_are_instance_scoped() -> None:
    parsed = parse_netlist(
        """
.subckt load in out
R1 in inner 1k
C1 inner out 1u
.ends load
Xleft a b load
Xright c d load
"""
    )

    left_resistor = parsed.circuit.elements[0]
    right_resistor = parsed.circuit.elements[2]
    assert isinstance(left_resistor, Resistor)
    assert isinstance(right_resistor, Resistor)
    assert left_resistor.n_minus == "Xleft.inner"
    assert right_resistor.n_minus == "Xright.inner"


def test_engineering_suffixes() -> None:
    assert parse_value("1k") == 1.0e3
    assert parse_value("2.2meg") == 2.2e6
    assert parse_value("3u") == 3.0e-6
    assert parse_value("4n") == 4.0e-9


def test_rejects_unsupported_element_with_line_number() -> None:
    with pytest.raises(NetlistParseError, match="line 2: unsupported element 'D1'"):
        parse_netlist(
            """
D1 a 0 diode
"""
        )


def test_rejects_unbalanced_waveform_parenthesis() -> None:
    with pytest.raises(NetlistParseError, match="unclosed parenthesis"):
        parse_netlist("V1 in 0 PULSE(0 1\n")


def test_rejects_unknown_subcircuit_instance() -> None:
    with pytest.raises(NetlistParseError, match="line 1: unknown subcircuit 'missing'"):
        parse_netlist("X1 a b missing\n")
