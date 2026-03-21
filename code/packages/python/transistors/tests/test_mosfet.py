"""Tests for MOSFET transistors (NMOS and PMOS)."""

import pytest

from transistors.mosfet import NMOS, PMOS
from transistors.types import MOSFETParams, MOSFETRegion


class TestNMOS:
    """Test NMOS transistor operating regions and electrical behavior."""

    def test_cutoff_region(self) -> None:
        """Vgs below threshold -> no current, switch OFF."""
        t = NMOS()
        assert t.region(vgs=0.0, vds=1.0) == MOSFETRegion.CUTOFF
        assert t.drain_current(vgs=0.0, vds=1.0) == 0.0
        assert not t.is_conducting(vgs=0.0)

    def test_cutoff_negative_vgs(self) -> None:
        """Negative Vgs should also be cutoff."""
        t = NMOS()
        assert t.region(vgs=-1.0, vds=0.0) == MOSFETRegion.CUTOFF
        assert t.drain_current(vgs=-1.0, vds=0.0) == 0.0

    def test_linear_region(self) -> None:
        """Vgs above threshold, low Vds -> linear region."""
        t = NMOS()
        assert t.region(vgs=1.5, vds=0.1) == MOSFETRegion.LINEAR
        ids = t.drain_current(vgs=1.5, vds=0.1)
        assert ids > 0

    def test_saturation_region(self) -> None:
        """Vgs above threshold, high Vds -> saturation."""
        t = NMOS()
        assert t.region(vgs=1.0, vds=3.0) == MOSFETRegion.SATURATION
        ids = t.drain_current(vgs=1.0, vds=3.0)
        assert ids > 0

    def test_saturation_current_independent_of_vds(self) -> None:
        """In saturation, current depends only on Vgs, not Vds."""
        t = NMOS()
        ids_1 = t.drain_current(vgs=1.5, vds=3.0)
        ids_2 = t.drain_current(vgs=1.5, vds=5.0)
        # Both should be in saturation with same Vgs -> same current
        assert abs(ids_1 - ids_2) < 1e-10

    def test_linear_current_increases_with_vds(self) -> None:
        """In linear region, current increases with Vds."""
        t = NMOS()
        ids_low = t.drain_current(vgs=3.0, vds=0.1)
        ids_high = t.drain_current(vgs=3.0, vds=0.5)
        assert ids_high > ids_low

    def test_is_conducting(self) -> None:
        """is_conducting should be True when Vgs >= Vth."""
        t = NMOS()
        assert not t.is_conducting(vgs=0.3)  # Below default Vth=0.4
        assert t.is_conducting(vgs=0.4)      # At Vth
        assert t.is_conducting(vgs=1.0)      # Above Vth

    def test_output_voltage_on(self) -> None:
        """When ON, output should be pulled to GND."""
        t = NMOS()
        assert t.output_voltage(vgs=3.3, vdd=3.3) == 0.0

    def test_output_voltage_off(self) -> None:
        """When OFF, output should be at Vdd."""
        t = NMOS()
        assert t.output_voltage(vgs=0.0, vdd=3.3) == 3.3

    def test_custom_params(self) -> None:
        """Custom parameters should be respected."""
        params = MOSFETParams(vth=0.7, k=0.002)
        t = NMOS(params)
        assert not t.is_conducting(vgs=0.5)  # Below custom Vth
        assert t.is_conducting(vgs=0.7)      # At custom Vth

    def test_transconductance_cutoff(self) -> None:
        """gm should be 0 in cutoff."""
        t = NMOS()
        assert t.transconductance(vgs=0.0, vds=1.0) == 0.0

    def test_transconductance_saturation(self) -> None:
        """gm should be positive in saturation."""
        t = NMOS()
        gm = t.transconductance(vgs=1.5, vds=3.0)
        assert gm > 0

    def test_boundary_cutoff_linear(self) -> None:
        """Just above Vth with small Vds, transistor is in linear."""
        t = NMOS()
        # At Vgs=0.4 (exactly Vth), Vov=0, so Vds >= Vov puts us in saturation.
        # Need Vgs slightly above Vth to get a non-zero Vov.
        assert t.region(vgs=0.5, vds=0.01) == MOSFETRegion.LINEAR

    def test_boundary_linear_saturation(self) -> None:
        """At Vds = Vgs - Vth, transistor enters saturation."""
        t = NMOS()
        vgs = 1.0
        vth = 0.4
        vds = vgs - vth  # Exactly at boundary
        assert t.region(vgs=vgs, vds=vds) == MOSFETRegion.SATURATION


class TestPMOS:
    """Test PMOS transistor operating regions and electrical behavior."""

    def test_cutoff_when_vgs_zero(self) -> None:
        """PMOS with Vgs=0 (gate at source level) should be OFF."""
        t = PMOS()
        assert t.region(vgs=0.0, vds=0.0) == MOSFETRegion.CUTOFF
        assert not t.is_conducting(vgs=0.0)

    def test_conducts_when_vgs_negative(self) -> None:
        """PMOS conducts when Vgs is sufficiently negative."""
        t = PMOS()
        assert t.is_conducting(vgs=-1.5)
        assert t.region(vgs=-1.5, vds=-3.0) == MOSFETRegion.SATURATION

    def test_linear_region(self) -> None:
        """PMOS in linear region with small |Vds|."""
        t = PMOS()
        assert t.region(vgs=-1.5, vds=-0.1) == MOSFETRegion.LINEAR

    def test_drain_current_positive(self) -> None:
        """PMOS drain current magnitude should be positive."""
        t = PMOS()
        ids = t.drain_current(vgs=-1.5, vds=-3.0)
        assert ids > 0

    def test_cutoff_no_current(self) -> None:
        """PMOS in cutoff should have zero current."""
        t = PMOS()
        assert t.drain_current(vgs=0.0, vds=-1.0) == 0.0

    def test_output_voltage_on(self) -> None:
        """When ON, PMOS pulls output to Vdd."""
        t = PMOS()
        assert t.output_voltage(vgs=-3.3, vdd=3.3) == 3.3

    def test_output_voltage_off(self) -> None:
        """When OFF, PMOS output is at GND."""
        t = PMOS()
        assert t.output_voltage(vgs=0.0, vdd=3.3) == 0.0

    def test_complementary_to_nmos(self) -> None:
        """PMOS should be ON when NMOS is OFF and vice versa."""
        nmos = NMOS()
        pmos = PMOS()
        vdd = 3.3

        # Input HIGH: NMOS ON, PMOS OFF
        assert nmos.is_conducting(vgs=vdd)
        assert not pmos.is_conducting(vgs=0.0)  # Vgs_p = Vin - Vdd = 0 when Vin=Vdd

        # Input LOW: NMOS OFF, PMOS ON
        assert not nmos.is_conducting(vgs=0.0)
        assert pmos.is_conducting(vgs=-vdd)  # Vgs_p = 0 - Vdd = -Vdd

    def test_transconductance_cutoff(self) -> None:
        """gm should be 0 in cutoff."""
        t = PMOS()
        assert t.transconductance(vgs=0.0, vds=0.0) == 0.0

    def test_transconductance_on(self) -> None:
        """gm should be positive when conducting."""
        t = PMOS()
        gm = t.transconductance(vgs=-1.5, vds=-3.0)
        assert gm > 0
