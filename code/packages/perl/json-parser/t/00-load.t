use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::JsonParser; 1 },
    'CodingAdventures::JsonParser loads' );

ok( eval { require CodingAdventures::JsonParser::ASTNode; 1 },
    'CodingAdventures::JsonParser::ASTNode loads' );

ok( CodingAdventures::JsonParser->VERSION, 'JsonParser has a VERSION' );
ok( CodingAdventures::JsonParser::ASTNode->VERSION, 'ASTNode has a VERSION' );

done_testing;
