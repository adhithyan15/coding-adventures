use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::VhdlParser; 1 },
    'CodingAdventures::VhdlParser loads' );

ok( eval { require CodingAdventures::VhdlParser::ASTNode; 1 },
    'CodingAdventures::VhdlParser::ASTNode loads' );

ok( CodingAdventures::VhdlParser->VERSION, 'VhdlParser has a VERSION' );
ok( CodingAdventures::VhdlParser::ASTNode->VERSION, 'ASTNode has a VERSION' );

done_testing;
