use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::RubyParser; 1 },
    'CodingAdventures::RubyParser loads' );

ok( eval { require CodingAdventures::RubyParser::ASTNode; 1 },
    'CodingAdventures::RubyParser::ASTNode loads' );

ok( CodingAdventures::RubyParser->VERSION, 'RubyParser has a VERSION' );
ok( CodingAdventures::RubyParser::ASTNode->VERSION, 'ASTNode has a VERSION' );

done_testing;
