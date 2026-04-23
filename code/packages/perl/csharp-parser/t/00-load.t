use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::CSharpParser; 1 },
    'CodingAdventures::CSharpParser loads' );

ok( eval { require CodingAdventures::CSharpParser::ASTNode; 1 },
    'CodingAdventures::CSharpParser::ASTNode loads' );

ok( CodingAdventures::CSharpParser->VERSION, 'CSharpParser has a VERSION' );
ok( CodingAdventures::CSharpParser::ASTNode->VERSION, 'ASTNode has a VERSION' );

done_testing;
