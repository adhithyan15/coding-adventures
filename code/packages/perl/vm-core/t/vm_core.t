use strict;
use warnings;
use Test2::V0;
use CodingAdventures::InterpreterIr;
use CodingAdventures::VmCore;

my $fn = CodingAdventures::InterpreterIr::IirFunction->new(
    name => 'main',
    return_type => CodingAdventures::InterpreterIr::Types::U64,
    instructions => [
        CodingAdventures::InterpreterIr::IirInstr->of('const', dest => 'a', srcs => [20], type_hint => CodingAdventures::InterpreterIr::Types::U64),
        CodingAdventures::InterpreterIr::IirInstr->of('const', dest => 'b', srcs => [22], type_hint => CodingAdventures::InterpreterIr::Types::U64),
        CodingAdventures::InterpreterIr::IirInstr->of('add', dest => 'c', srcs => ['a', 'b'], type_hint => CodingAdventures::InterpreterIr::Types::U64),
        CodingAdventures::InterpreterIr::IirInstr->of('ret', srcs => ['c']),
    ],
);
my $mod = CodingAdventures::InterpreterIr::IirModule->new(name => 'arith', functions => [$fn], entry_point => 'main');
is(CodingAdventures::VmCore::VMCore->new->execute($mod), 42, 'executes arithmetic');

done_testing();
