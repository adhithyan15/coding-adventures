use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::LatticeParser; 1 },
    'CodingAdventures::LatticeParser loads' );

ok( eval { require CodingAdventures::LatticeParser::ASTNode; 1 },
    'CodingAdventures::LatticeParser::ASTNode loads' );

ok( CodingAdventures::LatticeParser->VERSION, 'LatticeParser has a VERSION' );
ok( CodingAdventures::LatticeParser::ASTNode->VERSION, 'ASTNode has a VERSION' );

done_testing;
