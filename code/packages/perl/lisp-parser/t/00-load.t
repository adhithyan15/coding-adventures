use strict;
use warnings;
use Test2::V0;

# Basic smoke test — both modules must load cleanly.
ok( eval { require CodingAdventures::LispParser; 1 },          'LispParser loads' );
ok( eval { require CodingAdventures::LispParser::ASTNode; 1 }, 'ASTNode loads'    );

ok( defined $CodingAdventures::LispParser::VERSION,          'LispParser VERSION defined' );
ok( defined $CodingAdventures::LispParser::ASTNode::VERSION, 'ASTNode VERSION defined'    );

done_testing;
