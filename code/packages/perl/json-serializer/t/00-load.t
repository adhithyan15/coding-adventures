use strict;
use warnings;
use Test2::V0;

# ============================================================================
# 00-load.t — smoke test: module loads without errors
# ============================================================================

ok(eval { require CodingAdventures::JsonSerializer; 1 }, 'CodingAdventures::JsonSerializer loads');

ok( defined &CodingAdventures::JsonSerializer::encode,        'encode defined' );
ok( defined &CodingAdventures::JsonSerializer::decode,        'decode defined' );
ok( defined &CodingAdventures::JsonSerializer::validate,      'validate defined' );
ok( defined &CodingAdventures::JsonSerializer::schema_encode, 'schema_encode defined' );
ok( defined &CodingAdventures::JsonSerializer::is_null,       'is_null defined' );
ok( defined $CodingAdventures::JsonSerializer::NULL,          '$NULL defined' );
ok( defined $CodingAdventures::JsonSerializer::VERSION,       '$VERSION defined' );

done_testing;
