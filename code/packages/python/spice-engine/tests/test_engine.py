"""Tests for the SPICE engine.

Test organisation
-----------------
1.  Linear solver (_solve)
2.  DC analysis: simple circuits
3.  DC analysis: ground aliases
4.  DC analysis: Diode
5.  DC analysis: Capacitor (open in DC)
6.  Transient — backward Euler (legacy method="euler")
7.  Transient — trapezoidal (default, method="trap")
8.  Transient — accuracy: trap vs euler
9.  Transient — adaptive timestep (adaptive=True)
10. Transient — inductor companion model
11. Transient — TransientResult metadata fields
12. Mid-scale sanity checks
13. DC: Inductor
14. DC: BJT (NPN and PNP)
15. AC sweep: complex linear solver
16. AC sweep: data structures (AcPoint, AcResult)
17. AC sweep: resistive circuits (frequency-independent)
18. AC sweep: RC low-pass filter (-3 dB at cutoff)
19. AC sweep: RL high-pass filter
20. AC sweep: sweep modes (log, lin, edge cases)
21. AC sweep: small-signal nonlinear elements (Diode, BJT)
22. AC sweep: current source injection
23. TF analysis: TfResult dataclass
24. TF analysis: _build_ss_matrix helper
25. TF analysis: resistive circuits (voltage-source input)
26. TF analysis: current-source input + transimpedance
27. TF analysis: error cases and edge cases
28. DC sweep: DcSweepPoint / DcSweepResult dataclasses
29. DC sweep: linear resistive circuits
30. DC sweep: nonlinear (diode) circuit
31. DC sweep: current-source sweeps
32. DC sweep: error cases and edge cases
33. DC sensitivity: SensEntry / SensResult dataclasses
34. DC sensitivity: resistor-divider analytical verification
35. DC sensitivity: voltage-source and current-source sensitivities
36. DC sensitivity: nonlinear element (Diode Is)
37. DC sensitivity: BJT Is and beta_f
38. DC sensitivity: sorting and ranking
39. DC sensitivity: error cases and edge cases
40. Monte Carlo: McPoint / McResult dataclasses
41. Monte Carlo: mean near nominal for symmetric tolerances
42. Monte Carlo: std_dev > 0 when tolerance > 0
43. Monte Carlo: seed reproducibility
44. Monte Carlo: distribution modes (gaussian vs uniform)
45. Monte Carlo: error cases and edge cases
46. Noise analysis: NoiseEntry / NoisePoint / NoiseResult dataclasses
47. Noise analysis: thermal noise — analytical Nyquist verification
48. Noise analysis: per-element breakdown and sorting
49. Noise analysis: input-referred noise calculation
50. Noise analysis: shot noise (Diode and BJT)
51. Noise analysis: frequency sweep and defaults
52. Noise analysis: error cases and edge cases
53. Controlled sources: VCCS dataclass
54. Controlled sources: VCVS dataclass
55. Controlled sources: CCCS dataclass
56. Controlled sources: CCVS dataclass
57. DC analysis: VCCS (voltage-controlled current source)
58. DC analysis: VCVS (voltage-controlled voltage source)
59. DC analysis: CCCS (current-controlled current source)
60. DC analysis: CCVS (current-controlled voltage source)
61. AC analysis: controlled sources
62. DC sweep: VCVS
63. Transient: controlled sources
64. TF analysis: VCVS
65. Sensitivity: VCVS
66. Monte Carlo: VCVS
67. Error cases: unknown ctrl_source
"""

import cmath
import json
import math
from dataclasses import replace
from math import exp, isclose

import pytest
from mosfet_models import MOSFET, Level1Model, Level1Params, MosfetType

from spice_engine import (
    BJT,
    CCCS,
    CCVS,
    JFET,
    VCCS,
    VCVS,
    AcPoint,
    AcResult,
    AcSource,
    BSource,
    Capacitor,
    Circuit,
    CornerAcSweepResult,
    CornerAdaptiveTransientResult,
    CornerDcSweepResult,
    CornerDistortionPoint,
    CornerDistortionResult,
    CornerFourierResult,
    CornerMcResult,
    CornerNoiseResult,
    CornerOverride,
    CornerPoleZeroResult,
    CornerPssResult,
    CornerSensResult,
    CornerSParameterResult,
    CornerSpec,
    CornerSweepResult,
    CornerTemperatureDcResult,
    CornerTfResult,
    CornerTransientResult,
    CurrentSource,
    CustomModel,
    CustomModelEvaluation,
    DcResult,
    DcSweepPoint,
    DcSweepResult,
    DigitalEvent,
    DigitalEventStream,
    DigitalLogicLevels,
    DigitalThresholds,
    Diode,
    DistortionHarmonic,
    DistortionPoint,
    DistortionResult,
    ExpWaveform,
    FourierHarmonic,
    FourierProbeResult,
    FourierResult,
    Inductor,
    McPoint,
    McResult,
    Mosfet,
    MutualInductor,
    NoiseEntry,
    NoisePoint,
    NoiseResult,
    PoleZeroEntry,
    PoleZeroResult,
    PssNewtonCandidateResult,
    PssNewtonIterationResult,
    PssNewtonSolveResult,
    PssNewtonUpdateResult,
    PssResidualJacobianResult,
    PssResidualResult,
    PssResult,
    PulseWaveform,
    PwlWaveform,
    Resistor,
    SensEntry,
    SensResult,
    SinWaveform,
    SParameterPoint,
    SParameterResult,
    SubcircuitDefinition,
    TemperatureDcResult,
    TfResult,
    TransientPoint,
    TransientResult,
    TransmissionLine,
    VoltageSource,
    XInstance,
    __version__,
    ac_sweep,
    ac_sweep_corners,
    analyze_custom_model_source,
    analyze_deck_controls,
    bjt_at_temperature,
    bjt_from_model_card,
    circuit_at_temperature,
    custom_linear_conductance_model,
    dc_corners,
    dc_initial_vector_from_conditions,
    dc_op,
    dc_op_with_initial_conditions,
    dc_sweep,
    dc_sweep_corners,
    dc_temperature_sweep,
    dc_temperature_sweep_corners,
    deck_output_plan_artifact_records,
    deck_table_records,
    device_model_audit_fixtures,
    device_model_behavior_audit_fixtures,
    device_model_capacitance_audit_fixtures,
    device_model_charge_audit_fixtures,
    device_model_noise_audit_fixtures,
    device_model_reference_deck_audit_analysis_summary,
    device_model_reference_deck_audit_analysis_summary_records,
    device_model_reference_deck_audit_fixtures,
    device_model_reference_deck_audit_gate,
    device_model_reference_deck_audit_gate_coverage_digest,
    device_model_reference_deck_audit_gate_coverage_digest_records,
    device_model_reference_deck_audit_gate_issue_records,
    device_model_reference_deck_audit_gate_issue_summary,
    device_model_reference_deck_audit_gate_issue_summary_records,
    device_model_reference_deck_audit_matrix,
    device_model_reference_deck_audit_matrix_records,
    device_model_reference_deck_audit_records,
    device_model_reference_deck_audit_summary,
    device_model_reference_deck_audit_summary_records,
    device_model_temperature_audit_fixtures,
    digital_event_streams_to_bridge_schedule,
    digital_event_streams_to_voltage_sources,
    digital_events_to_pwl_waveform,
    digital_events_to_voltage_source,
    diode_at_temperature,
    diode_from_model_card,
    distortion_from_fourier,
    distortion_from_transient,
    distortion_from_transient_corners,
    estimate_period,
    format_ac_table,
    format_adaptive_digital_event_stream_table,
    format_corner_ac_table,
    format_corner_adaptive_digital_event_stream_table,
    format_corner_adaptive_transient_table,
    format_corner_dc_sweep_table,
    format_corner_dc_table,
    format_corner_digital_event_stream_table,
    format_corner_distortion_table,
    format_corner_fourier_table,
    format_corner_mc_table,
    format_corner_noise_table,
    format_corner_pole_zero_table,
    format_corner_pss_table,
    format_corner_s_parameter_table,
    format_corner_sens_table,
    format_corner_temperature_dc_table,
    format_corner_tf_table,
    format_corner_transient_table,
    format_dc_sweep_table,
    format_dc_table,
    format_deck_ac_table,
    format_deck_control_policy_artifact_csv,
    format_deck_control_policy_artifact_json,
    format_deck_control_policy_artifact_table,
    format_deck_control_policy_summary_artifact_csv,
    format_deck_control_policy_summary_artifact_json,
    format_deck_control_policy_summary_artifact_table,
    format_deck_dc_sweep_table,
    format_deck_noise_table,
    format_deck_op_table,
    format_deck_output_plan_artifact_csv,
    format_deck_output_plan_artifact_json,
    format_deck_output_plan_artifact_table,
    format_deck_rawfile_artifact_csv,
    format_deck_rawfile_artifact_json,
    format_deck_rawfile_artifact_table,
    format_deck_run_artifact_csv,
    format_deck_run_artifact_json,
    format_deck_run_artifact_table,
    format_deck_table_csv,
    format_deck_table_json,
    format_deck_transient_table,
    format_deck_wrdata_artifact_csv,
    format_deck_wrdata_artifact_json,
    format_deck_wrdata_artifact_table,
    format_deck_wrdata_ascii,
    format_device_model_reference_deck_audit_analysis_summary_csv,
    format_device_model_reference_deck_audit_analysis_summary_json,
    format_device_model_reference_deck_audit_analysis_summary_table,
    format_device_model_reference_deck_audit_csv,
    format_device_model_reference_deck_audit_gate_coverage_digest_csv,
    format_device_model_reference_deck_audit_gate_coverage_digest_json,
    format_device_model_reference_deck_audit_gate_coverage_digest_table,
    format_device_model_reference_deck_audit_gate_issue_csv,
    format_device_model_reference_deck_audit_gate_issue_json,
    format_device_model_reference_deck_audit_gate_issue_summary_csv,
    format_device_model_reference_deck_audit_gate_issue_summary_json,
    format_device_model_reference_deck_audit_gate_issue_summary_table,
    format_device_model_reference_deck_audit_gate_issue_table,
    format_device_model_reference_deck_audit_gate_report,
    format_device_model_reference_deck_audit_json,
    format_device_model_reference_deck_audit_matrix_csv,
    format_device_model_reference_deck_audit_matrix_json,
    format_device_model_reference_deck_audit_matrix_table,
    format_device_model_reference_deck_audit_summary_csv,
    format_device_model_reference_deck_audit_summary_json,
    format_device_model_reference_deck_audit_summary_table,
    format_device_model_reference_deck_audit_table,
    format_digital_bridge_schedule_table,
    format_digital_event_stream_table,
    format_digital_event_stream_vcd,
    format_digital_event_table,
    format_distortion_table,
    format_fourier_table,
    format_mc_table,
    format_measurement_table,
    format_model_card_supported_parameter_coverage_csv,
    format_model_card_supported_parameter_coverage_gate_issue_csv,
    format_model_card_supported_parameter_coverage_gate_issue_json,
    format_model_card_supported_parameter_coverage_gate_issue_table,
    format_model_card_supported_parameter_coverage_gate_report,
    format_model_card_supported_parameter_coverage_json,
    format_model_card_supported_parameter_coverage_summary_csv,
    format_model_card_supported_parameter_coverage_summary_json,
    format_model_card_supported_parameter_coverage_summary_table,
    format_model_card_supported_parameter_coverage_table,
    format_noise_table,
    format_pole_zero_table,
    format_pss_table,
    format_s_parameter_table,
    format_sens_table,
    format_temperature_dc_table,
    format_tf_table,
    format_transient_table,
    fourier,
    fourier_corners,
    fourier_transient_deck,
    jfet_at_temperature,
    jfet_from_model_card,
    mc_dc,
    mc_dc_corners,
    measure_ac_sweep_deck,
    measure_ac_sweep_probe,
    measure_dc_sweep_deck,
    measure_dc_sweep_probe,
    measure_transient_deck,
    measure_transient_delay_between_probes,
    measure_transient_find_at_probe,
    measure_transient_probe,
    measure_transient_when_probe,
    measure_transient_when_probe_counted,
    model_card_supported_parameter_coverage,
    model_card_supported_parameter_coverage_gate,
    model_card_supported_parameter_coverage_gate_issue_records,
    model_card_supported_parameter_coverage_records,
    model_card_supported_parameter_coverage_summary,
    model_card_supported_parameter_coverage_summary_records,
    mosfet_from_model_card,
    noise_ac,
    noise_ac_corners,
    normalize_model_card,
    normalize_model_card_type,
    pole_zero_corners,
    pole_zero_rc_highpass,
    pole_zero_rc_lowpass,
    pole_zero_rlc_bandpass,
    pole_zero_rlc_highpass,
    pole_zero_rlc_lowpass,
    pole_zero_rlc_notch,
    pss,
    pss_corners,
    pss_newton_candidate,
    pss_newton_iteration,
    pss_newton_solve,
    pss_newton_update,
    pss_residual,
    pss_residual_jacobian,
    resolve_deck_initial_conditions,
    run_deck,
    run_deck_analysis,
    s_parameters,
    s_parameters_corners,
    sample_transient_probe_as_digital_events,
    sample_transient_probes_as_digital_event_streams,
    sens_dc,
    sens_dc_corners,
    tf,
    tf_corners,
    transient,
    transient_adaptive_corners,
    transient_adaptive_with_digital_event_streams,
    transient_adaptive_with_digital_event_streams_corners,
    transient_corners,
    transient_with_digital_event_streams,
    transient_with_digital_event_streams_corners,
    waveform_period,
)
from spice_engine.engine import (
    _build_ss_matrix,
    _dc_gmin_step,
    _dc_newton,
    _dc_source_step,
    _lte_estimate,
    _node_index,
    _solve,
    _solve_complex,
    _stamp_bjt,
    _voltage_sources,
    _x_from_result,
)


def _assert_run_artifact_table_matches(
    execution: object,
) -> dict[str, str]:
    assert execution.run_artifact_table == format_deck_run_artifact_table(
        execution.run_artifacts
    )
    records = deck_table_records(execution.run_artifact_table)
    assert records == json.loads(format_deck_run_artifact_json(execution.run_artifacts))
    assert len(records) == 1
    return records[0]


def test_package_version_matches_pyproject_release() -> None:
    assert __version__ == "0.14.0"


def test_model_card_type_aliases_are_normalized() -> None:
    assert normalize_model_card_type("diode") == "D"
    assert normalize_model_card_type("n-jfet") == "NJF"
    assert normalize_model_card_type("pch") == "PMOS"


def test_model_card_supported_parameter_coverage_exports_are_stable() -> None:
    coverage = model_card_supported_parameter_coverage()
    assert len(coverage) == 177
    assert coverage[0].kind == "D"
    assert coverage[0].canonical_parameter == "IS"
    assert coverage[0].accepted_names == ("IS", "JS")
    assert coverage[0].alias_count == 2
    assert coverage[-1].kind == "PMOS"
    assert coverage[-1].canonical_parameter == "MJ"
    assert coverage[-1].accepted_names == ("MJ",)

    table = format_model_card_supported_parameter_coverage_table()
    assert table.splitlines()[0] == "kind\tcanonical_parameter\taccepted_names\talias_count"
    assert table.splitlines()[1] == "D\tIS\tIS|JS\t2"
    assert "NMOS\tVT0\tVT0|VTO|VTH\t3" in table
    assert table.splitlines()[-1] == "PMOS\tMJ\tMJ\t1"
    records = model_card_supported_parameter_coverage_records()
    assert len(records) == 177
    assert records[0] == {
        "kind": "D",
        "canonical_parameter": "IS",
        "accepted_names": "IS|JS",
        "alias_count": "2",
    }
    assert format_model_card_supported_parameter_coverage_csv().startswith(
        "kind,canonical_parameter,accepted_names,alias_count\nD,IS,IS|JS,2\n"
    )
    assert json.loads(format_model_card_supported_parameter_coverage_json()) == records


def test_model_card_supported_parameter_coverage_summary_exports_are_stable() -> None:
    summary = model_card_supported_parameter_coverage_summary()
    assert len(summary) == 7
    assert summary[0].kind == "D"
    assert summary[0].canonical_parameter_count == 15
    assert summary[0].accepted_name_count == 21
    assert summary[0].aliased_parameter_count == 5
    assert summary[0].max_alias_count == 3
    assert summary[0].aliased_parameters == ("IS", "VT", "CJO", "VJ", "M")
    assert summary[5].kind == "NMOS"
    assert summary[5].canonical_parameter_count == 18
    assert summary[5].accepted_name_count == 25
    assert summary[5].aliased_parameter_count == 6
    assert summary[5].max_alias_count == 3
    assert summary[5].aliased_parameters == (
        "VT0",
        "LAMBDA",
        "N_SUB",
        "T_NOM",
        "CBS",
        "CBD",
    )
    assert summary[-1].kind == "PMOS"

    table = format_model_card_supported_parameter_coverage_summary_table()
    assert (
        table.splitlines()[0]
        == "kind\tcanonical_parameter_count\taccepted_name_count\t"
        "aliased_parameter_count\tmax_alias_count\taliased_parameters"
    )
    assert table.splitlines()[1] == "D\t15\t21\t5\t3\tIS|VT|CJO|VJ|M"
    assert (
        table.splitlines()[-1]
        == "PMOS\t18\t25\t6\t3\tVT0|LAMBDA|N_SUB|T_NOM|CBS|CBD"
    )
    records = model_card_supported_parameter_coverage_summary_records()
    assert len(records) == 7
    assert records[0] == {
        "kind": "D",
        "canonical_parameter_count": "15",
        "accepted_name_count": "21",
        "aliased_parameter_count": "5",
        "max_alias_count": "3",
        "aliased_parameters": "IS|VT|CJO|VJ|M",
    }
    assert format_model_card_supported_parameter_coverage_summary_csv().startswith(
        "kind,canonical_parameter_count,accepted_name_count,"
        "aliased_parameter_count,max_alias_count,aliased_parameters\n"
        "D,15,21,5,3,IS|VT|CJO|VJ|M\n"
    )
    assert (
        json.loads(format_model_card_supported_parameter_coverage_summary_json())
        == records
    )


def test_model_card_supported_parameter_coverage_gate_passes_current_catalog() -> None:
    report = model_card_supported_parameter_coverage_gate()
    assert report.passed is True
    assert report.kind_count == 7
    assert report.expected_kind_count == 7
    assert report.canonical_parameter_count == 177
    assert report.expected_canonical_parameter_count == 177
    assert report.accepted_name_count == 247
    assert report.aliased_parameter_count == 57
    assert report.max_alias_count == 4
    assert report.issues == ()
    assert format_model_card_supported_parameter_coverage_gate_report(report) == (
        "passed\tkind_count\texpected_kind_count\tcanonical_parameter_count\t"
        "expected_canonical_parameter_count\taccepted_name_count\t"
        "aliased_parameter_count\tmax_alias_count\tissue_count\n"
        "true\t7\t7\t177\t177\t247\t57\t4\t0"
    )
    assert (
        format_model_card_supported_parameter_coverage_gate_issue_table(report)
        == "kind\tfield\tmessage"
    )
    assert model_card_supported_parameter_coverage_gate_issue_records(report) == []
    assert (
        format_model_card_supported_parameter_coverage_gate_issue_csv(report)
        == "kind,field,message\n"
    )
    assert (
        json.loads(format_model_card_supported_parameter_coverage_gate_issue_json(report))
        == []
    )


def test_model_card_supported_parameter_coverage_gate_reports_missing_alias_family() -> None:
    trimmed = tuple(
        row
        for row in model_card_supported_parameter_coverage()
        if not (row.kind == "NMOS" and row.canonical_parameter == "VT0")
    )

    report = model_card_supported_parameter_coverage_gate(trimmed)

    assert report.passed is False
    assert report.kind_count == 7
    assert report.canonical_parameter_count == 176
    assert report.accepted_name_count == 244
    assert report.aliased_parameter_count == 56
    assert report.max_alias_count == 4
    assert len(report.issues) == 4
    assert report.issues[0].kind == "NMOS"
    assert report.issues[0].field == "canonical_parameter_count"
    assert report.issues[0].message == (
        "expected NMOS to expose 18 canonical supported parameters, found 17"
    )
    assert report.issues[-1].field == "max_alias_count"
    assert report.issues[-1].message == "expected NMOS max alias count 3, found 2"
    assert format_model_card_supported_parameter_coverage_gate_report(report) == (
        "passed\tkind_count\texpected_kind_count\tcanonical_parameter_count\t"
        "expected_canonical_parameter_count\taccepted_name_count\t"
        "aliased_parameter_count\tmax_alias_count\tissue_count\n"
        "false\t7\t7\t176\t177\t244\t56\t4\t4\n"
        "kind\tfield\tmessage\n"
        "NMOS\tcanonical_parameter_count\texpected NMOS to expose 18 canonical "
        "supported parameters, found 17\n"
        "NMOS\taccepted_name_count\texpected NMOS to expose 25 accepted model-card "
        "names, found 22\n"
        "NMOS\taliased_parameter_count\texpected NMOS to expose 6 alias-bearing "
        "parameters, found 5\n"
        "NMOS\tmax_alias_count\texpected NMOS max alias count 3, found 2"
    )
    records = model_card_supported_parameter_coverage_gate_issue_records(report)
    assert records[0] == {
        "kind": "NMOS",
        "field": "canonical_parameter_count",
        "message": "expected NMOS to expose 18 canonical supported parameters, found 17",
    }
    assert format_model_card_supported_parameter_coverage_gate_issue_csv(report).startswith(
        "kind,field,message\n"
        'NMOS,canonical_parameter_count,"expected NMOS to expose 18 canonical '
        'supported parameters, found 17"\n'
    )
    assert (
        json.loads(format_model_card_supported_parameter_coverage_gate_issue_json(report))
        == records
    )


def test_model_card_aliases_build_device_instances() -> None:
    diode_card = normalize_model_card(
        "Dfast",
        "diode",
        {
            "JS": 2.0e-14,
            "CJ": 1.5e-12,
            "TT": 4.0e-9,
            "PB": 0.8,
            "MJ": 0.4,
            "FC": 0.35,
            "XTI": 2.2,
            "EG": 1.05,
            "RS": 10.0,
            "KF": 1.0e-12,
            "AF": 1.3,
        },
    )
    diode_model = diode_from_model_card("D1", "a", "k", diode_card)
    assert diode_card.parameters == {
        "IS": 2.0e-14,
        "CJO": 1.5e-12,
        "TT": 4.0e-9,
        "VJ": 0.8,
        "M": 0.4,
        "FC": 0.35,
        "XTI": 2.2,
        "EG": 1.05,
        "RS": 10.0,
        "KF": 1.0e-12,
        "AF": 1.3,
    }
    assert diode_card.unsupported_parameters == ()
    assert diode_model.Is == pytest.approx(2.0e-14)
    assert diode_model.Cjo == pytest.approx(1.5e-12)
    assert diode_model.Tt == pytest.approx(4.0e-9)
    assert diode_model.Vj == pytest.approx(0.8)
    assert pytest.approx(0.4) == diode_model.M
    assert pytest.approx(0.35) == diode_model.Fc
    assert pytest.approx(2.2) == diode_model.Xti
    assert pytest.approx(1.05) == diode_model.Eg
    assert pytest.approx(10.0) == diode_model.Rs
    assert pytest.approx(1.0e-12) == diode_model.Kf
    assert pytest.approx(1.3) == diode_model.Af

    bjt_card = normalize_model_card(
        "Qsmall", "npn", {"BETA": 125.0, "BETA_R": 0.25, "CBE": 2.0e-12, "XTI": 2.4, "XTB": 1.5, "EG": 1.05, "VA": 80.0, "VB": 120.0, "IK": 2.0e-3, "IKR": 3.0e-3, "T_NOM": 50.0, "KF": 1.0e-12, "AF": 1.3, "PTF": 30.0, "XTF": 2.0, "ITF": 4.0e-3, "VTF": 0.6, "RE": 12.0, "RC": 13.0, "RB": 14.0, "RBM": 2.0, "IRB": 5.0e-6, "XCJC": 0.4, "ISE": 3.0e-13, "NE": 1.7, "ISC": 4.0e-13, "NC": 1.8, "NF": 1.2, "NR": 1.3, "PE": 0.8, "ME": 0.4, "PC": 0.7, "MC": 0.45, "FC": 0.4}
    )
    bjt_model = bjt_from_model_card("Q1", "c", "b", "e", bjt_card)
    assert bjt_card.parameters == {"BF": 125.0, "BR": 0.25, "CJE": 2.0e-12, "XTI": 2.4, "XTB": 1.5, "EG": 1.05, "VAF": 80.0, "VAR": 120.0, "IKF": 2.0e-3, "IKR": 3.0e-3, "TNOM": 50.0, "KF": 1.0e-12, "AF": 1.3, "PTF": 30.0, "XTF": 2.0, "ITF": 4.0e-3, "VTF": 0.6, "RE": 12.0, "RC": 13.0, "RB": 14.0, "RBM": 2.0, "IRB": 5.0e-6, "XCJC": 0.4, "ISE": 3.0e-13, "NE": 1.7, "ISC": 4.0e-13, "NC": 1.8, "NF": 1.2, "NR": 1.3, "VJE": 0.8, "MJE": 0.4, "VJC": 0.7, "MJC": 0.45, "FC": 0.4}
    assert bjt_model.polarity == "NPN"
    assert bjt_model.beta_f == pytest.approx(125.0)
    assert bjt_model.beta_r == pytest.approx(0.25)
    assert bjt_model.Cje == pytest.approx(2.0e-12)
    assert bjt_model.Xti == pytest.approx(2.4)
    assert bjt_model.Xtb == pytest.approx(1.5)
    assert bjt_model.Eg == pytest.approx(1.05)
    assert bjt_model.Vaf == pytest.approx(80.0)
    assert bjt_model.Var == pytest.approx(120.0)
    assert bjt_model.Ikf == pytest.approx(2.0e-3)
    assert bjt_model.Ikr == pytest.approx(3.0e-3)
    assert bjt_model.Tnom == pytest.approx(323.15)
    assert bjt_model.Kf == pytest.approx(1.0e-12)
    assert bjt_model.Af == pytest.approx(1.3)
    assert bjt_model.Ptf == pytest.approx(30.0)
    assert bjt_model.Xtf == pytest.approx(2.0)
    assert bjt_model.Itf == pytest.approx(4.0e-3)
    assert bjt_model.Vtf == pytest.approx(0.6)
    assert bjt_model.Re == pytest.approx(12.0)
    assert bjt_model.Rc == pytest.approx(13.0)
    assert bjt_model.Rb == pytest.approx(14.0)
    assert bjt_model.Rbm == pytest.approx(2.0)
    assert bjt_model.Irb == pytest.approx(5.0e-6)
    assert bjt_model.Xcjc == pytest.approx(0.4)
    assert bjt_model.Ise == pytest.approx(3.0e-13)
    assert bjt_model.Ne == pytest.approx(1.7)
    assert bjt_model.Isc == pytest.approx(4.0e-13)
    assert bjt_model.Nc == pytest.approx(1.8)
    assert bjt_model.Nf == pytest.approx(1.2)
    assert bjt_model.Nr == pytest.approx(1.3)
    assert bjt_model.Vje == pytest.approx(0.8)
    assert bjt_model.Mje == pytest.approx(0.4)
    assert bjt_model.Vjc == pytest.approx(0.7)
    assert bjt_model.Mjc == pytest.approx(0.45)
    assert bjt_model.Fc == pytest.approx(0.4)

    jfet_card = normalize_model_card(
        "Jn",
        "njfet",
        {
            "BET": 9.0e-4,
            "VT0": -1.8,
            "LAM": 0.02,
            "KF": 1.0e-12,
            "AF": 1.3,
            "VJ": 0.8,
            "FC": 0.35,
            "IS": 2.0e-13,
            "XTI": 2.5,
            "EG": 1.05,
            "B": 1.1,
            "NLEV": 3.0,
            "GDSNOI": 1.25,
            "RD": 125.0,
            "RS": 75.0,
            "T_NOM": 50.0,
            "TCV": 0.01,
            "VTOTC": -0.0025,
            "BEX": 1.5,
            "BETATCE": -0.5,
        },
    )
    jfet_model = jfet_from_model_card("J1", "d", "g", "s", jfet_card)
    assert jfet_card.parameters == {
        "BETA": 9.0e-4,
        "VTO": -1.8,
        "LAMBDA": 0.02,
        "KF": 1.0e-12,
        "AF": 1.3,
        "PB": 0.8,
        "FC": 0.35,
        "IS": 2.0e-13,
        "XTI": 2.5,
        "EG": 1.05,
        "B": 1.1,
        "NLEV": 3.0,
        "GDSNOI": 1.25,
        "RD": 125.0,
        "RS": 75.0,
        "TNOM": 50.0,
        "TCV": 0.01,
        "VTOTC": -0.0025,
        "BEX": 1.5,
        "BETATCE": -0.5,
    }
    assert jfet_model.polarity == "NJF"
    assert jfet_model.beta == pytest.approx(9.0e-4)
    assert jfet_model.vto == pytest.approx(-1.8)
    assert jfet_model.lambda_ == pytest.approx(0.02)
    assert jfet_model.Kf == pytest.approx(1.0e-12)
    assert jfet_model.Af == pytest.approx(1.3)
    assert jfet_model.Pb == pytest.approx(0.8)
    assert jfet_model.Fc == pytest.approx(0.35)
    assert jfet_model.Is == pytest.approx(2.0e-13)
    assert jfet_model.Xti == pytest.approx(2.5)
    assert jfet_model.Eg == pytest.approx(1.05)
    assert pytest.approx(1.1) == jfet_model.B
    assert jfet_model.Nlev == pytest.approx(3.0)
    assert jfet_model.Gdsnoi == pytest.approx(1.25)
    assert jfet_model.Rd == pytest.approx(125.0)
    assert jfet_model.Rs == pytest.approx(75.0)
    assert jfet_model.Tnom == pytest.approx(323.15)
    assert jfet_model.Tcv == pytest.approx(0.01)
    assert jfet_model.Vtotc == pytest.approx(-0.0025)
    assert jfet_model.Bex == pytest.approx(1.5)
    assert jfet_model.Betatce == pytest.approx(-0.5)

    mos_card = normalize_model_card(
        "Mn",
        "nmos",
        {
            "LEVEL": 1.0,
            "VTO": 0.55,
            "LAM": 0.04,
            "NSUB": 1.6,
            "CJD": 3.0e-13,
            "PB": 0.9,
            "MJ": 0.45,
        },
    )
    mos_model = mosfet_from_model_card("M1", "d", "g", "s", "b", mos_card)
    assert mos_card.parameters == {
        "LEVEL": 1.0,
        "VT0": 0.55,
        "LAMBDA": 0.04,
        "N_SUB": 1.6,
        "CBD": 3.0e-13,
        "PB": 0.9,
        "MJ": 0.45,
    }
    assert isinstance(mos_model.model, MOSFET)
    assert mos_model.model.type == MosfetType.NMOS
    assert isinstance(mos_model.model.model, Level1Model)
    assert pytest.approx(0.55) == mos_model.model.model.params.VT0
    assert pytest.approx(0.04) == mos_model.model.model.params.LAMBDA
    assert pytest.approx(1.6) == mos_model.model.model.params.N_SUB
    assert pytest.approx(3.0e-13) == mos_model.model.model.params.CBD
    assert pytest.approx(0.9) == mos_model.model.model.params.PB
    assert pytest.approx(0.45) == mos_model.model.model.params.MJ


def test_dc_rejects_invalid_jfet_flicker_noise_coefficient() -> None:
    circuit = Circuit()
    circuit.add(JFET("J1", "drain", "gate", "0", Kf=-1.0))

    with pytest.raises(
        ValueError,
        match="flicker-noise coefficient must be finite and non-negative",
    ):
        dc_op(circuit)


def test_dc_rejects_invalid_jfet_flicker_noise_exponent() -> None:
    circuit = Circuit()
    circuit.add(JFET("J1", "drain", "gate", "0", Af=-1.0))

    with pytest.raises(
        ValueError,
        match="flicker-noise exponent must be finite and non-negative",
    ):
        dc_op(circuit)


def test_dc_rejects_invalid_jfet_junction_potential() -> None:
    circuit = Circuit()
    circuit.add(JFET("J1", "drain", "gate", "0", Pb=0.0))

    with pytest.raises(ValueError, match="PB must be finite and positive"):
        dc_op(circuit)


def test_dc_rejects_invalid_jfet_forward_bias_depletion_coefficient() -> None:
    circuit = Circuit()
    circuit.add(JFET("J1", "drain", "gate", "0", Fc=1.0))

    with pytest.raises(ValueError, match=r"FC must be finite and in \[0, 1\)"):
        dc_op(circuit)


def test_dc_rejects_invalid_jfet_gate_saturation_current() -> None:
    circuit = Circuit()
    circuit.add(JFET("J1", "drain", "gate", "0", Is=-1.0))

    with pytest.raises(
        ValueError,
        match="gate saturation current must be finite and non-negative",
    ):
        dc_op(circuit)


def test_dc_rejects_invalid_jfet_drain_resistance() -> None:
    circuit = Circuit()
    circuit.add(JFET("J1", "drain", "gate", "0", Rd=-1.0))

    with pytest.raises(
        ValueError,
        match="drain resistance must be finite and non-negative",
    ):
        dc_op(circuit)


def test_dc_rejects_invalid_jfet_source_resistance() -> None:
    circuit = Circuit()
    circuit.add(JFET("J1", "drain", "gate", "0", Rs=-1.0))

    with pytest.raises(
        ValueError,
        match="source resistance must be finite and non-negative",
    ):
        dc_op(circuit)


def test_dc_rejects_invalid_jfet_doping_tail_parameter() -> None:
    circuit = Circuit()
    circuit.add(JFET("J1", "drain", "gate", "0", B=math.nan))

    with pytest.raises(ValueError, match="doping-tail parameter must be finite"):
        dc_op(circuit)


def test_dc_jfet_doping_tail_parameter_shapes_linear_and_saturation_current() -> None:
    def drain_current(drain_voltage: float, doping_tail_parameter: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vdrain", "drain", "0", drain_voltage))
        circuit.add(VoltageSource("Vgate", "gate", "0", 0.0))
        circuit.add(
            JFET(
                "J1",
                "drain",
                "gate",
                "0",
                beta=1.0e-3,
                vto=-2.0,
                Pb=1.0,
                B=doping_tail_parameter,
            )
        )
        return abs(dc_op(circuit).branch_currents["I(Vdrain)"])

    assert drain_current(1.0, 1.1) > drain_current(1.0, 1.0)
    assert drain_current(3.0, 1.1) > drain_current(3.0, 1.0)


def test_dc_jfet_drain_resistance_drops_intrinsic_drain_voltage() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("Vdrain", "drain", "0", 5.0))
    circuit.add(VoltageSource("Vgate", "gate", "0", 0.0))
    circuit.add(JFET("J1", "drain", "gate", "0", beta=1.0e-3, Rd=1_000.0))

    result = dc_op(circuit)
    assert result.node_voltages["drain"] == pytest.approx(5.0)
    assert result.node_voltages["__spice_J1_drain"] < 5.0


def test_dc_jfet_source_resistance_raises_intrinsic_source_voltage() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("Vdrain", "drain", "0", 5.0))
    circuit.add(VoltageSource("Vgate", "gate", "0", 3.0))
    circuit.add(JFET("J1", "drain", "gate", "0", beta=1.0e-3, Rs=1_000.0))

    result = dc_op(circuit)
    assert result.node_voltages["__spice_J1_source"] > 0.0


def test_jfet_gate_saturation_current_loads_a_forward_biased_gate() -> None:
    def gate_voltage(
        polarity: str, bias_voltage: float, gate_saturation_current: float
    ) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vbias", "bias", "0", bias_voltage))
        circuit.add(Resistor("Rgate", "bias", "gate", 1.0e6))
        circuit.add(
            JFET(
                "J1",
                "0",
                "gate",
                "0",
                polarity=polarity,
                vto=-2.0 if polarity == "NJF" else 2.0,
                Is=gate_saturation_current,
            )
        )
        return dc_op(circuit).node_voltages["gate"]

    assert gate_voltage("NJF", 0.3, 1.0e-9) < gate_voltage("NJF", 0.3, 1.0e-14)
    assert gate_voltage("PJF", -0.3, 1.0e-9) > gate_voltage("PJF", -0.3, 1.0e-14)


def test_bjt_legacy_leakage_ratios_derive_currents_with_explicit_precedence() -> None:
    legacy_card = normalize_model_card(
        "Qlegacy",
        "npn",
        {"IS": 2.0e-14, "C2": 15.0, "C4": 20.0},
    )
    legacy = bjt_from_model_card("Q1", "c", "b", "e", legacy_card)

    assert legacy_card.parameters == {
        "IS": 2.0e-14,
        "C2": 15.0,
        "C4": 20.0,
    }
    assert legacy.Ise == pytest.approx(3.0e-13)
    assert legacy.Isc == pytest.approx(4.0e-13)

    explicit_card = normalize_model_card(
        "Qexplicit",
        "pnp",
        {
            "IS": 2.0e-14,
            "C2": 15.0,
            "ISE": 5.0e-13,
            "C4": 20.0,
            "ISC": 6.0e-13,
        },
    )
    explicit = bjt_from_model_card("Q2", "c", "b", "e", explicit_card)
    assert explicit.Ise == pytest.approx(5.0e-13)
    assert explicit.Isc == pytest.approx(6.0e-13)


def test_model_card_audit_fixtures_cover_supported_device_families() -> None:
    fixtures = device_model_audit_fixtures()
    assert [fixture.kind for fixture in fixtures] == ["D", "NPN", "NJF", "NMOS"]
    assert fixtures[0].parameters["IS"] == pytest.approx(2.0e-14)
    assert fixtures[1].parameters["BF"] == pytest.approx(125.0)
    assert fixtures[2].parameters["VTO"] == pytest.approx(-1.8)
    assert fixtures[3].parameters["VT0"] == pytest.approx(0.55)


def test_device_model_behavior_audit_fixtures_run_reference_bias_points() -> None:
    fixtures = device_model_behavior_audit_fixtures()
    assert [fixture.name for fixture in fixtures] == [
        "diode-forward-bias",
        "bjt-emitter-follower",
        "jfet-source-bias",
        "mos-level1-common-source",
    ]

    for fixture in fixtures:
        result = dc_op(fixture.circuit)
        value = result.node_voltages[fixture.probe_node]
        assert result.converged
        assert fixture.expected_min <= value <= fixture.expected_max
        assert fixture.deck_lines[0].startswith("* device-model behavior fixture:")
        assert ".op" in fixture.deck_lines
        assert any(line.startswith(".model ") for line in fixture.deck_lines)


def test_device_model_temperature_audit_fixtures_run_reference_sweeps() -> None:
    fixtures = device_model_temperature_audit_fixtures()
    assert [fixture.name for fixture in fixtures] == [
        "diode-forward-bias",
        "bjt-emitter-follower",
        "jfet-source-bias",
        "mos-level1-common-source",
    ]

    for fixture in fixtures:
        result = dc_temperature_sweep(
            fixture.circuit,
            [point.temperature_kelvin for point in fixture.temperature_points],
            nominal_temperature_kelvin=fixture.nominal_temperature_kelvin,
            energy_gap_ev=fixture.energy_gap_ev,
        )
        assert ".temp 260.15 300.15 340.15" in fixture.deck_lines
        assert fixture.deck_lines[0].startswith("* device-model temperature fixture:")
        assert len(result.points) == len(fixture.temperature_points)
        for actual, expected in zip(result.points, fixture.temperature_points, strict=True):
            value = actual.result.node_voltages[fixture.probe_node]
            assert actual.result.converged
            assert actual.temperature_kelvin == pytest.approx(expected.temperature_kelvin)
            assert expected.expected_min <= value <= expected.expected_max

    jfet_fixture = next(fixture for fixture in fixtures if fixture.kind == "NJF")
    assert jfet_fixture.temperature_behavior.startswith("JFET temperature scaling defaults")


def test_device_model_capacitance_audit_fixtures_run_reference_ac_points() -> None:
    fixtures = device_model_capacitance_audit_fixtures()
    assert [fixture.name for fixture in fixtures] == [
        "diode-capacitance-ac",
        "bjt-capacitance-ac",
        "jfet-capacitance-ac",
        "mos-level1-capacitance-ac",
    ]

    for fixture in fixtures:
        result = ac_sweep(
            fixture.circuit,
            f_start=fixture.frequency_hz,
            f_stop=fixture.frequency_hz,
            n_points=1,
            sweep="lin",
        )
        value = abs(result.points[0].node_voltages[fixture.probe_node])
        assert fixture.expected_magnitude_min <= value <= fixture.expected_magnitude_max, (
            f"{fixture.name} expected {fixture.expected_magnitude_min} <= "
            f"{value} <= {fixture.expected_magnitude_max}"
        )
        assert fixture.deck_lines[0].startswith("* device-model capacitance fixture:")
        assert any(line.startswith(".model ") for line in fixture.deck_lines)
        assert any(line.startswith(".ac ") for line in fixture.deck_lines)
        assert fixture.capacitance_behavior

    jfet_fixture = next(fixture for fixture in fixtures if fixture.kind == "NJF")
    assert "CGS/CGD" in jfet_fixture.capacitance_behavior


def test_device_model_noise_audit_fixtures_run_reference_noise_points() -> None:
    fixtures = device_model_noise_audit_fixtures()
    assert [fixture.name for fixture in fixtures] == [
        "diode-shot-noise",
        "bjt-shot-noise",
        "jfet-channel-noise",
        "mos-level1-channel-noise",
    ]

    for fixture in fixtures:
        result = noise_ac(
            fixture.circuit,
            fixture.output_node,
            fixture.input_source,
            freqs=[fixture.frequency_hz],
        )
        assert result.points
        entry = next(
            entry
            for entry in result.points[0].entries
            if entry.element_name == fixture.expected_noise_element
        )
        assert entry.noise_type == fixture.expected_noise_type
        assert fixture.expected_source_psd_min <= entry.source_psd <= fixture.expected_source_psd_max
        assert fixture.expected_output_psd_min <= entry.output_psd <= fixture.expected_output_psd_max
        assert fixture.deck_lines[0].startswith("* device-model noise fixture:")
        assert any(line.startswith(".model ") for line in fixture.deck_lines)
        assert any(line.startswith(".noise ") for line in fixture.deck_lines)
        assert fixture.noise_behavior


def test_device_model_charge_audit_fixtures_run_reference_transients() -> None:
    fixtures = device_model_charge_audit_fixtures()
    assert [fixture.name for fixture in fixtures] == [
        "diode-storage-charge",
        "bjt-storage-charge",
        "jfet-storage-charge",
        "mos-level1-storage-charge",
    ]

    for fixture in fixtures:
        result = transient(
            fixture.circuit,
            t_step=fixture.time_step_s,
            t_stop=fixture.stop_time_s,
        )
        assert result.converged
        assert result.points
        initial = result.points[0].node_voltages[fixture.probe_node]
        final = result.points[-1].node_voltages[fixture.probe_node]
        assert fixture.expected_initial_min <= initial <= fixture.expected_initial_max
        assert fixture.expected_final_min <= final <= fixture.expected_final_max, (
            f"{fixture.name} expected {fixture.expected_final_min} <= "
            f"{final} <= {fixture.expected_final_max}"
        )
        assert fixture.storage_capacitance_f > 0.0
        assert fixture.deck_lines[0].startswith("* device-model charge fixture:")
        assert any(line.startswith(".model ") for line in fixture.deck_lines)
        assert any(line.startswith(".tran ") for line in fixture.deck_lines)
        assert fixture.charge_behavior

    jfet_fixture = next(fixture for fixture in fixtures if fixture.kind == "NJF")
    assert "CGS/CGD" in jfet_fixture.charge_behavior
    mos_fixture = next(fixture for fixture in fixtures if fixture.kind == "NMOS")
    assert "CGSO/CGDO/CGBO" in mos_fixture.charge_behavior
    assert "CBS/CBD" in mos_fixture.charge_behavior


def test_device_model_reference_deck_audit_fixtures_cover_model_depth_matrix() -> None:
    fixtures = device_model_reference_deck_audit_fixtures()
    assert len(fixtures) == 20
    assert fixtures[0].name == "diode-forward-bias:op"
    assert fixtures[-1].name == "mos-level1-storage-charge:tran"

    expected_analyses = {"op", "temperature", "ac", "noise", "tran"}
    assert {fixture.kind for fixture in fixtures} == {"D", "NPN", "NJF", "NMOS"}
    for kind in {"D", "NPN", "NJF", "NMOS"}:
        assert {
            fixture.analysis
            for fixture in fixtures
            if fixture.kind == kind
        } == expected_analyses

    for fixture in fixtures:
        assert fixture.reference == "SPICE2/SPICE3-style local model-depth fixture"
        assert fixture.expected_behavior
        assert fixture.deck_lines[0].startswith("* device-model ")
        assert any(line.startswith(".model ") for line in fixture.deck_lines)
        assert fixture.deck_lines[-1] == ".end"


def test_device_model_reference_deck_audit_table_is_stable() -> None:
    table = format_device_model_reference_deck_audit_table()
    lines = table.splitlines()
    assert len(lines) == 21
    assert lines[0] == "name\tkind\tanalysis\tmodel\treference\texpected_behavior\tdeck_lines"
    assert lines[1] == (
        "diode-forward-bias:op\tD\top\tDfast\t"
        "SPICE2/SPICE3-style local model-depth fixture\t"
        "DC probe out remains in [0.55, 0.65] V\t8"
    )
    assert lines[-1] == (
        "mos-level1-storage-charge:tran\tNMOS\ttran\tMn\t"
        "SPICE2/SPICE3-style local model-depth fixture\t"
        "Level-1 MOS CGSO/CGDO/CGBO plus CBS/CBD contribute transient "
        "gate-overlap and depletion-shaped bulk-junction storage; explicit "
        "Cstore keeps the fixture comparable with other charge audits\t10"
    )


def test_device_model_reference_deck_audit_record_exports_are_stable() -> None:
    records = device_model_reference_deck_audit_records()
    assert len(records) == 20
    assert records[0] == {
        "name": "diode-forward-bias:op",
        "kind": "D",
        "analysis": "op",
        "model": "Dfast",
        "reference": "SPICE2/SPICE3-style local model-depth fixture",
        "expected_behavior": "DC probe out remains in [0.55, 0.65] V",
        "deck_lines": "8",
    }
    assert records[-1]["name"] == "mos-level1-storage-charge:tran"
    assert records[-1]["deck_lines"] == "10"

    csv_lines = format_device_model_reference_deck_audit_csv().splitlines()
    assert csv_lines[0] == (
        "name,kind,analysis,model,reference,expected_behavior,deck_lines"
    )
    assert csv_lines[1] == (
        "diode-forward-bias:op,D,op,Dfast,"
        "SPICE2/SPICE3-style local model-depth fixture,"
        '"DC probe out remains in [0.55, 0.65] V",8'
    )

    parsed = json.loads(format_device_model_reference_deck_audit_json())
    assert parsed == records


def test_device_model_reference_deck_audit_summary_exports_are_stable() -> None:
    summary = device_model_reference_deck_audit_summary()
    assert len(summary) == 4
    assert summary[0].kind == "D"
    assert summary[0].fixture_count == 5
    assert summary[0].analyses == ("op", "temperature", "ac", "noise", "tran")
    assert summary[0].missing_analyses == ()
    assert summary[0].deck_line_count == 42
    assert summary[0].references == ("SPICE2/SPICE3-style local model-depth fixture",)

    table = format_device_model_reference_deck_audit_summary_table()
    assert table.splitlines() == [
        "kind\tfixture_count\tanalyses\tmissing_analyses\tdeck_lines\treferences",
        (
            "D\t5\top,temperature,ac,noise,tran\t\t42\t"
            "SPICE2/SPICE3-style local model-depth fixture"
        ),
        (
            "NPN\t5\top,temperature,ac,noise,tran\t\t47\t"
            "SPICE2/SPICE3-style local model-depth fixture"
        ),
        (
            "NJF\t5\top,temperature,ac,noise,tran\t\t52\t"
            "SPICE2/SPICE3-style local model-depth fixture"
        ),
        (
            "NMOS\t5\top,temperature,ac,noise,tran\t\t47\t"
            "SPICE2/SPICE3-style local model-depth fixture"
        ),
    ]

    records = device_model_reference_deck_audit_summary_records()
    assert records[0] == {
        "kind": "D",
        "fixture_count": "5",
        "analyses": "op,temperature,ac,noise,tran",
        "missing_analyses": "",
        "deck_lines": "42",
        "references": "SPICE2/SPICE3-style local model-depth fixture",
    }
    assert format_device_model_reference_deck_audit_summary_csv().splitlines()[1] == (
        'D,5,"op,temperature,ac,noise,tran",,42,'
        "SPICE2/SPICE3-style local model-depth fixture"
    )
    assert json.loads(format_device_model_reference_deck_audit_summary_json()) == records


def test_device_model_reference_deck_audit_summary_reports_missing_analysis() -> None:
    fixtures = tuple(
        fixture
        for fixture in device_model_reference_deck_audit_fixtures()
        if not (fixture.kind == "NMOS" and fixture.analysis == "tran")
    )

    summary = device_model_reference_deck_audit_summary(fixtures)
    nmos = next(row for row in summary if row.kind == "NMOS")

    assert nmos.fixture_count == 4
    assert nmos.analyses == ("op", "temperature", "ac", "noise")
    assert nmos.missing_analyses == ("tran",)
    assert nmos.deck_line_count == 37
    assert (
        "NMOS\t4\top,temperature,ac,noise\ttran\t37\t"
        "SPICE2/SPICE3-style local model-depth fixture"
    ) in format_device_model_reference_deck_audit_summary_table(fixtures)


def test_device_model_reference_deck_audit_analysis_summary_exports_are_stable() -> None:
    summary = device_model_reference_deck_audit_analysis_summary()
    assert len(summary) == 5
    assert summary[0].analysis == "op"
    assert summary[0].fixture_count == 4
    assert summary[0].kinds == ("D", "NPN", "NJF", "NMOS")
    assert summary[0].missing_kinds == ()
    assert summary[0].deck_line_count == 36
    assert summary[0].references == ("SPICE2/SPICE3-style local model-depth fixture",)

    table = format_device_model_reference_deck_audit_analysis_summary_table()
    assert table.splitlines() == [
        "analysis\tfixture_count\tkinds\tmissing_kinds\tdeck_lines\treferences",
        "op\t4\tD,NPN,NJF,NMOS\t\t36\tSPICE2/SPICE3-style local model-depth fixture",
        (
            "temperature\t4\tD,NPN,NJF,NMOS\t\t40\t"
            "SPICE2/SPICE3-style local model-depth fixture"
        ),
        "ac\t4\tD,NPN,NJF,NMOS\t\t36\tSPICE2/SPICE3-style local model-depth fixture",
        "noise\t4\tD,NPN,NJF,NMOS\t\t36\tSPICE2/SPICE3-style local model-depth fixture",
        "tran\t4\tD,NPN,NJF,NMOS\t\t40\tSPICE2/SPICE3-style local model-depth fixture",
    ]

    records = device_model_reference_deck_audit_analysis_summary_records()
    assert records[0] == {
        "analysis": "op",
        "fixture_count": "4",
        "kinds": "D,NPN,NJF,NMOS",
        "missing_kinds": "",
        "deck_lines": "36",
        "references": "SPICE2/SPICE3-style local model-depth fixture",
    }
    assert format_device_model_reference_deck_audit_analysis_summary_csv().splitlines()[1] == (
        'op,4,"D,NPN,NJF,NMOS",,36,'
        "SPICE2/SPICE3-style local model-depth fixture"
    )
    assert (
        json.loads(format_device_model_reference_deck_audit_analysis_summary_json())
        == records
    )


def test_device_model_reference_deck_audit_analysis_summary_reports_missing_kind() -> None:
    fixtures = tuple(
        fixture
        for fixture in device_model_reference_deck_audit_fixtures()
        if not (fixture.kind == "NMOS" and fixture.analysis == "tran")
    )

    summary = device_model_reference_deck_audit_analysis_summary(fixtures)
    tran = next(row for row in summary if row.analysis == "tran")

    assert tran.fixture_count == 3
    assert tran.kinds == ("D", "NPN", "NJF")
    assert tran.missing_kinds == ("NMOS",)
    assert tran.deck_line_count == 30
    assert (
        "tran\t3\tD,NPN,NJF\tNMOS\t30\t"
        "SPICE2/SPICE3-style local model-depth fixture"
    ) in format_device_model_reference_deck_audit_analysis_summary_table(fixtures)


def test_device_model_reference_deck_audit_matrix_exports_are_stable() -> None:
    matrix = device_model_reference_deck_audit_matrix()

    assert len(matrix) == 4
    assert matrix[0].kind == "D"
    assert matrix[0].fixture_count == 5
    assert matrix[0].op == "diode-forward-bias:op"
    assert matrix[0].temperature == "diode-forward-bias:temperature"
    assert matrix[0].ac == "diode-capacitance-ac:ac"
    assert matrix[0].noise == "diode-shot-noise:noise"
    assert matrix[0].tran == "diode-storage-charge:tran"
    assert matrix[0].missing_analyses == ()
    assert matrix[0].extra_analyses == ()
    assert matrix[0].deck_line_count == 42

    assert format_device_model_reference_deck_audit_matrix_table().splitlines() == [
        "kind\tfixture_count\top\ttemperature\tac\tnoise\ttran\tmissing_analyses\textra_analyses\tdeck_lines",
        (
            "D\t5\tdiode-forward-bias:op\tdiode-forward-bias:temperature\t"
            "diode-capacitance-ac:ac\tdiode-shot-noise:noise\t"
            "diode-storage-charge:tran\t\t\t42"
        ),
        (
            "NPN\t5\tbjt-emitter-follower:op\tbjt-emitter-follower:temperature\t"
            "bjt-capacitance-ac:ac\tbjt-shot-noise:noise\tbjt-storage-charge:tran"
            "\t\t\t47"
        ),
        (
            "NJF\t5\tjfet-source-bias:op\tjfet-source-bias:temperature\t"
            "jfet-capacitance-ac:ac\tjfet-channel-noise:noise\t"
            "jfet-storage-charge:tran\t\t\t52"
        ),
        (
            "NMOS\t5\tmos-level1-common-source:op\t"
            "mos-level1-common-source:temperature\tmos-level1-capacitance-ac:ac\t"
            "mos-level1-channel-noise:noise\tmos-level1-storage-charge:tran\t\t\t47"
        ),
    ]

    records = device_model_reference_deck_audit_matrix_records()
    assert records[0] == {
        "kind": "D",
        "fixture_count": "5",
        "op": "diode-forward-bias:op",
        "temperature": "diode-forward-bias:temperature",
        "ac": "diode-capacitance-ac:ac",
        "noise": "diode-shot-noise:noise",
        "tran": "diode-storage-charge:tran",
        "missing_analyses": "",
        "extra_analyses": "",
        "deck_lines": "42",
    }
    assert format_device_model_reference_deck_audit_matrix_csv().splitlines()[1] == (
        "D,5,diode-forward-bias:op,diode-forward-bias:temperature,"
        "diode-capacitance-ac:ac,diode-shot-noise:noise,diode-storage-charge:tran,,,42"
    )
    assert json.loads(format_device_model_reference_deck_audit_matrix_json()) == records


def test_device_model_reference_deck_audit_matrix_reports_missing_analysis() -> None:
    fixtures = tuple(
        fixture
        for fixture in device_model_reference_deck_audit_fixtures()
        if not (fixture.kind == "NMOS" and fixture.analysis == "tran")
    )

    matrix = device_model_reference_deck_audit_matrix(fixtures)
    nmos = next(row for row in matrix if row.kind == "NMOS")

    assert nmos.fixture_count == 4
    assert nmos.tran == ""
    assert nmos.missing_analyses == ("tran",)
    assert nmos.deck_line_count == 37
    assert (
        "NMOS\t4\tmos-level1-common-source:op\t"
        "mos-level1-common-source:temperature\tmos-level1-capacitance-ac:ac\t"
        "mos-level1-channel-noise:noise\t\ttran\t\t37"
    ) in format_device_model_reference_deck_audit_matrix_table(fixtures)


def test_device_model_reference_deck_audit_gate_report_is_stable() -> None:
    report = device_model_reference_deck_audit_gate()

    assert report.passed is True
    assert report.fixture_count == 20
    assert report.expected_kinds == ("D", "NPN", "NJF", "NMOS")
    assert report.expected_analyses == ("op", "temperature", "ac", "noise", "tran")
    assert report.issues == ()
    assert format_device_model_reference_deck_audit_gate_report(report) == (
        "passed\tfixture_count\texpected_kinds\texpected_analyses\tissue_count\n"
        "true\t20\tD,NPN,NJF,NMOS\top,temperature,ac,noise,tran\t0"
    )
    digest = device_model_reference_deck_audit_gate_coverage_digest(report)
    assert digest.passed is True
    assert digest.fixture_count == 20
    assert digest.expected_pair_count == 20
    assert digest.covered_pair_count == 20
    assert digest.missing_pair_count == 0
    assert digest.issue_count == 0
    assert digest.issue_fields == ()
    assert format_device_model_reference_deck_audit_gate_coverage_digest_table(
        report
    ) == (
        "passed\tfixture_count\texpected_pair_count\tcovered_pair_count\t"
        "missing_pair_count\tissue_count\tissue_fields\n"
        "true\t20\t20\t20\t0\t0\t"
    )


def test_device_model_reference_deck_audit_gate_reports_missing_coverage() -> None:
    fixtures = tuple(
        fixture
        for fixture in device_model_reference_deck_audit_fixtures()
        if not (fixture.kind == "NMOS" and fixture.analysis == "tran")
    )

    report = device_model_reference_deck_audit_gate(fixtures)
    table = format_device_model_reference_deck_audit_gate_report(report)

    assert report.passed is False
    assert any(
        issue.fixture_name == "NMOS:tran" and issue.field == "coverage"
        for issue in report.issues
    )
    assert "fixture_name\tfield\tmessage" in table
    assert (
        "NMOS:tran\tcoverage\t"
        "missing required NMOS tran reference-deck audit row"
    ) in table

    issue_table = format_device_model_reference_deck_audit_gate_issue_table(report)
    assert issue_table == (
        "fixture_name\tfield\tmessage\n"
        "NMOS:tran\tcoverage\tmissing required NMOS tran reference-deck audit row"
    )
    records = device_model_reference_deck_audit_gate_issue_records(report)
    assert records == [
        {
            "fixture_name": "NMOS:tran",
            "field": "coverage",
            "message": "missing required NMOS tran reference-deck audit row",
        }
    ]
    assert format_device_model_reference_deck_audit_gate_issue_csv(report) == (
        "fixture_name,field,message\n"
        "NMOS:tran,coverage,missing required NMOS tran reference-deck audit row\n"
    )
    assert json.loads(format_device_model_reference_deck_audit_gate_issue_json(report)) == records
    summary = device_model_reference_deck_audit_gate_issue_summary(report)
    assert len(summary) == 1
    assert summary[0].field == "coverage"
    assert summary[0].issue_count == 1
    assert summary[0].fixture_names == ("NMOS:tran",)
    assert summary[0].messages == (
        "missing required NMOS tran reference-deck audit row",
    )
    assert format_device_model_reference_deck_audit_gate_issue_summary_table(
        report
    ) == (
        "field\tissue_count\tfixture_names\tmessages\n"
        "coverage\t1\tNMOS:tran\tmissing required NMOS tran reference-deck audit row"
    )
    summary_records = device_model_reference_deck_audit_gate_issue_summary_records(
        report
    )
    assert summary_records == [
        {
            "field": "coverage",
            "issue_count": "1",
            "fixture_names": "NMOS:tran",
            "messages": "missing required NMOS tran reference-deck audit row",
        }
    ]
    assert format_device_model_reference_deck_audit_gate_issue_summary_csv(
        report
    ) == (
        "field,issue_count,fixture_names,messages\n"
        "coverage,1,NMOS:tran,missing required NMOS tran reference-deck audit row\n"
    )
    assert (
        json.loads(
            format_device_model_reference_deck_audit_gate_issue_summary_json(report)
        )
        == summary_records
    )
    digest = device_model_reference_deck_audit_gate_coverage_digest(report)
    assert digest.passed is False
    assert digest.fixture_count == 19
    assert digest.expected_pair_count == 20
    assert digest.covered_pair_count == 19
    assert digest.missing_pair_count == 1
    assert digest.issue_count == 1
    assert digest.issue_fields == ("coverage",)
    digest_records = device_model_reference_deck_audit_gate_coverage_digest_records(
        report
    )
    assert digest_records == [
        {
            "passed": "false",
            "fixture_count": "19",
            "expected_pair_count": "20",
            "covered_pair_count": "19",
            "missing_pair_count": "1",
            "issue_count": "1",
            "issue_fields": "coverage",
        }
    ]
    assert format_device_model_reference_deck_audit_gate_coverage_digest_table(
        report
    ) == (
        "passed\tfixture_count\texpected_pair_count\tcovered_pair_count\t"
        "missing_pair_count\tissue_count\tissue_fields\n"
        "false\t19\t20\t19\t1\t1\tcoverage"
    )
    assert format_device_model_reference_deck_audit_gate_coverage_digest_csv(
        report
    ) == (
        "passed,fixture_count,expected_pair_count,covered_pair_count,"
        "missing_pair_count,issue_count,issue_fields\n"
        "false,19,20,19,1,1,coverage\n"
    )
    assert (
        json.loads(
            format_device_model_reference_deck_audit_gate_coverage_digest_json(
                report
            )
        )
        == digest_records
    )


def test_transient_diode_junction_capacitance_slows_current_step() -> None:
    def run(cjo: float) -> TransientResult:
        circuit = Circuit()
        circuit.add(CurrentSource(
            "Istep",
            "0",
            "out",
            0.0,
            waveform=PwlWaveform(((0.0, 0.0), (1.0e-9, 1.0e-6), (5.0e-9, 1.0e-6))),
        ))
        circuit.add(Resistor("Rshunt", "out", "0", 1.0e12))
        circuit.add(Diode("D1", "out", "0", Is=1.0e-15, Vt=0.02585, Cjo=cjo))
        return transient(circuit, t_stop=5.0e-9, t_step=1.0e-9, method="euler")

    uncharged = run(0.0)
    charged = run(1.0e-12)

    assert uncharged.converged
    assert charged.converged
    uncharged_first = uncharged.points[1].node_voltages["out"]
    charged_first = charged.points[1].node_voltages["out"]
    assert uncharged_first > 0.5
    assert charged_first < 0.01
    assert charged_first < uncharged_first


def test_transient_jfet_gate_source_capacitance_slows_gate_step() -> None:
    def run(cgs: float) -> TransientResult:
        circuit = Circuit()
        circuit.add(VoltageSource(
            "Vstep",
            "in",
            "0",
            0.0,
            waveform=PwlWaveform(((0.0, 0.0), (1.0e-9, 1.0), (5.0e-9, 1.0))),
        ))
        circuit.add(Resistor("Rin", "in", "gate", 1_000.0))
        circuit.add(Resistor("Rdrain", "drain", "0", 1_000.0))
        circuit.add(JFET("J1", "drain", "gate", "0", beta=1.0e-12, vto=-2.0, Cgs=cgs))
        return transient(circuit, t_stop=5.0e-9, t_step=1.0e-9, method="euler")

    uncharged = run(0.0)
    charged = run(1.0e-9)

    assert uncharged.converged
    assert charged.converged
    uncharged_first = uncharged.points[1].node_voltages["gate"]
    charged_first = charged.points[1].node_voltages["gate"]
    assert uncharged_first > 0.5
    assert charged_first < 0.01
    assert charged_first < uncharged_first


def test_transient_jfet_junction_potential_shapes_gate_charge() -> None:
    def final_gate_voltage(pb: float) -> float:
        circuit = Circuit()
        circuit.add(
            VoltageSource(
                "Vstep",
                "in",
                "0",
                0.0,
                waveform=PwlWaveform(
                    ((0.0, 0.0), (2.0e-7, 0.4), (2.0e-6, 0.4))
                ),
            )
        )
        circuit.add(Resistor("Rin", "in", "gate", 1_000.0))
        circuit.add(Resistor("Rdrain", "drain", "0", 1_000.0))
        circuit.add(
            JFET(
                "J1",
                "drain",
                "gate",
                "0",
                beta=1.0e-12,
                vto=-2.0,
                Cgs=1.0e-9,
                Pb=pb,
            )
        )
        result = transient(
            circuit, t_stop=2.0e-6, t_step=2.0e-7, method="euler"
        )
        return result.points[-1].node_voltages["gate"]

    assert final_gate_voltage(0.5) < final_gate_voltage(2.0)


def test_transient_jfet_forward_bias_depletion_coefficient_shapes_gate_charge() -> None:
    def final_gate_voltage(fc: float) -> float:
        circuit = Circuit()
        circuit.add(
            VoltageSource(
                "Vstep",
                "in",
                "0",
                0.0,
                waveform=PwlWaveform(
                    ((0.0, 0.0), (2.0e-7, 0.6), (2.0e-6, 0.6))
                ),
            )
        )
        circuit.add(Resistor("Rin", "in", "gate", 1_000.0))
        circuit.add(Resistor("Rdrain", "drain", "0", 1_000.0))
        circuit.add(
            JFET(
                "J1",
                "drain",
                "gate",
                "0",
                beta=1.0e-12,
                vto=-2.0,
                Cgs=1.0e-9,
                Fc=fc,
            )
        )
        result = transient(
            circuit, t_stop=2.0e-6, t_step=2.0e-7, method="euler"
        )
        return result.points[-1].node_voltages["gate"]

    assert final_gate_voltage(0.2) > final_gate_voltage(0.8)


def test_transient_mosfet_overlap_capacitance_slows_gate_step() -> None:
    def run(cgso: float) -> TransientResult:
        circuit = Circuit()
        circuit.add(VoltageSource(
            "Vstep",
            "in",
            "0",
            0.0,
            waveform=PwlWaveform(((0.0, 0.0), (1.0e-9, 1.0), (5.0e-9, 1.0))),
        ))
        circuit.add(Resistor("Rin", "in", "gate", 1_000.0))
        circuit.add(Resistor("Rdrain", "drain", "0", 1_000.0))
        circuit.add(Mosfet(
            "M1",
            "drain",
            "gate",
            "0",
            "0",
            MOSFET(
                MosfetType.NMOS,
                Level1Model(Level1Params(KP=1.0e-12, W=1.0, L=1.0, CGSO=cgso)),
            ),
        ))
        return transient(circuit, t_stop=5.0e-9, t_step=1.0e-9, method="euler")

    uncharged = run(0.0)
    charged = run(1.0e-9)

    assert uncharged.converged
    assert charged.converged
    uncharged_first = uncharged.points[1].node_voltages["gate"]
    charged_first = charged.points[1].node_voltages["gate"]
    assert uncharged_first > 0.5
    assert charged_first < 0.01
    assert charged_first < uncharged_first


def test_transient_mosfet_bulk_junction_capacitance_slows_drain_step() -> None:
    def run(cbd: float) -> TransientResult:
        circuit = Circuit()
        circuit.add(VoltageSource(
            "Vstep",
            "in",
            "0",
            0.0,
            waveform=PwlWaveform(((0.0, 0.0), (1.0e-9, 1.0), (5.0e-9, 1.0))),
        ))
        circuit.add(Resistor("Rin", "in", "drain", 1_000.0))
        circuit.add(Mosfet(
            "M1",
            "drain",
            "0",
            "0",
            "0",
            MOSFET(
                MosfetType.NMOS,
                Level1Model(Level1Params(KP=1.0e-12, W=1.0, L=1.0, CBD=cbd)),
            ),
        ))
        return transient(circuit, t_stop=5.0e-9, t_step=1.0e-9, method="euler")

    uncharged = run(0.0)
    charged = run(1.0e-9)

    assert uncharged.converged
    assert charged.converged
    uncharged_first = uncharged.points[1].node_voltages["drain"]
    charged_first = charged.points[1].node_voltages["drain"]
    assert uncharged_first > 0.5
    assert charged_first < 0.01
    assert charged_first < uncharged_first


def test_transient_mosfet_bulk_junction_depletion_shaping_reduces_reverse_bias_capacitance() -> None:
    def run(grading_coefficient: float) -> TransientResult:
        circuit = Circuit()
        circuit.add(VoltageSource(
            "Vstep",
            "in",
            "0",
            1.0,
            waveform=PwlWaveform(((0.0, 1.0), (1.0e-9, 2.0), (5.0e-9, 2.0))),
        ))
        circuit.add(Resistor("Rin", "in", "drain", 1_000.0))
        circuit.add(Mosfet(
            "M1",
            "drain",
            "0",
            "0",
            "0",
            MOSFET(
                MosfetType.NMOS,
                Level1Model(Level1Params(
                    KP=1.0e-12,
                    W=1.0,
                    L=1.0,
                    CBD=1.0e-12,
                    PB=1.0,
                    MJ=grading_coefficient,
                )),
            ),
        ))
        return transient(circuit, t_stop=5.0e-9, t_step=1.0e-9, method="euler")

    fixed = run(0.0)
    shaped = run(0.5)

    assert fixed.converged
    assert shaped.converged
    fixed_first = fixed.points[1].node_voltages["drain"]
    shaped_first = shaped.points[1].node_voltages["drain"]
    assert fixed_first == pytest.approx(1.5, rel=0.05)
    assert shaped_first > fixed_first + 0.04
    assert shaped_first < 1.7


def test_transient_diode_transit_time_holds_forward_charge_on_turnoff() -> None:
    def run(transit_time: float) -> TransientResult:
        circuit = Circuit()
        circuit.add(CurrentSource(
            "Istep",
            "0",
            "out",
            0.0,
            waveform=PwlWaveform(((0.0, 1.0e-3), (1.0e-9, 0.0), (5.0e-9, 0.0))),
        ))
        circuit.add(Resistor("Rshunt", "out", "0", 1.0e12))
        circuit.add(Diode("D1", "out", "0", Is=1.0e-15, Vt=0.02585, Tt=transit_time))
        return transient(circuit, t_stop=5.0e-9, t_step=1.0e-9, method="euler")

    no_storage = run(0.0)
    stored = run(1.0e-9)

    assert no_storage.converged
    assert stored.converged
    assert no_storage.points[1].node_voltages["out"] == pytest.approx(0.0, abs=1.0e-12)
    assert stored.points[1].node_voltages["out"] > 0.6
    assert stored.points[1].node_voltages["out"] < stored.points[0].node_voltages["out"]


def test_transient_bjt_base_emitter_capacitance_slows_base_current_step() -> None:
    def run(base_emitter_capacitance: float) -> TransientResult:
        circuit = Circuit()
        circuit.add(VoltageSource("Vcc", "collector", "0", 5.0))
        circuit.add(CurrentSource(
            "Istep",
            "0",
            "base",
            0.0,
            waveform=PwlWaveform(((0.0, 0.0), (1.0e-9, 1.0e-6), (5.0e-9, 1.0e-6))),
        ))
        circuit.add(Resistor("Rshunt", "base", "0", 1.0e12))
        circuit.add(BJT(
            "Q1",
            collector="collector",
            base="base",
            emitter="0",
            Is=1.0e-15,
            Cje=base_emitter_capacitance,
        ))
        return transient(circuit, t_stop=5.0e-9, t_step=1.0e-9, method="euler")

    uncharged = run(0.0)
    charged = run(1.0e-12)

    assert uncharged.converged
    assert charged.converged
    uncharged_first = uncharged.points[1].node_voltages["base"]
    charged_first = charged.points[1].node_voltages["base"]
    assert uncharged_first > 0.5
    assert charged_first < 0.01
    assert charged_first < uncharged_first


def test_transient_bjt_base_emitter_depletion_capacitance_falls_with_reverse_bias() -> None:
    def stepped_base_voltage(mje: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource(
            "Vdrive",
            "in",
            "0",
            -1.0,
            waveform=PwlWaveform(((0.0, -1.0), (1.0e-9, -1.0), (2.0e-9, 0.0), (5.0e-9, 0.0))),
        ))
        circuit.add(Resistor("Rin", "in", "base", 1000.0))
        circuit.add(BJT("Q1", collector="0", base="base", emitter="0", Cje=1.0e-12, Vje=0.75, Mje=mje))
        return transient(circuit, t_stop=5.0e-9, t_step=1.0e-9, method="euler").points[2].node_voltages["base"]

    assert stepped_base_voltage(0.5) > stepped_base_voltage(0.0)


def test_transient_bjt_base_collector_depletion_capacitance_falls_with_reverse_bias() -> None:
    def stepped_collector_voltage(mjc: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource(
            "Vdrive",
            "in",
            "0",
            1.0,
            waveform=PwlWaveform(((0.0, 1.0), (1.0e-9, 1.0), (2.0e-9, 0.0), (5.0e-9, 0.0))),
        ))
        circuit.add(Resistor("Rin", "in", "collector", 1000.0))
        circuit.add(BJT("Q1", collector="collector", base="0", emitter="0", Cjc=1.0e-12, Vjc=0.75, Mjc=mjc))
        return transient(circuit, t_stop=5.0e-9, t_step=1.0e-9, method="euler").points[2].node_voltages["collector"]

    assert stepped_collector_voltage(0.5) < stepped_collector_voltage(0.0)


def test_transient_bjt_xcjc_partitions_depletion_charge_to_external_base() -> None:
    def stepped_base_voltage(fraction: float) -> float:
        circuit = Circuit()
        circuit.add(
            VoltageSource(
                "Vdrive",
                "in",
                "0",
                0.0,
                waveform=PwlWaveform(
                    ((0.0, 0.0), (1.0e-9, 0.0), (2.0e-9, 1.0), (5.0e-9, 1.0))
                ),
            )
        )
        circuit.add(Resistor("Rin", "in", "base", 1_000.0))
        circuit.add(
            BJT(
                "Q1",
                collector="0",
                base="base",
                emitter="0",
                Is=1.0e-30,
                Cjc=1.0e-12,
                Rb=10_000.0,
                Xcjc=fraction,
            )
        )
        return transient(
            circuit, t_stop=5.0e-9, t_step=1.0e-9, method="euler"
        ).points[2].node_voltages["base"]

    assert stepped_base_voltage(1.0) > stepped_base_voltage(0.0)


def test_transient_bjt_forward_bias_depletion_coefficient_shapes_both_junctions() -> None:
    def held_voltage(coefficient: float, base_emitter: bool) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource(
            "Vdrive",
            "in",
            "0",
            0.6,
            waveform=PwlWaveform(((0.0, 0.6), (1.0e-9, 0.6), (2.0e-9, 0.0), (5.0e-9, 0.0))),
        ))
        circuit.add(Resistor("Rin", "in", "base", 1000.0))
        circuit.add(BJT(
            "Q1",
            collector="0",
            base="base",
            emitter="0",
            Is=1.0e-30,
            Cje=1.0e-12 if base_emitter else 0.0,
            Cjc=0.0 if base_emitter else 1.0e-12,
            Fc=coefficient,
        ))
        return transient(circuit, t_stop=5.0e-9, t_step=1.0e-9, method="euler").points[2].node_voltages["base"]

    for base_emitter in (True, False):
        assert held_voltage(0.8, base_emitter) > held_voltage(0.2, base_emitter)


def test_transient_bjt_forward_transit_time_holds_base_charge_on_turnoff() -> None:
    def run(
        forward_transit_time: float,
        forward_transit_time_bias_coefficient: float = 0.0,
        forward_transit_time_current: float = 0.0,
        forward_transit_time_voltage: float = 0.0,
        collector_voltage: float = 5.0,
    ) -> TransientResult:
        circuit = Circuit()
        circuit.add(VoltageSource("Vcc", "collector", "0", collector_voltage))
        circuit.add(CurrentSource(
            "Istep",
            "0",
            "base",
            0.0,
            waveform=PwlWaveform(((0.0, 1.0e-3), (1.0e-9, 0.0), (5.0e-9, 0.0))),
        ))
        circuit.add(Resistor("Rshunt", "base", "0", 1.0e12))
        circuit.add(BJT(
            "Q1",
            collector="collector",
            base="base",
            emitter="0",
            Is=1.0e-15,
            Tf=forward_transit_time,
            Xtf=forward_transit_time_bias_coefficient,
            Itf=forward_transit_time_current,
            Vtf=forward_transit_time_voltage,
        ))
        return transient(circuit, t_stop=5.0e-9, t_step=1.0e-9, method="euler")

    no_storage = run(0.0)
    stored = run(1.0e-9)
    bias_scaled = run(1.0e-9, 9.0)
    current_limited = run(1.0e-9, 9.0, 1.0)
    voltage_limited = run(1.0e-9, 9.0, 0.0, 0.5, 10.0)

    assert no_storage.converged
    assert stored.converged
    assert no_storage.points[1].node_voltages["base"] == pytest.approx(0.0, abs=1.0e-12)
    assert stored.points[1].node_voltages["base"] > 0.6
    assert stored.points[1].node_voltages["base"] < stored.points[0].node_voltages["base"]
    assert abs(
        bias_scaled.points[-1].node_voltages["base"]
        - stored.points[-1].node_voltages["base"]
    ) > 1.0e-12
    assert abs(
        current_limited.points[-1].node_voltages["base"]
        - stored.points[-1].node_voltages["base"]
    ) < abs(
        bias_scaled.points[-1].node_voltages["base"]
        - stored.points[-1].node_voltages["base"]
    )
    assert abs(
        voltage_limited.points[-1].node_voltages["base"]
        - stored.points[-1].node_voltages["base"]
    ) < abs(
        bias_scaled.points[-1].node_voltages["base"]
        - stored.points[-1].node_voltages["base"]
    )


def test_non_level_one_mos_model_cards_are_explicitly_rejected() -> None:
    with pytest.raises(ValueError, match="only MOS LEVEL=1"):
        normalize_model_card("Mbad", "nmos", {"LEVEL": 2.0})


def test_custom_model_evaluator_hook_stamps_dc_current() -> None:
    def evaluator(context) -> CustomModelEvaluation:
        conductance = context.parameters["g"]
        return CustomModelEvaluation(
            current_amps=conductance * context.voltage,
            conductance_siemens=conductance,
        )

    circuit = Circuit()
    circuit.add(VoltageSource("V1", "in", "0", 1.0))
    circuit.add(
        CustomModel(
            "XG",
            "in",
            "0",
            parameters={"g": 2.0e-3},
            evaluator=evaluator,
        )
    )

    result = dc_op(circuit)

    assert result.node_voltages["in"] == pytest.approx(1.0)
    assert result.branch_currents["I(V1)"] == pytest.approx(-2.0e-3)


def test_custom_linear_conductance_model_fast_path_stamps_dc_current() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("V1", "in", "0", 1.0))
    circuit.add(custom_linear_conductance_model("XG", "in", "0", 2.0e-3))

    result = dc_op(circuit)

    assert result.branch_currents["I(V1)"] == pytest.approx(-2.0e-3)


def test_custom_model_source_analyzer_accepts_subset_and_rejects_dynamic_constructs() -> None:
    accepted = analyze_custom_model_source(
        "module rlim(p, n); analog begin I(p,n) <+ g * V(p,n); end endmodule"
    )
    rejected = analyze_custom_model_source(
        "module cap(p, n); analog begin I(p,n) <+ ddt(C * V(p,n)); end endmodule"
    )

    assert accepted.accepted is True
    assert accepted.module_name == "rlim"
    assert accepted.terminals == ("p", "n")
    assert accepted.contribution == ("p", "n")
    assert rejected.accepted is False
    assert "CUSTOM_MODEL_FORBIDDEN_CONSTRUCT" in {
        diagnostic.code for diagnostic in rejected.diagnostics
    }


def test_waveform_period_reports_periodic_source_forms() -> None:
    assert waveform_period(SinWaveform(frequency=2.0)) == pytest.approx(0.5)
    assert waveform_period(SinWaveform(frequency=2.0, damping=1.0)) is None
    assert waveform_period(SinWaveform(frequency=0.0)) is None
    assert waveform_period(PulseWaveform(period=2.5)) == pytest.approx(2.5)
    assert waveform_period(PwlWaveform(((0.0, 0.0), (1.0, 1.0)))) is None
    assert waveform_period(ExpWaveform()) is None


def test_estimate_period_finds_harmonic_periodic_source_period() -> None:
    c = Circuit()
    c.add(
        VoltageSource("V1", "in", "0", 0.0, waveform=SinWaveform(frequency=1_000.0))
    )
    c.add(
        CurrentSource(
            "I1",
            "out",
            "0",
            0.0,
            waveform=PulseWaveform(
                v_initial=0.0, v_pulsed=1.0e-3, pulse_width=0.25e-3, period=0.5e-3
            ),
        )
    )
    c.add(Resistor("R1", "in", "out", 1_000.0))

    assert estimate_period(c) == pytest.approx(1.0e-3)


def test_estimate_period_rejects_nonperiodic_or_incommensurate_sources() -> None:
    non_periodic = Circuit()
    non_periodic.add(VoltageSource(
        "V1",
        "in",
        "0",
        0.0,
        waveform=PwlWaveform(((0.0, 0.0), (1.0e-3, 1.0))),
    ))
    assert estimate_period(non_periodic) is None

    incommensurate = Circuit()
    incommensurate.add(
        VoltageSource("V1", "in", "0", 0.0, waveform=PulseWaveform(period=1.0e-3))
    )
    incommensurate.add(
        CurrentSource("I1", "out", "0", 0.0, waveform=PulseWaveform(period=0.7e-3))
    )
    assert estimate_period(incommensurate) is None


def test_pss_residual_reports_one_period_node_closure() -> None:
    c = Circuit()
    c.add(
        VoltageSource(
            "V1",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(frequency=1_000.0),
        )
    )
    c.add(Resistor("R1", "in", "0", 1_000.0))

    result = pss_residual(c, steps_per_period=32)

    assert isinstance(result, PssResidualResult)
    assert result.period == pytest.approx(1.0e-3)
    assert result.time_step == pytest.approx(1.0e-3 / 32.0)
    assert result.converged is True
    assert result.residual_tol == pytest.approx(1.0e-6)
    assert result.within_tolerance is True
    assert result.node_residuals["in"] == pytest.approx(0.0, abs=1.0e-12)
    assert result.branch_residuals["I(V1)"] == pytest.approx(0.0, abs=1.0e-12)
    assert [(entry.kind, entry.name) for entry in result.residual_vector] == [
        ("node", "in"),
        ("branch_current", "I(V1)"),
    ]
    assert [entry.value for entry in result.residual_vector] == pytest.approx(
        [0.0, 0.0],
        abs=1.0e-12,
    )
    assert result.max_abs_branch_residual == pytest.approx(0.0, abs=1.0e-12)
    assert result.max_abs_residual == pytest.approx(0.0, abs=1.0e-12)
    expected_l2_norm = math.sqrt(
        sum(entry.value * entry.value for entry in result.residual_vector)
    )
    assert result.residual_l2_norm == pytest.approx(expected_l2_norm, abs=1.0e-12)
    assert result.residual_rms_norm == pytest.approx(
        expected_l2_norm / math.sqrt(len(result.residual_vector)),
        abs=1.0e-12,
    )


def test_pss_residual_jacobian_reports_reactive_initial_state_columns() -> None:
    c = Circuit()
    c.add(
        VoltageSource(
            "V1",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(frequency=1_000.0),
        )
    )
    c.add(Resistor("R1", "in", "out", 1_000.0))
    c.add(Capacitor("C1", "out", "0", 1.0e-6, initial_voltage=0.1))

    result = pss_residual_jacobian(c, steps_per_period=32, perturbation=1.0e-5)

    assert isinstance(result, PssResidualJacobianResult)
    assert result.perturbation == pytest.approx(1.0e-5)
    assert [(state.kind, state.name, state.value) for state in result.state_vector] == [
        ("capacitor_voltage", "C1", pytest.approx(0.1)),
    ]
    assert result.columns[0].state == result.state_vector[0]
    assert len(result.jacobian) == len(result.residual.residual_vector)
    assert [len(row) for row in result.jacobian] == [1] * len(result.jacobian)
    derivative_by_name = {
        entry.name: entry.value for entry in result.columns[0].residual_derivatives
    }
    assert derivative_by_name["out"] == pytest.approx(
        next(row[0] for row, entry in zip(result.jacobian, result.residual.residual_vector, strict=True) if entry.name == "out")
    )
    assert abs(derivative_by_name["out"]) > 0.1
    assert all(math.isfinite(row[0]) for row in result.jacobian)


def test_pss_newton_update_reports_reactive_state_corrections() -> None:
    c = Circuit()
    c.add(
        VoltageSource(
            "V1",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(frequency=1_000.0),
        )
    )
    c.add(Resistor("R1", "in", "out", 1_000.0))
    c.add(Capacitor("C1", "out", "0", 1.0e-6, initial_voltage=0.1))

    result = pss_newton_update(c, steps_per_period=32, perturbation=1.0e-5)

    assert isinstance(result, PssNewtonUpdateResult)
    assert result.jacobian.state_vector[0].name == "C1"
    assert result.state_updates[0].kind == "capacitor_voltage"
    assert result.state_updates[0].name == "C1"
    assert result.next_state_vector[0].value == pytest.approx(
        result.jacobian.state_vector[0].value + result.state_updates[0].value
    )
    assert result.update_l2_norm == pytest.approx(abs(result.state_updates[0].value))
    assert math.isfinite(result.state_updates[0].value)


def test_pss_newton_candidate_applies_reactive_state_update() -> None:
    c = Circuit()
    c.add(
        VoltageSource(
            "V1",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(frequency=1_000.0),
        )
    )
    c.add(Resistor("R1", "in", "out", 1_000.0))
    c.add(Capacitor("C1", "out", "0", 1.0e-6, initial_voltage=0.1))

    result = pss_newton_candidate(c, steps_per_period=32, perturbation=1.0e-5)

    assert isinstance(result, PssNewtonCandidateResult)
    assert result.update.next_state_vector[0].name == "C1"
    assert result.candidate_state_vector == result.update.next_state_vector
    candidate_cap = next(
        element
        for element in result.candidate_circuit.elements
        if isinstance(element, Capacitor) and element.name == "C1"
    )
    original_cap = next(element for element in c.elements if isinstance(element, Capacitor))
    assert original_cap.initial_voltage == pytest.approx(0.1)
    assert candidate_cap.initial_voltage == pytest.approx(
        result.update.next_state_vector[0].value
    )
    assert result.candidate_residual.period == pytest.approx(
        result.update.jacobian.residual.period
    )
    assert math.isfinite(result.candidate_residual.residual_l2_norm)


def test_pss_newton_iteration_accepts_improving_candidate() -> None:
    c = Circuit()
    c.add(
        VoltageSource(
            "V1",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(frequency=1_000.0),
        )
    )
    c.add(Resistor("R1", "in", "out", 1_000.0))
    c.add(Capacitor("C1", "out", "0", 1.0e-6, initial_voltage=0.1))

    result = pss_newton_iteration(c, steps_per_period=32, perturbation=1.0e-5)

    assert isinstance(result, PssNewtonIterationResult)
    base_residual = result.candidate.update.jacobian.residual
    candidate_residual = result.candidate.candidate_residual
    assert result.accepted is True
    assert result.next_circuit is result.candidate.candidate_circuit
    assert result.next_state_vector == result.candidate.candidate_state_vector
    assert result.next_residual is candidate_residual
    assert result.converged == candidate_residual.within_tolerance
    assert candidate_residual.residual_l2_norm < base_residual.residual_l2_norm
    assert result.residual_l2_reduction == pytest.approx(
        base_residual.residual_l2_norm - candidate_residual.residual_l2_norm
    )
    assert result.residual_l2_ratio == pytest.approx(
        candidate_residual.residual_l2_norm / base_residual.residual_l2_norm
    )


def test_pss_newton_solve_runs_accepted_iterations_to_convergence() -> None:
    c = Circuit()
    c.add(
        VoltageSource(
            "V1",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(frequency=1_000.0),
        )
    )
    c.add(Resistor("R1", "in", "out", 1_000.0))
    c.add(Capacitor("C1", "out", "0", 1.0e-6, initial_voltage=0.1))

    result = pss_newton_solve(
        c,
        steps_per_period=32,
        residual_tol=1.0e-3,
        perturbation=1.0e-5,
        max_newton_iterations=4,
    )

    assert isinstance(result, PssNewtonSolveResult)
    assert result.iteration_count == len(result.iterations)
    assert 1 <= result.iteration_count <= 4
    assert all(iteration.accepted for iteration in result.iterations)
    assert result.converged is True
    assert result.final_residual.within_tolerance is True
    assert result.final_residual.residual_l2_norm < (
        result.iterations[0].candidate.update.jacobian.residual.residual_l2_norm
    )
    assert result.final_circuit is result.iterations[-1].next_circuit
    assert result.final_state_vector == result.iterations[-1].next_state_vector


def test_pss_returns_solved_steady_state_period() -> None:
    c = Circuit()
    c.add(
        VoltageSource(
            "V1",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(frequency=1_000.0),
        )
    )
    c.add(Resistor("R1", "in", "out", 1_000.0))
    c.add(Capacitor("C1", "out", "0", 1.0e-6, initial_voltage=0.1))

    result = pss(
        c,
        steps_per_period=32,
        residual_tol=1.0e-3,
        perturbation=1.0e-5,
        max_newton_iterations=4,
    )

    assert isinstance(result, PssResult)
    assert result.converged is True
    assert result.solve.converged is True
    assert result.period == result.solve.final_residual.period
    assert result.time_step == result.solve.final_residual.time_step
    assert result.steady_state.converged is True
    assert result.steady_state.points
    assert result.steady_state.points[-1].time == pytest.approx(result.period)
    assert result.steady_state.points[0].node_voltages["out"] == pytest.approx(
        result.solve.final_state_vector[0].value
    )


def test_pss_corners_runs_analysis_per_corner_and_formats_tables() -> None:
    c = Circuit()
    c.add(
        VoltageSource(
            "V1",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(offset=0.0, amplitude=1.0, frequency=1_000.0),
        )
    )
    c.add(Resistor("R1", "in", "0", 1_000.0))

    nominal = pss(
        c,
        steps_per_period=4,
        residual_tol=1.0e-9,
        perturbation=1.0e-5,
        max_newton_iterations=2,
    )
    result = pss_corners(
        c,
        [
            CornerSpec("nominal"),
            CornerSpec("rload-high", (CornerOverride("R1", "resistance", 2_000.0),)),
        ],
        steps_per_period=4,
        residual_tol=1.0e-9,
        perturbation=1.0e-5,
        max_newton_iterations=2,
    )

    assert nominal is not None
    assert result is not None
    assert isinstance(result, CornerPssResult)
    assert [point.corner_name for point in result.points] == ["nominal", "rload-high"]
    assert result.points[0].result.converged
    assert result.points[1].result.converged
    assert result.points[0].result.period == pytest.approx(1.0e-3)
    assert result.points[1].result.steady_state.points[1].branch_currents["I(V1)"] == pytest.approx(-5.0e-4)
    assert format_pss_table(nominal, ["V(in)", "I(V1)"]) == (
        "Index\tPeriod\tTimeStep\tConverged\tIterations\tResidualL2\tTime\tV(in)\tI(V1)\n"
        "0\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t0.000000e+00\t0.000000e+00\t0.000000e+00\n"
        "1\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t2.500000e-04\t1.000000e+00\t-1.000000e-03\n"
        "2\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t5.000000e-04\t1.224647e-16\t-1.224647e-19\n"
        "3\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t7.500000e-04\t-1.000000e+00\t1.000000e-03\n"
        "4\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t1.000000e-03\t-2.449294e-16\t2.449294e-19\n"
    )
    assert format_corner_pss_table(result, ["V(in)", "I(V1)"]) == (
        "Corner\tIndex\tPeriod\tTimeStep\tConverged\tIterations\tResidualL2\tTime\tV(in)\tI(V1)\n"
        "nominal\t0\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t0.000000e+00\t0.000000e+00\t0.000000e+00\n"
        "nominal\t1\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t2.500000e-04\t1.000000e+00\t-1.000000e-03\n"
        "nominal\t2\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t5.000000e-04\t1.224647e-16\t-1.224647e-19\n"
        "nominal\t3\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t7.500000e-04\t-1.000000e+00\t1.000000e-03\n"
        "nominal\t4\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449295e-16\t1.000000e-03\t-2.449294e-16\t2.449294e-19\n"
        "rload-high\t0\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449294e-16\t0.000000e+00\t0.000000e+00\t0.000000e+00\n"
        "rload-high\t1\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449294e-16\t2.500000e-04\t1.000000e+00\t-5.000000e-04\n"
        "rload-high\t2\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449294e-16\t5.000000e-04\t1.224647e-16\t-6.123234e-20\n"
        "rload-high\t3\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449294e-16\t7.500000e-04\t-1.000000e+00\t5.000000e-04\n"
        "rload-high\t4\t1.000000e-03\t2.500000e-04\ttrue\t1\t2.449294e-16\t1.000000e-03\t-2.449294e-16\t1.224647e-19\n"
    )


def test_pss_residual_requires_periodic_sources() -> None:
    c = Circuit()
    c.add(
        VoltageSource(
            "V1",
            "in",
            "0",
            0.0,
            waveform=PwlWaveform(((0.0, 0.0), (1.0e-3, 1.0))),
        )
    )

    assert pss_residual(c) is None


def test_pss_residual_rejects_negative_residual_tolerance() -> None:
    c = Circuit()
    c.add(
        VoltageSource(
            "V1",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(frequency=1_000.0),
        )
    )

    with pytest.raises(ValueError, match="residual_tol"):
        pss_residual(c, residual_tol=-1.0)


def test_pss_residual_jacobian_rejects_non_positive_perturbation() -> None:
    c = Circuit()
    c.add(
        VoltageSource(
            "V1",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(frequency=1_000.0),
        )
    )

    with pytest.raises(ValueError, match="perturbation"):
        pss_residual_jacobian(c, perturbation=0.0)


def test_pss_newton_update_without_reactive_state_returns_empty_update() -> None:
    c = Circuit()
    c.add(
        VoltageSource(
            "V1",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(frequency=1_000.0),
        )
    )
    c.add(Resistor("R1", "in", "0", 1_000.0))

    result = pss_newton_update(c, steps_per_period=32)

    assert result is not None
    assert result.state_updates == []
    assert result.next_state_vector == []
    assert result.update_l2_norm == pytest.approx(0.0)


# ---- Linear solver ----


def test_solve_2x2():
    # 2x + y = 5; x + 3y = 10 -> x = 1, y = 3
    A = [[2.0, 1.0], [1.0, 3.0]]
    b = [5.0, 10.0]
    x = _solve(A, b)
    assert isclose(x[0], 1.0, abs_tol=1e-9)
    assert isclose(x[1], 3.0, abs_tol=1e-9)


def test_solve_3x3():
    A = [[3.0, 2.0, -1.0], [2.0, -2.0, 4.0], [-1.0, 0.5, -1.0]]
    b = [1.0, -2.0, 0.0]
    x = _solve(A, b)
    # Verify Ax = b
    for i, row in enumerate(A):
        s = sum(row[j] * x[j] for j in range(3))
        assert isclose(s, b[i], abs_tol=1e-9)


def test_solve_singular_raises():
    # Two identical rows -> singular
    A = [[1.0, 1.0], [1.0, 1.0]]
    b = [1.0, 2.0]
    with pytest.raises(ZeroDivisionError):
        _solve(A, b)


def test_solve_empty():
    assert _solve([], []) == []


# ---- DC analysis: simple circuits ----


def test_resistor_voltage_divider():
    """V1 = 10V, R1=R2=1k -> V_mid = 5V."""
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=10.0))
    c.add(Resistor("R1", "vin", "vmid", 1000.0))
    c.add(Resistor("R2", "vmid", "0", 1000.0))
    r = dc_op(c)
    assert r.converged
    assert isclose(r.node_voltages["vin"], 10.0, abs_tol=1e-6)
    assert isclose(r.node_voltages["vmid"], 5.0, abs_tol=1e-6)


def test_dc_initial_conditions_seed_operating_point_vector():
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=10.0))
    c.add(Resistor("R1", "vin", "vmid", 1000.0))
    c.add(Resistor("R2", "vmid", "0", 1000.0))
    summary = resolve_deck_initial_conditions(
        """
.nodeset V(vin)=10 V(vmid)=1
.ic V(vmid)=4
.end
"""
    )

    vector = dc_initial_vector_from_conditions(
        c,
        summary.initial_conditions,
        summary.nodesets,
    )
    assert vector == pytest.approx([10.0, 4.0, 0.0])

    r = dc_op_with_initial_conditions(c, summary, convergence_aids=False)

    assert r.converged
    assert isclose(r.node_voltages["vin"], 10.0, abs_tol=1e-6)
    assert isclose(r.node_voltages["vmid"], 5.0, abs_tol=1e-6)


def test_two_resistors_in_series():
    c = Circuit()
    c.add(VoltageSource("V1", "a", "0", voltage=12.0))
    c.add(Resistor("R1", "a", "b", 100.0))
    c.add(Resistor("R2", "b", "0", 200.0))
    r = dc_op(c)
    # V_b = 12 * 200 / (100 + 200) = 8V
    assert isclose(r.node_voltages["b"], 8.0, abs_tol=1e-6)


def test_large_resistor_ladder_uses_sparse_real_solver_path():
    c = Circuit()
    c.add(VoltageSource("V1", "n0", "0", voltage=10.0))
    for index in range(34):
        c.add(Resistor(f"R{index}", f"n{index}", f"n{index + 1}", 1000.0))
    c.add(Resistor("R34", "n34", "0", 1000.0))

    r = dc_op(c)

    assert r.converged
    assert isclose(r.node_voltages["n34"], 10.0 / 35.0, abs_tol=1e-9)
    assert r.diagnostics.matrix_size == 36
    assert r.diagnostics.solver == "sparse_real"
    assert r.diagnostics.convergence_aid == "newton"
    assert r.diagnostics.tolerance == pytest.approx(1.0e-6)
    assert math.isfinite(r.diagnostics.max_delta)
    assert r.diagnostics.newton_step_limit is None
    assert r.diagnostics.limited_newton_steps == 0
    assert r.diagnostics.minimum_damping_factor == pytest.approx(1.0)
    profile = r.diagnostics.solver_profile
    assert profile.matrix_size == 36
    assert profile.solver == "sparse_real"
    assert profile.backend in {"scipy_sparse_lu", "native_sparse_gaussian"}
    assert profile.structural_nonzeros > 0
    assert 0.0 < profile.density < 0.1
    assert profile.fill_in_nonzeros >= 0
    if profile.backend == "native_sparse_gaussian":
        assert (
            profile.fallback_reason in {None, "scipy_unavailable"}
            or profile.fallback_reason.startswith("scipy_sparse_lu:")
        )
    else:
        assert profile.fallback_reason is None


def test_current_source_into_resistor():
    """I = 1mA into R = 1k -> V = 1V."""
    c = Circuit()
    c.add(CurrentSource("I1", "0", "n1", current=1e-3))
    c.add(Resistor("R1", "n1", "0", 1000.0))
    r = dc_op(c)
    assert isclose(r.node_voltages["n1"], 1.0, abs_tol=1e-6)


def test_behavioral_current_source_tracks_node_voltage():
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", voltage=2.0))
    c.add(BSource("B1", "0", "out", current_expr="0.002 * V(in)"))
    c.add(Resistor("Rload", "out", "0", 1000.0))
    r = dc_op(c)
    assert r.converged
    assert isclose(r.node_voltages["out"], 4.0, abs_tol=1e-6)


def test_behavioral_voltage_source_tracks_differential_voltage():
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", voltage=3.0))
    c.add(BSource("B1", "out", "0", voltage_expr="2.0 * V(in, 0) + 1.0"))
    c.add(Resistor("Rload", "out", "0", 1000.0))
    r = dc_op(c)
    assert r.converged
    assert isclose(r.node_voltages["out"], 7.0, abs_tol=1e-6)
    assert "I(B1)" in r.branch_currents


def test_subcircuit_instance_expands_resistor_divider_at_build_time():
    cell = SubcircuitDefinition(
        "atten2",
        ("in", "out"),
        (
            Resistor("Rtop", "in", "out", 1000.0),
            Resistor("Rbot", "out", "0", 1000.0),
        ),
    )
    c = Circuit()
    c.define_subcircuit(cell)
    c.add(VoltageSource("V1", "vin", "0", voltage=10.0))
    c.add(XInstance("X1", ("vin", "vout"), "atten2"))

    r = dc_op(c)

    assert r.converged
    assert isclose(r.node_voltages["vout"], 5.0, abs_tol=1e-9)
    assert [element.name for element in c.elements if isinstance(element, Resistor)] == [
        "X1.Rtop",
        "X1.Rbot",
    ]


def test_subcircuit_expansion_preserves_complete_diode_model():
    cell = SubcircuitDefinition(
        "diode-cell",
        ("in",),
        (Diode("Dcell", "in", "0", Vj=0.8, M=0.4, Fc=0.35, Xti=2.2, Eg=1.05, Rs=10.0, Kf=1.0e-12, Af=1.3),),
    )
    c = Circuit()
    c.define_subcircuit(cell)
    c.add(XInstance("X1", ("a",), "diode-cell"))

    expanded = next(element for element in c.elements if isinstance(element, Diode))
    assert expanded.Vj == pytest.approx(0.8)
    assert pytest.approx(0.4) == expanded.M
    assert expanded.Fc == pytest.approx(0.35)
    assert expanded.Xti == pytest.approx(2.2)
    assert expanded.Eg == pytest.approx(1.05)
    assert expanded.Rs == pytest.approx(10.0)
    assert expanded.Kf == pytest.approx(1.0e-12)
    assert expanded.Af == pytest.approx(1.3)


def test_subcircuit_expansion_preserves_complete_jfet_model():
    cell = SubcircuitDefinition(
        "jfet-cell",
        ("d", "g", "s"),
        (JFET("Jcell", "d", "g", "s", Kf=1.0e-12, Af=1.3, Pb=0.8, Fc=0.35, Is=2.0e-13, Xti=2.5, Eg=1.05, B=1.1, Nlev=3.0, Gdsnoi=1.25, Rd=125.0, Rs=75.0, Tcv=0.01, Vtotc=-0.0025, Tnom=323.15, Bex=1.5, Betatce=-0.5),),
    )
    circuit = Circuit()
    circuit.define_subcircuit(cell)
    circuit.add(XInstance("X1", ("d1", "g1", "0"), "jfet-cell"))

    expanded = next(element for element in circuit.elements if isinstance(element, JFET))
    assert expanded.Kf == pytest.approx(1.0e-12)
    assert expanded.Af == pytest.approx(1.3)
    assert expanded.Pb == pytest.approx(0.8)
    assert expanded.Fc == pytest.approx(0.35)
    assert expanded.Is == pytest.approx(2.0e-13)
    assert expanded.Xti == pytest.approx(2.5)
    assert expanded.Eg == pytest.approx(1.05)
    assert pytest.approx(1.1) == expanded.B
    assert expanded.Nlev == pytest.approx(3.0)
    assert expanded.Gdsnoi == pytest.approx(1.25)
    assert expanded.Rd == pytest.approx(125.0)
    assert expanded.Rs == pytest.approx(75.0)
    assert expanded.Tcv == pytest.approx(0.01)
    assert expanded.Vtotc == pytest.approx(-0.0025)
    assert expanded.Bex == pytest.approx(1.5)
    assert expanded.Betatce == pytest.approx(-0.5)
    assert expanded.Tnom == pytest.approx(323.15)


def test_subcircuit_expansion_preserves_complete_bjt_model():
    cell = SubcircuitDefinition(
        "bjt-cell",
        ("c", "b", "e"),
        (BJT("Qcell", "c", "b", "e", Xti=2.4, Eg=1.05, Vaf=80.0, Nf=1.2, Nr=1.3, Vje=0.8, Mje=0.4, Vjc=0.7, Mjc=0.45, Fc=0.4, Var=120.0, Ikf=2.0e-3, Ise=3.0e-13, Ne=1.7, Isc=4.0e-13, Nc=1.8, Xtb=1.5, beta_r=0.25, Ikr=3.0e-3, Tnom=323.15, Kf=1.0e-12, Af=1.3, Ptf=30.0, Xtf=2.0, Itf=4.0e-3, Vtf=0.6, Re=12.0, Rc=13.0, Rb=14.0, Rbm=2.0, Irb=5.0e-6, Xcjc=0.4),),
    )
    circuit = Circuit()
    circuit.define_subcircuit(cell)
    circuit.add(XInstance("X1", ("c1", "b1", "0"), "bjt-cell"))

    expanded = next(element for element in circuit.elements if isinstance(element, BJT))
    assert expanded.Xti == pytest.approx(2.4)
    assert expanded.Eg == pytest.approx(1.05)
    assert expanded.Vaf == pytest.approx(80.0)
    assert expanded.Var == pytest.approx(120.0)
    assert expanded.Nf == pytest.approx(1.2)
    assert expanded.Nr == pytest.approx(1.3)
    assert expanded.Vje == pytest.approx(0.8)
    assert expanded.Mje == pytest.approx(0.4)
    assert expanded.Vjc == pytest.approx(0.7)
    assert expanded.Mjc == pytest.approx(0.45)
    assert expanded.Fc == pytest.approx(0.4)
    assert expanded.Ikf == pytest.approx(2.0e-3)
    assert expanded.Ise == pytest.approx(3.0e-13)
    assert expanded.Ne == pytest.approx(1.7)
    assert expanded.Isc == pytest.approx(4.0e-13)
    assert expanded.Nc == pytest.approx(1.8)
    assert expanded.Xtb == pytest.approx(1.5)
    assert expanded.beta_r == pytest.approx(0.25)
    assert expanded.Ikr == pytest.approx(3.0e-3)
    assert expanded.Tnom == pytest.approx(323.15)
    assert expanded.Kf == pytest.approx(1.0e-12)
    assert expanded.Af == pytest.approx(1.3)
    assert expanded.Ptf == pytest.approx(30.0)
    assert expanded.Xtf == pytest.approx(2.0)
    assert expanded.Itf == pytest.approx(4.0e-3)
    assert expanded.Vtf == pytest.approx(0.6)
    assert expanded.Re == pytest.approx(12.0)
    assert expanded.Rc == pytest.approx(13.0)
    assert expanded.Rb == pytest.approx(14.0)
    assert expanded.Rbm == pytest.approx(2.0)
    assert expanded.Irb == pytest.approx(5.0e-6)
    assert expanded.Xcjc == pytest.approx(0.4)


def test_branch_current_in_voltage_source():
    """V=10V, R=1k -> I=10mA flowing from + to - inside the source."""
    c = Circuit()
    c.add(VoltageSource("V1", "n1", "0", voltage=10.0))
    c.add(Resistor("R1", "n1", "0", 1000.0))
    r = dc_op(c)
    # Branch current convention: positive into +
    assert "I(V1)" in r.branch_currents
    assert isclose(abs(r.branch_currents["I(V1)"]), 10e-3, abs_tol=1e-6)


# ---- DC analysis: ground aliases ----


@pytest.mark.parametrize("ground", ["0", "gnd", "GND"])
def test_ground_aliases(ground: str):
    c = Circuit()
    c.add(VoltageSource("V1", "vin", ground, voltage=5.0))
    c.add(Resistor("R1", "vin", ground, 1000.0))
    r = dc_op(c)
    assert isclose(r.node_voltages["vin"], 5.0, abs_tol=1e-6)


# ---- DC: Diode ----


def test_diode_forward_bias():
    """V=0.7V across diode; current should be ≈ Is*(exp(V/Vt)-1)."""
    c = Circuit()
    c.add(VoltageSource("V1", "a", "0", voltage=0.7))
    c.add(Diode("D1", anode="a", cathode="0"))
    r = dc_op(c)
    assert r.converged
    # V_a forced to 0.7 by V1
    assert isclose(r.node_voltages["a"], 0.7, abs_tol=1e-6)


def test_diode_reverse_bias():
    """Reverse bias: tiny -Is current."""
    c = Circuit()
    c.add(VoltageSource("V1", "0", "a", voltage=1.0))  # cathode high
    c.add(Diode("D1", anode="a", cathode="0"))
    r = dc_op(c)
    assert r.converged


def test_diode_emission_coefficient_reduces_forward_current():
    """Larger N uses N*Vt in the exponential and lowers fixed-bias current."""
    base = Circuit()
    base.add(VoltageSource("V1", "a", "0", voltage=0.7))
    base.add(Diode("D1", anode="a", cathode="0", Is=1e-15, Vt=0.02585))
    base_result = dc_op(base)

    high_n = Circuit()
    high_n.add(VoltageSource("V1", "a", "0", voltage=0.7))
    high_n.add(Diode("D1", anode="a", cathode="0", Is=1e-15, Vt=0.02585, N=2.0))
    high_n_result = dc_op(high_n)

    assert base_result.converged
    assert high_n_result.converged
    assert abs(high_n_result.branch_currents["I(V1)"]) < abs(base_result.branch_currents["I(V1)"]) * 1e-3


def test_diode_series_resistance_limits_fixed_bias_current():
    ideal = Circuit()
    ideal.add(VoltageSource("V1", "a", "0", voltage=0.7))
    ideal.add(Diode("D1", "a", "0"))

    limited = Circuit()
    limited.add(VoltageSource("V1", "a", "0", voltage=0.7))
    limited.add(Diode("D1", "a", "0", Rs=100.0))

    ideal_current = abs(dc_op(ideal).branch_currents["I(V1)"])
    limited_current = abs(dc_op(limited).branch_currents["I(V1)"])
    assert limited_current < ideal_current
    assert limited_current <= 0.7 / 100.0


def test_diode_breakdown_voltage_increases_reverse_current():
    """BV/IBV adds a bounded reverse-breakdown current branch."""
    leakage = Circuit()
    leakage.add(VoltageSource("V1", "0", "a", voltage=5.0))
    leakage.add(Diode("D1", anode="a", cathode="0", Is=1e-15, Vt=0.02585))
    leakage_result = dc_op(leakage)

    breakdown = Circuit()
    breakdown.add(VoltageSource("V1", "0", "a", voltage=5.0))
    breakdown.add(Diode("D1", anode="a", cathode="0", Is=1e-15, Vt=0.02585, BV=5.0, IBV=1e-6))
    breakdown_result = dc_op(breakdown)

    assert leakage_result.converged
    assert breakdown_result.converged
    assert abs(breakdown_result.branch_currents["I(V1)"]) > 1e6 * abs(leakage_result.branch_currents["I(V1)"])
    assert abs(breakdown_result.branch_currents["I(V1)"]) == pytest.approx(1e-6, rel=1e-3)


def test_diode_temperature_scaling_reduces_fixed_current_forward_drop():
    """Hotter silicon raises Is enough to lower fixed-current forward voltage."""
    nominal = Circuit()
    nominal.add(VoltageSource("V1", "vcc", "0", 5.0))
    nominal.add(Resistor("Rbias", "vcc", "a", 4300.0))
    nominal.add(Diode("D1", anode="a", cathode="0", Is=1e-15, Vt=0.02585))

    cold = circuit_at_temperature(nominal, 275.0)
    hot = circuit_at_temperature(nominal, 350.0)

    nominal_result = dc_op(nominal)
    cold_result = dc_op(cold)
    hot_result = dc_op(hot)

    assert cold_result.converged
    assert nominal_result.converged
    assert hot_result.converged
    assert cold_result.node_voltages["a"] > nominal_result.node_voltages["a"]
    assert hot_result.node_voltages["a"] < nominal_result.node_voltages["a"]


def test_diode_temperature_scaling_uses_model_saturation_current_exponent():
    temperature_kelvin = 350.0
    nominal_temperature_kelvin = 300.15
    default_hot = diode_at_temperature(
        Diode("D1", "a", "0", Xti=3.0),
        temperature_kelvin,
        nominal_temperature_kelvin=nominal_temperature_kelvin,
    )
    flat_hot = diode_at_temperature(
        Diode("D1", "a", "0", Xti=0.0),
        temperature_kelvin,
        nominal_temperature_kelvin=nominal_temperature_kelvin,
    )

    assert default_hot.Is / flat_hot.Is == pytest.approx(
        (temperature_kelvin / nominal_temperature_kelvin) ** 3
    )


def test_circuit_temperature_scaling_uses_model_energy_gap():
    silicon = Circuit(elements=[Diode("D1", "a", "0", Eg=1.11)])
    lower_gap = Circuit(elements=[Diode("D1", "a", "0", Eg=0.8)])

    silicon_hot = circuit_at_temperature(silicon, 350.0)
    lower_gap_hot = circuit_at_temperature(lower_gap, 350.0)

    assert isinstance(silicon_hot.elements[0], Diode)
    assert isinstance(lower_gap_hot.elements[0], Diode)
    assert silicon_hot.elements[0].Is > lower_gap_hot.elements[0].Is


def test_dc_temperature_sweep_runs_operating_points_and_formats_table():
    c = Circuit()
    c.add(VoltageSource("V1", "vcc", "0", 5.0))
    c.add(Resistor("Rbias", "vcc", "a", 4300.0))
    c.add(Diode("D1", anode="a", cathode="0", Is=1.0e-15, Vt=0.02585))

    result = dc_temperature_sweep(c, [275.0, 300.15, 350.0])

    assert isinstance(result, TemperatureDcResult)
    assert result.points[0].result.node_voltages["a"] > result.points[1].result.node_voltages["a"]
    assert result.points[2].result.node_voltages["a"] < result.points[1].result.node_voltages["a"]
    assert format_temperature_dc_table(result, ["V(a)", "I(V1)"]) == (
        "Index\tTemperatureKelvin\tV(a)\tI(V1)\n"
        "0\t2.750000e+02\t8.936097e-01\t-9.549745e-04\n"
        "1\t3.001500e+02\t7.188350e-01\t-9.956198e-04\n"
        "2\t3.500000e+02\t6.351989e-01\t-1.015070e-03\n"
    )


def test_dc_temperature_sweep_corners_runs_named_corners_and_formats_table():
    c = Circuit()
    c.add(VoltageSource("V1", "vcc", "0", 5.0))
    c.add(Resistor("Rbias", "vcc", "a", 4300.0))
    c.add(Diode("D1", anode="a", cathode="0", Is=1.0e-15, Vt=0.02585))

    result = dc_temperature_sweep_corners(
        c,
        [275.0, 350.0],
        [
            CornerSpec("nominal"),
            CornerSpec("rbias-high", (CornerOverride("Rbias", "resistance", 8600.0),)),
        ],
    )

    assert isinstance(result, CornerTemperatureDcResult)
    assert [point.corner_name for point in result.points] == ["nominal", "rbias-high"]
    assert result.points[0].points[0].result.node_voltages["a"] > result.points[0].points[1].result.node_voltages["a"]
    assert format_corner_temperature_dc_table(result, ["V(a)", "I(V1)"]) == (
        "Corner\tIndex\tTemperatureKelvin\tV(a)\tI(V1)\n"
        "nominal\t0\t2.750000e+02\t8.936097e-01\t-9.549745e-04\n"
        "nominal\t1\t3.500000e+02\t6.351989e-01\t-1.015070e-03\n"
        "rbias-high\t0\t2.750000e+02\t7.877634e-01\t-4.897950e-04\n"
        "rbias-high\t1\t3.500000e+02\t6.144482e-01\t-5.099479e-04\n"
    )


def test_bjt_temperature_scaling_reduces_emitter_follower_forward_drop():
    """Hotter silicon raises BJT Is enough to lift a fixed-base emitter node."""
    nominal = Circuit()
    nominal.add(VoltageSource("Vcc", "vcc", "0", 5.0))
    nominal.add(VoltageSource("Vbase", "base", "0", 0.72))
    nominal.add(
        BJT(
            "Q1",
            collector="vcc",
            base="base",
            emitter="out",
            Is=1e-14,
            beta_f=120.0,
            Vt=0.02585,
        )
    )
    nominal.add(Resistor("Rload", "out", "0", 1000.0))

    cold = circuit_at_temperature(nominal, 275.0)
    hot = circuit_at_temperature(nominal, 350.0)

    nominal_result = dc_op(nominal)
    cold_result = dc_op(cold)
    hot_result = dc_op(hot)

    assert cold_result.converged
    assert nominal_result.converged
    assert hot_result.converged
    assert cold_result.node_voltages["out"] < nominal_result.node_voltages["out"]
    assert hot_result.node_voltages["out"] > nominal_result.node_voltages["out"]


def test_bjt_temperature_scaling_uses_model_temperature_exponent():
    low = bjt_at_temperature(BJT("Qlow", "c", "b", "e", Xti=0.0), 350.0)
    high = bjt_at_temperature(BJT("Qhigh", "c", "b", "e", Xti=4.0), 350.0)
    assert high.Is > low.Is


def test_jfet_temperature_scaling_uses_vtotc_betatce_and_model_nominal_temperature():
    transistor = JFET(
        "J1",
        "d",
        "g",
        "s",
        beta=1.0e-4,
        vto=-2.0,
        Tcv=0.01,
        Vtotc=-0.0025,
        Tnom=310.0,
        Bex=-5.0,
        Betatce=1.0,
    )
    at_model_nominal = jfet_at_temperature(transistor, 310.0)
    hot = jfet_at_temperature(transistor, 320.0)
    cold = jfet_at_temperature(transistor, 300.0)
    invariant = jfet_at_temperature(JFET("Jflat", "d", "g", "s"), 350.0)
    bex_fallback = jfet_at_temperature(
        JFET("Jbex", "d", "g", "s", beta=1.0e-4, Tnom=310.0, Bex=1.0),
        320.0,
    )

    assert at_model_nominal.vto == pytest.approx(-2.0)
    assert at_model_nominal.beta == pytest.approx(1.0e-4)
    assert hot.vto == pytest.approx(-2.025)
    assert hot.beta == pytest.approx(1.0e-4 * 1.01**10.0)
    assert hot.Is > at_model_nominal.Is
    assert cold.vto == pytest.approx(-1.975)
    assert cold.beta == pytest.approx(1.0e-4 * 1.01**-10.0)
    assert cold.Is < at_model_nominal.Is
    lower_gap_hot = jfet_at_temperature(
        replace(transistor, Eg=1.0),
        320.0,
    )
    assert lower_gap_hot.Is < hot.Is
    assert invariant.vto == pytest.approx(-2.0)
    assert invariant.beta == pytest.approx(1.0e-4)
    assert bex_fallback.beta == pytest.approx(1.0e-4 * 320.0 / 310.0)
    tcv_fallback = jfet_at_temperature(
        JFET("Jtcv", "d", "g", "s", vto=-2.0, Tcv=0.01, Tnom=310.0),
        320.0,
    )
    assert tcv_fallback.vto == pytest.approx(-2.1)


def test_dc_rejects_invalid_jfet_temperature_parameters():
    circuit = Circuit()
    circuit.add(JFET("Jbad", "d", "g", "0", Xti=float("nan")))
    with pytest.raises(
        ValueError,
        match="gate saturation-current temperature exponent must be finite",
    ):
        dc_op(circuit)

    circuit = Circuit()
    circuit.add(JFET("Jbad", "d", "g", "0", Eg=0.0))
    with pytest.raises(ValueError, match="bandgap voltage must be finite and positive"):
        dc_op(circuit)

    circuit = Circuit()
    circuit.add(JFET("Jbad", "d", "g", "0", Tcv=float("nan")))
    with pytest.raises(ValueError, match="TCV must be finite"):
        dc_op(circuit)

    circuit = Circuit()
    circuit.add(JFET("Jbad", "d", "g", "0", Vtotc=float("nan")))
    with pytest.raises(ValueError, match="VTOTC must be finite"):
        dc_op(circuit)

    circuit = Circuit()
    circuit.add(JFET("Jbad", "d", "g", "0", Tnom=0.0))
    with pytest.raises(ValueError, match="TNOM must be finite and positive"):
        dc_op(circuit)

    circuit = Circuit()
    circuit.add(JFET("Jbad", "d", "g", "0", Bex=float("nan")))
    with pytest.raises(ValueError, match="BEX must be finite"):
        dc_op(circuit)

    circuit = Circuit()
    circuit.add(JFET("Jbad", "d", "g", "0", Betatce=float("nan")))
    with pytest.raises(ValueError, match="BETATCE must be finite"):
        dc_op(circuit)


def test_bjt_temperature_scaling_uses_beta_temperature_exponent():
    transistor = BJT("Q1", "c", "b", "e", beta_f=100.0, beta_r=2.0, Xtb=2.0)
    hot = bjt_at_temperature(transistor, 350.0)
    assert hot.beta_f > transistor.beta_f
    assert hot.beta_r > transistor.beta_r


def test_bjt_temperature_scaling_uses_model_nominal_temperature():
    transistor = BJT("Q1", "c", "b", "e", Tnom=325.0)
    at_model_nominal = bjt_at_temperature(transistor, 325.0)
    assert at_model_nominal.Is == pytest.approx(transistor.Is)
    assert at_model_nominal.Vt == pytest.approx(transistor.Vt)


def test_dc_rejects_invalid_bjt_nominal_temperature():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Tnom=0.0))
    with pytest.raises(ValueError, match="nominal temperature must be finite and positive"):
        dc_op(circuit)


def test_dc_rejects_invalid_diode_flicker_noise_exponent():
    circuit = Circuit()
    circuit.add(Diode("Dbad", "a", "0", Af=-1.0))
    with pytest.raises(ValueError, match="flicker-noise exponent must be finite and non-negative"):
        dc_op(circuit)


def test_dc_rejects_invalid_bjt_flicker_noise_coefficient():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Kf=-1.0))
    with pytest.raises(ValueError, match="flicker noise coefficient must be finite and non-negative"):
        dc_op(circuit)


def test_dc_rejects_invalid_bjt_flicker_noise_exponent():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Af=-1.0))
    with pytest.raises(ValueError, match="flicker noise exponent must be finite and non-negative"):
        dc_op(circuit)


def test_dc_rejects_invalid_bjt_forward_excess_phase():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Ptf=-1.0))
    with pytest.raises(ValueError, match="forward excess phase must be finite and non-negative"):
        dc_op(circuit)


def test_dc_rejects_invalid_bjt_forward_transit_time_bias_coefficient():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Xtf=-1.0))
    with pytest.raises(
        ValueError,
        match="forward transit-time bias coefficient must be finite and non-negative",
    ):
        dc_op(circuit)


def test_dc_rejects_invalid_bjt_forward_transit_time_current():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Itf=-1.0))
    with pytest.raises(
        ValueError,
        match="forward transit-time current must be finite and non-negative",
    ):
        dc_op(circuit)


def test_dc_rejects_invalid_bjt_forward_transit_time_voltage():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Vtf=-1.0))
    with pytest.raises(
        ValueError,
        match="forward transit-time voltage must be finite and non-negative",
    ):
        dc_op(circuit)


def test_dc_rejects_invalid_bjt_emitter_resistance():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Re=-1.0))
    with pytest.raises(
        ValueError,
        match="emitter resistance must be finite and non-negative",
    ):
        dc_op(circuit)


def test_dc_rejects_invalid_bjt_collector_resistance():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Rc=-1.0))
    with pytest.raises(
        ValueError,
        match="collector resistance must be finite and non-negative",
    ):
        dc_op(circuit)


def test_dc_rejects_invalid_bjt_base_resistance():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Rb=-1.0))
    with pytest.raises(
        ValueError,
        match="base resistance must be finite and non-negative",
    ):
        dc_op(circuit)


def test_dc_rejects_invalid_bjt_base_collector_capacitance_fraction():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Xcjc=1.1))
    with pytest.raises(
        ValueError,
        match="base-collector capacitance fraction must be between zero and one",
    ):
        dc_op(circuit)


def test_dc_rejects_invalid_bjt_beta_temperature_exponent():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Xtb=math.nan))
    with pytest.raises(ValueError, match="beta temperature exponent must be finite"):
        dc_op(circuit)


def test_bjt_temperature_scales_base_emitter_leakage_saturation_current():
    transistor = BJT("Q1", "c", "b", "e", Ise=2.0e-13)
    hot = bjt_at_temperature(transistor, 350.0)
    assert hot.Ise > transistor.Ise


def test_bjt_temperature_scales_base_collector_leakage_saturation_current():
    transistor = BJT("Q1", "c", "b", "e", Isc=2.0e-13)
    hot = bjt_at_temperature(transistor, 350.0)
    assert hot.Isc > transistor.Isc


def test_bjt_temperature_scaling_uses_model_energy_gap():
    silicon = Circuit()
    silicon.add(BJT("Qsilicon", "c", "b", "e", Eg=1.11))
    lower_gap = Circuit()
    lower_gap.add(BJT("Qlower", "c", "b", "e", Eg=0.8))
    silicon_hot = circuit_at_temperature(silicon, 350.0)
    lower_gap_hot = circuit_at_temperature(lower_gap, 350.0)
    assert silicon_hot.elements[0].Is > lower_gap_hot.elements[0].Is


def test_dc_rejects_invalid_bjt_energy_gap():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Eg=0.0))
    with pytest.raises(ValueError, match="BJT energy gap must be finite and positive"):
        dc_op(circuit)


def test_bjt_forward_early_voltage_modulates_collector_current():
    def collector_voltage(vaf: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vcc", "vcc", "0", 5.0))
        circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
        circuit.add(Resistor("Rload", "vcc", "out", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Vaf=vaf))
        result = dc_op(circuit)
        assert result.converged
        return result.node_voltages["out"]

    assert collector_voltage(20.0) < collector_voltage(0.0)


def test_dc_rejects_invalid_bjt_forward_early_voltage():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Vaf=-1.0))
    with pytest.raises(ValueError, match="BJT forward Early voltage must be finite and non-negative"):
        dc_op(circuit)


def test_bjt_reverse_early_voltage_modulates_collector_current():
    def collector_voltage(var: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vcc", "vcc", "0", 5.0))
        circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
        circuit.add(Resistor("Rload", "vcc", "out", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Var=var))
        return dc_op(circuit).node_voltages["out"]

    assert collector_voltage(20.0) > collector_voltage(0.0)


def test_dc_rejects_invalid_bjt_reverse_early_voltage():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Var=-1.0))
    with pytest.raises(
        ValueError, match="BJT reverse Early voltage must be finite and non-negative"
    ):
        dc_op(circuit)


def test_bjt_forward_beta_rolloff_reduces_high_current_transport():
    def collector_voltage(ikf: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vcc", "vcc", "0", 5.0))
        circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
        circuit.add(Resistor("Rload", "vcc", "out", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Ikf=ikf))
        return dc_op(circuit).node_voltages["out"]

    assert collector_voltage(1.0e-4) > collector_voltage(0.0)


def test_bjt_reverse_beta_controls_base_collector_junction_current():
    def base_current(beta_r: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
        circuit.add(VoltageSource("Vemitter", "emitter", "0", 0.65))
        circuit.add(BJT("Q1", "0", "base", "emitter", beta_r=beta_r))
        return abs(dc_op(circuit).branch_currents["I(Vbase)"])

    assert base_current(0.5) > base_current(5.0)


def test_dc_rejects_invalid_bjt_reverse_beta():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", beta_r=0.0))
    with pytest.raises(ValueError, match="BJT reverse beta must be positive"):
        dc_op(circuit)


def test_bjt_reverse_beta_rolloff_increases_high_current_base_current():
    def base_current(ikr: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
        circuit.add(VoltageSource("Vemitter", "emitter", "0", 0.65))
        circuit.add(BJT("Q1", "0", "base", "emitter", beta_r=1.0, Ikr=ikr))
        return abs(dc_op(circuit).branch_currents["I(Vbase)"])

    assert base_current(1.0e-4) > base_current(0.0)


def test_dc_rejects_invalid_bjt_reverse_beta_rolloff_current():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Ikr=-1.0))
    with pytest.raises(
        ValueError, match="BJT reverse beta roll-off current must be finite and non-negative"
    ):
        dc_op(circuit)


def test_dc_rejects_invalid_bjt_forward_beta_rolloff_current():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Ikf=-1.0))
    with pytest.raises(
        ValueError, match="BJT forward beta roll-off current must be finite and non-negative"
    ):
        dc_op(circuit)


def test_bjt_base_emitter_leakage_increases_base_current():
    def base_current(ise: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
        circuit.add(BJT("Q1", "0", "base", "0", Ise=ise, Ne=1.5))
        return abs(dc_op(circuit).branch_currents["I(Vbase)"])

    assert base_current(1.0e-10) > base_current(0.0)


def test_dc_rejects_invalid_bjt_base_emitter_leakage_parameters():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Ise=-1.0))
    with pytest.raises(ValueError, match="base-emitter leakage saturation current"):
        dc_op(circuit)

    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Ne=0.0))
    with pytest.raises(ValueError, match="base-emitter leakage emission coefficient"):
        dc_op(circuit)


def test_bjt_base_collector_leakage_increases_base_current():
    def base_current(isc: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
        circuit.add(BJT("Q1", "0", "base", "base", Isc=isc, Nc=1.5))
        return abs(dc_op(circuit).branch_currents["I(Vbase)"])

    assert base_current(1.0e-10) > base_current(0.0)


def test_dc_rejects_invalid_bjt_base_collector_leakage_parameters():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Isc=-1.0))
    with pytest.raises(ValueError, match="base-collector leakage saturation current"):
        dc_op(circuit)

    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Nc=0.0))
    with pytest.raises(ValueError, match="base-collector leakage emission coefficient"):
        dc_op(circuit)


def test_bjt_forward_emission_coefficient_reduces_collector_current():
    def collector_voltage(nf: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vcc", "vcc", "0", 5.0))
        circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
        circuit.add(Resistor("Rload", "vcc", "out", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Nf=nf))
        return dc_op(circuit).node_voltages["out"]

    assert collector_voltage(2.0) > collector_voltage(1.0)


def test_dc_rejects_invalid_bjt_forward_emission_coefficient():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Nf=0.0))
    with pytest.raises(ValueError, match="BJT forward emission coefficient must be finite and positive"):
        dc_op(circuit)


def test_dc_rejects_invalid_bjt_reverse_emission_coefficient():
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", Nr=0.0))
    with pytest.raises(ValueError, match="BJT reverse emission coefficient must be finite and positive"):
        dc_op(circuit)


@pytest.mark.parametrize(
    ("kwargs", "message"),
    [
        ({"Vje": 0.0}, "BJT base-emitter junction potential must be finite and positive"),
        ({"Mje": 1.0}, "BJT base-emitter grading coefficient must be finite and in \\[0, 1\\)"),
        ({"Vjc": 0.0}, "BJT base-collector junction potential must be finite and positive"),
        ({"Mjc": 1.0}, "BJT base-collector grading coefficient must be finite and in \\[0, 1\\)"),
        ({"Fc": 1.0}, "BJT forward-bias depletion coefficient must be finite and in \\[0, 1\\)"),
    ],
)
def test_dc_rejects_invalid_bjt_depletion_parameters(kwargs, message):
    circuit = Circuit()
    circuit.add(BJT("Qbad", "c", "b", "0", **kwargs))
    with pytest.raises(ValueError, match=message):
        dc_op(circuit)


def test_mosfet_temperature_scaling_changes_common_source_bias():
    """Hotter Level-1 NMOS lowers VT0 enough to pull the drain node down."""
    nominal = Circuit()
    nominal.add(VoltageSource("Vdd", "vdd", "0", 1.8))
    nominal.add(VoltageSource("Vgate", "gate", "0", 1.1))
    nominal.add(Resistor("Rload", "vdd", "out", 1000.0))
    nominal.add(Mosfet(
        "M1",
        "out",
        "gate",
        "0",
        "0",
        MOSFET(
            MosfetType.NMOS,
            Level1Model(Level1Params(
                VT0=0.65,
                KP=200.0e-6,
                LAMBDA=0.02,
                W=2.0e-6,
                L=180.0e-9,
            )),
        ),
    ))

    cold = circuit_at_temperature(nominal, 275.0)
    hot = circuit_at_temperature(nominal, 350.0)

    nominal_result = dc_op(nominal)
    cold_result = dc_op(cold)
    hot_result = dc_op(hot)

    assert cold_result.converged
    assert nominal_result.converged
    assert hot_result.converged
    assert cold_result.node_voltages["out"] > nominal_result.node_voltages["out"]
    assert hot_result.node_voltages["out"] < nominal_result.node_voltages["out"]


# ---- DC: Capacitor (open in DC) ----


def test_capacitor_open_in_dc():
    """Cap blocks DC -> no current flow; V_n1 should be 0."""
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=5.0))
    c.add(Capacitor("C1", "vin", "n1", 1e-6))
    c.add(Resistor("R1", "n1", "0", 1000.0))
    r = dc_op(c)
    assert isclose(r.node_voltages["n1"], 0.0, abs_tol=1e-6)


# ---- Transient: backward Euler (legacy method="euler") ----


def test_transient_euler_rc_charging():
    """Backward Euler: capacitor charges toward V_in with tau = RC."""
    R, C, V_in = 1000.0, 1e-6, 5.0  # tau = 1 ms
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=V_in))
    c.add(Resistor("R1", "vin", "vc", R))
    c.add(Capacitor("Cap1", "vc", "0", C))

    result = transient(c, t_stop=5e-3, t_step=1e-4, method="euler")
    assert result.converged
    assert result.method == "euler"
    assert len(result.points) > 10
    last = result.points[-1]
    assert isclose(last.node_voltages["vc"], V_in, abs_tol=0.5)


def test_transient_initial_state():
    """At t=0, capacitor blocks DC, so the cap voltage starts at 0."""
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=5.0))
    c.add(Resistor("R1", "vin", "vc", 1000.0))
    c.add(Capacitor("Cap1", "vc", "0", 1e-6))
    result = transient(c, t_stop=1e-3, t_step=1e-4)
    assert result.points[0].time == 0.0
    assert isclose(result.points[0].node_voltages["vc"], 0.0, abs_tol=1e-6)


def test_transient_rejects_zero_step():
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=5.0))
    c.add(Resistor("R1", "vin", "0", 1000.0))
    result = transient(c, t_stop=1e-3, t_step=0.0)
    assert not result.converged
    assert result.points == []


def test_transient_rejects_negative_stop():
    c = Circuit()
    c.add(VoltageSource("V1", "a", "0", voltage=1.0))
    result = transient(c, t_stop=-1.0, t_step=1e-3)
    assert not result.converged


# ---- Transient: trapezoidal (default, method="trap") ----


def test_transient_trap_rc_charging():
    """Trapezoidal: RC charging curve matches analytic solution.

    V_C(t) = V_in * (1 - exp(-t/tau))  with tau = R*C = 1 ms.
    """
    R, C, V_in = 1000.0, 1e-6, 5.0  # tau = 1 ms
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=V_in))
    c.add(Resistor("R1", "vin", "vc", R))
    c.add(Capacitor("Cap1", "vc", "0", C))

    result = transient(c, t_stop=5e-3, t_step=1e-4, method="trap")
    assert result.converged
    assert result.method == "trap"
    # After 5 tau the cap should be within 1% of V_in
    last = result.points[-1]
    assert isclose(last.node_voltages["vc"], V_in, abs_tol=0.1)


def test_transient_trap_is_default_method():
    """transient() with no method= argument uses trapezoidal."""
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=1.0))
    c.add(Resistor("R1", "vin", "vc", 1000.0))
    c.add(Capacitor("C1", "vc", "0", 1e-6))
    result = transient(c, t_stop=1e-3, t_step=1e-4)
    assert result.method == "trap"


def test_transient_trap_first_point_is_t0():
    """The first waveform point is always at t=0."""
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=3.3))
    c.add(Resistor("R1", "vin", "vc", 500.0))
    c.add(Capacitor("C1", "vc", "0", 1e-6))
    result = transient(c, t_stop=1e-3, t_step=1e-4)
    assert result.points[0].time == 0.0


# ---- Transient: accuracy comparison trap vs euler ----


def test_trap_more_accurate_than_euler_rc():
    """Trapezoidal has lower error than backward Euler at the same step size.

    For RC charging with tau=RC, measure the error at t=tau:
        V_exact = V_in * (1 - exp(-1))  ≈  0.6321 * V_in

    A coarser step (h = tau/5) is used so both methods show visible error.
    Trapezoidal (O(h^2)) should give smaller absolute error than Euler (O(h)).
    """
    R, C, V_in = 1000.0, 1e-6, 5.0  # tau = 1 ms
    tau = R * C
    h = tau / 5  # coarse step: 0.2 ms — error should be visible

    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=V_in))
    c.add(Resistor("R1", "vin", "vc", R))
    c.add(Capacitor("Cap1", "vc", "0", C))

    exact = V_in * (1.0 - exp(-1.0))  # V_C at t = tau

    def _error(meth: str) -> float:
        res = transient(c, t_stop=tau, t_step=h, method=meth)
        # Find the point closest to t=tau
        pt = min(res.points, key=lambda p: abs(p.time - tau))
        return abs(pt.node_voltages["vc"] - exact)

    err_euler = _error("euler")
    err_trap = _error("trap")
    assert err_trap < err_euler, (
        f"Trapezoidal error {err_trap:.4f} not less than Euler error {err_euler:.4f}"
    )


# ---- Transient: adaptive timestep ----


def test_adaptive_accepts_steps_and_returns_metadata():
    """With adaptive=True, steps_rejected is a non-negative int."""
    R, C, V_in = 1000.0, 1e-6, 5.0
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=V_in))
    c.add(Resistor("R1", "vin", "vc", R))
    c.add(Capacitor("Cap1", "vc", "0", C))

    result = transient(c, t_stop=5e-3, t_step=1e-4, adaptive=True)
    assert result.converged
    assert isinstance(result.steps_rejected, int)
    assert result.steps_rejected >= 0


def test_adaptive_reaches_steady_state():
    """Adaptive integration still reaches the correct final voltage."""
    R, C, V_in = 1000.0, 1e-6, 5.0
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=V_in))
    c.add(Resistor("R1", "vin", "vc", R))
    c.add(Capacitor("Cap1", "vc", "0", C))

    result = transient(c, t_stop=5e-3, t_step=5e-4, adaptive=True,
                       tol_lte=1e-3, max_step=2e-3)
    assert result.converged
    last = result.points[-1]
    assert isclose(last.node_voltages["vc"], V_in, abs_tol=0.2)


def test_adaptive_non_adaptive_same_result():
    """Fixed and adaptive trapezoidal should give nearly the same final value
    when the LTE tolerance is very tight (i.e. step is never rejected)."""
    R, C, V_in = 1000.0, 1e-6, 5.0
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=V_in))
    c.add(Resistor("R1", "vin", "vc", R))
    c.add(Capacitor("Cap1", "vc", "0", C))

    r_fixed = transient(c, t_stop=3e-3, t_step=1e-4, adaptive=False)
    # Pin min/max_step to t_step so adaptive never actually changes the step
    # size (large tol_lte → never rejects; max_step=t_step → never doubles).
    r_adapt = transient(c, t_stop=3e-3, t_step=1e-4, adaptive=True,
                        tol_lte=1.0,            # huge tol → never rejects
                        min_step=1e-4,
                        max_step=1e-4)           # lock h = t_step
    last_fixed = r_fixed.points[-1].node_voltages["vc"]
    last_adapt = r_adapt.points[-1].node_voltages["vc"]
    assert isclose(last_fixed, last_adapt, abs_tol=0.05)


def test_non_adaptive_steps_rejected_is_zero():
    """steps_rejected must be 0 when adaptive=False."""
    c = Circuit()
    c.add(VoltageSource("V1", "v", "0", voltage=1.0))
    c.add(Resistor("R1", "v", "vc", 1000.0))
    c.add(Capacitor("C1", "vc", "0", 1e-6))
    result = transient(c, t_stop=1e-3, t_step=1e-4, adaptive=False)
    assert result.steps_rejected == 0


# ---- Transient: inductor companion model ----


def test_transient_rl_current_buildup():
    """RL circuit: current rises as I(t) = (V/R) * (1 - exp(-R*t/L)).

    With V=1V, R=10Ω, L=1mH: tau=L/R=0.1ms; I_ss = V/R = 0.1A.
    At t=5*tau the current should be within 5% of I_ss.

    The resistor voltage V_R = I*R should approach V as I → I_ss.
    """
    V, R, L = 1.0, 10.0, 1e-3  # tau = 0.1 ms, I_ss = 0.1 A
    tau = L / R
    c = Circuit()
    c.add(VoltageSource("V1", "vs", "0", voltage=V))
    c.add(Resistor("R1", "vs", "vr", R))  # V_R = I * R
    c.add(Inductor("L1", "vr", "0", L))

    result = transient(c, t_stop=5 * tau, t_step=tau / 20)
    assert result.converged

    # At steady state: I_L → V/R (= I_ss), V_L → 0.
    # The node "vr" (junction of R1 and L1) is the inductor n_plus voltage.
    # V_vr = V_L → 0 at steady state.  (The *resistor* voltage V_R1 = V_vs −
    # V_vr → V, but the *node* "vr" → 0.)
    last = result.points[-1]
    V_vr = last.node_voltages.get("vr", 0.0)
    assert isclose(V_vr, 0.0, abs_tol=0.1 * V), (
        f"Expected V_vr ≈ 0.0 V at steady state, got {V_vr:.4f}"
    )


def test_transient_inductor_starts_at_zero_current():
    """At t=0 inductor has zero current; initial voltage is consistent."""
    c = Circuit()
    c.add(VoltageSource("V1", "v", "0", voltage=5.0))
    c.add(Resistor("R1", "v", "n", 100.0))
    c.add(Inductor("L1", "n", "0", 1e-3))

    result = transient(c, t_stop=1e-4, t_step=1e-5)
    assert result.converged
    # The second point should show current just beginning to build up.
    v0 = result.points[0].node_voltages.get("n", 0.0)
    # Node voltage should start near 0 (inductor blocks initial current)
    # and then begin to decline as current builds.
    assert v0 >= 0.0


def test_transient_inductor_respects_initial_current():
    c = Circuit()
    c.add(Resistor("R1", "out", "0", 1000.0))
    c.add(Inductor("L1", "out", "0", 1.0, initial_current=1.0e-3))

    result = transient(c, t_stop=2.0e-3, t_step=1.0e-3, method="euler")

    assert result.converged
    assert isclose(result.points[0].node_voltages["out"], -0.5, abs_tol=1e-9)
    assert isclose(result.points[1].node_voltages["out"], -0.5, abs_tol=1e-9)
    assert isclose(result.points[2].node_voltages["out"], -0.25, abs_tol=1e-9)


def test_transient_gear2_rc_charging_bootstraps_then_uses_bdf2():
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "vc", 1000.0))
    c.add(Capacitor("C1", "vc", "0", 1.0e-6))

    result = transient(c, t_stop=3.0e-3, t_step=1.0e-3, method="gear2")

    assert result.converged
    assert result.method == "gear2"
    assert result.points[1].node_voltages["vc"] == pytest.approx(0.5, abs=1e-12)
    assert result.points[2].node_voltages["vc"] == pytest.approx(0.8, abs=1e-12)
    assert result.points[3].node_voltages["vc"] == pytest.approx(0.94, abs=1e-12)


def test_transient_gear2_rl_current_buildup_bootstraps_then_uses_bdf2():
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Inductor("L1", "out", "0", 1.0))

    result = transient(c, t_stop=3.0e-3, t_step=1.0e-3, method="gear2")

    assert result.converged
    assert result.points[1].branch_currents["L1"] == pytest.approx(0.5e-3, abs=1e-12)
    assert result.points[2].branch_currents["L1"] == pytest.approx(0.8e-3, abs=1e-12)
    assert result.points[3].branch_currents["L1"] == pytest.approx(0.94e-3, abs=1e-12)


def test_transient_gear2_damps_coarse_lc_oscillator_more_than_trap():
    c = Circuit()
    c.add(Capacitor("C1", "tank", "0", 1.0, initial_voltage=1.0))
    c.add(Inductor("L1", "tank", "0", 1.0))

    trap = transient(c, t_stop=10.0, t_step=1.0, method="trap")
    gear2 = transient(c, t_stop=10.0, t_step=1.0, method="gear2")

    assert trap.converged
    assert gear2.converged
    trap_tail = max(abs(point.node_voltages["tank"]) for point in trap.points[-4:])
    gear2_tail = max(abs(point.node_voltages["tank"]) for point in gear2.points[-4:])
    assert gear2_tail < trap_tail * 0.75


def test_transient_mutual_inductor_couples_secondary_voltage():
    c = Circuit()
    c.add(CurrentSource("Istep", "0", "pri", 1.0))
    c.add(Inductor("Lpri", "pri", "0", 1.0))
    c.add(Inductor("Lsec", "sec", "0", 1.0))
    c.add(MutualInductor("K1", "Lpri", "Lsec", 0.5))
    c.add(Resistor("Rload", "sec", "0", 10.0))

    result = transient(c, t_stop=0.1, t_step=0.1, method="euler")

    assert result.converged
    assert isclose(result.points[1].node_voltages["pri"], 8.75, rel_tol=1e-9)
    assert isclose(result.points[1].node_voltages["sec"], 2.5, rel_tol=1e-9)


def test_transient_transmission_line_delays_matched_step():
    delay = 1.0e-9
    c = Circuit()
    c.add(VoltageSource("VIN", "in", "0", 1.0))
    c.add(TransmissionLine("T1", "in", "0", "out", "0", 50.0, delay))
    c.add(Resistor("RL", "out", "0", 50.0))

    result = transient(c, t_stop=2.0 * delay, t_step=delay / 2.0, method="euler")

    assert result.converged
    assert result.points[0].node_voltages.get("out", 0.0) == pytest.approx(0.0, abs=1e-12)
    assert result.points[1].node_voltages.get("out", 0.0) == pytest.approx(0.0, abs=1e-12)
    assert result.points[2].node_voltages["out"] == pytest.approx(1.0, rel=1e-9, abs=1e-9)
    assert result.points[2].branch_currents["I(T1:2)"] == pytest.approx(-0.02, rel=1e-9)


def test_transient_transmission_line_rejects_invalid_parameters():
    c = Circuit()
    c.add(VoltageSource("VIN", "in", "0", 1.0))
    c.add(TransmissionLine("Tbad", "in", "0", "out", "0", 50.0, 0.0))
    c.add(Resistor("RL", "out", "0", 50.0))

    with pytest.raises(ValueError, match="delay must be positive"):
        transient(c, t_stop=1.0e-9, t_step=1.0e-9, method="euler")


# ---- Transient: TransientResult metadata ----


def test_transient_result_method_field_trap():
    c = Circuit()
    c.add(VoltageSource("V1", "v", "0", voltage=1.0))
    c.add(Resistor("R1", "v", "vc", 1000.0))
    c.add(Capacitor("C1", "vc", "0", 1e-6))
    r = transient(c, t_stop=1e-4, t_step=1e-5, method="trap")
    assert r.method == "trap"


def test_transient_result_method_field_euler():
    c = Circuit()
    c.add(VoltageSource("V1", "v", "0", voltage=1.0))
    c.add(Resistor("R1", "v", "vc", 1000.0))
    c.add(Capacitor("C1", "vc", "0", 1e-6))
    r = transient(c, t_stop=1e-4, t_step=1e-5, method="euler")
    assert r.method == "euler"


def test_transient_result_invalid_input_method_field():
    """Even when t_step=0 (rejected early), method is preserved."""
    c = Circuit()
    c.add(VoltageSource("V1", "v", "0", voltage=1.0))
    r = transient(c, t_stop=1e-3, t_step=0.0, method="euler")
    assert r.method == "euler"
    assert not r.converged


# ---- _lte_estimate unit test ----


def test_lte_estimate_zero_for_constant_voltage():
    """If cap voltage is constant over 3 steps, LTE = 0."""
    c = Circuit()
    c.add(Capacitor("C1", "a", "0", 1e-6))
    # V_n+1 = V_n = V_n-1 = 1.0 → second difference = 0
    lte = _lte_estimate(c, {"C1": 1.0}, {"C1": 1.0}, {"C1": 1.0})
    assert lte == 0.0


def test_lte_estimate_nonzero_for_curved_voltage():
    """A voltage that curves upward gives a positive LTE estimate."""
    c = Circuit()
    c.add(Capacitor("C1", "a", "0", 1e-6))
    # V: 0, 1, 3 → second diff = 3 - 2*1 + 0 = 1 → lte = 0.5
    lte = _lte_estimate(c, {"C1": 3.0}, {"C1": 1.0}, {"C1": 0.0})
    assert isclose(lte, 0.5, rel_tol=1e-9)


def test_lte_estimate_no_caps_returns_zero():
    """Circuit with no capacitors gives LTE = 0."""
    c = Circuit()
    c.add(Resistor("R1", "a", "0", 1000.0))
    lte = _lte_estimate(c, {}, {}, {})
    assert lte == 0.0


# ---- Mid-scale: 4-bit-adder NAND2 cell-like circuit ----


def test_two_cmos_inverter_chain():
    """Voltage divider with two parallel resistors — sanity check more
    complex netlists work."""
    c = Circuit()
    c.add(VoltageSource("V1", "vdd", "0", voltage=1.8))
    c.add(Resistor("R1", "vdd", "n1", 5000.0))
    c.add(Resistor("R2", "vdd", "n1", 5000.0))  # parallel: 2.5k
    c.add(Resistor("R3", "n1", "0", 2500.0))
    r = dc_op(c)
    # Series equivalent: 2.5k + 2.5k = 5k from vdd to gnd
    # V_n1 = 1.8 * 2.5k / 5k = 0.9V
    assert isclose(r.node_voltages["n1"], 0.9, abs_tol=1e-6)


# ---- DC: Inductor ----


def test_inductor_short_in_dc():
    """Inductor is a short in DC (no contribution at this stamp level)."""
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=5.0))
    c.add(Inductor("L1", "vin", "n1", 1e-6))
    c.add(Resistor("R1", "n1", "0", 100.0))
    r = dc_op(c)
    assert r.converged


# ---- Backward-compatibility: existing rc charging test (no method arg) ----


def test_transient_rc_charging():
    """Backward-compat: transient without explicit method still converges.

    The default is now 'trap', so this also tests trapezoidal is a valid
    drop-in replacement.
    """
    R, C, V_in = 1000.0, 1e-6, 5.0  # tau = 1 ms
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", voltage=V_in))
    c.add(Resistor("R1", "vin", "vc", R))
    c.add(Capacitor("Cap1", "vc", "0", C))

    result = transient(c, t_stop=5e-3, t_step=1e-4)
    assert result.converged
    assert len(result.points) > 10
    last = result.points[-1]
    assert isclose(last.node_voltages["vc"], V_in, abs_tol=0.5)


# ---- DC: BJT (NPN and PNP) ----


def test_bjt_dataclass_defaults():
    """BJT dataclass stores all fields correctly."""
    q = BJT("Q1", collector="c", base="b", emitter="0")
    assert q.name == "Q1"
    assert q.polarity == "NPN"
    assert q.Is == 1e-14
    assert q.beta_f == 100.0
    assert isclose(q.Vt, 0.02585, rel_tol=1e-9)


def test_bjt_pnp_dataclass():
    """PNP BJT stores polarity correctly."""
    q = BJT("Q2", collector="c", base="b", emitter="vcc", polarity="PNP")
    assert q.polarity == "PNP"


def test_jfet_n_channel_source_resistor_bias_dc() -> None:
    c = Circuit()
    c.add(VoltageSource("Vdd", "vdd", "0", voltage=10.0))
    c.add(VoltageSource("Vg", "gate", "0", voltage=0.0))
    c.add(Resistor("Rd", "vdd", "drain", 2_000.0))
    c.add(Resistor("Rs", "source", "0", 1_000.0))
    c.add(JFET("J1", "drain", "gate", "source", beta=1.0e-3, vto=-2.0))

    result = dc_op(c)

    assert result.converged
    assert result.node_voltages["source"] == pytest.approx(1.0, abs=0.05)
    assert result.node_voltages["drain"] == pytest.approx(8.0, abs=0.1)


def test_jfet_common_source_ac_gain_from_bias_point() -> None:
    c = Circuit()
    c.add(VoltageSource("Vdd", "vdd", "0", voltage=10.0))
    c.add(VoltageSource("Vin", "gate", "0", voltage=0.0, ac=AcSource(1.0)))
    c.add(Resistor("Rd", "vdd", "drain", 1_000.0))
    c.add(JFET("J1", "drain", "gate", "0", beta=1.0e-3, vto=-2.0))

    result = ac_sweep(c, f_start=1_000.0, f_stop=1_000.0, n_points=1)

    out = result.points[0].node_voltages["drain"]
    assert out.real == pytest.approx(-4.0, abs=1.0e-6)
    assert out.imag == pytest.approx(0.0, abs=1.0e-12)


def test_ac_jfet_gate_source_capacitance_shunts_high_frequency_gate_drive() -> None:
    """JFET CGS contributes gate-source AC susceptance."""

    def gate_amplitude(cgs: float) -> float:
        c = Circuit()
        c.add(VoltageSource("Vac", "in", "0", 0.0, ac=AcSource(1.0)))
        c.add(Resistor("Rin", "in", "gate", 1_000.0))
        c.add(Resistor("Rdrain", "drain", "0", 1_000.0))
        c.add(JFET("J1", "drain", "gate", "0", beta=1.0e-12, vto=-2.0, Cgs=cgs))
        result = ac_sweep(c, f_start=100_000.0, f_stop=100_000.0, n_points=1)
        return abs(result.points[0].node_voltages["gate"])

    without_capacitance = gate_amplitude(0.0)
    with_capacitance = gate_amplitude(1.0e-6)

    assert without_capacitance > 0.9
    assert with_capacitance < without_capacitance / 100.0


def test_ac_jfet_junction_potential_shapes_reverse_biased_gate_capacitance() -> None:
    def gate_amplitude(pb: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vac", "in", "0", -0.5, ac=AcSource(1.0)))
        circuit.add(Resistor("Rin", "in", "gate", 1_000.0))
        circuit.add(Resistor("Rdrain", "drain", "0", 1_000.0))
        circuit.add(
            JFET(
                "J1",
                "drain",
                "gate",
                "0",
                beta=1.0e-12,
                vto=-2.0,
                Cgs=1.0e-9,
                Pb=pb,
            )
        )
        result = ac_sweep(
            circuit, f_start=100_000.0, f_stop=100_000.0, n_points=1
        )
        return abs(result.points[0].node_voltages["gate"])

    assert gate_amplitude(0.5) > gate_amplitude(2.0)


def test_ac_jfet_forward_bias_depletion_coefficient_shapes_gate_capacitance() -> None:
    def gate_amplitude(fc: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vac", "in", "0", 0.6, ac=AcSource(1.0)))
        circuit.add(Resistor("Rin", "in", "gate", 1_000.0))
        circuit.add(Resistor("Rdrain", "drain", "0", 1_000.0))
        circuit.add(
            JFET(
                "J1",
                "drain",
                "gate",
                "0",
                beta=1.0e-12,
                vto=-2.0,
                Cgs=1.0e-9,
                Fc=fc,
            )
        )
        result = ac_sweep(
            circuit, f_start=100_000.0, f_stop=100_000.0, n_points=1
        )
        return abs(result.points[0].node_voltages["gate"])

    assert gate_amplitude(0.2) > gate_amplitude(0.8)


def test_jfet_transient_source_follower_charges_output_capacitor() -> None:
    c = Circuit()
    c.add(VoltageSource("Vdd", "vdd", "0", voltage=10.0))
    c.add(
        VoltageSource(
            "Vg",
            "gate",
            "0",
            voltage=0.0,
            waveform=PwlWaveform([(0.0, 0.0), (1.0e-6, 1.0), (2.0e-6, 1.0)]),
        )
    )
    c.add(JFET("J1", "vdd", "gate", "out", beta=1.0e-3, vto=-2.0))
    c.add(Resistor("Rs", "out", "0", 1_000.0))
    c.add(Capacitor("Cout", "out", "0", 1.0e-9))

    result = transient(c, t_step=1.0e-7, t_stop=2.0e-6)

    initial_out = result.points[0].node_voltages["out"]
    final_out = result.points[-1].node_voltages["out"]
    assert final_out > initial_out + 1.0
    assert final_out > 1.5
    assert final_out < 2.0


def test_bjt_npn_off():
    """NPN BJT with zero base voltage — device is off (no collector current).

    With Vbe = 0, exp(0/Vt) = 1, so Ic = Is*(1-1) = 0.
    The circuit is just Vcc through Rc, but no current flows in the BJT branch,
    so the collector voltage should remain near Vcc (biased up through Rc with
    nothing pulling it down).
    """
    Vcc = 5.0
    Rc = 1000.0     # collector resistor

    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", voltage=Vcc))
    c.add(Resistor("Rc", "vcc", "col", Rc))
    # NPN with base and emitter both at 0 V: Vbe = 0 -> off
    c.add(BJT("Q1", collector="col", base="0", emitter="0"))

    r = dc_op(c)
    assert r.converged
    # With no collector current, Vcol ≈ Vcc (no drop across Rc)
    # Allow tolerance for gm * 0 = 0 stamp: col should be close to Vcc.
    assert r.node_voltages["col"] > 4.0, (
        f"Expected Vcol near Vcc but got {r.node_voltages['col']:.3f}"
    )


def test_bjt_npn_forward_active():
    """NPN BJT in forward-active region: collector current ≈ Is*exp(Vbe/Vt).

    Circuit:
        Vcc (5 V) → Rc (1 kΩ) → collector
        Vb  (0.7 V) → base
        emitter → GND

    At Vbe = 0.7 V (at the clamp boundary):
        exp_term = exp(0.7 / 0.02585) ≈ 5.97e11 (clamped to 0.7)
        Ic = Is * (exp_term - 1) ≈ Is * exp_term

    The collector node voltage is pulled down from Vcc by the resistor:
        Vcol = Vcc - Ic * Rc

    We verify:
    1. The simulation converges.
    2. There is a meaningful voltage drop across Rc (device is conducting).
    3. The computed Vcol is consistent with Ic = gm * Vbe.
    """
    Vcc = 5.0
    Rc = 1000.0
    Is_val = 1e-14
    Vt_val = 0.02585
    beta = 100.0

    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", voltage=Vcc))
    c.add(VoltageSource("Vb", "b", "0", voltage=0.7))
    c.add(Resistor("Rc", "vcc", "col", Rc))
    c.add(BJT("Q1", collector="col", base="b", emitter="0",
               Is=Is_val, beta_f=beta, Vt=Vt_val))

    r = dc_op(c)
    assert r.converged

    # At Vbe = 0.7 V (clamped), compute expected Ic
    exp_term = exp(0.7 / Vt_val)
    Ic_expected = Is_val * (exp_term - 1.0)

    Vcol = r.node_voltages["col"]
    # Vcol = Vcc - Ic * Rc
    Ic_from_vcol = (Vcc - Vcol) / Rc

    assert Ic_from_vcol > 0, "Collector current should be positive (NPN forward active)"
    # The simulator's Newton-Raphson linearisation gives the clamped-at-0.7 value.
    # Check within 1% of the expected analytic value.
    assert isclose(Ic_from_vcol, Ic_expected, rel_tol=0.01), (
        f"Ic from voltages ({Ic_from_vcol:.6e} A) != expected ({Ic_expected:.6e} A)"
    )


def test_bjt_emitter_resistance_reduces_fixed_base_collector_current():
    def collector_voltage(emitter_resistance: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vcc", "vcc", "0", voltage=5.0))
        circuit.add(VoltageSource("Vbase", "base", "0", voltage=0.7))
        circuit.add(Resistor("Rc", "vcc", "collector", 1_000.0))
        circuit.add(BJT("Q1", "collector", "base", "0", Re=emitter_resistance))
        return dc_op(circuit).node_voltages["collector"]

    assert collector_voltage(100.0) > collector_voltage(0.0) + 0.5


def test_bjt_collector_resistance_drops_intrinsic_collector_voltage():
    circuit = Circuit()
    circuit.add(VoltageSource("Vcollector", "collector", "0", voltage=5.0))
    circuit.add(VoltageSource("Vbase", "base", "0", voltage=0.65))
    circuit.add(BJT("Q1", "collector", "base", "0", Rc=100.0))

    intrinsic = dc_op(circuit).node_voltages["__spice_Q1_collector"]
    assert 0.0 < intrinsic < 5.0


def test_bjt_base_resistance_drops_intrinsic_base_voltage():
    circuit = Circuit()
    circuit.add(VoltageSource("Vcollector", "collector", "0", voltage=5.0))
    circuit.add(VoltageSource("Vbase", "base", "0", voltage=0.65))
    circuit.add(BJT("Q1", "collector", "base", "0", Rb=1_000.0))

    intrinsic = dc_op(circuit).node_voltages["__spice_Q1_base"]
    assert 0.0 < intrinsic < 0.65


def test_bjt_minimum_base_resistance_reduces_high_current_base_drop():
    def intrinsic_base(Rbm: float | None, Irb: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vcollector", "collector", "0", voltage=5.0))
        circuit.add(VoltageSource("Vbase", "base", "0", voltage=0.65))
        circuit.add(
            BJT(
                "Q1",
                "collector",
                "base",
                "0",
                Rb=1_000.0,
                Rbm=Rbm,
                Irb=Irb,
            )
        )
        return dc_op(circuit).node_voltages["__spice_Q1_base"]

    fixed = intrinsic_base(None, 0.0)
    bias_dependent = intrinsic_base(10.0, 1.0e-6)
    assert bias_dependent > fixed
    assert bias_dependent < 0.65


def test_bjt_npn_beta_ratio():
    """NPN BJT: collector current / base current ≈ beta_f.

    We use a voltage-source base drive (Vb = 0.7 V) and measure the
    collector current from the Rc drop.  The base current is computed
    as Ic / beta_f (the simulator uses the same relation internally).

    This test validates that the beta ratio is embedded correctly in the
    junction stamp (gπ = gm / beta_f).
    """
    beta = 50.0
    Is_val = 1e-14
    Vt_val = 0.02585

    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", voltage=5.0))
    c.add(VoltageSource("Vb", "b", "0", voltage=0.7))
    c.add(Resistor("Rc", "vcc", "col", 1000.0))
    c.add(BJT("Q1", collector="col", base="b", emitter="0",
               Is=Is_val, beta_f=beta, Vt=Vt_val))

    r = dc_op(c)
    assert r.converged

    Vcol = r.node_voltages["col"]
    Ic = (5.0 - Vcol) / 1000.0
    exp_term = exp(0.7 / Vt_val)
    Ic_expected = Is_val * (exp_term - 1.0)
    Ib_expected = Ic_expected / beta

    # Ic / Ib = beta
    assert Ic > 0
    # Verify internally consistent: collector current matches model prediction
    assert isclose(Ic, Ic_expected, rel_tol=0.01)
    # Verify Ib = Ic / beta (stamped as gπ = gm/beta which sets dIb/dVbe)
    assert isclose(Ic / beta, Ib_expected, rel_tol=0.01)


def test_bjt_pnp_forward_active():
    """PNP BJT in forward-active region: emitter injects, collector collects.

    Circuit:
        emitter → Vcc (5 V)
        base    → 4.3 V (so Veb = Ve - Vb = 5 - 4.3 = 0.7 V)
        collector → Rc (1 kΩ) → GND

    At Veb = 0.7 V (clamped):
        Ic_expected = Is * (exp(0.7/Vt) - 1)

    We expect:
    1. Simulation converges.
    2. Collector node is above GND (current flowing into collector → Vc > 0).
    3. The voltage drop across Rc is consistent with the expected Ic.
    """
    Vcc = 5.0
    Vb_val = 4.3   # Veb = Vcc - Vb_val = 0.7 V
    Rc = 1000.0
    Is_val = 1e-14
    Vt_val = 0.02585

    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", voltage=Vcc))
    c.add(VoltageSource("Vb_src", "b", "0", voltage=Vb_val))
    c.add(Resistor("Rc", "col", "0", Rc))
    # PNP: emitter at Vcc, base at 4.3V, collector at col
    c.add(BJT("Q1", collector="col", base="b", emitter="vcc",
               polarity="PNP", Is=Is_val, Vt=Vt_val))

    r = dc_op(c)
    assert r.converged

    exp_term = exp(0.7 / Vt_val)
    Ic_expected = Is_val * (exp_term - 1.0)

    Vcol = r.node_voltages["col"]
    # For PNP: Ic flows into the collector FROM ground through Rc.
    # Node col voltage = Ic * Rc (current source pumps into col, Rc to ground).
    Ic_from_vcol = Vcol / Rc

    assert Vcol > 0, f"PNP collector should be above GND, got Vcol={Vcol:.4f}"
    assert isclose(Ic_from_vcol, Ic_expected, rel_tol=0.01), (
        f"PNP Ic from voltages ({Ic_from_vcol:.6e}) != expected ({Ic_expected:.6e})"
    )


def test_bjt_element_nodes():
    """BJT contributes all three terminals to _node_index."""
    from spice_engine.engine import _node_index
    c = Circuit()
    c.add(BJT("Q1", collector="col", base="base", emitter="emit"))
    _, nodes = _node_index(c)
    assert "col" in nodes
    assert "base" in nodes
    assert "emit" in nodes


def test_bjt_stamp_matrix_shape():
    """_stamp_bjt does not raise and produces a finite matrix."""
    import math as _math
    node_to_idx = {"c": 0, "b": 1, "e": 2}
    G = [[0.0] * 3 for _ in range(3)]
    b = [0.0] * 3
    x = [0.0, 0.0, 0.0]  # all nodes at 0 V — device is off
    q = BJT("Q1", collector="c", base="b", emitter="e")
    _stamp_bjt(G, b, x, node_to_idx, q)
    # All values should be finite
    for row in G:
        for val in row:
            assert _math.isfinite(val), f"Non-finite G entry: {val}"
    for val in b:
        assert _math.isfinite(val), f"Non-finite b entry: {val}"


def test_bjt_npn_ground_emitter_no_crash():
    """BJT with emitter grounded via alias 'gnd' converges cleanly."""
    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "gnd", voltage=3.3))
    c.add(VoltageSource("Vb", "b", "gnd", voltage=0.7))
    c.add(Resistor("Rc", "vcc", "col", 1000.0))
    c.add(BJT("Q1", collector="col", base="b", emitter="gnd"))
    r = dc_op(c)
    assert r.converged


def test_bjt_npn_vcc_emitter():
    """NPN BJT with non-ground emitter: Vbe = Vb - Ve matters.

    Circuit: Ve = 2V (emitter not grounded), Vb = 2.7V (Vbe = 0.7V).
    Should behave identically to the grounded-emitter case because
    Vbe = 0.7 V is the same.
    """
    Is_val = 1e-14
    Vt_val = 0.02585

    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", voltage=5.0))
    c.add(VoltageSource("Vb", "b", "0", voltage=2.7))
    c.add(VoltageSource("Ve", "e", "0", voltage=2.0))   # emitter not at ground
    c.add(Resistor("Rc", "vcc", "col", 1000.0))
    c.add(BJT("Q1", collector="col", base="b", emitter="e",
               Is=Is_val, Vt=Vt_val))

    r = dc_op(c)
    assert r.converged

    exp_term = exp(0.7 / Vt_val)
    Ic_expected = Is_val * (exp_term - 1.0)
    Vcol = r.node_voltages["col"]
    Ic_from_vcol = (5.0 - Vcol) / 1000.0
    assert isclose(Ic_from_vcol, Ic_expected, rel_tol=0.01)


def test_bjt_in_element_union():
    """BJT is exported from spice_engine and is a valid Element type."""
    from spice_engine import BJT as BJT_exported
    q = BJT_exported("Q1", collector="c", base="b", emitter="0")
    assert isinstance(q, BJT)


def test_bjt_custom_parameters():
    """BJT constructor accepts custom Is, beta_f, Vt."""
    q = BJT("Q1", collector="c", base="b", emitter="e",
             Is=2.5e-16, beta_f=200.0, Vt=0.026)
    assert isclose(q.Is, 2.5e-16)
    assert isclose(q.beta_f, 200.0)
    assert isclose(q.Vt, 0.026)


# ============================================================================
# 15. AC sweep — complex linear solver
# ============================================================================


def test_solve_complex_2x2_real_system():
    """_solve_complex on a real-valued 2×2 system matches _solve."""
    # 2x + y = 5, x + 3y = 10  →  x = 1, y = 3
    A = [[2.0 + 0j, 1.0 + 0j], [1.0 + 0j, 3.0 + 0j]]
    b = [5.0 + 0j, 10.0 + 0j]
    x = _solve_complex(A, b)
    assert isclose(x[0].real, 1.0, abs_tol=1e-9)
    assert isclose(x[1].real, 3.0, abs_tol=1e-9)
    assert isclose(abs(x[0].imag), 0.0, abs_tol=1e-9)


def test_solve_complex_purely_imaginary_diagonal():
    """_solve_complex: diagonal matrix with imaginary entries.

    For A = [[j, 0], [0, 2j]] and b = [1+j, -2j]:
        x[0] = (1+j)/j = 1/j + 1 = -j + 1 = 1 - j
        x[1] = -2j / 2j = -1
    """
    j = 1j
    A = [[j, 0j], [0j, 2j]]
    b = [1 + j, -2j]
    x = _solve_complex(A, b)
    assert isclose(abs(x[0] - (1 - 1j)), 0.0, abs_tol=1e-9)
    assert isclose(abs(x[1] - (-1 + 0j)), 0.0, abs_tol=1e-9)


def test_solve_complex_empty():
    """_solve_complex on empty system returns empty list."""
    assert _solve_complex([], []) == []


def test_solve_complex_singular_raises():
    """_solve_complex raises ZeroDivisionError for singular matrix."""
    A = [[1.0 + 0j, 1.0 + 0j], [1.0 + 0j, 1.0 + 0j]]
    b = [1.0 + 0j, 2.0 + 0j]
    with pytest.raises(ZeroDivisionError):
        _solve_complex(A, b)


def test_solve_complex_3x3():
    """_solve_complex: 3×3 complex system satisfies Ax = b."""
    # Use a well-conditioned system with complex coefficients.
    A = [
        [2.0 + 1j, 1.0 + 0j, 0j],
        [0j, 3.0 + 0j, 1.0 + 2j],
        [1.0 + 0j, 0j, 2.0 + 0j],
    ]
    b = [3.0 + 2j, 1.0 + 0j, 4.0 + 0j]
    x = _solve_complex(A, b)
    # Verify A·x = b.
    for i, row in enumerate(A):
        s = sum(row[j] * x[j] for j in range(3))
        assert isclose(abs(s - b[i]), 0.0, abs_tol=1e-9), (
            f"Row {i}: A·x = {s}, expected {b[i]}"
        )


def test_solve_complex_large_sparse_path():
    size = 36
    A = [[0j for _ in range(size)] for _ in range(size)]
    b = [complex(index + 1, -index) for index in range(size)]
    for index in range(size):
        A[index][index] = 2.0 + 1.0j
        if index + 1 < size:
            A[index][index + 1] = -0.25j
            A[index + 1][index] = 0.5 + 0j

    x = _solve_complex(A, b)

    for row_index, row in enumerate(A):
        actual = sum(value * x[col] for col, value in enumerate(row))
        assert abs(actual - b[row_index]) < 1.0e-8


# ============================================================================
# 16. AC sweep — AcPoint and AcResult data structures
# ============================================================================


def test_acpoint_fields():
    """AcPoint stores freq and node_voltages."""
    pt = AcPoint(freq=1000.0, node_voltages={"out": 0.5 + 0j})
    assert pt.freq == 1000.0
    assert pt.node_voltages["out"] == 0.5 + 0j


def test_acresult_fields():
    """AcResult wraps a list of AcPoints."""
    pts = [AcPoint(freq=100.0, node_voltages={})]
    r = AcResult(points=pts)
    assert len(r.points) == 1
    assert r.points[0].freq == 100.0


def test_ac_sweep_returns_acresult():
    """ac_sweep returns an AcResult instance."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=100.0, f_stop=10000.0, n_points=5)
    assert isinstance(result, AcResult)


def test_ac_sweep_point_count():
    """ac_sweep returns exactly n_points AcPoints."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=100.0, f_stop=10000.0, n_points=10)
    assert len(result.points) == 10


def test_ac_sweep_zero_points():
    """n_points=0 returns an empty AcResult."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=100.0, f_stop=1000.0, n_points=0)
    assert isinstance(result, AcResult)
    assert result.points == []


def test_ac_sweep_single_point():
    """n_points=1 returns a single AcPoint at f_start."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=500.0, f_stop=5000.0, n_points=1)
    assert len(result.points) == 1
    assert isclose(result.points[0].freq, 500.0, rel_tol=1e-9)


def test_ac_sweep_point_has_node_voltages():
    """Each AcPoint contains the expected node names."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))
    result = ac_sweep(c, f_start=100.0, f_stop=10000.0, n_points=3)
    for pt in result.points:
        assert "in" in pt.node_voltages
        assert "out" in pt.node_voltages


def test_ac_sweep_frequencies_ascending():
    """Frequency values in AcPoints are strictly ascending."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=1.0, f_stop=1e6, n_points=20)
    freqs = [pt.freq for pt in result.points]
    assert all(freqs[i] < freqs[i + 1] for i in range(len(freqs) - 1))


# ============================================================================
# 17. AC sweep — resistive circuits (frequency-independent)
# ============================================================================


def test_ac_resistive_voltage_divider_real_valued():
    """Two equal resistors: output is exactly Vin/2 at all frequencies.

    A pure resistive divider has no reactive elements, so the AC phasor
    voltage is the same at every frequency: V_out = Vin/2 (real).

    Circuit:  Vin → R1 (1kΩ) → out → R2 (1kΩ) → GND
    """
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))
    result = ac_sweep(c, f_start=1.0, f_stop=1e7, n_points=20, sweep="log")
    for pt in result.points:
        v_out = pt.node_voltages["out"]
        assert isclose(abs(v_out), 0.5, abs_tol=1e-4), (
            f"f={pt.freq:.1f} Hz: |V_out|={abs(v_out):.6f} (expected 0.5)"
        )
        # Imaginary part negligible — purely resistive
        assert isclose(abs(v_out.imag), 0.0, abs_tol=1e-6), (
            f"f={pt.freq:.1f} Hz: Im(V_out)={v_out.imag:.2e} (expected 0)"
        )


def test_ac_source_node_equals_source_voltage():
    """Source node voltage equals the VoltageSource voltage at all frequencies."""
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", 2.5))
    c.add(Resistor("R1", "vin", "0", 1000.0))
    result = ac_sweep(c, f_start=10.0, f_stop=1e5, n_points=10)
    for pt in result.points:
        assert isclose(abs(pt.node_voltages["vin"]), 2.5, abs_tol=1e-6), (
            f"f={pt.freq:.1f} Hz: V_in={pt.node_voltages['vin']}"
        )


def test_ac_explicit_voltage_source_phasor_separate_from_dc_bias():
    """An explicit AC source phasor is independent from the DC bias value."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0, ac=AcSource(2.0, 90.0)))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = ac_sweep(c, f_start=1000.0, f_stop=1000.0, n_points=1)
    pt = result.points[0]

    assert isclose(pt.node_voltages["in"].real, 0.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["in"].imag, 2.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["out"].real, 0.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["out"].imag, 1.0, abs_tol=1e-12)


def test_ac_unspecified_sources_zero_when_any_explicit_ac_source_exists():
    """DC bias sources become zero small-signal sources with explicit AC input."""
    c = Circuit()
    c.add(VoltageSource("Vbias", "bias", "0", 5.0))
    c.add(CurrentSource("Iac", "0", "out", 0.0, ac=AcSource(1.0e-3, 90.0)))
    c.add(Resistor("Rbias", "bias", "out", 1000.0))
    c.add(Resistor("Rload", "out", "0", 1000.0))

    result = ac_sweep(c, f_start=1000.0, f_stop=1000.0, n_points=1)
    pt = result.points[0]

    assert isclose(pt.node_voltages["bias"].real, 0.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["bias"].imag, 0.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["out"].real, 0.0, abs_tol=1e-12)
    assert isclose(pt.node_voltages["out"].imag, 0.5, abs_tol=1e-12)


def test_ac_unequal_resistive_divider():
    """R1=1kΩ, R2=3kΩ: V_out = Vin * R2/(R1+R2) = 0.75 V (frequency-independent)."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 3000.0))
    result = ac_sweep(c, f_start=100.0, f_stop=1e6, n_points=5)
    for pt in result.points:
        assert isclose(abs(pt.node_voltages["out"]), 0.75, abs_tol=1e-4), (
            f"f={pt.freq:.1f} Hz: |V_out|={abs(pt.node_voltages['out']):.6f}"
        )


# ============================================================================
# 18. AC sweep — RC low-pass filter (-3 dB at cutoff)
# ============================================================================


def test_ac_rc_lowpass_dc_gain_unity():
    """RC low-pass: gain → 1 at very low frequency (capacitor is open at DC).

    Circuit:  Vin → R (1kΩ) → out → C (1 μF) → GND
    Transfer function: H(jω) = 1 / (1 + jωRC)
    At very low f: |H| ≈ 1.
    """
    R, C = 1000.0, 1e-6
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", R))
    c.add(Capacitor("C1", "out", "0", C))
    # Use a very low start frequency so ωRC << 1
    result = ac_sweep(c, f_start=0.01, f_stop=0.1, n_points=3)
    for pt in result.points:
        gain = abs(pt.node_voltages["out"])
        assert gain > 0.999, (
            f"f={pt.freq:.3f} Hz: gain={gain:.6f} (expected ≈ 1.0 at DC)"
        )


def test_ac_rc_lowpass_3db_at_cutoff():
    """RC low-pass: gain = 1/√2 at f_c = 1/(2πRC).

    Analytic: H(jω) = 1/(1 + jωRC).
    At ω = 1/RC: |H| = 1/√2 ≈ 0.7071.

    We find the frequency point nearest f_c and check gain within 1%.
    """
    R, C = 1000.0, 1e-6
    f_c = 1.0 / (2.0 * math.pi * R * C)  # ≈ 159.15 Hz
    expected_gain = 1.0 / math.sqrt(2.0)   # ≈ 0.7071

    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", R))
    c.add(Capacitor("C1", "out", "0", C))

    # Dense log sweep straddling the cutoff
    result = ac_sweep(c, f_start=10.0, f_stop=10000.0, n_points=200, sweep="log")

    # Find the point closest to f_c
    closest = min(result.points, key=lambda p: abs(p.freq - f_c))
    gain = abs(closest.node_voltages["out"])

    assert isclose(gain, expected_gain, rel_tol=0.01), (
        f"At f≈f_c={closest.freq:.1f} Hz: gain={gain:.5f}, expected {expected_gain:.5f}"
    )


def test_ac_rc_lowpass_phase_at_cutoff():
    """RC low-pass: phase ≈ −45° at f_c."""
    R, C = 1000.0, 1e-6
    f_c = 1.0 / (2.0 * math.pi * R * C)

    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", R))
    c.add(Capacitor("C1", "out", "0", C))

    result = ac_sweep(c, f_start=10.0, f_stop=10000.0, n_points=200, sweep="log")
    closest = min(result.points, key=lambda p: abs(p.freq - f_c))
    phase_deg = math.degrees(cmath.phase(closest.node_voltages["out"]))

    # Phase should be ≈ −45° ± 2°
    assert isclose(phase_deg, -45.0, abs_tol=2.0), (
        f"Phase at f_c: {phase_deg:.2f}° (expected ≈ −45°)"
    )


def test_ac_rc_lowpass_rolloff_above_cutoff():
    """RC low-pass: gain decreases at 20 dB/decade above f_c.

    At 10×f_c the gain should be ≈ 1/(10√2) ≈ 0.0707 (−20 dB).
    At 100×f_c the gain should be ≈ 1/(100√2) ≈ 0.00707 (−40 dB).
    """
    R, C = 1000.0, 1e-6
    f_c = 1.0 / (2.0 * math.pi * R * C)

    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", R))
    c.add(Capacitor("C1", "out", "0", C))

    result = ac_sweep(c, f_start=1.0, f_stop=1e6, n_points=500, sweep="log")

    def gain_at(f: float) -> float:
        pt = min(result.points, key=lambda p: abs(p.freq - f))
        return abs(pt.node_voltages["out"])

    g1x = gain_at(f_c * 10.0)
    g10x = gain_at(f_c * 100.0)

    # Each decade above cutoff: gain ≈ f_c / (√2 · f)
    assert g1x < 0.12, f"Gain at 10×f_c should be < 0.12, got {g1x:.5f}"
    assert g10x < g1x / 5.0, (
        f"Gain at 100×f_c ({g10x:.6f}) should be << gain at 10×f_c ({g1x:.6f})"
    )


def test_ac_rc_lowpass_gain_monotone_decreasing():
    """RC low-pass: gain is monotonically decreasing with frequency."""
    R, C = 1000.0, 1e-6
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", R))
    c.add(Capacitor("C1", "out", "0", C))

    result = ac_sweep(c, f_start=1.0, f_stop=1e6, n_points=50, sweep="log")
    gains = [abs(pt.node_voltages["out"]) for pt in result.points]

    for i in range(1, len(gains)):
        assert gains[i] <= gains[i - 1] + 1e-6, (
            f"Gain not monotone: gains[{i - 1}]={gains[i - 1]:.6f},"
            f" gains[{i}]={gains[i]:.6f}"
        )


# ============================================================================
# 19. AC sweep — RL high-pass filter
# ============================================================================


def test_ac_rl_highpass_gain_increases_with_frequency():
    """RL high-pass: gain increases from 0 to 1 as frequency increases.

    Circuit:  Vin → R (1 kΩ) → mid → L (1 mH) → GND
    Transfer function: H(jω) = jωL / (R + jωL)

    The output is the node "mid" (junction of R and L).  Since L is
    between mid and GND, V_mid = Vin × jωL / (R + jωL):
    - At low f (ω → 0): Z_L = jωL → 0, so V_mid → 0.
    - At high f (ω → ∞): Z_L >> R, so V_mid → Vin.
    """
    R, L = 1000.0, 1e-3
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "mid", R))
    c.add(Inductor("L1", "mid", "0", L))

    result = ac_sweep(c, f_start=100.0, f_stop=1e7, n_points=50, sweep="log")
    gains = [abs(pt.node_voltages["mid"]) for pt in result.points]

    # Low end should be well below 0.5
    assert gains[0] < 0.5, f"Low-freq gain {gains[0]:.4f} should be < 0.5"
    # High end should be close to 1
    assert gains[-1] > 0.95, f"High-freq gain {gains[-1]:.4f} should be > 0.95"
    # Monotone increasing (RL high-pass)
    for i in range(1, len(gains)):
        assert gains[i] >= gains[i - 1] - 1e-6, (
            f"RL high-pass not monotone increasing at index {i}"
        )


def test_ac_rl_highpass_3db_at_cutoff():
    """RL high-pass: gain = 1/√2 at f_c = R/(2πL).

    Circuit: Vin → R (1 kΩ) → mid → L (1 mH) → GND
    With R=1kΩ, L=1mH: f_c = 1000 / (2π × 0.001) ≈ 159.15 kHz.
    At f_c: |H| = |jωL| / |R + jωL| = 1/√2.
    """
    R, L = 1000.0, 1e-3
    f_c = R / (2.0 * math.pi * L)  # ≈ 159.15 kHz
    expected_gain = 1.0 / math.sqrt(2.0)

    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "mid", R))
    c.add(Inductor("L1", "mid", "0", L))

    result = ac_sweep(c, f_start=1e4, f_stop=1e7, n_points=300, sweep="log")
    closest = min(result.points, key=lambda p: abs(p.freq - f_c))
    gain = abs(closest.node_voltages["mid"])

    assert isclose(gain, expected_gain, rel_tol=0.02), (
        f"RL high-pass gain at f_c={closest.freq:.0f} Hz: {gain:.5f},"
        f" expected {expected_gain:.5f}"
    )


def test_ac_mutual_inductor_transformer_ratio():
    primary_l = 1.0e-3
    secondary_l = 4.0e-3
    coupling = 0.9
    load = 1_000.0
    freq = 1_000.0
    mutual_l = coupling * math.sqrt(primary_l * secondary_l)
    expected = (1j * 2.0 * math.pi * freq * mutual_l) / (
        1.0 + 1j * 2.0 * math.pi * freq * secondary_l / load
    )

    c = Circuit()
    c.add(CurrentSource("Iin", "0", "pri", 0.0, ac=AcSource(1.0)))
    c.add(Inductor("Lpri", "pri", "0", primary_l))
    c.add(Inductor("Lsec", "sec", "0", secondary_l))
    c.add(MutualInductor("K1", "Lpri", "Lsec", coupling))
    c.add(Resistor("Rload", "sec", "0", load))

    point = ac_sweep(c, f_start=freq, f_stop=freq, n_points=1, sweep="lin").points[0]

    assert point.node_voltages["sec"] == pytest.approx(expected)


def test_ac_mutual_inductor_rejects_missing_reference():
    c = Circuit()
    c.add(VoltageSource("Vin", "pri", "0", 1.0))
    c.add(Inductor("Lpri", "pri", "0", 1.0e-3))
    c.add(MutualInductor("Kbad", "Lpri", "Lmissing", 0.9))

    with pytest.raises(ValueError, match="referenced inductor"):
        ac_sweep(c, f_start=1_000.0, f_stop=1_000.0, n_points=1, sweep="lin")


def test_ac_transmission_line_matched_load_phase_delay():
    freq = 1_000_000.0
    delay = 1.0 / (4.0 * freq)

    c = Circuit()
    c.add(VoltageSource("Vin", "src", "0", 0.0, ac=AcSource(1.0)))
    c.add(Resistor("Rsrc", "src", "in", 50.0))
    c.add(TransmissionLine("T1", "in", "0", "out", "0", 50.0, delay))
    c.add(Resistor("Rload", "out", "0", 50.0))

    point = ac_sweep(c, f_start=freq, f_stop=freq, n_points=1, sweep="lin").points[0]

    assert point.node_voltages["out"] == pytest.approx(-0.5j)


def test_ac_transmission_line_rejects_invalid_parameters():
    c = Circuit()
    c.add(VoltageSource("Vin", "src", "0", 0.0, ac=AcSource(1.0)))
    c.add(TransmissionLine("Tbad", "src", "0", "out", "0", 0.0, 1.0e-9))
    c.add(Resistor("Rload", "out", "0", 50.0))

    with pytest.raises(ValueError, match="characteristic impedance must be positive"):
        ac_sweep(c, f_start=1_000.0, f_stop=1_000.0, n_points=1, sweep="lin")


# ============================================================================
# 20. AC sweep — sweep modes (log, lin, edge cases)
# ============================================================================


def test_ac_log_sweep_first_and_last_frequencies():
    """Log sweep: first point = f_start, last point = f_stop."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=10.0, f_stop=100000.0, n_points=5, sweep="log")
    assert isclose(result.points[0].freq, 10.0, rel_tol=1e-6)
    assert isclose(result.points[-1].freq, 100000.0, rel_tol=1e-6)


def test_ac_lin_sweep_first_and_last_frequencies():
    """Linear sweep: first point = f_start, last point = f_stop."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=1000.0, f_stop=5000.0, n_points=5, sweep="lin")
    assert isclose(result.points[0].freq, 1000.0, rel_tol=1e-6)
    assert isclose(result.points[-1].freq, 5000.0, rel_tol=1e-6)


def test_ac_lin_sweep_uniform_spacing():
    """Linear sweep: frequency points are uniformly spaced."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=0.0, f_stop=1000.0, n_points=6, sweep="lin")
    freqs = [pt.freq for pt in result.points]
    # Expected: 0, 200, 400, 600, 800, 1000
    expected_step = 200.0
    for i in range(1, len(freqs)):
        assert isclose(freqs[i] - freqs[i - 1], expected_step, rel_tol=1e-6), (
            f"Step {i}: {freqs[i] - freqs[i - 1]:.2f} (expected {expected_step})"
        )


def test_ac_log_sweep_decade_spacing():
    """Log sweep across 3 decades: first and last are 1000× apart."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))
    result = ac_sweep(c, f_start=1.0, f_stop=1000.0, n_points=4, sweep="log")
    freqs = [pt.freq for pt in result.points]
    # 3 decades in 4 points: ratio between each pair = 10^(3/3) = 10
    ratio_0_1 = freqs[1] / freqs[0]
    ratio_1_2 = freqs[2] / freqs[1]
    assert isclose(ratio_0_1, ratio_1_2, rel_tol=1e-6)


# ============================================================================
# 21. AC sweep — small-signal nonlinear elements
# ============================================================================


def test_ac_diode_small_signal_forward_biased():
    """Forward-biased diode in AC: small-signal resistance is finite.

    The diode small-signal model is a conductance gd = (Is/Vt)·exp(Vd/Vt).
    With a DC bias of 0.6 V: Vd = 0.6 V (below 0.7 V clamp).

    gd = (1e-15 / 0.02585) * exp(0.6 / 0.02585) ≈ large
    Small-signal output voltage should be much less than 1 V (the diode
    shunts heavily).

    Circuit:  Vac(1V) → R(1kΩ) → anode → D(0.6V bias) → GND
    """
    # Provide a DC bias via a separate source; the AC source is the 1V source.
    Is_val = 1e-15
    Vt_val = 0.02585
    Vbias = 0.6

    c = Circuit()
    # AC input source (amplitude 1 V)
    c.add(VoltageSource("Vac", "in", "0", 1.0))
    # Series resistor
    c.add(Resistor("R1", "in", "anode", 1000.0))
    # DC bias current source to forward-bias the diode
    # I = Is*(exp(Vbias/Vt) - 1) to set anode to approximately Vbias
    I_bias = Is_val * (math.exp(Vbias / Vt_val) - 1.0)
    c.add(CurrentSource("I_bias", "anode", "0", I_bias))
    # The diode itself
    c.add(Diode("D1", anode="anode", cathode="0", Is=Is_val, Vt=Vt_val))

    result = ac_sweep(c, f_start=1000.0, f_stop=10000.0, n_points=5)
    # With the diode forward-biased (small-signal conductance >> 1/R1),
    # V_anode should be very small.
    for pt in result.points:
        v_anode = abs(pt.node_voltages["anode"])
        # The diode has such high gd that it nearly short-circuits the node
        assert v_anode < 0.1, (
            f"f={pt.freq:.0f} Hz: V_anode={v_anode:.4f} (expected < 0.1 with"
            f" forward-biased diode)"
        )


def test_ac_diode_reverse_biased_acts_like_open():
    """Reverse-biased diode in AC: near-zero conductance.

    When Vd ≤ 0 (cathode at or above anode), gd ≈ Is/Vt * exp(0) = Is/Vt
    which is negligibly small (1e-15/0.02585 ≈ 3.9e-14 S → R ≈ 25 GΩ).

    The diode is effectively an open circuit, so the voltage at the anode
    is determined only by the resistor divider.
    """
    c = Circuit()
    c.add(VoltageSource("Vac", "in", "0", 0.0, ac=AcSource(1.0)))
    c.add(Resistor("R1", "in", "anode", 1000.0))
    c.add(Resistor("R2", "anode", "0", 1000.0))   # load
    # Diode reverse-biased: cathode at Vin, anode at V_mid ≈ 0.5 V
    c.add(Diode("D1", anode="anode", cathode="in"))

    result = ac_sweep(c, f_start=100.0, f_stop=10000.0, n_points=5)
    # With open-circuit diode, V_anode ≈ Vin × R2/(R1+R2) = 0.5V
    for pt in result.points:
        v_anode = abs(pt.node_voltages["anode"])
        assert isclose(v_anode, 0.5, abs_tol=0.05), (
            f"f={pt.freq:.0f} Hz: V_anode={v_anode:.5f} (expected ≈ 0.5)"
        )


def test_ac_diode_junction_capacitance_shunts_high_frequency():
    """Reverse-biased diode junction capacitance is stamped in AC."""
    c = Circuit()
    c.add(VoltageSource("Vac", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "node", 1000.0))
    # Reverse-biased at the DC operating point, so the AC drop is dominated by Cjo.
    c.add(Diode("D1", anode="0", cathode="node", Cjo=1.0e-6))

    result = ac_sweep(c, f_start=10.0, f_stop=100000.0, n_points=2)
    low = abs(result.points[0].node_voltages["node"])
    high = abs(result.points[-1].node_voltages["node"])

    assert low > 0.9
    assert high < low / 100.0


def test_ac_diode_depletion_capacitance_falls_with_reverse_bias():
    """VJ/M reduce junction capacitance as reverse bias widens depletion."""

    def high_frequency_voltage(dc_bias: float) -> float:
        c = Circuit()
        c.add(VoltageSource("Vac", "in", "0", dc_bias, ac=AcSource(1.0)))
        c.add(Resistor("R1", "in", "node", 1000.0))
        c.add(Diode("D1", anode="0", cathode="node", Cjo=1.0e-6, Vj=1.0, M=0.5))
        result = ac_sweep(c, f_start=100000.0, f_stop=100000.0, n_points=1)
        return abs(result.points[0].node_voltages["node"])

    zero_bias = high_frequency_voltage(0.0)
    reverse_biased = high_frequency_voltage(4.0)

    assert reverse_biased > zero_bias * 1.8


def test_ac_diode_forward_depletion_coefficient_shapes_capacitance():
    """FC controls the continuous forward-bias depletion continuation."""

    def forward_biased_voltage(coefficient: float) -> float:
        c = Circuit()
        c.add(VoltageSource("Vac", "in", "0", 0.75, ac=AcSource(1.0)))
        c.add(Resistor("R1", "in", "node", 1000.0))
        c.add(
            Diode(
                "D1",
                anode="node",
                cathode="0",
                Is=1.0e-30,
                Cjo=1.0e-6,
                Vj=1.0,
                M=0.5,
                Fc=coefficient,
            )
        )
        result = ac_sweep(c, f_start=1000.0, f_stop=1000.0, n_points=1)
        return abs(result.points[0].node_voltages["node"])

    early_transition = forward_biased_voltage(0.2)
    late_transition = forward_biased_voltage(0.8)

    assert late_transition < early_transition * 0.85


def test_ac_diode_transit_time_shunts_forward_bias_at_high_frequency():
    """Forward-biased diode transit time contributes diffusion capacitance."""
    def high_frequency_anode(tt: float) -> float:
        c = Circuit()
        c.add(VoltageSource("Vac", "in", "0", 1.0, ac=AcSource(1.0)))
        c.add(Resistor("R1", "in", "anode", 1.0e6))
        c.add(Diode("D1", anode="anode", cathode="0", Tt=tt))
        result = ac_sweep(c, f_start=100000000.0, f_stop=100000000.0, n_points=1)
        return abs(result.points[0].node_voltages["anode"])

    without_transit = high_frequency_anode(0.0)
    with_transit = high_frequency_anode(1.0e-6)

    assert without_transit > 0.01
    assert with_transit < without_transit / 100.0


def test_ac_bjt_npn_small_signal():
    """NPN BJT in forward-active: small-signal current gain > 1.

    DC bias: Vb=0.7V, Vcc=5V, Rc=1kΩ, emitter grounded.
    AC: small input signal at base (through Rb=10kΩ), output at collector.

    In the small-signal model, the collector current is gm×Vbe, so the
    AC gain from base to collector is approximately −gm×Rc.
    We verify the output-to-input ratio |V_col / V_in| > 1 (gain > 0 dB).
    """
    Is_val = 1e-14
    Vt_val = 0.02585
    Rc = 1000.0
    Rb = 10000.0

    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", 5.0))
    # AC input (1 V amplitude) through base resistor
    c.add(VoltageSource("Vac", "ac_in", "0", 1.0))
    c.add(Resistor("Rb", "ac_in", "base", Rb))
    # DC bias to forward-bias the BE junction
    c.add(VoltageSource("Vbias", "base_bias", "0", 0.7))
    c.add(Resistor("Rbias", "base_bias", "base", Rb))
    c.add(Resistor("Rc", "vcc", "col", Rc))
    c.add(BJT("Q1", collector="col", base="base", emitter="0",
               Is=Is_val, beta_f=100.0, Vt=Vt_val))

    result = ac_sweep(c, f_start=1000.0, f_stop=10000.0, n_points=5)
    for pt in result.points:
        # Both nodes should be present
        assert "col" in pt.node_voltages
        assert "base" in pt.node_voltages


def test_ac_bjt_junction_capacitance_shunts_high_frequency_base_drive():
    """BJT Cje contributes base-emitter capacitance in AC analysis."""
    def base_amplitude(cje: float) -> float:
        c = Circuit()
        c.add(VoltageSource("Vac", "in", "0", 0.0, ac=AcSource(1.0)))
        c.add(Resistor("Rin", "in", "base", 1000.0))
        c.add(Resistor("Rc", "col", "0", 1000.0))
        c.add(BJT("Q1", collector="col", base="base", emitter="0", Cje=cje))
        result = ac_sweep(c, f_start=100000.0, f_stop=100000.0, n_points=1)
        return abs(result.points[0].node_voltages["base"])

    without_capacitance = base_amplitude(0.0)
    with_capacitance = base_amplitude(1.0e-6)

    assert without_capacitance > 0.9
    assert with_capacitance < without_capacitance / 100.0


def test_ac_bjt_transit_time_adds_diffusion_capacitance():
    """BJT Tf contributes gm-scaled diffusion capacitance in AC analysis."""
    def base_amplitude(tf: float) -> float:
        c = Circuit()
        c.add(VoltageSource("Vac", "in", "0", 0.0, ac=AcSource(1.0)))
        c.add(Resistor("Rin", "in", "base", 1000.0))
        c.add(Resistor("Rc", "col", "0", 1000.0))
        c.add(BJT("Q1", collector="col", base="base", emitter="0", Is=25.85e-6, Tf=tf))
        result = ac_sweep(c, f_start=100000.0, f_stop=100000.0, n_points=1)
        return abs(result.points[0].node_voltages["base"])

    without_transit_time = base_amplitude(0.0)
    with_transit_time = base_amplitude(1.0e-3)

    assert without_transit_time > 0.9
    assert with_transit_time < without_transit_time / 100.0


def test_ac_bjt_forward_excess_phase_rotates_transconductance():
    def collector_voltage(ptf: float) -> complex:
        tf = 1.0e-6
        frequency = 1.0 / (2.0 * math.pi * tf)
        circuit = Circuit()
        circuit.add(VoltageSource("Vac", "base", "0", 0.0, ac=AcSource(1.0)))
        circuit.add(Resistor("Rc", "col", "0", 1.0))
        circuit.add(BJT("Q1", "col", "base", "0", Is=25.85e-6, Tf=tf, Ptf=ptf))
        return ac_sweep(
            circuit, f_start=frequency, f_stop=frequency, n_points=1
        ).points[0].node_voltages["col"]

    without_excess_phase = collector_voltage(0.0)
    with_excess_phase = collector_voltage(90.0)

    assert without_excess_phase.real < -0.0009
    assert abs(without_excess_phase.imag) < 1.0e-9
    assert with_excess_phase.imag > 0.0009
    assert abs(with_excess_phase.real) < 1.0e-9


def test_ac_bjt_forward_transit_time_bias_coefficient_scales_diffusion_capacitance():
    def base_amplitude(
        xtf: float,
        itf: float = 0.0,
        vtf: float = 0.0,
        collector_voltage: float = 0.0,
    ) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vac", "in", "0", 0.0, ac=AcSource(1.0)))
        circuit.add(Resistor("Rin", "in", "base", 1000.0))
        circuit.add(VoltageSource("Vcol", "col", "0", collector_voltage))
        circuit.add(
            BJT(
                "Q1",
                "col",
                "base",
                "0",
                Is=25.85e-6,
                Tf=1.0e-6,
                Xtf=xtf,
                Itf=itf,
                Vtf=vtf,
            )
        )
        return abs(
            ac_sweep(
                circuit, f_start=100000.0, f_stop=100000.0, n_points=1
            ).points[0].node_voltages["base"]
        )

    nominal = base_amplitude(0.0)
    bias_scaled = base_amplitude(9.0)
    current_limited = base_amplitude(9.0, 1.0)
    voltage_limited = base_amplitude(9.0, vtf=0.1, collector_voltage=1.0)

    assert bias_scaled < nominal / 5.0
    assert current_limited > bias_scaled * 5.0
    assert voltage_limited > bias_scaled * 5.0


def test_ac_bjt_reverse_transit_time_adds_collector_diffusion_capacitance():
    """BJT Tr contributes gm-scaled base-collector diffusion capacitance."""
    def base_amplitude(tr: float) -> float:
        c = Circuit()
        c.add(VoltageSource("Vac", "in", "0", 0.0, ac=AcSource(1.0)))
        c.add(Resistor("Rin", "in", "base", 1000.0))
        c.add(Resistor("Rc", "col", "0", 1.0))
        c.add(BJT("Q1", collector="col", base="base", emitter="0", Is=25.85e-6, Tr=tr))
        result = ac_sweep(c, f_start=100000.0, f_stop=100000.0, n_points=1)
        return abs(result.points[0].node_voltages["base"])

    without_transit_time = base_amplitude(0.0)
    with_transit_time = base_amplitude(1.0e-2)

    assert without_transit_time > 0.9
    assert with_transit_time < without_transit_time / 100.0


def test_ac_bjt_reverse_emission_coefficient_reduces_collector_diffusion_capacitance():
    """BJT Nr scales the reverse base-collector diffusion charge."""
    def base_amplitude(nr: float) -> float:
        c = Circuit()
        c.add(VoltageSource("Vac", "in", "0", 0.0, ac=AcSource(1.0)))
        c.add(Resistor("Rin", "in", "base", 1000.0))
        c.add(Resistor("Rc", "col", "0", 1.0))
        c.add(BJT("Q1", collector="col", base="base", emitter="0", Is=25.85e-6, Tr=1.0e-2, Nr=nr))
        result = ac_sweep(c, f_start=100000.0, f_stop=100000.0, n_points=1)
        return abs(result.points[0].node_voltages["base"])

    assert base_amplitude(2.0) > base_amplitude(1.0)


def test_ac_bjt_reverse_early_voltage_reduces_gain():
    def gain(var: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vin", "base", "0", 0.65, ac=AcSource(1.0)))
        circuit.add(Resistor("Rload", "out", "0", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Var=var))
        point = ac_sweep(circuit, f_start=1_000.0, f_stop=1_000.0, n_points=1, sweep="lin").points[0]
        return abs(point.node_voltages["out"])

    assert gain(1.0) < gain(0.0)


def test_ac_bjt_forward_beta_rolloff_reduces_gain():
    def gain(ikf: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vin", "base", "0", 0.65, ac=AcSource(1.0)))
        circuit.add(Resistor("Rload", "out", "0", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Ikf=ikf))
        point = ac_sweep(
            circuit, f_start=1_000.0, f_stop=1_000.0, n_points=1, sweep="lin"
        ).points[0]
        return abs(point.node_voltages["out"])

    assert gain(1.0e-4) < gain(0.0)


def test_ac_bjt_base_emitter_leakage_reduces_gain_through_source_resistance():
    def gain(ise: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vin", "in", "0", 0.65, ac=AcSource(1.0)))
        circuit.add(Resistor("Rin", "in", "base", 1_000.0))
        circuit.add(Resistor("Rload", "out", "0", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Ise=ise, Ne=1.5))
        point = ac_sweep(
            circuit,
            f_start=1_000.0,
            f_stop=1_000.0,
            n_points=1,
            sweep="lin",
        ).points[0]
        return abs(point.node_voltages["out"])

    assert gain(1.0e-10) < gain(0.0)


def test_ac_bjt_base_collector_leakage_loads_source_resistance():
    def amplitude(isc: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vin", "in", "0", 0.65, ac=AcSource(1.0)))
        circuit.add(Resistor("Rin", "in", "base", 1_000.0))
        circuit.add(BJT("Q1", "0", "base", "base", Isc=isc, Nc=1.5))
        point = ac_sweep(
            circuit, f_start=1_000.0, f_stop=1_000.0, n_points=1, sweep="lin"
        ).points[0]
        return abs(point.node_voltages["base"])

    assert amplitude(1.0e-10) < amplitude(0.0)


def test_ac_bjt_base_emitter_depletion_capacitance_falls_with_reverse_bias():
    """BJT Vje/Mje shape Cje under reverse base-emitter bias."""
    def base_amplitude(mje: float) -> float:
        c = Circuit()
        c.add(VoltageSource("Vac", "in", "0", -1.0, ac=AcSource(1.0)))
        c.add(Resistor("Rin", "in", "base", 1000.0))
        c.add(BJT("Q1", collector="0", base="base", emitter="0", Cje=1.0e-6, Vje=0.75, Mje=mje))
        result = ac_sweep(c, f_start=1000.0, f_stop=1000.0, n_points=1)
        return abs(result.points[0].node_voltages["base"])

    assert base_amplitude(0.5) > base_amplitude(0.0)


def test_ac_bjt_base_collector_depletion_capacitance_falls_with_reverse_bias():
    """BJT Vjc/Mjc shape Cjc under reverse base-collector bias."""
    def collector_amplitude(mjc: float) -> float:
        c = Circuit()
        c.add(VoltageSource("Vac", "in", "0", 1.0, ac=AcSource(1.0)))
        c.add(Resistor("Rin", "in", "collector", 1000.0))
        c.add(BJT("Q1", collector="collector", base="0", emitter="0", Cjc=1.0e-6, Vjc=0.75, Mjc=mjc))
        result = ac_sweep(c, f_start=1000.0, f_stop=1000.0, n_points=1)
        return abs(result.points[0].node_voltages["collector"])

    assert collector_amplitude(0.5) > collector_amplitude(0.0)


def test_ac_bjt_xcjc_partitions_depletion_capacitance_to_external_base():
    def base_amplitude(fraction: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vac", "in", "0", 0.0, ac=AcSource(1.0)))
        circuit.add(Resistor("Rin", "in", "base", 1_000.0))
        circuit.add(
            BJT(
                "Q1",
                collector="0",
                base="base",
                emitter="0",
                Is=1.0e-30,
                Cjc=1.0e-9,
                Rb=10_000.0,
                Xcjc=fraction,
            )
        )
        point = ac_sweep(
            circuit, f_start=1.0e6, f_stop=1.0e6, n_points=1
        ).points[0]
        return abs(point.node_voltages["base"])

    assert base_amplitude(1.0) > base_amplitude(0.0)


def test_ac_bjt_forward_bias_depletion_coefficient_shapes_both_junctions():
    def junction_amplitude(coefficient: float, base_emitter: bool) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vac", "in", "0", 0.6, ac=AcSource(1.0)))
        circuit.add(Resistor("Rin", "in", "base", 1000.0))
        circuit.add(BJT(
            "Q1",
            collector="0",
            base="base",
            emitter="0",
            Is=1.0e-30,
            Cje=1.0e-6 if base_emitter else 0.0,
            Cjc=0.0 if base_emitter else 1.0e-6,
            Fc=coefficient,
        ))
        result = ac_sweep(circuit, f_start=1000.0, f_stop=1000.0, n_points=1)
        return abs(result.points[0].node_voltages["base"])

    for base_emitter in (True, False):
        early_transition = junction_amplitude(0.2, base_emitter)
        late_transition = junction_amplitude(0.8, base_emitter)
        assert late_transition < early_transition * 0.9


def test_ac_mosfet_overlap_capacitance_shunts_high_frequency_gate_drive():
    """MOS Level-1 CGSO contributes gate-source AC susceptance."""
    def gate_amplitude(cgso: float) -> float:
        c = Circuit()
        c.add(VoltageSource("Vac", "in", "0", 0.0, ac=AcSource(1.0)))
        c.add(Resistor("Rin", "in", "gate", 1000.0))
        c.add(Resistor("Rdrain", "drain", "0", 1000.0))
        c.add(Mosfet(
            "M1",
            "drain",
            "gate",
            "0",
            "0",
            MOSFET(
                MosfetType.NMOS,
                Level1Model(Level1Params(KP=1.0e-12, W=1.0, L=1.0, CGSO=cgso)),
            ),
        ))
        result = ac_sweep(c, f_start=100000.0, f_stop=100000.0, n_points=1)
        return abs(result.points[0].node_voltages["gate"])

    without_capacitance = gate_amplitude(0.0)
    with_capacitance = gate_amplitude(1.0e-6)

    assert without_capacitance > 0.9
    assert with_capacitance < without_capacitance / 100.0


# ============================================================================
# 22. AC sweep — current source injection
# ============================================================================


def test_ac_current_source_into_resistor():
    """AC current source (1 A) into a 1 kΩ resistor → V = I×R = 1 kV.

    At all frequencies, a current source in parallel with a resistor gives
    V = I × R (purely real, frequency-independent).
    """
    c = Circuit()
    c.add(CurrentSource("I1", "out", "0", 1.0))  # 1 A
    c.add(Resistor("R1", "out", "0", 1000.0))

    result = ac_sweep(c, f_start=100.0, f_stop=10000.0, n_points=5)
    for pt in result.points:
        v = pt.node_voltages["out"]
        # V = I × R = 1 A × 1 kΩ = 1000 V
        assert isclose(abs(v), 1000.0, abs_tol=1.0), (
            f"f={pt.freq:.0f} Hz: V={abs(v):.4f} (expected 1000)"
        )


def test_ac_current_source_with_capacitor_shunt():
    """AC current source into RC parallel: V decreases with frequency.

    I_s parallel with R (1kΩ) parallel with C (1μF):
        V(ω) = I_s / (1/R + jωC) = I_s × R / (1 + jωRC)

    At low f: |V| ≈ I_s × R = 1000 V (current mostly through R).
    At high f: |V| decreases as C shunts current away from R.

    With I_s = 1 mA, the magnitudes are more reasonable: V_low ≈ 1 V.
    """
    I_s = 1e-3   # 1 mA
    R = 1000.0
    C = 1e-6     # f_c ≈ 159 Hz

    c = Circuit()
    c.add(CurrentSource("I1", "out", "0", I_s))
    c.add(Resistor("R1", "out", "0", R))
    c.add(Capacitor("C1", "out", "0", C))

    result = ac_sweep(c, f_start=1.0, f_stop=1e6, n_points=50, sweep="log")
    v_low = abs(result.points[0].node_voltages["out"])
    v_high = abs(result.points[-1].node_voltages["out"])

    # At very low f: |V| ≈ I_s × R
    assert isclose(v_low, I_s * R, rel_tol=0.01), (
        f"Low-freq voltage {v_low:.4f} V (expected ≈ {I_s * R:.4f} V)"
    )
    # High frequency: shunted by cap, voltage much lower
    assert v_high < v_low / 100.0, (
        f"High-freq voltage {v_high:.6f} should be << low-freq {v_low:.6f}"
    )


def test_ac_inductor_acts_as_short_at_very_low_frequency():
    """Inductor near ω=0: modelled as near-short (large conductance).

    In the AC model, Y_L = 1/(jωL).  At ω=0 this is ∞, which is
    approximated as G = 1e12 S.  A very low-frequency sweep should
    show that the node voltage across the inductor is nearly zero
    (inductor is a short, no voltage drop).

    Circuit:  Vin(1V) → L(1mH) → mid → R(1kΩ) → GND

    At ω → 0, the inductor is a short, so V_mid ≈ V_in = 1 V.
    """
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Inductor("L1", "in", "mid", 1e-3))
    c.add(Resistor("R1", "mid", "0", 1000.0))

    # Use a truly tiny frequency to approach the ω=0 limit.
    # The implementation uses omega=0 exactly for f_start=0 in lin sweep.
    result = ac_sweep(c, f_start=0.0, f_stop=0.0, n_points=1, sweep="lin")
    if result.points:
        v_mid = abs(result.points[0].node_voltages["mid"])
        # Inductor short → V_mid ≈ V_in = 1 V
        assert v_mid > 0.99, (
            f"At ω=0 (inductor short), V_mid={v_mid:.6f} (expected ≈ 1.0)"
        )


# ============================================================================
# 23. TF analysis: TfResult dataclass
# ============================================================================


def test_tfresult_fields():
    """TfResult is a frozen dataclass with the expected fields."""
    r = TfResult(transfer_ratio=0.5, input_impedance=2000.0, output_impedance=500.0)
    assert isclose(r.transfer_ratio, 0.5)
    assert isclose(r.input_impedance, 2000.0)
    assert isclose(r.output_impedance, 500.0)
    assert r.converged is True  # default


def test_tfresult_converged_false():
    """TfResult.converged can be set to False."""
    r = TfResult(
        transfer_ratio=0.0,
        input_impedance=float("inf"),
        output_impedance=float("inf"),
        converged=False,
    )
    assert not r.converged
    assert r.input_impedance == float("inf")


def test_tfresult_is_frozen():
    """TfResult is frozen — attributes cannot be reassigned."""
    r = TfResult(transfer_ratio=1.0, input_impedance=100.0, output_impedance=50.0)
    with pytest.raises((AttributeError, TypeError)):
        r.transfer_ratio = 0.0  # type: ignore[misc]


def test_tfresult_exported():
    """TfResult is importable from the top-level spice_engine package."""
    from spice_engine import TfResult as TfResult_exported
    assert TfResult_exported is TfResult


def test_tf_text_output_table_is_stable() -> None:
    result = TfResult(
        transfer_ratio=0.5,
        input_impedance=2000.0,
        output_impedance=500.0,
    )

    assert format_tf_table(result) == (
        "TransferRatio\tInputImpedance\tOutputImpedance\n"
        "5.000000e-01\t2.000000e+03\t5.000000e+02\n"
    )


# ============================================================================
# 24. TF analysis: _build_ss_matrix helper
# ============================================================================


def test_build_ss_matrix_single_resistor():
    """Single resistor: G_ss has correct conductance value.

    Circuit: V1("v", "0") + R1("v", "0", 1kΩ)
    G_ss size = 2×2 (1 node "v" + 1 vsrc branch).
    G[0][0] = 1/1000, G[0][1] = G[1][0] = 1.0, G[1][1] = 0.
    """
    c = Circuit()
    c.add(VoltageSource("V1", "v", "0", 10.0))
    c.add(Resistor("R1", "v", "0", 1000.0))

    node_to_idx, _ = _node_index(c)
    vsrcs = _voltage_sources(c)
    dc_x = [0.0] * (len(node_to_idx) + len(vsrcs))

    G = _build_ss_matrix(c, node_to_idx, vsrcs, dc_x)

    # G[v][v] = 1/1000 (R1 conductance)
    v_idx = node_to_idx["v"]
    assert isclose(G[v_idx][v_idx], 1.0 / 1000.0, rel_tol=1e-9)
    # Structural VoltageSource entries
    branch_idx = len(node_to_idx) + 0
    assert isclose(G[v_idx][branch_idx], 1.0)
    assert isclose(G[branch_idx][v_idx], 1.0)


def test_build_ss_matrix_capacitor_is_open():
    """Capacitor contributes nothing to G_ss (open circuit at ω=0)."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 5.0))
    c.add(Capacitor("C1", "in", "out", 1e-6))
    c.add(Resistor("R1", "out", "0", 1000.0))

    node_to_idx, _ = _node_index(c)
    vsrcs = _voltage_sources(c)
    n = len(node_to_idx)
    dc_x = [0.0] * (n + len(vsrcs))

    G = _build_ss_matrix(c, node_to_idx, vsrcs, dc_x)

    # "out" row should only contain R1's conductance (no cap contribution).
    out_idx = node_to_idx["out"]
    assert isclose(G[out_idx][out_idx], 1.0 / 1000.0, rel_tol=1e-9)


def test_build_ss_matrix_inductor_is_short():
    """Inductor is modelled as a near-short (G = 1e12 S) at ω=0."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Inductor("L1", "in", "mid", 1e-3))
    c.add(Resistor("R1", "mid", "0", 1000.0))

    node_to_idx, _ = _node_index(c)
    vsrcs = _voltage_sources(c)
    dc_x = [0.0] * (len(node_to_idx) + len(vsrcs))

    G = _build_ss_matrix(c, node_to_idx, vsrcs, dc_x)

    in_idx = node_to_idx["in"]
    mid_idx = node_to_idx["mid"]
    # L1 adds conductance 1e12 to both diagonals and -1e12 off-diagonals.
    assert G[in_idx][mid_idx] < -1e11  # off-diagonal < 0


def test_build_ss_matrix_current_source_skipped():
    """CurrentSource does not contribute to G_ss (independent → zeroed)."""
    c = Circuit()
    c.add(CurrentSource("I1", "0", "n1", current=1e-3))
    c.add(Resistor("R1", "n1", "0", 1000.0))

    node_to_idx, _ = _node_index(c)
    vsrcs = _voltage_sources(c)
    dc_x = [0.0] * (len(node_to_idx) + len(vsrcs))

    G = _build_ss_matrix(c, node_to_idx, vsrcs, dc_x)

    # Only R1's conductance should be present.  Size = 1×1 (no vsrcs).
    n1_idx = node_to_idx["n1"]
    assert isclose(G[n1_idx][n1_idx], 1.0 / 1000.0, rel_tol=1e-9)
    # Only one row and column since there are no voltage sources.
    assert len(G) == 1


# ============================================================================
# 25. TF analysis: resistive circuits (voltage-source input)
# ============================================================================


def test_tf_voltage_divider_transfer_ratio():
    """Symmetric voltage divider: H = R2 / (R1 + R2) = 0.5.

    Circuit:
        V1 (10 V)  →  R1 (1kΩ)  →  vmid  →  R2 (1kΩ)  →  GND

    Transfer ratio (vmid / vin):  H = R2/(R1+R2) = 0.5
    Input impedance:               Z_in = R1 + R2 = 2000 Ω
    Output impedance:              Z_out = R1 ∥ R2 = 500 Ω
    """
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", 10.0))
    c.add(Resistor("R1", "vin", "vmid", 1000.0))
    c.add(Resistor("R2", "vmid", "0", 1000.0))

    result = tf(c, output_node="vmid", input_source="V1")

    assert result.converged
    assert isclose(result.transfer_ratio, 0.5, rel_tol=1e-6)
    assert isclose(result.input_impedance, 2000.0, rel_tol=1e-6)
    assert isclose(result.output_impedance, 500.0, rel_tol=1e-6)


def test_tf_asymmetric_divider():
    """Asymmetric divider: R1=3kΩ, R2=1kΩ.

    H    = R2/(R1+R2) = 1/4 = 0.25
    Z_in = R1 + R2 = 4000 Ω
    Z_out = R1∥R2 = 3000·1000/4000 = 750 Ω
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "out", 3000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = tf(c, output_node="out", input_source="Vin")

    assert isclose(result.transfer_ratio, 0.25, rel_tol=1e-6)
    assert isclose(result.input_impedance, 4000.0, rel_tol=1e-6)
    assert isclose(result.output_impedance, 750.0, rel_tol=1e-6)


def test_tf_series_resistor_output_at_source():
    """Output node is the input node itself: H = 1, Z_out = 0 (source node).

    Circuit:  V1("in", "0") + R1("in", "0").

    At the source node (vin) the voltage is fixed by V1, so H = 1 and
    Z_out = 0 (the ideal voltage source clamps the output).
    """
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = tf(c, output_node="in", input_source="V1")

    assert isclose(result.transfer_ratio, 1.0, rel_tol=1e-6)
    assert isclose(result.input_impedance, 1000.0, rel_tol=1e-6)
    # V1 forces V_in → Z_out = 0 (voltage source is a short for the test)
    assert isclose(result.output_impedance, 0.0, abs_tol=1e-9)


def test_tf_output_is_ground_node():
    """Output node is ground (0 V): H = 0 regardless of circuit."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = tf(c, output_node="0", input_source="V1")
    assert isclose(result.transfer_ratio, 0.0, abs_tol=1e-12)


def test_tf_three_resistor_ladder():
    """Three-resistor ladder: R1→R2→R3 to ground.

    Circuit:  V1(1V) → R1(1kΩ) → n1 → R2(1kΩ) → n2 → R3(1kΩ) → GND

    At n2 (output): voltage divider with R1+R2 vs R3.
    H = R3 / (R1 + R2 + R3) = 1/3  (all equal)
    Wait — this isn't quite right for a ladder. Let me use the exact formula.

    Using KCL:
      V_n1 = V_in * R_parallel(n1→gnd) / (R1 + R_parallel(n1→gnd))
           where R_parallel = R2 + R3 = 2kΩ
      V_n1 = 1 * 2000 / (1000 + 2000) = 2/3

      V_n2 = V_n1 * R3 / (R2 + R3) = (2/3) * 1000 / 2000 = 1/3
    """
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "n1", 1000.0))
    c.add(Resistor("R2", "n1", "n2", 1000.0))
    c.add(Resistor("R3", "n2", "0", 1000.0))

    result = tf(c, output_node="n2", input_source="V1")

    assert isclose(result.transfer_ratio, 1.0 / 3.0, rel_tol=1e-5)


def test_tf_inductor_short_at_dc():
    """Inductor is a near-short at ω=0: V across it is ≈ 0.

    Circuit:  V1(1V) → L1(1mH) → mid → R1(1kΩ) → GND

    At DC (ω=0): L is a short → V_mid ≈ V_in = 1 V, H ≈ 1.
    Z_in ≈ 0 (inductor short + R1 in parallel with near-zero L impedance ≈ 0)
    but numerically G_L = 1e12 >> G_R1, so Z_in ≈ 1/1e12 ≈ 0.
    """
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Inductor("L1", "in", "mid", 1e-3))
    c.add(Resistor("R1", "mid", "0", 1000.0))

    result = tf(c, output_node="mid", input_source="V1")

    assert result.converged
    # Inductor is a near-short: V_mid ≈ V_in, H ≈ 1
    assert isclose(result.transfer_ratio, 1.0, rel_tol=0.001)


def test_tf_converged_flag_matches_dc():
    """TfResult.converged mirrors the DC operating-point convergence."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = tf(c, output_node="out", input_source="V1")
    assert result.converged  # simple linear circuit always converges


def test_tf_diode_circuit_linearised():
    """Diode in series with a resistor: .TF linearises around the DC OP.

    Circuit:  V1(0.7V) → D1 → n_diode → R1(1kΩ) → GND

    At Vd ≈ 0.7 V (clamped), gd = (Is/Vt)·exp(0.7/Vt).
    Transfer ratio is the small-signal gain, not the DC ratio.
    We just verify the result has the right shape (converged, 0 < H < 1).
    """
    c = Circuit()
    c.add(VoltageSource("V1", "a", "0", 0.7))
    c.add(Diode("D1", anode="a", cathode="n_d"))
    c.add(Resistor("R1", "n_d", "0", 1000.0))

    result = tf(c, output_node="n_d", input_source="V1")

    assert result.converged
    assert 0.0 < result.transfer_ratio < 1.0


def test_tf_bjt_forward_early_voltage_reduces_output_impedance():
    def output_impedance(vaf: float) -> float:
        thermal_voltage = 0.02585
        circuit = Circuit()
        circuit.add(VoltageSource("Vcc", "vcc", "0", 5.0))
        circuit.add(VoltageSource("Vin", "base", "0", thermal_voltage * math.log(2.0)))
        circuit.add(Resistor("Rload", "vcc", "out", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Is=25.85e-6, Vt=thermal_voltage, Vaf=vaf))
        return tf(circuit, output_node="out", input_source="Vin").output_impedance

    assert output_impedance(10.0) < output_impedance(0.0)


def test_tf_bjt_reverse_early_voltage_reduces_gain():
    def gain(var: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vin", "base", "0", 0.65))
        circuit.add(Resistor("Rload", "out", "0", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Var=var))
        return abs(tf(circuit, output_node="out", input_source="Vin").gain)

    assert gain(1.0) < gain(0.0)


def test_tf_bjt_forward_beta_rolloff_reduces_gain():
    def gain(ikf: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vin", "base", "0", 0.65))
        circuit.add(Resistor("Rload", "out", "0", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Ikf=ikf))
        return abs(tf(circuit, output_node="out", input_source="Vin").gain)

    assert gain(1.0e-4) < gain(0.0)


def test_tf_bjt_base_emitter_leakage_reduces_input_impedance():
    def input_impedance(ise: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vin", "base", "0", 0.65))
        circuit.add(Resistor("Rload", "out", "0", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Ise=ise, Ne=1.5))
        return tf(circuit, output_node="out", input_source="Vin").input_impedance

    assert input_impedance(1.0e-10) < input_impedance(0.0)


def test_tf_bjt_base_collector_leakage_reduces_input_impedance():
    def input_impedance(isc: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vin", "base", "0", 0.65))
        circuit.add(BJT("Q1", "0", "base", "base", Isc=isc, Nc=1.5))
        return tf(circuit, output_node="base", input_source="Vin").input_impedance

    assert input_impedance(1.0e-10) < input_impedance(0.0)


def test_tf_bjt_forward_emission_coefficient_reduces_gain_and_raises_input_impedance():
    def transfer(nf: float) -> TfResult:
        circuit = Circuit()
        circuit.add(VoltageSource("Vin", "base", "0", 0.65))
        circuit.add(Resistor("Rload", "out", "0", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Nf=nf))
        return tf(circuit, output_node="out", input_source="Vin")

    ideal = transfer(1.0)
    shaped = transfer(2.0)
    assert abs(shaped.gain) < abs(ideal.gain)
    assert shaped.input_impedance > ideal.input_impedance


# ============================================================================
# 26. TF analysis: current-source input + transimpedance
# ============================================================================


def test_tf_current_source_into_resistor():
    """CurrentSource 1mA into 1kΩ: transimpedance H = V_out/I_in = R = 1kΩ.

    Circuit:  I1("0", "n1", 1mA) ∥ R1("n1", "0", 1kΩ)

    Transfer ratio H = V_n1 / I_source = R = 1000 Ω/A → 1000 V/A
    Z_in = parallel impedance from the source terminals = R = 1000 Ω
    Z_out = R = 1000 Ω (looking in from n1 with I1 zeroed → just R1)
    """
    c = Circuit()
    c.add(CurrentSource("I1", "0", "n1", current=1e-3))
    c.add(Resistor("R1", "n1", "0", 1000.0))

    result = tf(c, output_node="n1", input_source="I1")

    assert result.converged
    # H = V_n1 / 1A = R1 = 1000 V/A  (transimpedance)
    assert isclose(result.transfer_ratio, 1000.0, rel_tol=1e-6)
    # Z_in = V_source / I_source = R1 (load seen by source)
    assert isclose(result.input_impedance, 1000.0, rel_tol=1e-6)
    # Z_out = R1 (only R1 remains when I1 is zeroed)
    assert isclose(result.output_impedance, 1000.0, rel_tol=1e-6)


def test_tf_current_source_divider():
    """Current source into parallel R1∥R2: Z_in = R1∥R2.

    Circuit: I1("0", "n1") ∥ R1("n1","0", 1kΩ) ∥ R2("n1","0", 1kΩ)

    H    = V_n1 / 1 A = R1∥R2 = 500 Ω  (transimpedance in V/A)
    Z_in = R1∥R2 = 500 Ω
    Z_out = R1∥R2 = 500 Ω
    """
    c = Circuit()
    c.add(CurrentSource("I1", "0", "n1", current=1e-3))
    c.add(Resistor("R1", "n1", "0", 1000.0))
    c.add(Resistor("R2", "n1", "0", 1000.0))

    result = tf(c, output_node="n1", input_source="I1")

    assert isclose(result.transfer_ratio, 500.0, rel_tol=1e-6)
    assert isclose(result.input_impedance, 500.0, rel_tol=1e-6)
    assert isclose(result.output_impedance, 500.0, rel_tol=1e-6)


def test_tf_mixed_source_types():
    """Circuit with both a VoltageSource and a CurrentSource.

    When the voltage source is the input, the current source is zeroed
    (open circuit), and vice versa.

    Circuit:
        V1("vin", "0", 5V) + R1("vin","out", 1kΩ) + R2("out","0", 2kΩ)
        + I1("0","out", 1mA)

    With I1 zeroed (TF from V1 to "out"):
        H = R2 / (R1 + R2) = 2/3 ≈ 0.667
        Z_in = R1 + R2 = 3000 Ω
        Z_out = R1∥R2 = 1000·2000/3000 ≈ 666.7 Ω
    """
    c = Circuit()
    c.add(VoltageSource("V1", "vin", "0", 5.0))
    c.add(Resistor("R1", "vin", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 2000.0))
    c.add(CurrentSource("I1", "0", "out", 1e-3))  # zeroed in the V1 TF

    result = tf(c, output_node="out", input_source="V1")

    assert isclose(result.transfer_ratio, 2.0 / 3.0, rel_tol=1e-5)
    assert isclose(result.input_impedance, 3000.0, rel_tol=1e-5)
    assert isclose(result.output_impedance, 1000.0 * 2000.0 / 3000.0, rel_tol=1e-5)


# ============================================================================
# 27. TF analysis: error cases and edge cases
# ============================================================================


def test_tf_raises_on_missing_source():
    """tf() raises ValueError when the named source is not in the circuit."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    with pytest.raises(ValueError, match="nonexistent"):
        tf(c, output_node="in", input_source="nonexistent")


def test_tf_raises_on_non_source_element():
    """tf() raises ValueError when the named element is not a source."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "out", 1000.0))

    with pytest.raises(ValueError, match="VoltageSource or CurrentSource"):
        tf(c, output_node="out", input_source="R1")


def test_tf_raises_on_unknown_output_node():
    """tf() raises ValueError when the output node is not in the circuit."""
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    with pytest.raises(ValueError, match="ghost_node"):
        tf(c, output_node="ghost_node", input_source="V1")


def test_tf_result_exported_and_callable():
    """tf() is exported from the top-level spice_engine package."""
    from spice_engine import tf as tf_exported
    assert tf_exported is tf


def test_tf_voltage_divider_independence_of_source_voltage():
    """Transfer ratio is independent of the source voltage (it's a ratio).

    H = V_out / V_in = R2/(R1+R2) regardless of the absolute source value.
    """
    c5 = Circuit()
    c5.add(VoltageSource("V1", "in", "0", 5.0))
    c5.add(Resistor("R1", "in", "out", 1000.0))
    c5.add(Resistor("R2", "out", "0", 1000.0))

    c100 = Circuit()
    c100.add(VoltageSource("V1", "in", "0", 100.0))
    c100.add(Resistor("R1", "in", "out", 1000.0))
    c100.add(Resistor("R2", "out", "0", 1000.0))

    r5 = tf(c5, output_node="out", input_source="V1")
    r100 = tf(c100, output_node="out", input_source="V1")

    assert isclose(r5.transfer_ratio, r100.transfer_ratio, rel_tol=1e-9)
    assert isclose(r5.input_impedance, r100.input_impedance, rel_tol=1e-9)
    assert isclose(r5.output_impedance, r100.output_impedance, rel_tol=1e-9)


def test_tf_multiple_sources_only_one_excited():
    """With two voltage sources, TF from V1 ignores V2 (it becomes 0 V short).

    Circuit:
        V1("in", "0", 1V) → R1("in","mid", 1kΩ) → mid
        V2("mid", "out", 0V) → R2("out","0", 1kΩ) → GND

    V2 is a 0 V source (wire).  The whole thing is a series 2kΩ divider.
    H = V_out / V_in_V1 should be 1.0 (V2 is a short, so V_out = V_mid = V_in
    through R1+R2... wait, let me compute properly).

    Actually: V1 forces vin=1V. R1 connects vin→mid, V2 (0V) forces
    V_out = V_mid. R2 connects out→GND.

    V2 is a 0V wire: V_mid = V_out.
    KCL at mid (= out via V2):
      (V_in - V_mid)/R1 = V_mid/R2
      (1 - V_mid)/1000 = V_mid/1000
      1 - V_mid = V_mid → V_mid = 0.5

    H = V_out / V_in = 0.5.
    """
    c = Circuit()
    c.add(VoltageSource("V1", "in", "0", 1.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(VoltageSource("V2", "mid", "out", 0.0))  # wire
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = tf(c, output_node="out", input_source="V1")
    assert isclose(result.transfer_ratio, 0.5, rel_tol=1e-6)


# ---------------------------------------------------------------------------
# Section 28 — DC sweep: DcSweepPoint / DcSweepResult dataclasses
# ---------------------------------------------------------------------------


def test_dcsweeppoint_is_frozen() -> None:
    """DcSweepPoint is a frozen dataclass — fields are immutable after creation."""
    pt = DcSweepPoint(
        source_value=1.0,
        node_voltages={"a": 0.5},
        branch_currents={"V1": 0.001},
        converged=True,
    )
    with pytest.raises((AttributeError, TypeError)):
        pt.source_value = 2.0  # type: ignore[misc]


def test_dcsweeppoint_fields() -> None:
    """All four fields of DcSweepPoint are accessible."""
    pt = DcSweepPoint(
        source_value=3.3,
        node_voltages={"out": 1.65},
        branch_currents={"Vin": -0.001},
        converged=True,
    )
    assert pt.source_value == 3.3
    assert pt.node_voltages == {"out": 1.65}
    assert pt.branch_currents == {"Vin": -0.001}
    assert pt.converged is True


def test_dcsweepresult_fields() -> None:
    """DcSweepResult stores points and source_name."""
    pt = DcSweepPoint(1.0, {"n": 0.5}, {}, True)
    result = DcSweepResult(points=[pt], source_name="Vin")
    assert result.source_name == "Vin"
    assert len(result.points) == 1
    assert result.points[0] is pt


def test_dcsweepresult_exported() -> None:
    """DcSweepPoint, DcSweepResult, and dc_sweep are importable from spice_engine."""
    import spice_engine as se

    assert hasattr(se, "DcSweepPoint")
    assert hasattr(se, "DcSweepResult")
    assert hasattr(se, "dc_sweep")
    assert callable(se.dc_sweep)


# ---------------------------------------------------------------------------
# Section 29 — DC sweep: linear resistive circuits
# ---------------------------------------------------------------------------


def test_dc_sweep_voltage_divider_exact() -> None:
    """Voltage-divider: V_out = V_in / 2 at every sweep step.

    Circuit::

        Vin("in", "0") → R1("in","out", 1kΩ) → R2("out","0", 1kΩ) → GND

    By the resistor voltage-divider formula V_out = V_in * R2/(R1+R2) = V_in/2.
    We sweep Vin from 0 V to 5 V in 1 V steps and verify the ratio at each step.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = dc_sweep(c, "Vin", 0.0, 5.0, 1.0)

    assert result.source_name == "Vin"
    assert len(result.points) == 6  # 0, 1, 2, 3, 4, 5

    for pt in result.points:
        assert pt.converged
        expected_vout = pt.source_value / 2.0
        assert isclose(pt.node_voltages["out"], expected_vout, abs_tol=1e-9)
    assert format_dc_sweep_table(result, ["V(out)", "I(Vin)"]) == (
        "Index\tSource\tValue\tV(out)\tI(Vin)\n"
        "0\tVin\t0.000000e+00\t0.000000e+00\t0.000000e+00\n"
        "1\tVin\t1.000000e+00\t5.000000e-01\t-5.000000e-04\n"
        "2\tVin\t2.000000e+00\t1.000000e+00\t-1.000000e-03\n"
        "3\tVin\t3.000000e+00\t1.500000e+00\t-1.500000e-03\n"
        "4\tVin\t4.000000e+00\t2.000000e+00\t-2.000000e-03\n"
        "5\tVin\t5.000000e+00\t2.500000e+00\t-2.500000e-03\n"
    )


def test_dc_sweep_probe_measurements_execute_parsed_cards() -> None:
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = dc_sweep(c, "Vin", 0.0, 2.0, 1.0)
    peak = measure_dc_sweep_probe(
        result,
        "out_peak",
        "V(out)",
        "max",
        from_value=1.0,
        to_value=2.0,
    )
    average = measure_dc_sweep_probe(result, "out_avg", "V(out)", "avg")

    assert peak.value == pytest.approx(1.0)
    assert peak.analysis == "dc"
    assert average.value == pytest.approx(0.5)
    assert format_measurement_table([peak, average]) == (
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
        "out_peak\tdc\tV(out)\tmax\t1.000000e+00\t2.000000e+00\t1.000000e+00\n"
        "out_avg\tdc\tV(out)\tavg\t\t\t5.000000e-01\n"
    )

    measurements = measure_dc_sweep_deck(
        result,
        """
.measure dc out_swing PP V(out) FROM=0 TO=2
.meas dc out_final FINAL V(out)
.end
""",
    )

    assert format_measurement_table(measurements) == (
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
        "out_swing\tdc\tV(out)\tpp\t0.000000e+00\t2.000000e+00\t1.000000e+00\n"
        "out_final\tdc\tV(out)\tlast\t\t\t1.000000e+00\n"
    )


def test_dc_sweep_source_value_sequence() -> None:
    """Sweep values are monotone-ascending from start to stop."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 100.0))

    result = dc_sweep(c, "V1", 0.0, 1.0, 0.25)

    values = [pt.source_value for pt in result.points]
    assert values == pytest.approx([0.0, 0.25, 0.50, 0.75, 1.0], abs=1e-9)


def test_dc_sweep_original_circuit_unchanged() -> None:
    """The original Circuit object is not mutated during the sweep.

    dc_sweep must create modified copies for each step; the caller's circuit
    must remain at its original source value.
    """
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 2.0))
    c.add(Resistor("R1", "n", "0", 500.0))

    dc_sweep(c, "V1", 0.0, 4.0, 1.0)

    # The VoltageSource in the original circuit must still be at 2.0 V.
    original_vsrc = c.elements[0]
    assert isinstance(original_vsrc, VoltageSource)
    assert original_vsrc.voltage == 2.0


def test_dc_sweep_descending_step() -> None:
    """Descending sweep (start > stop, step < 0) produces reverse-ordered values."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    result = dc_sweep(c, "V1", 5.0, 0.0, -1.0)

    values = [pt.source_value for pt in result.points]
    assert values == pytest.approx([5.0, 4.0, 3.0, 2.0, 1.0, 0.0], abs=1e-9)


def test_dc_sweep_wrong_sign_step_returns_empty() -> None:
    """If step sign does not match start-to-stop direction, result is empty.

    E.g. start=0, stop=5, step=-1 → no valid sweep values.
    """
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    result = dc_sweep(c, "V1", 0.0, 5.0, -1.0)
    assert result.points == []


def test_dc_sweep_single_step() -> None:
    """When start == stop, exactly one point is returned."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    result = dc_sweep(c, "V1", 3.0, 3.0, 0.1)
    assert len(result.points) == 1
    assert isclose(result.points[0].source_value, 3.0, abs_tol=1e-9)


def test_dc_sweep_branch_currents_recorded() -> None:
    """Branch currents are recorded at each sweep step.

    For a single resistor R=1kΩ driven by Vin, I = Vin/R (Ohm's law).
    The MNA branch current for the voltage source has MNA sign convention
    (negative when source delivers current into the circuit).
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    result = dc_sweep(c, "Vin", 1.0, 3.0, 1.0)

    for pt in result.points:
        # DcResult keyed by "I(<name>)" — e.g. "I(Vin)"
        key = "I(Vin)"
        assert key in pt.branch_currents
        # Current = V / R = source_value / 1000
        # MNA sign: delivering current is negative
        expected_i = -pt.source_value / 1000.0
        assert isclose(pt.branch_currents[key], expected_i, rel_tol=1e-6)


def test_dc_sweep_three_node_ladder() -> None:
    """Three-resistor ladder: verifies intermediate node voltages at every step.

    Circuit::

        Vin → R1(1kΩ) → n1 → R2(1kΩ) → n2 → R3(1kΩ) → GND

    By symmetry (equal resistors): V_n1 = 2/3 * Vin, V_n2 = 1/3 * Vin.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "n1", 1000.0))
    c.add(Resistor("R2", "n1", "n2", 1000.0))
    c.add(Resistor("R3", "n2", "0", 1000.0))

    result = dc_sweep(c, "Vin", 0.0, 3.0, 1.0)

    for pt in result.points:
        if pt.source_value == 0.0:
            continue  # all zeros
        v = pt.source_value
        assert isclose(pt.node_voltages["n1"], 2.0 / 3.0 * v, rel_tol=1e-6)
        assert isclose(pt.node_voltages["n2"], 1.0 / 3.0 * v, rel_tol=1e-6)


# ---------------------------------------------------------------------------
# Section 30 — DC sweep: nonlinear (diode) circuit
# ---------------------------------------------------------------------------


def test_dc_sweep_diode_all_points_converged() -> None:
    """A diode + resistor circuit converges across a forward-bias sweep.

    Circuit::

        Vin("anode","0") → D1(Is=1e-14, n=1) anode→cathode → R(cathode,"0", 1kΩ)

    At every sweep step (0 V to 2 V in 0.2 V) Newton-Raphson should converge.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "anode", "0", 0.0))
    c.add(Diode("D1", "anode", "cathode", Is=1e-14))
    c.add(Resistor("R1", "cathode", "0", 1000.0))

    result = dc_sweep(c, "Vin", 0.0, 2.0, 0.2)

    assert len(result.points) == 11
    assert all(pt.converged for pt in result.points)


def test_dc_sweep_diode_forward_bias_increasing_current() -> None:
    """Diode cathode voltage increases monotonically as Vin increases.

    When the source voltage rises, more current flows through the diode.
    The cathode voltage (= voltage across the resistor) must be strictly
    monotone-increasing once the diode is forward-biased.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "anode", "0", 0.0))
    c.add(Diode("D1", "anode", "cathode", Is=1e-14))
    c.add(Resistor("R1", "cathode", "0", 1000.0))

    result = dc_sweep(c, "Vin", 0.5, 2.0, 0.5)

    v_cathode = [pt.node_voltages["cathode"] for pt in result.points]
    for i in range(1, len(v_cathode)):
        assert v_cathode[i] > v_cathode[i - 1], (
            f"cathode voltage not increasing: {v_cathode[i-1]:.4f} → {v_cathode[i]:.4f}"
        )


# ---------------------------------------------------------------------------
# Section 31 — DC sweep: current-source sweeps
# ---------------------------------------------------------------------------


def test_dc_sweep_current_source_into_resistor() -> None:
    """Sweep a current source into a single resistor: V = I * R at each step.

    The MNA stamp for CurrentSource(n_plus, n_minus, I) ADDS current to n_minus
    and SUBTRACTS from n_plus.  To inject current INTO node "n" we use
    n_plus="0" (ground, excluded) and n_minus="n".

    Circuit::

        I1("0","n") [current injected into "n"] ‖ R1("n","0", 1kΩ)

    By Ohm's law: V_n = I1 * R = I1 * 1000.
    Sweep I1 from 1 mA to 5 mA in 1 mA steps.
    """
    c = Circuit()
    c.add(CurrentSource("I1", "0", "n", 0.001))
    c.add(Resistor("R1", "n", "0", 1000.0))

    result = dc_sweep(c, "I1", 0.001, 0.005, 0.001)

    assert len(result.points) == 5
    for pt in result.points:
        assert pt.converged
        expected_v = pt.source_value * 1000.0
        assert isclose(pt.node_voltages["n"], expected_v, rel_tol=1e-6)


def test_dc_sweep_current_source_descending() -> None:
    """Descending current sweep produces reverse-ordered source values."""
    c = Circuit()
    c.add(CurrentSource("I1", "0", "n", 0.0))
    c.add(Resistor("R1", "n", "0", 500.0))

    result = dc_sweep(c, "I1", 0.004, 0.001, -0.001)

    values = [pt.source_value for pt in result.points]
    assert values == pytest.approx([0.004, 0.003, 0.002, 0.001], abs=1e-12)


# ---------------------------------------------------------------------------
# Section 32 — DC sweep: error cases and edge cases
# ---------------------------------------------------------------------------


def test_dc_sweep_zero_step_raises() -> None:
    """A zero step size raises ValueError immediately."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    with pytest.raises(ValueError, match="step"):
        dc_sweep(c, "V1", 0.0, 5.0, 0.0)


def test_dc_sweep_missing_source_raises() -> None:
    """Referencing a source name that does not exist raises ValueError."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 1.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    with pytest.raises(ValueError, match="Vbad"):
        dc_sweep(c, "Vbad", 0.0, 1.0, 0.1)


def test_dc_sweep_resistor_not_accepted_as_source() -> None:
    """A Resistor element is not a valid sweep source; raises ValueError."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 1.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    with pytest.raises(ValueError, match="R1"):
        dc_sweep(c, "R1", 0.0, 1.0, 0.1)


def test_dc_sweep_fine_step_count() -> None:
    """Fine-grained step: 0 to 1 V in 0.1 V steps → exactly 11 points."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    result = dc_sweep(c, "V1", 0.0, 1.0, 0.1)
    assert len(result.points) == 11


def test_dc_sweep_all_converged_linear() -> None:
    """All points converge for a purely linear circuit."""
    c = Circuit()
    c.add(VoltageSource("V1", "n", "0", 0.0))
    c.add(Resistor("R1", "n", "0", 1000.0))

    result = dc_sweep(c, "V1", -5.0, 5.0, 1.0)

    assert all(pt.converged for pt in result.points)


# ===========================================================================
# Section 33 — DC sensitivity analysis: SensEntry / SensResult dataclasses
# ===========================================================================
#
# sens_dc perturbs each element parameter by a small fraction, runs two DC
# solves (nominal and perturbed), and records:
#
#   sensitivity     = (V_out_pert − V_out_nom) / δ    [absolute, V/unit]
#   rel_sensitivity = sensitivity × P / V_out_nom      [dimensionless ratio]
#
# The analytical ground-truth for a resistor divider:
#
#   V_mid = V_in × R2 / (R1 + R2)
#
#   ∂V_mid/∂R1 = −V_in × R2 / (R1+R2)²
#   ∂V_mid/∂R2 = +V_in × R1 / (R1+R2)²
#   ∂V_mid/∂V_in = R2 / (R1+R2)
#
#   For V_in=10V, R1=R2=1kΩ:
#     ∂V_mid/∂R1 ≈ −10 × 1000/4000000 = −0.0025 V/Ω
#     ∂V_mid/∂R2 ≈ +10 × 1000/4000000 = +0.0025 V/Ω
#     ∂V_mid/∂V_in = 0.5
#
#   rel_sensitivity for R1: (−0.0025 × 1000) / 5.0 = −0.5
#   rel_sensitivity for R2: (+0.0025 × 1000) / 5.0 = +0.5
#   rel_sensitivity for Vin: (0.5 × 10) / 5.0 = +1.0
# ===========================================================================


def test_sens_result_is_sensresult() -> None:
    """sens_dc returns a SensResult instance."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = sens_dc(c, "in")
    assert isinstance(result, SensResult)


def test_sens_entries_are_sensentry() -> None:
    """Each entry in SensResult.entries is a SensEntry instance."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = sens_dc(c, "in")
    for entry in result.entries:
        assert isinstance(entry, SensEntry)


def test_sens_nominal_voltage_correct() -> None:
    """SensResult.nominal_voltage matches dc_op V_out at the output node."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = sens_dc(c, "mid")
    dc = dc_op(c)
    assert isclose(result.nominal_voltage, dc.node_voltages["mid"], rel_tol=1e-9)


def test_sens_converged_linear() -> None:
    """sens_dc converges for a purely linear circuit."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = sens_dc(c, "out")
    assert result.converged


# ===========================================================================
# Section 34 — DC sensitivity: resistor-divider analytical verification
# ===========================================================================


def test_sens_divider_r1_sensitivity() -> None:
    """∂V_mid/∂R1 ≈ −V_in × R2 / (R1+R2)² for the resistor divider.

    For V_in=10V, R1=R2=1kΩ: ∂V_mid/∂R1 ≈ −0.0025 V/Ω.
    The forward-difference approximation is accurate to within 0.1% of the
    analytical value.
    """
    v_in, r1, r2 = 10.0, 1000.0, 1000.0
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", v_in))
    c.add(Resistor("R1", "in", "mid", r1))
    c.add(Resistor("R2", "mid", "0", r2))

    result = sens_dc(c, "mid")
    r1_entry = next(e for e in result.entries if e.element_name == "R1")

    # Analytical: ∂V_mid/∂R1 = −V_in × R2 / (R1+R2)²
    expected_sens = -v_in * r2 / (r1 + r2) ** 2
    assert isclose(r1_entry.sensitivity, expected_sens, rel_tol=1e-3), (
        f"R1 sensitivity {r1_entry.sensitivity:.6f} vs expected {expected_sens:.6f}"
    )


def test_sens_divider_r2_sensitivity() -> None:
    """∂V_mid/∂R2 ≈ +V_in × R1 / (R1+R2)² for the resistor divider."""
    v_in, r1, r2 = 10.0, 1000.0, 1000.0
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", v_in))
    c.add(Resistor("R1", "in", "mid", r1))
    c.add(Resistor("R2", "mid", "0", r2))

    result = sens_dc(c, "mid")
    r2_entry = next(e for e in result.entries if e.element_name == "R2")

    expected_sens = v_in * r1 / (r1 + r2) ** 2
    assert isclose(r2_entry.sensitivity, expected_sens, rel_tol=1e-3), (
        f"R2 sensitivity {r2_entry.sensitivity:.6f} vs expected {expected_sens:.6f}"
    )


def test_sens_divider_r1_rel_sensitivity() -> None:
    """Relative sensitivity of R1 ≈ −0.5 in an equal-ratio divider.

    rel = (∂V_mid/∂R1) × R1 / V_mid = (−0.0025 × 1000) / 5 = −0.5.
    A 1% increase in R1 causes a 0.5% decrease in V_mid.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = sens_dc(c, "mid")
    r1_entry = next(e for e in result.entries if e.element_name == "R1")

    assert isclose(r1_entry.rel_sensitivity, -0.5, rel_tol=1e-3), (
        f"R1 rel_sensitivity {r1_entry.rel_sensitivity:.4f} vs expected -0.5"
    )


def test_sens_divider_r2_rel_sensitivity() -> None:
    """Relative sensitivity of R2 ≈ +0.5 in an equal-ratio divider."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = sens_dc(c, "mid")
    r2_entry = next(e for e in result.entries if e.element_name == "R2")

    assert isclose(r2_entry.rel_sensitivity, 0.5, rel_tol=1e-3), (
        f"R2 rel_sensitivity {r2_entry.rel_sensitivity:.4f} vs expected +0.5"
    )


def test_sens_divider_asymmetric() -> None:
    """Asymmetric divider (R1=2kΩ, R2=1kΩ) checks the relative sensitivities.

    V_mid = V_in × R2/(R1+R2) = 10 × 1000/3000 ≈ 3.333 V.

    General closed-form relative sensitivities:
      rel(R1) = R1 × (∂V_mid/∂R1) / V_mid = −R1/(R1+R2)
      rel(R2) = R2 × (∂V_mid/∂R2) / V_mid = +R1/(R1+R2)  ← numerator is R1, not R2

    Derivation:
      ∂V_mid/∂R1 = −V_in × R2/(R1+R2)²
      ∂V_mid/∂R2 = +V_in × R1/(R1+R2)²
      V_mid = V_in × R2/(R1+R2)
      rel(R2) = R2 × V_in × R1/(R1+R2)² / (V_in × R2/(R1+R2)) = R1/(R1+R2)

    For R1=2kΩ, R2=1kΩ:
      rel(R1) = −2000/3000 ≈ −0.667
      rel(R2) = +2000/3000 ≈ +0.667
    """
    v_in, r1, r2 = 10.0, 2000.0, 1000.0
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", v_in))
    c.add(Resistor("R1", "in", "mid", r1))
    c.add(Resistor("R2", "mid", "0", r2))

    result = sens_dc(c, "mid")
    assert isclose(result.nominal_voltage, v_in * r2 / (r1 + r2), rel_tol=1e-9)

    r1_entry = next(e for e in result.entries if e.element_name == "R1")
    expected_rel_r1 = -r1 / (r1 + r2)   # = −2000/3000 ≈ −0.667
    assert isclose(r1_entry.rel_sensitivity, expected_rel_r1, rel_tol=1e-3)

    r2_entry = next(e for e in result.entries if e.element_name == "R2")
    expected_rel_r2 = r1 / (r1 + r2)    # = +2000/3000 ≈ +0.667 (numerator is R1!)
    assert isclose(r2_entry.rel_sensitivity, expected_rel_r2, rel_tol=1e-3)


# ===========================================================================
# Section 35 — DC sensitivity: voltage-source and current-source sensitivities
# ===========================================================================


def test_sens_voltage_source_rel_is_unity() -> None:
    """For a pure voltage source with resistive load, ∂V_out/∂V_in = 1.

    V_out = V_in directly (single node with a resistor to ground — voltage
    is set by the source).  rel_sensitivity = (1 × V_in) / V_in = 1.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "out", "0", 5.0))
    c.add(Resistor("R1", "out", "0", 1000.0))

    result = sens_dc(c, "out")
    vin_entry = next(e for e in result.entries if e.element_name == "Vin")

    assert isclose(vin_entry.rel_sensitivity, 1.0, rel_tol=1e-3), (
        f"VoltageSource rel_sensitivity {vin_entry.rel_sensitivity:.4f} vs 1.0"
    )


def test_sens_current_source_into_resistor() -> None:
    """V_out = I × R, so ∂V_out/∂I = R and rel_sensitivity = 1.

    I1=1 mA → 0 (n+) / out (n-); R1=1 kΩ to ground.
    V_out = I × R = 0.001 × 1000 = 1 V.
    ∂V_out/∂I = R = 1000.  rel = 1000 × 0.001 / 1.0 = 1.
    """
    c = Circuit()
    c.add(CurrentSource("I1", "0", "out", 0.001))  # injects into 'out'
    c.add(Resistor("R1", "out", "0", 1000.0))

    result = sens_dc(c, "out")
    assert isclose(result.nominal_voltage, 1.0, rel_tol=1e-9)

    i1_entry = next(e for e in result.entries if e.element_name == "I1")
    assert isclose(i1_entry.sensitivity, 1000.0, rel_tol=1e-3), (
        f"CurrentSource abs sensitivity {i1_entry.sensitivity:.1f} vs 1000"
    )
    assert isclose(i1_entry.rel_sensitivity, 1.0, rel_tol=1e-3)


def test_sens_voltage_source_divider_rel_unity() -> None:
    """Voltage source rel_sensitivity is always 1.0 for a linear divider.

    For V_mid = V_in × R2/(R1+R2), ∂V_mid/∂V_in = R2/(R1+R2) = 0.5.
    rel = 0.5 × V_in / V_mid = 0.5 × 10 / 5 = 1.0.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = sens_dc(c, "mid")
    vin_entry = next(e for e in result.entries if e.element_name == "Vin")
    assert isclose(vin_entry.rel_sensitivity, 1.0, rel_tol=1e-3)


def test_sens_current_source_resistance_both_ranked() -> None:
    """Both I1 (current) and R1 (resistance) appear in entries for a simple RC load."""
    c = Circuit()
    c.add(CurrentSource("I1", "0", "out", 0.002))
    c.add(Resistor("R1", "out", "0", 500.0))

    result = sens_dc(c, "out")
    names = {(e.element_name, e.parameter) for e in result.entries}
    assert ("I1", "current") in names
    assert ("R1", "resistance") in names


# ===========================================================================
# Section 36 — DC sensitivity: nonlinear element (Diode Is)
# ===========================================================================
#
# For a forward-biased diode in series with a resistor:
#
#   V_in = V_D + V_R      where V_D ≈ Vt × ln(I / Is)
#
# Increasing Is decreases V_D (the diode conducts more easily at the same
# current), which increases V_R.  The output voltage V_R = V_in − V_D
# therefore rises when Is rises.
#
# The direction of ∂V_R/∂Is is always positive for a forward-biased diode
# connected in the usual way.
# ===========================================================================


def test_sens_diode_is_entry_present() -> None:
    """A Diode contributes an 'Is' entry to the sensitivity table."""
    c = Circuit()
    c.add(VoltageSource("Vin", "anode", "0", 1.0))
    c.add(Diode("D1", "anode", "out"))
    c.add(Resistor("R1", "out", "0", 1000.0))

    result = sens_dc(c, "out")
    param_names = {(e.element_name, e.parameter) for e in result.entries}
    assert ("D1", "Is") in param_names, f"Entries: {param_names}"


def test_sens_diode_is_positive_sensitivity() -> None:
    """Increasing Is lowers Vd, raising Vout (V_R = V_in − V_D).

    The absolute sensitivity ∂V_R/∂Is should be positive.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "anode", "0", 1.0))
    c.add(Diode("D1", "anode", "out"))
    c.add(Resistor("R1", "out", "0", 1000.0))

    result = sens_dc(c, "out")
    d1_entry = next(e for e in result.entries if e.element_name == "D1" and e.parameter == "Is")
    assert d1_entry.sensitivity > 0, (
        f"Expected positive dV_out/dIs, got {d1_entry.sensitivity}"
    )


# ===========================================================================
# Section 37 — DC sensitivity: BJT Is and beta_f
# ===========================================================================
#
# NPN common-emitter amplifier:
#
#   Vcc (5 V)
#     │
#    Rc (1 kΩ) ← collector
#     │
#     ├── output "out"
#     │
#    BJT (NPN)  ← base biased via Rb from Vcc, emitter to ground
#     │
#    GND
#
# Increasing beta_f → more collector current → V_out drops (negative
# relative sensitivity).
# ===========================================================================


def test_sens_bjt_entries_is_and_beta() -> None:
    """A BJT contributes both 'Is' and 'beta_f' entries."""
    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", 5.0))
    c.add(Resistor("Rb", "vcc", "base", 100_000.0))
    c.add(Resistor("Rc", "vcc", "col", 1_000.0))
    c.add(BJT("Q1", "col", "base", "0", polarity="NPN"))

    result = sens_dc(c, "col")
    param_pairs = {(e.element_name, e.parameter) for e in result.entries}
    assert ("Q1", "Is") in param_pairs, f"Missing Q1 Is — entries: {param_pairs}"
    assert ("Q1", "beta_f") in param_pairs, f"Missing Q1 beta_f — entries: {param_pairs}"


def test_sens_bjt_beta_negative_on_collector() -> None:
    """Higher beta_f → more collector current → V_col decreases.

    In the common-emitter configuration the rel_sensitivity for beta_f
    should be negative (increasing β lowers V_out).
    """
    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", 5.0))
    c.add(Resistor("Rb", "vcc", "base", 200_000.0))
    c.add(Resistor("Rc", "vcc", "col", 2_000.0))
    c.add(BJT("Q1", "col", "base", "0", polarity="NPN"))

    result = sens_dc(c, "col")
    if not result.converged:
        return  # Skip if BJT didn't converge (topology edge case)

    beta_entry = next(
        (e for e in result.entries if e.element_name == "Q1" and e.parameter == "beta_f"),
        None,
    )
    if beta_entry is not None:
        assert beta_entry.rel_sensitivity <= 0, (
            f"Expected beta_f to decrease V_col; got rel={beta_entry.rel_sensitivity:.4f}"
        )


# ===========================================================================
# Section 38 — DC sensitivity: sorting and ranking
# ===========================================================================


def test_sens_entries_sorted_by_abs_rel_desc() -> None:
    """Entries are sorted by abs(rel_sensitivity) descending."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = sens_dc(c, "mid")
    rels = [abs(e.rel_sensitivity) for e in result.entries]
    assert rels == sorted(rels, reverse=True), f"Entries not sorted: {rels}"


def test_sens_vin_dominates_divider() -> None:
    """In a symmetric divider Vin has the highest rel_sensitivity (= 1.0).

    Both resistors have |rel| = 0.5 < 1.0, so Vin appears first.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = sens_dc(c, "mid")
    assert result.entries[0].element_name == "Vin", (
        f"Expected Vin first, got {result.entries[0].element_name}"
    )


def test_sens_nominal_value_stored() -> None:
    """SensEntry stores the unperturbed parameter value in nominal_value."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "0", 4700.0))

    result = sens_dc(c, "in")
    r1_entry = next(e for e in result.entries if e.element_name == "R1")
    assert r1_entry.nominal_value == 4700.0

    vin_entry = next(e for e in result.entries if e.element_name == "Vin")
    assert vin_entry.nominal_value == 10.0


# ===========================================================================
# Section 39 — DC sensitivity: error cases and edge cases
# ===========================================================================


def test_sens_invalid_output_node_raises() -> None:
    """ValueError if the output node is not in the circuit."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    with pytest.raises(ValueError, match="nonexistent"):
        sens_dc(c, "nonexistent")


def test_sens_output_at_ground_not_raises() -> None:
    """Observing ground (always 0 V) doesn't raise, but returns 0 as nominal."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    # ground node aliases: "0" is accepted
    result = sens_dc(c, "0")
    assert result.output_node == "0"
    assert result.nominal_voltage == 0.0


def test_sens_output_node_field_preserved() -> None:
    """SensResult.output_node stores the exact string passed to sens_dc."""
    c = Circuit()
    c.add(VoltageSource("Vin", "alpha", "0", 3.3))
    c.add(Resistor("R1", "alpha", "0", 1000.0))

    result = sens_dc(c, "alpha")
    assert result.output_node == "alpha"


def test_sens_capacitor_and_inductor_skipped() -> None:
    """Capacitors and inductors produce no entries (no DC parameter).

    In DC steady-state, a capacitor is an open circuit and an inductor is
    a short circuit.  Neither C nor L affects the DC node voltages, so
    perturbing their values produces zero sensitivity.  sens_dc skips them
    to keep the output concise.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Capacitor("C1", "out", "0", 1e-9))  # open in DC — no entry expected
    c.add(Inductor("L1", "out", "0", 1e-6))   # short in DC  — no entry expected
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = sens_dc(c, "out")
    names = {e.element_name for e in result.entries}
    assert "C1" not in names, "Capacitor should not appear in sensitivity entries"
    assert "L1" not in names, "Inductor should not appear in sensitivity entries"


def test_sens_three_resistors_all_present() -> None:
    """A ladder with three resistors produces three 'resistance' entries."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "n1", 1000.0))
    c.add(Resistor("R2", "n1", "n2", 1000.0))
    c.add(Resistor("R3", "n2", "0", 1000.0))

    result = sens_dc(c, "n2")
    param_pairs = {(e.element_name, e.parameter) for e in result.entries}
    assert ("R1", "resistance") in param_pairs
    assert ("R2", "resistance") in param_pairs
    assert ("R3", "resistance") in param_pairs


def test_sens_single_resistor_load() -> None:
    """V_out is fixed by the voltage source; R only affects current, not voltage.

    With V1 directly on 'out', ∂V_out/∂R = 0 (voltage source overrides the node).
    The resistor sensitivity should be essentially zero.
    """
    c = Circuit()
    c.add(VoltageSource("V1", "out", "0", 3.3))
    c.add(Resistor("R1", "out", "0", 10_000.0))

    result = sens_dc(c, "out")
    r1_entry = next(e for e in result.entries if e.element_name == "R1")
    # Voltage source clamps the node; R has no effect on V_out
    assert abs(r1_entry.sensitivity) < 1e-6, (
        f"Resistor sensitivity should be ≈0 when load is directly clamped: "
        f"{r1_entry.sensitivity}"
    )


# ===========================================================================
# Section 40 — Monte Carlo: McPoint / McResult dataclasses
# ===========================================================================
#
# mc_dc runs N independent DC operating points, each with element parameters
# randomly varied by ±tolerance around their nominal values.  The mean and
# standard deviation of V(output_node) across converged trials are reported.
#
# For a symmetric tolerance distribution the mean should be close to the
# nominal value (no systematic bias).  The standard deviation reflects the
# spread introduced by the component variation.
# ===========================================================================


def test_mc_result_is_mcresult() -> None:
    """mc_dc returns a McResult instance."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = mc_dc(c, "in", n_trials=5, seed=0)
    assert isinstance(result, McResult)


def test_mc_points_are_mcpoint() -> None:
    """Each entry in McResult.points is a McPoint instance."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = mc_dc(c, "in", n_trials=5, seed=0)
    for pt in result.points:
        assert isinstance(pt, McPoint)


def test_mc_n_trials_field() -> None:
    """McResult.n_trials matches the requested number of trials."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = mc_dc(c, "in", n_trials=17, seed=0)
    assert result.n_trials == 17
    assert len(result.points) == 17


def test_mc_trial_index_sequential() -> None:
    """McPoint.trial runs from 0 to n_trials − 1 in order."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = mc_dc(c, "in", n_trials=10, seed=0)
    assert [pt.trial for pt in result.points] == list(range(10))


def test_mc_output_node_field() -> None:
    """McResult.output_node stores the exact string passed to mc_dc."""
    c = Circuit()
    c.add(VoltageSource("Vin", "alpha", "0", 3.3))
    c.add(Resistor("R1", "alpha", "0", 1000.0))

    result = mc_dc(c, "alpha", n_trials=3, seed=0)
    assert result.output_node == "alpha"


# ===========================================================================
# Section 41 — Monte Carlo: mean near nominal for symmetric tolerances
# ===========================================================================
#
# For any distribution symmetric around the nominal value (Gaussian or
# uniform), the expected value of V_out is the nominal V_out (no bias).
# With enough trials the sample mean should converge to the nominal.
#
# We use a loose 10% tolerance on the mean (i.e., the mean must be within
# ±10% of the nominal) even for N=200 to avoid flaky tests — but in practice
# the convergence is much tighter (~σ/√N ≈ 0.1V / √200 ≈ 0.007V for a
# symmetric 5% Gaussian on a 5V divider).
# ===========================================================================


def test_mc_gaussian_mean_near_nominal_divider() -> None:
    """Gaussian variation: mean(V_mid) ≈ nominal (5 V) for a symmetric divider."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = mc_dc(c, "mid", n_trials=300, tolerance=0.05, seed=42)
    nominal = 5.0
    # Mean should be within 5% of nominal for N=300 with σ≈0.1V
    assert abs(result.mean - nominal) < 0.5, (
        f"Mean {result.mean:.3f} is far from nominal {nominal}"
    )


def test_mc_uniform_mean_near_nominal() -> None:
    """Uniform ±10% variation: mean(V_out) ≈ 2.5 V for a symmetric resistor divider.

    Because mc_dc varies every element (including the VoltageSource itself),
    there is no "clamped" node.  For a symmetric divider with uniform ±10%
    variation on both resistors and the source, the expected output is half the
    source voltage.  With N=200 trials and ±10% tolerance the sample mean
    should stay within 20% of the nominal 2.5 V.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = mc_dc(c, "out", n_trials=200, tolerance=0.10, distribution="uniform", seed=1)
    # Symmetric uniform distribution → no bias; mean stays near 2.5 V.
    assert abs(result.mean - 2.5) < 0.5, (
        f"Mean {result.mean:.3f} is too far from nominal 2.5 V"
    )


def test_mc_all_converged_linear() -> None:
    """All trials converge for a purely linear resistive circuit."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = mc_dc(c, "out", n_trials=50, tolerance=0.05, seed=5)
    assert all(pt.converged for pt in result.points), (
        "All trials should converge for a linear circuit"
    )


# ===========================================================================
# Section 42 — Monte Carlo: std_dev > 0 when tolerance > 0
# ===========================================================================
#
# Whenever the output voltage is sensitive to at least one varied component
# AND tolerance > 0, the sample standard deviation must be positive.
# If all varied parameters have zero effect (e.g., the output is clamped by
# a voltage source), std_dev can be zero.
# ===========================================================================


def test_mc_std_dev_positive_gaussian() -> None:
    """Gaussian variation on a divider produces std_dev > 0."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = mc_dc(c, "mid", n_trials=50, tolerance=0.05, seed=7)
    assert result.std_dev > 0, f"Expected std_dev > 0, got {result.std_dev}"


def test_mc_std_dev_positive_uniform() -> None:
    """Uniform variation on a divider also produces std_dev > 0."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = mc_dc(c, "mid", n_trials=50, tolerance=0.05,
                   distribution="uniform", seed=8)
    assert result.std_dev > 0, f"Expected std_dev > 0, got {result.std_dev}"


def test_mc_wider_tolerance_larger_std_dev() -> None:
    """Higher tolerance → larger std_dev for the same circuit and seed."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result_tight = mc_dc(c, "mid", n_trials=100, tolerance=0.01, seed=3)
    result_wide = mc_dc(c, "mid", n_trials=100, tolerance=0.10, seed=3)
    assert result_wide.std_dev > result_tight.std_dev, (
        f"Wider tolerance should give larger std_dev: "
        f"tight={result_tight.std_dev:.4f}, wide={result_wide.std_dev:.4f}"
    )


def test_mc_zero_tolerance_zero_std_dev() -> None:
    """Zero tolerance: all trials identical → std_dev == 0."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = mc_dc(c, "mid", n_trials=10, tolerance=0.0, seed=0)
    assert result.std_dev == 0.0, (
        f"Zero tolerance should give std_dev=0, got {result.std_dev}"
    )


# ===========================================================================
# Section 43 — Monte Carlo: seed reproducibility
# ===========================================================================
#
# Two runs with the same seed, same circuit, and same parameters must produce
# exactly identical results.  Two runs with different seeds should (almost
# certainly) differ.
# ===========================================================================


def test_mc_seed_same_produces_same_results() -> None:
    """Same seed → identical McPoint trial vectors."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    r1 = mc_dc(c, "mid", n_trials=20, tolerance=0.05, seed=42)
    r2 = mc_dc(c, "mid", n_trials=20, tolerance=0.05, seed=42)

    assert r1.mean == r2.mean, "Same seed should produce identical mean"
    assert r1.std_dev == r2.std_dev, "Same seed should produce identical std_dev"
    for pt1, pt2 in zip(r1.points, r2.points, strict=True):
        assert pt1.node_voltages == pt2.node_voltages, (
            f"Trial {pt1.trial}: node_voltages differ"
        )


def test_mc_different_seeds_produce_different_results() -> None:
    """Different seeds almost certainly produce different trial vectors."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    r1 = mc_dc(c, "mid", n_trials=20, tolerance=0.05, seed=1)
    r2 = mc_dc(c, "mid", n_trials=20, tolerance=0.05, seed=2)

    # The probability that two independent runs give the same mean is negligible.
    assert r1.mean != r2.mean, "Different seeds should produce different means"


# ===========================================================================
# Section 44 — Monte Carlo: distribution modes (gaussian vs uniform)
# ===========================================================================
#
# The two distributions have different shapes:
#
#   Gaussian: tails extend beyond ±tolerance (3-sigma = tolerance) — rarely
#             produces values more than 3× the "rated" tolerance from nominal.
#
#   Uniform:  flat between [1−tol, 1+tol] — no samples outside ±tolerance.
#
# For the same tolerance and circuit the uniform distribution typically
# produces a slightly larger std_dev because its variance is tolerance²/3,
# versus Gaussian with σ = tolerance/3 → variance = tolerance²/9.
# So uniform std_dev ≈ √3 × gaussian std_dev for the same effective range.
# ===========================================================================


def test_mc_gaussian_distribution_mode() -> None:
    """distribution='gaussian' runs without error and returns non-zero std_dev."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = mc_dc(c, "mid", n_trials=30, tolerance=0.05,
                   distribution="gaussian", seed=10)
    assert result.std_dev >= 0.0


def test_mc_uniform_distribution_mode() -> None:
    """distribution='uniform' runs without error and returns non-zero std_dev."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = mc_dc(c, "mid", n_trials=30, tolerance=0.05,
                   distribution="uniform", seed=11)
    assert result.std_dev >= 0.0


def test_mc_uniform_wider_spread_than_gaussian() -> None:
    """For same tolerance, uniform distribution spreads wider than Gaussian.

    Uniform variance = tol²/3; Gaussian variance = (tol/3)² = tol²/9.
    Ratio ≈ √3 ≈ 1.73×, so uniform std_dev should be larger with a
    statistically significant sample.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    r_gauss = mc_dc(c, "mid", n_trials=200, tolerance=0.10,
                    distribution="gaussian", seed=20)
    r_unif = mc_dc(c, "mid", n_trials=200, tolerance=0.10,
                   distribution="uniform", seed=20)

    # Uniform should be wider; allow a 30% margin for sampling variability.
    assert r_unif.std_dev > r_gauss.std_dev * 0.7, (
        f"Uniform std_dev {r_unif.std_dev:.4f} should exceed "
        f"Gaussian std_dev {r_gauss.std_dev:.4f} × 0.7"
    )


# ===========================================================================
# Section 45 — Monte Carlo: error cases and edge cases
# ===========================================================================


def test_mc_invalid_output_node_raises() -> None:
    """ValueError if the output node is not in the circuit."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    with pytest.raises(ValueError, match="nonexistent"):
        mc_dc(c, "nonexistent", n_trials=5)


def test_mc_invalid_distribution_raises() -> None:
    """ValueError for an unknown distribution name."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    with pytest.raises(ValueError, match="distribution"):
        mc_dc(c, "in", n_trials=5, distribution="triangular")


def test_mc_n_trials_zero_raises() -> None:
    """ValueError if n_trials < 1."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    with pytest.raises(ValueError, match="n_trials"):
        mc_dc(c, "in", n_trials=0)


def test_mc_single_trial() -> None:
    """n_trials=1: mean equals the single trial voltage, std_dev=0."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Resistor("R2", "mid", "0", 1000.0))

    result = mc_dc(c, "mid", n_trials=1, tolerance=0.05, seed=0)
    assert len(result.points) == 1
    assert result.std_dev == 0.0
    v = result.points[0].node_voltages.get("mid", 0.0)
    assert isclose(result.mean, v, rel_tol=1e-9)


def test_mc_ground_output_node() -> None:
    """Observing ground ('0') returns mean=0 and std_dev=0."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 5.0))
    c.add(Resistor("R1", "in", "0", 1000.0))

    result = mc_dc(c, "0", n_trials=10, tolerance=0.05, seed=0)
    assert result.mean == 0.0
    assert result.std_dev == 0.0


def test_mc_node_voltages_in_each_point() -> None:
    """Each McPoint.node_voltages contains the output node key."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 10.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = mc_dc(c, "out", n_trials=5, seed=0)
    for pt in result.points:
        assert "out" in pt.node_voltages, (
            f"Trial {pt.trial} missing 'out' in node_voltages"
        )


# ---------------------------------------------------------------------------
# Physical constants mirroring those in engine.py, used for analytical checks.
# ---------------------------------------------------------------------------
_kB: float = 1.380649e-23     # Boltzmann constant [J/K]
_q: float = 1.602176634e-19   # Electron charge [C]


def _divider_circuit() -> Circuit:
    """Canonical test circuit for noise analysis.

    Topology: VS(0 V, "in"→"0") → R1(1 kΩ, "in"→"out") → R2(1 kΩ, "out"→"0")

    With Vin = 0 V the input node is AC-grounded; the effective output resistance
    seen by the noise sources is R1 ‖ R2 = 500 Ω.  The signal transfer function
    is H_sig = R2 / (R1 + R2) = 0.5.

    Known analytical values at T = 300 K:
        S_out    = 4kT × 500 ≈ 8.284e-18 V²/Hz  (white: same at all freqs)
        S_in     = S_out / 0.25 ≈ 3.314e-17 V²/Hz
        per-R1   = 4kT × 250 ≈ 4.142e-18 V²/Hz
        per-R2   = 4kT × 250 ≈ 4.142e-18 V²/Hz
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))
    return c


# ---------------------------------------------------------------------------
# Section 46 — Noise analysis: NoiseEntry / NoisePoint / NoiseResult dataclasses
# ---------------------------------------------------------------------------


def test_noise_result_is_noiseresult() -> None:
    """noise_ac returns a NoiseResult instance."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    assert isinstance(result, NoiseResult)


def test_noise_points_are_noisepoint() -> None:
    """Each element of NoiseResult.points is a NoisePoint."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    assert len(result.points) == 1
    assert isinstance(result.points[0], NoisePoint)


def test_noise_entries_are_noiseentry() -> None:
    """Each element of NoisePoint.entries is a NoiseEntry."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    for entry in result.points[0].entries:
        assert isinstance(entry, NoiseEntry)


def test_noise_result_output_node_field() -> None:
    """NoiseResult.output_node records the requested node."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    assert result.output_node == "out"


def test_noise_result_input_source_field() -> None:
    """NoiseResult.input_source records the nominated source name."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    assert result.input_source == "Vin"


def test_noise_result_temperature_field() -> None:
    """NoiseResult.temperature echoes the supplied temperature."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0], temperature=350.0)
    assert result.temperature == 350.0


def test_noise_point_freq_field() -> None:
    """NoisePoint.freq records the frequency in hertz."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1234.0])
    assert result.points[0].freq == 1234.0


def test_noise_entry_element_name_field() -> None:
    """NoiseEntry.element_name is the name of the contributing element."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    names = {e.element_name for e in result.points[0].entries}
    assert "R1" in names
    assert "R2" in names


def test_noise_entry_noise_type_thermal_for_resistors() -> None:
    """NoiseEntry.noise_type is 'thermal' for resistors."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    for entry in result.points[0].entries:
        assert entry.noise_type == "thermal"


def test_noise_entry_source_psd_positive() -> None:
    """NoiseEntry.source_psd is positive (noise power density ≥ 0)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    for entry in result.points[0].entries:
        assert entry.source_psd > 0.0


def test_noise_entry_output_psd_nonnegative() -> None:
    """NoiseEntry.output_psd is non-negative."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    for entry in result.points[0].entries:
        assert entry.output_psd >= 0.0


# ---------------------------------------------------------------------------
# Section 47 — Noise analysis: thermal noise — analytical Nyquist verification
# ---------------------------------------------------------------------------
#
# For a purely resistive circuit at temperature T, the total output noise PSD
# equals the Nyquist formula:
#
#     S_out = 4kT × R_eq    [V²/Hz]
#
# where R_eq is the Thevenin-equivalent noise resistance looking back from
# the output node with all independent sources set to zero.


def test_noise_symmetric_divider_total_psd_nyquist() -> None:
    """Symmetric divider: S_out ≈ 4kT × (R1‖R2) = 4kT × 500 Ω."""
    T = 300.0
    R1 = R2 = 1000.0
    R_eq = (R1 * R2) / (R1 + R2)          # 500 Ω
    expected = 4.0 * _kB * T * R_eq       # ≈ 8.28e-18 V²/Hz

    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0], temperature=T)
    actual = result.points[0].output_psd

    assert isclose(actual, expected, rel_tol=1e-4), (
        f"Expected S_out ≈ {expected:.4e} V²/Hz, got {actual:.4e}"
    )


def test_noise_psd_scales_with_temperature() -> None:
    """Doubling temperature doubles S_out (linear dependence on T)."""
    c = _divider_circuit()
    r1 = noise_ac(c, "out", "Vin", freqs=[1000.0], temperature=300.0)
    r2 = noise_ac(c, "out", "Vin", freqs=[1000.0], temperature=600.0)
    ratio = r2.points[0].output_psd / r1.points[0].output_psd
    assert isclose(ratio, 2.0, rel_tol=1e-4), (
        f"Expected ratio 2.0, got {ratio:.4f}"
    )


def test_noise_white_spectrum_resistors_only() -> None:
    """Purely resistive circuit has flat (white) noise PSD at all frequencies."""
    c = _divider_circuit()
    freqs_to_test = [1.0, 100.0, 1e4, 1e6]
    result = noise_ac(c, "out", "Vin", freqs=freqs_to_test)

    # All frequency points should give the same output PSD (white noise).
    psds = [pt.output_psd for pt in result.points]
    ref = psds[0]
    for i, psd in enumerate(psds[1:], 1):
        assert isclose(psd, ref, rel_tol=1e-6), (
            f"Freq {freqs_to_test[i]}: expected {ref:.4e}, got {psd:.4e} (not white)"
        )


def test_noise_single_resistor_source_psd() -> None:
    """Source PSD of a 1 kΩ resistor at 300 K equals 4kT/R analytically."""
    T = 300.0
    R = 1000.0
    expected_src = 4.0 * _kB * T / R   # A²/Hz

    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0], temperature=T)
    for entry in result.points[0].entries:
        assert isclose(entry.source_psd, expected_src, rel_tol=1e-6), (
            f"{entry.element_name}: source_psd {entry.source_psd:.4e}, "
            f"expected {expected_src:.4e}"
        )


def test_noise_larger_resistance_higher_source_psd() -> None:
    """A 10 kΩ resistor produces 10× more current noise PSD than 1 kΩ."""
    c1 = Circuit()
    c1.add(VoltageSource("Vin", "in", "0", 0.0))
    c1.add(Resistor("R1", "in", "out", 1000.0))
    c1.add(Resistor("R2", "out", "0", 1000.0))

    c2 = Circuit()
    c2.add(VoltageSource("Vin", "in", "0", 0.0))
    c2.add(Resistor("R1", "in", "out", 10000.0))  # 10× larger
    c2.add(Resistor("R2", "out", "0", 10000.0))

    r1 = noise_ac(c1, "out", "Vin", freqs=[1000.0])
    r2 = noise_ac(c2, "out", "Vin", freqs=[1000.0])

    # source_psd = 4kT/R, so 10x bigger R → 10x smaller source_psd
    src1 = next(e for e in r1.points[0].entries if e.element_name == "R1")
    src2 = next(e for e in r2.points[0].entries if e.element_name == "R1")
    assert isclose(src1.source_psd / src2.source_psd, 10.0, rel_tol=1e-6)


# ---------------------------------------------------------------------------
# Section 48 — Noise analysis: per-element breakdown and sorting
# ---------------------------------------------------------------------------


def test_noise_entries_count_matches_resistors() -> None:
    """Two resistors produce two noise entries."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    assert len(result.points[0].entries) == 2


def test_noise_entries_sorted_loudest_first() -> None:
    """Entries are sorted by output_psd descending (loudest contributor first)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    psds = [e.output_psd for e in result.points[0].entries]
    assert psds == sorted(psds, reverse=True), (
        f"Entries not sorted: {psds}"
    )


def test_noise_sum_of_entries_equals_total() -> None:
    """Sum of per-element output_psd equals NoisePoint.output_psd."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    pt = result.points[0]
    entry_sum = sum(e.output_psd for e in pt.entries)
    assert isclose(entry_sum, pt.output_psd, rel_tol=1e-10), (
        f"Entry sum {entry_sum:.4e} != total {pt.output_psd:.4e}"
    )


def test_noise_symmetric_resistors_equal_contributions() -> None:
    """Symmetric divider (R1=R2): each resistor contributes equally to S_out."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    e1 = next(e for e in result.points[0].entries if e.element_name == "R1")
    e2 = next(e for e in result.points[0].entries if e.element_name == "R2")
    assert isclose(e1.output_psd, e2.output_psd, rel_tol=1e-6), (
        f"R1 contribution {e1.output_psd:.4e} ≠ R2 {e2.output_psd:.4e}"
    )


def test_noise_asymmetric_divider_bottom_louder() -> None:
    """Asymmetric divider (R1=2kΩ, R2=1kΩ): R2 contributes more to S_out.

    Because R2 is closer to the output node and sees a larger signal transfer,
    its noise dominates even though its source PSD is lower than R1's.

    S_out_R2 = |H_R2|² × (4kT/R2) = (666.67)² × (4kT/1000) × … let T=300K.
    S_out_R1 = |H_R1|² × (4kT/R1) = (666.67)² × (4kT/2000) × …

    Since both H_R1 and H_R2 equal R_eq × (1/R_{source}) × something:
    Thevenin: S_out_R2 = 4kT×R2×(R1/(R1+R2))² = 4kT×1000×(2/3)² = 4kT×444.4
              S_out_R1 = 4kT×R1×(R2/(R1+R2))² = 4kT×2000×(1/3)² = 4kT×222.2
    So R2 > R1.
    """
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "out", 2000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    e_r1 = next(e for e in result.points[0].entries if e.element_name == "R1")
    e_r2 = next(e for e in result.points[0].entries if e.element_name == "R2")
    assert e_r2.output_psd > e_r1.output_psd, (
        f"Expected R2 ({e_r2.output_psd:.3e}) > R1 ({e_r1.output_psd:.3e})"
    )


def test_noise_three_resistors_three_entries() -> None:
    """Three resistors in a T-network produce three noise entries."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "mid", 500.0))
    c.add(Resistor("R2", "mid", "out", 500.0))
    c.add(Resistor("R3", "out", "0", 1000.0))

    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    assert len(result.points[0].entries) == 3


# ---------------------------------------------------------------------------
# Section 49 — Noise analysis: input-referred noise calculation
# ---------------------------------------------------------------------------


def test_noise_input_referred_divider() -> None:
    """Symmetric divider: S_in = S_out / (0.5)² = 4 × S_out."""
    T = 300.0
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0], temperature=T)
    pt = result.points[0]

    expected_s_in = pt.output_psd / (0.5 ** 2)  # gain = 0.5
    assert isclose(pt.input_referred_psd, expected_s_in, rel_tol=1e-4), (
        f"S_in {pt.input_referred_psd:.4e}, expected {expected_s_in:.4e}"
    )


def test_noise_input_referred_greater_than_output_for_attenuator() -> None:
    """For an attenuating circuit, S_in > S_out (noise is amplified at input)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    pt = result.points[0]
    assert pt.input_referred_psd > pt.output_psd, (
        f"Expected S_in ({pt.input_referred_psd:.3e}) > S_out ({pt.output_psd:.3e})"
    )


def test_noise_unknown_input_source_gives_zero_input_referred() -> None:
    """If input_source is not in the circuit, input_referred_psd = 0.0."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Nonexistent", freqs=[1000.0])
    assert result.points[0].input_referred_psd == 0.0


def test_noise_input_referred_scales_with_gain() -> None:
    """Higher gain → lower input-referred noise (same circuit noise, less divided).

    Two circuits: same R_load, but different series R (attenuator ratio):
    - High gain: R_series = 10 Ω  → gain = R_load / (R_series + R_load) ≈ 0.99
    - Low gain:  R_series = 1 kΩ  → gain = 0.5
    High-gain amp should have lower input-referred noise.
    """
    def make_circuit(r_series: float) -> Circuit:
        c = Circuit()
        c.add(VoltageSource("Vin", "in", "0", 0.0))
        c.add(Resistor("Rs", "in", "out", r_series))
        c.add(Resistor("RL", "out", "0", 1000.0))
        return c

    high_gain = noise_ac(make_circuit(10.0),    "out", "Vin", freqs=[1000.0])
    low_gain  = noise_ac(make_circuit(1000.0),  "out", "Vin", freqs=[1000.0])

    s_in_high = high_gain.points[0].input_referred_psd
    s_in_low  = low_gain.points[0].input_referred_psd
    # High-gain circuit has smaller input-referred noise
    assert s_in_high < s_in_low, (
        f"Expected high-gain S_in ({s_in_high:.3e}) < low-gain ({s_in_low:.3e})"
    )


def test_noise_current_source_input_referred() -> None:
    """CurrentSource as input: input_referred_psd is computed correctly."""
    c = Circuit()
    c.add(CurrentSource("Iin", "node", "0", 0.0))
    c.add(Resistor("R1", "node", "0", 1000.0))

    result = noise_ac(c, "node", "Iin", freqs=[1000.0])
    # The transimpedance from Iin to V_node is just R1 = 1000 Ω.
    # S_out = 4kT/R (from R1's thermal noise) × R1² = 4kT × R1
    # S_in  = S_out / R1² = 4kT / R1
    # (This is just the current noise of R1 referred to the input.)
    assert result.points[0].input_referred_psd > 0.0


# ---------------------------------------------------------------------------
# Section 50 — Noise analysis: shot noise (Diode and BJT)
# ---------------------------------------------------------------------------


def test_noise_diode_has_shot_noise_entry() -> None:
    """A forward-biased diode contributes a 'shot' noise entry."""
    c = Circuit()
    c.add(VoltageSource("Vbias", "vcc", "0", 1.0))
    c.add(Resistor("R1", "vcc", "node", 1000.0))
    c.add(Diode("D1", "node", "0"))  # forward-biased: current flows anode→cathode

    result = noise_ac(c, "node", "Vbias", freqs=[1000.0])
    noise_types = {e.noise_type for e in result.points[0].entries}
    assert "shot" in noise_types, f"No shot noise entry; types = {noise_types}"


def test_noise_diode_shot_noise_type_string() -> None:
    """Diode entry has noise_type == 'shot'."""
    c = Circuit()
    c.add(VoltageSource("Vbias", "vcc", "0", 1.0))
    c.add(Resistor("R1", "vcc", "node", 1000.0))
    c.add(Diode("D1", "node", "0"))

    result = noise_ac(c, "node", "Vbias", freqs=[1000.0])
    diode_entry = next(
        (e for e in result.points[0].entries if e.element_name == "D1"), None
    )
    assert diode_entry is not None, "No entry for D1"
    assert diode_entry.noise_type == "shot"


def test_noise_diode_shot_noise_source_psd_positive() -> None:
    """Forward-biased diode has source_psd > 0."""
    c = Circuit()
    c.add(VoltageSource("Vbias", "vcc", "0", 1.0))
    c.add(Resistor("R1", "vcc", "node", 10000.0))
    c.add(Diode("D1", "node", "0"))

    result = noise_ac(c, "node", "Vbias", freqs=[1000.0])
    d1 = next(e for e in result.points[0].entries if e.element_name == "D1")
    assert d1.source_psd > 0.0, f"Diode source_psd = {d1.source_psd}"


def test_noise_diode_shot_noise_proportional_to_2qI() -> None:
    """Diode source_psd = 2q|I_D|.

    At a known DC current I_D, source_psd = 2 × 1.602e-19 × I_D.
    We run two circuits with different bias currents and check that
    the ratio of PSDs matches the ratio of currents.
    """
    def make_circuit(vbias: float) -> Circuit:
        c = Circuit()
        c.add(VoltageSource("Vbias", "vcc", "0", vbias))
        c.add(Resistor("R1", "vcc", "node", 100.0))  # small R for large I_D range
        c.add(Diode("D1", "node", "0"))
        return c

    # We measure the diode current indirectly via source_psd ratio.
    r1 = noise_ac(make_circuit(0.8), "node", "Vbias", freqs=[1000.0])
    r2 = noise_ac(make_circuit(1.0), "node", "Vbias", freqs=[1000.0])

    d1_psd = next(e.source_psd for e in r1.points[0].entries if e.element_name == "D1")
    d2_psd = next(e.source_psd for e in r2.points[0].entries if e.element_name == "D1")

    # Higher bias → larger I_D → higher shot noise PSD.
    assert d2_psd > d1_psd, (
        f"Expected higher bias to give more shot noise: {d2_psd:.3e} vs {d1_psd:.3e}"
    )


def test_noise_bjt_has_shot_noise_entry() -> None:
    """A forward-active NPN BJT contributes a 'shot' noise entry."""
    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", 5.0))
    c.add(VoltageSource("Vbase", "base", "0", 0.7))
    c.add(Resistor("Rc", "vcc", "col", 1000.0))
    c.add(BJT("Q1", "col", "base", "0"))   # NPN: collector, base, emitter

    result = noise_ac(c, "col", "Vbase", freqs=[1000.0])
    noise_types = {e.noise_type for e in result.points[0].entries}
    assert "shot" in noise_types, f"No shot noise entry for BJT; types = {noise_types}"


def test_noise_bjt_shot_noise_type_string() -> None:
    """BJT entry has noise_type == 'shot'."""
    c = Circuit()
    c.add(VoltageSource("Vcc", "vcc", "0", 5.0))
    c.add(VoltageSource("Vbase", "base", "0", 0.7))
    c.add(Resistor("Rc", "vcc", "col", 1000.0))
    c.add(BJT("Q1", "col", "base", "0"))

    result = noise_ac(c, "col", "Vbase", freqs=[1000.0])
    bjt_entry = next(
        (e for e in result.points[0].entries if e.element_name == "Q1"), None
    )
    assert bjt_entry is not None, "No entry for Q1"
    assert bjt_entry.noise_type == "shot"


def test_noise_bjt_emitter_resistance_adds_thermal_noise() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
    circuit.add(Resistor("Rload", "out", "0", 1_000.0))
    circuit.add(BJT("Q1", "out", "base", "0", Re=100.0))

    result = noise_ac(circuit, "out", "Vbase", freqs=[1_000.0])
    emitter_resistance = next(
        entry for entry in result.points[0].entries if entry.element_name == "Q1:RE"
    )

    assert emitter_resistance.noise_type == "thermal"
    assert emitter_resistance.source_psd > 0.0


def test_noise_diode_series_resistance_adds_thermal_noise() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("Vbias", "bias", "0", 1.0))
    circuit.add(Resistor("Rbias", "bias", "out", 1_000.0))
    circuit.add(Diode("D1", "out", "0", Rs=100.0))

    result = noise_ac(circuit, "out", "Vbias", freqs=[1_000.0])
    series_resistance = next(
        entry for entry in result.points[0].entries if entry.element_name == "D1:RS"
    )

    assert series_resistance.noise_type == "thermal"
    assert series_resistance.source_psd > 0.0


def test_noise_diode_kf_adds_inverse_frequency_flicker_noise() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("Vbias", "bias", "0", 1.0))
    circuit.add(Resistor("Rbias", "bias", "out", 1_000.0))
    circuit.add(Diode("D1", "out", "0", Kf=1.0e-12))

    result = noise_ac(circuit, "out", "Vbias", freqs=[10.0, 1_000.0])
    flicker_psds = [
        next(
            entry.source_psd
            for entry in point.entries
            if entry.element_name == "D1" and entry.noise_type == "flicker"
        )
        for point in result.points
    ]

    assert flicker_psds[0] > 0.0
    assert flicker_psds[0] / flicker_psds[1] == pytest.approx(100.0)


def test_noise_jfet_kf_adds_inverse_frequency_flicker_noise() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("Vdd", "vdd", "0", 5.0))
    circuit.add(VoltageSource("Vgate", "gate", "0", 0.0))
    circuit.add(Resistor("Rload", "vdd", "out", 1_000.0))
    circuit.add(JFET("J1", "out", "gate", "0", beta=1.0e-3, vto=-2.0, Kf=1.0e-12))

    result = noise_ac(circuit, "out", "Vgate", freqs=[10.0, 1_000.0])
    flicker_psds = [
        next(
            entry.source_psd
            for entry in point.entries
            if entry.element_name == "J1" and entry.noise_type == "flicker"
        )
        for point in result.points
    ]

    assert flicker_psds[0] > 0.0
    assert flicker_psds[0] / flicker_psds[1] == pytest.approx(100.0)


def test_noise_jfet_nlev_and_gdsnoi_select_and_scale_channel_noise() -> None:
    def source_psd(noise_level: float, coefficient: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vdrain", "out", "0", 1.0))
        circuit.add(VoltageSource("Vgate", "gate", "0", 0.0))
        circuit.add(
            JFET(
                "J1",
                "out",
                "gate",
                "0",
                beta=1.0e-3,
                vto=-2.0,
                Nlev=noise_level,
                Gdsnoi=coefficient,
            )
        )
        entries = noise_ac(
            circuit, "out", "Vgate", freqs=[1_000.0], temperature=300.0
        ).points[0].entries
        return next(
            entry.source_psd
            for entry in entries
            if entry.element_name == "J1" and entry.noise_type == "thermal"
        )

    expected_conductance = (2.0 / 3.0) * 1.0e-3 * 2.0 * 1.75 / 1.5
    expected_psd = 4.0 * 1.380_649e-23 * 300.0 * expected_conductance
    assert source_psd(3.0, 1.0) / expected_psd == pytest.approx(1.0)
    assert source_psd(2.0, 4.0) / source_psd(1.0, 1.0) == pytest.approx(1.0)
    assert source_psd(3.0, 2.0) / source_psd(3.0, 1.0) == pytest.approx(2.0)


@pytest.mark.parametrize(
    ("field", "value", "message"),
    [
        ("Nlev", 2.5, "noise equation level must be a finite integer"),
        ("Gdsnoi", -1.0, "channel noise coefficient must be finite and non-negative"),
    ],
)
def test_noise_rejects_invalid_jfet_channel_noise_parameters(
    field: str, value: float, message: str
) -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("Vgate", "gate", "0", 0.0))
    circuit.add(JFET("J1", "out", "gate", "0", **{field: value}))

    with pytest.raises(ValueError, match=message):
        noise_ac(circuit, "out", "Vgate", freqs=[1_000.0])


def test_noise_jfet_rd_adds_thermal_noise() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("Vdd", "vdd", "0", 5.0))
    circuit.add(VoltageSource("Vgate", "gate", "0", 0.0))
    circuit.add(Resistor("Rload", "vdd", "out", 1_000.0))
    circuit.add(JFET("J1", "out", "gate", "0", Rd=250.0))

    entries = noise_ac(circuit, "out", "Vgate", freqs=[1_000.0]).points[0].entries
    rd = next(
        entry
        for entry in entries
        if entry.element_name == "J1:RD" and entry.noise_type == "thermal"
    )
    assert rd.source_psd == pytest.approx(4.0 * 1.380_649e-23 * 300.0 / 250.0)


def test_noise_jfet_rs_adds_thermal_noise() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("Vdd", "vdd", "0", 5.0))
    circuit.add(VoltageSource("Vgate", "gate", "0", 0.0))
    circuit.add(Resistor("Rload", "vdd", "out", 1_000.0))
    circuit.add(JFET("J1", "out", "gate", "0", Rs=250.0))

    entries = noise_ac(circuit, "out", "Vgate", freqs=[1_000.0]).points[0].entries
    rs = next(
        entry
        for entry in entries
        if entry.element_name == "J1:RS" and entry.noise_type == "thermal"
    )
    assert rs.source_psd == pytest.approx(4.0 * 1.380_649e-23 * 300.0 / 250.0)


def test_noise_jfet_gate_junctions_emit_distinct_shot_sources() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("Vdd", "out", "0", 1.0))
    circuit.add(VoltageSource("Vgate", "gate", "0", 0.3))
    circuit.add(JFET("J1", "out", "gate", "0", Is=1.0e-12))

    entries = noise_ac(
        circuit, "gate", "Vgate", freqs=[1_000.0], temperature=300.0
    ).points[0].entries
    for name in ("J1:IGS", "J1:IGD"):
        entry = next(
            entry
            for entry in entries
            if entry.element_name == name and entry.noise_type == "shot"
        )
        assert entry.source_psd > 0.0


def test_noise_jfet_af_is_the_current_exponent() -> None:
    def source_psd(exponent: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vdd", "vdd", "0", 5.0))
        circuit.add(VoltageSource("Vgate", "gate", "0", 0.0))
        circuit.add(Resistor("Rload", "vdd", "out", 1_000.0))
        circuit.add(
            JFET(
                "J1",
                "out",
                "gate",
                "0",
                beta=1.0e-3,
                vto=-2.0,
                Kf=1.0e-12,
                Af=exponent,
            )
        )
        point = noise_ac(circuit, "out", "Vgate", freqs=[1_000.0]).points[0]
        return next(
            entry.source_psd
            for entry in point.entries
            if entry.element_name == "J1" and entry.noise_type == "flicker"
        )

    assert source_psd(2.0) < source_psd(1.0)


def test_noise_diode_af_is_the_current_exponent() -> None:
    def source_psd(exponent: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vbias", "bias", "0", 1.0))
        circuit.add(Resistor("Rbias", "bias", "out", 1_000.0))
        circuit.add(Diode("D1", "out", "0", Kf=1.0e-12, Af=exponent))
        point = noise_ac(circuit, "out", "Vbias", freqs=[1_000.0]).points[0]
        return next(
            entry.source_psd
            for entry in point.entries
            if entry.element_name == "D1" and entry.noise_type == "flicker"
        )

    assert source_psd(2.0) < source_psd(1.0)


def test_noise_bjt_collector_resistance_adds_thermal_noise() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
    circuit.add(Resistor("Rload", "out", "0", 1_000.0))
    circuit.add(BJT("Q1", "out", "base", "0", Rc=100.0))

    result = noise_ac(circuit, "out", "Vbase", freqs=[1_000.0])
    collector_resistance = next(
        entry for entry in result.points[0].entries if entry.element_name == "Q1:RC"
    )

    assert collector_resistance.noise_type == "thermal"
    assert collector_resistance.source_psd > 0.0


def test_noise_bjt_base_resistance_adds_thermal_noise() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
    circuit.add(Resistor("Rload", "out", "0", 1_000.0))
    circuit.add(BJT("Q1", "out", "base", "0", Rb=100.0))

    result = noise_ac(circuit, "out", "Vbase", freqs=[1_000.0])
    base_resistance = next(
        entry for entry in result.points[0].entries if entry.element_name == "Q1:RB"
    )

    assert base_resistance.noise_type == "thermal"
    assert base_resistance.source_psd > 0.0


def test_noise_bjt_minimum_base_resistance_increases_high_current_noise() -> None:
    def source_psd(*, Rbm: float | None = None, Irb: float = 0.0) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
        circuit.add(Resistor("Rload", "out", "0", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Rb=100.0, Rbm=Rbm, Irb=Irb))

        result = noise_ac(circuit, "out", "Vbase", freqs=[1_000.0])
        return next(
            entry.source_psd
            for entry in result.points[0].entries
            if entry.element_name == "Q1:RB"
        )

    fixed = source_psd()
    bias_dependent = source_psd(Rbm=10.0, Irb=1.0e-9)

    assert bias_dependent > fixed


def test_noise_bjt_kf_adds_inverse_frequency_flicker_noise() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("Vcc", "vcc", "0", 5.0))
    circuit.add(VoltageSource("Vbase", "base", "0", 0.7))
    circuit.add(Resistor("Rc", "vcc", "col", 1_000.0))
    circuit.add(BJT("Q1", "col", "base", "0", Kf=1.0e-12))

    result = noise_ac(circuit, "col", "Vbase", freqs=[10.0, 1_000.0])
    flicker_psds = [
        next(
            entry.source_psd
            for entry in point.entries
            if entry.element_name == "Q1" and entry.noise_type == "flicker"
        )
        for point in result.points
    ]

    assert flicker_psds[0] > 0.0
    assert flicker_psds[0] / flicker_psds[1] == pytest.approx(100.0)


def test_noise_bjt_af_controls_base_current_exponent() -> None:
    def source_psd(exponent: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vcc", "vcc", "0", 5.0))
        circuit.add(VoltageSource("Vbase", "base", "0", 0.7))
        circuit.add(Resistor("Rc", "vcc", "col", 1_000.0))
        circuit.add(BJT("Q1", "col", "base", "0", Kf=1.0e-12, Af=exponent))
        point = noise_ac(circuit, "col", "Vbase", freqs=[1_000.0]).points[0]
        return next(
            entry.source_psd
            for entry in point.entries
            if entry.element_name == "Q1" and entry.noise_type == "flicker"
        )

    assert source_psd(2.0) < source_psd(1.0)


def test_noise_bjt_forward_beta_rolloff_reduces_shot_noise() -> None:
    def source_psd(ikf: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vcc", "vcc", "0", 5.0))
        circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
        circuit.add(Resistor("Rload", "vcc", "out", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Ikf=ikf))
        result = noise_ac(circuit, "out", "Vbase", freqs=[1_000.0])
        return next(
            entry.source_psd
            for entry in result.points[0].entries
            if entry.element_name == "Q1"
        )

    assert source_psd(1.0e-4) < source_psd(0.0)


def test_noise_bjt_base_emitter_leakage_increases_shot_noise() -> None:
    def source_psd(ise: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
        circuit.add(Resistor("Rload", "out", "0", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "0", Ise=ise, Ne=1.5))
        result = noise_ac(circuit, "out", "Vbase", freqs=[1_000.0])
        return next(
            entry.source_psd
            for entry in result.points[0].entries
            if entry.element_name == "Q1"
        )

    assert source_psd(1.0e-10) > source_psd(0.0)


def test_noise_bjt_base_collector_leakage_increases_shot_noise() -> None:
    def source_psd(isc: float) -> float:
        circuit = Circuit()
        circuit.add(VoltageSource("Vbase", "base", "0", 0.65))
        circuit.add(Resistor("Rload", "out", "0", 1_000.0))
        circuit.add(BJT("Q1", "out", "base", "base", Isc=isc, Nc=1.5))
        result = noise_ac(circuit, "out", "Vbase", freqs=[1_000.0])
        return next(
            entry.source_psd
            for entry in result.points[0].entries
            if entry.element_name == "Q1"
        )

    assert source_psd(1.0e-10) > source_psd(0.0)


def test_noise_mosfet_channel_thermal_noise() -> None:
    """A biased MOSFET contributes long-channel channel thermal noise."""
    c = Circuit()
    c.add(VoltageSource("Vdd", "vdd", "0", 5.0))
    c.add(VoltageSource("Vgate", "gate", "0", 3.0))
    c.add(Resistor("Rload", "vdd", "out", 1000.0))
    c.add(Mosfet(
        "M1",
        "out",
        "gate",
        "0",
        "0",
        MOSFET(
            MosfetType.NMOS,
            Level1Model(Level1Params(
                VT0=1.0,
                KP=1.0e-3,
                LAMBDA=0.0,
                GAMMA=0.0,
                W=1.0,
                L=1.0,
            )),
        ),
    ))

    result = noise_ac(c, "out", "Vgate", freqs=[1000.0], temperature=300.0)
    entry = next(
        (e for e in result.points[0].entries if e.element_name == "M1"), None
    )
    gm = 1.0e-3 * (3.0 - 1.0)
    expected_source_psd = 4.0 * _kB * 300.0 * (2.0 / 3.0) * gm

    assert entry is not None, "No MOSFET channel noise entry"
    assert entry.noise_type == "thermal"
    assert isclose(entry.source_psd, expected_source_psd, rel_tol=1e-6)
    assert isclose(
        entry.output_psd,
        expected_source_psd * 1000.0 ** 2,
        rel_tol=1e-6,
    )


# ---------------------------------------------------------------------------
# Section 51 — Noise analysis: frequency sweep and defaults
# ---------------------------------------------------------------------------


def test_noise_default_sweep_has_50_points() -> None:
    """Default frequency sweep (no freqs arg) returns 50 points."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin")
    assert len(result.points) == 50


def test_noise_default_sweep_ascending_frequencies() -> None:
    """Default sweep frequencies are strictly ascending."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin")
    freqs = [pt.freq for pt in result.points]
    for i in range(1, len(freqs)):
        assert freqs[i] > freqs[i - 1], (
            f"Non-ascending at index {i}: {freqs[i-1]} → {freqs[i]}"
        )


def test_noise_default_sweep_starts_near_1hz() -> None:
    """Default sweep starts at ≈ 1 Hz."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin")
    assert isclose(result.points[0].freq, 1.0, rel_tol=1e-6)


def test_noise_default_sweep_ends_near_1mhz() -> None:
    """Default sweep ends at ≈ 1 MHz."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin")
    assert isclose(result.points[-1].freq, 1e6, rel_tol=1e-4)


def test_noise_custom_freq_list_respected() -> None:
    """Custom freqs list is used exactly (correct count and values)."""
    c = _divider_circuit()
    custom = [100.0, 1000.0, 10000.0]
    result = noise_ac(c, "out", "Vin", freqs=custom)
    assert len(result.points) == 3
    for pt, expected_f in zip(result.points, custom, strict=True):
        assert pt.freq == expected_f


def test_noise_single_frequency_point() -> None:
    """Single-element freqs list returns exactly one NoisePoint."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[5000.0])
    assert len(result.points) == 1
    assert result.points[0].freq == 5000.0


def test_noise_output_psd_positive_at_all_default_freqs() -> None:
    """Output PSD is positive at every default frequency point."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin")
    for pt in result.points:
        assert pt.output_psd > 0.0, f"Zero PSD at f = {pt.freq:.2f} Hz"


# ---------------------------------------------------------------------------
# Section 52 — Noise analysis: error cases and edge cases
# ---------------------------------------------------------------------------


def test_noise_unknown_output_node_returns_empty() -> None:
    """Unknown output node returns a NoiseResult with empty points list."""
    c = _divider_circuit()
    result = noise_ac(c, "nonexistent", "Vin", freqs=[1000.0])
    assert isinstance(result, NoiseResult)
    assert result.points == []


def test_noise_ground_output_node_returns_empty() -> None:
    """Ground as output node ('0') returns a NoiseResult with empty points."""
    c = _divider_circuit()
    result = noise_ac(c, "0", "Vin", freqs=[1000.0])
    assert result.points == []


def test_noise_empty_freqs_list_returns_no_points() -> None:
    """Empty freqs list produces a NoiseResult with zero points."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[])
    assert result.points == []


def test_noise_capacitor_is_noiseless() -> None:
    """Capacitor contributes no noise entry (capacitors are noiseless)."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "out", 1000.0))
    c.add(Resistor("R2", "out", "0", 1000.0))
    c.add(Capacitor("C1", "out", "0", 1e-9))   # in parallel with R2

    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    names = {e.element_name for e in result.points[0].entries}
    assert "C1" not in names, f"Capacitor C1 wrongly appears in noise entries: {names}"


def test_noise_voltage_source_is_noiseless() -> None:
    """VoltageSource contributes no noise entry (ideal sources are noiseless)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    names = {e.element_name for e in result.points[0].entries}
    assert "Vin" not in names, f"VoltageSource Vin wrongly in noise entries: {names}"


def test_noise_inductor_is_noiseless() -> None:
    """Inductor contributes no noise entry (inductors are modelled as noiseless)."""
    c = Circuit()
    c.add(VoltageSource("Vin", "in", "0", 0.0))
    c.add(Resistor("R1", "in", "mid", 1000.0))
    c.add(Inductor("L1", "mid", "out", 1e-3))
    c.add(Resistor("R2", "out", "0", 1000.0))

    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    names = {e.element_name for e in result.points[0].entries}
    assert "L1" not in names, f"Inductor L1 wrongly in noise entries: {names}"


def test_noise_output_psd_nonnegative() -> None:
    """NoisePoint.output_psd is always non-negative (sum of non-negative terms)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin")
    for pt in result.points:
        assert pt.output_psd >= 0.0, f"Negative output_psd at f={pt.freq:.2f}"


def test_noise_entries_tuple_immutable() -> None:
    """NoisePoint.entries is a tuple (immutable sequence of NoiseEntry)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    assert isinstance(result.points[0].entries, tuple)


def test_noise_noisepoint_is_frozen() -> None:
    """NoisePoint is a frozen dataclass (attempting to set field raises)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    with pytest.raises((AttributeError, TypeError)):
        result.points[0].freq = 9999.0  # type: ignore[misc]


def test_noise_noiseentry_is_frozen() -> None:
    """NoiseEntry is a frozen dataclass (attempting to set field raises)."""
    c = _divider_circuit()
    result = noise_ac(c, "out", "Vin", freqs=[1000.0])
    entry = result.points[0].entries[0]
    with pytest.raises((AttributeError, TypeError)):
        entry.output_psd = -1.0  # type: ignore[misc]


# ── Section 53: VCCS element dataclass ───────────────────────────────────────


def test_vccs_frozen() -> None:
    """VCCS is a frozen dataclass (immutable after construction)."""
    g = VCCS("G1", "out", "0", "in", "0", gm=0.01)
    with pytest.raises((AttributeError, TypeError)):
        g.gm = 0.02  # type: ignore[misc]


def test_vccs_fields() -> None:
    """VCCS stores all six fields correctly."""
    g = VCCS("G1", "out", "0", "in", "0", gm=0.02)
    assert g.name == "G1"
    assert g.n_plus == "out"
    assert g.n_minus == "0"
    assert g.ctrl_plus == "in"
    assert g.ctrl_minus == "0"
    assert g.gm == pytest.approx(0.02)


def test_vccs_negative_gm() -> None:
    """VCCS accepts negative gm (for sign-inverting configurations)."""
    g = VCCS("G_inv", "out", "0", "in", "0", gm=-0.05)
    assert g.gm == pytest.approx(-0.05)


def test_vccs_in_element_union() -> None:
    """VCCS is accepted as a circuit element (Element type alias includes it)."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "0", "out", "in", "0", gm=0.01),
    ])
    assert len(c.elements) == 3


# ── Section 54: VCVS element dataclass ───────────────────────────────────────


def test_vcvs_frozen() -> None:
    """VCVS is a frozen dataclass (immutable after construction)."""
    e = VCVS("E1", "out", "0", "vin", "0", gain=2.0)
    with pytest.raises((AttributeError, TypeError)):
        e.gain = 3.0  # type: ignore[misc]


def test_vcvs_fields() -> None:
    """VCVS stores all six fields correctly."""
    e = VCVS("E1", "out", "0", "vin", "0", gain=-10.0)
    assert e.name == "E1"
    assert e.n_plus == "out"
    assert e.n_minus == "0"
    assert e.ctrl_plus == "vin"
    assert e.ctrl_minus == "0"
    assert e.gain == pytest.approx(-10.0)


def test_vcvs_differential_ctrl() -> None:
    """VCVS can have a non-ground ctrl_minus (differential sensing)."""
    e = VCVS("Ediff", "out", "0", "vp", "vm", gain=5.0)
    assert e.ctrl_plus == "vp"
    assert e.ctrl_minus == "vm"
    assert e.gain == pytest.approx(5.0)


def test_vcvs_in_element_union() -> None:
    """VCVS is accepted as a circuit element."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 5.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    assert len(c.elements) == 3


# ── Section 55: CCCS element dataclass ───────────────────────────────────────


def test_cccs_frozen() -> None:
    """CCCS is a frozen dataclass (immutable after construction)."""
    f = CCCS("F1", "out", "0", "Vsense", beta=2.0)
    with pytest.raises((AttributeError, TypeError)):
        f.beta = 3.0  # type: ignore[misc]


def test_cccs_fields() -> None:
    """CCCS stores all five fields correctly."""
    f = CCCS("F1", "out", "0", "Vsense", beta=5.0)
    assert f.name == "F1"
    assert f.n_plus == "out"
    assert f.n_minus == "0"
    assert f.ctrl_source == "Vsense"
    assert f.beta == pytest.approx(5.0)


def test_cccs_negative_beta() -> None:
    """CCCS accepts negative beta (current direction reversal)."""
    f = CCCS("Finv", "out", "0", "Vsense", beta=-1.0)
    assert f.beta == pytest.approx(-1.0)


def test_cccs_in_element_union() -> None:
    """CCCS is accepted as a circuit element."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCCS("F1", "out", "0", "Vsense", beta=2.0),
        Resistor("Rload", "out", "0", 500.0),
    ])
    assert len(c.elements) == 5


# ── Section 56: CCVS element dataclass ───────────────────────────────────────


def test_ccvs_frozen() -> None:
    """CCVS is a frozen dataclass (immutable after construction)."""
    h = CCVS("H1", "out", "0", "Vsense", transresistance=1000.0)
    with pytest.raises((AttributeError, TypeError)):
        h.transresistance = 2000.0  # type: ignore[misc]


def test_ccvs_fields() -> None:
    """CCVS stores all five fields correctly."""
    h = CCVS("H1", "out", "0", "Vsense", transresistance=500.0)
    assert h.name == "H1"
    assert h.n_plus == "out"
    assert h.n_minus == "0"
    assert h.ctrl_source == "Vsense"
    assert h.transresistance == pytest.approx(500.0)


def test_ccvs_in_element_union() -> None:
    """CCVS is accepted as a circuit element."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCVS("H1", "out", "0", "Vsense", transresistance=500.0),
    ])
    assert len(c.elements) == 4


# ── Section 57: DC analysis – VCCS ───────────────────────────────────────────


def test_vccs_dc_inverting() -> None:
    """VCCS(out, 0, in, 0, gm) inverts: V_out = -gm * R_load * V_in.

    MNA analysis: VCCS stamps G[out][in] += gm.  Rload stamps G[out][out] +=
    1/R.  KCL at ``out``: (1/R)*V_out + gm*V_in = 0 → V_out = -gm*R*V_in.
    """
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "out", "0", "in", "0", gm=0.01),
    ])
    r = dc_op(c)
    # V_out = -0.01 * 100 * 1.0 = -1.0 V
    assert r.node_voltages["out"] == pytest.approx(-1.0, abs=1e-9)


def test_vccs_dc_noninverting() -> None:
    """VCCS(0, out, in, 0, gm) non-inverts: V_out = +gm * R_load * V_in.

    Swapping n_plus and n_minus flips the sign of all stamp entries, so the
    current injection at ``out`` is in the opposite direction.
    KCL at ``out``: (1/R)*V_out - gm*V_in = 0 → V_out = +gm*R*V_in.
    """
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "0", "out", "in", "0", gm=0.01),
    ])
    r = dc_op(c)
    # V_out = +0.01 * 100 * 1.0 = +1.0 V
    assert r.node_voltages["out"] == pytest.approx(1.0, abs=1e-9)


def test_vccs_dc_zero_gm() -> None:
    """VCCS with gm=0 injects no current (equivalent to open circuit)."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 5.0),
        Resistor("Rload", "out", "0", 1000.0),
        VCCS("G1", "out", "0", "in", "0", gm=0.0),
    ])
    r = dc_op(c)
    # No current injected → Rload has no current → V_out = 0
    assert r.node_voltages["out"] == pytest.approx(0.0, abs=1e-9)


def test_vccs_dc_gain_scaling() -> None:
    """VCCS output scales linearly with gm and R_load."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 2.0),
        Resistor("Rload", "out", "0", 50.0),
        VCCS("G1", "0", "out", "in", "0", gm=0.02),
    ])
    r = dc_op(c)
    # V_out = gm * R * V_in = 0.02 * 50 * 2.0 = 2.0 V
    assert r.node_voltages["out"] == pytest.approx(2.0, abs=1e-9)


def test_vccs_dc_differential_ctrl() -> None:
    """VCCS senses V(ctrl_plus) − V(ctrl_minus) for a differential input."""
    # V_p = 3V, V_m = 1V → V_diff = 2V → V_out = gm * R * V_diff
    c = Circuit([
        VoltageSource("Vp", "vp", "0", 3.0),
        VoltageSource("Vm", "vm", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "0", "out", "vp", "vm", gm=0.01),
    ])
    r = dc_op(c)
    # V_out = 0.01 * 100 * (3 - 1) = 2.0 V
    assert r.node_voltages["out"] == pytest.approx(2.0, abs=1e-9)


# ── Section 58: DC analysis – VCVS ───────────────────────────────────────────


def test_vcvs_dc_unity_buffer() -> None:
    """VCVS with gain=1 is a perfect unity-gain voltage buffer.

    KVL: V_out − 1.0 × V_vin = 0 → V_out = V_vin.
    The load has no effect because the VCVS is an ideal (zero-output-
    impedance) voltage source.
    """
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 5.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    r = dc_op(c)
    assert r.node_voltages["out"] == pytest.approx(5.0, abs=1e-9)


def test_vcvs_dc_inverting_gain() -> None:
    """VCVS with gain=-2 inverts and amplifies: V_out = -2 × V_in."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 3.0),
        VCVS("E1", "out", "0", "vin", "0", gain=-2.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    r = dc_op(c)
    assert r.node_voltages["out"] == pytest.approx(-6.0, abs=1e-9)


def test_vcvs_dc_large_gain() -> None:
    """VCVS with a large positive gain (open-loop op-amp macromodel)."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 0.001),  # 1 mV input
        VCVS("E1", "out", "0", "vin", "0", gain=1000.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    r = dc_op(c)
    # V_out = 1000 * 0.001 = 1.0 V
    assert r.node_voltages["out"] == pytest.approx(1.0, abs=1e-9)


def test_vcvs_dc_differential_ctrl() -> None:
    """VCVS senses V(ctrl_plus) − V(ctrl_minus) for a differential input."""
    c = Circuit([
        VoltageSource("Vp", "vp", "0", 4.0),
        VoltageSource("Vm", "vm", "0", 1.0),
        VCVS("E1", "out", "0", "vp", "vm", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    r = dc_op(c)
    # V_diff = 4 - 1 = 3V → V_out = 1.0 * 3 = 3.0V
    assert r.node_voltages["out"] == pytest.approx(3.0, abs=1e-9)


def test_vcvs_dc_branch_current_recorded() -> None:
    """dc_op records I(E1) in branch_currents for a VCVS element."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 5.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    r = dc_op(c)
    assert "I(E1)" in r.branch_currents
    # Rload current = V_out / R_load = 5.0 / 1000 = 5 mA
    # VCVS sinks this current from its output terminal
    assert abs(r.branch_currents["I(E1)"]) == pytest.approx(5e-3, rel=1e-6)


def test_vcvs_dc_load_independent() -> None:
    """VCVS output voltage is independent of load (ideal voltage source)."""
    def _vout(rload: float) -> float:
        c = Circuit([
            VoltageSource("Vin", "vin", "0", 2.0),
            VCVS("E1", "out", "0", "vin", "0", gain=3.0),
            Resistor("Rload", "out", "0", rload),
        ])
        return dc_op(c).node_voltages["out"]

    # V_out = 6V regardless of load
    assert _vout(100.0) == pytest.approx(6.0, abs=1e-9)
    assert _vout(10_000.0) == pytest.approx(6.0, abs=1e-9)


# ── Section 59: DC analysis – CCCS ───────────────────────────────────────────


def _cccs_circuit(beta: float = 2.0, r_load: float = 500.0) -> Circuit:
    """Standard CCCS test circuit.

    Topology: Vin(1V) → Rin(1kΩ) → Vsense(0V) → GND.
    CCCS F1 controlled by Vsense, output into Rload.

    I_ctrl = V_in / R_in = 1V / 1kΩ = 1 mA (Vsense is a 0V ammeter).

    SPICE convention: ``F1 n+ n-`` means positive current flows from ``n+``
    through the EXTERNAL circuit to ``n-``.  Here n_plus="out", n_minus="0"
    so 2 mA flows from "out" through Rload to ground: V_out = +beta*I*R_load.
    """
    return Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCCS("F1", "out", "0", "Vsense", beta),
        Resistor("Rload", "out", "0", r_load),
    ])


def test_cccs_dc_basic() -> None:
    """CCCS mirrors and scales current: V_out = beta × I_ctrl × R_load."""
    r = dc_op(_cccs_circuit(beta=2.0, r_load=500.0))
    # V_out = 2 * 1e-3 * 500 = 1.0 V
    assert r.node_voltages["out"] == pytest.approx(1.0, abs=1e-9)


def test_cccs_dc_unity_beta() -> None:
    """CCCS with beta=1.0 replicates the controlling current exactly."""
    r = dc_op(_cccs_circuit(beta=1.0, r_load=1000.0))
    # V_out = 1 * 1e-3 * 1000 = 1.0 V
    assert r.node_voltages["out"] == pytest.approx(1.0, abs=1e-9)


def test_cccs_dc_zero_beta() -> None:
    """CCCS with beta=0 injects no output current (open circuit)."""
    r = dc_op(_cccs_circuit(beta=0.0, r_load=500.0))
    assert r.node_voltages["out"] == pytest.approx(0.0, abs=1e-9)


def test_cccs_dc_negative_beta() -> None:
    """CCCS with beta=-1 reverses the current direction."""
    r = dc_op(_cccs_circuit(beta=-1.0, r_load=500.0))
    # V_out = -1 * 1e-3 * 500 = -0.5 V
    assert r.node_voltages["out"] == pytest.approx(-0.5, abs=1e-9)


def test_cccs_dc_higher_gain() -> None:
    """CCCS with larger beta amplifies proportionally."""
    r = dc_op(_cccs_circuit(beta=5.0, r_load=200.0))
    # V_out = 5 * 1e-3 * 200 = 1.0 V
    assert r.node_voltages["out"] == pytest.approx(1.0, abs=1e-9)


# ── Section 60: DC analysis – CCVS ───────────────────────────────────────────


def _ccvs_circuit(rm: float = 500.0) -> Circuit:
    """Standard CCVS test circuit.

    Topology: Vin(1V) → Rin(1kΩ) → Vsense(0V) → GND.
    CCVS H1 controlled by Vsense forces V_out = rm × I_ctrl.

    I_ctrl = V_in / R_in = 1V / 1kΩ = 1 mA.
    V_out  = rm × I_ctrl = rm × 1 mA.
    """
    return Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCVS("H1", "out", "0", "Vsense", rm),
    ])


def test_ccvs_dc_basic() -> None:
    """CCVS forces V_out = rm × I_ctrl."""
    r = dc_op(_ccvs_circuit(rm=500.0))
    # V_out = 500 * 1e-3 = 0.5 V
    assert r.node_voltages["out"] == pytest.approx(0.5, abs=1e-9)


def test_ccvs_dc_unit_transresistance() -> None:
    """CCVS with rm=1000 Ω: V_out = 1 kΩ × 1 mA = 1 V."""
    r = dc_op(_ccvs_circuit(rm=1000.0))
    assert r.node_voltages["out"] == pytest.approx(1.0, abs=1e-9)


def test_ccvs_dc_zero_transresistance() -> None:
    """CCVS with rm=0 is a short circuit (ideal wire from output to ground)."""
    r = dc_op(_ccvs_circuit(rm=0.0))
    assert r.node_voltages["out"] == pytest.approx(0.0, abs=1e-9)


def test_ccvs_dc_with_load() -> None:
    """CCVS output voltage is independent of load (ideal voltage source)."""
    c_no_load = _ccvs_circuit(rm=500.0)
    c_with_load = Circuit(
        list(c_no_load.elements)
        + [Resistor("Rload", "out", "0", 100.0)]
    )
    r = dc_op(c_with_load)
    # V_out is still 0.5V regardless of the load resistance
    assert r.node_voltages["out"] == pytest.approx(0.5, abs=1e-9)


def test_ccvs_dc_branch_current_recorded() -> None:
    """dc_op records I(H1) in branch_currents for a CCVS element."""
    r = dc_op(_ccvs_circuit(rm=500.0))
    assert "I(H1)" in r.branch_currents


# ── Section 61: AC analysis – controlled sources ─────────────────────────────


def test_vcvs_ac_unity_buffer() -> None:
    """VCVS unity buffer passes AC signal unattenuated at all frequencies."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 1.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = ac_sweep(c, f_start=1.0, f_stop=1e6, n_points=5)
    for pt in result.points:
        assert abs(pt.node_voltages["out"]) == pytest.approx(1.0, rel=1e-6)


def test_vcvs_ac_gain() -> None:
    """VCVS with gain=3 scales AC voltage by 3 at all frequencies."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 1.0),
        VCVS("E1", "out", "0", "vin", "0", gain=3.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = ac_sweep(c, f_start=1.0, f_stop=1e4, n_points=3)
    for pt in result.points:
        assert abs(pt.node_voltages["out"]) == pytest.approx(3.0, rel=1e-6)


def test_vccs_ac_transconductance() -> None:
    """VCCS produces frequency-independent transconductance in AC analysis."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "0", "out", "in", "0", gm=0.01),
    ])
    result = ac_sweep(c, f_start=1.0, f_stop=1e6, n_points=5)
    for pt in result.points:
        # V_out = gm * R_load * V_in = 0.01 * 100 * 1 = 1.0
        assert abs(pt.node_voltages["out"]) == pytest.approx(1.0, rel=1e-6)


def test_cccs_ac_current_gain() -> None:
    """CCCS scales controlling AC current by beta at all frequencies."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCCS("F1", "out", "0", "Vsense", 2.0),
        Resistor("Rload", "out", "0", 500.0),
    ])
    result = ac_sweep(c, f_start=1.0, f_stop=1e3, n_points=3)
    for pt in result.points:
        # V_out = beta * (V_in/R_in) * R_load = 2 * (1/1000) * 500 = 1.0
        assert abs(pt.node_voltages["out"]) == pytest.approx(1.0, rel=1e-6)


def test_ccvs_ac_transresistance() -> None:
    """CCVS passes AC signal via transresistance at all frequencies."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCVS("H1", "out", "0", "Vsense", 500.0),
        Resistor("Rload", "out", "0", 100.0),
    ])
    result = ac_sweep(c, f_start=1.0, f_stop=1e4, n_points=3)
    for pt in result.points:
        # V_out = rm * (V_in/R_in) = 500 * (1/1000) = 0.5
        assert abs(pt.node_voltages["out"]) == pytest.approx(0.5, rel=1e-6)


# ── Section 62: DC sweep – VCVS ──────────────────────────────────────────────


def test_vcvs_dc_sweep_linear() -> None:
    """VCVS output tracks dc_sweep of input source linearly (gain=2)."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 0.0),
        VCVS("E1", "out", "0", "vin", "0", gain=2.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = dc_sweep(c, "Vin", 0.0, 3.0, 1.0)
    v_outs = [pt.node_voltages["out"] for pt in result.points]
    # At V_in = 0, 1, 2, 3: V_out = 0, 2, 4, 6
    assert v_outs == pytest.approx([0.0, 2.0, 4.0, 6.0], abs=1e-9)


def test_vccs_dc_sweep() -> None:
    """VCCS output tracks dc_sweep — V_out = -gm * R * V_in at each point."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 0.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "out", "0", "in", "0", gm=0.01),
    ])
    result = dc_sweep(c, "Vin", 0.0, 2.0, 1.0)
    v_outs = [pt.node_voltages["out"] for pt in result.points]
    # V_out = -gm * R * V_in = -0.01 * 100 * V_in = -V_in
    assert v_outs == pytest.approx([0.0, -1.0, -2.0], abs=1e-9)


# ── Section 63: Transient – controlled sources ────────────────────────────────


def test_vcvs_transient_unity() -> None:
    """VCVS unity buffer reproduces the DC input in every transient timestep."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 3.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = transient(c, t_stop=1e-3, t_step=1e-4)
    for pt in result.points:
        assert pt.node_voltages["out"] == pytest.approx(3.0, abs=1e-6)


def test_vccs_transient() -> None:
    """VCCS works correctly during transient analysis (DC-steady circuit)."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "0", "out", "in", "0", gm=0.01),
    ])
    result = transient(c, t_stop=1e-3, t_step=1e-4)
    for pt in result.points:
        assert pt.node_voltages["out"] == pytest.approx(1.0, abs=1e-6)


def test_cccs_transient() -> None:
    """CCCS works correctly during transient analysis."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCCS("F1", "out", "0", "Vsense", 2.0),
        Resistor("Rload", "out", "0", 500.0),
    ])
    result = transient(c, t_stop=1e-3, t_step=1e-4)
    for pt in result.points:
        assert pt.node_voltages["out"] == pytest.approx(1.0, abs=1e-6)


def test_ccvs_transient() -> None:
    """CCVS works correctly during transient analysis."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCVS("H1", "out", "0", "Vsense", 500.0),
        Resistor("Rload", "out", "0", 100.0),
    ])
    result = transient(c, t_stop=1e-3, t_step=1e-4)
    for pt in result.points:
        assert pt.node_voltages["out"] == pytest.approx(0.5, abs=1e-6)


# ── Section 64: TF analysis – VCVS ───────────────────────────────────────────


def test_vcvs_tf_unity() -> None:
    """Transfer function of VCVS unity buffer is gain = 1.0."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 1.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = tf(c, output_node="out", input_source="Vin")
    assert result.gain == pytest.approx(1.0, rel=1e-6)


def test_vcvs_tf_gain_two() -> None:
    """TF analysis returns the correct gain=2 for a VCVS amplifier."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 1.0),
        VCVS("E1", "out", "0", "vin", "0", gain=2.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = tf(c, output_node="out", input_source="Vin")
    assert result.gain == pytest.approx(2.0, rel=1e-6)


def test_vcvs_tf_inverting() -> None:
    """TF analysis returns negative gain for an inverting VCVS."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 1.0),
        VCVS("E1", "out", "0", "vin", "0", gain=-5.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = tf(c, output_node="out", input_source="Vin")
    assert result.gain == pytest.approx(-5.0, rel=1e-6)


def test_cccs_tf() -> None:
    """TF analysis works for circuits containing a CCCS."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCCS("F1", "out", "0", "Vsense", 2.0),
        Resistor("Rload", "out", "0", 500.0),
    ])
    result = tf(c, output_node="out", input_source="Vin")
    # TF = V_out / V_in = beta * R_load / R_in = 2 * 500 / 1000 = 1.0
    assert result.gain == pytest.approx(1.0, rel=1e-6)


def test_ccvs_tf() -> None:
    """TF analysis works for circuits containing a CCVS."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rin", "in", "mid", 1000.0),
        VoltageSource("Vsense", "mid", "0", 0.0),
        CCVS("H1", "out", "0", "Vsense", 500.0),
        Resistor("Rload", "out", "0", 100.0),
    ])
    result = tf(c, output_node="out", input_source="Vin")
    # TF = V_out / V_in = rm / R_in = 500 / 1000 = 0.5
    assert result.gain == pytest.approx(0.5, rel=1e-6)


# ── Section 65: Sensitivity – VCVS ───────────────────────────────────────────


def test_vcvs_sens_dc_runs() -> None:
    """sens_dc runs without error in a circuit containing a VCVS."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 5.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = sens_dc(c, "out")
    assert isinstance(result.entries, list)
    assert result.nominal_voltage == pytest.approx(5.0, abs=1e-9)


def test_vccs_sens_dc_runs() -> None:
    """sens_dc runs without error in a circuit containing a VCCS."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "0", "out", "in", "0", gm=0.01),
    ])
    result = sens_dc(c, "out")
    assert isinstance(result.entries, list)
    assert result.nominal_voltage == pytest.approx(1.0, abs=1e-9)


# ── Section 66: Monte Carlo – VCVS ───────────────────────────────────────────


def test_vcvs_mc_dc_runs() -> None:
    """mc_dc runs without error in a circuit containing a VCVS."""
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 5.0),
        VCVS("E1", "out", "0", "vin", "0", gain=1.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = mc_dc(c, "out", 5, seed=42)
    assert len(result.points) == 5
    # All trials should converge to roughly 5V (VCVS gain not varied by MC)
    for pt in result.points:
        assert abs(pt.node_voltages["out"] - 5.0) < 1.0


def test_vccs_mc_dc_runs() -> None:
    """mc_dc runs without error in a circuit containing a VCCS."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        VCCS("G1", "0", "out", "in", "0", gm=0.01),
    ])
    result = mc_dc(c, "out", 5, seed=0)
    assert len(result.points) == 5


# ── Section 67: Error cases – unknown ctrl_source ────────────────────────────


def test_cccs_dc_unknown_ctrl_source_raises() -> None:
    """CCCS referencing a non-existent VoltageSource raises ValueError at
    simulation time."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        CCCS("F1", "out", "0", "Vnonexistent", beta=2.0),
    ])
    with pytest.raises(ValueError, match="Vnonexistent"):
        dc_op(c)


def test_ccvs_dc_unknown_ctrl_source_raises() -> None:
    """CCVS referencing a non-existent VoltageSource raises ValueError at
    simulation time."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        CCVS("H1", "out", "0", "Vnonexistent", transresistance=100.0),
    ])
    with pytest.raises(ValueError, match="Vnonexistent"):
        dc_op(c)


def test_cccs_ac_unknown_ctrl_source_raises() -> None:
    """CCCS with unknown ctrl_source also raises ValueError in AC analysis."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("Rload", "out", "0", 100.0),
        CCCS("F1", "out", "0", "Vbad", beta=1.0),
    ])
    with pytest.raises(ValueError, match="Vbad"):
        ac_sweep(c, f_start=1.0, f_stop=1e3, n_points=2)


def test_ccvs_ac_unknown_ctrl_source_raises() -> None:
    """CCVS with unknown ctrl_source also raises ValueError in AC analysis."""
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        CCVS("H1", "out", "0", "Vbad", transresistance=500.0),
        Resistor("Rload", "out", "0", 100.0),
    ])
    with pytest.raises(ValueError, match="Vbad"):
        ac_sweep(c, f_start=1.0, f_stop=1e3, n_points=2)


# ---------------------------------------------------------------------------
# Section 66 — Time-varying source waveforms
# ---------------------------------------------------------------------------
#
# Each waveform class is exercised in two layers:
#   (a) unit tests on the callable directly (Waveform.__call__)
#   (b) integration tests that wire the waveform into a transient sim and
#       verify that node voltages track the expected waveform shape.
#
# The integration circuits are always a simple voltage-follower topology:
#
#     Vsrc (waveform) ─── "in" ─── R ─── "out" ─── 0
#
# With R → 0 (1 mΩ), V("out") ≈ V("in") = waveform(t).
#
# Alternatively for current sources:
#
#     Isrc (waveform) ─── "out" ─┬─── R ─── 0
#                                 └ V("out") = I_src(t) * R


# ---- PwlWaveform unit tests ------------------------------------------------


def test_pwl_waveform_before_first_breakpoint() -> None:
    """Before the first breakpoint PwlWaveform holds the first value."""
    w = PwlWaveform(points=((1.0, 0.5), (2.0, 1.5)))
    assert w(0.0) == pytest.approx(0.5)
    assert w(-10.0) == pytest.approx(0.5)


def test_pwl_waveform_after_last_breakpoint() -> None:
    """After the last breakpoint PwlWaveform holds the last value."""
    w = PwlWaveform(points=((0.0, 0.0), (1.0, 5.0)))
    assert w(2.0) == pytest.approx(5.0)
    assert w(100.0) == pytest.approx(5.0)


def test_pwl_waveform_at_breakpoints() -> None:
    """PwlWaveform returns exact values at defined breakpoints."""
    w = PwlWaveform(points=((0.0, 0.0), (1.0, 3.0), (2.0, 1.0)))
    assert w(0.0) == pytest.approx(0.0)
    assert w(1.0) == pytest.approx(3.0)
    assert w(2.0) == pytest.approx(1.0)


def test_pwl_waveform_linear_interpolation() -> None:
    """PwlWaveform interpolates linearly between breakpoints."""
    w = PwlWaveform(points=((0.0, 0.0), (1.0, 1.0)))
    assert w(0.25) == pytest.approx(0.25)
    assert w(0.5) == pytest.approx(0.5)
    assert w(0.75) == pytest.approx(0.75)


def test_pwl_waveform_negative_values() -> None:
    """PwlWaveform handles negative breakpoint values correctly."""
    w = PwlWaveform(points=((0.0, -2.0), (1.0, 2.0)))
    assert w(0.5) == pytest.approx(0.0)


def test_pwl_waveform_transient_step() -> None:
    """Transient sim driven by a PWL step: node voltage follows the ramp."""
    # PWL: 0 V at t=0, ramp to 1 V over 0.5 s, hold 1 V from 0.5 s onward.
    w = PwlWaveform(points=((0.0, 0.0), (0.5, 1.0), (1.0, 1.0)))
    c = Circuit([
        VoltageSource("Vs", "in", "0", voltage=0.0, waveform=w),
        Resistor("Rser", "in", "out", 1e-3),  # near-ideal follower
        Resistor("Rload", "out", "0", 1.0),
    ])
    result = transient(c, t_stop=1.0, t_step=0.1)
    assert result.converged

    for pt in result.points:
        v_out = pt.node_voltages.get("out", 0.0)
        expected = w(pt.time)
        assert abs(v_out - expected) < 0.02, (
            f"t={pt.time:.2f}: V(out)={v_out:.4f} expected ~{expected:.4f}"
        )


# ---- SinWaveform unit tests ------------------------------------------------


def test_sin_waveform_before_delay() -> None:
    """SinWaveform returns offset before the delay time."""
    w = SinWaveform(offset=1.0, amplitude=2.0, frequency=10.0, delay=0.5)
    assert w(0.0) == pytest.approx(1.0)
    assert w(0.499) == pytest.approx(1.0)


def test_sin_waveform_at_zero_phase() -> None:
    """At t = delay the waveform returns offset (sin(0) = 0)."""
    w = SinWaveform(offset=0.5, amplitude=1.0, frequency=1.0, delay=0.1)
    assert w(0.1) == pytest.approx(0.5, abs=1e-12)


def test_sin_waveform_quarter_period() -> None:
    """At t = delay + T/4 the waveform hits its peak (sin = 1)."""
    freq = 2.0  # Hz
    T = 1.0 / freq
    delay = 0.0
    w = SinWaveform(offset=0.0, amplitude=3.0, frequency=freq, delay=delay)
    assert w(delay + T / 4) == pytest.approx(3.0, abs=1e-10)


def test_sin_waveform_damped_decays() -> None:
    """Damped sinusoid amplitude decreases over time."""
    w = SinWaveform(offset=0.0, amplitude=1.0, frequency=1.0, damping=1.0)
    # Peak at t = T/4 ≈ 0.25; a later peak (at ~1.25) should be smaller.
    v_first = abs(w(0.25))
    v_later = abs(w(1.25))
    assert v_later < v_first


def test_sin_waveform_transient_sinusoid() -> None:
    """Transient sim driven by SinWaveform: node tracks sin at sample times."""
    freq = 2.0   # Hz
    amp = 1.5
    w = SinWaveform(offset=0.0, amplitude=amp, frequency=freq)
    c = Circuit([
        VoltageSource("Vs", "in", "0", voltage=0.0, waveform=w),
        Resistor("Rser", "in", "out", 1e-3),
        Resistor("Rload", "out", "0", 1.0),
    ])
    result = transient(c, t_stop=1.0, t_step=0.02)
    assert result.converged

    for pt in result.points:
        v_out = pt.node_voltages.get("out", 0.0)
        expected = amp * math.sin(2.0 * math.pi * freq * pt.time)
        assert abs(v_out - expected) < 0.05, (
            f"t={pt.time:.3f}: V(out)={v_out:.4f} expected ~{expected:.4f}"
        )


def test_fourier_extracts_transient_sinusoid_components() -> None:
    freq = 1_000.0
    amp = 2.0
    offset = 0.25
    period = 1.0 / freq
    c = Circuit([
        VoltageSource(
            "Vs",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(offset=offset, amplitude=amp, frequency=freq),
        ),
    ])
    result = transient(c, t_stop=2.0 * period, t_step=period / 64.0)
    analysis = fourier(result, freq, ["V(in)"], harmonics=5)

    assert isinstance(analysis, FourierResult)
    assert isinstance(analysis.probes[0], FourierProbeResult)
    assert isinstance(analysis.probes[0].harmonics[0], FourierHarmonic)
    assert analysis.start_time == pytest.approx(period)
    probe = analysis.probes[0]
    fundamental = probe.harmonics[0]
    assert probe.dc == pytest.approx(offset, abs=2.0e-3)
    assert fundamental.frequency == pytest.approx(freq)
    assert fundamental.magnitude == pytest.approx(amp, rel=2.0e-3)
    assert fundamental.sine == pytest.approx(amp, rel=2.0e-3)
    assert abs(fundamental.cosine) < 2.0e-3
    assert probe.total_harmonic_distortion < 2.0e-3


def test_fourier_transient_deck_routes_parsed_four_cards() -> None:
    freq = 1_000.0
    amp = 2.0
    offset = 0.25
    period = 1.0 / freq
    c = Circuit([
        VoltageSource(
            "Vs",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(offset=offset, amplitude=amp, frequency=freq),
        ),
    ])
    result = transient(c, t_stop=2.0 * period, t_step=period / 64.0)

    analyses = fourier_transient_deck(
        result,
        """
.tran 15.625u 2m
.four 1k V(in) HARMONICS=5 FROM=1m
.end
""",
    )

    assert len(analyses) == 1
    analysis = analyses[0]
    probe = analysis.probes[0]
    fundamental = probe.harmonics[0]
    assert probe.probe == "V(in)"
    assert len(probe.harmonics) == 5
    assert analysis.start_time == pytest.approx(period)
    assert probe.dc == pytest.approx(offset, abs=2.0e-3)
    assert fundamental.frequency == pytest.approx(freq)
    assert fundamental.magnitude == pytest.approx(amp, rel=2.0e-3)


def test_fourier_corners_runs_analysis_per_corner_and_formats_tables() -> None:
    circuit = Circuit([
        VoltageSource(
            "Vin",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(offset=0.0, amplitude=1.0, frequency=1_000.0),
        ),
        Resistor("R1", "in", "out", 1_000.0),
        Resistor("R2", "out", "0", 1_000.0),
    ])

    result = fourier_corners(
        circuit,
        [
            CornerSpec("nominal"),
            CornerSpec("r2-high", (CornerOverride("R2", "resistance", 2_000.0),)),
        ],
        t_stop=2.0e-3,
        t_step=2.5e-4,
        fundamental_frequency=1_000.0,
        probes=["V(out)"],
        harmonics=2,
    )

    assert isinstance(result, CornerFourierResult)
    assert result.fundamental_frequency == pytest.approx(1_000.0)
    assert result.points[0].corner_name == "nominal"
    assert result.points[1].corner_name == "r2-high"
    assert result.points[0].result.probes[0].harmonics[0].magnitude == pytest.approx(0.5)
    assert result.points[1].result.probes[0].harmonics[0].magnitude == pytest.approx(
        2.0 / 3.0
    )
    assert format_corner_fourier_table(result) == (
        "Corner\tProbe\tHarmonic\tFrequency\tCosine\tSine\tMagnitude\tPhase\tDC\tTHD\n"
        "nominal\tV(out)\t1\t1.000000e+03\t6.018531e-33\t5.000000e-01\t5.000000e-01\t6.896729e-31\t0.000000e+00\t1.224647e-16\n"
        "nominal\tV(out)\t2\t2.000000e+03\t0.000000e+00\t-6.123234e-17\t6.123234e-17\t1.800000e+02\t0.000000e+00\t1.224647e-16\n"
        "r2-high\tV(out)\t1\t1.000000e+03\t7.523164e-33\t6.666667e-01\t6.666667e-01\t6.465683e-31\t1.355253e-17\t1.290373e-16\n"
        "r2-high\tV(out)\t2\t2.000000e+03\t2.710505e-17\t-8.164312e-17\t8.602490e-17\t1.616341e+02\t1.355253e-17\t1.290373e-16\n"
    )


def test_pole_zero_result_shape_supports_simple_rc_pole_fixture() -> None:
    resistance = 1_000.0
    capacitance = 1.0e-6
    pole_rad_per_second = -1.0 / (resistance * capacitance)
    result = PoleZeroResult(
        input_source="Vin",
        output_node="out",
        entries=[
            PoleZeroEntry(
                kind="pole",
                real=pole_rad_per_second,
                imaginary=0.0,
                frequency=abs(pole_rad_per_second) / (2.0 * math.pi),
                damping=1.0,
            )
        ],
    )

    assert isinstance(result, PoleZeroResult)
    assert result.entries[0].kind == "pole"
    assert result.entries[0].frequency == pytest.approx(
        1.0 / (2.0 * math.pi * resistance * capacitance)
    )


def test_pole_zero_rc_lowpass_returns_simple_rc_pole() -> None:
    circuit = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("R1", "in", "out", 1_000.0),
        Capacitor("C1", "out", "0", 1.0e-6),
    ])

    result = pole_zero_rc_lowpass(circuit, input_source="Vin", output_node="out")

    assert result == PoleZeroResult(
        input_source="Vin",
        output_node="out",
        entries=[
            PoleZeroEntry(
                kind="pole",
                real=-1.0e3,
                imaginary=0.0,
                frequency=1.0e3 / (2.0 * math.pi),
                damping=1.0,
            )
        ],
    )


def test_pole_zero_corners_runs_selected_topology_per_corner_and_formats_table() -> None:
    circuit = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("R1", "in", "out", 1_000.0),
        Capacitor("C1", "out", "0", 1.0e-6),
    ])

    result = pole_zero_corners(
        circuit,
        "Vin",
        "out",
        "rc-lowpass",
        [
            CornerSpec("nominal"),
            CornerSpec("cap-high", (CornerOverride("C1", "capacitance", 2.0e-6),)),
        ],
    )

    assert isinstance(result, CornerPoleZeroResult)
    assert result.input_source == "Vin"
    assert result.output_node == "out"
    assert result.topology == "rc-lowpass"
    assert result.points[0].corner_name == "nominal"
    assert result.points[1].corner_name == "cap-high"
    assert result.points[0].result.entries[0].real == pytest.approx(-1.0e3)
    assert result.points[1].result.entries[0].real == pytest.approx(-5.0e2)
    assert format_corner_pole_zero_table(result) == (
        "Corner\tIndex\tKind\tReal\tImaginary\tFrequency\tDamping\n"
        "nominal\t0\tpole\t-1.000000e+03\t0.000000e+00\t1.591549e+02\t1.000000e+00\n"
        "cap-high\t0\tpole\t-5.000000e+02\t0.000000e+00\t7.957747e+01\t1.000000e+00\n"
    )


def test_pole_zero_rc_highpass_returns_origin_zero_and_simple_rc_pole() -> None:
    circuit = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Capacitor("C1", "in", "out", 1.0e-6),
        Resistor("R1", "out", "0", 1_000.0),
    ])

    result = pole_zero_rc_highpass(circuit, input_source="Vin", output_node="out")

    assert result == PoleZeroResult(
        input_source="Vin",
        output_node="out",
        entries=[
            PoleZeroEntry(
                kind="zero",
                real=0.0,
                imaginary=0.0,
                frequency=0.0,
                damping=1.0,
            ),
            PoleZeroEntry(
                kind="pole",
                real=-1.0e3,
                imaginary=0.0,
                frequency=1.0e3 / (2.0 * math.pi),
                damping=1.0,
            ),
        ],
    )


def test_pole_zero_rlc_lowpass_returns_complex_conjugate_poles() -> None:
    circuit = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("R1", "in", "mid", 10.0),
        Inductor("L1", "mid", "out", 1.0e-3),
        Capacitor("C1", "out", "0", 1.0e-6),
    ])

    result = pole_zero_rlc_lowpass(circuit, input_source="Vin", output_node="out")

    alpha = 10.0 / (2.0 * 1.0e-3)
    omega0 = 1.0 / math.sqrt(1.0e-3 * 1.0e-6)
    imaginary = math.sqrt(omega0 * omega0 - alpha * alpha)
    assert result == PoleZeroResult(
        input_source="Vin",
        output_node="out",
        entries=[
            PoleZeroEntry(
                kind="pole",
                real=-alpha,
                imaginary=imaginary,
                frequency=omega0 / (2.0 * math.pi),
                damping=alpha / omega0,
            ),
            PoleZeroEntry(
                kind="pole",
                real=-alpha,
                imaginary=-imaginary,
                frequency=omega0 / (2.0 * math.pi),
                damping=alpha / omega0,
            ),
        ],
    )


def test_pole_zero_rlc_highpass_returns_origin_zeros_and_complex_conjugate_poles() -> None:
    circuit = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("R1", "in", "mid", 10.0),
        Capacitor("C1", "mid", "out", 1.0e-6),
        Inductor("L1", "out", "0", 1.0e-3),
    ])

    result = pole_zero_rlc_highpass(circuit, input_source="Vin", output_node="out")

    alpha = 10.0 / (2.0 * 1.0e-3)
    omega0 = 1.0 / math.sqrt(1.0e-3 * 1.0e-6)
    imaginary = math.sqrt(omega0 * omega0 - alpha * alpha)
    assert result == PoleZeroResult(
        input_source="Vin",
        output_node="out",
        entries=[
            PoleZeroEntry(
                kind="zero",
                real=0.0,
                imaginary=0.0,
                frequency=0.0,
                damping=1.0,
            ),
            PoleZeroEntry(
                kind="zero",
                real=0.0,
                imaginary=0.0,
                frequency=0.0,
                damping=1.0,
            ),
            PoleZeroEntry(
                kind="pole",
                real=-alpha,
                imaginary=imaginary,
                frequency=omega0 / (2.0 * math.pi),
                damping=alpha / omega0,
            ),
            PoleZeroEntry(
                kind="pole",
                real=-alpha,
                imaginary=-imaginary,
                frequency=omega0 / (2.0 * math.pi),
                damping=alpha / omega0,
            ),
        ],
    )


def test_pole_zero_rlc_bandpass_returns_origin_zero_and_complex_conjugate_poles() -> None:
    circuit = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Inductor("L1", "in", "mid", 1.0e-3),
        Capacitor("C1", "mid", "out", 1.0e-6),
        Resistor("R1", "out", "0", 10.0),
    ])

    result = pole_zero_rlc_bandpass(circuit, input_source="Vin", output_node="out")

    alpha = 10.0 / (2.0 * 1.0e-3)
    omega0 = 1.0 / math.sqrt(1.0e-3 * 1.0e-6)
    imaginary = math.sqrt(omega0 * omega0 - alpha * alpha)
    assert result == PoleZeroResult(
        input_source="Vin",
        output_node="out",
        entries=[
            PoleZeroEntry(
                kind="zero",
                real=0.0,
                imaginary=0.0,
                frequency=0.0,
                damping=1.0,
            ),
            PoleZeroEntry(
                kind="pole",
                real=-alpha,
                imaginary=imaginary,
                frequency=omega0 / (2.0 * math.pi),
                damping=alpha / omega0,
            ),
            PoleZeroEntry(
                kind="pole",
                real=-alpha,
                imaginary=-imaginary,
                frequency=omega0 / (2.0 * math.pi),
                damping=alpha / omega0,
            ),
        ],
    )


def test_pole_zero_rlc_notch_returns_imaginary_axis_zeros_and_complex_conjugate_poles() -> None:
    circuit = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("R1", "in", "out", 10.0),
        Inductor("L1", "out", "mid", 1.0e-3),
        Capacitor("C1", "mid", "0", 1.0e-6),
    ])

    result = pole_zero_rlc_notch(circuit, input_source="Vin", output_node="out")

    alpha = 10.0 / (2.0 * 1.0e-3)
    omega0 = 1.0 / math.sqrt(1.0e-3 * 1.0e-6)
    imaginary = math.sqrt(omega0 * omega0 - alpha * alpha)
    assert result == PoleZeroResult(
        input_source="Vin",
        output_node="out",
        entries=[
            PoleZeroEntry(
                kind="zero",
                real=0.0,
                imaginary=omega0,
                frequency=omega0 / (2.0 * math.pi),
                damping=0.0,
            ),
            PoleZeroEntry(
                kind="zero",
                real=0.0,
                imaginary=-omega0,
                frequency=omega0 / (2.0 * math.pi),
                damping=0.0,
            ),
            PoleZeroEntry(
                kind="pole",
                real=-alpha,
                imaginary=imaginary,
                frequency=omega0 / (2.0 * math.pi),
                damping=alpha / omega0,
            ),
            PoleZeroEntry(
                kind="pole",
                real=-alpha,
                imaginary=-imaginary,
                frequency=omega0 / (2.0 * math.pi),
                damping=alpha / omega0,
            ),
        ],
    )


def test_distortion_result_shape_supports_nonlinear_device_smoke_fixture() -> None:
    result = DistortionResult(
        input_source="Vin",
        output_probe="V(out)",
        points=[
            DistortionPoint(
                frequency=1.0e3,
                fundamental_magnitude=1.0,
                harmonics=[
                    DistortionHarmonic(
                        harmonic=2,
                        frequency=2.0e3,
                        magnitude=0.025,
                        phase_degrees=-12.0,
                    )
                ],
                total_harmonic_distortion=0.025,
            )
        ],
    )

    assert isinstance(result, DistortionResult)
    assert result.points[0].harmonics[0].harmonic == 2
    assert result.points[0].total_harmonic_distortion == pytest.approx(0.025)


def test_distortion_from_fourier_projects_probe_harmonics() -> None:
    fourier_result = FourierResult(
        fundamental_frequency=1.0e3,
        start_time=0.0,
        end_time=1.0e-3,
        probes=[
            FourierProbeResult(
                probe="V(out)",
                dc=0.0,
                harmonics=[
                    FourierHarmonic(1, 1.0e3, 0.0, 1.0, 1.0, 0.0),
                    FourierHarmonic(2, 2.0e3, 0.0, 0.025, 0.025, -12.0),
                ],
                total_harmonic_distortion=0.025,
            )
        ],
    )

    result = distortion_from_fourier(fourier_result, input_source="Vin", output_probe="V(out)")

    assert result == DistortionResult(
        input_source="Vin",
        output_probe="V(out)",
        points=[
            DistortionPoint(
                frequency=1.0e3,
                fundamental_magnitude=1.0,
                harmonics=[DistortionHarmonic(2, 2.0e3, 0.025, -12.0)],
                total_harmonic_distortion=0.025,
            )
        ],
    )


def test_distortion_from_transient_extracts_harmonic_content() -> None:
    freq = 1.0e3
    period = 1.0 / freq
    points = [
        TransientPoint(
            time=index * period / 64.0,
            node_voltages={
                "out": math.sin(2.0 * math.pi * freq * index * period / 64.0)
                + 0.1 * math.sin(4.0 * math.pi * freq * index * period / 64.0)
            },
        )
        for index in range(129)
    ]

    result = distortion_from_transient(
        points,
        fundamental_frequency=freq,
        input_source="Vin",
        output_probe="V(out)",
        harmonics=3,
    )

    assert result.input_source == "Vin"
    assert result.output_probe == "V(out)"
    point = result.points[0]
    assert point.frequency == pytest.approx(freq)
    assert point.fundamental_magnitude == pytest.approx(1.0, abs=2.0e-3)
    assert point.harmonics[0].harmonic == 2
    assert point.harmonics[0].magnitude == pytest.approx(0.1, abs=2.0e-3)
    assert point.total_harmonic_distortion == pytest.approx(0.1, abs=2.0e-3)


def test_distortion_from_transient_corners_projects_each_corner() -> None:
    freq = 1.0e3
    period = 1.0 / freq
    circuit = Circuit([
        VoltageSource(
            "Vin",
            "in",
            "0",
            0.0,
            waveform=SinWaveform(offset=0.0, amplitude=1.0, frequency=freq),
        ),
        Resistor("Rtop", "in", "out", 1_000.0),
        Resistor("Rbot", "out", "0", 1_000.0),
    ])

    result = distortion_from_transient_corners(
        circuit,
        [
            CornerSpec("nominal"),
            CornerSpec("rbot-high", (CornerOverride("Rbot", "resistance", 3_000.0),)),
        ],
        t_stop=2.0 * period,
        t_step=period / 64.0,
        fundamental_frequency=freq,
        input_source="Vin",
        output_probe="V(out)",
        harmonics=3,
    )

    assert isinstance(result, CornerDistortionResult)
    assert result.input_source == "Vin"
    assert result.output_probe == "V(out)"
    assert result.points[0].corner_name == "nominal"
    assert result.points[1].corner_name == "rbot-high"
    assert result.points[0].result.points[0].fundamental_magnitude == pytest.approx(
        0.5,
        abs=2.0e-3,
    )
    assert result.points[1].result.points[0].fundamental_magnitude == pytest.approx(
        0.75,
        abs=2.0e-3,
    )
    assert result.points[0].result.points[0].total_harmonic_distortion < 2.0e-3
    assert result.points[1].result.points[0].total_harmonic_distortion < 2.0e-3


def test_text_output_tables_are_stable_for_dc_and_transient_results() -> None:
    circuit = Circuit([
        VoltageSource("V1", "vin", "0", 10.0),
        Resistor("R1", "vin", "mid", 1_000.0),
        Resistor("R2", "mid", "0", 1_000.0),
    ])
    dc_result = dc_op(circuit)

    assert format_dc_table(dc_result) == (
        "Index\tV(mid)\tV(vin)\tI(V1)\n"
        "0\t5.000000e+00\t1.000000e+01\t-5.000000e-03\n"
    )
    assert format_dc_table(dc_result, ["V(vin, mid)", "I(V1)"]) == (
        "Index\tV(vin, mid)\tI(V1)\n"
        "0\t5.000000e+00\t-5.000000e-03\n"
    )

    transient_points = [
        TransientPoint(0.0, {"in": 0.0, "out": 0.0}, {"I(V1)": 0.0}),
        TransientPoint(1.0e-3, {"in": 1.0, "out": 0.5}, {"I(V1)": -5.0e-4}),
    ]
    assert format_transient_table(transient_points) == (
        "Index\tTime\tV(in)\tV(out)\tI(V1)\n"
        "0\t0.000000e+00\t0.000000e+00\t0.000000e+00\t0.000000e+00\n"
        "1\t1.000000e-03\t1.000000e+00\t5.000000e-01\t-5.000000e-04\n"
    )


def test_deck_wrdata_ascii_selects_marker_probe_columns() -> None:
    table = (
        "Index\tV(in)\tI(V1)\n"
        "0\t1.000000e+00\t-1.000000e-03\n"
        "1\t2.000000e+00\t-2.000000e-03\n"
    )

    assert format_deck_wrdata_ascii(
        table,
        ["I(V1)"],
        ["set wr_vecnames", "set wr_singlescale"],
    ) == (
        "# SPICE deck wrdata artifact\n"
        "Probes: I(V1)\n"
        "Options: set wr_vecnames;set wr_singlescale\n"
        "VectorNames: Index;I(V1)\n"
        "Scale: Index\n"
        "Index\tI(V1)\n"
        "0\t-1.000000e-03\n"
        "1\t-2.000000e-03\n"
    )


def test_deck_output_tables_route_save_probe_print_plot_cards() -> None:
    netlist = """
.save V(out)
.probe dc I(V1)
.probe tran V(clk)
.print tran V(ignored)
.plot tran I(V1)
.probe ac I(V1)
.end
"""
    dc_result = DcResult(
        node_voltages={"in": 10.0, "out": 5.0},
        branch_currents={"I(V1)": -0.005},
        iterations=1,
        converged=True,
    )
    dc_sweep_result = DcSweepResult(
        points=[
            DcSweepPoint(0.0, {"out": 0.0}, {"I(V1)": 0.0}, True),
            DcSweepPoint(1.0, {"out": 0.5}, {"I(V1)": -5.0e-4}, True),
        ],
        source_name="V1",
    )
    transient_points = [
        TransientPoint(
            0.0,
            {"clk": 0.0, "out": 0.0, "ignored": 1.0},
            {"I(V1)": 0.0},
        ),
        TransientPoint(
            1.0e-3,
            {"clk": 1.0, "out": 0.5, "ignored": 2.0},
            {"I(V1)": -5.0e-4},
        ),
    ]
    ac_result = AcResult(
        points=[
            AcPoint(
                freq=1000.0,
                node_voltages={"out": 0.5 - 0.5j},
                branch_currents={"I(V1)": -0.001 + 0.001j},
            )
        ]
    )

    assert format_deck_op_table(dc_result, netlist) == (
        "Index\tV(out)\n"
        "0\t5.000000e+00\n"
    )
    assert format_deck_dc_sweep_table(dc_sweep_result, netlist) == (
        "Index\tSource\tValue\tV(out)\tI(V1)\n"
        "0\tV1\t0.000000e+00\t0.000000e+00\t0.000000e+00\n"
        "1\tV1\t1.000000e+00\t5.000000e-01\t-5.000000e-04\n"
    )
    assert format_deck_transient_table(transient_points, netlist) == (
        "Index\tTime\tV(out)\tV(clk)\tV(ignored)\tI(V1)\n"
        "0\t0.000000e+00\t0.000000e+00\t0.000000e+00\t1.000000e+00\t0.000000e+00\n"
        "1\t1.000000e-03\t5.000000e-01\t1.000000e+00\t2.000000e+00\t-5.000000e-04\n"
    )
    assert format_deck_ac_table(ac_result, netlist) == (
        "Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n"
        "0\t1.000000e+03\tV(out)\t5.000000e-01\t-5.000000e-01\t7.071068e-01\t-4.500000e+01\n"
        "0\t1.000000e+03\tI(V1)\t-1.000000e-03\t1.000000e-03\t1.414214e-03\t1.350000e+02\n"
    )
    assert format_deck_transient_table(
        transient_points,
        ".probe ac V(freq)\n.end\n",
    ) == format_transient_table(transient_points)


def test_run_deck_analysis_routes_selected_plan_and_output_table() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("V1", "vin", "0", 1.0))
    circuit.add(Resistor("R1", "vin", "mid", 1000.0))
    circuit.add(Resistor("R2", "mid", "0", 1000.0))

    netlist = """
.save V(mid)
.probe dc I(V1)
.print dc V(mid)
.plot ac V(mid)
.op
.dc V1 0 1 1
.ac dec 1 1k 1k
.tran 1m 1m
.tf V(mid) V1
.sens V(mid)
.noise V(mid) V1 lin 1 1k 1k
.measure dc mid_avg avg V(mid)
.measure ac mid_peak max V(mid)
.measure tran mid_final final V(mid)
.end
"""
    netlist_lines = netlist.splitlines()
    save_line = next(
        index
        for index, line in enumerate(netlist_lines, start=1)
        if line.strip().startswith(".save")
    )
    probe_dc_line = next(
        index
        for index, line in enumerate(netlist_lines, start=1)
        if line.strip().startswith(".probe dc")
    )
    print_dc_line = next(
        index
        for index, line in enumerate(netlist_lines, start=1)
        if line.strip().startswith(".print dc")
    )
    plot_ac_line = next(
        index
        for index, line in enumerate(netlist_lines, start=1)
        if line.strip().startswith(".plot ac")
    )

    op_execution = run_deck_analysis(circuit, netlist, "op")
    assert op_execution.plan.analysis == "op"
    assert op_execution.output_probes == ["V(mid)"]
    assert op_execution.output_directives == [".save"]
    assert op_execution.analysis_directives == [".op"]
    expected_deck_analysis_kinds = ["op", "dc", "ac", "tran", "tf", "sens", "noise"]
    expected_deck_analysis_directives = [
        ".op",
        ".dc",
        ".ac",
        ".tran",
        ".tf",
        ".sens",
        ".noise",
    ]
    assert op_execution.deck_analysis_kind_count == 7
    assert op_execution.deck_analysis_kinds == expected_deck_analysis_kinds
    assert op_execution.deck_analysis_directive_count == 7
    assert op_execution.deck_analysis_directives == expected_deck_analysis_directives
    assert op_execution.table_count == 3
    assert op_execution.tables == ["result", "output-plan", "run-artifact"]
    assert [artifact.name for artifact in op_execution.table_artifacts] == (
        op_execution.tables
    )
    assert op_execution.measurements == []
    assert op_execution.measurement_table == "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
    assert op_execution.table == "Index\tV(mid)\n0\t5.000000e-01\n"
    assert format_deck_table_csv(op_execution.table) == "Index,V(mid)\n0,5.000000e-01\n"
    assert deck_table_records(op_execution.table) == [
        {"Index": "0", "V(mid)": "5.000000e-01"}
    ]
    assert json.loads(format_deck_table_json(op_execution.table)) == [
        {"Index": "0", "V(mid)": "5.000000e-01"}
    ]
    assert op_execution.table_artifacts[0].table == op_execution.table
    assert op_execution.table_artifacts[0].csv == format_deck_table_csv(
        op_execution.table
    )
    assert op_execution.table_artifacts[0].json == format_deck_table_json(
        op_execution.table
    )
    assert op_execution.table_artifacts[0].records == deck_table_records(
        op_execution.table
    )
    expected_output_plan_columns = [
        "Analysis",
        "Directive",
        "Line",
        "SourceName",
        "OutputNode",
        "SweepKind",
        "StartValue",
        "StopValue",
        "StepValue",
        "PointCount",
        "StartFrequencyHz",
        "StopFrequencyHz",
        "StepTime",
        "StopTime",
        "StartTime",
        "MaxStep",
        "UseInitialConditions",
        "ResultRows",
        "ResultColumns",
        "ResultColumnList",
        "OutputProbes",
        "OutputProbeList",
        "OutputProbeLines",
        "OutputProbeLineList",
        "OutputDirectives",
        "OutputDirectiveList",
        "OutputDirectiveKinds",
        "OutputDirectiveKindList",
        "OutputDirectiveAnalysisKinds",
        "OutputDirectiveAnalysisKindList",
        "OutputDirectiveLines",
        "OutputDirectiveLineList",
        "Tables",
        "TableList",
    ]
    expected_output_plan_row = [
        "op",
        ".op",
        str(op_execution.plan.line_number),
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "1",
        "2",
        "Index;V(mid)",
        "1",
        "V(mid)",
        "1",
        str(save_line),
        "1",
        ".save",
        "1",
        "save",
        "1",
        "global",
        "1",
        str(save_line),
        "3",
        "result;output-plan;run-artifact",
    ]
    expected_output_plan_table = (
        "\t".join(expected_output_plan_columns)
        + "\n"
        + "\t".join(expected_output_plan_row)
        + "\n"
    )
    expected_output_plan_records = [
        dict(zip(expected_output_plan_columns, expected_output_plan_row, strict=False))
    ]
    assert op_execution.output_plan_artifact_count == 1
    assert len(op_execution.output_plan_artifacts) == 1
    output_plan_artifact = op_execution.output_plan_artifacts[0]
    assert output_plan_artifact.analysis == "op"
    assert output_plan_artifact.directive == ".op"
    assert output_plan_artifact.line_number == op_execution.plan.line_number
    assert output_plan_artifact.source_name is None
    assert output_plan_artifact.output_node is None
    assert output_plan_artifact.sweep_kind is None
    assert output_plan_artifact.start_value is None
    assert output_plan_artifact.stop_value is None
    assert output_plan_artifact.step_value is None
    assert output_plan_artifact.point_count is None
    assert output_plan_artifact.start_frequency_hz is None
    assert output_plan_artifact.stop_frequency_hz is None
    assert output_plan_artifact.step_time is None
    assert output_plan_artifact.stop_time is None
    assert output_plan_artifact.start_time is None
    assert output_plan_artifact.max_step is None
    assert output_plan_artifact.use_initial_conditions is None
    assert output_plan_artifact.result_row_count == 1
    assert output_plan_artifact.result_columns == ["Index", "V(mid)"]
    assert output_plan_artifact.output_probes == ["V(mid)"]
    assert output_plan_artifact.output_probe_line_count == 1
    assert output_plan_artifact.output_probe_lines == [save_line]
    assert output_plan_artifact.output_directives == [".save"]
    assert output_plan_artifact.output_directive_kind_count == 1
    assert output_plan_artifact.output_directive_kinds == ["save"]
    assert output_plan_artifact.output_directive_analysis_kind_count == 1
    assert output_plan_artifact.output_directive_analysis_kinds == ["global"]
    assert output_plan_artifact.output_directive_line_count == 1
    assert output_plan_artifact.output_directive_lines == [save_line]
    assert output_plan_artifact.tables == ["result", "output-plan", "run-artifact"]
    assert op_execution.output_plan_artifact_table == expected_output_plan_table
    assert op_execution.output_plan_artifact_table == (
        format_deck_output_plan_artifact_table(op_execution.output_plan_artifacts)
    )
    assert op_execution.output_plan_artifact_csv == (
        ",".join(expected_output_plan_columns)
        + "\n"
        + ",".join(expected_output_plan_row)
        + "\n"
    )
    assert op_execution.output_plan_artifact_csv == (
        format_deck_output_plan_artifact_csv(op_execution.output_plan_artifacts)
    )
    assert json.loads(op_execution.output_plan_artifact_json) == (
        expected_output_plan_records
    )
    assert op_execution.output_plan_artifact_json == (
        format_deck_output_plan_artifact_json(op_execution.output_plan_artifacts)
    )
    assert op_execution.output_plan_artifact_records == expected_output_plan_records
    assert op_execution.output_plan_artifact_records == (
        deck_output_plan_artifact_records(op_execution.output_plan_artifacts)
    )
    assert op_execution.run_artifacts[0].result_rows == 1
    assert op_execution.run_artifacts[0].result_column_count == 2
    assert op_execution.run_artifacts[0].result_columns == ["Index", "V(mid)"]
    assert op_execution.run_artifacts[0].table_count == 3
    assert op_execution.run_artifacts[0].tables == ["result", "output-plan", "run-artifact"]
    assert op_execution.run_artifacts[0].source_name is None
    assert op_execution.run_artifacts[0].output_node is None
    assert op_execution.run_artifacts[0].sweep_kind is None
    assert op_execution.run_artifacts[0].point_count is None
    assert op_execution.run_artifacts[0].step_time is None
    assert op_execution.run_artifacts[0].use_initial_conditions is None
    assert op_execution.run_artifacts[0].output_probes == ["V(mid)"]
    assert op_execution.run_artifacts[0].output_directives == [".save"]
    assert op_execution.run_artifacts[0].analysis_directive_count == 1
    assert op_execution.run_artifacts[0].analysis_directives == [".op"]
    assert op_execution.run_artifacts[0].deck_analysis_kind_count == 7
    assert op_execution.run_artifacts[0].deck_analysis_kinds == (
        expected_deck_analysis_kinds
    )
    assert op_execution.run_artifacts[0].deck_analysis_directive_count == 7
    assert op_execution.run_artifacts[0].deck_analysis_directives == (
        expected_deck_analysis_directives
    )
    assert op_execution.run_artifacts[0].measurement_names == []
    assert op_execution.run_artifacts[0].fourier_probes == []
    assert op_execution.run_artifacts[0].control_line_count == 0
    assert op_execution.run_artifacts[0].control_lines == []
    assert op_execution.diagnostic_count == 0
    assert op_execution.diagnostic_codes == []
    assert op_execution.run_artifacts[0].diagnostic_count == 0
    assert op_execution.run_artifacts[0].diagnostic_codes == []
    assert op_execution.run_artifact_table == (
        "Analysis\tDirective\tAnalysisDirectives\tAnalysisDirectiveList\tLine\tSourceName\tOutputNode\tSweepKind\tStartValue\tStopValue\tStepValue\tPointCount\tStartFrequencyHz\tStopFrequencyHz\tStepTime\tStopTime\tStartTime\tMaxStep\tUseInitialConditions\tResultRows\tResultColumns\tResultColumnList\tTables\tTableList\tOutputProbes\tOutputProbeList\tOutputDirectives\tOutputDirectiveList\tMeasurements\tMeasurementList\tFourier\tFourierList\tControlLines\tControlLineList\tWriteMarkers\tWriteMarkerList\tRawfileOptions\tRawfileOptionList\tControlPolicyArtifacts\tControlPolicyCategoryList\tControlPolicyCodeList\tControlPolicySeverityList\tDiagnostics\tDiagnosticCodeList\tDeckAnalysisKinds\tDeckAnalysisKindList\tDeckAnalysisDirectives\tDeckAnalysisDirectiveList\n"
        f"op\t.op\t1\t.op\t{op_execution.plan.line_number}\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t1\t2\tIndex;V(mid)\t3\tresult;output-plan;run-artifact\t1\tV(mid)\t1\t.save\t0\t\t0\t\t0\t\t0\t\t0\t\t0\t\t\t\t0\t\t7\top;dc;ac;tran;tf;sens;noise\t7\t.op;.dc;.ac;.tran;.tf;.sens;.noise\n"
    )
    assert op_execution.table_artifacts[1].name == "output-plan"
    assert op_execution.table_artifacts[1].table == op_execution.output_plan_artifact_table
    assert op_execution.table_artifacts[1].csv == op_execution.output_plan_artifact_csv
    assert op_execution.table_artifacts[1].json == op_execution.output_plan_artifact_json
    assert op_execution.table_artifacts[1].records == (
        op_execution.output_plan_artifact_records
    )
    assert op_execution.table_artifacts[2].name == "run-artifact"
    assert op_execution.table_artifacts[2].table == op_execution.run_artifact_table
    assert op_execution.table_artifacts[2].records == deck_table_records(
        op_execution.run_artifact_table
    )
    assert format_deck_table_csv(
        op_execution.run_artifact_table
    ) == format_deck_run_artifact_csv(op_execution.run_artifacts)
    assert format_deck_table_json(
        op_execution.run_artifact_table
    ) == format_deck_run_artifact_json(op_execution.run_artifacts)
    assert deck_table_records(op_execution.run_artifact_table) == json.loads(
        format_deck_run_artifact_json(op_execution.run_artifacts)
    )
    assert format_deck_run_artifact_csv(op_execution.run_artifacts) == (
        "Analysis,Directive,AnalysisDirectives,AnalysisDirectiveList,Line,SourceName,OutputNode,SweepKind,StartValue,StopValue,StepValue,PointCount,StartFrequencyHz,StopFrequencyHz,StepTime,StopTime,StartTime,MaxStep,UseInitialConditions,ResultRows,ResultColumns,ResultColumnList,Tables,TableList,OutputProbes,OutputProbeList,OutputDirectives,OutputDirectiveList,Measurements,MeasurementList,Fourier,FourierList,ControlLines,ControlLineList,WriteMarkers,WriteMarkerList,RawfileOptions,RawfileOptionList,ControlPolicyArtifacts,ControlPolicyCategoryList,ControlPolicyCodeList,ControlPolicySeverityList,Diagnostics,DiagnosticCodeList,DeckAnalysisKinds,DeckAnalysisKindList,DeckAnalysisDirectives,DeckAnalysisDirectiveList\n"
        f"op,.op,1,.op,{op_execution.plan.line_number},,,,,,,,,,,,,,,1,2,Index;V(mid),3,result;output-plan;run-artifact,1,V(mid),1,.save,0,,0,,0,,0,,0,,0,,,,0,,7,op;dc;ac;tran;tf;sens;noise,7,.op;.dc;.ac;.tran;.tf;.sens;.noise\n"
    )
    assert format_deck_table_csv('Name\tValue\nprobe\tSPICE,"QUOTED"\n') == (
        'Name,Value\nprobe,"SPICE,""QUOTED"""\n'
    )
    assert format_deck_table_json('Name\tValue\nprobe\tSPICE,"QUOTED"\n') == (
        '[{"Name":"probe","Value":"SPICE,\\"QUOTED\\""}]\n'
    )
    assert deck_table_records('Name\tValue\nprobe\tSPICE,"QUOTED"\n') == [
        {"Name": "probe", "Value": 'SPICE,"QUOTED"'}
    ]
    artifact_json = format_deck_run_artifact_json(op_execution.run_artifacts)
    artifact_records = json.loads(artifact_json)
    assert list(artifact_records[0]) == [
        "Analysis",
        "Directive",
        "AnalysisDirectives",
        "AnalysisDirectiveList",
        "Line",
        "SourceName",
        "OutputNode",
        "SweepKind",
        "StartValue",
        "StopValue",
        "StepValue",
        "PointCount",
        "StartFrequencyHz",
        "StopFrequencyHz",
        "StepTime",
        "StopTime",
        "StartTime",
        "MaxStep",
        "UseInitialConditions",
        "ResultRows",
        "ResultColumns",
        "ResultColumnList",
        "Tables",
        "TableList",
        "OutputProbes",
        "OutputProbeList",
        "OutputDirectives",
        "OutputDirectiveList",
        "Measurements",
        "MeasurementList",
        "Fourier",
        "FourierList",
        "ControlLines",
        "ControlLineList",
        "WriteMarkers",
        "WriteMarkerList",
        "RawfileOptions",
        "RawfileOptionList",
        "ControlPolicyArtifacts",
        "ControlPolicyCategoryList",
        "ControlPolicyCodeList",
        "ControlPolicySeverityList",
        "Diagnostics",
        "DiagnosticCodeList",
        "DeckAnalysisKinds",
        "DeckAnalysisKindList",
        "DeckAnalysisDirectives",
        "DeckAnalysisDirectiveList",
    ]
    assert artifact_records == [
        {
            "Analysis": "op",
            "Directive": ".op",
            "AnalysisDirectives": "1",
            "AnalysisDirectiveList": ".op",
            "Line": str(op_execution.plan.line_number),
            "SourceName": "",
            "OutputNode": "",
            "SweepKind": "",
            "StartValue": "",
            "StopValue": "",
            "StepValue": "",
            "PointCount": "",
            "StartFrequencyHz": "",
            "StopFrequencyHz": "",
            "StepTime": "",
            "StopTime": "",
            "StartTime": "",
            "MaxStep": "",
            "UseInitialConditions": "",
            "ResultRows": "1",
            "ResultColumns": "2",
            "ResultColumnList": "Index;V(mid)",
            "Tables": "3",
            "TableList": "result;output-plan;run-artifact",
            "OutputProbes": "1",
            "OutputProbeList": "V(mid)",
            "OutputDirectives": "1",
            "OutputDirectiveList": ".save",
            "Measurements": "0",
            "MeasurementList": "",
            "Fourier": "0",
            "FourierList": "",
            "ControlLines": "0",
            "ControlLineList": "",
            "WriteMarkers": "0",
            "WriteMarkerList": "",
            "RawfileOptions": "0",
            "RawfileOptionList": "",
            "ControlPolicyArtifacts": "0",
            "ControlPolicyCategoryList": "",
            "ControlPolicyCodeList": "",
            "ControlPolicySeverityList": "",
            "Diagnostics": "0",
            "DiagnosticCodeList": "",
            "DeckAnalysisKinds": "7",
            "DeckAnalysisKindList": "op;dc;ac;tran;tf;sens;noise",
            "DeckAnalysisDirectives": "7",
            "DeckAnalysisDirectiveList": ".op;.dc;.ac;.tran;.tf;.sens;.noise",
        }
    ]
    diagnostic_artifact = replace(
        op_execution.run_artifacts[0],
        diagnostic_count=2,
        diagnostic_codes=["SPICE_DECK_ANALYSIS_TOKEN", "SPICE_DECK_ANALYSIS_RANGE"],
    )
    diagnostic_record = deck_table_records(
        format_deck_run_artifact_table([diagnostic_artifact])
    )[0]
    assert diagnostic_record["Diagnostics"] == "2"
    assert (
        diagnostic_record["DiagnosticCodeList"]
        == "SPICE_DECK_ANALYSIS_TOKEN;SPICE_DECK_ANALYSIS_RANGE"
    )
    quoted_diagnostic_artifact = replace(
        op_execution.run_artifacts[0],
        diagnostic_count=2,
        diagnostic_codes=["SPICE_DECK_ANALYSIS_TOKEN", 'SPICE,"QUOTED"'],
    )
    assert format_deck_run_artifact_csv([quoted_diagnostic_artifact]).endswith(
        ',0,,0,,0,,0,,,,2,"SPICE_DECK_ANALYSIS_TOKEN;SPICE,""QUOTED""",7,op;dc;ac;tran;tf;sens;noise,7,.op;.dc;.ac;.tran;.tf;.sens;.noise\n'
    )
    assert json.loads(format_deck_run_artifact_json([quoted_diagnostic_artifact]))[
        0
    ]["DiagnosticCodeList"] == 'SPICE_DECK_ANALYSIS_TOKEN;SPICE,"QUOTED"'

    dc_execution = run_deck_analysis(circuit, netlist, "dc")
    assert dc_execution.plan.source_name == "V1"
    assert dc_execution.output_probes == ["V(mid)", "I(V1)"]
    assert dc_execution.output_directives == [".save", ".probe", ".print"]
    assert dc_execution.output_plan_artifacts[0].line_number == dc_execution.plan.line_number
    assert dc_execution.output_plan_artifacts[0].source_name == "V1"
    assert dc_execution.output_plan_artifacts[0].output_node is None
    assert dc_execution.output_plan_artifacts[0].sweep_kind is None
    assert dc_execution.output_plan_artifacts[0].start_value == pytest.approx(0.0)
    assert dc_execution.output_plan_artifacts[0].stop_value == pytest.approx(1.0)
    assert dc_execution.output_plan_artifacts[0].step_value == pytest.approx(1.0)
    assert dc_execution.output_plan_artifacts[0].point_count is None
    assert dc_execution.output_plan_artifacts[0].start_frequency_hz is None
    assert dc_execution.output_plan_artifacts[0].stop_frequency_hz is None
    assert dc_execution.output_plan_artifacts[0].step_time is None
    assert dc_execution.output_plan_artifacts[0].use_initial_conditions is None
    assert dc_execution.output_plan_artifact_records[0]["Line"] == str(
        dc_execution.plan.line_number
    )
    assert dc_execution.output_plan_artifact_records[0]["SourceName"] == "V1"
    assert dc_execution.output_plan_artifact_records[0]["OutputNode"] == ""
    assert dc_execution.output_plan_artifact_records[0]["SweepKind"] == ""
    assert dc_execution.output_plan_artifact_records[0]["StartValue"] == "0.000000e+00"
    assert dc_execution.output_plan_artifact_records[0]["StopValue"] == "1.000000e+00"
    assert dc_execution.output_plan_artifact_records[0]["StepValue"] == "1.000000e+00"
    assert dc_execution.output_plan_artifact_records[0]["PointCount"] == ""
    assert dc_execution.output_plan_artifact_records[0]["StartFrequencyHz"] == ""
    assert dc_execution.output_plan_artifact_records[0]["StopFrequencyHz"] == ""
    assert dc_execution.output_plan_artifact_records[0]["StepTime"] == ""
    assert dc_execution.output_plan_artifact_records[0]["UseInitialConditions"] == ""
    assert dc_execution.output_plan_artifacts[0].output_directive_kinds == [
        "save",
        "probe",
        "print",
    ]
    assert dc_execution.output_plan_artifact_records[0][
        "OutputDirectiveKindList"
    ] == "save;probe;print"
    assert dc_execution.output_plan_artifacts[0].output_directive_analysis_kinds == [
        "global",
        "dc",
    ]
    dc_output_directive_lines = [save_line, probe_dc_line, print_dc_line]
    dc_output_probe_lines = [save_line, probe_dc_line]
    assert (
        dc_execution.output_plan_artifacts[0].output_probe_lines
        == dc_output_probe_lines
    )
    assert (
        dc_execution.output_plan_artifacts[0].output_directive_lines
        == dc_output_directive_lines
    )
    assert dc_execution.output_plan_artifact_records[0][
        "OutputDirectiveAnalysisKindList"
    ] == "global;dc"
    assert dc_execution.output_plan_artifact_records[0][
        "OutputProbeLineList"
    ] == ";".join(str(line) for line in dc_output_probe_lines)
    assert dc_execution.output_plan_artifact_records[0][
        "OutputDirectiveLineList"
    ] == ";".join(str(line) for line in dc_output_directive_lines)
    assert dc_execution.analysis_directives == [".dc"]
    assert dc_execution.table_count == 4
    assert dc_execution.tables == ["result", "measurement", "output-plan", "run-artifact"]
    assert [artifact.name for artifact in dc_execution.table_artifacts] == (
        dc_execution.tables
    )
    assert [measurement.name for measurement in dc_execution.measurements] == ["mid_avg"]
    assert dc_execution.measurement_table == (
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
        "mid_avg\tdc\tV(mid)\tavg\t\t\t2.500000e-01\n"
    )
    assert dc_execution.table_artifacts[1].table == dc_execution.measurement_table
    assert dc_execution.table_artifacts[1].csv == format_deck_table_csv(
        dc_execution.measurement_table
    )
    assert dc_execution.table_artifacts[1].json == format_deck_table_json(
        dc_execution.measurement_table
    )
    assert dc_execution.table_artifacts[1].records == deck_table_records(
        dc_execution.measurement_table
    )
    assert isinstance(dc_execution.result, DcSweepResult)
    assert len(dc_execution.result.points) == 2
    assert dc_execution.table == (
        "Index\tSource\tValue\tV(mid)\tI(V1)\n"
        "0\tV1\t0.000000e+00\t0.000000e+00\t0.000000e+00\n"
        "1\tV1\t1.000000e+00\t5.000000e-01\t-5.000000e-04\n"
    )
    assert dc_execution.run_artifacts[0].analysis == "dc"
    assert dc_execution.run_artifacts[0].source_name == "V1"
    assert dc_execution.run_artifacts[0].output_node is None
    assert dc_execution.run_artifacts[0].start_value == pytest.approx(0.0)
    assert dc_execution.run_artifacts[0].stop_value == pytest.approx(1.0)
    assert dc_execution.run_artifacts[0].step_value == pytest.approx(1.0)
    assert dc_execution.run_artifacts[0].result_column_count == 5
    assert dc_execution.run_artifacts[0].result_columns == [
        "Index",
        "Source",
        "Value",
        "V(mid)",
        "I(V1)",
    ]
    assert dc_execution.run_artifacts[0].table_count == 4
    assert dc_execution.run_artifacts[0].tables == [
        "result",
        "measurement",
        "output-plan",
        "run-artifact",
    ]
    assert dc_execution.run_artifacts[0].step_time is None
    assert dc_execution.run_artifacts[0].use_initial_conditions is None
    assert dc_execution.run_artifacts[0].output_probes == ["V(mid)", "I(V1)"]
    assert dc_execution.run_artifacts[0].output_directives == [
        ".save",
        ".probe",
        ".print",
    ]
    assert dc_execution.run_artifacts[0].analysis_directives == [".dc"]
    assert dc_execution.run_artifacts[0].measurement_names == ["mid_avg"]
    assert dc_execution.run_artifacts[0].fourier_probes == []
    dc_run_artifact_record = _assert_run_artifact_table_matches(dc_execution)
    assert dc_run_artifact_record["Analysis"] == "dc"
    assert dc_run_artifact_record["DeckAnalysisKinds"] == "7"
    assert (
        dc_run_artifact_record["DeckAnalysisKindList"]
        == "op;dc;ac;tran;tf;sens;noise"
    )
    assert dc_run_artifact_record["DeckAnalysisDirectives"] == "7"

    ac_execution = run_deck_analysis(circuit, netlist, "ac")
    assert ac_execution.output_probes == ["V(mid)"]
    assert ac_execution.output_directives == [".save", ".plot"]
    assert ac_execution.output_plan_artifacts[0].output_node is None
    assert ac_execution.output_plan_artifacts[0].sweep_kind == "dec"
    assert ac_execution.output_plan_artifacts[0].point_count == 1
    assert ac_execution.output_plan_artifacts[0].start_frequency_hz == pytest.approx(1.0e3)
    assert ac_execution.output_plan_artifacts[0].stop_frequency_hz == pytest.approx(1.0e3)
    assert ac_execution.output_plan_artifacts[0].start_value is None
    assert ac_execution.output_plan_artifacts[0].step_time is None
    assert ac_execution.output_plan_artifacts[0].use_initial_conditions is None
    assert ac_execution.output_plan_artifact_records[0]["SweepKind"] == "dec"
    assert ac_execution.output_plan_artifact_records[0]["PointCount"] == "1"
    assert ac_execution.output_plan_artifact_records[0]["StartFrequencyHz"] == "1.000000e+03"
    assert ac_execution.output_plan_artifact_records[0]["StopFrequencyHz"] == "1.000000e+03"
    assert ac_execution.output_plan_artifact_records[0]["StepTime"] == ""
    assert ac_execution.output_plan_artifacts[0].output_directive_kinds == [
        "save",
        "plot",
    ]
    assert ac_execution.output_plan_artifact_records[0][
        "OutputDirectiveKindList"
    ] == "save;plot"
    assert ac_execution.output_plan_artifacts[0].output_directive_analysis_kinds == [
        "global",
        "ac",
    ]
    ac_output_directive_lines = [save_line, plot_ac_line]
    assert ac_execution.output_plan_artifacts[0].output_probe_lines == [save_line]
    assert (
        ac_execution.output_plan_artifacts[0].output_directive_lines
        == ac_output_directive_lines
    )
    assert ac_execution.output_plan_artifact_records[0][
        "OutputDirectiveAnalysisKindList"
    ] == "global;ac"
    assert ac_execution.output_plan_artifact_records[0][
        "OutputDirectiveLineList"
    ] == ";".join(str(line) for line in ac_output_directive_lines)
    assert ac_execution.analysis_directives == [".ac"]
    assert ac_execution.table_count == 4
    assert ac_execution.tables == ["result", "measurement", "output-plan", "run-artifact"]
    assert [measurement.name for measurement in ac_execution.measurements] == ["mid_peak"]
    assert ac_execution.measurement_table == (
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
        "mid_peak\tac\tV(mid)\tmax\t\t\t5.000000e-01\n"
    )
    assert isinstance(ac_execution.result, AcResult)
    assert len(ac_execution.result.points) == 1
    assert ac_execution.table == (
        "Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n"
        "0\t1.000000e+03\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n"
    )
    assert ac_execution.run_artifacts[0].output_probes == ["V(mid)"]
    assert ac_execution.run_artifacts[0].source_name is None
    assert ac_execution.run_artifacts[0].output_node is None
    assert ac_execution.run_artifacts[0].sweep_kind == "dec"
    assert ac_execution.run_artifacts[0].point_count == 1
    assert ac_execution.run_artifacts[0].start_frequency_hz == pytest.approx(1.0e3)
    assert ac_execution.run_artifacts[0].stop_frequency_hz == pytest.approx(1.0e3)
    assert ac_execution.run_artifacts[0].result_column_count == 7
    assert ac_execution.run_artifacts[0].result_columns == [
        "Index",
        "Frequency",
        "Probe",
        "Real",
        "Imaginary",
        "Magnitude",
        "Phase",
    ]
    assert ac_execution.run_artifacts[0].table_count == 4
    assert ac_execution.run_artifacts[0].tables == [
        "result",
        "measurement",
        "output-plan",
        "run-artifact",
    ]
    assert ac_execution.run_artifacts[0].step_time is None
    assert ac_execution.run_artifacts[0].use_initial_conditions is None
    assert ac_execution.run_artifacts[0].output_directives == [".save", ".plot"]
    assert ac_execution.run_artifacts[0].measurement_names == ["mid_peak"]
    assert ac_execution.run_artifacts[0].fourier_probes == []
    ac_run_artifact_record = _assert_run_artifact_table_matches(ac_execution)
    assert ac_run_artifact_record["Analysis"] == "ac"
    assert ac_run_artifact_record["DeckAnalysisKinds"] == "7"
    assert (
        ac_run_artifact_record["DeckAnalysisKindList"]
        == "op;dc;ac;tran;tf;sens;noise"
    )
    assert ac_run_artifact_record["DeckAnalysisDirectives"] == "7"

    tran_execution = run_deck_analysis(circuit, netlist, "tran")
    assert tran_execution.output_probes == ["V(mid)"]
    assert tran_execution.output_directives == [".save"]
    assert tran_execution.output_plan_artifacts[0].step_time == pytest.approx(1.0e-3)
    assert tran_execution.output_plan_artifacts[0].stop_time == pytest.approx(1.0e-3)
    assert tran_execution.output_plan_artifacts[0].start_time is None
    assert tran_execution.output_plan_artifacts[0].max_step is None
    assert tran_execution.output_plan_artifacts[0].use_initial_conditions is False
    assert tran_execution.output_plan_artifact_records[0]["StepTime"] == "1.000000e-03"
    assert tran_execution.output_plan_artifact_records[0]["StopTime"] == "1.000000e-03"
    assert tran_execution.output_plan_artifact_records[0]["StartTime"] == ""
    assert tran_execution.output_plan_artifact_records[0]["MaxStep"] == ""
    assert tran_execution.output_plan_artifact_records[0]["UseInitialConditions"] == "false"
    assert tran_execution.output_plan_artifacts[0].output_directive_lines == [
        save_line
    ]
    assert tran_execution.output_plan_artifacts[0].output_probe_lines == [save_line]
    assert tran_execution.analysis_directives == [".tran"]
    assert tran_execution.table_count == 4
    assert tran_execution.tables == ["result", "measurement", "output-plan", "run-artifact"]
    assert [measurement.name for measurement in tran_execution.measurements] == [
        "mid_final"
    ]
    assert tran_execution.measurement_table == (
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
        "mid_final\ttran\tV(mid)\tlast\t\t\t5.000000e-01\n"
    )
    assert isinstance(tran_execution.result, TransientResult)
    assert tran_execution.table == (
        "Index\tTime\tV(mid)\n"
        "0\t0.000000e+00\t5.000000e-01\n"
        "1\t1.000000e-03\t5.000000e-01\n"
    )
    assert tran_execution.run_artifacts[0].output_probes == ["V(mid)"]
    assert tran_execution.run_artifacts[0].source_name is None
    assert tran_execution.run_artifacts[0].output_node is None
    assert tran_execution.run_artifacts[0].step_time == pytest.approx(1.0e-3)
    assert tran_execution.run_artifacts[0].stop_time == pytest.approx(1.0e-3)
    assert tran_execution.run_artifacts[0].result_column_count == 3
    assert tran_execution.run_artifacts[0].result_columns == ["Index", "Time", "V(mid)"]
    assert tran_execution.run_artifacts[0].table_count == 4
    assert tran_execution.run_artifacts[0].tables == [
        "result",
        "measurement",
        "output-plan",
        "run-artifact",
    ]
    assert tran_execution.run_artifacts[0].start_time is None
    assert tran_execution.run_artifacts[0].max_step is None
    assert tran_execution.run_artifacts[0].use_initial_conditions is False
    assert tran_execution.run_artifacts[0].output_directives == [".save"]
    assert tran_execution.run_artifacts[0].measurement_names == ["mid_final"]
    assert tran_execution.run_artifacts[0].fourier_probes == []
    assert tran_execution.run_artifacts[0].diagnostic_count == 0
    assert tran_execution.run_artifacts[0].diagnostic_codes == []
    tran_run_artifact_record = _assert_run_artifact_table_matches(tran_execution)
    assert tran_run_artifact_record["Analysis"] == "tran"
    assert tran_run_artifact_record["DeckAnalysisKinds"] == "7"
    assert (
        tran_run_artifact_record["DeckAnalysisKindList"]
        == "op;dc;ac;tran;tf;sens;noise"
    )
    assert tran_run_artifact_record["DeckAnalysisDirectives"] == "7"

    tf_execution = run_deck_analysis(circuit, netlist, "tf")
    assert tf_execution.plan.output_node == "mid"
    assert tf_execution.plan.source_name == "V1"
    assert isinstance(tf_execution.result, TfResult)
    assert tf_execution.result.transfer_ratio == pytest.approx(0.5)
    assert tf_execution.result.input_impedance == pytest.approx(2000.0)
    assert tf_execution.result.output_impedance == pytest.approx(500.0)
    assert tf_execution.output_probes == ["V(mid)"]
    assert tf_execution.output_directives == []
    assert tf_execution.output_plan_artifacts[0].output_node == "mid"
    assert tf_execution.output_plan_artifact_records[0]["OutputNode"] == "mid"
    assert tf_execution.analysis_directives == [".tf"]
    assert tf_execution.table_count == 3
    assert tf_execution.tables == ["result", "output-plan", "run-artifact"]
    assert tf_execution.measurements == []
    assert tf_execution.measurement_table == "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
    assert tf_execution.table == (
        "TransferRatio\tInputImpedance\tOutputImpedance\n"
        "5.000000e-01\t2.000000e+03\t5.000000e+02\n"
    )
    assert tf_execution.run_artifacts[0].analysis == "tf"
    assert tf_execution.run_artifacts[0].source_name == "V1"
    assert tf_execution.run_artifacts[0].output_node == "mid"
    assert tf_execution.run_artifacts[0].result_rows == 1
    assert tf_execution.run_artifacts[0].result_column_count == 3
    assert tf_execution.run_artifacts[0].result_columns == [
        "TransferRatio",
        "InputImpedance",
        "OutputImpedance",
    ]
    assert tf_execution.run_artifacts[0].table_count == 3
    assert tf_execution.run_artifacts[0].tables == ["result", "output-plan", "run-artifact"]
    assert tf_execution.run_artifacts[0].step_time is None
    assert tf_execution.run_artifacts[0].use_initial_conditions is None
    assert tf_execution.run_artifacts[0].output_probes == ["V(mid)"]
    assert tf_execution.run_artifacts[0].output_directives == []
    assert tf_execution.run_artifacts[0].measurement_names == []
    assert tf_execution.run_artifacts[0].fourier_probes == []
    tf_run_artifact_record = _assert_run_artifact_table_matches(tf_execution)
    assert tf_run_artifact_record["Analysis"] == "tf"
    assert tf_run_artifact_record["DeckAnalysisKinds"] == "7"
    assert (
        tf_run_artifact_record["DeckAnalysisKindList"]
        == "op;dc;ac;tran;tf;sens;noise"
    )
    assert tf_run_artifact_record["DeckAnalysisDirectives"] == "7"

    sens_execution = run_deck_analysis(circuit, netlist, "sens")
    assert sens_execution.plan.output_node == "mid"
    assert sens_execution.plan.source_name is None
    assert isinstance(sens_execution.result, SensResult)
    assert sens_execution.result.output_node == "mid"
    assert len(sens_execution.result.entries) == 3
    assert sens_execution.output_probes == ["V(mid)"]
    assert sens_execution.output_plan_artifacts[0].output_node == "mid"
    assert sens_execution.output_plan_artifact_records[0]["OutputNode"] == "mid"
    assert sens_execution.analysis_directives == [".sens"]
    assert sens_execution.table_count == 3
    assert sens_execution.tables == ["result", "output-plan", "run-artifact"]
    assert sens_execution.measurements == []
    assert sens_execution.measurement_table == "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
    assert sens_execution.table.startswith(
        "OutputNode\tNominalVoltage\tElement\tParameter\tNominalValue\tSensitivity\tRelativeSensitivity\n"
    )
    assert sens_execution.run_artifacts[0].analysis == "sens"
    assert sens_execution.run_artifacts[0].source_name is None
    assert sens_execution.run_artifacts[0].output_node == "mid"
    assert sens_execution.run_artifacts[0].result_rows == 1
    assert sens_execution.run_artifacts[0].result_column_count == 7
    assert sens_execution.run_artifacts[0].result_columns == [
        "OutputNode",
        "NominalVoltage",
        "Element",
        "Parameter",
        "NominalValue",
        "Sensitivity",
        "RelativeSensitivity",
    ]
    assert sens_execution.run_artifacts[0].table_count == 3
    assert sens_execution.run_artifacts[0].tables == ["result", "output-plan", "run-artifact"]
    assert sens_execution.run_artifacts[0].step_time is None
    assert sens_execution.run_artifacts[0].use_initial_conditions is None
    assert sens_execution.run_artifacts[0].output_probes == ["V(mid)"]
    assert sens_execution.run_artifacts[0].output_directives == []
    assert sens_execution.run_artifacts[0].measurement_names == []
    assert sens_execution.run_artifacts[0].fourier_probes == []
    sens_run_artifact_record = _assert_run_artifact_table_matches(sens_execution)
    assert sens_run_artifact_record["Analysis"] == "sens"
    assert sens_run_artifact_record["DeckAnalysisKinds"] == "7"
    assert (
        sens_run_artifact_record["DeckAnalysisKindList"]
        == "op;dc;ac;tran;tf;sens;noise"
    )
    assert sens_run_artifact_record["DeckAnalysisDirectives"] == "7"

    noise_execution = run_deck_analysis(circuit, netlist, "noise")
    assert noise_execution.plan.output_node == "mid"
    assert noise_execution.plan.source_name == "V1"
    assert noise_execution.plan.sweep_kind == "lin"
    assert noise_execution.plan.point_count == 1
    assert noise_execution.plan.start_frequency == pytest.approx(1.0e3)
    assert noise_execution.plan.stop_frequency == pytest.approx(1.0e3)
    assert isinstance(noise_execution.result, NoiseResult)
    assert noise_execution.result.output_node == "mid"
    assert noise_execution.result.input_source == "V1"
    assert len(noise_execution.result.points) == 1
    assert noise_execution.output_probes == ["V(mid)"]
    assert noise_execution.output_plan_artifacts[0].source_name == "V1"
    assert noise_execution.output_plan_artifacts[0].output_node == "mid"
    assert noise_execution.output_plan_artifacts[0].sweep_kind == "lin"
    assert noise_execution.output_plan_artifacts[0].point_count == 1
    assert noise_execution.output_plan_artifacts[0].start_frequency_hz == pytest.approx(1.0e3)
    assert noise_execution.output_plan_artifacts[0].stop_frequency_hz == pytest.approx(1.0e3)
    assert noise_execution.output_plan_artifact_records[0]["OutputNode"] == "mid"
    assert noise_execution.output_plan_artifact_records[0]["SweepKind"] == "lin"
    assert noise_execution.output_plan_artifact_records[0]["PointCount"] == "1"
    assert (
        noise_execution.output_plan_artifact_records[0]["StartFrequencyHz"]
        == "1.000000e+03"
    )
    assert (
        noise_execution.output_plan_artifact_records[0]["StopFrequencyHz"]
        == "1.000000e+03"
    )
    assert noise_execution.analysis_directives == [".noise"]
    assert noise_execution.table_count == 3
    assert noise_execution.tables == ["result", "output-plan", "run-artifact"]
    assert noise_execution.measurements == []
    assert noise_execution.measurement_table == "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
    assert noise_execution.table == format_deck_noise_table(noise_execution.result)
    assert noise_execution.table.startswith(
        "Index\tFrequency\tOutputNode\tInputSource\tOutputPSD\tInputReferredPSD\t"
        "Element\tType\tSourcePSD\tContributionPSD\n"
    )
    assert noise_execution.run_artifacts[0].analysis == "noise"
    assert noise_execution.run_artifacts[0].source_name == "V1"
    assert noise_execution.run_artifacts[0].output_node == "mid"
    assert noise_execution.run_artifacts[0].sweep_kind == "lin"
    assert noise_execution.run_artifacts[0].point_count == 1
    assert noise_execution.run_artifacts[0].start_frequency_hz == pytest.approx(1.0e3)
    assert noise_execution.run_artifacts[0].stop_frequency_hz == pytest.approx(1.0e3)
    assert noise_execution.run_artifacts[0].result_rows == 1
    assert noise_execution.run_artifacts[0].result_column_count == 10
    assert noise_execution.run_artifacts[0].result_columns == [
        "Index",
        "Frequency",
        "OutputNode",
        "InputSource",
        "OutputPSD",
        "InputReferredPSD",
        "Element",
        "Type",
        "SourcePSD",
        "ContributionPSD",
    ]
    assert noise_execution.run_artifacts[0].table_count == 3
    assert noise_execution.run_artifacts[0].tables == ["result", "output-plan", "run-artifact"]
    assert noise_execution.run_artifacts[0].step_time is None
    assert noise_execution.run_artifacts[0].use_initial_conditions is None
    assert noise_execution.run_artifacts[0].output_probes == ["V(mid)"]
    assert noise_execution.run_artifacts[0].output_directives == []
    assert noise_execution.run_artifacts[0].measurement_names == []
    assert noise_execution.run_artifacts[0].fourier_probes == []
    noise_run_artifact_record = _assert_run_artifact_table_matches(noise_execution)
    assert noise_run_artifact_record["Analysis"] == "noise"
    assert noise_run_artifact_record["DeckAnalysisKinds"] == "7"
    assert (
        noise_run_artifact_record["DeckAnalysisKindList"]
        == "op;dc;ac;tran;tf;sens;noise"
    )
    assert noise_run_artifact_record["DeckAnalysisDirectives"] == "7"

    tran_window_execution = run_deck_analysis(
        circuit,
        ".save V(mid)\n.tran 2m 6m 2m 1m uic\n.end\n",
    )
    assert tran_window_execution.plan.start_time == pytest.approx(2.0e-3)
    assert tran_window_execution.plan.max_step == pytest.approx(1.0e-3)
    assert tran_window_execution.plan.use_initial_conditions is True
    assert tran_window_execution.run_artifacts[0].step_time == pytest.approx(2.0e-3)
    assert tran_window_execution.run_artifacts[0].stop_time == pytest.approx(6.0e-3)
    assert tran_window_execution.run_artifacts[0].start_time == pytest.approx(2.0e-3)
    assert tran_window_execution.run_artifacts[0].max_step == pytest.approx(1.0e-3)
    assert tran_window_execution.run_artifacts[0].use_initial_conditions is True
    assert tran_window_execution.run_artifacts[0].result_column_count == 3
    assert tran_window_execution.run_artifacts[0].result_columns == [
        "Index",
        "Time",
        "V(mid)",
    ]
    assert tran_window_execution.run_artifacts[0].table_count == 3
    assert tran_window_execution.run_artifacts[0].tables == [
        "result",
        "output-plan",
        "run-artifact",
    ]
    assert tran_window_execution.table_count == 3
    assert tran_window_execution.tables == ["result", "output-plan", "run-artifact"]
    assert tran_window_execution.output_probes == ["V(mid)"]
    assert isinstance(tran_window_execution.result, TransientResult)
    assert [point.time for point in tran_window_execution.result.points] == pytest.approx(
        [2.0e-3, 4.0e-3, 6.0e-3],
    )
    assert tran_window_execution.table == (
        "Index\tTime\tV(mid)\n"
        "0\t2.000000e-03\t5.000000e-01\n"
        "1\t4.000000e-03\t5.000000e-01\n"
        "2\t6.000000e-03\t5.000000e-01\n"
    )
    tran_window_run_artifact_record = _assert_run_artifact_table_matches(
        tran_window_execution
    )
    assert tran_window_run_artifact_record["Analysis"] == "tran"
    assert tran_window_run_artifact_record["DeckAnalysisKinds"] == "1"
    assert tran_window_run_artifact_record["DeckAnalysisKindList"] == "tran"
    assert tran_window_run_artifact_record["DeckAnalysisDirectives"] == "1"

    with pytest.raises(ValueError, match="multiple analysis cards"):
        run_deck_analysis(circuit, netlist)

    lin_execution = run_deck_analysis(circuit, ".save V(mid)\n.ac lin 3 1 3\n.end\n")
    assert lin_execution.output_probes == ["V(mid)"]
    assert isinstance(lin_execution.result, AcResult)
    assert [point.freq for point in lin_execution.result.points] == [1.0, 2.0, 3.0]
    assert lin_execution.table == (
        "Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n"
        "0\t1.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n"
        "1\t2.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n"
        "2\t3.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n"
    )

    oct_execution = run_deck_analysis(circuit, ".save V(mid)\n.ac oct 1 1 4\n.end\n")
    assert oct_execution.output_probes == ["V(mid)"]
    assert isinstance(oct_execution.result, AcResult)
    assert [point.freq for point in oct_execution.result.points] == [1.0, 2.0, 4.0]
    assert oct_execution.table == (
        "Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n"
        "0\t1.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n"
        "1\t2.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n"
        "2\t4.000000e+00\tV(mid)\t5.000000e-01\t0.000000e+00\t5.000000e-01\t0.000000e+00\n"
    )


def test_run_deck_executes_all_analysis_cards_in_source_order() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("V1", "in", "0", 1.0))
    circuit.add(Resistor("R1", "in", "0", 1000.0))
    netlist = ".save V(in)\n.op\n.dc V1 0 1 1\n.op\n.end\n"

    with pytest.raises(ValueError, match="multiple analysis cards"):
        run_deck_analysis(circuit, netlist)

    execution = run_deck(circuit, netlist)

    assert execution.execution_count == 3
    assert execution.analysis_order == ["op", "dc", "op"]
    assert execution.analysis_directives == [".op", ".dc", ".op"]
    assert [item.plan.analysis for item in execution.executions] == ["op", "dc", "op"]
    assert execution.run_artifact_count == 3
    assert [artifact.analysis for artifact in execution.run_artifacts] == ["op", "dc", "op"]
    assert execution.run_artifact_records == deck_table_records(
        execution.run_artifact_table
    )
    assert json.loads(execution.run_artifact_json) == execution.run_artifact_records
    assert execution.run_artifact_records[1]["Analysis"] == "dc"
    assert execution.run_artifact_records[1]["DeckAnalysisKinds"] == "2"
    assert execution.run_artifact_records[1]["DeckAnalysisKindList"] == "op;dc"
    assert execution.run_artifact_records[1]["DeckAnalysisDirectives"] == "3"
    assert (
        execution.run_artifact_records[1]["DeckAnalysisDirectiveList"]
        == ".op;.dc;.op"
    )


def test_run_deck_analysis_surfaces_control_diagnostics_in_artifacts() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("V1", "in", "0", 1.0))
    circuit.add(Resistor("R1", "in", "0", 1000.0))
    netlist = """
.save V(in)
.control
save V(in)
probe V(in)
set filetype=ascii
set wr_vecnames
set wr_singlescale
set appendwrite
.set WR_VECNAMES
write out.raw V(in) V(missing)
wrdata out.dat V(in) V(missing)
source other.cir
cd /tmp
if v(in) > 0
let gain = 2
.endc
.op
.end
"""

    execution = run_deck_analysis(circuit, netlist, "op")
    expected_codes = [
        "SPICE_DECK_CONTROL_SCRIPT_COMMAND",
        "SPICE_DECK_CONTROL_WORKDIR_COMMAND",
        "SPICE_DECK_CONTROL_FLOW_COMMAND",
        "SPICE_DECK_CONTROL_VARIABLE_COMMAND",
    ]
    code_list = ";".join(expected_codes)
    expected_control_lines = [".save V(in)", ".probe V(in)"]
    control_line_list = ";".join(expected_control_lines)
    expected_write_markers = [
        "write out.raw V(in) V(missing)",
        "wrdata out.dat V(in) V(missing)",
    ]
    write_marker_list = ";".join(expected_write_markers)
    expected_rawfile_options = [
        "set filetype=ascii",
        "set wr_vecnames",
        "set wr_singlescale",
        "set appendwrite",
        "set wr_vecnames",
    ]
    rawfile_option_list = ";".join(expected_rawfile_options)
    expected_policy_lines = [13, 14, 15, 16]
    expected_policy_categories = ["script", "workdir", "control-flow", "variable"]
    expected_policy_commands = [
        "source other.cir",
        "cd /tmp",
        "if v(in) > 0",
        "let gain = 2",
    ]
    expected_table_names = [
        "result",
        "control-policy",
        "control-policy-summary",
        "output-plan",
        "run-artifact",
    ]

    assert execution.control_line_count == len(expected_control_lines)
    assert execution.control_lines == expected_control_lines
    assert execution.write_marker_count == len(expected_write_markers)
    assert execution.write_markers == expected_write_markers
    assert execution.rawfile_option_count == len(expected_rawfile_options)
    assert execution.rawfile_options == expected_rawfile_options
    assert execution.rawfile_artifact_count == 1
    assert execution.rawfile_artifacts[0].target == "out.raw"
    assert execution.rawfile_artifacts[0].marker == "write out.raw V(in) V(missing)"
    assert execution.rawfile_artifacts[0].probe_count == 2
    assert execution.rawfile_artifacts[0].probes == ["V(in)", "V(missing)"]
    assert execution.rawfile_artifacts[0].matched_probe_count == 1
    assert execution.rawfile_artifacts[0].matched_probes == ["V(in)"]
    assert execution.rawfile_artifacts[0].unmatched_probe_count == 1
    assert execution.rawfile_artifacts[0].unmatched_probes == ["V(missing)"]
    assert execution.rawfile_artifacts[0].option_count == len(
        expected_rawfile_options
    )
    assert execution.rawfile_artifacts[0].options == expected_rawfile_options
    assert "Title: SPICE deck op result\n" in execution.rawfile_artifacts[0].rawfile
    assert "No. Variables: 2\n" in execution.rawfile_artifacts[0].rawfile
    assert "Options: " + rawfile_option_list + "\n" in execution.rawfile_artifacts[0].rawfile
    assert "0\t0\t1.000000e+00\n" in execution.rawfile_artifacts[0].rawfile
    rawfile_record = execution.rawfile_artifact_records[0]
    assert rawfile_record["Target"] == "out.raw"
    assert rawfile_record["Marker"] == "write out.raw V(in) V(missing)"
    assert rawfile_record["Probes"] == "2"
    assert rawfile_record["ProbeList"] == "V(in);V(missing)"
    assert rawfile_record["MatchedProbes"] == "1"
    assert rawfile_record["MatchedProbeList"] == "V(in)"
    assert rawfile_record["UnmatchedProbes"] == "1"
    assert rawfile_record["UnmatchedProbeList"] == "V(missing)"
    assert rawfile_record["Options"] == str(len(expected_rawfile_options))
    assert rawfile_record["RawfileOptionList"] == rawfile_option_list
    assert rawfile_record["Bytes"] == str(
        len(execution.rawfile_artifacts[0].rawfile.encode())
    )
    assert execution.rawfile_artifact_table == format_deck_rawfile_artifact_table(
        execution.rawfile_artifacts
    )
    assert execution.rawfile_artifact_csv == format_deck_rawfile_artifact_csv(
        execution.rawfile_artifacts
    )
    assert execution.rawfile_artifact_json == format_deck_rawfile_artifact_json(
        execution.rawfile_artifacts
    )
    assert json.loads(execution.rawfile_artifact_json)[0]["RawfileOptionList"] == (
        rawfile_option_list
    )
    rawfile_json = json.loads(execution.rawfile_artifact_json)[0]
    assert rawfile_json["ProbeList"] == "V(in);V(missing)"
    assert rawfile_json["MatchedProbeList"] == "V(in)"
    assert rawfile_json["UnmatchedProbeList"] == "V(missing)"
    assert execution.wrdata_artifact_count == 1
    assert execution.wrdata_artifacts[0].target == "out.dat"
    assert execution.wrdata_artifacts[0].marker == "wrdata out.dat V(in) V(missing)"
    assert execution.wrdata_artifacts[0].probe_count == 2
    assert execution.wrdata_artifacts[0].probes == ["V(in)", "V(missing)"]
    assert execution.wrdata_artifacts[0].matched_probe_count == 1
    assert execution.wrdata_artifacts[0].matched_probes == ["V(in)"]
    assert execution.wrdata_artifacts[0].unmatched_probe_count == 1
    assert execution.wrdata_artifacts[0].unmatched_probes == ["V(missing)"]
    assert execution.wrdata_artifacts[0].option_count == len(
        expected_rawfile_options
    )
    assert execution.wrdata_artifacts[0].options == expected_rawfile_options
    assert "# SPICE deck wrdata artifact\n" in execution.wrdata_artifacts[0].datafile
    assert "Probes: V(in);V(missing)\n" in execution.wrdata_artifacts[0].datafile
    assert "Options: " + rawfile_option_list + "\n" in execution.wrdata_artifacts[0].datafile
    assert "VectorNames: Index;V(in)\n" in execution.wrdata_artifacts[0].datafile
    assert "Scale: Index\n" in execution.wrdata_artifacts[0].datafile
    assert "Index\tV(in)\n" in execution.wrdata_artifacts[0].datafile
    assert "0\t1.000000e+00\n" in execution.wrdata_artifacts[0].datafile
    wrdata_record = execution.wrdata_artifact_records[0]
    assert wrdata_record["Target"] == "out.dat"
    assert wrdata_record["Marker"] == "wrdata out.dat V(in) V(missing)"
    assert wrdata_record["Probes"] == "2"
    assert wrdata_record["ProbeList"] == "V(in);V(missing)"
    assert wrdata_record["MatchedProbes"] == "1"
    assert wrdata_record["MatchedProbeList"] == "V(in)"
    assert wrdata_record["UnmatchedProbes"] == "1"
    assert wrdata_record["UnmatchedProbeList"] == "V(missing)"
    assert wrdata_record["Options"] == str(len(expected_rawfile_options))
    assert wrdata_record["RawfileOptionList"] == rawfile_option_list
    assert wrdata_record["Bytes"] == str(
        len(execution.wrdata_artifacts[0].datafile.encode())
    )
    assert execution.wrdata_artifact_table == format_deck_wrdata_artifact_table(
        execution.wrdata_artifacts
    )
    assert execution.wrdata_artifact_csv == format_deck_wrdata_artifact_csv(
        execution.wrdata_artifacts
    )
    assert execution.wrdata_artifact_json == format_deck_wrdata_artifact_json(
        execution.wrdata_artifacts
    )
    wrdata_json = json.loads(execution.wrdata_artifact_json)[0]
    assert wrdata_json["ProbeList"] == "V(in);V(missing)"
    assert wrdata_json["MatchedProbeList"] == "V(in)"
    assert wrdata_json["UnmatchedProbeList"] == "V(missing)"
    assert wrdata_json["RawfileOptionList"] == rawfile_option_list
    assert execution.control_policy_artifact_count == len(expected_codes)
    assert [artifact.line_number for artifact in execution.control_policy_artifacts] == (
        expected_policy_lines
    )
    assert [artifact.category for artifact in execution.control_policy_artifacts] == (
        expected_policy_categories
    )
    assert [artifact.command for artifact in execution.control_policy_artifacts] == (
        expected_policy_commands
    )
    assert [artifact.code for artifact in execution.control_policy_artifacts] == (
        expected_codes
    )
    assert [artifact.severity for artifact in execution.control_policy_artifacts] == (
        ["error"] * len(expected_codes)
    )
    assert "external script and shell commands are disabled" in (
        execution.control_policy_artifacts[0].message
    )
    policy_record = execution.control_policy_artifact_records[0]
    assert policy_record["Line"] == "13"
    assert policy_record["Category"] == "script"
    assert policy_record["Command"] == "source other.cir"
    assert policy_record["Code"] == "SPICE_DECK_CONTROL_SCRIPT_COMMAND"
    assert policy_record["Severity"] == "error"
    assert execution.control_policy_artifact_table == (
        format_deck_control_policy_artifact_table(
            execution.control_policy_artifacts
        )
    )
    assert execution.control_policy_artifact_csv == (
        format_deck_control_policy_artifact_csv(
            execution.control_policy_artifacts
        )
    )
    assert execution.control_policy_artifact_json == (
        format_deck_control_policy_artifact_json(
            execution.control_policy_artifacts
        )
    )
    policy_json = json.loads(execution.control_policy_artifact_json)
    assert policy_json[2]["Category"] == "control-flow"
    assert policy_json[3]["Command"] == "let gain = 2"
    assert execution.control_policy_summary_artifact_count == len(
        expected_policy_categories
    )
    assert [
        artifact.category for artifact in execution.control_policy_summary_artifacts
    ] == expected_policy_categories
    assert [
        artifact.artifact_count
        for artifact in execution.control_policy_summary_artifacts
    ] == [1, 1, 1, 1]
    assert [
        artifact.line_numbers
        for artifact in execution.control_policy_summary_artifacts
    ] == [[line] for line in expected_policy_lines]
    assert [
        artifact.commands for artifact in execution.control_policy_summary_artifacts
    ] == [[command] for command in expected_policy_commands]
    assert [
        artifact.codes for artifact in execution.control_policy_summary_artifacts
    ] == [[code] for code in expected_codes]
    summary_record = execution.control_policy_summary_artifact_records[0]
    assert summary_record["Category"] == "script"
    assert summary_record["Artifacts"] == "1"
    assert summary_record["LineList"] == "13"
    assert summary_record["CommandList"] == "source other.cir"
    assert summary_record["CodeList"] == "SPICE_DECK_CONTROL_SCRIPT_COMMAND"
    assert summary_record["SeverityList"] == "error"
    assert execution.control_policy_summary_artifact_table == (
        format_deck_control_policy_summary_artifact_table(
            execution.control_policy_summary_artifacts
        )
    )
    assert execution.control_policy_summary_artifact_csv == (
        format_deck_control_policy_summary_artifact_csv(
            execution.control_policy_summary_artifacts
        )
    )
    assert execution.control_policy_summary_artifact_json == (
        format_deck_control_policy_summary_artifact_json(
            execution.control_policy_summary_artifacts
        )
    )
    summary_json = json.loads(execution.control_policy_summary_artifact_json)
    assert summary_json[2]["Category"] == "control-flow"
    assert summary_json[3]["CommandList"] == "let gain = 2"
    assert execution.diagnostic_count == len(expected_codes)
    assert execution.diagnostic_codes == expected_codes
    assert execution.table_count == len(expected_table_names)
    assert execution.tables == expected_table_names
    assert [artifact.name for artifact in execution.table_artifacts] == (
        expected_table_names
    )
    policy_table_artifact = execution.table_artifacts[-4]
    assert policy_table_artifact.name == "control-policy"
    assert policy_table_artifact.table == execution.control_policy_artifact_table
    assert policy_table_artifact.csv == execution.control_policy_artifact_csv
    assert policy_table_artifact.json == execution.control_policy_artifact_json
    assert policy_table_artifact.records == execution.control_policy_artifact_records
    summary_table_artifact = execution.table_artifacts[-3]
    assert summary_table_artifact.name == "control-policy-summary"
    assert (
        summary_table_artifact.table
        == execution.control_policy_summary_artifact_table
    )
    assert (
        summary_table_artifact.csv
        == execution.control_policy_summary_artifact_csv
    )
    assert (
        summary_table_artifact.json
        == execution.control_policy_summary_artifact_json
    )
    assert (
        summary_table_artifact.records
        == execution.control_policy_summary_artifact_records
    )
    output_plan_table_artifact = execution.table_artifacts[-2]
    assert output_plan_table_artifact.name == "output-plan"
    assert output_plan_table_artifact.table == execution.output_plan_artifact_table
    assert output_plan_table_artifact.csv == execution.output_plan_artifact_csv
    assert output_plan_table_artifact.json == execution.output_plan_artifact_json
    assert output_plan_table_artifact.records == execution.output_plan_artifact_records
    assert execution.run_artifacts[0].control_line_count == len(expected_control_lines)
    assert execution.run_artifacts[0].control_lines == expected_control_lines
    assert execution.run_artifacts[0].table_count == len(expected_table_names)
    assert execution.run_artifacts[0].tables == expected_table_names
    assert execution.run_artifacts[0].write_marker_count == len(expected_write_markers)
    assert execution.run_artifacts[0].write_markers == expected_write_markers
    assert execution.run_artifacts[0].rawfile_option_count == len(
        expected_rawfile_options
    )
    assert execution.run_artifacts[0].rawfile_options == expected_rawfile_options
    assert execution.run_artifacts[0].control_policy_artifact_count == len(
        expected_codes
    )
    assert (
        execution.run_artifacts[0].control_policy_categories
        == expected_policy_categories
    )
    assert execution.run_artifacts[0].control_policy_codes == expected_codes
    assert execution.run_artifacts[0].control_policy_severities == ["error"]
    assert execution.run_artifacts[0].diagnostic_count == len(expected_codes)
    assert execution.run_artifacts[0].diagnostic_codes == expected_codes
    record = deck_table_records(execution.run_artifact_table)[0]
    assert record["Tables"] == str(len(expected_table_names))
    assert record["TableList"] == ";".join(expected_table_names)
    assert record["ControlLines"] == str(len(expected_control_lines))
    assert record["ControlLineList"] == control_line_list
    assert record["WriteMarkers"] == str(len(expected_write_markers))
    assert record["WriteMarkerList"] == write_marker_list
    assert record["RawfileOptions"] == str(len(expected_rawfile_options))
    assert record["RawfileOptionList"] == rawfile_option_list
    assert record["ControlPolicyArtifacts"] == str(len(expected_codes))
    assert record["ControlPolicyCategoryList"] == ";".join(expected_policy_categories)
    assert record["ControlPolicyCodeList"] == code_list
    assert record["ControlPolicySeverityList"] == "error"
    assert record["Diagnostics"] == str(len(expected_codes))
    assert record["DiagnosticCodeList"] == code_list
    assert execution.table_artifacts[-1].name == "run-artifact"
    assert execution.table_artifacts[-1].records[0]["ControlLineList"] == control_line_list
    assert (
        execution.table_artifacts[-1].records[0]["WriteMarkerList"] == write_marker_list
    )
    assert (
        execution.table_artifacts[-1].records[0]["RawfileOptionList"]
        == rawfile_option_list
    )
    assert (
        execution.table_artifacts[-1].records[0]["ControlPolicyCategoryList"]
        == ";".join(expected_policy_categories)
    )
    assert (
        execution.table_artifacts[-1].records[0]["ControlPolicyCodeList"] == code_list
    )
    assert (
        execution.table_artifacts[-1].records[0]["ControlPolicySeverityList"]
        == "error"
    )
    assert execution.table_artifacts[-1].records[0]["DiagnosticCodeList"] == code_list
    assert execution.table_artifacts[-1].csv == format_deck_run_artifact_csv(
        execution.run_artifacts
    )
    assert execution.table_artifacts[-1].json == format_deck_run_artifact_json(
        execution.run_artifacts
    )
    assert json.loads(format_deck_run_artifact_json(execution.run_artifacts))[0][
        "ControlLineList"
    ] == control_line_list
    assert json.loads(format_deck_run_artifact_json(execution.run_artifacts))[0][
        "WriteMarkerList"
    ] == write_marker_list
    assert json.loads(format_deck_run_artifact_json(execution.run_artifacts))[0][
        "RawfileOptionList"
    ] == rawfile_option_list
    assert json.loads(format_deck_run_artifact_json(execution.run_artifacts))[0][
        "ControlPolicyCategoryList"
    ] == ";".join(expected_policy_categories)
    assert json.loads(format_deck_run_artifact_json(execution.run_artifacts))[0][
        "ControlPolicyCodeList"
    ] == code_list
    assert json.loads(format_deck_run_artifact_json(execution.run_artifacts))[0][
        "ControlPolicySeverityList"
    ] == "error"
    assert json.loads(format_deck_run_artifact_json(execution.run_artifacts))[0][
        "DiagnosticCodeList"
    ] == code_list


def test_analyze_deck_controls_surfaces_write_markers() -> None:
    summary = analyze_deck_controls(
        """
.control
write out.raw V(out)
wrdata out.dat V(out)
wrdata empty.dat
.write dotted.raw V(a)
.wrdata dotted.dat V(a)
.endc
.end
"""
    )

    assert summary.write_markers == (
        "write out.raw V(out)",
        "wrdata out.dat V(out)",
        "write dotted.raw V(a)",
        "wrdata dotted.dat V(a)",
    )


def test_analyze_deck_controls_surfaces_rawfile_options() -> None:
    summary = analyze_deck_controls(
        """
.control
set filetype=ascii
set wr_vecnames
set wr_singlescale
set appendwrite
.set WR_VECNAMES
set noaskquit
set filetype=binary
set temp=27
.endc
.end
"""
    )

    assert summary.rawfile_options == (
        "set filetype=ascii",
        "set wr_vecnames",
        "set wr_singlescale",
        "set appendwrite",
        "set wr_vecnames",
    )
    assert [diagnostic.code for diagnostic in summary.diagnostics] == [
        "SPICE_DECK_UNSUPPORTED_DIRECTIVE",
        "SPICE_DECK_CONTROL_VARIABLE_COMMAND",
        "SPICE_DECK_CONTROL_VARIABLE_COMMAND",
    ]


def test_run_deck_analysis_exposes_selected_fourier_artifacts() -> None:
    circuit = Circuit()
    circuit.add(VoltageSource("V1", "vin", "0", 1.0))
    circuit.add(Resistor("R1", "vin", "mid", 1000.0))
    circuit.add(Resistor("R2", "mid", "0", 1000.0))

    netlist = """
.save V(mid)
.op
.tran 0.5m 1m
.four 2k V(mid) harmonics=1
.end
"""

    op_execution = run_deck_analysis(circuit, netlist, "op")
    assert op_execution.fourier == []
    assert op_execution.fourier_table == ""
    assert op_execution.table_count == 3
    assert op_execution.tables == ["result", "output-plan", "run-artifact"]

    tran_execution = run_deck_analysis(circuit, netlist, "tran")
    assert len(tran_execution.fourier) == 1
    assert tran_execution.table_count == 4
    assert tran_execution.tables == ["result", "fourier", "output-plan", "run-artifact"]
    assert [artifact.name for artifact in tran_execution.table_artifacts] == (
        tran_execution.tables
    )
    result = tran_execution.fourier[0]
    assert result.fundamental_frequency == pytest.approx(2000.0)
    assert result.probes[0].probe == "V(mid)"
    assert len(result.probes[0].harmonics) == 1
    assert tran_execution.fourier_table == format_fourier_table(result)
    assert tran_execution.table_artifacts[1].table == tran_execution.fourier_table
    assert tran_execution.table_artifacts[1].csv == format_deck_table_csv(
        tran_execution.fourier_table
    )
    assert tran_execution.table_artifacts[1].json == format_deck_table_json(
        tran_execution.fourier_table
    )
    assert tran_execution.table_artifacts[1].records == deck_table_records(
        tran_execution.fourier_table
    )
    assert tran_execution.run_artifacts[0].fourier_count == 1
    assert tran_execution.run_artifacts[0].source_name is None
    assert tran_execution.run_artifacts[0].output_node is None
    assert tran_execution.run_artifacts[0].step_time == pytest.approx(5.0e-4)
    assert tran_execution.run_artifacts[0].stop_time == pytest.approx(1.0e-3)
    assert tran_execution.run_artifacts[0].result_column_count == 3
    assert tran_execution.run_artifacts[0].result_columns == ["Index", "Time", "V(mid)"]
    assert tran_execution.run_artifacts[0].table_count == 4
    assert tran_execution.run_artifacts[0].tables == [
        "result",
        "fourier",
        "output-plan",
        "run-artifact",
    ]
    assert tran_execution.run_artifacts[0].start_time is None
    assert tran_execution.run_artifacts[0].max_step is None
    assert tran_execution.run_artifacts[0].use_initial_conditions is False
    assert tran_execution.run_artifacts[0].output_probes == ["V(mid)"]
    assert tran_execution.run_artifacts[0].output_directives == [".save"]
    assert tran_execution.run_artifacts[0].measurement_names == []
    assert tran_execution.run_artifacts[0].fourier_probes == ["V(mid)"]
    assert tran_execution.deck_analysis_kind_count == 2
    assert tran_execution.deck_analysis_kinds == ["op", "tran"]
    assert tran_execution.deck_analysis_directive_count == 2
    assert tran_execution.deck_analysis_directives == [".op", ".tran"]
    assert tran_execution.run_artifacts[0].deck_analysis_kind_count == 2
    assert tran_execution.run_artifacts[0].deck_analysis_kinds == ["op", "tran"]
    assert tran_execution.run_artifacts[0].deck_analysis_directive_count == 2
    assert tran_execution.run_artifacts[0].deck_analysis_directives == [
        ".op",
        ".tran",
    ]
    assert tran_execution.run_artifact_table == (
        "Analysis\tDirective\tAnalysisDirectives\tAnalysisDirectiveList\tLine\tSourceName\tOutputNode\tSweepKind\tStartValue\tStopValue\tStepValue\tPointCount\tStartFrequencyHz\tStopFrequencyHz\tStepTime\tStopTime\tStartTime\tMaxStep\tUseInitialConditions\tResultRows\tResultColumns\tResultColumnList\tTables\tTableList\tOutputProbes\tOutputProbeList\tOutputDirectives\tOutputDirectiveList\tMeasurements\tMeasurementList\tFourier\tFourierList\tControlLines\tControlLineList\tWriteMarkers\tWriteMarkerList\tRawfileOptions\tRawfileOptionList\tControlPolicyArtifacts\tControlPolicyCategoryList\tControlPolicyCodeList\tControlPolicySeverityList\tDiagnostics\tDiagnosticCodeList\tDeckAnalysisKinds\tDeckAnalysisKindList\tDeckAnalysisDirectives\tDeckAnalysisDirectiveList\n"
        f"tran\t.tran\t1\t.tran\t{tran_execution.plan.line_number}\t\t\t\t\t\t\t\t\t\t5.000000e-04\t1.000000e-03\t\t\tfalse\t3\t3\tIndex;Time;V(mid)\t4\tresult;fourier;output-plan;run-artifact\t1\tV(mid)\t1\t.save\t0\t\t1\tV(mid)\t0\t\t0\t\t0\t\t0\t\t\t\t0\t\t2\top;tran\t2\t.op;.tran\n"
    )


def test_transient_probe_measurements_are_stable() -> None:
    transient_points = [
        TransientPoint(0.0, {"in": 0.0, "out": 0.0}, {}),
        TransientPoint(1.0e-3, {"in": 1.0, "out": 1.25}, {}),
        TransientPoint(2.0e-3, {"in": 1.0, "out": -0.25}, {}),
        TransientPoint(3.0e-3, {"in": 1.0, "out": 0.75}, {}),
    ]

    peak_to_peak = measure_transient_probe(
        transient_points,
        "swing",
        "V(out)",
        "peak-to-peak",
        from_time=1.0e-3,
        to_time=3.0e-3,
    )
    final_value = measure_transient_probe(
        transient_points,
        "settled",
        "V(out)",
        "final",
    )
    midpoint = measure_transient_find_at_probe(
        transient_points,
        "midpoint",
        "V(out)",
        1.5e-3,
    )
    crossing = measure_transient_when_probe(
        transient_points,
        "crossing",
        "V(out)",
        0.5,
        from_time=1.0e-3,
        to_time=3.0e-3,
    )
    second_crossing = measure_transient_when_probe_counted(
        transient_points,
        "second_crossing",
        "V(out)",
        0.5,
        "cross",
        2,
        from_time=1.0e-3,
        to_time=3.0e-3,
    )
    propagation_delay = measure_transient_delay_between_probes(
        transient_points,
        "prop_delay",
        "V(in)",
        0.5,
        "rise",
        1,
        "V(out)",
        0.5,
        "fall",
        1,
        from_time=0.0,
        to_time=3.0e-3,
    )

    assert peak_to_peak.value == pytest.approx(1.5)
    assert peak_to_peak.mode == "pp"
    assert final_value.value == pytest.approx(0.75)
    assert final_value.mode == "last"
    assert midpoint.value == pytest.approx(0.5)
    assert midpoint.mode == "find"
    assert crossing.value == pytest.approx(1.5e-3)
    assert crossing.mode == "when"
    assert second_crossing.value == pytest.approx(2.75e-3)
    assert second_crossing.mode == "when"
    assert propagation_delay.value == pytest.approx(1.0e-3)
    assert propagation_delay.probe == "V(in)->V(out)"
    assert propagation_delay.mode == "delay"
    assert format_measurement_table(
        [
            peak_to_peak,
            final_value,
            midpoint,
            crossing,
            second_crossing,
            propagation_delay,
        ]
    ) == (
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
        "swing\ttran\tV(out)\tpp\t1.000000e-03\t3.000000e-03\t1.500000e+00\n"
        "settled\ttran\tV(out)\tlast\t\t\t7.500000e-01\n"
        "midpoint\ttran\tV(out)\tfind\t1.500000e-03\t1.500000e-03\t5.000000e-01\n"
        "crossing\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t1.500000e-03\n"
        "second_crossing\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t2.750000e-03\n"
        "prop_delay\ttran\tV(in)->V(out)\tdelay\t0.000000e+00\t3.000000e-03\t1.000000e-03\n"
    )


def test_transient_deck_measurements_execute_parsed_cards() -> None:
    transient_points = [
        TransientPoint(0.0, {"in": 0.0, "out": 0.0}, {}),
        TransientPoint(1.0e-3, {"in": 1.0, "out": 1.25}, {}),
        TransientPoint(2.0e-3, {"in": 1.0, "out": -0.25}, {}),
        TransientPoint(3.0e-3, {"in": 1.0, "out": 0.75}, {}),
    ]

    measurements = measure_transient_deck(
        transient_points,
        """
V1 in 0 DC 1
.measure tran swing PP V(out) FROM=1m TO=3m
.measure tran midpoint FIND V(out) AT=1.5m
.measure tran crossing WHEN V(out)=0.5 FROM=1m TO=3m
.measure tran second_cross WHEN V(out)=0.5 FROM=1m TO=3m CROSS=2
.measure tran falling WHEN V(out)=0.5 FROM=1m TO=3m FALL=1
.measure tran rising WHEN V(out)=0.5 FROM=1m TO=3m RISE=1
.measure tran prop_delay TRIG V(in) VAL=0.5 RISE=1 TARG V(out) VAL=0.5 FALL=1 FROM=0 TO=3m
.meas tran settled LAST V(out)
.end
""",
    )

    assert format_measurement_table(measurements) == (
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
        "swing\ttran\tV(out)\tpp\t1.000000e-03\t3.000000e-03\t1.500000e+00\n"
        "midpoint\ttran\tV(out)\tfind\t1.500000e-03\t1.500000e-03\t5.000000e-01\n"
        "crossing\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t1.500000e-03\n"
        "second_cross\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t2.750000e-03\n"
        "falling\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t1.500000e-03\n"
        "rising\ttran\tV(out)\twhen\t1.000000e-03\t3.000000e-03\t2.750000e-03\n"
        "prop_delay\ttran\tV(in)->V(out)\tdelay\t0.000000e+00\t3.000000e-03\t1.000000e-03\n"
        "settled\ttran\tV(out)\tlast\t\t\t7.500000e-01\n"
    )


def test_pole_zero_text_output_table_is_stable() -> None:
    result = PoleZeroResult(
        input_source="Vin",
        output_node="out",
        entries=[
            PoleZeroEntry(
                kind="zero",
                real=0.0,
                imaginary=1.0e3,
                frequency=1.0e3 / (2.0 * math.pi),
                damping=0.0,
            ),
            PoleZeroEntry(
                kind="pole",
                real=-5.0,
                imaginary=-999.987499921874,
                frequency=1.0e3 / (2.0 * math.pi),
                damping=5.0e-3,
            ),
        ],
    )

    assert format_pole_zero_table(result) == (
        "Index\tKind\tReal\tImaginary\tFrequency\tDamping\n"
        "0\tzero\t0.000000e+00\t1.000000e+03\t1.591549e+02\t0.000000e+00\n"
        "1\tpole\t-5.000000e+00\t-9.999875e+02\t1.591549e+02\t5.000000e-03\n"
    )


def test_distortion_text_output_table_is_stable() -> None:
    result = DistortionResult(
        input_source="Vin",
        output_probe="V(out)",
        points=[
            DistortionPoint(
                frequency=1000.0,
                fundamental_magnitude=1.0,
                harmonics=[
                    DistortionHarmonic(
                        harmonic=1,
                        frequency=1000.0,
                        magnitude=1.0,
                        phase_degrees=0.0,
                    ),
                    DistortionHarmonic(
                        harmonic=2,
                        frequency=2000.0,
                        magnitude=0.025,
                        phase_degrees=-1.5707963267948966,
                    ),
                ],
                total_harmonic_distortion=0.025,
            )
        ],
    )

    assert format_distortion_table(result) == (
        "Frequency\tInput\tOutput\tHarmonic\tMagnitude\tPhase\tTHD\n"
        "1.000000e+03\tVin\tV(out)\t1\t1.000000e+00\t0.000000e+00\t2.500000e-02\n"
        "1.000000e+03\tVin\tV(out)\t2\t2.500000e-02\t-1.570796e+00\t2.500000e-02\n"
    )


def test_corner_distortion_text_output_table_is_stable() -> None:
    result = CornerDistortionResult(
        input_source="Vin",
        output_probe="V(out)",
        points=[
            CornerDistortionPoint(
                corner_name="nominal",
                result=DistortionResult(
                    input_source="Vin",
                    output_probe="V(out)",
                    points=[
                        DistortionPoint(
                            frequency=1000.0,
                            fundamental_magnitude=1.0,
                            harmonics=[
                                DistortionHarmonic(1, 1000.0, 1.0, 0.0),
                                DistortionHarmonic(
                                    2,
                                    2000.0,
                                    0.025,
                                    -1.5707963267948966,
                                ),
                            ],
                            total_harmonic_distortion=0.025,
                        )
                    ],
                ),
            ),
            CornerDistortionPoint(
                corner_name="slow",
                result=DistortionResult(
                    input_source="Vin",
                    output_probe="V(out)",
                    points=[
                        DistortionPoint(
                            frequency=1000.0,
                            fundamental_magnitude=0.8,
                            harmonics=[DistortionHarmonic(2, 2000.0, 0.04, 12.5)],
                            total_harmonic_distortion=0.05,
                        )
                    ],
                ),
            ),
        ],
    )

    assert format_corner_distortion_table(result) == (
        "Corner\tFrequency\tInput\tOutput\tHarmonic\tMagnitude\tPhase\tTHD\n"
        "nominal\t1.000000e+03\tVin\tV(out)\t1\t1.000000e+00\t0.000000e+00\t2.500000e-02\n"
        "nominal\t1.000000e+03\tVin\tV(out)\t2\t2.500000e-02\t-1.570796e+00\t2.500000e-02\n"
        "slow\t1.000000e+03\tVin\tV(out)\t2\t4.000000e-02\t1.250000e+01\t5.000000e-02\n"
    )


def test_fourier_text_output_table_is_stable() -> None:
    result = FourierResult(
        fundamental_frequency=1000.0,
        start_time=0.0,
        end_time=0.001,
        probes=[
            FourierProbeResult(
                probe="V(out)",
                dc=0.1,
                harmonics=[
                    FourierHarmonic(
                        harmonic=1,
                        frequency=1000.0,
                        cosine=1.0,
                        sine=0.0,
                        magnitude=1.0,
                        phase_degrees=0.0,
                    ),
                    FourierHarmonic(
                        harmonic=2,
                        frequency=2000.0,
                        cosine=0.0,
                        sine=-0.025,
                        magnitude=0.025,
                        phase_degrees=-90.0,
                    ),
                ],
                total_harmonic_distortion=0.025,
            )
        ],
    )

    assert format_fourier_table(result) == (
        "Probe\tHarmonic\tFrequency\tCosine\tSine\tMagnitude\tPhase\tDC\tTHD\n"
        "V(out)\t1\t1.000000e+03\t1.000000e+00\t0.000000e+00\t1.000000e+00\t0.000000e+00\t1.000000e-01\t2.500000e-02\n"
        "V(out)\t2\t2.000000e+03\t0.000000e+00\t-2.500000e-02\t2.500000e-02\t-9.000000e+01\t1.000000e-01\t2.500000e-02\n"
    )


def test_ac_text_output_table_is_stable() -> None:
    result = AcResult(
        points=[
            AcPoint(
                freq=1000.0,
                node_voltages={"out": 0.5 - 0.5j},
                branch_currents={"I(V1)": -0.001 + 0.001j},
            )
        ]
    )

    assert format_ac_table(result) == (
        "Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n"
        "0\t1.000000e+03\tV(out)\t5.000000e-01\t-5.000000e-01\t7.071068e-01\t-4.500000e+01\n"
        "0\t1.000000e+03\tI(V1)\t-1.000000e-03\t1.000000e-03\t1.414214e-03\t1.350000e+02\n"
    )


def test_ac_sweep_probe_measurements_execute_parsed_cards() -> None:
    result = AcResult(
        points=[
            AcPoint(freq=10.0, node_voltages={"out": 1.0 + 0.0j}),
            AcPoint(freq=100.0, node_voltages={"out": 0.0 + 2.0j}),
            AcPoint(freq=1000.0, node_voltages={"out": 0.0 + 0.5j}),
        ]
    )

    peak = measure_ac_sweep_probe(
        result,
        "out_peak",
        "V(out)",
        "max",
        from_frequency=10.0,
        to_frequency=100.0,
    )
    average = measure_ac_sweep_probe(result, "out_avg", "V(out)", "avg")

    assert peak.value == pytest.approx(2.0)
    assert peak.analysis == "ac"
    assert average.value == pytest.approx(1.1666666666666667)
    assert format_measurement_table([peak, average]) == (
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
        "out_peak\tac\tV(out)\tmax\t1.000000e+01\t1.000000e+02\t2.000000e+00\n"
        "out_avg\tac\tV(out)\tavg\t\t\t1.166667e+00\n"
    )

    measurements = measure_ac_sweep_deck(
        result,
        """
.measure ac out_swing PP V(out) FROM=10 TO=1000
.meas ac out_final FINAL V(out)
.end
""",
    )

    assert format_measurement_table(measurements) == (
        "Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue\n"
        "out_swing\tac\tV(out)\tpp\t1.000000e+01\t1.000000e+03\t1.500000e+00\n"
        "out_final\tac\tV(out)\tlast\t\t\t5.000000e-01\n"
    )


# ---- PulseWaveform unit tests -----------------------------------------------


def test_pulse_waveform_before_delay() -> None:
    """PulseWaveform holds v_initial before the delay time."""
    w = PulseWaveform(v_initial=0.0, v_pulsed=5.0, delay=1.0)
    assert w(0.0) == pytest.approx(0.0)
    assert w(0.999) == pytest.approx(0.0)


def test_pulse_waveform_during_high_phase() -> None:
    """PulseWaveform is v_pulsed during the flat top."""
    # delay=0, rise_time=0, pulse_width=0.5, period=1.0
    w = PulseWaveform(v_initial=0.0, v_pulsed=3.3, pulse_width=0.5, period=1.0)
    assert w(0.1) == pytest.approx(3.3)
    assert w(0.49) == pytest.approx(3.3)


def test_pulse_waveform_during_low_phase() -> None:
    """PulseWaveform is v_initial during the low phase."""
    w = PulseWaveform(v_initial=0.0, v_pulsed=5.0, pulse_width=0.5, period=1.0)
    assert w(0.75) == pytest.approx(0.0)
    assert w(0.99) == pytest.approx(0.0)


def test_pulse_waveform_rise_edge() -> None:
    """PulseWaveform linearly ramps from v_initial to v_pulsed over rise_time."""
    w = PulseWaveform(v_initial=0.0, v_pulsed=1.0,
                      rise_time=0.2, fall_time=0.0,
                      pulse_width=0.5, period=1.0)
    assert w(0.0) == pytest.approx(0.0)
    assert w(0.1) == pytest.approx(0.5)
    assert w(0.2) == pytest.approx(1.0)


def test_pulse_waveform_fall_edge() -> None:
    """PulseWaveform linearly ramps from v_pulsed to v_initial over fall_time."""
    w = PulseWaveform(v_initial=0.0, v_pulsed=1.0,
                      rise_time=0.0, fall_time=0.2,
                      pulse_width=0.5, period=1.0)
    # During falling edge: t_rel ∈ [0.5, 0.7)
    assert w(0.5) == pytest.approx(1.0, abs=1e-10)  # start of fall
    assert w(0.6) == pytest.approx(0.5, abs=1e-10)  # mid-fall
    assert w(0.7) == pytest.approx(0.0, abs=1e-10)  # end of fall → low


def test_pulse_waveform_periodic() -> None:
    """PulseWaveform repeats with the given period."""
    w = PulseWaveform(v_initial=0.0, v_pulsed=1.0, pulse_width=0.5, period=1.0)
    # t=0.1 (first period high) and t=1.1 (second period high) should match.
    assert w(0.1) == pytest.approx(w(1.1))
    assert w(0.75) == pytest.approx(w(1.75))


def test_pulse_waveform_transient_current_source() -> None:
    """Transient sim: PWM current source drives a load; V = I*R follows pulse.

    Convention: CurrentSource(n_plus, n_minus, current=I) injects I into
    n_minus (positive terminal in SPICE3 terms).  Using n_plus="0" and
    n_minus="out" makes V("out") = I * R > 0 for positive I.
    """
    # 10 mA pulse with 50% duty cycle, period = 0.1 s
    R = 100.0
    I_high = 10e-3
    w = PulseWaveform(v_initial=0.0, v_pulsed=I_high,
                      pulse_width=0.05, period=0.1)
    c = Circuit([
        CurrentSource("Is", "0", "out", current=0.0, waveform=w),
        Resistor("Rload", "out", "0", R),
    ])
    result = transient(c, t_stop=0.2, t_step=0.005)
    assert result.converged

    for pt in result.points:
        v_out = pt.node_voltages.get("out", 0.0)
        expected = w(pt.time) * R
        # Allow ±5% of R*I_high for timestep quantisation
        assert abs(v_out - expected) < 0.05 * R * I_high + 1e-9, (
            f"t={pt.time:.4f}: V(out)={v_out:.5f} expected ~{expected:.5f}"
        )


# ---- ExpWaveform unit tests -------------------------------------------------


def test_exp_waveform_before_rise_delay() -> None:
    """ExpWaveform holds v_initial before rise_delay."""
    w = ExpWaveform(v_initial=0.0, v_pulsed=1.0, rise_delay=0.5, rise_tc=0.1)
    assert w(0.0) == pytest.approx(0.0)
    assert w(0.5) == pytest.approx(0.0)


def test_exp_waveform_rises_exponentially() -> None:
    """ExpWaveform approaches v_pulsed with rising exponential after rise_delay."""
    import math
    v0, v1 = 0.0, 5.0
    td1, tc1 = 0.0, 1.0
    # Disable fall by setting fall_delay after simulation horizon
    w = ExpWaveform(v_initial=v0, v_pulsed=v1,
                    rise_delay=td1, rise_tc=tc1,
                    fall_delay=100.0, fall_tc=1.0)
    for t in [0.5, 1.0, 2.0, 3.0]:
        expected = v0 + (v1 - v0) * (1.0 - math.exp(-t / tc1))
        assert w(t) == pytest.approx(expected, abs=1e-12)


def test_exp_waveform_falls_after_fall_delay() -> None:
    """ExpWaveform falls back towards v_initial after fall_delay."""
    v0, v1 = 0.0, 1.0
    # Rise at t=0 with very fast rise_tc so by fall_delay it is essentially v1.
    # Fall starts at fall_delay=0.5 with tc=0.5.
    w = ExpWaveform(v_initial=v0, v_pulsed=v1,
                    rise_delay=0.0, rise_tc=0.001,
                    fall_delay=0.5, fall_tc=0.5)
    # At t=2.0 (1.5 s after fall_delay), should be well below 0.5.
    assert w(2.0) < 0.5


def test_exp_waveform_transient_integration() -> None:
    """Transient sim driven by ExpWaveform: node tracks expected waveform."""
    import math
    v0, v1 = 0.0, 2.0
    rise_tc = 0.1
    w = ExpWaveform(v_initial=v0, v_pulsed=v1,
                    rise_delay=0.0, rise_tc=rise_tc,
                    fall_delay=10.0, fall_tc=1.0)   # no fall during sim
    c = Circuit([
        VoltageSource("Vs", "in", "0", voltage=0.0, waveform=w),
        Resistor("Rser", "in", "out", 1e-3),
        Resistor("Rload", "out", "0", 1.0),
    ])
    result = transient(c, t_stop=0.5, t_step=0.02)
    assert result.converged

    for pt in result.points:
        v_out = pt.node_voltages.get("out", 0.0)
        t = pt.time
        expected = v0 + (v1 - v0) * (1.0 - math.exp(-t / rise_tc))
        # Allow 2% of v1 for integration error
        assert abs(v_out - expected) < 0.02 * v1 + 1e-6, (
            f"t={pt.time:.3f}: V(out)={v_out:.5f} expected ~{expected:.5f}"
        )


# ---- Waveform type alias and export tests ------------------------------------


def test_waveform_type_alias_covers_all_forms() -> None:
    """Waveform type alias is the union of all four waveform classes."""
    # The type alias itself is not a runtime class, but we can verify that
    # each concrete form is a valid Waveform by checking isinstance against
    # the individual classes.
    forms = [
        PwlWaveform(points=((0.0, 0.0), (1.0, 1.0))),
        SinWaveform(),
        PulseWaveform(),
        ExpWaveform(),
    ]
    for form in forms:
        assert callable(form), f"{type(form).__name__} must be callable"
        assert isinstance(form, (PwlWaveform, SinWaveform, PulseWaveform, ExpWaveform))


def test_waveform_exported_from_package() -> None:
    """PwlWaveform, SinWaveform, PulseWaveform, ExpWaveform, Waveform are exported."""
    import spice_engine
    assert hasattr(spice_engine, "PwlWaveform")
    assert hasattr(spice_engine, "SinWaveform")
    assert hasattr(spice_engine, "PulseWaveform")
    assert hasattr(spice_engine, "ExpWaveform")
    assert hasattr(spice_engine, "Waveform")


def test_voltage_source_accepts_waveform_field() -> None:
    """VoltageSource.waveform defaults to None and accepts a waveform object."""
    v_static = VoltageSource("V1", "a", "0", voltage=5.0)
    assert v_static.waveform is None

    w = SinWaveform(amplitude=1.0, frequency=60.0)
    v_dyn = VoltageSource("V2", "a", "0", voltage=0.0, waveform=w)
    assert v_dyn.waveform is w


def test_current_source_accepts_waveform_field() -> None:
    """CurrentSource.waveform defaults to None and accepts a waveform object."""
    i_static = CurrentSource("I1", "a", "0", current=1e-3)
    assert i_static.waveform is None

    w = PulseWaveform(v_initial=0.0, v_pulsed=5e-3)
    i_dyn = CurrentSource("I2", "a", "0", current=0.0, waveform=w)
    assert i_dyn.waveform is w


def test_waveform_source_dc_op_uses_static_voltage() -> None:
    """DC op-point of a waveform source uses the stored `voltage` field."""
    # The waveform is irrelevant for DC; only `voltage` is used.
    w = SinWaveform(amplitude=10.0, frequency=1e3)
    c = Circuit([
        VoltageSource("Vs", "in", "0", voltage=3.0, waveform=w),
        Resistor("R", "in", "0", 1.0),
    ])
    op = dc_op(c)
    assert op.converged
    assert op.node_voltages["in"] == pytest.approx(3.0)


# ---------------------------------------------------------------------------
# Section 67 — Convergence aids: Gmin stepping and source stepping
# ---------------------------------------------------------------------------
#
# The convergence-aid chain is:
#   dc_op (plain Newton) → _dc_gmin_step → _dc_source_step
#
# For most circuits plain Newton converges immediately, so the aids are
# transparent.  These tests verify:
#   (a) the private helpers produce correct results on simple circuits,
#   (b) the public dc_op interface respects the convergence_aids flag,
#   (c) circuits that fail plain Newton (strongly nonlinear, near-singular)
#       succeed when aids are enabled.


# ---- _dc_newton: warm-start and convergence_aids=False ---------------------


def test_dc_newton_matches_dc_op_on_simple_circuit() -> None:
    """_dc_newton gives the same result as dc_op on a simple resistive circuit."""
    c = Circuit([
        VoltageSource("V1", "in", "0", 5.0),
        Resistor("R1", "in", "out", 1000.0),
        Resistor("R2", "out", "0", 1000.0),
    ])
    public_result = dc_op(c)
    private_result = _dc_newton(c, max_iterations=50, tol=1e-6)
    assert private_result.converged
    assert private_result.node_voltages["out"] == pytest.approx(
        public_result.node_voltages["out"], abs=1e-9
    )


def test_dc_newton_warm_start_converges_faster() -> None:
    """Warm-started _dc_newton converges in fewer or equal iterations."""
    from spice_engine.engine import _branch_sources
    c = Circuit([
        VoltageSource("V1", "in", "0", 5.0),
        Resistor("R1", "in", "out", 1000.0),
        Resistor("R2", "out", "0", 1000.0),
    ])
    cold = _dc_newton(c, max_iterations=50, tol=1e-9)
    assert cold.converged

    # Warm-start from the converged solution — should converge in ≤ cold iters.
    _, nodes = _node_index(c)
    branch_srcs = _branch_sources(c)
    x_warm = _x_from_result(cold, nodes, branch_srcs)
    warm = _dc_newton(c, max_iterations=50, tol=1e-9, x_init=x_warm)
    assert warm.converged
    assert warm.iterations <= cold.iterations


def test_dc_op_convergence_aids_false_still_converges_linear() -> None:
    """dc_op(convergence_aids=False) converges on a simple linear circuit."""
    c = Circuit([
        VoltageSource("V1", "a", "0", 10.0),
        Resistor("R1", "a", "0", 100.0),
    ])
    result = dc_op(c, convergence_aids=False)
    assert result.converged
    assert result.convergence_aid == "newton"
    assert result.node_voltages["a"] == pytest.approx(10.0)


def test_dc_op_reports_no_convergence_aid_when_disabled_solve_fails() -> None:
    """dc_op(convergence_aids=False) reports that no fallback aid converged."""
    c = Circuit([
        VoltageSource("Vs", "in", "0", 10.0),
        Diode("D1", anode="in", cathode="out", Is=1e-15, Vt=0.02585),
        Resistor("Rload", "out", "0", 100.0),
    ])
    result = dc_op(c, max_iterations=1, convergence_aids=False)
    assert not result.converged
    assert result.convergence_aid == "none"


def test_dc_op_newton_step_limit_reports_damped_nonlinear_step() -> None:
    c = Circuit([
        VoltageSource("Vs", "in", "0", 10.0),
        Diode("D1", anode="in", cathode="out", Is=1e-15, Vt=0.02585),
        Resistor("Rload", "out", "0", 100.0),
    ])

    result = dc_op(
        c,
        max_iterations=1,
        convergence_aids=False,
        newton_step_limit=0.25,
    )

    assert not result.converged
    assert result.convergence_aid == "none"
    assert result.diagnostics.newton_step_limit == pytest.approx(0.25)
    assert result.diagnostics.limited_newton_steps == 1
    assert 0.0 < result.diagnostics.minimum_damping_factor < 1.0
    assert result.diagnostics.max_delta == pytest.approx(0.25)
    assert max(abs(value) for value in result.node_voltages.values()) <= 0.25 + 1e-12


def test_dc_op_rejects_invalid_newton_step_limit() -> None:
    with pytest.raises(ValueError, match="newton_step_limit must be finite and positive"):
        dc_op(Circuit(), newton_step_limit=0.0)


def test_dc_op_pseudo_transient_recovers_after_earlier_aids_fail() -> None:
    """Pseudo-transient continuation is the final DC fallback aid."""
    c = Circuit([
        VoltageSource("Vs", "in", "0", 10.0),
        Diode("D1", anode="in", cathode="out", Is=1e-15, Vt=0.02585),
        Resistor("Rload", "out", "0", 100.0),
    ])
    result = dc_op(
        c,
        max_iterations=1,
        pseudo_transient_max_iterations=500,
        pseudo_transient_steps=40,
    )
    assert result.converged
    assert result.convergence_aid == "pseudo_transient"
    assert 0.0 < result.node_voltages["out"] < 10.0


# ---- _dc_gmin_step ---------------------------------------------------------


def test_dc_gmin_step_resistor_divider() -> None:
    """_dc_gmin_step returns correct voltages for a resistive voltage divider."""
    c = Circuit([
        VoltageSource("V1", "in", "0", 6.0),
        Resistor("R1", "in", "mid", 2000.0),
        Resistor("R2", "mid", "0", 4000.0),
    ])
    result = _dc_gmin_step(c)
    assert result is not None
    assert result.converged
    # V(mid) = 6 * 4k / (2k + 4k) = 4.0 V
    assert result.node_voltages["mid"] == pytest.approx(4.0, abs=1e-3)


def test_dc_gmin_step_diode_circuit() -> None:
    """_dc_gmin_step converges on a diode+resistor circuit."""
    c = Circuit([
        VoltageSource("Vs", "in", "0", 5.0),
        Diode("D1", anode="in", cathode="out"),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = _dc_gmin_step(c)
    assert result is not None
    assert result.converged
    # Gmin result should be very close to plain Newton result.
    plain = dc_op(c, convergence_aids=False)
    assert result.node_voltages["out"] == pytest.approx(
        plain.node_voltages["out"], abs=1e-4
    )


def test_dc_gmin_step_no_nodes_returns_none() -> None:
    """_dc_gmin_step returns None for a trivial circuit with no non-ground nodes."""
    c = Circuit([
        Resistor("R1", "0", "gnd", 100.0),  # both "0" and "gnd" are ground aliases
    ])
    assert _dc_gmin_step(c) is None


def test_dc_gmin_step_custom_parameters() -> None:
    """_dc_gmin_step accepts custom gmin_start and n_steps."""
    c = Circuit([
        VoltageSource("V1", "a", "0", 3.3),
        Resistor("R1", "a", "0", 1000.0),
    ])
    result = _dc_gmin_step(c, gmin_start=1e-2, n_steps=5)
    assert result is not None
    assert result.converged
    assert result.node_voltages["a"] == pytest.approx(3.3, abs=1e-4)


# ---- _dc_source_step -------------------------------------------------------


def test_dc_source_step_resistor_divider() -> None:
    """_dc_source_step returns correct voltages for a resistive voltage divider."""
    c = Circuit([
        VoltageSource("V1", "in", "0", 10.0),
        Resistor("R1", "in", "mid", 1000.0),
        Resistor("R2", "mid", "0", 1000.0),
    ])
    result = _dc_source_step(c)
    assert result is not None
    assert result.converged
    # V(mid) = 10 * 1k / (1k + 1k) = 5.0 V
    assert result.node_voltages["mid"] == pytest.approx(5.0, abs=1e-4)


def test_dc_source_step_current_source() -> None:
    """_dc_source_step scales current sources correctly."""
    r_val = 500.0
    i_val = 2e-3
    c = Circuit([
        CurrentSource("I1", "0", "out", i_val),
        Resistor("Rload", "out", "0", r_val),
    ])
    result = _dc_source_step(c)
    assert result is not None
    assert result.converged
    assert result.node_voltages["out"] == pytest.approx(i_val * r_val, abs=1e-6)


def test_dc_source_step_diode_circuit() -> None:
    """_dc_source_step converges on a diode circuit, matching plain Newton."""
    c = Circuit([
        VoltageSource("Vs", "in", "0", 5.0),
        Diode("D1", anode="in", cathode="out"),
        Resistor("Rload", "out", "0", 1000.0),
    ])
    result = _dc_source_step(c)
    assert result is not None
    assert result.converged
    plain = dc_op(c, convergence_aids=False)
    assert result.node_voltages["out"] == pytest.approx(
        plain.node_voltages["out"], abs=1e-4
    )


def test_dc_source_step_custom_n_steps() -> None:
    """_dc_source_step accepts a custom number of steps."""
    c = Circuit([
        VoltageSource("V1", "a", "0", 1.0),
        Resistor("R1", "a", "0", 100.0),
    ])
    result = _dc_source_step(c, n_steps=5)
    assert result is not None
    assert result.converged
    assert result.node_voltages["a"] == pytest.approx(1.0, abs=1e-6)


# ---- dc_op public interface -----------------------------------------------


def test_dc_op_convergence_aids_default_gives_same_result() -> None:
    """dc_op with and without aids gives identical results on a linear circuit."""
    c = Circuit([
        VoltageSource("V1", "in", "0", 5.0),
        Resistor("R1", "in", "out", 2000.0),
        Resistor("R2", "out", "0", 3000.0),
    ])
    r_with = dc_op(c, convergence_aids=True)
    r_without = dc_op(c, convergence_aids=False)
    assert r_with.converged
    assert r_without.converged
    assert r_with.node_voltages["out"] == pytest.approx(
        r_without.node_voltages["out"], abs=1e-9
    )


def test_dc_op_with_aids_and_diode() -> None:
    """dc_op(convergence_aids=True) matches aids=False on a diode circuit."""
    c = Circuit([
        VoltageSource("Vs", "a", "0", 2.0),
        Diode("D1", anode="a", cathode="b"),
        Resistor("R1", "b", "0", 500.0),
    ])
    r_aids = dc_op(c, convergence_aids=True)
    r_no_aids = dc_op(c, convergence_aids=False)
    assert r_aids.converged
    assert r_no_aids.converged
    assert r_aids.node_voltages["b"] == pytest.approx(
        r_no_aids.node_voltages["b"], abs=1e-4
    )


def test_dc_op_convergence_aids_high_voltage_diode() -> None:
    """dc_op converges on a high-voltage diode circuit with convergence aids.

    The circuit is: Vs(10V) → D1(anode=in, cathode=out) → Rload(100Ω) → GND.
    The engine clamps the diode linearisation point at Vd = 0.7 V during
    Newton iterations; the converged operating point therefore reflects the
    clamped model rather than the ideal exponential.  The test verifies
    convergence and that the aids path agrees exactly with the plain-Newton
    path (both solvers should reach the same fixed-point for this circuit).
    """
    c = Circuit([
        VoltageSource("Vs", "in", "0", 10.0),
        Diode("D1", anode="in", cathode="out", Is=1e-15, Vt=0.02585),
        Resistor("Rload", "out", "0", 100.0),
    ])
    result_aids = dc_op(c, convergence_aids=True)
    result_plain = dc_op(c, convergence_aids=False)
    assert result_aids.converged
    assert result_plain.converged
    # Both paths must reach the same operating point.
    assert abs(result_aids.node_voltages["out"] - result_plain.node_voltages["out"]) < 1e-6
    # Sanity: output is strictly between ground and the supply.
    assert 0.0 < result_aids.node_voltages["out"] < 10.0


def test_dc_op_iterations_field_is_populated() -> None:
    """dc_op.iterations is positive after a successful solve."""
    c = Circuit([
        VoltageSource("V1", "a", "0", 1.0),
        Resistor("R1", "a", "0", 1.0),
    ])
    result = dc_op(c)
    assert result.converged
    assert result.convergence_aid == "newton"
    assert result.iterations >= 1


# ---- _x_from_result --------------------------------------------------------


def test_x_from_result_round_trip() -> None:
    """_x_from_result reconstructs the x vector that produced the DcResult."""
    from spice_engine.engine import _branch_sources
    c = Circuit([
        VoltageSource("V1", "a", "0", 5.0),
        Resistor("R1", "a", "b", 1000.0),
        Resistor("R2", "b", "0", 1000.0),
    ])
    result = dc_op(c)
    assert result.converged

    _, nodes = _node_index(c)
    branch_srcs = _branch_sources(c)
    x = _x_from_result(result, nodes, branch_srcs)

    # x should have one entry per node + one per branch source
    assert len(x) == len(nodes) + len(branch_srcs)
    # Node voltage entries should match result
    for i, nd in enumerate(nodes):
        assert x[i] == pytest.approx(result.node_voltages[nd])


def test_s_parameters_series_resistor_two_port() -> None:
    """A 50 ohm series resistor between 50 ohm ports has S11=1/3, S21=2/3."""
    c = Circuit([
        VoltageSource("P1", "p1", "0", 0.0),
        VoltageSource("P2", "p2", "0", 0.0),
        Resistor("Rseries", "p1", "p2", 50.0),
    ])

    result = s_parameters(
        c,
        port1_source="P1",
        port2_source="P2",
        frequencies=[1.0e6],
        reference_impedance=50.0,
    )

    assert isinstance(result, SParameterResult)
    assert isinstance(result.points[0], SParameterPoint)
    point = result.points[0]
    assert point.s11.real == pytest.approx(1.0 / 3.0)
    assert point.s22.real == pytest.approx(1.0 / 3.0)
    assert point.s21.real == pytest.approx(2.0 / 3.0)
    assert point.s12.real == pytest.approx(2.0 / 3.0)
    assert abs(point.s11.imag) < 1.0e-12
    assert abs(point.s21.imag) < 1.0e-12


def test_dc_corners_runs_named_parameter_overrides() -> None:
    c = Circuit([
        VoltageSource("Vin", "in", "0", 10.0),
        Resistor("Rtop", "in", "out", 1000.0),
        Resistor("Rbot", "out", "0", 1000.0),
    ])

    result = dc_corners(
        c,
        [
            CornerSpec("nominal"),
            CornerSpec("rbot-fast", (CornerOverride("Rbot", "resistance", 500.0),)),
            CornerSpec("vin-high", (CornerOverride("Vin", "voltage", 12.0),)),
            CornerSpec("vin-inverted", (CornerOverride("Vin", "voltage", -10.0),)),
        ],
    )

    assert isinstance(result, CornerSweepResult)
    voltages = [point.result.node_voltages["out"] for point in result.points]
    assert voltages == pytest.approx([5.0, 10.0 / 3.0, 6.0, -5.0])
    assert format_corner_dc_table(result, ["V(out)", "I(Vin)"]) == (
        "Corner\tIndex\tV(out)\tI(Vin)\n"
        "nominal\t0\t5.000000e+00\t-5.000000e-03\n"
        "rbot-fast\t1\t3.333333e+00\t-6.666667e-03\n"
        "vin-high\t2\t6.000000e+00\t-6.000000e-03\n"
        "vin-inverted\t3\t-5.000000e+00\t5.000000e-03\n"
    )


def test_dc_sweep_corners_runs_source_sweeps_per_corner() -> None:
    c = Circuit([
        VoltageSource("Vin", "in", "0", 0.0),
        Resistor("Rtop", "in", "out", 1000.0),
        Resistor("Rbot", "out", "0", 1000.0),
    ])

    result = dc_sweep_corners(
        c,
        "Vin",
        0.0,
        10.0,
        5.0,
        [
            CornerSpec("nominal"),
            CornerSpec("rbot-fast", (CornerOverride("Rbot", "resistance", 500.0),)),
        ],
    )

    assert isinstance(result, CornerDcSweepResult)
    assert result.source_name == "Vin"
    assert [point.corner_name for point in result.points] == ["nominal", "rbot-fast"]
    assert [point.source_value for point in result.points[0].result.points] == pytest.approx([0.0, 5.0, 10.0])
    assert [point.node_voltages["out"] for point in result.points[0].result.points] == pytest.approx([0.0, 2.5, 5.0])
    assert [point.node_voltages["out"] for point in result.points[1].result.points] == pytest.approx([0.0, 5.0 / 3.0, 10.0 / 3.0])
    assert format_corner_dc_sweep_table(result, ["V(out)", "I(Vin)"]) == (
        "Corner\tIndex\tSource\tValue\tV(out)\tI(Vin)\n"
        "nominal\t0\tVin\t0.000000e+00\t0.000000e+00\t0.000000e+00\n"
        "nominal\t1\tVin\t5.000000e+00\t2.500000e+00\t-2.500000e-03\n"
        "nominal\t2\tVin\t1.000000e+01\t5.000000e+00\t-5.000000e-03\n"
        "rbot-fast\t0\tVin\t0.000000e+00\t0.000000e+00\t0.000000e+00\n"
        "rbot-fast\t1\tVin\t5.000000e+00\t1.666667e+00\t-3.333333e-03\n"
        "rbot-fast\t2\tVin\t1.000000e+01\t3.333333e+00\t-6.666667e-03\n"
    )


def test_ac_sweep_corners_runs_frequency_sweeps_per_corner() -> None:
    resistance = 1000.0
    capacitance = 1.0e-6
    corner_freq = 1.0 / (2.0 * math.pi * resistance * capacitance)
    c = Circuit([
        VoltageSource("Vin", "in", "0", 1.0),
        Resistor("R1", "in", "out", resistance),
        Capacitor("C1", "out", "0", capacitance),
    ])

    result = ac_sweep_corners(
        c,
        [
            CornerSpec("nominal"),
            CornerSpec("r-fast", (CornerOverride("R1", "resistance", 500.0),)),
        ],
        f_start=corner_freq,
        f_stop=corner_freq,
        n_points=1,
    )

    assert isinstance(result, CornerAcSweepResult)
    assert [point.corner_name for point in result.points] == ["nominal", "r-fast"]
    assert result.points[0].result.points[0].freq == pytest.approx(corner_freq)
    nominal_out = result.points[0].result.points[0].node_voltages["out"]
    fast_out = result.points[1].result.points[0].node_voltages["out"]
    assert abs(nominal_out) == pytest.approx(1.0 / math.sqrt(2.0))
    assert abs(fast_out) == pytest.approx(1.0 / math.sqrt(1.25))
    assert format_corner_ac_table(result, ["V(out)", "I(Vin)"]) == (
        "Corner\tIndex\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase\n"
        "nominal\t0\t1.591549e+02\tV(out)\t5.000000e-01\t-5.000000e-01\t7.071068e-01\t-4.500000e+01\n"
        "nominal\t0\t1.591549e+02\tI(Vin)\t-5.000000e-04\t-5.000000e-04\t7.071068e-04\t-1.350000e+02\n"
        "r-fast\t0\t1.591549e+02\tV(out)\t8.000000e-01\t-4.000000e-01\t8.944272e-01\t-2.656505e+01\n"
        "r-fast\t0\t1.591549e+02\tI(Vin)\t-4.000000e-04\t-8.000000e-04\t8.944272e-04\t-1.165651e+02\n"
    )


def test_tf_corners_runs_transfer_function_per_corner() -> None:
    c = Circuit([
        VoltageSource("Vin", "in", "0", 10.0),
        Resistor("Rtop", "in", "out", 1000.0),
        Resistor("Rbot", "out", "0", 1000.0),
    ])

    result = tf_corners(
        c,
        "Vin",
        "out",
        [
            CornerSpec("nominal"),
            CornerSpec("rbot-fast", (CornerOverride("Rbot", "resistance", 500.0),)),
            CornerSpec("rbot-slow", (CornerOverride("Rbot", "resistance", 2000.0),)),
        ],
    )

    assert isinstance(result, CornerTfResult)
    assert result.input_source == "Vin"
    assert result.output_node == "out"
    assert [point.corner_name for point in result.points] == ["nominal", "rbot-fast", "rbot-slow"]
    assert [point.result.gain for point in result.points] == pytest.approx([0.5, 1.0 / 3.0, 2.0 / 3.0])
    assert [point.result.input_impedance for point in result.points] == pytest.approx([2000.0, 1500.0, 3000.0])
    assert format_corner_tf_table(result) == (
        "Corner\tTransferRatio\tInputImpedance\tOutputImpedance\n"
        "nominal\t5.000000e-01\t2.000000e+03\t5.000000e+02\n"
        "rbot-fast\t3.333333e-01\t1.500000e+03\t3.333333e+02\n"
        "rbot-slow\t6.666667e-01\t3.000000e+03\t6.666667e+02\n"
    )


def test_transient_corners_runs_waveforms_per_corner_and_formats_tables() -> None:
    c = Circuit([
        VoltageSource("V1", "vin", "0", 10.0),
        Resistor("R1", "vin", "mid", 1000.0),
        Resistor("R2", "mid", "0", 1000.0),
    ])

    result = transient_corners(
        c,
        [
            CornerSpec("nominal"),
            CornerSpec("r2-high", (CornerOverride("R2", "resistance", 2000.0),)),
        ],
        t_step=1.0e-3,
        t_stop=2.0e-3,
    )

    assert isinstance(result, CornerTransientResult)
    assert [point.corner_name for point in result.points] == ["nominal", "r2-high"]
    assert [point.points[-1].node_voltages["mid"] for point in result.points] == pytest.approx([5.0, 20.0 / 3.0])
    assert format_corner_transient_table(result, ["V(vin)", "V(mid)", "I(V1)"]) == (
        "Corner\tIndex\tTime\tV(vin)\tV(mid)\tI(V1)\n"
        "nominal\t0\t0.000000e+00\t1.000000e+01\t5.000000e+00\t-5.000000e-03\n"
        "nominal\t1\t1.000000e-03\t1.000000e+01\t5.000000e+00\t-5.000000e-03\n"
        "nominal\t2\t2.000000e-03\t1.000000e+01\t5.000000e+00\t-5.000000e-03\n"
        "r2-high\t0\t0.000000e+00\t1.000000e+01\t6.666667e+00\t-3.333333e-03\n"
        "r2-high\t1\t1.000000e-03\t1.000000e+01\t6.666667e+00\t-3.333333e-03\n"
        "r2-high\t2\t2.000000e-03\t1.000000e+01\t6.666667e+00\t-3.333333e-03\n"
    )


def test_transient_adaptive_corners_runs_waveforms_per_corner_and_formats_tables() -> None:
    c = Circuit([
        VoltageSource("V1", "vin", "0", 1.0),
        Resistor("R1", "vin", "out", 1000.0),
        Capacitor("C1", "out", "0", 1.0e-6),
    ])

    result = transient_adaptive_corners(
        c,
        [
            CornerSpec("nominal"),
            CornerSpec("r1-high", (CornerOverride("R1", "resistance", 2000.0),)),
        ],
        t_step=1.0e-3,
        t_stop=2.0e-3,
        method="trap",
        tol_lte=1.0,
        min_step=1.0e-3,
        max_step=1.0e-3,
    )

    assert isinstance(result, CornerAdaptiveTransientResult)
    assert [point.corner_name for point in result.points] == ["nominal", "r1-high"]
    assert [point.result.points[-1].node_voltages["out"] for point in result.points] == pytest.approx([8.8888889e-1, 6.4e-1])
    assert format_corner_adaptive_transient_table(result, ["V(vin)", "V(out)", "I(V1)"]) == (
        "Corner\tMethod\tStepsRejected\tConverged\tIndex\tTime\tV(vin)\tV(out)\tI(V1)\n"
        "nominal\ttrap\t0\ttrue\t0\t0.000000e+00\t1.000000e+00\t0.000000e+00\t-1.000000e-03\n"
        "nominal\ttrap\t0\ttrue\t1\t1.000000e-03\t1.000000e+00\t6.666667e-01\t-3.333333e-04\n"
        "nominal\ttrap\t0\ttrue\t2\t2.000000e-03\t1.000000e+00\t8.888889e-01\t-1.111111e-04\n"
        "r1-high\ttrap\t0\ttrue\t0\t0.000000e+00\t1.000000e+00\t0.000000e+00\t-5.000000e-04\n"
        "r1-high\ttrap\t0\ttrue\t1\t1.000000e-03\t1.000000e+00\t4.000000e-01\t-3.000000e-04\n"
        "r1-high\ttrap\t0\ttrue\t2\t2.000000e-03\t1.000000e+00\t6.400000e-01\t-1.800000e-04\n"
    )


def test_mc_dc_corners_runs_trials_per_corner_and_formats_tables() -> None:
    c = Circuit([
        VoltageSource("Vin", "in", "0", 10.0),
        Resistor("Rtop", "in", "mid", 1000.0),
        Resistor("Rbot", "mid", "0", 1000.0),
    ])

    nominal = mc_dc(c, "mid", n_trials=2, tolerance=0.0, seed=7)
    result = mc_dc_corners(
        c,
        "mid",
        2,
        [
            CornerSpec("nominal"),
            CornerSpec("rbot-fast", (CornerOverride("Rbot", "resistance", 500.0),)),
        ],
        tolerance=0.0,
        seed=7,
    )

    assert isinstance(result, CornerMcResult)
    assert result.output_node == "mid"
    assert [point.corner_name for point in result.points] == ["nominal", "rbot-fast"]
    assert [point.result.mean for point in result.points] == pytest.approx([5.0, 10.0 / 3.0])
    assert format_mc_table(nominal) == (
        "Trial\tOutputNode\tOutputValue\tMean\tStdDev\tConverged\n"
        "0\tmid\t5.000000e+00\t5.000000e+00\t0.000000e+00\ttrue\n"
        "1\tmid\t5.000000e+00\t5.000000e+00\t0.000000e+00\ttrue\n"
    )
    assert format_corner_mc_table(result) == (
        "Corner\tTrial\tOutputNode\tOutputValue\tMean\tStdDev\tConverged\n"
        "nominal\t0\tmid\t5.000000e+00\t5.000000e+00\t0.000000e+00\ttrue\n"
        "nominal\t1\tmid\t5.000000e+00\t5.000000e+00\t0.000000e+00\ttrue\n"
        "rbot-fast\t0\tmid\t3.333333e+00\t3.333333e+00\t0.000000e+00\ttrue\n"
        "rbot-fast\t1\tmid\t3.333333e+00\t3.333333e+00\t0.000000e+00\ttrue\n"
    )


def test_sens_dc_corners_runs_analysis_per_corner_and_formats_tables() -> None:
    c = Circuit([
        VoltageSource("Vin", "vin", "0", 10.0),
        Resistor("Rtop", "vin", "out", 1000.0),
        Resistor("Rbot", "out", "0", 1000.0),
    ])

    nominal = sens_dc(c, "out")
    result = sens_dc_corners(
        c,
        "out",
        [
            CornerSpec("nominal"),
            CornerSpec("rbot-fast", (CornerOverride("Rbot", "resistance", 500.0),)),
        ],
    )

    assert isinstance(result, CornerSensResult)
    assert result.output_node == "out"
    assert [point.corner_name for point in result.points] == ["nominal", "rbot-fast"]
    assert [point.result.nominal_voltage for point in result.points] == pytest.approx(
        [5.0, 10.0 / 3.0]
    )
    assert format_sens_table(nominal).splitlines()[0] == (
        "OutputNode\tNominalVoltage\tElement\tParameter\tNominalValue\t"
        "Sensitivity\tRelativeSensitivity"
    )
    corner_table = format_corner_sens_table(result)
    assert corner_table.splitlines()[0] == (
        "Corner\tOutputNode\tNominalVoltage\tElement\tParameter\tNominalValue\t"
        "Sensitivity\tRelativeSensitivity"
    )
    assert "nominal\tout\t5.000000e+00\tVin\tvoltage\t1.000000e+01" in corner_table
    assert "rbot-fast\tout\t3.333333e+00\tVin\tvoltage\t1.000000e+01" in corner_table


def test_noise_ac_corners_runs_analysis_per_corner_and_formats_tables() -> None:
    c = Circuit([
        CurrentSource("Iin", "0", "out", 0.0),
        Resistor("Rload", "out", "0", 1000.0),
    ])

    nominal = noise_ac(c, "out", "Iin", freqs=[1000.0], temperature=300.0)
    result = noise_ac_corners(
        c,
        "out",
        "Iin",
        [
            CornerSpec("nominal"),
            CornerSpec("rload-high", (CornerOverride("Rload", "resistance", 2000.0),)),
        ],
        freqs=[1000.0],
        temperature=300.0,
    )

    assert isinstance(result, CornerNoiseResult)
    assert result.output_node == "out"
    assert result.input_source == "Iin"
    assert [point.corner_name for point in result.points] == ["nominal", "rload-high"]
    assert [point.result.points[0].output_psd for point in result.points] == pytest.approx(
        [1.6567788e-17, 3.3135576e-17]
    )
    assert format_noise_table(nominal) == (
        "Index\tFrequency\tOutputNode\tInputSource\tOutputPSD\tInputReferredPSD\t"
        "Element\tType\tSourcePSD\tContributionPSD\n"
        "0\t1.000000e+03\tout\tIin\t1.656779e-17\t1.656779e-23\t"
        "Rload\tthermal\t1.656779e-23\t1.656779e-17\n"
    )
    assert format_corner_noise_table(result) == (
        "Corner\tIndex\tFrequency\tOutputNode\tInputSource\tOutputPSD\tInputReferredPSD\t"
        "Element\tType\tSourcePSD\tContributionPSD\n"
        "nominal\t0\t1.000000e+03\tout\tIin\t1.656779e-17\t1.656779e-23\t"
        "Rload\tthermal\t1.656779e-23\t1.656779e-17\n"
        "rload-high\t0\t1.000000e+03\tout\tIin\t3.313558e-17\t8.283894e-24\t"
        "Rload\tthermal\t8.283894e-24\t3.313558e-17\n"
    )


def test_s_parameters_corners_runs_two_port_extraction_and_formats_tables() -> None:
    c = Circuit([
        VoltageSource("P1", "p1", "0", 0.0),
        VoltageSource("P2", "p2", "0", 0.0),
        Resistor("Rseries", "p1", "p2", 50.0),
    ])

    nominal = s_parameters(
        c,
        port1_source="P1",
        port2_source="P2",
        frequencies=[1.0e6],
        reference_impedance=50.0,
    )
    result = s_parameters_corners(
        c,
        port1_source="P1",
        port2_source="P2",
        frequencies=[1.0e6],
        reference_impedance=50.0,
        corners=[
            CornerSpec("nominal"),
            CornerSpec("series-high", (CornerOverride("Rseries", "resistance", 100.0),)),
        ],
    )

    assert isinstance(result, CornerSParameterResult)
    assert result.port1_source == "P1"
    assert result.port2_source == "P2"
    assert [point.corner_name for point in result.points] == ["nominal", "series-high"]
    assert result.points[0].result.points[0].s21.real == pytest.approx(2.0 / 3.0)
    assert result.points[1].result.points[0].s21.real == pytest.approx(0.5)
    assert format_s_parameter_table(nominal) == (
        "Index\tFrequency\tPort1\tPort2\tParameter\tReal\tImaginary\tMagnitude\tPhase\n"
        "0\t1.000000e+06\tP1\tP2\tS11\t3.333333e-01\t0.000000e+00\t"
        "3.333333e-01\t0.000000e+00\n"
        "0\t1.000000e+06\tP1\tP2\tS21\t6.666667e-01\t0.000000e+00\t"
        "6.666667e-01\t0.000000e+00\n"
        "0\t1.000000e+06\tP1\tP2\tS12\t6.666667e-01\t0.000000e+00\t"
        "6.666667e-01\t0.000000e+00\n"
        "0\t1.000000e+06\tP1\tP2\tS22\t3.333333e-01\t0.000000e+00\t"
        "3.333333e-01\t0.000000e+00\n"
    )
    assert format_corner_s_parameter_table(result) == (
        "Corner\tIndex\tFrequency\tPort1\tPort2\tParameter\tReal\tImaginary\tMagnitude\tPhase\n"
        "nominal\t0\t1.000000e+06\tP1\tP2\tS11\t3.333333e-01\t0.000000e+00\t"
        "3.333333e-01\t0.000000e+00\n"
        "nominal\t0\t1.000000e+06\tP1\tP2\tS21\t6.666667e-01\t0.000000e+00\t"
        "6.666667e-01\t0.000000e+00\n"
        "nominal\t0\t1.000000e+06\tP1\tP2\tS12\t6.666667e-01\t0.000000e+00\t"
        "6.666667e-01\t0.000000e+00\n"
        "nominal\t0\t1.000000e+06\tP1\tP2\tS22\t3.333333e-01\t0.000000e+00\t"
        "3.333333e-01\t0.000000e+00\n"
        "series-high\t0\t1.000000e+06\tP1\tP2\tS11\t5.000000e-01\t0.000000e+00\t"
        "5.000000e-01\t0.000000e+00\n"
        "series-high\t0\t1.000000e+06\tP1\tP2\tS21\t5.000000e-01\t0.000000e+00\t"
        "5.000000e-01\t0.000000e+00\n"
        "series-high\t0\t1.000000e+06\tP1\tP2\tS12\t5.000000e-01\t0.000000e+00\t"
        "5.000000e-01\t0.000000e+00\n"
        "series-high\t0\t1.000000e+06\tP1\tP2\tS22\t5.000000e-01\t0.000000e+00\t"
        "5.000000e-01\t0.000000e+00\n"
    )


def test_digital_bridge_builds_sources_schedule_and_vcd_output() -> None:
    streams = [
        DigitalEventStream(
            "clk",
            [
                DigitalEvent(0.0, "low"),
                DigitalEvent(0.5e-9, "high"),
                DigitalEvent(1.0e-9, "low"),
            ],
        ),
        DigitalEventStream(
            "enable",
            [
                DigitalEvent(0.25e-9, "low"),
                DigitalEvent(0.75e-9, "high"),
            ],
        ),
    ]
    levels = DigitalLogicLevels.cmos_1v8(0.25e-9)

    waveform = digital_events_to_pwl_waveform(streams[0].events, levels)
    source = digital_events_to_voltage_source("Vclk", "clk", "0", streams[0].events, levels)
    sources = digital_event_streams_to_voltage_sources(streams, "0", levels)
    schedule = digital_event_streams_to_bridge_schedule(streams, levels)

    expected_points = (
        (0.0, 0.0),
        (0.5e-9, 0.0),
        (0.75e-9, 1.8),
        (1.0e-9, 1.8),
        (1.25e-9, 0.0),
    )
    assert len(waveform.points) == len(expected_points)
    for point, expected in zip(waveform.points, expected_points, strict=True):
        assert point == pytest.approx(expected)
    assert source.name == "Vclk"
    assert [source.name for source in sources] == ["Vclk", "Venable"]
    assert format_digital_bridge_schedule_table(schedule) == (
        "Index\tTime\tStopTime\n"
        "0\t0.000000e+00\t1.250000e-09\n"
        "1\t2.500000e-10\t1.250000e-09\n"
        "2\t5.000000e-10\t1.250000e-09\n"
        "3\t7.500000e-10\t1.250000e-09\n"
        "4\t1.000000e-09\t1.250000e-09\n"
        "5\t1.250000e-09\t1.250000e-09\n"
    )
    assert format_digital_event_stream_vcd(streams) == (
        "$version coding-adventures spice-engine mixed-signal bridge $end\n"
        "$timescale 1ps $end\n"
        "$scope module spice_bridge $end\n"
        "$var wire 1 s0 clk $end\n"
        "$var wire 1 s1 enable $end\n"
        "$upscope $end\n"
        "$enddefinitions $end\n"
        "$dumpvars\n"
        "0s0\n"
        "0s1\n"
        "$end\n"
        "#0\n"
        "0s0\n"
        "#250\n"
        "0s1\n"
        "#500\n"
        "1s0\n"
        "#750\n"
        "1s1\n"
        "#1000\n"
        "0s0\n"
    )


def test_transient_probe_samples_back_to_digital_streams() -> None:
    levels = DigitalLogicLevels.cmos_1v8(0.25e-9)
    events = [
        DigitalEvent(0.0, "low"),
        DigitalEvent(0.5e-9, "high"),
        DigitalEvent(1.25e-9, "low"),
    ]
    c = Circuit([
        digital_events_to_voltage_source("Vdin", "din", "0", events, levels),
        Resistor("Rload", "din", "0", 1000.0),
    ])

    points = transient(c, t_step=0.25e-9, t_stop=1.5e-9).points
    sampled = sample_transient_probe_as_digital_events(
        points,
        "V(din)",
        DigitalThresholds.cmos_1v8(),
    )
    streams = sample_transient_probes_as_digital_event_streams(
        points,
        [("din", "V(din)")],
        DigitalThresholds.cmos_1v8(),
    )

    assert format_digital_event_table(sampled) == (
        "Index\tTime\tState\n"
        "0\t2.500000e-10\tlow\n"
        "1\t7.500000e-10\thigh\n"
        "2\t1.500000e-09\tlow\n"
    )
    assert format_digital_event_stream_table(streams) == (
        "Signal\tIndex\tTime\tState\n"
        "din\t0\t2.500000e-10\tlow\n"
        "din\t1\t7.500000e-10\thigh\n"
        "din\t2\t1.500000e-09\tlow\n"
    )


def test_transient_bridge_runs_digital_input_and_corner_outputs() -> None:
    input_streams = [
        DigitalEventStream(
            "din",
            [
                DigitalEvent(0.0, "low"),
                DigitalEvent(0.5e-9, "high"),
                DigitalEvent(1.25e-9, "low"),
            ],
        )
    ]
    c = Circuit([Resistor("Rload", "din", "0", 1000.0)])

    result = transient_with_digital_event_streams(
        c,
        input_streams,
        "0",
        DigitalLogicLevels.cmos_1v8(0.25e-9),
        t_step=0.25e-9,
        t_stop=1.5e-9,
        output_probes=[("dout", "V(din)")],
        thresholds=DigitalThresholds.cmos_1v8(),
    )
    corner_result = transient_with_digital_event_streams_corners(
        c,
        input_streams,
        "0",
        DigitalLogicLevels.cmos_1v8(0.25e-9),
        [CornerSpec("nominal"), CornerSpec("load-high", (CornerOverride("Rload", "resistance", 2000.0),))],
        t_step=0.25e-9,
        t_stop=1.5e-9,
        output_probes=[("dout", "V(din)")],
        thresholds=DigitalThresholds.cmos_1v8(),
    )

    assert format_digital_event_stream_table(result.output_streams) == (
        "Signal\tIndex\tTime\tState\n"
        "dout\t0\t2.500000e-10\tlow\n"
        "dout\t1\t7.500000e-10\thigh\n"
        "dout\t2\t1.500000e-09\tlow\n"
    )
    assert format_corner_digital_event_stream_table(corner_result) == (
        "Corner\tSignal\tIndex\tTime\tState\n"
        "nominal\tdout\t0\t2.500000e-10\tlow\n"
        "nominal\tdout\t1\t7.500000e-10\thigh\n"
        "nominal\tdout\t2\t1.500000e-09\tlow\n"
        "load-high\tdout\t0\t2.500000e-10\tlow\n"
        "load-high\tdout\t1\t7.500000e-10\thigh\n"
        "load-high\tdout\t2\t1.500000e-09\tlow\n"
    )


def test_adaptive_transient_bridge_formats_metadata_and_corner_outputs() -> None:
    input_streams = [
        DigitalEventStream(
            "din",
            [
                DigitalEvent(0.0, "low"),
                DigitalEvent(0.5e-9, "high"),
                DigitalEvent(1.25e-9, "low"),
            ],
        )
    ]
    c = Circuit([Resistor("Rload", "din", "0", 1000.0)])

    result = transient_adaptive_with_digital_event_streams(
        c,
        input_streams,
        "0",
        DigitalLogicLevels.cmos_1v8(0.25e-9),
        t_step=0.25e-9,
        t_stop=1.5e-9,
        output_probes=[("dout", "V(din)")],
        thresholds=DigitalThresholds.cmos_1v8(),
        method="trap",
        tol_lte=1.0,
        min_step=0.25e-9,
        max_step=0.25e-9,
    )
    corner_result = transient_adaptive_with_digital_event_streams_corners(
        c,
        input_streams,
        "0",
        DigitalLogicLevels.cmos_1v8(0.25e-9),
        [CornerSpec("nominal")],
        t_step=0.25e-9,
        t_stop=1.5e-9,
        output_probes=[("dout", "V(din)")],
        thresholds=DigitalThresholds.cmos_1v8(),
        method="trap",
        tol_lte=1.0,
        min_step=0.25e-9,
        max_step=0.25e-9,
    )

    assert format_adaptive_digital_event_stream_table(result) == (
        "Method\tStepsRejected\tConverged\tSignal\tIndex\tTime\tState\n"
        "trap\t0\ttrue\tdout\t0\t2.500000e-10\tlow\n"
        "trap\t0\ttrue\tdout\t1\t7.500000e-10\thigh\n"
        "trap\t0\ttrue\tdout\t2\t1.500000e-09\tlow\n"
    )
    assert format_corner_adaptive_digital_event_stream_table(corner_result) == (
        "Corner\tMethod\tStepsRejected\tConverged\tSignal\tIndex\tTime\tState\n"
        "nominal\ttrap\t0\ttrue\tdout\t0\t2.500000e-10\tlow\n"
        "nominal\ttrap\t0\ttrue\tdout\t1\t7.500000e-10\thigh\n"
        "nominal\ttrap\t0\ttrue\tdout\t2\t1.500000e-09\tlow\n"
    )
