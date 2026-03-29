use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::StarlarkParser; 1 },
    'CodingAdventures::StarlarkParser loads' );

ok( eval { require CodingAdventures::StarlarkParser::ASTNode; 1 },
    'CodingAdventures::StarlarkParser::ASTNode loads' );

ok( CodingAdventures::StarlarkParser->VERSION, 'StarlarkParser has a VERSION' );
ok( CodingAdventures::StarlarkParser::ASTNode->VERSION, 'ASTNode has a VERSION' );

done_testing;
