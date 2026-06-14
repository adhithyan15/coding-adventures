package CodingAdventures::Brainfuck::LangVm;
use strict;
use warnings;
use CodingAdventures::InterpreterIr;
use CodingAdventures::JitCore;
use CodingAdventures::VmCore;

our $VERSION = '0.01';

sub _mutate_cell {
    my ($instructions, $op, $amount) = @_;
    push @$instructions,
        CodingAdventures::InterpreterIr::IirInstr->of('load_mem', dest => 'cell', srcs => ['ptr'], type_hint => CodingAdventures::InterpreterIr::Types::U8),
        CodingAdventures::InterpreterIr::IirInstr->of($op, dest => 'cell', srcs => ['cell', $amount], type_hint => CodingAdventures::InterpreterIr::Types::U8),
        CodingAdventures::InterpreterIr::IirInstr->of('store_mem', srcs => ['ptr', 'cell'], type_hint => CodingAdventures::InterpreterIr::Types::U8);
}

sub compile_to_iir {
    my ($source, $module_name) = @_;
    $module_name //= 'brainfuck';
    my @instructions = (CodingAdventures::InterpreterIr::IirInstr->of('const', dest => 'ptr', srcs => [0], type_hint => CodingAdventures::InterpreterIr::Types::U32));
    my @loops;
    my $loop_id = 0;
    for my $ch (split //, $source) {
        if ($ch eq '>') { push @instructions, CodingAdventures::InterpreterIr::IirInstr->of('add', dest => 'ptr', srcs => ['ptr', 1], type_hint => CodingAdventures::InterpreterIr::Types::U32); }
        elsif ($ch eq '<') { push @instructions, CodingAdventures::InterpreterIr::IirInstr->of('sub', dest => 'ptr', srcs => ['ptr', 1], type_hint => CodingAdventures::InterpreterIr::Types::U32); }
        elsif ($ch eq '+') { _mutate_cell(\@instructions, 'add', 1); }
        elsif ($ch eq '-') { _mutate_cell(\@instructions, 'sub', 1); }
        elsif ($ch eq '.') { push @instructions, CodingAdventures::InterpreterIr::IirInstr->of('load_mem', dest => 'cell', srcs => ['ptr'], type_hint => CodingAdventures::InterpreterIr::Types::U8), CodingAdventures::InterpreterIr::IirInstr->of('io_out', srcs => ['cell']); }
        elsif ($ch eq ',') { push @instructions, CodingAdventures::InterpreterIr::IirInstr->of('io_in', dest => 'cell', type_hint => CodingAdventures::InterpreterIr::Types::U8), CodingAdventures::InterpreterIr::IirInstr->of('store_mem', srcs => ['ptr', 'cell'], type_hint => CodingAdventures::InterpreterIr::Types::U8); }
        elsif ($ch eq '[') {
            my $labels = { start => "loop_${loop_id}_start", end => "loop_${loop_id}_end" }; $loop_id++; push @loops, $labels;
            push @instructions,
                CodingAdventures::InterpreterIr::IirInstr->of('label', srcs => [$labels->{start}]),
                CodingAdventures::InterpreterIr::IirInstr->of('load_mem', dest => 'cell', srcs => ['ptr'], type_hint => CodingAdventures::InterpreterIr::Types::U8),
                CodingAdventures::InterpreterIr::IirInstr->of('cmp_eq', dest => 'is_zero', srcs => ['cell', 0], type_hint => CodingAdventures::InterpreterIr::Types::Bool),
                CodingAdventures::InterpreterIr::IirInstr->of('jmp_if_true', srcs => ['is_zero', $labels->{end}]);
        }
        elsif ($ch eq ']') {
            my $labels = pop @loops or die "Unmatched ']' -- no matching '[' found";
            push @instructions, CodingAdventures::InterpreterIr::IirInstr->of('jmp', srcs => [$labels->{start}]), CodingAdventures::InterpreterIr::IirInstr->of('label', srcs => [$labels->{end}]);
        }
    }
    die "Unmatched '[' -- " . scalar(@loops) . " unclosed bracket(s)" if @loops;
    push @instructions, CodingAdventures::InterpreterIr::IirInstr->of('ret_void');
    my $mod = CodingAdventures::InterpreterIr::IirModule->new(
        name => $module_name,
        functions => [ CodingAdventures::InterpreterIr::IirFunction->new(name => 'main', return_type => CodingAdventures::InterpreterIr::Types::Void, instructions => \@instructions, register_count => 8, type_status => CodingAdventures::InterpreterIr::FunctionTypeStatus::PartiallyTyped) ],
        entry_point => 'main',
        language => 'brainfuck',
    );
    $mod->validate;
    return $mod;
}

sub execute_on_lang_vm {
    my ($source, $input, $use_jit) = @_;
    my $module = compile_to_iir($source);
    my $vm = CodingAdventures::VmCore::VMCore->new(input => ($input // ''), u8_wrap => 1);
    $use_jit ? CodingAdventures::JitCore::JITCore->new(vm => $vm)->execute_with_jit($module) : $vm->execute($module);
    return { output => $vm->{output}, memory => $vm->{memory}, vm => $vm, module => $module };
}

1;
