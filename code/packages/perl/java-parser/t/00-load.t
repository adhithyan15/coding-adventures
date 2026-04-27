use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::JavaParser; 1 },
    'CodingAdventures::JavaParser loads' );

ok( eval { require CodingAdventures::JavaParser::ASTNode; 1 },
    'CodingAdventures::JavaParser::ASTNode loads' );

ok( CodingAdventures::JavaParser->VERSION, 'JavaParser has a VERSION' );
ok( CodingAdventures::JavaParser::ASTNode->VERSION, 'ASTNode has a VERSION' );

done_testing;
