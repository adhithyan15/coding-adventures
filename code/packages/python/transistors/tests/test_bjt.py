"""Tests for BJT transistors (NPN and PNP)."""

import pytest

from transistors.bjt import NPN, PNP
from transistors.types import BJTParams, BJTRegion


class TestNPN:
    """Test NPN transistor operating regions and electrical behavior."""

    def test_cutoff_region(self) -> None:
        """Vbe below threshold -> no current."""
        t = NPN()
        assert t.region(vbe=0.0, vce=5.0) == BJTRegion.CUTOFF
        assert t.collector_current(vbe=0.0, vce=5.0) == 0.0
        assert not t.is_conducting(vbe=0.0)

    def test_active_region(self) -> None:
        """Vbe at threshold, Vce > Vce_sat -> active (amplifier)."""
        t = NPN()
        assert t.region(vbe=0.7, vce=3.0) == BJTRegion.ACTIVE
        ic = t.collector_current(vbe=0.7, vce=3.0)
        assert ic > 0

    def test_saturation_region(self) -> None:
        """Vbe at threshold, Vce <= Vce_sat -> saturated (switch ON)."""
        t = NPN()
        assert t.region(vbe=0.7, vce=0.1) == BJTRegion.SATURATION

    def test_is_conducting(self) -> None:
        """is_conducting should be True when Vbe >= Vbe_on."""
        t = NPN()
        assert not t.is_conducting(vbe=0.5)
        assert t.is_conducting(vbe=0.7)
        assert t.is_conducting(vbe=1.0)

    def test_current_gain(self) -> None:
        """In active region, Ic should be approximately beta * Ib."""
        t = NPN(BJTParams(beta=100))
        ic = t.collector_current(vbe=0.7, vce=3.0)
        ib = t.base_current(vbe=0.7, vce=3.0)
        # ic / ib should be approximately beta
        if ib > 0:
            assert abs(ic / ib - 100.0) < 1.0

    def test_base_current_cutoff(self) -> None:
        """Base current should be 0 in cutoff."""
        t = NPN()
        assert t.base_current(vbe=0.0, vce=5.0) == 0.0

    def test_transconductance_cutoff(self) -> None:
        """gm should be 0 in cutoff."""
        t = NPN()
        assert t.transconductance(vbe=0.0, vce=5.0) == 0.0

    def test_transconductance_active(self) -> None:
        """gm should be positive in active region."""
        t = NPN()
        gm = t.transconductance(vbe=0.7, vce=3.0)
        assert gm > 0

    def test_custom_beta(self) -> None:
        """Custom beta should affect current gain."""
        t_low = NPN(BJTParams(beta=50))
        t_high = NPN(BJTParams(beta=200))
        ic_low = t_low.collector_current(vbe=0.7, vce=3.0)
        ic_high = t_high.collector_current(vbe=0.7, vce=3.0)
        # Same Ic (determined by Is and Vbe), different Ib
        ib_low = t_low.base_current(vbe=0.7, vce=3.0)
        ib_high = t_high.base_current(vbe=0.7, vce=3.0)
        assert ib_low > ib_high  # Lower beta = more base current

    def test_saturation_boundary(self) -> None:
        """At Vce = Vce_sat, transistor is in saturation."""
        t = NPN()
        assert t.region(vbe=0.7, vce=0.2) == BJTRegion.SATURATION

    def test_active_boundary(self) -> None:
        """Just above Vce_sat, transistor is in active."""
        t = NPN()
        assert t.region(vbe=0.7, vce=0.3) == BJTRegion.ACTIVE


class TestPNP:
    """Test PNP transistor operating regions and electrical behavior."""

    def test_cutoff_region(self) -> None:
        """PNP with small |Vbe| should be OFF."""
        t = PNP()
        assert t.region(vbe=0.0, vce=0.0) == BJTRegion.CUTOFF
        assert t.collector_current(vbe=0.0, vce=0.0) == 0.0
        assert not t.is_conducting(vbe=0.0)

    def test_conducts_with_negative_vbe(self) -> None:
        """PNP conducts when |Vbe| >= Vbe_on (Vbe typically negative)."""
        t = PNP()
        assert t.is_conducting(vbe=-0.7)
        assert t.region(vbe=-0.7, vce=-3.0) == BJTRegion.ACTIVE

    def test_saturation(self) -> None:
        """PNP in saturation when |Vce| <= Vce_sat."""
        t = PNP()
        assert t.region(vbe=-0.7, vce=-0.1) == BJTRegion.SATURATION

    def test_drain_current_positive(self) -> None:
        """PNP collector current magnitude should be positive."""
        t = PNP()
        ic = t.collector_current(vbe=-0.7, vce=-3.0)
        assert ic > 0

    def test_base_current(self) -> None:
        """PNP should have non-zero base current when conducting."""
        t = PNP()
        ib = t.base_current(vbe=-0.7, vce=-3.0)
        assert ib > 0

    def test_cutoff_no_base_current(self) -> None:
        """PNP base current should be 0 in cutoff."""
        t = PNP()
        assert t.base_current(vbe=0.0, vce=0.0) == 0.0

    def test_transconductance(self) -> None:
        """PNP gm should be positive when conducting."""
        t = PNP()
        gm = t.transconductance(vbe=-0.7, vce=-3.0)
        assert gm > 0

    def test_transconductance_cutoff(self) -> None:
        """PNP gm should be 0 in cutoff."""
        t = PNP()
        assert t.transconductance(vbe=0.0, vce=0.0) == 0.0
