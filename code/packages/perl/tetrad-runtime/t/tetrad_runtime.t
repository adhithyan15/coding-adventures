use strict;
use warnings;
use Test2::V0;
use CodingAdventures::TetradRuntime;

is(CodingAdventures::TetradRuntime::run_tetrad('let x = 40; return x + 2;'), 42, 'runs arithmetic');
my $artifact = CodingAdventures::TetradRuntime::emit_tetrad('return 7;', 'wasm');
is($artifact->{target}, 'wasm', 'emits wasm target');
like($artifact->{body}, qr/language=tetrad/, 'artifact records source language');

done_testing();
