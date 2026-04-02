use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::CliBuilder; 1 }, 'CliBuilder loads' );
ok( eval { require CodingAdventures::CliBuilder::TokenClassifier; 1 }, 'TokenClassifier loads' );
ok( eval { require CodingAdventures::CliBuilder::SpecLoader; 1 }, 'SpecLoader loads' );
ok( eval { require CodingAdventures::CliBuilder::HelpGenerator; 1 }, 'HelpGenerator loads' );
ok( eval { require CodingAdventures::CliBuilder::FlagValidator; 1 }, 'FlagValidator loads' );
ok( eval { require CodingAdventures::CliBuilder::Parser; 1 }, 'Parser loads' );
ok( CodingAdventures::CliBuilder->VERSION, 'CliBuilder has VERSION' );

done_testing;
