use strict;
use warnings;
use Test2::V0;

# ============================================================================
# Module Loading Test
#
# This test verifies that the CaesarCipher module can be loaded and that
# it exposes a version number. This is the most basic sanity check --
# if this fails, nothing else will work.
# ============================================================================

ok(eval { require CodingAdventures::CaesarCipher; 1 }, 'CodingAdventures::CaesarCipher loads')
    or diag("Failed to load: $@");

ok(defined $CodingAdventures::CaesarCipher::VERSION, 'module loaded with version');
is($CodingAdventures::CaesarCipher::VERSION, '0.1.0', 'version is 0.1.0');

done_testing;
