use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::SqlParser; 1 },
    'CodingAdventures::SqlParser loads' );

ok( eval { require CodingAdventures::SqlParser::ASTNode; 1 },
    'CodingAdventures::SqlParser::ASTNode loads' );

ok( CodingAdventures::SqlParser->VERSION, 'SqlParser has a VERSION' );
ok( CodingAdventures::SqlParser::ASTNode->VERSION, 'ASTNode has a VERSION' );

done_testing;
