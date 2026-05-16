from math import isclose

import pytest
from mosfet_models import Level1Model, MosfetType
from spice_engine import (
    AcSource,
    BJT,
    CCCS,
    CCVS,
    VCCS,
    VCVS,
    Capacitor,
    CurrentSource,
    Diode,
    Inductor,
    Mosfet,
    Resistor,
    VoltageSource,
    ac_sweep,
    dc_op,
    mc_dc,
    noise_ac,
    sens_dc,
    tf,
)

from spice_netlist_parser import (
    AcAnalysis,
    DcAnalysis,
    McAnalysis,
    ModelCard,
    NetlistParseError,
    NoiseAnalysis,
    OpAnalysis,
    SensAnalysis,
    TfAnalysis,
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


def test_parse_tf_analysis_card_and_run_transfer_function() -> None:
    parsed = parse_netlist(
        """
Vin in 0 DC 1
R1 in out 1k
R2 out 0 1k
.tf V(out) Vin
"""
    )

    assert parsed.analyses == [TfAnalysis(output_node="out", input_source="Vin")]
    assert parsed.tf_cards() == [TfAnalysis(output_node="out", input_source="Vin")]
    card = parsed.tf_cards()[0]
    result = tf(parsed.circuit, output_node=card.output_node, input_source=card.input_source)
    assert isclose(result.transfer_ratio, 0.5, abs_tol=1e-9)


def test_tf_analysis_card_rejects_non_voltage_output_probe() -> None:
    with pytest.raises(NetlistParseError, match=r"\.tf output must be a voltage probe"):
        parse_netlist(
            """
Vin in 0 DC 1
R1 in out 1k
.tf out Vin
"""
        )


def test_parse_sens_analysis_card_and_run_dc_sensitivity() -> None:
    parsed = parse_netlist(
        """
Vin in 0 DC 1
Rtop in out 1k
Rbot out 0 1k
.sens V(out)
"""
    )

    assert parsed.analyses == [SensAnalysis(output_node="out")]
    assert parsed.sens_cards() == [SensAnalysis(output_node="out")]
    card = parsed.sens_cards()[0]
    result = sens_dc(parsed.circuit, card.output_node)
    assert result.converged
    assert isclose(result.nominal_voltage, 0.5, abs_tol=1e-9)
    assert any(
        entry.element_name == "Vin" and entry.parameter == "voltage"
        for entry in result.entries
    )


def test_sens_analysis_card_rejects_non_voltage_output_probe() -> None:
    with pytest.raises(NetlistParseError, match=r"\.sens output must be a voltage probe"):
        parse_netlist(
            """
Vin in 0 DC 1
R1 in out 1k
.sens out
"""
        )


def test_parse_mc_analysis_card_and_run_monte_carlo() -> None:
    parsed = parse_netlist(
        """
Vin in 0 DC 1
Rtop in out 1k
Rbot out 0 1k
.mc V(out) 6 0 uniform 7
"""
    )

    assert parsed.analyses == [
        McAnalysis(
            output_node="out",
            n_trials=6,
            tolerance=0.0,
            distribution="uniform",
            seed=7,
        )
    ]
    assert parsed.mc_cards() == parsed.analyses
    card = parsed.mc_cards()[0]
    result = mc_dc(
        parsed.circuit,
        card.output_node,
        n_trials=card.n_trials,
        tolerance=card.tolerance,
        distribution=card.distribution,
        seed=card.seed,
    )
    assert result.n_trials == 6
    assert isclose(result.mean, 0.5, abs_tol=1e-9)
    assert isclose(result.std_dev, 0.0, abs_tol=1e-12)


def test_mc_analysis_card_rejects_non_voltage_output_probe() -> None:
    with pytest.raises(NetlistParseError, match=r"\.mc output must be a voltage probe"):
        parse_netlist(
            """
Vin in 0 DC 1
R1 in out 1k
.mc out 10
"""
        )


def test_parse_noise_analysis_card_and_run_noise_ac() -> None:
    parsed = parse_netlist(
        """
Vin in 0 DC 1
Rtop in out 1k
Rbot out 0 1k
.noise V(out) Vin 1k temp=300
"""
    )

    assert parsed.analyses == [
        NoiseAnalysis(
            output_node="out",
            input_source="Vin",
            freqs=(1000.0,),
            temperature=300.0,
        )
    ]
    assert parsed.noise_cards() == parsed.analyses
    card = parsed.noise_cards()[0]
    result = noise_ac(
        parsed.circuit,
        card.output_node,
        card.input_source,
        freqs=list(card.freqs),
        temperature=card.temperature,
    )
    assert result.output_node == "out"
    assert result.input_source == "Vin"
    assert len(result.points) == 1
    assert result.points[0].output_psd > 0.0


def test_noise_analysis_card_rejects_non_voltage_output_probe() -> None:
    with pytest.raises(NetlistParseError, match=r"\.noise output must be a voltage probe"):
        parse_netlist(
            """
Vin in 0 DC 1
R1 in out 1k
.noise out Vin 1k
"""
        )


def test_parse_vcvs_into_operating_point_circuit() -> None:
    parsed = parse_netlist(
        """
Vctrl in 0 DC 1.5
Eamp out 0 in 0 4
Rload out 0 1k
.op
"""
    )

    assert isinstance(parsed.circuit.elements[1], VCVS)
    assert parsed.circuit.elements[1].ctrl_plus == "in"
    assert parsed.circuit.elements[1].gain == 4.0

    result = dc_op(parsed.circuit)
    assert result.converged
    assert isclose(result.node_voltages["out"], 6.0, abs_tol=1e-9)


def test_parse_cccs_into_operating_point_circuit() -> None:
    parsed = parse_netlist(
        """
Vin in 0 DC 1
Rin in mid 1k
Vsense mid 0 0
Fcopy out 0 Vsense 2
Rload out 0 500
.op
"""
    )

    assert isinstance(parsed.circuit.elements[3], CCCS)
    assert parsed.circuit.elements[3].ctrl_source == "Vsense"
    assert parsed.circuit.elements[3].beta == 2.0

    result = dc_op(parsed.circuit)
    assert result.converged
    assert isclose(result.node_voltages["out"], 1.0, abs_tol=1e-9)


def test_parse_ccvs_into_operating_point_circuit() -> None:
    parsed = parse_netlist(
        """
Vin in 0 DC 1
Rin in mid 1k
Vsense mid 0 0
Hamp out 0 Vsense 1k
Rload out 0 500
.op
"""
    )

    assert isinstance(parsed.circuit.elements[3], CCVS)
    assert parsed.circuit.elements[3].ctrl_source == "Vsense"
    assert parsed.circuit.elements[3].transresistance == 1000.0

    result = dc_op(parsed.circuit)
    assert result.converged
    assert isclose(result.node_voltages["out"], 1.0, abs_tol=1e-9)


def test_parse_diode_model_into_operating_point_circuit() -> None:
    parsed = parse_netlist(
        """
.model fast D(IS=1e-12 VT=25m)
V1 in 0 DC 0.7
D1 in out fast
Rload out 0 1k
.op
"""
    )

    assert parsed.models == {
        "fast": ModelCard("fast", "D", {"IS": 1.0e-12, "VT": 25.0e-3})
    }
    diode = parsed.circuit.elements[1]
    assert isinstance(diode, Diode)
    assert diode.anode == "in"
    assert diode.cathode == "out"
    assert diode.Is == 1.0e-12
    assert diode.Vt == 25.0e-3

    result = dc_op(parsed.circuit)
    assert result.converged
    assert 0.1 < result.node_voltages["out"] < 0.7


def test_parse_bjt_model_into_operating_point_circuit() -> None:
    parsed = parse_netlist(
        """
.model fast NPN(IS=1e-14 BF=120 VT=25m)
Vcc vcc 0 DC 5
Vb base 0 DC 0.7
Rc vcc col 100
Q1 col base 0 fast
.op
"""
    )

    assert parsed.models == {
        "fast": ModelCard("fast", "NPN", {"IS": 1.0e-14, "BF": 120.0, "VT": 25.0e-3})
    }
    bjt = parsed.circuit.elements[3]
    assert isinstance(bjt, BJT)
    assert bjt.collector == "col"
    assert bjt.base == "base"
    assert bjt.emitter == "0"
    assert bjt.polarity == "NPN"
    assert bjt.Is == 1.0e-14
    assert bjt.beta_f == 120.0
    assert bjt.Vt == 25.0e-3

    result = dc_op(parsed.circuit)
    assert result.converged
    assert 0.0 < result.node_voltages["col"] < 5.0


def test_parse_pnp_bjt_model_aliases_beta_f() -> None:
    parsed = parse_netlist(
        """
.model slow PNP(IS=2e-14 BETA_F=80 VT=26m)
Qp col base emit slow
"""
    )

    bjt = parsed.circuit.elements[0]
    assert isinstance(bjt, BJT)
    assert bjt.polarity == "PNP"
    assert bjt.Is == 2.0e-14
    assert bjt.beta_f == 80.0
    assert isclose(bjt.Vt, 26.0e-3)


def test_parse_mosfet_model_into_operating_point_circuit() -> None:
    parsed = parse_netlist(
        """
.model nfast NMOS(VT0=0.45 KP=200u LAMBDA=0.02)
Vdd vdd 0 DC 1.8
Vgate gate 0 DC 1.8
Rload vdd out 1k
M1 out gate 0 0 nfast W=2u L=180n
.op
"""
    )

    assert parsed.models["nfast"].name == "nfast"
    assert parsed.models["nfast"].kind == "NMOS"
    assert parsed.models["nfast"].params["VT0"] == 0.45
    assert isclose(parsed.models["nfast"].params["KP"], 200.0e-6)
    assert parsed.models["nfast"].params["LAMBDA"] == 0.02
    mosfet = parsed.circuit.elements[3]
    assert isinstance(mosfet, Mosfet)
    assert mosfet.drain == "out"
    assert mosfet.gate == "gate"
    assert mosfet.source == "0"
    assert mosfet.body == "0"
    assert mosfet.model.type == MosfetType.NMOS
    assert isinstance(mosfet.model.model, Level1Model)
    assert mosfet.model.model.params.VT0 == 0.45
    assert isclose(mosfet.model.model.params.KP, 200.0e-6)
    assert isclose(mosfet.model.model.params.W, 2.0e-6)
    assert isclose(mosfet.model.model.params.L, 180.0e-9)

    result = dc_op(parsed.circuit)
    assert result.converged
    assert 0.0 <= result.node_voltages["out"] < 1.8


def test_parse_pmos_mosfet_model() -> None:
    parsed = parse_netlist(
        """
.model pfast PMOS(VTO=0.4 KP=120u NSUB=1.2)
Mp out gate vdd vdd pfast W=3u L=250n
"""
    )

    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert mosfet.model.type == MosfetType.PMOS
    assert isinstance(mosfet.model.model, Level1Model)
    assert mosfet.model.model.params.VT0 == 0.4
    assert isclose(mosfet.model.model.params.KP, 120.0e-6)
    assert mosfet.model.model.params.N_SUB == 1.2
    assert isclose(mosfet.model.model.params.W, 3.0e-6)
    assert isclose(mosfet.model.model.params.L, 250.0e-9)


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


def test_parse_ac_source_specs_separate_from_dc_bias() -> None:
    parsed = parse_netlist(
        """
Vin in 0 DC 10 AC 2 90
Vbias bias 0 5
Iprobe 0 out AC 1m 90
R1 in out 1k
R2 out 0 1k
.ac dec 1 1k 1k
"""
    )

    vin = parsed.circuit.elements[0]
    vbias = parsed.circuit.elements[1]
    iprobe = parsed.circuit.elements[2]
    assert isinstance(vin, VoltageSource)
    assert isinstance(vbias, VoltageSource)
    assert isinstance(iprobe, CurrentSource)
    assert vin.voltage == 10.0
    assert vin.ac == AcSource(2.0, 90.0)
    assert vbias.ac is None
    assert iprobe.current == 0.0
    assert iprobe.ac == AcSource(1.0e-3, 90.0)

    result = ac_sweep(parsed.circuit, f_start=1.0e3, f_stop=1.0e3, n_points=1)
    pt = result.points[0]
    assert isclose(pt.node_voltages["bias"].real, 0.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["bias"].imag, 0.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["in"].real, 0.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["in"].imag, 2.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["out"].real, 0.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["out"].imag, 1.5, abs_tol=1e-12)


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


def test_expands_subcircuit_vcvs_nodes_into_engine_elements() -> None:
    parsed = parse_netlist(
        """
.subckt gain inp outp
Ebuf outp 0 inp 0 2
.ends gain
V1 in 0 DC 1.25
Xgain in out gain
Rload out 0 1k
.op
"""
    )

    assert [element.name for element in parsed.circuit.elements] == [
        "V1",
        "Xgain.Ebuf",
        "Rload",
    ]
    vcvs = parsed.circuit.elements[1]
    assert isinstance(vcvs, VCVS)
    assert vcvs.n_plus == "out"
    assert vcvs.ctrl_plus == "in"

    result = dc_op(parsed.circuit)
    assert result.converged
    assert isclose(result.node_voltages["out"], 2.5, abs_tol=1e-9)


def test_expands_subcircuit_cccs_nodes_and_control_source_into_engine_elements() -> None:
    parsed = parse_netlist(
        """
.subckt mirror inp outp
Rin inp mid 1k
Vsense mid 0 0
Fcopy outp 0 Vsense 2
.ends mirror
Vin in 0 DC 1
Xmirror in out mirror
Rload out 0 500
.op
"""
    )

    assert [element.name for element in parsed.circuit.elements] == [
        "Vin",
        "Xmirror.Rin",
        "Xmirror.Vsense",
        "Xmirror.Fcopy",
        "Rload",
    ]
    cccs = parsed.circuit.elements[3]
    assert isinstance(cccs, CCCS)
    assert cccs.n_plus == "out"
    assert cccs.ctrl_source == "Xmirror.Vsense"

    result = dc_op(parsed.circuit)
    assert result.converged
    assert isclose(result.node_voltages["out"], 1.0, abs_tol=1e-9)


def test_expands_subcircuit_ccvs_control_source_into_engine_elements() -> None:
    parsed = parse_netlist(
        """
.subckt transimpedance inp outp
Rin inp mid 1k
Vsense mid 0 0
Hamp outp 0 Vsense 1k
.ends transimpedance
Vin in 0 DC 1
Xamp in out transimpedance
Rload out 0 500
.op
"""
    )

    assert [element.name for element in parsed.circuit.elements] == [
        "Vin",
        "Xamp.Rin",
        "Xamp.Vsense",
        "Xamp.Hamp",
        "Rload",
    ]
    ccvs = parsed.circuit.elements[3]
    assert isinstance(ccvs, CCVS)
    assert ccvs.n_plus == "out"
    assert ccvs.ctrl_source == "Xamp.Vsense"

    result = dc_op(parsed.circuit)
    assert result.converged
    assert isclose(result.node_voltages["out"], 1.0, abs_tol=1e-9)


def test_expands_subcircuit_diode_nodes_into_engine_elements() -> None:
    parsed = parse_netlist(
        """
.model clamp D(IS=1e-12 VT=25m)
.subckt limiter inp outp
Dlim inp outp clamp
.ends limiter
Xlim in out limiter
"""
    )

    diode = parsed.circuit.elements[0]
    assert isinstance(diode, Diode)
    assert diode.name == "Xlim.Dlim"
    assert diode.anode == "in"
    assert diode.cathode == "out"
    assert diode.Is == 1.0e-12


def test_expands_subcircuit_bjt_nodes_into_engine_elements() -> None:
    parsed = parse_netlist(
        """
.model npn NPN(BF=50)
.subckt follower c b e
Qbuf c b inner npn
Re inner e 100
.ends follower
Xbuf out in 0 follower
"""
    )

    bjt = parsed.circuit.elements[0]
    resistor = parsed.circuit.elements[1]
    assert isinstance(bjt, BJT)
    assert bjt.name == "Xbuf.Qbuf"
    assert bjt.collector == "out"
    assert bjt.base == "in"
    assert bjt.emitter == "Xbuf.inner"
    assert bjt.beta_f == 50.0
    assert isinstance(resistor, Resistor)
    assert resistor.n_plus == "Xbuf.inner"
    assert resistor.n_minus == "0"


def test_expands_subcircuit_mosfet_nodes_into_engine_elements() -> None:
    parsed = parse_netlist(
        """
.model nfast NMOS(W=1u L=130n)
.subckt pulldown in out vss
Mpull out in inner vss nfast
Rtail inner vss 10
.ends pulldown
Xpd gate drain 0 pulldown
"""
    )

    mosfet = parsed.circuit.elements[0]
    resistor = parsed.circuit.elements[1]
    assert isinstance(mosfet, Mosfet)
    assert mosfet.name == "Xpd.Mpull"
    assert mosfet.drain == "drain"
    assert mosfet.gate == "gate"
    assert mosfet.source == "Xpd.inner"
    assert mosfet.body == "0"
    assert isinstance(resistor, Resistor)
    assert resistor.n_plus == "Xpd.inner"
    assert resistor.n_minus == "0"


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
    with pytest.raises(NetlistParseError, match="line 2: unsupported element 'Z1'"):
        parse_netlist(
            """
Z1 c b e model
"""
        )


def test_rejects_diode_with_unknown_model() -> None:
    with pytest.raises(NetlistParseError, match="line 2: unknown model 'missing' for diode 'D1'"):
        parse_netlist(
            """
D1 a 0 missing
"""
        )


def test_rejects_diode_bound_to_non_diode_model() -> None:
    with pytest.raises(NetlistParseError, match="line 3: model 'amp' has kind 'NPN'"):
        parse_netlist(
            """
.model amp NPN(IS=1e-15)
D1 a 0 amp
"""
        )


def test_rejects_bjt_with_unknown_model() -> None:
    with pytest.raises(NetlistParseError, match="line 2: unknown model 'missing' for BJT 'Q1'"):
        parse_netlist(
            """
Q1 c b e missing
"""
        )


def test_rejects_bjt_bound_to_non_bjt_model() -> None:
    with pytest.raises(NetlistParseError, match="line 3: model 'clamp' has kind 'D'"):
        parse_netlist(
            """
.model clamp D(IS=1e-15)
Q1 c b e clamp
"""
        )


def test_rejects_mosfet_with_unknown_model() -> None:
    with pytest.raises(
        NetlistParseError,
        match="line 2: unknown model 'missing' for MOSFET 'M1'",
    ):
        parse_netlist(
            """
M1 d g s b missing
"""
        )


def test_rejects_mosfet_bound_to_non_mosfet_model() -> None:
    with pytest.raises(NetlistParseError, match="line 3: model 'clamp' has kind 'D'"):
        parse_netlist(
            """
.model clamp D(IS=1e-15)
M1 d g s b clamp
"""
        )


def test_rejects_mosfet_parameter_without_assignment() -> None:
    with pytest.raises(NetlistParseError, match="line 3: invalid MOSFET parameter syntax 'W'"):
        parse_netlist(
            """
.model nfast NMOS
M1 d g s b nfast W
"""
        )


def test_rejects_unbalanced_waveform_parenthesis() -> None:
    with pytest.raises(NetlistParseError, match="unclosed parenthesis"):
        parse_netlist("V1 in 0 PULSE(0 1\n")


def test_rejects_unknown_subcircuit_instance() -> None:
    with pytest.raises(NetlistParseError, match="line 1: unknown subcircuit 'missing'"):
        parse_netlist("X1 a b missing\n")
