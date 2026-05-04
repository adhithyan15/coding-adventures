use strict;
use warnings;
use Test2::V0;
use CodingAdventures::DartmouthBasicIrCompiler;

is(CodingAdventures::DartmouthBasicIrCompiler::run_dartmouth_basic("10 LET A = 40\n20 PRINT A + 2\n30 END"), "42\n", 'runs BASIC through LANG VM');
my $artifact = CodingAdventures::DartmouthBasicIrCompiler::emit_dartmouth_basic('10 END', 'clr');
is($artifact->{target}, 'clr', 'emits clr target');
like($artifact->{body}, qr/dartmouth-basic/, 'artifact records language');

done_testing();
