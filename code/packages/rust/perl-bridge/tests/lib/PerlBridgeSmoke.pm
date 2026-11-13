package PerlBridgeSmoke;

# The Perl half of the perl_bridge_smoke example: load the Rust library through
# DynaLoader, exactly as an XS module does.  DynaLoader looks for
#   auto/PerlBridgeSmoke/PerlBridgeSmoke.<dlext>
# under @INC and calls boot_PerlBridgeSmoke in it, which registers the XSUBs
# (tests/smoke.pl stages the library there first).  A library that fails to load
# dies here; there is no fallback.

use strict;
use warnings;
use DynaLoader;

our $VERSION = '0.01';
our @ISA = ('DynaLoader');

# RTLD_GLOBAL, as the repository's other Rust XS modules request.
sub dl_load_flags { 0x01 }

__PACKAGE__->bootstrap($VERSION);

1;
