package CodingAdventures::VmCore;
use strict;
use warnings;
use CodingAdventures::InterpreterIr;

our $VERSION = '0.01';

package CodingAdventures::VmCore::BuiltinRegistry;
use strict;
use warnings;
sub new {
    my ($class, $register_defaults) = @_;
    $register_defaults = 1 unless defined $register_defaults;
    my $self = bless { handlers => {}, order => [] }, $class;
    if ($register_defaults) {
        $self->register(noop => sub { undef });
        $self->register(assert_eq => sub {
            my ($args) = @_;
            die "assert_eq failed: $args->[0] != $args->[1]" if $args->[0] ne $args->[1];
            return undef;
        });
    }
    return $self;
}
sub register {
    my ($self, $name, $handler) = @_;
    push @{ $self->{order} }, $name unless exists $self->{handlers}{$name};
    $self->{handlers}{$name} = $handler;
}
sub call {
    my ($self, $name, $args) = @_;
    die "unknown builtin: $name" unless exists $self->{handlers}{$name};
    return $self->{handlers}{$name}->($args || []);
}
sub names { [ @{ $_[0]->{order} } ] }
sub entries { [ map { [ $_, $_[0]->{handlers}{$_} ] } @{ $_[0]->{order} } ] }

package CodingAdventures::VmCore::VMFrame;
use strict;
use warnings;
sub new {
    my ($class, $fn, $args) = @_;
    $args ||= [];
    my $self = bless { fn => $fn, ip => 0, registers => {}, slots => {} }, $class;
    for my $i (0..$#{ $fn->{params} }) {
        my $param = $fn->{params}[$i];
        my $value = $args->[$i];
        $self->{registers}{ $param->{name} } = { value => $value };
        $self->{slots}{ $param->{name} } = { value => $value };
    }
    return $self;
}
sub resolve {
    my ($self, $value) = @_;
    return undef unless defined $value;
    if (!ref($value) && exists $self->{registers}{$value}) {
        return $self->{registers}{$value}{value};
    }
    if (ref($value) eq 'ARRAY') {
        return [ map { $self->resolve($_) } @$value ];
    }
    return $value;
}
sub write {
    my ($self, $name, $value) = @_;
    $self->{registers}{$name} = { value => $value } if defined $name;
}
sub load_slot {
    my ($self, $name) = @_;
    return exists $self->{slots}{$name} ? $self->{slots}{$name}{value} : undef;
}
sub store_slot {
    my ($self, $name, $value) = @_;
    $self->{slots}{$name} = { value => $value };
}

package CodingAdventures::VmCore::BranchStats;
use strict;
use warnings;
sub new { bless { taken_count => 0, not_taken_count => 0 }, shift }
sub record {
    my ($self, $taken) = @_;
    $taken ? $self->{taken_count}++ : $self->{not_taken_count}++;
}

package CodingAdventures::VmCore::VMCore;
use strict;
use warnings;
sub new {
    my ($class, %args) = @_;
    my @input = map { ord($_) & 0xff } split //, ($args{input} // '');
    return bless {
        builtins          => $args{builtins} || CodingAdventures::VmCore::BuiltinRegistry->new(1),
        memory            => {},
        io_ports          => {},
        output            => '',
        max_frames        => $args{max_frames} // $args{maxFrames} // 64,
        profiler_enabled  => exists $args{profiler_enabled} ? $args{profiler_enabled} : exists $args{profilerEnabled} ? $args{profilerEnabled} : 1,
        u8_wrap           => $args{u8_wrap} // $args{u8Wrap} // 0,
        frames            => [],
        jit_handlers      => {},
        metric_data       => {
            function_call_counts => {},
            total_instructions_executed => 0,
            total_frames_pushed => 0,
            total_jit_hits => 0,
            branch_stats => {},
            loop_back_edge_counts => {},
        },
        coverage          => {},
        coverage_enabled  => 0,
        module            => undef,
        interrupted       => 0,
        input_buffer      => \@input,
        trace             => undef,
    }, $class;
}
sub execute {
    my ($self, $mod, $fn, $args) = @_;
    $fn //= $mod->{entry_point};
    $args ||= [];
    $mod->validate;
    $self->{module} = $mod;
    $self->{interrupted} = 0;
    return $self->invoke_function($fn, $args);
}
sub execute_traced {
    my ($self, $mod, $fn, $args) = @_;
    $self->{trace} = [];
    my $result = $self->execute($mod, $fn, $args);
    my $trace = $self->{trace};
    $self->{trace} = undef;
    return { result => $result, trace => $trace };
}
sub metrics { $_[0]->{metric_data} }
sub register_builtin { $_[0]->{builtins}->register($_[1], $_[2]) }
sub register_jit_handler { $_[0]->{jit_handlers}{$_[1]} = $_[2] }
sub unregister_jit_handler { delete $_[0]->{jit_handlers}{$_[1]} }
sub interrupt { $_[0]->{interrupted} = 1 }
sub invoke_function {
    my ($self, $name, $args) = @_;
    die 'no module loaded' unless $self->{module};
    my $fn = $self->{module}->get_function($name) or die "unknown function: $name";
    $fn->{call_count}++;
    $self->{metric_data}{function_call_counts}{$name}++ if $self->{profiler_enabled};
    if (my $handler = $self->{jit_handlers}{$name}) {
        $self->{metric_data}{total_jit_hits}++;
        return $handler->($args || []);
    }
    die "maximum frame depth exceeded: $self->{max_frames}" if @{ $self->{frames} } >= $self->{max_frames};
    my $frame = CodingAdventures::VmCore::VMFrame->new($fn, $args);
    push @{ $self->{frames} }, $frame;
    $self->{metric_data}{total_frames_pushed}++;
    my $result = $self->run_frame($frame);
    pop @{ $self->{frames} };
    return $result;
}
sub run_frame {
    my ($self, $frame) = @_;
    my $labels = $frame->{fn}->label_index;
    while ($frame->{ip} < @{ $frame->{fn}{instructions} }) {
        die 'VM interrupted' if $self->{interrupted};
        my $instr = $frame->{fn}{instructions}[ $frame->{ip} ];
        $self->record_instruction($frame->{fn}{name}, $frame->{ip}, $instr);
        my $result = $self->dispatch($frame, $instr, $labels);
        return $result->{value} if $result->{kind} eq 'return';
        $frame->{ip} = $result->{kind} eq 'jump' ? $result->{ip} : $frame->{ip} + 1;
    }
    return undef;
}
sub dispatch {
    my ($self, $frame, $instr, $labels) = @_;
    my $op = $instr->{op};
    if ($op eq 'const' || $op eq 'move' || $op eq 'tetrad.move') {
        $self->write_observed($frame, $instr, $frame->resolve($instr->{srcs}[0]));
    } elsif ($op =~ /^(?:add|sub|mul|div|mod|and|or|xor|shl|shr|cmp_eq|cmp_ne|cmp_lt|cmp_le|cmp_gt|cmp_ge)$/) {
        $self->write_observed($frame, $instr, $self->binary_op($op, $frame->resolve($instr->{srcs}[0]), $frame->resolve($instr->{srcs}[1])));
    } elsif ($op eq 'neg') {
        $self->write_observed($frame, $instr, -$self->to_number($frame->resolve($instr->{srcs}[0])));
    } elsif ($op eq 'not') {
        $self->write_observed($frame, $instr, ~$self->to_number($frame->resolve($instr->{srcs}[0])));
    } elsif ($op eq 'cast') {
        $self->write_observed($frame, $instr, $self->cast($frame->resolve($instr->{srcs}[0]), $instr->{type_hint} // $instr->{srcs}[1] // CodingAdventures::InterpreterIr::Types::Any));
    } elsif ($op eq 'type_assert') {
        $self->assert_type($frame->resolve($instr->{srcs}[0]), $instr->{type_hint} // $instr->{srcs}[1] // CodingAdventures::InterpreterIr::Types::Any);
    } elsif ($op eq 'label') {
    } elsif ($op eq 'jmp') {
        return { kind => 'jump', ip => $self->jump_target($frame, $labels, $instr->{srcs}[0] // '') };
    } elsif ($op eq 'jmp_if_true' || $op eq 'jmp_if_false') {
        my $taken = $self->truthy($frame->resolve($instr->{srcs}[0]));
        $taken = !$taken if $op eq 'jmp_if_false';
        $self->record_branch($frame->{fn}{name}, $frame->{ip}, $taken);
        return { kind => 'jump', ip => $self->jump_target($frame, $labels, $instr->{srcs}[1] // '') } if $taken;
    } elsif ($op eq 'ret') {
        return { kind => 'return', value => $frame->resolve($instr->{srcs}[0]) };
    } elsif ($op eq 'ret_void') {
        return { kind => 'return', value => undef };
    } elsif ($op eq 'call') {
        my @args = map { $frame->resolve($_) } @{ $instr->{srcs} }[1..$#{ $instr->{srcs} }];
        $self->write_observed($frame, $instr, $self->invoke_function($instr->{srcs}[0] // '', \@args));
    } elsif ($op eq 'call_builtin') {
        my @args = map { $frame->resolve($_) } @{ $instr->{srcs} }[1..$#{ $instr->{srcs} }];
        $self->write_observed($frame, $instr, $self->{builtins}->call($instr->{srcs}[0] // '', \@args));
    } elsif ($op eq 'load_mem') {
        $self->write_observed($frame, $instr, $self->{memory}{ $self->to_number($frame->resolve($instr->{srcs}[0])) } // 0);
    } elsif ($op eq 'store_mem') {
        $self->{memory}{ $self->to_number($frame->resolve($instr->{srcs}[0])) } = $self->wrap_value($frame->resolve($instr->{srcs}[1]), $instr->{type_hint});
    } elsif ($op eq 'io_in') {
        my $value = @{ $self->{input_buffer} } ? shift @{ $self->{input_buffer} } : 0;
        $self->write_observed($frame, $instr, $value);
    } elsif ($op eq 'io_out') {
        my $value = $frame->resolve($instr->{srcs}[0]);
        $self->{output} .= defined($value) && $value !~ /^-?\d+$/ ? "$value" : chr($self->to_number($value) & 0xff);
    } elsif ($op eq 'is_null') {
        $self->write_observed($frame, $instr, !defined $frame->resolve($instr->{srcs}[0]) ? 1 : 0);
    } elsif ($op eq 'safepoint') {
    } else {
        die "unknown opcode: $op";
    }
    return { kind => 'next' };
}
sub record_instruction {
    my ($self, $function_name, $ip, $instr) = @_;
    $self->{metric_data}{total_instructions_executed}++;
    push @{ $self->{trace} }, { function_name => $function_name, ip => $ip, instruction => $instr->to_string } if defined $self->{trace};
}
sub record_branch {
    my ($self, $function_name, $ip, $taken) = @_;
    my $key = "$function_name:$ip";
    $self->{metric_data}{branch_stats}{$key} ||= CodingAdventures::VmCore::BranchStats->new;
    $self->{metric_data}{branch_stats}{$key}->record($taken);
}
sub jump_target {
    my ($self, $frame, $labels, $label) = @_;
    die "$frame->{fn}{name} branches to undefined label $label" unless exists $labels->{$label};
    if ($labels->{$label} < $frame->{ip}) {
        $self->{metric_data}{loop_back_edge_counts}{"$frame->{fn}{name}:$label"}++;
    }
    return $labels->{$label};
}
sub write_observed {
    my ($self, $frame, $instr, $value) = @_;
    my $wrapped = $self->wrap_value($value, $instr->{type_hint});
    $frame->write($instr->{dest}, $wrapped);
    $instr->record_observation($instr->{type_hint} // $self->runtime_type($wrapped));
}
sub binary_op {
    my ($self, $op, $left, $right) = @_;
    return $self->to_number($left) + $self->to_number($right) if $op eq 'add';
    return $self->to_number($left) - $self->to_number($right) if $op eq 'sub';
    return $self->to_number($left) * $self->to_number($right) if $op eq 'mul';
    return int($self->to_number($left) / $self->to_number($right)) if $op eq 'div';
    return $self->to_number($left) % $self->to_number($right) if $op eq 'mod';
    return $self->to_number($left) & $self->to_number($right) if $op eq 'and';
    return $self->to_number($left) | $self->to_number($right) if $op eq 'or';
    return $self->to_number($left) ^ $self->to_number($right) if $op eq 'xor';
    return $self->to_number($left) << $self->to_number($right) if $op eq 'shl';
    return $self->to_number($left) >> $self->to_number($right) if $op eq 'shr';
    return $left == $right ? 1 : 0 if $op eq 'cmp_eq';
    return $left != $right ? 1 : 0 if $op eq 'cmp_ne';
    return $self->to_number($left) < $self->to_number($right) ? 1 : 0 if $op eq 'cmp_lt';
    return $self->to_number($left) <= $self->to_number($right) ? 1 : 0 if $op eq 'cmp_le';
    return $self->to_number($left) > $self->to_number($right) ? 1 : 0 if $op eq 'cmp_gt';
    return $self->to_number($left) >= $self->to_number($right) ? 1 : 0 if $op eq 'cmp_ge';
    die "unknown opcode: $op";
}
sub cast {
    my ($self, $value, $type) = @_;
    return $self->to_number($value) & 0xff if $type eq CodingAdventures::InterpreterIr::Types::U8;
    return $self->to_number($value) & 0xffff if $type eq CodingAdventures::InterpreterIr::Types::U16;
    return $self->to_number($value) if $type eq CodingAdventures::InterpreterIr::Types::U32;
    return $self->truthy($value) ? 1 : 0 if $type eq CodingAdventures::InterpreterIr::Types::Bool;
    return "$value" if $type eq CodingAdventures::InterpreterIr::Types::Str;
    return undef if $type eq CodingAdventures::InterpreterIr::Types::Nil;
    return $value;
}
sub assert_type {
    my ($self, $value, $type) = @_;
    die "type assertion failed: expected $type, got " . $self->runtime_type($value)
        if $type ne CodingAdventures::InterpreterIr::Types::Any && $self->runtime_type($value) ne $type;
}
sub runtime_type {
    my ($self, $value) = @_;
    return CodingAdventures::InterpreterIr::Types::Nil unless defined $value;
    return CodingAdventures::InterpreterIr::Types::Bool if !ref($value) && ($value eq '0' || $value eq '1');
    return CodingAdventures::InterpreterIr::Types::U8 if !ref($value) && $value =~ /^\d+$/ && $value >= 0 && $value <= 255;
    return CodingAdventures::InterpreterIr::Types::U64 if !ref($value) && $value =~ /^-?\d+$/;
    return CodingAdventures::InterpreterIr::Types::Str if !ref($value);
    return CodingAdventures::InterpreterIr::Types::Any;
}
sub wrap_value {
    my ($self, $value, $type_hint) = @_;
    return defined($value) && $self->{u8_wrap} && defined($type_hint) && $type_hint eq CodingAdventures::InterpreterIr::Types::U8 ? ($value & 0xff) : $value;
}
sub truthy { defined $_[1] && $_[1] ne '0' && $_[1] ne '' }
sub to_number {
    my ($self, $value) = @_;
    return 0 unless defined $value;
    return 0 + $value if !ref($value) && $value =~ /^-?\d+(?:\.\d+)?$/;
    die "expected number, got $value";
}

package CodingAdventures::VmCore;
1;
