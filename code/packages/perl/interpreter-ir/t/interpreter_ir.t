use strict;
use warnings;
use Test2::V0;
use CodingAdventures::InterpreterIr;

my $slot = CodingAdventures::InterpreterIr::SlotState->new;
$slot->record('u8')->record('str');
is($slot->{kind}, CodingAdventures::InterpreterIr::SlotKind::Polymorphic, 'slot becomes polymorphic');
ok($slot->is_polymorphic, 'polymorphic predicate');

my $fn = CodingAdventures::InterpreterIr::IirFunction->new(
    name => 'main',
    return_type => CodingAdventures::InterpreterIr::Types::U8,
    instructions => [
        CodingAdventures::InterpreterIr::IirInstr->of('const', dest => 'x', srcs => [1], type_hint => CodingAdventures::InterpreterIr::Types::U8),
        CodingAdventures::InterpreterIr::IirInstr->of('label', srcs => ['done']),
        CodingAdventures::InterpreterIr::IirInstr->of('ret', srcs => ['x']),
    ],
);
my $mod = CodingAdventures::InterpreterIr::IirModule->new(name => 'test', functions => [$fn], entry_point => 'main');
ok(lives { $mod->validate }, 'module validates');
is($fn->{type_status}, CodingAdventures::InterpreterIr::FunctionTypeStatus::FullyTyped, 'function is typed');

done_testing();
