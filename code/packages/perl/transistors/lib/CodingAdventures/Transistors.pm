package CodingAdventures::Transistors;

# ============================================================================
# CodingAdventures::Transistors — MOSFET, BJT, CMOS, and TTL transistor
# simulation in pure Perl
# ============================================================================
#
# This module is part of the coding-adventures monorepo, a ground-up
# implementation of the computing stack from transistors to operating systems.
# It sits at layer 10 in the stack — the very lowest level of digital hardware.
#
# ## Why Transistors Matter
#
# Logic gates (AND, OR, NOT) are abstractions.  In real hardware, each gate is
# built from transistors — tiny electrically-controlled switches etched into
# silicon. This module simulates those transistors and shows how logic gates
# emerge from them.
#
# ## Two Main Transistor Families
#
# MOSFET (Metal-Oxide-Semiconductor Field-Effect Transistor):
#   Voltage-controlled.  Used in all modern chips (CMOS technology).
#   Near-zero STATIC power consumption.  Every CPU, GPU, phone chip.
#
# BJT (Bipolar Junction Transistor):
#   Current-controlled.  Invented 1947 at Bell Labs.  Used in historical
#   TTL logic (7400 series), still preferred for some analog applications.
#   Higher static power than MOSFETs.
#
# ## CMOS Logic (Complementary MOS)
#
# Every modern digital chip uses CMOS, which pairs NMOS and PMOS transistors:
#
#   NMOS forms the PULL-DOWN network  (connects output to GND)
#   PMOS forms the PULL-UP  network  (connects output to Vdd)
#
# For any valid input, exactly ONE network is active.  The other is off.
# This is why CMOS consumes near-zero static power — there is no direct path
# from Vdd to GND.
#
# ## TTL Logic (Transistor-Transistor Logic)
#
# TTL used BJTs in a resistor network.  It dominated 1965–1985, but its fatal
# flaw was STATIC POWER CONSUMPTION (1–10 mW per gate).
#
#   1 million gates × 10 mW = 10,000 W — a space heater!
#
# CMOS gates consume near-zero power at rest, allowing chips to scale to
# billions of gates.
#
# ## Transistor Counts per Gate
#
#   Gate     | NMOS | PMOS | Total | Technology
#   ---------|------|------|-------|----------
#   NOT      |  1   |  1   |   2   | CMOS
#   NAND     |  2   |  2   |   4   | CMOS
#   NOR      |  2   |  2   |   4   | CMOS
#   AND      |  3   |  3   |   6   | CMOS (NAND + NOT)
#   OR       |  3   |  3   |   6   | CMOS (NOR + NOT)
#   TTL NAND |  —   |  —   |   3   | BJT (NPN only)
#
# Usage:
#
#   use CodingAdventures::Transistors qw(
#       nmos pmos npn pnp
#       cmos_not cmos_nand cmos_nor cmos_and cmos_or
#       ttl_not  ttl_nand
#   );
#
#   my $out = cmos_not(1);        # 0
#   my $out = cmos_nand(1, 1);    # 0
#   my $out = nmos(1.8, 0.4);     # 1  (conducting)
#   my $out = pmos(0.0, 0.4);     # 1  (conducting)
#
# All functions are also available as class methods.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(
    nmos  pmos  npn  pnp
    cmos_not  cmos_nand  cmos_nor  cmos_and  cmos_or  cmos_xor  cmos_xnor
    ttl_not   ttl_nand
    MOSFET_CUTOFF  MOSFET_LINEAR  MOSFET_SATURATION
    BJT_CUTOFF     BJT_ACTIVE     BJT_SATURATION
);

# ============================================================================
# Operating Region Constants
# ============================================================================
#
# A transistor is an ANALOG device that operates differently depending on the
# voltages applied to its terminals.  The three "regions" describe these modes.
#
# MOSFET REGIONS — think of a water faucet:
#
#   CUTOFF:      Faucet fully closed.  No current flows.
#                (Vgs < Vth — gate voltage below threshold)
#
#   LINEAR:      Faucet open; flow proportional to both handle and pressure.
#                (Vgs >= Vth, Vds < Vgs - Vth — acts like a resistor)
#
#   SATURATION:  Faucet wide open but the pipe is the bottleneck.
#                (Vgs >= Vth, Vds >= Vgs - Vth — constant current amplifier)
#
# For DIGITAL circuits we use CUTOFF (OFF) and deep LINEAR (ON).
# For ANALOG amplifiers we use SATURATION.
#
# BJT REGIONS — confusingly named differently from MOSFET:
#
#   CUTOFF:      No base current → no collector current.  Switch OFF.
#   ACTIVE:      Ic = beta * Ib.  This is the BJT AMPLIFIER region.
#   SATURATION:  Fully ON as a switch.  (NOT the same as MOSFET saturation!)
#
# WARNING: MOSFET "saturation" = amplifier; BJT "saturation" = fully ON.
# These are OPPOSITE behaviours despite sharing a name.  Confusing even
# for experienced engineers.

use constant MOSFET_CUTOFF     => 'cutoff';
use constant MOSFET_LINEAR     => 'linear';
use constant MOSFET_SATURATION => 'saturation';

use constant BJT_CUTOFF     => 'cutoff';
use constant BJT_ACTIVE     => 'active';
use constant BJT_SATURATION => 'saturation';

# ============================================================================
# Default Physical Parameters
# ============================================================================
#
# These represent typical values for real devices.  They are used as defaults
# when the caller does not specify custom parameters.

# MOSFET defaults — 180 nm CMOS process node.
# 180 nm is the last "large" process node still widely used in education and
# mixed-signal designs.  Modern nodes are 3–7 nm but use the same physics.
my %MOSFET_DEFAULTS = (
    vth     => 0.4,      # Threshold voltage (V)
    k       => 0.001,    # Transconductance parameter (A/V^2)
    c_drain => 0.5e-15,  # Drain junction capacitance (F)
);

# BJT defaults — 2N2222A NPN transistor.
# The 2N2222 is one of the most common transistors ever made, used in hobby
# projects, early spacecraft, and educational electronics for 60+ years.
my %BJT_DEFAULTS = (
    beta    => 100.0,    # Current gain (Ic/Ib)
    vbe_on  => 0.7,      # Base-emitter on voltage (V) — silicon junction
    vce_sat => 0.2,      # Collector-emitter saturation voltage (V)
    is      => 1e-14,    # Reverse saturation current (A) — tiny leakage
);

# ============================================================================
# MOSFET Functions — nmos() and pmos()
# ============================================================================

# nmos — determine whether an NMOS transistor is conducting.
#
# ## What is NMOS?
#
# An NMOS (N-channel MOSFET) conducts current from drain to source when the
# gate voltage (Vgs) EXCEEDS the threshold voltage (Vth).
#
# Think of it as a normally-OPEN switch that CLOSES when voltage is applied:
#
#       Output --|
#                | NMOS  (gate = input signal)
#                |
#               GND
#
#   Input HIGH (Vgs > Vth) → NMOS ON  → output pulled to GND (LOW)
#   Input LOW  (Vgs < Vth) → NMOS OFF → output disconnected from GND
#
# In CMOS gates, NMOS forms the PULL-DOWN network.
#
# @param  $vgs   Gate-to-source voltage (V).  Typically the input signal.
# @param  $vth   Threshold voltage (V).  Default: 0.4 V (180 nm process).
# @return        1 if conducting (ON), 0 if not conducting (OFF).

sub nmos {
    my ( $vgs, $vth ) = @_;
    if ( $vgs =~ /::/ ) {
        ( $vgs, $vth ) = ( $vth, $_[2] );
    }
    $vth //= $MOSFET_DEFAULTS{vth};
    return ( $vgs >= $vth ) ? 1 : 0;
}

# pmos — determine whether a PMOS transistor is conducting.
#
# ## What is PMOS?
#
# A PMOS (P-channel MOSFET) is the COMPLEMENT of NMOS.  It conducts when the
# gate voltage is LOW (below the source voltage by more than |Vth|).
#
# Think of it as a normally-CLOSED switch that OPENS when voltage is applied:
#
#       Vdd
#        |
#        | PMOS  (gate = input signal)
#        |
#     Output
#
#   Input LOW  (|Vgs| >= Vth) → PMOS ON  → output pulled to Vdd (HIGH)
#   Input HIGH (|Vgs| < Vth)  → PMOS OFF → output disconnected from Vdd
#
# In CMOS gates, PMOS forms the PULL-UP network.
#
# KEY INSIGHT: NMOS turns ON with HIGH, PMOS turns ON with LOW.
# Pairing them in CMOS ensures exactly one is ON at any time → zero static power.
#
# @param  $vgs   Gate-to-source voltage (V).  Typically negative (gate below source).
# @param  $vth   Threshold voltage magnitude (V).  Default: 0.4 V.
# @return        1 if conducting (ON), 0 if not conducting (OFF).

sub pmos {
    my ( $vgs, $vth ) = @_;
    if ( $vgs =~ /::/ ) {
        ( $vgs, $vth ) = ( $vth, $_[2] );
    }
    $vth //= $MOSFET_DEFAULTS{vth};
    # PMOS conducts when |Vgs| >= Vth  (gate pulled below source)
    return ( abs($vgs) >= $vth ) ? 1 : 0;
}

# ============================================================================
# BJT Functions — npn() and pnp()
# ============================================================================

# npn — determine whether an NPN BJT is conducting.
#
# ## What is NPN?
#
# An NPN BJT (Bipolar Junction Transistor) is CURRENT-controlled.  A small
# current into the base (Ib) controls a much larger collector current (Ic):
#
#   Ic = beta * Ib
#
# This amplification (beta typically 50–300) made radios, TVs, and early
# computers possible.
#
# Unlike MOSFETs, BJTs draw continuous base current even at steady state.
# This is why they lost to CMOS for digital logic (power consumption).
#
# An NPN transistor turns ON when Vbe >= ~0.7 V (the forward voltage of the
# base-emitter silicon junction).
#
# @param  $ib    Base current (A).  Positive = forward biased.
# @param  $beta  Current gain (dimensionless).  Default: 100.
# @return        1 if conducting (Ib implies Vbe >= 0.7 V), 0 otherwise.

sub npn {
    my ( $ib, $beta ) = @_;
    if ( defined $ib && $ib =~ /::/ ) {
        ( $ib, $beta ) = ( $beta, $_[2] );
    }
    $beta //= $BJT_DEFAULTS{beta};
    # If base current is positive, the BJT is driven into conduction.
    # In digital operation, any Ib > 0 means the transistor is ON.
    return ( $ib > 0 ) ? 1 : 0;
}

# pnp — determine whether a PNP BJT is conducting.
#
# ## What is PNP?
#
# The complement of NPN.  A PNP transistor turns ON when the base is pulled
# LOW relative to the emitter.  For PNP, base current flows OUT of the base.
#
# Convention: we pass |Ib| (magnitude).  A positive |Ib| means the transistor
# is being actively driven.
#
# @param  $ib    Base current magnitude (A).
# @param  $beta  Current gain.  Default: 100.
# @return        1 if conducting, 0 otherwise.

sub pnp {
    my ( $ib, $beta ) = @_;
    if ( defined $ib && $ib =~ /::/ ) {
        ( $ib, $beta ) = ( $beta, $_[2] );
    }
    $beta //= $BJT_DEFAULTS{beta};
    return ( $ib > 0 ) ? 1 : 0;
}

# ============================================================================
# CMOS Digital Logic Gates
# ============================================================================
#
# Each CMOS gate uses a PULL-UP network (PMOS) and a PULL-DOWN network (NMOS).
# For any valid input combination, exactly one network is active.
#
# Vdd = 3.3 V (default for 180 nm CMOS).
# Vth = 0.4 V.
#
# For a digital HIGH input:  Vgs_nmos = Vdd (3.3 V)  → NMOS conducting
#                             Vgs_pmos = 0 - Vdd = -3.3 V → |Vgs| = 3.3 >> Vth → PMOS conducting
#
# Wait — that would mean both on simultaneously!  The trick is that NMOS and
# PMOS in CMOS are SERIES/PARALLEL duals of each other:
#   NAND: NMOS in SERIES, PMOS in PARALLEL
#   NOR:  NMOS in PARALLEL, PMOS in SERIES
#
# This duality guarantees that when the pull-down network is ON, the pull-up
# is OFF, and vice versa.

my $VDD = 3.3;  # Supply voltage (V)
my $VTH = 0.4;  # Threshold voltage (V)

# _vgs_nmos — Vgs for an NMOS transistor driven by a digital input.
# For NMOS: gate = input, source = GND, so Vgs = Vin.
sub _vgs_nmos { return $_[0] == 1 ? $VDD : 0.0 }

# _vgs_pmos — Vgs for a PMOS transistor driven by a digital input.
# For PMOS: gate = input, source = Vdd, so Vgs = Vin - Vdd.
#   Input HIGH: Vgs = Vdd - Vdd = 0   → |Vgs| = 0   < Vth → PMOS OFF
#   Input LOW:  Vgs = 0   - Vdd = -Vdd → |Vgs| = Vdd > Vth → PMOS ON
sub _vgs_pmos { return $_[0] == 1 ? 0.0 : -$VDD }

# cmos_not — CMOS NOT (Inverter) gate.
#
# ## Circuit (2 transistors)
#
#       Vdd
#        |
#   +----+---- PMOS (gate = A)
#        |
#    Output = NOT(A)
#        |
#   +----+---- NMOS (gate = A)
#        |
#       GND
#
# Truth table:
#   A | NOT A
#   --|------
#   0 |   1      (NMOS OFF, PMOS ON → output = Vdd = 1)
#   1 |   0      (NMOS ON,  PMOS OFF → output = GND = 0)
#
# Power insight: for either input, one transistor is always OFF.
# That OFF transistor breaks the Vdd-to-GND path → zero static current.
#
# @param  $a   Input (0 or 1).
# @return      0 or 1.

sub cmos_not {
    my $a = ( @_ == 2 ) ? $_[1] : $_[0];
    die "Transistors: a must be 0 or 1\n" if $a != 0 && $a != 1;

    my $nmos_on = nmos( _vgs_nmos($a), $VTH );
    my $pmos_on = pmos( _vgs_pmos($a), $VTH );

    # Exactly one should be on for valid digital inputs.
    if    ( $pmos_on && !$nmos_on ) { return 1; }
    elsif ( $nmos_on && !$pmos_on ) { return 0; }
    else                             { return $a == 0 ? 1 : 0; }  # fallback
}

# cmos_nand — CMOS NAND gate.
#
# ## Why NAND is the "Natural" CMOS Gate
#
# CMOS topology naturally produces INVERTED outputs.  NAND is therefore
# simpler than AND (which requires an inverter stage).
#
# ## Circuit (4 transistors)
#
#       Vdd
#      / \
#  PMOS(A) PMOS(B)   ← parallel: EITHER OFF pulls up
#      \ /
#    Output
#      |
#   NMOS(A)           ← series: BOTH must be ON to pull down
#      |
#   NMOS(B)
#      |
#      GND
#
# PULL-DOWN network (NMOS series): A AND B must both be ON → NAND output = 0
# PULL-UP network (PMOS parallel): EITHER A or B OFF → NAND output = 1
#
# Truth table:
#   A | B | NAND(A,B)
#   --|---|----------
#   0 | 0 |    1      NMOS: A=OFF → no pull down; PMOS(A)=ON, PMOS(B)=ON → pull up
#   0 | 1 |    1      NMOS: A=OFF → no pull down; PMOS(A)=ON → pull up
#   1 | 0 |    1      NMOS: B=OFF → no pull down; PMOS(B)=ON → pull up
#   1 | 1 |    0      NMOS: A=ON, B=ON → pull down; PMOS: both OFF
#
# @param  $a, $b   Inputs (0 or 1).
# @return          0 or 1.

sub cmos_nand {
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    die "Transistors: a must be 0 or 1\n" if $a != 0 && $a != 1;
    die "Transistors: b must be 0 or 1\n" if $b != 0 && $b != 1;

    my $nmos1_on = nmos( _vgs_nmos($a), $VTH );
    my $nmos2_on = nmos( _vgs_nmos($b), $VTH );
    my $pmos1_on = pmos( _vgs_pmos($a), $VTH );
    my $pmos2_on = pmos( _vgs_pmos($b), $VTH );

    my $pulldown_on = ( $nmos1_on && $nmos2_on ) ? 1 : 0;  # series
    my $pullup_on   = ( $pmos1_on || $pmos2_on ) ? 1 : 0;  # parallel

    if    ( $pullup_on   && !$pulldown_on ) { return 1; }
    elsif ( $pulldown_on && !$pullup_on   ) { return 0; }
    else                                    { return !( $a && $b ) ? 1 : 0; }
}

# cmos_nor — CMOS NOR gate.
#
# ## Circuit (4 transistors)
#
#       Vdd
#        |
#    PMOS(A)           ← series: BOTH must be ON to pull up
#        |
#    PMOS(B)
#        |
#    Output
#      / \
#  NMOS(A) NMOS(B)    ← parallel: EITHER ON pulls down
#      \ /
#       GND
#
# PULL-DOWN (NMOS parallel): EITHER A or B ON → NOR output = 0
# PULL-UP   (PMOS series):   BOTH A and B must be OFF → NOR output = 1
#
# Truth table:
#   A | B | NOR(A,B)
#   --|---|----------
#   0 | 0 |    1
#   0 | 1 |    0
#   1 | 0 |    0
#   1 | 1 |    0
#
# @param  $a, $b   Inputs (0 or 1).
# @return          0 or 1.

sub cmos_nor {
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    die "Transistors: a must be 0 or 1\n" if $a != 0 && $a != 1;
    die "Transistors: b must be 0 or 1\n" if $b != 0 && $b != 1;

    my $nmos1_on = nmos( _vgs_nmos($a), $VTH );
    my $nmos2_on = nmos( _vgs_nmos($b), $VTH );
    my $pmos1_on = pmos( _vgs_pmos($a), $VTH );
    my $pmos2_on = pmos( _vgs_pmos($b), $VTH );

    my $pulldown_on = ( $nmos1_on || $nmos2_on ) ? 1 : 0;  # parallel
    my $pullup_on   = ( $pmos1_on && $pmos2_on ) ? 1 : 0;  # series

    if    ( $pullup_on   && !$pulldown_on ) { return 1; }
    elsif ( $pulldown_on && !$pullup_on   ) { return 0; }
    else                                    { return ( $a == 0 && $b == 0 ) ? 1 : 0; }
}

# cmos_and — CMOS AND gate (NAND + Inverter).
#
# There is no "direct" CMOS AND gate.  The CMOS topology naturally produces
# inverted outputs.  To build AND we add an inverter after a NAND:
#
#   AND(A, B) = NOT(NAND(A, B))
#
# This uses 6 transistors total (4 NAND + 2 NOT).
#
# This is also why NAND is the preferred primitive in chip design:
#   - NAND: 4 transistors
#   - AND:  6 transistors (more area, more power, slower)
#
# @param  $a, $b   Inputs (0 or 1).
# @return          0 or 1.

sub cmos_and {
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    die "Transistors: a must be 0 or 1\n" if $a != 0 && $a != 1;
    die "Transistors: b must be 0 or 1\n" if $b != 0 && $b != 1;
    return cmos_not( cmos_nand( $a, $b ) );
}

# cmos_or — CMOS OR gate (NOR + Inverter).
#
#   OR(A, B) = NOT(NOR(A, B))
#
# Uses 6 transistors total (4 NOR + 2 NOT).
#
# @param  $a, $b   Inputs (0 or 1).
# @return          0 or 1.

sub cmos_or {
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    die "Transistors: a must be 0 or 1\n" if $a != 0 && $a != 1;
    die "Transistors: b must be 0 or 1\n" if $b != 0 && $b != 1;
    return cmos_not( cmos_nor( $a, $b ) );
}

# cmos_xor — CMOS XOR gate (built from 4 NAND gates).
#
# ## Circuit (16 transistors — 4 NAND gates of 4 transistors each)
#
# The 4-NAND construction for XOR:
#   Let C  = NAND(A, B)           # middle NAND
#   Let D  = NAND(A, C)           # upper NAND
#   Let E  = NAND(B, C)           # lower NAND
#   XOR    = NAND(D, E)           # output NAND
#
# Truth table:
#   A | B | XOR
#   --|---|----
#   0 | 0 |  0
#   0 | 1 |  1
#   1 | 0 |  1
#   1 | 1 |  0
#
# @param  $a, $b   Inputs (0 or 1).
# @return          0 or 1.

sub cmos_xor {
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    die "Transistors: a must be 0 or 1\n" if $a != 0 && $a != 1;
    die "Transistors: b must be 0 or 1\n" if $b != 0 && $b != 1;
    # 4-NAND XOR construction:
    my $c = cmos_nand( $a, $b );                  # NAND(A, B)
    my $d = cmos_nand( $a, $c );                  # NAND(A, NAND(A,B))
    my $e = cmos_nand( $b, $c );                  # NAND(B, NAND(A,B))
    return cmos_nand( $d, $e );                   # NAND(D, E)
}

# cmos_xnor — CMOS XNOR gate (XOR followed by Inverter = 18 transistors).
#
# XNOR(A, B) = NOT(XOR(A, B))
#
# Truth table:
#   A | B | XNOR
#   --|---|-----
#   0 | 0 |  1    (same — equal)
#   0 | 1 |  0    (different)
#   1 | 0 |  0    (different)
#   1 | 1 |  1    (same — equal)
#
# @param  $a, $b   Inputs (0 or 1).
# @return          0 or 1.

sub cmos_xnor {
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    die "Transistors: a must be 0 or 1\n" if $a != 0 && $a != 1;
    die "Transistors: b must be 0 or 1\n" if $b != 0 && $b != 1;
    return cmos_not( cmos_xor( $a, $b ) );
}

# ============================================================================
# TTL Logic Gates
# ============================================================================
#
# TTL (Transistor-Transistor Logic) uses NPN BJTs with resistors.
# Supply voltage: Vcc = 5 V.
# Input thresholds: LOW < 0.8 V, HIGH > 2.0 V.
#
# Key difference from CMOS:
#   - TTL consumes power CONTINUOUSLY (static current through resistors).
#   - CMOS consumes near-zero power at rest.
#
# TTL was the dominant logic family from ~1965–1985.  The "7400 series"
# chips defined standard gate functions that are still used today (though
# modern 7400 chips are actually implemented in CMOS internally).

my $VCC     = 5.0;   # TTL supply voltage (V)
my $VBE_ON  = 0.7;   # NPN base-emitter on voltage (V)
my $VCE_SAT = 0.2;   # NPN collector-emitter saturation voltage (V)

# TTL input thresholds (from 74-series datasheet):
my $TTL_LOW_MAX  = 0.8;   # Maximum voltage recognised as logic LOW
my $TTL_HIGH_MIN = 2.0;   # Minimum voltage recognised as logic HIGH

# ttl_nand — TTL NAND gate (simplified 7400-style).
#
# ## Simplified Circuit
#
#       Vcc (+5 V)
#        |
#       R1 (4 kΩ)
#        |
#   +----+----+
#   |  Q1     |  Multi-emitter input transistor
#   |  (NPN)  |--- E1 → Input A
#   |         |--- E2 → Input B
#   +----+----+
#        |
#   +----+----+
#   |  Q2     |  Phase splitter
#   +----+----+
#        |
#   +----+----+
#   |  Q3     |  Output transistor (totem-pole output)
#   +----+----+
#        |
#       GND
#
# Operation:
#   Any input LOW  → Q1 saturates that emitter → Q2 & Q3 OFF → output HIGH
#   All inputs HIGH → Q1 base current → Q2 & Q3 ON   → output LOW
#
# Static power: 1–10 mW per gate (even at rest).
#
# @param  $a, $b   Inputs (0 or 1).
# @return          0 or 1.

sub ttl_nand {
    my ( $a, $b ) = ( @_ == 3 ) ? ( $_[1], $_[2] ) : ( $_[0], $_[1] );
    die "Transistors: a must be 0 or 1\n" if $a != 0 && $a != 1;
    die "Transistors: b must be 0 or 1\n" if $b != 0 && $b != 1;

    # Convert digital bits to TTL voltage levels.
    my $va = $a == 1 ? $VCC : 0.0;
    my $vb = $b == 1 ? $VCC : 0.0;

    # TTL input thresholds.
    my $a_high = $va > $TTL_HIGH_MIN ? 1 : 0;
    my $b_high = $vb > $TTL_HIGH_MIN ? 1 : 0;

    if ( $a_high && $b_high ) {
        # Both inputs HIGH → Q3 saturates → output ≈ Vce_sat (LOW)
        return 0;
    } else {
        # At least one input LOW → Q3 OFF → output pulled HIGH through R1
        return 1;
    }
}

# ttl_not — TTL NOT (Inverter) using a simplified RTL-style circuit.
#
# ## Circuit (RTL — Resistor-Transistor Logic, predecessor to TTL)
#
#       Vcc
#        |
#       Rc (1 kΩ collector resistor)
#        |
#       Output
#        |
#   +----+----+
#   |  Q1     |  Single NPN transistor
#   +----+----+
#        |
#       GND
#
#   Input ---[Rb (10 kΩ)]--- Base of Q1
#
# Operation:
#   Input HIGH (Vin > Vbe_on = 0.7 V):
#     Q1 saturates → output pulled to Vce_sat ≈ 0.2 V → logic LOW
#   Input LOW  (Vin < Vbe_on):
#     Q1 in cutoff → output pulled to Vcc through Rc → logic HIGH
#
# Historical note: RTL was used in the Apollo Guidance Computer (1969) that
# navigated Apollo 11 to the Moon.  Every circuit — navigation, attitude
# control, abort — was implemented with NPN transistors and resistors like this.
#
# @param  $a   Input (0 or 1).
# @return      0 or 1.

sub ttl_not {
    my $a = ( @_ == 2 ) ? $_[1] : $_[0];
    die "Transistors: a must be 0 or 1\n" if $a != 0 && $a != 1;
    return $a == 1 ? 0 : 1;
}

1;

__END__

=head1 NAME

CodingAdventures::Transistors - MOSFET, BJT, CMOS, and TTL transistor simulation

=head1 SYNOPSIS

    use CodingAdventures::Transistors qw(
        nmos pmos npn pnp
        cmos_not cmos_nand cmos_nor cmos_and cmos_or
        ttl_not  ttl_nand
    );

    # MOSFET conductance
    my $on  = nmos(1.8, 0.4);    # 1 — conducting (Vgs > Vth)
    my $off = nmos(0.2, 0.4);    # 0 — off (Vgs < Vth)

    # CMOS logic gates
    my $y = cmos_not(1);         # 0
    my $y = cmos_nand(1, 1);     # 0
    my $y = cmos_and(1, 1);      # 1

    # TTL logic
    my $y = ttl_nand(1, 1);      # 0
    my $y = ttl_not(0);          # 1

=head1 DESCRIPTION

Pure-Perl simulation of transistor-level digital logic, implementing:

=over 4

=item * MOSFET: nmos(), pmos() — conductance determination from Vgs/Vth

=item * BJT: npn(), pnp() — conductance determination from base current

=item * CMOS gates: cmos_not, cmos_nand, cmos_nor, cmos_and, cmos_or

=item * TTL gates: ttl_not, ttl_nand

=back

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
