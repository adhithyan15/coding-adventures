use strict;
use warnings;
use Test2::V0;
use CodingAdventures::CodegenCore;
use CodingAdventures::InterpreterIr;

my $fn = CodingAdventures::InterpreterIr::IirFunction->new(
    name => 'main',
    return_type => CodingAdventures::InterpreterIr::Types::Void,
    instructions => [ CodingAdventures::InterpreterIr::IirInstr->of('ret_void') ],
);
my $mod = CodingAdventures::InterpreterIr::IirModule->new(name => 'demo', functions => [$fn], entry_point => 'main', language => 'demo');
my $artifact = CodingAdventures::CodegenCore::BackendRegistry->default->compile($mod, 'wasm');
is($artifact->{target}, 'wasm', 'target recorded');
like($artifact->{body}, qr/\.function main/, 'function emitted');

done_testing();
