use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::NibParser; 1 }, 'module loads' );

my ($ast, $err) = CodingAdventures::NibParser->parse('fn main() { return 0; }');
ok(!$err, 'parsed successfully');
ok($ast, 'got ast');
is($ast->rule_name, 'program', 'root rule name is program');

done_testing;
