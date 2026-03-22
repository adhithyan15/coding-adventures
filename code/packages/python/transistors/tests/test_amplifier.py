"""Tests for analog amplifier analysis."""

from transistors.amplifier import analyze_common_emitter_amp, analyze_common_source_amp
from transistors.bjt import NPN
from transistors.mosfet import NMOS
from transistors.types import BJTParams


class TestCommonSourceAmplifier:
    """Test NMOS common-source amplifier analysis."""

    def test_inverting_gain(self) -> None:
        """Common-source amplifier should have negative voltage gain."""
        t = NMOS()
        result = analyze_common_source_amp(t, vgs=1.5, vdd=3.3, r_drain=10_000)
        assert result.voltage_gain < 0

    def test_high_input_impedance(self) -> None:
        """MOSFET amplifiers should have very high input impedance."""
        t = NMOS()
        result = analyze_common_source_amp(t, vgs=1.5, vdd=3.3, r_drain=10_000)
        assert result.input_impedance > 1e9

    def test_positive_transconductance(self) -> None:
        """Transconductance should be positive."""
        t = NMOS()
        result = analyze_common_source_amp(t, vgs=1.5, vdd=3.3, r_drain=10_000)
        assert result.transconductance > 0

    def test_positive_bandwidth(self) -> None:
        """Bandwidth should be positive."""
        t = NMOS()
        result = analyze_common_source_amp(t, vgs=1.5, vdd=3.3, r_drain=10_000)
        assert result.bandwidth > 0

    def test_operating_point(self) -> None:
        """Operating point should contain required keys."""
        t = NMOS()
        result = analyze_common_source_amp(t, vgs=1.5, vdd=3.3, r_drain=10_000)
        assert "vgs" in result.operating_point
        assert "vds" in result.operating_point
        assert "ids" in result.operating_point
        assert "gm" in result.operating_point

    def test_higher_rd_more_gain(self) -> None:
        """Higher drain resistance should give more voltage gain."""
        t = NMOS()
        r1 = analyze_common_source_amp(t, vgs=1.5, vdd=3.3, r_drain=5_000)
        r2 = analyze_common_source_amp(t, vgs=1.5, vdd=3.3, r_drain=20_000)
        assert abs(r2.voltage_gain) > abs(r1.voltage_gain)


class TestCommonEmitterAmplifier:
    """Test NPN common-emitter amplifier analysis."""

    def test_inverting_gain(self) -> None:
        """Common-emitter amplifier should have negative voltage gain."""
        t = NPN()
        result = analyze_common_emitter_amp(t, vbe=0.7, vcc=5.0, r_collector=4700)
        assert result.voltage_gain < 0

    def test_moderate_input_impedance(self) -> None:
        """BJT amplifiers have moderate input impedance (r_pi)."""
        t = NPN()
        result = analyze_common_emitter_amp(t, vbe=0.7, vcc=5.0, r_collector=4700)
        # r_pi should be in kOhm range for typical bias
        assert 100 < result.input_impedance < 1e6

    def test_positive_transconductance(self) -> None:
        """Transconductance should be positive."""
        t = NPN()
        result = analyze_common_emitter_amp(t, vbe=0.7, vcc=5.0, r_collector=4700)
        assert result.transconductance > 0

    def test_higher_beta_higher_impedance(self) -> None:
        """Higher beta should give higher input impedance."""
        t_low = NPN(BJTParams(beta=50))
        t_high = NPN(BJTParams(beta=200))
        r1 = analyze_common_emitter_amp(t_low, vbe=0.7, vcc=5.0, r_collector=4700)
        r2 = analyze_common_emitter_amp(t_high, vbe=0.7, vcc=5.0, r_collector=4700)
        assert r2.input_impedance > r1.input_impedance

    def test_operating_point(self) -> None:
        """Operating point should contain required keys."""
        t = NPN()
        result = analyze_common_emitter_amp(t, vbe=0.7, vcc=5.0, r_collector=4700)
        assert "vbe" in result.operating_point
        assert "vce" in result.operating_point
        assert "ic" in result.operating_point
        assert "ib" in result.operating_point
