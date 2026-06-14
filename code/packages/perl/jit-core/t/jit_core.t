use strict;
use warnings;
use Test2::V0;
use CodingAdventures::InterpreterIr;
use CodingAdventures::JitCore;
use CodingAdventures::VmCore;

my $fn = CodingAdventures::InterpreterIr::IirFunction->new(
    name => 'main',
    return_type => CodingAdventures::InterpreterIr::Types::U8,
    instructions => [
        CodingAdventures::InterpreterIr::IirInstr->of('const', dest => 'x', srcs => [7], type_hint => CodingAdventures::InterpreterIr::Types::U8),
        CodingAdventures::InterpreterIr::IirInstr->of('ret', srcs => ['x']),
    ],
);
my $mod = CodingAdventures::InterpreterIr::IirModule->new(name => 'jit', functions => [$fn], entry_point => 'main');
my $jit = CodingAdventures::JitCore::JITCore->new(vm => CodingAdventures::VmCore::VMCore->new(u8_wrap => 1));
is($jit->execute_with_jit($mod), 7, 'runs via pure VM JIT backend');

done_testing();
