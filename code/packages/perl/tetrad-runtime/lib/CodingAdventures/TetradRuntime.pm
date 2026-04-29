package CodingAdventures::TetradRuntime;
use strict;
use warnings;
use CodingAdventures::CodegenCore;
use CodingAdventures::InterpreterIr;
use CodingAdventures::JitCore;
use CodingAdventures::VmCore;

our $VERSION = '0.01';

my %KEYWORD = map { $_ => 1 } qw(fn let return);
my %OPS = ('+' => 'add', '-' => 'sub', '*' => 'mul', '/' => 'div', '%' => 'mod');

sub tokenize_tetrad {
    my ($source) = @_;
    my @tokens;
    my $i = 0;
    while ($i < length $source) {
        my $c = substr($source, $i, 1);
        if ($c =~ /\s/) { $i++; next; }
        if ($c eq '#') { $i++ while $i < length($source) && substr($source, $i, 1) ne "\n"; next; }
        if ($c eq ':' && substr($source, $i + 1, 1) eq '=') { push @tokens, { type => 'symbol', value => ':=' }; $i += 2; next; }
        if (index('+-*/%(),{}=;', $c) >= 0) { push @tokens, { type => 'symbol', value => $c }; $i++; next; }
        if ($c =~ /\d/) { my $s = $i; $i++ while $i < length($source) && substr($source, $i, 1) =~ /\d/; push @tokens, { type => 'number', value => substr($source, $s, $i - $s) }; next; }
        if ($c =~ /[A-Za-z_]/) { my $s = $i; $i++ while $i < length($source) && substr($source, $i, 1) =~ /[A-Za-z0-9_]/; my $v = substr($source, $s, $i - $s); push @tokens, { type => $KEYWORD{$v} ? 'keyword' : 'name', value => $v }; next; }
        die "unexpected Tetrad character: $c";
    }
    push @tokens, { type => 'eof', value => '' };
    return \@tokens;
}

package CodingAdventures::TetradRuntime::Parser;
use strict;
use warnings;
sub new { bless { tokens => $_[1], p => 0 }, $_[0] }
sub peek { $_[0]->{tokens}[ $_[0]->{p} + ($_[1] // 0) ] || { type => 'eof', value => '' } }
sub consume {
    my ($self, $type, $value) = @_;
    my $t = $self->peek;
    die 'expected ' . ($value // $type) . ', got ' . $t->{value} if $t->{type} ne $type || (defined($value) && $t->{value} ne $value);
    $self->{p}++;
    return $t;
}
sub match {
    my ($self, $type, $value) = @_;
    my $t = $self->peek;
    return 0 unless $t->{type} eq $type && $t->{value} eq $value;
    $self->{p}++;
    return 1;
}
sub semis { 1 while $_[0]->match(symbol => ';') }
sub parse_program {
    my ($self) = @_;
    my @forms;
    while ($self->peek->{type} ne 'eof') {
        push @forms, $self->peek->{value} eq 'fn' ? $self->parse_function : $self->parse_statement;
        $self->semis;
    }
    return { forms => \@forms };
}
sub parse_function {
    my ($self) = @_;
    $self->consume(keyword => 'fn');
    my $name = $self->consume('name')->{value};
    $self->consume(symbol => '(');
    my @params;
    if (!$self->match(symbol => ')')) {
        do { push @params, $self->consume('name')->{value} } while $self->match(symbol => ',');
        $self->consume(symbol => ')');
    }
    $self->consume(symbol => '{');
    my @body;
    while (!$self->match(symbol => '}')) { push @body, $self->parse_statement; $self->semis; }
    return { kind => 'function', name => $name, params => \@params, body => \@body };
}
sub parse_statement {
    my ($self) = @_;
    if ($self->match(keyword => 'let')) { my $name = $self->consume('name')->{value}; $self->consume(symbol => '='); return { kind => 'let', name => $name, expr => $self->expr }; }
    if ($self->match(keyword => 'return')) { return { kind => 'return', expr => $self->expr }; }
    if ($self->peek->{type} eq 'name' && ($self->peek(1)->{value} eq '=' || $self->peek(1)->{value} eq ':=')) { my $name = $self->consume('name')->{value}; $self->{p}++; return { kind => 'assign', name => $name, expr => $self->expr }; }
    return { kind => 'expr', expr => $self->expr };
}
sub expr { $_[0]->add }
sub add { my $self = shift; my $e = $self->mul; while ($self->peek->{value} eq '+' || $self->peek->{value} eq '-') { my $op = $self->consume('symbol')->{value}; $e = { kind => 'binary', left => $e, op => $op, right => $self->mul }; } return $e; }
sub mul { my $self = shift; my $e = $self->primary; while ($self->peek->{value} eq '*' || $self->peek->{value} eq '/' || $self->peek->{value} eq '%') { my $op = $self->consume('symbol')->{value}; $e = { kind => 'binary', left => $e, op => $op, right => $self->primary }; } return $e; }
sub primary {
    my ($self) = @_;
    return { kind => 'number', value => 0 + $self->consume('number')->{value} } if $self->peek->{type} eq 'number';
    if ($self->peek->{type} eq 'name') {
        my $name = $self->consume('name')->{value};
        if ($self->match(symbol => '(')) { my @args; if (!$self->match(symbol => ')')) { do { push @args, $self->expr } while $self->match(symbol => ','); $self->consume(symbol => ')'); } return { kind => 'call', name => $name, args => \@args }; }
        return { kind => 'var', name => $name };
    }
    if ($self->match(symbol => '(')) { my $e = $self->expr; $self->consume(symbol => ')'); return $e; }
    die 'expected expression, got ' . $self->peek->{value};
}

package CodingAdventures::TetradRuntime;
use strict;
use warnings;
sub parse_tetrad { CodingAdventures::TetradRuntime::Parser->new(tokenize_tetrad($_[0]))->parse_program }
sub _new_ctx { { instructions => [], n => 0 } }
sub _temp { my ($c) = @_; my $t = 't' . $c->{n}; $c->{n}++; return $t }
sub _emit { my ($c, $op, %args) = @_; push @{ $c->{instructions} }, CodingAdventures::InterpreterIr::IirInstr->of($op, %args) }
sub _compile_expr {
    my ($expr, $c) = @_;
    if ($expr->{kind} eq 'number') { my $d = _temp($c); _emit($c, 'const', dest => $d, srcs => [ $expr->{value} & 0xff ], type_hint => CodingAdventures::InterpreterIr::Types::U8); return $d; }
    return $expr->{name} if $expr->{kind} eq 'var';
    if ($expr->{kind} eq 'binary') { my $d = _temp($c); _emit($c, $OPS{$expr->{op}} || 'add', dest => $d, srcs => [ _compile_expr($expr->{left}, $c), _compile_expr($expr->{right}, $c) ], type_hint => CodingAdventures::InterpreterIr::Types::U8); return $d; }
    my $d = _temp($c); _emit($c, 'call', dest => $d, srcs => [ $expr->{name}, map { _compile_expr($_, $c) } @{ $expr->{args} } ], type_hint => CodingAdventures::InterpreterIr::Types::U8); return $d;
}
sub _compile_stmt {
    my ($stmt, $c) = @_;
    if ($stmt->{kind} eq 'let' || $stmt->{kind} eq 'assign') { _emit($c, 'tetrad.move', dest => $stmt->{name}, srcs => [ _compile_expr($stmt->{expr}, $c) ], type_hint => CodingAdventures::InterpreterIr::Types::U8); return 0; }
    if ($stmt->{kind} eq 'return') { _emit($c, 'ret', srcs => [ _compile_expr($stmt->{expr}, $c) ]); return 1; }
    _compile_expr($stmt->{expr}, $c); return 0;
}
sub _compile_function {
    my ($def) = @_;
    my $c = _new_ctx(); my $terminated = 0;
    for my $stmt (@{ $def->{body} }) { $terminated = _compile_stmt($stmt, $c) unless $terminated; }
    _emit($c, 'ret_void') unless $terminated;
    return CodingAdventures::InterpreterIr::IirFunction->new(
        name => $def->{name},
        params => [ map { { name => $_, type => CodingAdventures::InterpreterIr::Types::U8 } } @{ $def->{params} } ],
        return_type => $terminated ? CodingAdventures::InterpreterIr::Types::U8 : CodingAdventures::InterpreterIr::Types::Void,
        instructions => $c->{instructions},
        register_count => ($c->{n} + @{ $def->{params} }) > 32 ? ($c->{n} + @{ $def->{params} }) : 32,
        type_status => CodingAdventures::InterpreterIr::FunctionTypeStatus::FullyTyped,
    );
}
sub compile_tetrad {
    my ($source, $module_name) = @_;
    $module_name //= 'tetrad';
    my $program = parse_tetrad($source);
    my (@functions, @top);
    for my $form (@{ $program->{forms} }) { $form->{kind} eq 'function' ? push(@functions, _compile_function($form)) : push(@top, $form); }
    my $has_main = grep { $_->{name} eq 'main' } @functions;
    push @functions, _compile_function({ kind => 'function', name => 'main', params => [], body => \@top }) unless $has_main;
    my $mod = CodingAdventures::InterpreterIr::IirModule->new(name => $module_name, functions => \@functions, entry_point => 'main', language => 'tetrad');
    $mod->validate;
    return $mod;
}
sub run_tetrad {
    my ($source, $use_jit) = @_;
    my $mod = compile_tetrad($source);
    my $vm = CodingAdventures::VmCore::VMCore->new(u8_wrap => 1);
    return $use_jit ? CodingAdventures::JitCore::JITCore->new(vm => $vm)->execute_with_jit($mod) : $vm->execute($mod);
}
sub emit_tetrad { CodingAdventures::CodegenCore::BackendRegistry->default->compile(compile_tetrad($_[0]), $_[1]) }
1;
