"""Tests for electrical analysis functions."""

from transistors.analysis import (
    analyze_power,
    analyze_timing,
    compare_cmos_vs_ttl,
    compute_noise_margins,
    demonstrate_cmos_scaling,
)
from transistors.cmos_gates import CMOSInverter, CMOSNand, CMOSNor
from transistors.ttl_gates import TTLNand
from transistors.types import CircuitParams


class TestNoiseMargins:
    """Test noise margin computation."""

    def test_cmos_positive_margins(self) -> None:
        """CMOS noise margins should be positive."""
        nm = compute_noise_margins(CMOSInverter())
        assert nm.nml > 0
        assert nm.nmh > 0

    def test_cmos_symmetric(self) -> None:
        """CMOS noise margins should be roughly symmetric."""
        nm = compute_noise_margins(CMOSInverter())
        assert abs(nm.nml - nm.nmh) < nm.nml * 0.5

    def test_ttl_positive_margins(self) -> None:
        """TTL noise margins should be positive."""
        nm = compute_noise_margins(TTLNand())
        assert nm.nml > 0
        assert nm.nmh > 0

    def test_cmos_vol_near_zero(self) -> None:
        """CMOS output LOW should be near 0V."""
        nm = compute_noise_margins(CMOSInverter())
        assert nm.vol < 0.1

    def test_ttl_vol_vce_sat(self) -> None:
        """TTL output LOW should be near Vce_sat."""
        nm = compute_noise_margins(TTLNand())
        assert nm.vol < 0.5


class TestPowerAnalysis:
    """Test power consumption analysis."""

    def test_cmos_zero_static_power(self) -> None:
        """CMOS gates should have near-zero static power."""
        power = analyze_power(CMOSInverter())
        assert power.static_power < 1e-9

    def test_ttl_significant_static_power(self) -> None:
        """TTL gates should have milliwatt-level static power."""
        power = analyze_power(TTLNand())
        assert power.static_power > 1e-3

    def test_positive_dynamic_power(self) -> None:
        """Dynamic power should be positive at non-zero frequency."""
        power = analyze_power(CMOSInverter(), frequency=1e9)
        assert power.dynamic_power > 0

    def test_total_power_sum(self) -> None:
        """Total power should be static + dynamic."""
        power = analyze_power(CMOSInverter(), frequency=1e9)
        assert abs(power.total_power - (power.static_power + power.dynamic_power)) < 1e-15

    def test_energy_per_switch_positive(self) -> None:
        """Energy per switch should be positive."""
        power = analyze_power(CMOSInverter())
        assert power.energy_per_switch > 0

    def test_cmos_nand_power(self) -> None:
        """CMOSNand should also work with analyze_power."""
        power = analyze_power(CMOSNand())
        assert power.static_power == 0.0

    def test_cmos_nor_power(self) -> None:
        """CMOSNor should also work with analyze_power."""
        power = analyze_power(CMOSNor())
        assert power.static_power == 0.0


class TestTimingAnalysis:
    """Test timing characteristic analysis."""

    def test_cmos_positive_delays(self) -> None:
        """CMOS propagation delays should be positive."""
        timing = analyze_timing(CMOSInverter())
        assert timing.tphl > 0
        assert timing.tplh > 0
        assert timing.tpd > 0

    def test_tpd_is_average(self) -> None:
        """tpd should be the average of tphl and tplh."""
        timing = analyze_timing(CMOSInverter())
        expected = (timing.tphl + timing.tplh) / 2.0
        assert abs(timing.tpd - expected) < 1e-20

    def test_cmos_faster_than_ttl(self) -> None:
        """CMOS delay should be faster than TTL delay."""
        cmos_timing = analyze_timing(CMOSInverter())
        ttl_timing = analyze_timing(TTLNand())
        assert cmos_timing.tpd < ttl_timing.tpd

    def test_positive_rise_fall(self) -> None:
        """Rise and fall times should be positive."""
        timing = analyze_timing(CMOSInverter())
        assert timing.rise_time > 0
        assert timing.fall_time > 0

    def test_max_frequency_positive(self) -> None:
        """Maximum frequency should be positive."""
        timing = analyze_timing(CMOSInverter())
        assert timing.max_frequency > 0

    def test_cmos_nand_timing(self) -> None:
        """CMOSNand should also work with analyze_timing."""
        timing = analyze_timing(CMOSNand())
        assert timing.tpd > 0

    def test_cmos_nor_timing(self) -> None:
        """CMOSNor should also work with analyze_timing."""
        timing = analyze_timing(CMOSNor())
        assert timing.tpd > 0


class TestComparisonUtilities:
    """Test CMOS vs TTL comparison and scaling functions."""

    def test_compare_returns_both(self) -> None:
        """compare_cmos_vs_ttl should return both CMOS and TTL data."""
        result = compare_cmos_vs_ttl()
        assert "cmos" in result
        assert "ttl" in result

    def test_cmos_less_static_power(self) -> None:
        """CMOS should have much less static power than TTL."""
        result = compare_cmos_vs_ttl()
        assert result["cmos"]["static_power_w"] < result["ttl"]["static_power_w"]

    def test_scaling_returns_list(self) -> None:
        """demonstrate_cmos_scaling should return a list of dicts."""
        result = demonstrate_cmos_scaling()
        assert isinstance(result, list)
        assert len(result) > 0

    def test_scaling_default_nodes(self) -> None:
        """Default should produce 6 technology nodes."""
        result = demonstrate_cmos_scaling()
        assert len(result) == 6

    def test_scaling_custom_nodes(self) -> None:
        """Custom technology nodes should be respected."""
        result = demonstrate_cmos_scaling([180e-9, 45e-9])
        assert len(result) == 2

    def test_scaling_vdd_decreases(self) -> None:
        """Supply voltage should generally decrease with scaling."""
        result = demonstrate_cmos_scaling()
        # First node (180nm) should have higher Vdd than last (3nm)
        assert result[0]["vdd_v"] > result[-1]["vdd_v"]

    def test_scaling_has_expected_keys(self) -> None:
        """Each scaling result should have expected keys."""
        result = demonstrate_cmos_scaling([180e-9])
        entry = result[0]
        assert "node_nm" in entry
        assert "vdd_v" in entry
        assert "vth_v" in entry
        assert "propagation_delay_s" in entry
        assert "dynamic_power_w" in entry
        assert "leakage_current_a" in entry
