use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::VerilogParser; 1 },
    'CodingAdventures::VerilogParser loads' );

ok( eval { require CodingAdventures::VerilogParser::ASTNode; 1 },
    'CodingAdventures::VerilogParser::ASTNode loads' );

ok( CodingAdventures::VerilogParser->VERSION, 'VerilogParser has a VERSION' );
ok( CodingAdventures::VerilogParser::ASTNode->VERSION, 'ASTNode has a VERSION' );

done_testing;
