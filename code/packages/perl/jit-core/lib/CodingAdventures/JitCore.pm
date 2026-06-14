package CodingAdventures::JitCore;
use strict;
use warnings;
use CodingAdventures::CodegenCore;
use CodingAdventures::InterpreterIr;
use CodingAdventures::VmCore;

our $VERSION = '0.01';

package CodingAdventures::JitCore::PureVmBackend;
use strict;
use warnings;
sub new { bless {}, shift }
sub compile_callable {
    my ($self, $fn, $mod, $source_vm) = @_;
    my $builtins = CodingAdventures::VmCore::BuiltinRegistry->new(0);
    for my $entry (@{ $source_vm->{builtins}->entries }) {
        $builtins->register($entry->[0], $entry->[1]);
    }
    return sub {
        my ($args) = @_;
        my $child = CodingAdventures::VmCore::VMCore->new(
            builtins => $builtins,
            profiler_enabled => 0,
            u8_wrap => $fn->{return_type} eq CodingAdventures::InterpreterIr::Types::U8,
        );
        return $child->execute($mod, $fn->{name}, $args || []);
    };
}

package CodingAdventures::JitCore::JITCore;
use strict;
use warnings;
sub new {
    my ($class, %args) = @_;
    return bless {
        vm => $args{vm},
        backend => $args{backend} || CodingAdventures::JitCore::PureVmBackend->new,
        registry => $args{registry} || CodingAdventures::CodegenCore::BackendRegistry->default,
    }, $class;
}
sub execute_with_jit {
    my ($self, $mod, $fn, $args) = @_;
    $self->compile_ready_functions($mod);
    return $self->{vm}->execute($mod, $fn // $mod->{entry_point}, $args || []);
}
sub compile_ready_functions {
    my ($self, $mod) = @_;
    my @compiled;
    for my $fn (@{ $mod->{functions} }) {
        next unless $self->should_compile($fn);
        $self->{vm}->register_jit_handler($fn->{name}, $self->{backend}->compile_callable($fn, $mod, $self->{vm}));
        push @compiled, $fn->{name};
    }
    return \@compiled;
}
sub emit { $_[0]->{registry}->compile($_[1], $_[2]) }
sub should_compile {
    my ($self, $fn) = @_;
    return 1 if $fn->{type_status} eq CodingAdventures::InterpreterIr::FunctionTypeStatus::FullyTyped;
    return $fn->{call_count} >= 10 if $fn->{type_status} eq CodingAdventures::InterpreterIr::FunctionTypeStatus::PartiallyTyped;
    return $fn->{call_count} >= 100;
}

package CodingAdventures::JitCore;
1;
