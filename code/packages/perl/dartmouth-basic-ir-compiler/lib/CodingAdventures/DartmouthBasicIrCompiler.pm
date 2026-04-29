package CodingAdventures::DartmouthBasicIrCompiler;
use strict;
use warnings;
use CodingAdventures::CodegenCore;
use CodingAdventures::InterpreterIr;
use CodingAdventures::JitCore;
use CodingAdventures::VmCore;

our $VERSION = '0.01';

my %OPS = ('+' => 'add', '-' => 'sub', '*' => 'mul', '/' => 'div');
my %CMPS = ('=' => 'cmp_eq', '<>' => 'cmp_ne', '<' => 'cmp_lt', '<=' => 'cmp_le', '>' => 'cmp_gt', '>=' => 'cmp_ge');
sub _trim { my $s = $_[0] // ''; $s =~ s/^\s+|\s+$//g; $s }
sub _label { '_line_' . $_[0] }
sub _validate_var { my $u = uc $_[0]; die "invalid BASIC variable: $_[0]" unless $u =~ /^[A-Z][A-Z0-9]?$/; $u }

sub parse_basic_lines {
    my ($source) = @_;
    my @lines;
    for my $raw (split /\r?\n/, $source) {
        my $line = _trim($raw);
        next unless length $line;
        die "missing BASIC line number: $line" unless $line =~ /^(\d+)\s*(.*)$/;
        push @lines, { number => 0 + $1, text => _trim($2) };
    }
    return [ sort { $a->{number} <=> $b->{number} } @lines ];
}
sub tokenize_basic_expr {
    my ($source) = @_;
    my @tokens; my $i = 0;
    while ($i < length $source) {
        my $c = substr($source, $i, 1);
        if ($c =~ /\s/) { $i++; next; }
        if ($c =~ /\d/) { my $s = $i; $i++ while $i < length($source) && substr($source, $i, 1) =~ /\d/; push @tokens, substr($source, $s, $i - $s); next; }
        if ($c =~ /[A-Za-z]/) { my $s = $i; $i++ while $i < length($source) && substr($source, $i, 1) =~ /[A-Za-z0-9]/; push @tokens, _validate_var(substr($source, $s, $i - $s)); next; }
        if (index('()+-*/', $c) >= 0) { push @tokens, $c; $i++; next; }
        die "unexpected BASIC expression character: $c";
    }
    return \@tokens;
}

package CodingAdventures::DartmouthBasicIrCompiler::ExprParser;
use strict;
use warnings;
sub new { bless { tokens => $_[1], p => 0 }, $_[0] }
sub peek { $_[0]->{tokens}[ $_[0]->{p} ] }
sub next { my $self = shift; die 'unexpected end of BASIC expression' unless defined $self->peek; return $self->{tokens}[ $self->{p}++ ]; }
sub expect { die 'expected ' . $_[1] unless $_[0]->next eq $_[1] }
sub parse { my $self = shift; my $e = $self->add; die 'unexpected expression token: ' . $self->peek if defined $self->peek; $e }
sub add { my $self = shift; my $e = $self->mul; while (defined($self->peek) && ($self->peek eq '+' || $self->peek eq '-')) { my $op = $self->next; $e = { kind => 'binary', op => $op, left => $e, right => $self->mul }; } $e }
sub mul { my $self = shift; my $e = $self->primary; while (defined($self->peek) && ($self->peek eq '*' || $self->peek eq '/')) { my $op = $self->next; $e = { kind => 'binary', op => $op, left => $e, right => $self->primary }; } $e }
sub primary {
    my $self = shift;
    if (defined($self->peek) && $self->peek eq '-') { $self->next; return { kind => 'binary', op => '-', left => { kind => 'number', value => 0 }, right => $self->primary }; }
    if (defined($self->peek) && $self->peek eq '(') { $self->next; my $e = $self->add; $self->expect(')'); return $e; }
    my $t = $self->next;
    return $t =~ /^\d+$/ ? { kind => 'number', value => 0 + $t } : { kind => 'var', name => CodingAdventures::DartmouthBasicIrCompiler::_validate_var($t) };
}

package CodingAdventures::DartmouthBasicIrCompiler;
use strict;
use warnings;
sub _parse_expr { CodingAdventures::DartmouthBasicIrCompiler::ExprParser->new(tokenize_basic_expr($_[0]))->parse }
sub _ctx { { instructions => [], var_names => {}, for_loops => [], n => 0 } }
sub _temp { my ($c) = @_; my $t = 't' . $c->{n}; $c->{n}++; $t }
sub _var_register { my ($c, $name) = @_; my $v = _validate_var($name); $c->{var_names}{$v} = 1; "v_$v" }
sub _emit { my ($c, $op, %args) = @_; push @{ $c->{instructions} }, CodingAdventures::InterpreterIr::IirInstr->of($op, %args) }
sub _compile_expr {
    my ($e, $c) = @_;
    if ($e->{kind} eq 'number') { my $d = _temp($c); _emit($c, 'const', dest => $d, srcs => [$e->{value}], type_hint => CodingAdventures::InterpreterIr::Types::U64); return $d; }
    return _var_register($c, $e->{name}) if $e->{kind} eq 'var';
    my $d = _temp($c); _emit($c, $OPS{$e->{op}} || 'add', dest => $d, srcs => [_compile_expr($e->{left}, $c), _compile_expr($e->{right}, $c)], type_hint => CodingAdventures::InterpreterIr::Types::U64); $d;
}
sub _split_condition {
    my ($condition) = @_;
    for my $op ('<=', '>=', '<>', '=', '<', '>') {
        my $i = index($condition, $op);
        return (_trim(substr($condition, 0, $i)), $op, _trim(substr($condition, $i + length($op)))) if $i >= 0;
    }
    die "missing comparison operator: $condition";
}
sub _compile_print {
    my ($rest, $c) = @_;
    if ($rest eq '') { my $d = _temp($c); _emit($c, 'const', dest => $d, srcs => [''], type_hint => CodingAdventures::InterpreterIr::Types::Str); _emit($c, 'call_builtin', srcs => ['__basic_print', $d], type_hint => CodingAdventures::InterpreterIr::Types::Nil); return; }
    if ($rest =~ /^"(.*)"$/) { my $d = _temp($c); _emit($c, 'const', dest => $d, srcs => [$1], type_hint => CodingAdventures::InterpreterIr::Types::Str); _emit($c, 'call_builtin', srcs => ['__basic_print', $d], type_hint => CodingAdventures::InterpreterIr::Types::Nil); return; }
    _emit($c, 'call_builtin', srcs => ['__basic_print', _compile_expr(_parse_expr($rest), $c)], type_hint => CodingAdventures::InterpreterIr::Types::Nil);
}
sub _compile_if {
    my ($rest, $c) = @_;
    my $then = index(uc($rest), 'THEN'); die 'IF requires THEN' if $then < 0;
    my ($left, $op, $right) = _split_condition(_trim(substr($rest, 0, $then)));
    my $target = 0 + _trim(substr($rest, $then + 4));
    my $d = _temp($c);
    _emit($c, $CMPS{$op} || 'cmp_eq', dest => $d, srcs => [_compile_expr(_parse_expr($left), $c), _compile_expr(_parse_expr($right), $c)], type_hint => CodingAdventures::InterpreterIr::Types::Bool);
    _emit($c, 'jmp_if_true', srcs => [$d, _label($target)]);
}
sub _compile_assignment {
    my ($text, $c) = @_;
    my $body = $text; $body = _trim(substr($body, 4)) if uc(substr($body, 0, 4)) eq 'LET ';
    my $eq = index($body, '='); die "expected assignment: $text" if $eq < 0;
    _emit($c, 'move', dest => _var_register($c, _trim(substr($body, 0, $eq))), srcs => [_compile_expr(_parse_expr(_trim(substr($body, $eq + 1))), $c)], type_hint => CodingAdventures::InterpreterIr::Types::U64);
}
sub _compile_for {
    my ($line_number, $rest, $c) = @_;
    my $eq = index($rest, '='); die 'FOR requires =' if $eq < 0;
    my $variable = _validate_var(_trim(substr($rest, 0, $eq)));
    my $after_eq = _trim(substr($rest, $eq + 1));
    my $to = index(uc($after_eq), ' TO '); die 'FOR requires TO' if $to < 0;
    my $start_text = _trim(substr($after_eq, 0, $to));
    my $after_to = _trim(substr($after_eq, $to + 4));
    my $step = index(uc($after_to), ' STEP ');
    my $limit_text = $step < 0 ? $after_to : _trim(substr($after_to, 0, $step));
    my $step_text = $step < 0 ? '1' : _trim(substr($after_to, $step + 6));
    my $var_reg = _var_register($c, $variable);
    _emit($c, 'move', dest => $var_reg, srcs => [_compile_expr(_parse_expr($start_text), $c)], type_hint => CodingAdventures::InterpreterIr::Types::U64);
    my $label = "for_${line_number}_" . scalar(@{ $c->{for_loops} });
    _emit($c, 'label', srcs => [$label]);
    push @{ $c->{for_loops} }, { variable => $variable, label => $label, limit => _compile_expr(_parse_expr($limit_text), $c), step => _compile_expr(_parse_expr($step_text), $c), descending => $step_text =~ /^\s*-/ ? 1 : 0 };
}
sub _compile_next {
    my ($rest, $c) = @_;
    my $expected = _trim($rest) eq '' ? undef : _validate_var(_trim($rest));
    my $loop = pop @{ $c->{for_loops} } or die 'NEXT without FOR';
    die "NEXT $expected does not match FOR $loop->{variable}" if defined($expected) && $expected ne $loop->{variable};
    my $reg = _var_register($c, $loop->{variable}); _emit($c, 'add', dest => $reg, srcs => [$reg, $loop->{step}], type_hint => CodingAdventures::InterpreterIr::Types::U64);
    my $keep = _temp($c); _emit($c, $loop->{descending} ? 'cmp_ge' : 'cmp_le', dest => $keep, srcs => [$reg, $loop->{limit}], type_hint => CodingAdventures::InterpreterIr::Types::Bool); _emit($c, 'jmp_if_true', srcs => [$keep, $loop->{label}]);
}
sub _compile_line {
    my ($line, $c) = @_;
    my $text = _trim($line->{text}); my $upper = uc $text;
    return if $text eq '' || substr($upper, 0, 3) eq 'REM';
    if ($upper eq 'END' || $upper eq 'STOP') { _emit($c, 'ret_void'); return; }
    if (substr($upper, 0, 5) eq 'PRINT') { _compile_print(_trim(substr($text, 5)), $c); return; }
    if (substr($upper, 0, 4) eq 'GOTO') { _emit($c, 'jmp', srcs => [_label(0 + _trim(substr($text, 4)))]); return; }
    if (substr($upper, 0, 2) eq 'IF') { _compile_if(_trim(substr($text, 2)), $c); return; }
    if (substr($upper, 0, 3) eq 'FOR') { _compile_for($line->{number}, _trim(substr($text, 3)), $c); return; }
    if (substr($upper, 0, 4) eq 'NEXT') { _compile_next(_trim(substr($text, 4)), $c); return; }
    _compile_assignment($text, $c);
}
sub compile_dartmouth_basic {
    my ($source, $module_name) = @_;
    $module_name //= 'dartmouth-basic';
    my $c = _ctx();
    for my $line (@{ parse_basic_lines($source) }) { _emit($c, 'label', srcs => [_label($line->{number})]); _compile_line($line, $c); }
    _emit($c, 'ret_void');
    my @vars = sort keys %{ $c->{var_names} };
    my $reg_count = ($c->{n} + @vars) > 64 ? ($c->{n} + @vars) : 64;
    my $module = CodingAdventures::InterpreterIr::IirModule->new(name => $module_name, functions => [ CodingAdventures::InterpreterIr::IirFunction->new(name => 'main', return_type => CodingAdventures::InterpreterIr::Types::Void, instructions => $c->{instructions}, register_count => $reg_count, type_status => CodingAdventures::InterpreterIr::FunctionTypeStatus::PartiallyTyped) ], entry_point => 'main', language => 'dartmouth-basic');
    $module->validate;
    return { module => $module, var_names => \@vars };
}
sub run_dartmouth_basic {
    my ($source, $use_jit) = @_;
    my $compiled = compile_dartmouth_basic($source); my $output = '';
    my $vm = CodingAdventures::VmCore::VMCore->new;
    $vm->register_builtin('__basic_print' => sub { $output .= (defined $_[0][0] ? "$_[0][0]" : '') . "\n"; undef });
    $use_jit ? CodingAdventures::JitCore::JITCore->new(vm => $vm)->execute_with_jit($compiled->{module}) : $vm->execute($compiled->{module});
    return $output;
}
sub emit_dartmouth_basic { CodingAdventures::CodegenCore::BackendRegistry->default->compile(compile_dartmouth_basic($_[0])->{module}, $_[1]) }
1;
