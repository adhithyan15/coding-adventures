package CodingAdventures::GF256Native;

# ============================================================================
# CodingAdventures::GF256Native — Rust-backed GF(256) arithmetic for Perl
# ============================================================================
#
# Loads the Rust-compiled GF256Native.so via DynaLoader and registers the
# boot function boot_CodingAdventures__GF256Native which installs XSUBs.
#
# ## Usage
#
#   use CodingAdventures::GF256Native;
#   use GN = 'CodingAdventures::GF256Native';
#
#   GN::add(83, 202)         # 153  (XOR)
#   GN::multiply(2, 16)      # 32
#   GN::divide(4, 2)         # 2
#   GN::power(2, 8)          # 29   (reduced mod 0x11D)
#   GN::inverse(83)          # multiplicative inverse of 83

use strict;
use warnings;
use DynaLoader;

our $VERSION = '0.01';
our @ISA = ('DynaLoader');

sub dl_load_flags { 0x01 }

__PACKAGE__->bootstrap($VERSION);

1;

__END__

=head1 NAME

CodingAdventures::GF256Native — Rust-backed GF(256) Galois Field arithmetic

=head1 SYNOPSIS

    use CodingAdventures::GF256Native;

    my $sum = CodingAdventures::GF256Native::add(83, 202);       # 153
    my $prd = CodingAdventures::GF256Native::multiply(17, 31);
    my $inv = CodingAdventures::GF256Native::inverse(83);
    my $pw  = CodingAdventures::GF256Native::power(2, 8);        # 29

=head1 FUNCTIONS

=over 4

=item add($a, $b)

Addition in GF(256) = XOR. Returns integer 0–255.

=item subtract($a, $b)

Subtraction in GF(256) = XOR. Same as add in characteristic 2.

=item multiply($a, $b)

Multiplication using log/antilog tables. Returns integer 0–255.

=item divide($a, $b)

Division. Dies with "GF256: division by zero" if $b is 0.

=item power($base, $exp)

Raise $base to the non-negative integer power $exp.

=item inverse($a)

Multiplicative inverse. Dies if $a is 0.

=back
