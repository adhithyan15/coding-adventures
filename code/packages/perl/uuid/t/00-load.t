use strict;
use warnings;
use Test2::V0;

# ---------------------------------------------------------------------------
# Smoke test: verify the module loads and exports the expected symbols.
# ---------------------------------------------------------------------------

ok(eval { require CodingAdventures::UUID; 1 }, 'CodingAdventures::UUID loads');

my @expected_exports = qw(
    generate_v1  generate_v3  generate_v4  generate_v5  generate_v7
    parse  validate  nil_uuid
);

for my $fn (@expected_exports) {
    ok(
        CodingAdventures::UUID->can($fn),
        "CodingAdventures::UUID can $fn"
    );
}

# Verify namespace constants are defined
ok(defined $CodingAdventures::UUID::NAMESPACE_DNS,  'NAMESPACE_DNS is defined');
ok(defined $CodingAdventures::UUID::NAMESPACE_URL,  'NAMESPACE_URL is defined');
ok(defined $CodingAdventures::UUID::NAMESPACE_OID,  'NAMESPACE_OID is defined');
ok(defined $CodingAdventures::UUID::NAMESPACE_X500, 'NAMESPACE_X500 is defined');

done_testing();
