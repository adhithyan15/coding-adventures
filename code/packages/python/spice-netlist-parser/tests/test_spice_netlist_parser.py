from math import isclose

import pytest
from mosfet_models import Level1Model, MosfetType
from spice_engine import (
    BJT,
    CCCS,
    CCVS,
    JFET,
    VCCS,
    VCVS,
    AcSource,
    Capacitor,
    CurrentSource,
    Diode,
    Inductor,
    Mosfet,
    MutualInductor,
    Resistor,
    TransmissionLine,
    VoltageSource,
    ac_sweep,
    dc_op,
    mc_dc,
    noise_ac,
    sens_dc,
    tf,
    transient,
)

from spice_netlist_parser import (
    AcAnalysis,
    DcAnalysis,
    DistortionAnalysis,
    FourAnalysis,
    McAnalysis,
    MeasureAnalysis,
    ModelCard,
    NetlistParseError,
    NoiseAnalysis,
    OpAnalysis,
    OptionsAnalysis,
    OutputProbe,
    PlotAnalysis,
    PoleZeroAnalysis,
    PrintAnalysis,
    ProbeAnalysis,
    SaveAnalysis,
    SensAnalysis,
    TempAnalysis,
    TfAnalysis,
    TranAnalysis,
    __version__,
    build_analysis_plan,
    parse_netlist,
    run_netlist,
)
from spice_netlist_parser.parser import parse_value


def test_package_version_matches_pyproject_release() -> None:
    assert __version__ == "0.3.0"


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


def test_builds_and_runs_core_analysis_plan() -> None:
    deck = """
V1 in 0 DC 1 AC 1
R1 in out 1k
R2 out 0 1k
C1 out 0 1u IC=0
.options method=trap
.op
.dc V1 0 1 0.5
.ac dec 1 1k 1k
.tran 1m 1m
.end
"""
    parsed = parse_netlist(deck)

    plan = parsed.analysis_plan()
    assert plan == build_analysis_plan(parsed)
    assert [(step.index, step.kind) for step in plan] == [
        (1, "op"),
        (2, "dc"),
        (3, "ac"),
        (4, "tran"),
    ]

    results = parsed.run_analysis_plan()
    assert [result.kind for result in results] == ["op", "dc", "ac", "tran"]
    assert isclose(results[0].result.node_voltages["out"], 0.5, abs_tol=1e-9)
    assert len(results[1].result.points) == 3
    assert isclose(results[1].result.points[-1].node_voltages["out"], 0.5, abs_tol=1e-9)
    assert len(results[2].result.points) == 1
    assert abs(results[2].result.points[0].node_voltages["out"]) > 0.0
    assert results[3].result.method == "trap"
    assert results[3].result.points[-1].node_voltages["out"] > 0.0

    assert len(run_netlist(deck)) == 4


def test_parse_reactive_elements_and_analysis_cards() -> None:
    parsed = parse_netlist(
        """
Vstep in 0 PULSE(0 1 0 1n 1n 10n 20n)
I1 out 0 1m
Rload in out 2.2k
Cload out 0 10p IC=2.5
L1 out 0 1u IC=3m
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
    assert parsed.circuit.elements[3].initial_voltage == 2.5
    assert isinstance(parsed.circuit.elements[4], Inductor)
    assert parsed.circuit.elements[4].initial_current == 3.0e-3
    assert isinstance(parsed.circuit.elements[5], VCCS)
    assert parsed.analyses == [
        TranAnalysis(t_step=1.0e-9, t_stop=20.0e-9),
        DcAnalysis(source_name="Vstep", start=0.0, stop=1.0, step=0.5),
        AcAnalysis(mode="dec", points=10, start_hz=1.0e3, stop_hz=1.0e6),
    ]


def test_parse_mutual_inductor_card() -> None:
    parsed = parse_netlist(
        """
Lpri p 0 10m
Lsec s 0 40m
Kcouple Lpri Lsec 0.75
"""
    )

    assert isinstance(parsed.circuit.elements[2], MutualInductor)
    mutual = parsed.circuit.elements[2]
    assert mutual.name == "Kcouple"
    assert mutual.primary == "Lpri"
    assert mutual.secondary == "Lsec"
    assert mutual.coupling == 0.75


def test_mutual_inductor_rejects_missing_referenced_inductor() -> None:
    with pytest.raises(NetlistParseError, match="referenced inductor"):
        parse_netlist(
            """
Lpri p 0 10m
Kbad Lpri Lmissing 0.75
"""
        )


def test_mutual_inductor_rejects_non_finite_coupling() -> None:
    with pytest.raises(NetlistParseError, match="coupling must be finite"):
        parse_netlist(
            """
Lpri p 0 10m
Lsec s 0 40m
Kbad Lpri Lsec 1e999
"""
        )


def test_parse_transmission_line_card() -> None:
    parsed = parse_netlist(
        """
Tdelay in 0 out 0 Z0=50 TD=1n
"""
    )

    assert isinstance(parsed.circuit.elements[0], TransmissionLine)
    line = parsed.circuit.elements[0]
    assert line.name == "Tdelay"
    assert line.n1 == "in"
    assert line.n2 == "0"
    assert line.n3 == "out"
    assert line.n4 == "0"
    assert line.characteristic_impedance == 50.0
    assert line.delay == 1.0e-9


def test_transmission_line_rejects_unsupported_positional_form() -> None:
    with pytest.raises(NetlistParseError, match="invalid transmission line parameter syntax"):
        parse_netlist("Tdelay in 0 out 0 50 1n")


def test_transmission_line_rejects_missing_parameters() -> None:
    with pytest.raises(NetlistParseError, match="requires TD"):
        parse_netlist("Tdelay in 0 out 0 Z0=50")


def test_transmission_line_rejects_non_positive_parameters() -> None:
    with pytest.raises(NetlistParseError, match="characteristic impedance must be positive"):
        parse_netlist("Tdelay in 0 out 0 Z0=0 TD=1n")
    with pytest.raises(NetlistParseError, match="delay must be positive"):
        parse_netlist("Tdelay in 0 out 0 Z0=50 TD=0")


def test_parse_options_analysis_card() -> None:
    parsed = parse_netlist(
        """
.options reltol=1m abstol=1n gmin=1p method=trap noopiter
"""
    )

    assert parsed.analyses == [
        OptionsAnalysis(
            {
                "reltol": 1.0e-3,
                "abstol": 1.0e-9,
                "gmin": 1.0e-12,
                "method": "trap",
                "noopiter": True,
            }
        )
    ]
    assert parsed.options_cards() == parsed.analyses


def test_options_cards_build_engine_call_kwargs() -> None:
    parsed = parse_netlist(
        """
V1 vin 0 DC 10
R1 vin mid 1k
R2 mid 0 1k
.options reltol=1u itl1=7 gmin=1p method=gear2 trtol=2m minstep=1n maxstep=5n itl4=9
.op
.tran 1n 2n
"""
    )
    tran = parsed.tran_cards()[0]

    assert parsed.dc_op_kwargs() == {
        "tol": 1.0e-6,
        "max_iterations": 7,
        "pseudo_transient_shunt_conductance": 1.0e-12,
    }
    result = dc_op(parsed.circuit, **parsed.dc_op_kwargs())
    assert isclose(result.node_voltages["mid"], 5.0, abs_tol=1.0e-9)

    assert parsed.transient_kwargs(tran, adaptive=True) == {
        "method": "gear2",
        "tol": 1.0e-6,
        "tol_lte": 2.0e-3,
        "min_step": 1.0e-9,
        "max_step": 5.0e-9,
        "max_iterations": 9,
        "adaptive": True,
    }
    transient_result = transient(
        parsed.circuit,
        t_stop=tran.t_stop,
        t_step=tran.t_step,
        **parsed.transient_kwargs(tran, adaptive=True),
    )
    assert transient_result.converged
    assert transient_result.method == "gear2"


def test_parse_temp_analysis_card() -> None:
    parsed = parse_netlist(".temp 27 75 -40")

    assert parsed.analyses == [TempAnalysis((27.0, 75.0, -40.0))]
    assert parsed.temp_cards() == parsed.analyses
    assert isclose(parsed.operating_temperature_kelvin(), 300.15, abs_tol=1e-12)
    assert isclose(parsed.operating_temperature_kelvin(1), 348.15, abs_tol=1e-12)


def test_operating_temperature_defaults_without_temp_cards() -> None:
    parsed = parse_netlist("R1 in out 1k")

    assert parsed.operating_temperature_kelvin(default=301.0) == 301.0
    with pytest.raises(NetlistParseError, match=r"temperature index 3 exceeds \.temp entries"):
        parse_netlist(".temp 27").operating_temperature_kelvin(3)


def test_temp_card_rejects_missing_temperatures() -> None:
    with pytest.raises(NetlistParseError, match=r"\.temp expects at least 2 fields"):
        parse_netlist(".temp")


def test_parse_print_and_plot_output_cards() -> None:
    parsed = parse_netlist(
        """
.print TRAN V(out) I(Vin)
.plot ac V(in) V(out)
"""
    )

    assert parsed.analyses == [
        PrintAnalysis(
            "tran",
            (
                OutputProbe("voltage", "out"),
                OutputProbe("current", "Vin"),
            ),
        ),
        PlotAnalysis(
            "ac",
            (
                OutputProbe("voltage", "in"),
                OutputProbe("voltage", "out"),
            ),
        ),
    ]
    assert parsed.print_cards() == [parsed.analyses[0]]
    assert parsed.plot_cards() == [parsed.analyses[1]]


def test_parse_save_probe_and_measure_cards() -> None:
    parsed = parse_netlist(
        """
.save V(out) I(Vin)
.probe tran V(out)
.measure tran peak MAX V(out) FROM=0 TO=1m
"""
    )

    assert parsed.analyses == [
        SaveAnalysis(
            (
                OutputProbe("voltage", "out"),
                OutputProbe("current", "Vin"),
            )
        ),
        ProbeAnalysis("tran", (OutputProbe("voltage", "out"),)),
        MeasureAnalysis(
            "tran",
            "peak",
            "max",
            OutputProbe("voltage", "out"),
            start=0.0,
            stop=1.0e-3,
        ),
    ]
    assert parsed.save_cards() == [parsed.analyses[0]]
    assert parsed.probe_cards() == [parsed.analyses[1]]
    assert parsed.measure_cards() == [parsed.analyses[2]]


def test_output_cards_reject_missing_or_unknown_probes() -> None:
    with pytest.raises(NetlistParseError, match=r"\.print expects at least 3 fields"):
        parse_netlist(".print tran")
    with pytest.raises(NetlistParseError, match=r"\.plot probe must be V\(node\) or I\(source\)"):
        parse_netlist(".plot tran P(out)")
    with pytest.raises(NetlistParseError, match=r"\.save probe must be V\(node\) or I\(source\)"):
        parse_netlist(".save P(out)")
    with pytest.raises(NetlistParseError, match=r"\.probe probe must be V\(node\) or I\(source\)"):
        parse_netlist(".probe tran")
    with pytest.raises(NetlistParseError, match=r"\.measure FIND requires AT=<value>"):
        parse_netlist(".measure tran final FIND V(out)")
    with pytest.raises(NetlistParseError, match=r"\.measure operation must be FIND"):
        parse_netlist(".measure tran peak PEAK V(out) AT=1m")


def test_select_outputs_and_measure_results_from_analysis_plan() -> None:
    deck = """
V1 in 0 DC 1 AC 1
R1 in out 1k
R2 out 0 1k
C1 out 0 1u IC=0
.save V(out)
.print dc V(in)
.probe tran I(V1)
.measure dc half FIND V(out) AT=1
.measure tran final FIND V(out) AT=1m
.measure tran average AVG V(out)
.op
.dc V1 0 1 0.5
.ac dec 1 1k 1k
.tran 1m 1m
.end
"""
    parsed = parse_netlist(deck)
    results = parsed.run_analysis_plan()

    outputs = parsed.select_outputs(results)
    assert [output.kind for output in outputs] == ["op", "dc", "ac", "tran"]
    assert isclose(outputs[0].rows[0].values["V(out)"], 0.5, abs_tol=1e-9)
    assert list(outputs[1].rows[-1].values) == ["V(out)", "V(in)"]
    assert isclose(outputs[1].rows[-1].values["V(in)"], 1.0, abs_tol=1e-9)
    assert isinstance(outputs[2].rows[0].values["V(out)"], complex)
    assert "I(V1)" in outputs[3].rows[-1].values

    measures = parsed.measure_results(results)
    assert [measure.name for measure in measures] == ["half", "final", "average"]
    assert isclose(measures[0].value, 0.5, abs_tol=1e-9)
    assert isclose(measures[1].value, outputs[3].rows[-1].values["V(out)"], abs_tol=1e-9)
    assert 0.0 < measures[2].value <= outputs[3].rows[-1].values["V(out)"]


def test_parse_four_analysis_card() -> None:
    parsed = parse_netlist(".four 1k V(out) I(Vin)")

    assert parsed.analyses == [
        FourAnalysis(
            1.0e3,
            (
                OutputProbe("voltage", "out"),
                OutputProbe("current", "Vin"),
            ),
        )
    ]
    assert parsed.four_cards() == parsed.analyses


def test_four_card_rejects_missing_or_unknown_probes() -> None:
    with pytest.raises(NetlistParseError, match=r"\.four expects at least 3 fields"):
        parse_netlist(".four 1k")
    with pytest.raises(NetlistParseError, match=r"\.four probe must be V\(node\) or I\(source\)"):
        parse_netlist(".four 1k P(out)")


def test_parse_distortion_and_pole_zero_analysis_cards() -> None:
    parsed = parse_netlist(
        """
.disto dec 5 1k 1meg V(out) I(Vin)
.pz V(out) Vin pole
"""
    )

    assert parsed.analyses == [
        DistortionAnalysis(
            "dec",
            5,
            1.0e3,
            1.0e6,
            (
                OutputProbe("voltage", "out"),
                OutputProbe("current", "Vin"),
            ),
        ),
        PoleZeroAnalysis("out", "Vin", "pole"),
    ]
    assert parsed.distortion_cards() == [parsed.analyses[0]]
    assert parsed.pole_zero_cards() == [parsed.analyses[1]]


def test_distortion_and_pole_zero_cards_reject_invalid_shapes() -> None:
    with pytest.raises(NetlistParseError, match=r"\.disto expects at least 6 fields"):
        parse_netlist(".disto dec 5 1k 1meg")
    with pytest.raises(NetlistParseError, match=r"\.disto probe must be V\(node\) or I\(source\)"):
        parse_netlist(".disto dec 5 1k 1meg P(out)")
    with pytest.raises(NetlistParseError, match=r"\.pz output must be a voltage probe"):
        parse_netlist(".pz out Vin")
    with pytest.raises(NetlistParseError, match=r"\.pz kind must be"):
        parse_netlist(".pz V(out) Vin residue")


def test_parse_transient_method_from_tran_card() -> None:
    parsed = parse_netlist(".tran 1n 20n method=gear2")

    assert parsed.tran_cards() == [
        TranAnalysis(t_step=1.0e-9, t_stop=20.0e-9, method="gear2")
    ]
    assert parsed.transient_method(parsed.tran_cards()[0]) == "gear2"


def test_transient_method_falls_back_to_options_and_tran_takes_precedence() -> None:
    parsed = parse_netlist(
        """
.options method=trap
.tran 1n 20n method=euler
"""
    )

    assert parsed.options_cards()[0].values["method"] == "trap"
    assert parsed.transient_method() == "trap"
    assert parsed.transient_method(parsed.tran_cards()[0]) == "euler"


def test_transient_method_rejects_unsupported_values() -> None:
    with pytest.raises(NetlistParseError, match="must be euler, trap, or gear2"):
        parse_netlist(".tran 1n 20n method=bogus")
    with pytest.raises(NetlistParseError, match="must be euler, trap, or gear2"):
        parse_netlist(".options method=bogus")


def test_options_card_rejects_empty_values() -> None:
    with pytest.raises(NetlistParseError, match=r"\.options 'gmin' requires a value"):
        parse_netlist(".options gmin=")


def test_capacitor_rejects_unsupported_element_params() -> None:
    with pytest.raises(NetlistParseError, match="unsupported capacitor parameter"):
        parse_netlist("C1 in 0 1u FOO=1")


def test_inductor_rejects_unsupported_element_params() -> None:
    with pytest.raises(NetlistParseError, match="unsupported inductor parameter"):
        parse_netlist("L1 in 0 1u FOO=1")


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
.temp 75
Vin in 0 DC 1
Rtop in out 1k
Rbot out 0 1k
.noise V(out) Vin 1k temp=300
"""
    )

    assert parsed.analyses == [
        TempAnalysis((75.0,)),
        NoiseAnalysis(
            output_node="out",
            input_source="Vin",
            freqs=(1000.0,),
            temperature=300.0,
            temperature_is_explicit=True,
        )
    ]
    assert parsed.noise_cards() == [parsed.analyses[1]]
    card = parsed.noise_cards()[0]
    assert parsed.noise_temperature_kelvin(card) == 300.0
    result = noise_ac(
        parsed.circuit,
        card.output_node,
        card.input_source,
        freqs=list(card.freqs),
        temperature=parsed.noise_temperature_kelvin(card),
    )
    assert result.output_node == "out"
    assert result.input_source == "Vin"
    assert len(result.points) == 1
    assert result.points[0].output_psd > 0.0


def test_noise_analysis_uses_temp_card_when_noise_temp_is_omitted() -> None:
    parsed = parse_netlist(
        """
.temp 50
Vin in 0 DC 1
Rtop in out 1k
Rbot out 0 1k
.noise V(out) Vin 1k
"""
    )
    card = parsed.noise_cards()[0]

    assert not card.temperature_is_explicit
    assert isclose(parsed.noise_temperature_kelvin(card), 323.15, abs_tol=1e-12)


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
.model fast D(IS=1e-12 VT=25m N=2 BV=5 IBV=1u CJO=2p TT=4n)
V1 in 0 DC 0.7
D1 in out fast
Rload out 0 1k
.op
"""
    )

    assert parsed.models == {
        "fast": ModelCard(
            "fast",
            "D",
            {
                "IS": 1.0e-12,
                "VT": 25.0e-3,
                "N": 2.0,
                "BV": 5.0,
                "IBV": 1.0e-6,
                "CJO": 2.0e-12,
                "TT": 4.0e-9,
            },
        )
    }
    diode = parsed.circuit.elements[1]
    assert isinstance(diode, Diode)
    assert diode.anode == "in"
    assert diode.cathode == "out"
    assert diode.Is == 1.0e-12
    assert diode.Vt == 25.0e-3
    assert diode.N == 2.0
    assert diode.BV == 5.0
    assert diode.IBV == 1.0e-6
    assert diode.Cjo == 2.0e-12
    assert diode.Tt == 4.0e-9

    result = dc_op(parsed.circuit)
    assert result.converged
    assert 0.0 < result.node_voltages["out"] < 0.7


def test_parse_diode_saturation_current_alias() -> None:
    parsed = parse_netlist(
        """
.model clamp D(JS=2p)
D1 in out clamp
"""
    )

    diode = parsed.circuit.elements[0]
    assert isinstance(diode, Diode)
    assert diode.Is == 2.0e-12


@pytest.mark.parametrize("parameter", ["IS", "JS"])
@pytest.mark.parametrize("value", ["0", "-1p", "1e999"])
def test_rejects_invalid_diode_saturation_current(
    parameter: str, value: str
) -> None:
    with pytest.raises(
        NetlistParseError, match="diode IS must be finite and positive"
    ):
        parse_netlist(f".model clamp D({parameter}={value})")


def test_parse_diode_thermal_voltage_alias() -> None:
    parsed = parse_netlist(
        """
.model clamp D(V_T=27m)
D1 in out clamp
"""
    )

    diode = parsed.circuit.elements[0]
    assert isinstance(diode, Diode)
    assert isclose(diode.Vt, 27.0e-3)


@pytest.mark.parametrize("parameter", ["VT", "V_T"])
@pytest.mark.parametrize("value", ["0", "-1m", "1e999"])
def test_rejects_invalid_diode_thermal_voltage(parameter: str, value: str) -> None:
    with pytest.raises(
        NetlistParseError, match="diode VT must be finite and positive"
    ):
        parse_netlist(f".model clamp D({parameter}={value})")


def test_parse_bjt_model_into_operating_point_circuit() -> None:
    parsed = parse_netlist(
        """
.model fast NPN(IS=1e-14 BF=120 VT=25m CJE=2p CJC=3p TF=4n TR=5n)
Vcc vcc 0 DC 5
Vb base 0 DC 0.7
Rc vcc col 100
Q1 col base 0 fast
.op
"""
    )

    assert parsed.models == {
        "fast": ModelCard(
            "fast",
            "NPN",
            {
                "IS": 1.0e-14,
                "BF": 120.0,
                "VT": 25.0e-3,
                "CJE": 2.0e-12,
                "CJC": 3.0e-12,
                "TF": 4.0e-9,
                "TR": 5.0e-9,
            },
        )
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
    assert bjt.Cje == 2.0e-12
    assert bjt.Cjc == 3.0e-12
    assert bjt.Tf == 4.0e-9
    assert bjt.Tr == 5.0e-9

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


def test_parse_jfet_model_into_operating_point_circuit() -> None:
    parsed = parse_netlist(
        """
.model fast NJF(BETA=2m VTO=-3 LAMBDA=0.02)
J1 drain gate source fast
"""
    )

    assert parsed.models == {
        "fast": ModelCard(
            "fast",
            "NJF",
            {"BETA": 2.0e-3, "VTO": -3.0, "LAMBDA": 0.02},
        )
    }
    jfet = parsed.circuit.elements[0]
    assert isinstance(jfet, JFET)
    assert jfet.drain == "drain"
    assert jfet.gate == "gate"
    assert jfet.source == "source"
    assert jfet.polarity == "NJF"
    assert jfet.beta == 2.0e-3
    assert jfet.vto == -3.0
    assert jfet.lambda_ == 0.02


@pytest.mark.parametrize("parameter", ["CGS", "CGS0"])
def test_parse_jfet_gate_source_capacitance_aliases(parameter: str) -> None:
    parsed = parse_netlist(
        f"""
.model fast NJF({parameter}=3p)
J1 drain gate source fast
"""
    )

    jfet = parsed.circuit.elements[0]
    assert isinstance(jfet, JFET)
    assert isclose(jfet.Cgs, 3.0e-12)


@pytest.mark.parametrize("value", ["-1p", "1e999"])
def test_rejects_invalid_jfet_gate_source_capacitance(value: str) -> None:
    with pytest.raises(
        NetlistParseError, match="JFET CGS must be finite and non-negative"
    ):
        parse_netlist(f".model fast NJF(CGS={value})")


@pytest.mark.parametrize("parameter", ["CGD", "CGD0"])
def test_parse_jfet_gate_drain_capacitance_aliases(parameter: str) -> None:
    parsed = parse_netlist(
        f"""
.model fast NJF({parameter}=4p)
J1 drain gate source fast
"""
    )

    jfet = parsed.circuit.elements[0]
    assert isinstance(jfet, JFET)
    assert isclose(jfet.Cgd, 4.0e-12)


@pytest.mark.parametrize("value", ["-1p", "1e999"])
def test_rejects_invalid_jfet_gate_drain_capacitance(value: str) -> None:
    with pytest.raises(
        NetlistParseError, match="JFET CGD must be finite and non-negative"
    ):
        parse_netlist(f".model fast NJF(CGD={value})")


def test_parse_pjf_model_aliases_beta() -> None:
    parsed = parse_netlist(
        """
.model pslow PJF(B=750u)
Jp drain gate source pslow
"""
    )

    jfet = parsed.circuit.elements[0]
    assert isinstance(jfet, JFET)
    assert jfet.polarity == "PJF"
    assert isclose(jfet.beta, 750.0e-6)
    assert jfet.vto == 2.0


def test_parse_mosfet_model_into_operating_point_circuit() -> None:
    parsed = parse_netlist(
        """
.model nfast NMOS(VT0=0.45 KP=200u LAMBDA=0.02 CGSO=3p CGDO=4p CGBO=5p CBS=6p CBD=7p)
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
    assert isclose(parsed.models["nfast"].params["CGSO"], 3.0e-12)
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
    assert isclose(mosfet.model.model.params.CGSO, 3.0e-12)
    assert isclose(mosfet.model.model.params.CGDO, 4.0e-12)
    assert isclose(mosfet.model.model.params.CGBO, 5.0e-12)
    assert isclose(mosfet.model.model.params.CBS, 6.0e-12)
    assert isclose(mosfet.model.model.params.CBD, 7.0e-12)
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


@pytest.mark.parametrize("drain_squares", ["-1", "1e999"])
def test_rejects_invalid_mosfet_instance_drain_squares(drain_squares: str) -> None:
    with pytest.raises(
        NetlistParseError, match="MOSFET NRD must be finite and non-negative"
    ):
        parse_netlist(
            f".model nfast NMOS\nM1 d g s b nfast NRD={drain_squares}\n"
        )


@pytest.mark.parametrize(("drain_squares", "expected"), [("0", 0.0), ("2.5", 2.5)])
def test_lowers_valid_mosfet_instance_drain_squares(
    drain_squares: str, expected: float
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS\nM1 d g s b nfast NRD={drain_squares}\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert expected == mosfet.model.model.params.NRD


@pytest.mark.parametrize("source_squares", ["-1", "1e999"])
def test_rejects_invalid_mosfet_instance_source_squares(source_squares: str) -> None:
    with pytest.raises(
        NetlistParseError, match="MOSFET NRS must be finite and non-negative"
    ):
        parse_netlist(
            f".model nfast NMOS\nM1 d g s b nfast NRS={source_squares}\n"
        )


@pytest.mark.parametrize(("source_squares", "expected"), [("0", 0.0), ("3.5", 3.5)])
def test_lowers_valid_mosfet_instance_source_squares(
    source_squares: str, expected: float
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS\nM1 d g s b nfast NRS={source_squares}\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert expected == mosfet.model.model.params.NRS


@pytest.mark.parametrize("drain_area", ["-1n", "1e999"])
def test_rejects_invalid_mosfet_instance_drain_area(drain_area: str) -> None:
    with pytest.raises(
        NetlistParseError, match="MOSFET AD must be finite and non-negative"
    ):
        parse_netlist(f".model nfast NMOS\nM1 d g s b nfast AD={drain_area}\n")


@pytest.mark.parametrize(("drain_area", "expected"), [("0", 0.0), ("3n", 3.0e-9)])
def test_lowers_valid_mosfet_instance_drain_area(
    drain_area: str, expected: float
) -> None:
    parsed = parse_netlist(f".model nfast NMOS\nM1 d g s b nfast AD={drain_area}\n")
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.AD, expected)


@pytest.mark.parametrize("source_area", ["-1n", "1e999"])
def test_rejects_invalid_mosfet_instance_source_area(source_area: str) -> None:
    with pytest.raises(
        NetlistParseError, match="MOSFET AS must be finite and non-negative"
    ):
        parse_netlist(f".model nfast NMOS\nM1 d g s b nfast AS={source_area}\n")


@pytest.mark.parametrize(("source_area", "expected"), [("0", 0.0), ("4n", 4.0e-9)])
def test_lowers_valid_mosfet_instance_source_area(
    source_area: str, expected: float
) -> None:
    parsed = parse_netlist(f".model nfast NMOS\nM1 d g s b nfast AS={source_area}\n")
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.AS, expected)


@pytest.mark.parametrize("drain_perimeter", ["-1u", "1e999"])
def test_rejects_invalid_mosfet_instance_drain_perimeter(
    drain_perimeter: str,
) -> None:
    with pytest.raises(
        NetlistParseError, match="MOSFET PD must be finite and non-negative"
    ):
        parse_netlist(
            f".model nfast NMOS\nM1 d g s b nfast PD={drain_perimeter}\n"
        )


@pytest.mark.parametrize(
    ("drain_perimeter", "expected"), [("0", 0.0), ("6u", 6.0e-6)]
)
def test_lowers_valid_mosfet_instance_drain_perimeter(
    drain_perimeter: str, expected: float
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS\nM1 d g s b nfast PD={drain_perimeter}\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.PD, expected)


@pytest.mark.parametrize("source_perimeter", ["-1u", "1e999"])
def test_rejects_invalid_mosfet_instance_source_perimeter(
    source_perimeter: str,
) -> None:
    with pytest.raises(
        NetlistParseError, match="MOSFET PS must be finite and non-negative"
    ):
        parse_netlist(
            f".model nfast NMOS\nM1 d g s b nfast PS={source_perimeter}\n"
        )


@pytest.mark.parametrize(
    ("source_perimeter", "expected"), [("0", 0.0), ("7u", 7.0e-6)]
)
def test_lowers_valid_mosfet_instance_source_perimeter(
    source_perimeter: str, expected: float
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS\nM1 d g s b nfast PS={source_perimeter}\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.PS, expected)


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
.model clamp D(IS=1e-12 VT=25m N=2 BV=5 IBV=1u CJ0=3p TT=5n)
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
    assert diode.N == 2.0
    assert diode.BV == 5.0
    assert diode.IBV == 1.0e-6
    assert diode.Cjo == 3.0e-12
    assert diode.Tt == 5.0e-9


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


def test_expands_subcircuit_jfet_nodes_into_engine_elements() -> None:
    parsed = parse_netlist(
        """
.model nchan NJF(BETA=1m)
.subckt source_follower d g s
Jbuf d g inner nchan
Rtail inner s 100
.ends source_follower
Xbuf out in 0 source_follower
"""
    )

    jfet = parsed.circuit.elements[0]
    resistor = parsed.circuit.elements[1]
    assert isinstance(jfet, JFET)
    assert jfet.name == "Xbuf.Jbuf"
    assert jfet.drain == "out"
    assert jfet.gate == "in"
    assert jfet.source == "Xbuf.inner"
    assert jfet.beta == 1.0e-3
    assert isinstance(resistor, Resistor)
    assert resistor.n_plus == "Xbuf.inner"
    assert resistor.n_minus == "0"


def test_expands_subcircuit_mutual_inductor_refs_into_engine_elements() -> None:
    parsed = parse_netlist(
        """
.subckt transformer p1 p2 s1 s2
Lpri p1 p2 10m
Lsec s1 s2 40m
Kcore Lpri Lsec 0.9
.ends transformer
Xtx in 0 out 0 transformer
"""
    )

    mutual = parsed.circuit.elements[2]
    assert isinstance(mutual, MutualInductor)
    assert mutual.name == "Xtx.Kcore"
    assert mutual.primary == "Xtx.Lpri"
    assert mutual.secondary == "Xtx.Lsec"
    assert mutual.coupling == 0.9


def test_expands_subcircuit_transmission_line_nodes_into_engine_elements() -> None:
    parsed = parse_netlist(
        """
.subckt delay in out
T1 in 0 out 0 Z0=75 TD=2n
.ends delay
Xdelay a b delay
"""
    )

    line = parsed.circuit.elements[0]
    assert isinstance(line, TransmissionLine)
    assert line.name == "Xdelay.T1"
    assert line.n1 == "a"
    assert line.n2 == "0"
    assert line.n3 == "b"
    assert line.n4 == "0"
    assert line.characteristic_impedance == 75.0
    assert line.delay == 2.0e-9


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


@pytest.mark.parametrize("level", ["0", "2", "1.000000000002", "1e999"])
def test_rejects_unsupported_mosfet_model_levels(level: str) -> None:
    with pytest.raises(
        NetlistParseError,
        match="only MOS LEVEL=1 model cards are supported",
    ):
        parse_netlist(f".model nfast NMOS(LEVEL={level})\nM1 d g s b nfast\n")


@pytest.mark.parametrize("parameters", ["LEVEL=1", "LEVEL=1.0000000000005", ""])
def test_preserves_supported_and_implicit_mosfet_model_level_one(parameters: str) -> None:
    parsed = parse_netlist(f".model nfast NMOS({parameters})\nM1 d g s b nfast\n")

    assert isinstance(parsed.circuit.elements[0], Mosfet)


@pytest.mark.parametrize("oxide_thickness", ["0", "-1n", "1e999"])
def test_rejects_invalid_mosfet_model_oxide_thickness(oxide_thickness: str) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET TOX must be finite and positive",
    ):
        parse_netlist(f".model nfast NMOS(TOX={oxide_thickness})\nM1 d g s b nfast\n")


def test_lowers_mosfet_model_oxide_thickness() -> None:
    parsed = parse_netlist(".model nfast NMOS(TOX=7n)\nM1 d g s b nfast\n")

    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.TOX, 7.0e-9)


@pytest.mark.parametrize("surface_mobility", ["-1", "1e999"])
def test_rejects_invalid_mosfet_model_surface_mobility(surface_mobility: str) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET U0 must be finite and non-negative",
    ):
        parse_netlist(f".model nfast NMOS(U0={surface_mobility})\nM1 d g s b nfast\n")


@pytest.mark.parametrize("alias", ["U0", "UO"])
def test_lowers_mosfet_model_surface_mobility_and_derives_kp(alias: str) -> None:
    parsed = parse_netlist(f".model nfast NMOS({alias}=450 TOX=12n)\nM1 d g s b nfast\n")

    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert mosfet.model.model.params.U0 == 450.0
    assert isclose(mosfet.model.model.params.KP, 1.294924875e-4)


def test_explicit_mosfet_model_kp_overrides_mobility_derivation() -> None:
    parsed = parse_netlist(".model nfast NMOS(U0=450 TOX=12n KP=123u)\nM1 d g s b nfast\n")

    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.KP, 123.0e-6)


@pytest.mark.parametrize("transconductance", ["0", "-1u", "1e999"])
def test_rejects_invalid_explicit_mosfet_model_transconductance(
    transconductance: str,
) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET KP must be finite and positive",
    ):
        parse_netlist(f".model nfast NMOS(KP={transconductance})\nM1 d g s b nfast\n")


def test_preserves_positive_explicit_mosfet_model_transconductance() -> None:
    parsed = parse_netlist(".model nfast NMOS(KP=175u)\nM1 d g s b nfast\n")

    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.KP, 175.0e-6)


@pytest.mark.parametrize("alias", ["VT0", "VTO", "VTH"])
def test_rejects_non_finite_mosfet_model_threshold_voltage(alias: str) -> None:
    with pytest.raises(NetlistParseError, match="MOSFET VT0 must be finite"):
        parse_netlist(f".model nfast NMOS({alias}=1e999)\nM1 d g s b nfast\n")


@pytest.mark.parametrize("alias", ["VT0", "VTO", "VTH"])
def test_lowers_finite_mosfet_model_threshold_voltage_aliases(alias: str) -> None:
    parsed = parse_netlist(f".model nfast NMOS({alias}=-0.38)\nM1 d g s b nfast\n")

    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert mosfet.model.model.params.VT0 == -0.38


@pytest.mark.parametrize("alias", ["LAMBDA", "LAM"])
def test_rejects_non_finite_mosfet_model_channel_modulation(alias: str) -> None:
    with pytest.raises(NetlistParseError, match="MOSFET LAMBDA must be finite"):
        parse_netlist(f".model nfast NMOS({alias}=1e999)\nM1 d g s b nfast\n")


@pytest.mark.parametrize("alias", ["LAMBDA", "LAM"])
def test_lowers_finite_mosfet_model_channel_modulation_aliases(alias: str) -> None:
    parsed = parse_netlist(f".model nfast NMOS({alias}=-0.02)\nM1 d g s b nfast\n")

    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert mosfet.model.model.params.LAMBDA == -0.02


@pytest.mark.parametrize("body_effect", ["-0.01", "1e999"])
def test_rejects_invalid_mosfet_model_body_effect(body_effect: str) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET GAMMA must be finite and non-negative",
    ):
        parse_netlist(f".model nfast NMOS(GAMMA={body_effect})\nM1 d g s b nfast\n")


@pytest.mark.parametrize("body_effect", ["0", "0.45"])
def test_lowers_valid_mosfet_model_body_effect(body_effect: str) -> None:
    parsed = parse_netlist(f".model nfast NMOS(GAMMA={body_effect})\nM1 d g s b nfast\n")

    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert float(body_effect) == mosfet.model.model.params.GAMMA


@pytest.mark.parametrize("surface_potential", ["0", "-0.01", "1e999"])
def test_rejects_invalid_mosfet_model_surface_potential(
    surface_potential: str,
) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET PHI must be finite and positive",
    ):
        parse_netlist(
            f".model nfast NMOS(PHI={surface_potential})\nM1 d g s b nfast\n"
        )


def test_lowers_positive_mosfet_model_surface_potential() -> None:
    parsed = parse_netlist(".model nfast NMOS(PHI=0.65)\nM1 d g s b nfast\n")

    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert mosfet.model.model.params.PHI == 0.65


@pytest.mark.parametrize("width", ["0", "-1u", "1e999"])
def test_rejects_invalid_mosfet_model_width(width: str) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET W must be finite and positive",
    ):
        parse_netlist(f".model nfast NMOS(W={width})\nM1 d g s b nfast\n")


def test_lowers_positive_mosfet_model_width() -> None:
    parsed = parse_netlist(".model nfast NMOS(W=4u)\nM1 d g s b nfast\n")

    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.W, 4.0e-6)


@pytest.mark.parametrize("length", ["0", "-1u", "1e999"])
def test_rejects_invalid_mosfet_model_length(length: str) -> None:
    with pytest.raises(NetlistParseError, match="MOSFET L must be finite and positive"):
        parse_netlist(f".model nfast NMOS(L={length})\nM1 d g s b nfast\n")


def test_lowers_positive_mosfet_model_length() -> None:
    parsed = parse_netlist(".model nfast NMOS(L=2u)\nM1 d g s b nfast\n")
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.L, 2.0e-6)


@pytest.mark.parametrize("parameters", ["LD=-1n", "LD=1e999", "L=100n LD=50n"])
def test_rejects_invalid_mosfet_model_lateral_diffusion(parameters: str) -> None:
    with pytest.raises(
        NetlistParseError,
        match=r"MOSFET LD must be finite and non-negative with L - 2\*LD > 0",
    ):
        parse_netlist(f".model nfast NMOS({parameters})\nM1 d g s b nfast\n")


@pytest.mark.parametrize("lateral_diffusion", ["0", "10n"])
def test_lowers_valid_mosfet_model_lateral_diffusion(
    lateral_diffusion: str,
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS(L=180n LD={lateral_diffusion})\nM1 d g s b nfast\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.LD, parse_value(lateral_diffusion))


@pytest.mark.parametrize("saturation_current", ["0", "-1p", "1e999"])
def test_rejects_invalid_mosfet_model_saturation_current(
    saturation_current: str,
) -> None:
    with pytest.raises(NetlistParseError, match="MOSFET IS must be finite and positive"):
        parse_netlist(
            f".model nfast NMOS(IS={saturation_current})\nM1 d g s b nfast\n"
        )


def test_lowers_positive_mosfet_model_saturation_current() -> None:
    parsed = parse_netlist(".model nfast NMOS(IS=2f)\nM1 d g s b nfast\n")
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.IS, 2.0e-15)


@pytest.mark.parametrize("alias", ["TNOM", "T_NOM"])
@pytest.mark.parametrize("temperature", ["0", "-1", "1e999"])
def test_rejects_invalid_mosfet_model_nominal_temperature(
    alias: str, temperature: str
) -> None:
    with pytest.raises(NetlistParseError, match="MOSFET TNOM must be finite and positive"):
        parse_netlist(f".model nfast NMOS({alias}={temperature})\nM1 d g s b nfast\n")


@pytest.mark.parametrize("alias", ["TNOM", "T_NOM"])
def test_lowers_positive_mosfet_model_nominal_temperature(alias: str) -> None:
    parsed = parse_netlist(f".model nfast NMOS({alias}=325)\nM1 d g s b nfast\n")
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert mosfet.model.model.params.T_NOM == 325.0


@pytest.mark.parametrize("drain_resistance", ["-1", "1e999"])
def test_rejects_invalid_mosfet_model_drain_resistance(
    drain_resistance: str,
) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET RD must be finite and non-negative",
    ):
        parse_netlist(
            f".model nfast NMOS(RD={drain_resistance})\nM1 d g s b nfast\n"
        )


@pytest.mark.parametrize("drain_resistance", ["0", "12.5"])
def test_lowers_valid_mosfet_model_drain_resistance(
    drain_resistance: str,
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS(RD={drain_resistance})\nM1 d g s b nfast\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert float(drain_resistance) == mosfet.model.model.params.RD


@pytest.mark.parametrize("source_resistance", ["-1", "1e999"])
def test_rejects_invalid_mosfet_model_source_resistance(
    source_resistance: str,
) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET RS must be finite and non-negative",
    ):
        parse_netlist(
            f".model nfast NMOS(RS={source_resistance})\nM1 d g s b nfast\n"
        )


@pytest.mark.parametrize("source_resistance", ["0", "9.75"])
def test_lowers_valid_mosfet_model_source_resistance(
    source_resistance: str,
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS(RS={source_resistance})\nM1 d g s b nfast\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert float(source_resistance) == mosfet.model.model.params.RS


@pytest.mark.parametrize("sheet_resistance", ["-1", "1e999"])
def test_rejects_invalid_mosfet_model_sheet_resistance(
    sheet_resistance: str,
) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET RSH must be finite and non-negative",
    ):
        parse_netlist(
            f".model nfast NMOS(RSH={sheet_resistance})\nM1 d g s b nfast\n"
        )


@pytest.mark.parametrize("sheet_resistance", ["0", "42.5"])
def test_lowers_valid_mosfet_model_sheet_resistance(
    sheet_resistance: str,
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS(RSH={sheet_resistance})\nM1 d g s b nfast\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert float(sheet_resistance) == mosfet.model.model.params.RSH


@pytest.mark.parametrize("junction_capacitance", ["-1p", "1e999"])
def test_rejects_invalid_mosfet_model_junction_capacitance(
    junction_capacitance: str,
) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET CJ must be finite and non-negative",
    ):
        parse_netlist(
            f".model nfast NMOS(CJ={junction_capacitance})\nM1 d g s b nfast\n"
        )


@pytest.mark.parametrize(
    ("junction_capacitance", "expected"), [("0", 0.0), ("2p", 2.0e-12)]
)
def test_lowers_valid_mosfet_model_junction_capacitance(
    junction_capacitance: str, expected: float
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS(CJ={junction_capacitance})\nM1 d g s b nfast\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.CJ, expected)


@pytest.mark.parametrize("sidewall_capacitance", ["-1p", "1e999"])
def test_rejects_invalid_mosfet_model_sidewall_capacitance(
    sidewall_capacitance: str,
) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET CJSW must be finite and non-negative",
    ):
        parse_netlist(
            f".model nfast NMOS(CJSW={sidewall_capacitance})\nM1 d g s b nfast\n"
        )


@pytest.mark.parametrize(
    ("sidewall_capacitance", "expected"), [("0", 0.0), ("3p", 3.0e-12)]
)
def test_lowers_valid_mosfet_model_sidewall_capacitance(
    sidewall_capacitance: str, expected: float
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS(CJSW={sidewall_capacitance})\nM1 d g s b nfast\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.CJSW, expected)


@pytest.mark.parametrize("junction_current", ["-1p", "1e999"])
def test_rejects_invalid_mosfet_model_junction_current(
    junction_current: str,
) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET JS must be finite and non-negative",
    ):
        parse_netlist(
            f".model nfast NMOS(JS={junction_current})\nM1 d g s b nfast\n"
        )


@pytest.mark.parametrize(
    ("junction_current", "expected"), [("0", 0.0), ("4p", 4.0e-12)]
)
def test_lowers_valid_mosfet_model_junction_current(
    junction_current: str, expected: float
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS(JS={junction_current})\nM1 d g s b nfast\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.JS, expected)


@pytest.mark.parametrize("bulk_potential", ["0", "-0.1", "1e999"])
def test_rejects_invalid_mosfet_model_bulk_potential(
    bulk_potential: str,
) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET PB must be finite and positive",
    ):
        parse_netlist(
            f".model nfast NMOS(PB={bulk_potential})\nM1 d g s b nfast\n"
        )


def test_lowers_positive_mosfet_model_bulk_potential() -> None:
    parsed = parse_netlist(".model nfast NMOS(PB=0.72)\nM1 d g s b nfast\n")
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.PB, 0.72)


@pytest.mark.parametrize("grading_coefficient", ["-0.1", "1e999"])
def test_rejects_invalid_mosfet_model_junction_grading(
    grading_coefficient: str,
) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET MJ must be finite and non-negative",
    ):
        parse_netlist(
            f".model nfast NMOS(MJ={grading_coefficient})\nM1 d g s b nfast\n"
        )


@pytest.mark.parametrize("grading_coefficient", ["0", "0.45"])
def test_lowers_valid_mosfet_model_junction_grading(
    grading_coefficient: str,
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS(MJ={grading_coefficient})\nM1 d g s b nfast\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.MJ, float(grading_coefficient))


@pytest.mark.parametrize("grading_coefficient", ["-0.1", "1e999"])
def test_rejects_invalid_mosfet_model_sidewall_grading(
    grading_coefficient: str,
) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET MJSW must be finite and non-negative",
    ):
        parse_netlist(
            f".model nfast NMOS(MJSW={grading_coefficient})\nM1 d g s b nfast\n"
        )


@pytest.mark.parametrize("grading_coefficient", ["0", "0.33"])
def test_lowers_valid_mosfet_model_sidewall_grading(
    grading_coefficient: str,
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS(MJSW={grading_coefficient})\nM1 d g s b nfast\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.MJSW, float(grading_coefficient))


@pytest.mark.parametrize("coefficient", ["-0.1", "1", "1e999"])
def test_rejects_invalid_mosfet_model_forward_bias_coefficient(
    coefficient: str,
) -> None:
    with pytest.raises(
        NetlistParseError,
        match=r"MOSFET FC must be finite and in \[0, 1\)",
    ):
        parse_netlist(f".model nfast NMOS(FC={coefficient})\nM1 d g s b nfast\n")


@pytest.mark.parametrize("coefficient", ["0", "0.5"])
def test_lowers_valid_mosfet_model_forward_bias_coefficient(
    coefficient: str,
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS(FC={coefficient})\nM1 d g s b nfast\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.FC, float(coefficient))


@pytest.mark.parametrize("coefficient", ["-1e-18", "1e999"])
def test_rejects_invalid_mosfet_model_flicker_noise_coefficient(
    coefficient: str,
) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET KF must be finite and non-negative",
    ):
        parse_netlist(f".model nfast NMOS(KF={coefficient})\nM1 d g s b nfast\n")


@pytest.mark.parametrize("coefficient", ["0", "2e-18"])
def test_lowers_valid_mosfet_model_flicker_noise_coefficient(
    coefficient: str,
) -> None:
    parsed = parse_netlist(
        f".model nfast NMOS(KF={coefficient})\nM1 d g s b nfast\n"
    )
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.KF, float(coefficient))


@pytest.mark.parametrize("exponent", ["-0.1", "1e999"])
def test_rejects_invalid_mosfet_model_flicker_noise_exponent(exponent: str) -> None:
    with pytest.raises(
        NetlistParseError,
        match="MOSFET AF must be finite and non-negative",
    ):
        parse_netlist(f".model nfast NMOS(AF={exponent})\nM1 d g s b nfast\n")


@pytest.mark.parametrize("exponent", ["0", "1.5"])
def test_lowers_valid_mosfet_model_flicker_noise_exponent(exponent: str) -> None:
    parsed = parse_netlist(f".model nfast NMOS(AF={exponent})\nM1 d g s b nfast\n")
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(mosfet.model.model.params.AF, float(exponent))


@pytest.mark.parametrize(
    ("alias", "canonical"),
    [("CJS", "CBS"), ("CJD", "CBD")],
)
@pytest.mark.parametrize("capacitance", ["-1p", "1e999"])
def test_rejects_invalid_mosfet_junction_capacitance_aliases(
    alias: str, canonical: str, capacitance: str
) -> None:
    with pytest.raises(
        NetlistParseError,
        match=f"MOSFET {canonical} must be finite and non-negative",
    ):
        parse_netlist(
            f".model nfast NMOS({alias}={capacitance})\nM1 d g s b nfast\n"
        )


@pytest.mark.parametrize(
    ("alias", "canonical"),
    [("CJS", "CBS"), ("CJD", "CBD")],
)
def test_lowers_mosfet_junction_capacitance_aliases(
    alias: str, canonical: str
) -> None:
    parsed = parse_netlist(f".model nfast NMOS({alias}=2p)\nM1 d g s b nfast\n")
    mosfet = parsed.circuit.elements[0]
    assert isinstance(mosfet, Mosfet)
    assert isclose(getattr(mosfet.model.model.params, canonical), 2.0e-12)


@pytest.mark.parametrize("parameter", ["NSS=-1", "NSS=1e999", "TPG=0.5"])
def test_rejects_invalid_mosfet_electrostatic_process_parameters(
    parameter: str,
) -> None:
    message = (
        "MOSFET NSS must be finite and non-negative"
        if parameter.startswith("NSS")
        else "MOSFET TPG must be -1, 0, or 1"
    )
    with pytest.raises(NetlistParseError, match=message):
        parse_netlist(f".model nfast NMOS({parameter})\nM1 d g s b nfast\n")


def test_derives_mosfet_electrostatic_defaults_with_explicit_precedence() -> None:
    derived = parse_netlist(
        ".model nfast NMOS(NSUB=4e15 TOX=100n NSS=1e10 TPG=-1)\n"
        "M1 d g s b nfast\n"
    ).circuit.elements[0]
    explicit = parse_netlist(
        ".model nfast NMOS(NSUB=4e15 TOX=100n NSS=1e10 TPG=-1 "
        "VT0=0.61 GAMMA=0.42 PHI=0.73)\nM1 d g s b nfast\n"
    ).circuit.elements[0]
    assert isinstance(derived, Mosfet)
    assert derived.model.model.params.GAMMA > 0.0
    assert derived.model.model.params.PHI > 0.0
    assert not isclose(derived.model.model.params.VT0, 0.7)
    assert isinstance(explicit, Mosfet)
    assert isclose(explicit.model.model.params.VT0, 0.61)
    assert isclose(explicit.model.model.params.GAMMA, 0.42)
    assert isclose(explicit.model.model.params.PHI, 0.73)


def test_rejects_unbalanced_waveform_parenthesis() -> None:
    with pytest.raises(NetlistParseError, match="unclosed parenthesis"):
        parse_netlist("V1 in 0 PULSE(0 1\n")


def test_rejects_unknown_subcircuit_instance() -> None:
    with pytest.raises(NetlistParseError, match="line 1: unknown subcircuit 'missing'"):
        parse_netlist("X1 a b missing\n")
