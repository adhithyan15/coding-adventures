use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::DartmouthBasicParser; 1 },
    'CodingAdventures::DartmouthBasicParser loads' );

ok( eval { require CodingAdventures::DartmouthBasicParser::ASTNode; 1 },
    'CodingAdventures::DartmouthBasicParser::ASTNode loads' );

ok( CodingAdventures::DartmouthBasicParser->VERSION,
    'DartmouthBasicParser has a VERSION' );

ok( CodingAdventures::DartmouthBasicParser::ASTNode->VERSION,
    'ASTNode has a VERSION' );

done_testing;
