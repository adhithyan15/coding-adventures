package CodingAdventures::InterpreterIr;
use strict;
use warnings;

our $VERSION = '0.01';

package CodingAdventures::InterpreterIr::Types;
use strict;
use warnings;
use constant {
    U8           => 'u8',
    U16          => 'u16',
    U32          => 'u32',
    U64          => 'u64',
    Bool         => 'bool',
    Str          => 'str',
    Nil          => 'nil',
    Void         => 'void',
    Any          => 'any',
    Polymorphic  => 'polymorphic',
};
sub is_ref { defined $_[0] && $_[0] =~ /^ref<.*>$/ }
sub unwrap_ref { is_ref($_[0]) ? substr($_[0], 4, -1) : $_[0] }
sub ref_type { 'ref<' . $_[0] . '>' }
sub is_concrete { defined $_[0] && $_[0] ne Any && $_[0] ne Polymorphic }

package CodingAdventures::InterpreterIr::SlotKind;
use strict;
use warnings;
use constant {
    Uninitialized => 'uninitialized',
    Monomorphic   => 'monomorphic',
    Polymorphic   => 'polymorphic',
    Megamorphic   => 'megamorphic',
};

package CodingAdventures::InterpreterIr::SlotState;
use strict;
use warnings;
sub new { bless { observations => {}, order => [], kind => CodingAdventures::InterpreterIr::SlotKind::Uninitialized, count => 0 }, shift }
sub record {
    my ($self, $runtime_type) = @_;
    $self->{count}++;
    if (!exists $self->{observations}{$runtime_type}) {
        $self->{observations}{$runtime_type} = 0;
        push @{ $self->{order} }, $runtime_type;
    }
    $self->{observations}{$runtime_type}++;
    my $unique = scalar @{ $self->{order} };
    $self->{kind} = $unique == 1 ? CodingAdventures::InterpreterIr::SlotKind::Monomorphic
        : $unique <= 4 ? CodingAdventures::InterpreterIr::SlotKind::Polymorphic
        : CodingAdventures::InterpreterIr::SlotKind::Megamorphic;
    return $self;
}
sub observed_types { [ @{ $_[0]->{order} } ] }
sub is_monomorphic { $_[0]->{kind} eq CodingAdventures::InterpreterIr::SlotKind::Monomorphic }
sub is_polymorphic { $_[0]->{kind} eq CodingAdventures::InterpreterIr::SlotKind::Polymorphic }

package CodingAdventures::InterpreterIr::FunctionTypeStatus;
use strict;
use warnings;
use constant {
    FullyTyped     => 'fully_typed',
    PartiallyTyped => 'partially_typed',
    Untyped        => 'untyped',
};

package CodingAdventures::InterpreterIr::IirInstr;
use strict;
use warnings;
my %VALUE_OP = map { $_ => 1 } qw(
    add sub mul div mod neg and or xor not shl shr cmp_eq cmp_ne cmp_lt cmp_le cmp_gt cmp_ge
    const load_reg load_mem call call_builtin io_in cast alloc box unbox field_load is_null tetrad.move move
);
sub new {
    my ($class, %args) = @_;
    die 'IirInstr requires op' unless defined $args{op};
    return bless {
        op                => $args{op},
        dest              => $args{dest},
        srcs              => $args{srcs} || [],
        type_hint         => $args{type_hint} // $args{typeHint},
        observed_type     => $args{observed_type} // $args{observedType},
        observation_count => $args{observation_count} // $args{observationCount} // 0,
        observed_slot     => $args{observed_slot} // $args{observedSlot},
        deopt_anchor      => $args{deopt_anchor} // $args{deoptAnchor},
        may_alloc         => $args{may_alloc} // $args{mayAlloc} // 0,
    }, $class;
}
sub of {
    my ($class, $op, %args) = @_;
    return $class->new(%args, op => $op);
}
sub typed { CodingAdventures::InterpreterIr::Types::is_concrete($_[0]->{type_hint}) }
sub has_observation { defined $_[0]->{observed_type} || $_[0]->{observation_count} > 0 || defined $_[0]->{observed_slot} }
sub polymorphic { defined $_[0]->{observed_slot} && $_[0]->{observed_slot}->is_polymorphic }
sub effective_type { $_[0]->{type_hint} // $_[0]->{observed_type} }
sub record_observation {
    my ($self, $runtime_type, $slot) = @_;
    $self->{observed_type} = $runtime_type;
    $self->{observation_count}++;
    $self->{observed_slot} = $slot->record($runtime_type) if defined $slot;
    return $self;
}
sub is_value_opcode { $VALUE_OP{$_[0]} }
sub to_string {
    my ($self) = @_;
    my $dest = defined $self->{dest} ? "$self->{dest} = " : '';
    my @args = map { defined($_) ? ($_ =~ /^-?\d+(?:\.\d+)?$/ ? $_ : '"' . $_ . '"') : 'nil' } @{ $self->{srcs} };
    my $type = defined $self->effective_type ? ' : ' . $self->effective_type : '';
    return $dest . $self->{op} . '(' . join(', ', @args) . ')' . $type;
}

package CodingAdventures::InterpreterIr::IirFunction;
use strict;
use warnings;
sub new {
    my ($class, %args) = @_;
    die 'IirFunction requires name' unless defined $args{name};
    my $self = bless {
        name           => $args{name},
        params         => $args{params} || [],
        return_type    => $args{return_type} // $args{returnType} // CodingAdventures::InterpreterIr::Types::Any,
        instructions   => $args{instructions} || [],
        register_count => $args{register_count} // $args{registerCount} // 0,
        type_status    => $args{type_status} // $args{typeStatus},
        call_count     => $args{call_count} // $args{callCount} // 0,
        feedback_slots => $args{feedback_slots} // $args{feedbackSlots} // {},
        source_map     => $args{source_map} // $args{sourceMap} // {},
    }, $class;
    $self->{type_status} //= $self->infer_type_status;
    return $self;
}
sub param_names { [ map { $_->{name} } @{ $_[0]->{params} } ] }
sub param_types { [ map { $_->{type} } @{ $_[0]->{params} } ] }
sub infer_type_status {
    my ($self) = @_;
    my $signature_typed = CodingAdventures::InterpreterIr::Types::is_concrete($self->{return_type});
    for my $param (@{ $self->{params} }) {
        $signature_typed &&= CodingAdventures::InterpreterIr::Types::is_concrete($param->{type});
    }
    my ($values, $typed_values) = (0, 0);
    for my $instr (@{ $self->{instructions} }) {
        next unless CodingAdventures::InterpreterIr::IirInstr::is_value_opcode($instr->{op});
        $values++;
        $typed_values++ if $instr->typed;
    }
    return CodingAdventures::InterpreterIr::FunctionTypeStatus::FullyTyped if $signature_typed && $typed_values == $values;
    return CodingAdventures::InterpreterIr::FunctionTypeStatus::PartiallyTyped if $signature_typed || $typed_values > 0;
    return CodingAdventures::InterpreterIr::FunctionTypeStatus::Untyped;
}
sub label_index {
    my ($self) = @_;
    my %labels;
    for my $i (0..$#{ $self->{instructions} }) {
        my $instr = $self->{instructions}[$i];
        if ($instr->{op} eq 'label') {
            my $label = defined $instr->{srcs}[0] ? "$instr->{srcs}[0]" : ($instr->{dest} // '');
            $labels{$label} = $i if length $label;
        }
    }
    return \%labels;
}

package CodingAdventures::InterpreterIr::IirModule;
use strict;
use warnings;
sub new {
    my ($class, %args) = @_;
    die 'IirModule requires name' unless defined $args{name};
    return bless {
        name        => $args{name},
        functions   => $args{functions} || [],
        entry_point => $args{entry_point} // $args{entryPoint} // 'main',
        language    => $args{language} // 'unknown',
        metadata    => $args{metadata} || {},
    }, $class;
}
sub get_function {
    my ($self, $name) = @_;
    for my $fn (@{ $self->{functions} }) {
        return $fn if $fn->{name} eq $name;
    }
    return undef;
}
sub function_names { [ map { $_->{name} } @{ $_[0]->{functions} } ] }
sub add_or_replace {
    my ($self, $fn) = @_;
    for my $i (0..$#{ $self->{functions} }) {
        if ($self->{functions}[$i]{name} eq $fn->{name}) {
            $self->{functions}[$i] = $fn;
            return;
        }
    }
    push @{ $self->{functions} }, $fn;
}
sub validate {
    my ($self) = @_;
    my %seen;
    for my $fn (@{ $self->{functions} }) {
        die "duplicate function: $fn->{name}" if $seen{$fn->{name}}++;
    }
    die "missing entry point: $self->{entry_point}" unless $seen{$self->{entry_point}};
    for my $fn (@{ $self->{functions} }) {
        my $labels = $fn->label_index;
        for my $i (0..$#{ $fn->{instructions} }) {
            my $instr = $fn->{instructions}[$i];
            next unless $instr->{op} eq 'jmp' || $instr->{op} eq 'jmp_if_true' || $instr->{op} eq 'jmp_if_false';
            my $label = $instr->{op} eq 'jmp' ? ($instr->{srcs}[0] // '') : ($instr->{srcs}[1] // '');
            die "$fn->{name}:$i branches to undefined label $label" unless exists $labels->{$label};
        }
    }
    return 1;
}

package CodingAdventures::InterpreterIr;
1;
