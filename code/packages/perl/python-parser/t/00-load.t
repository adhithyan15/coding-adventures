use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::PythonParser; 1 },
    'CodingAdventures::PythonParser loads' );

ok( eval { require CodingAdventures::PythonParser::ASTNode; 1 },
    'CodingAdventures::PythonParser::ASTNode loads' );

ok( CodingAdventures::PythonParser->VERSION, 'PythonParser has a VERSION' );
ok( CodingAdventures::PythonParser::ASTNode->VERSION, 'ASTNode has a VERSION' );

done_testing;
