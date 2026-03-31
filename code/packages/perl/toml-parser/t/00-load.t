use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::TomlParser; 1 },
    'CodingAdventures::TomlParser loads' );

ok( eval { require CodingAdventures::TomlParser::ASTNode; 1 },
    'CodingAdventures::TomlParser::ASTNode loads' );

ok( CodingAdventures::TomlParser->VERSION, 'TomlParser has a VERSION' );
ok( CodingAdventures::TomlParser::ASTNode->VERSION, 'ASTNode has a VERSION' );

done_testing;
