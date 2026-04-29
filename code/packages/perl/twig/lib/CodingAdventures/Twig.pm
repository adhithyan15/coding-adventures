package CodingAdventures::Twig;
use strict;
use warnings;
use CodingAdventures::CodegenCore;
use CodingAdventures::InterpreterIr;
use CodingAdventures::JitCore;
use CodingAdventures::VmCore;

our $VERSION = '0.01';

package CodingAdventures::Twig::SymbolRef;
use strict;
use warnings;
sub new { bless { name => $_[1] }, $_[0] }

package CodingAdventures::Twig;
use strict;
use warnings;
my %BUILTIN = map { $_ => 1 } qw(+ - * / = < > cons car cdr null? pair? number? print);

sub _is_symbol { ref($_[0]) && ref($_[0]) eq 'CodingAdventures::Twig::SymbolRef' }
sub _symbol_name { die 'expected symbol' unless _is_symbol($_[0]); $_[0]->{name} }
sub _symbol_name_or_undef { _is_symbol($_[0]) ? $_[0]->{name} : undef }

sub tokenize_twig {
    my ($source) = @_;
    my @tokens;
    my $i = 0;
    while ($i < length $source) {
        my $c = substr($source, $i, 1);
        if ($c =~ /\s/) { $i++; next; }
        if ($c eq ';') { $i++ while $i < length($source) && substr($source, $i, 1) ne "\n"; next; }
        if ($c eq '(' || $c eq ')') { push @tokens, $c; $i++; next; }
        my $s = $i;
        $i++ while $i < length($source) && substr($source, $i, 1) !~ /\s/ && substr($source, $i, 1) ne '(' && substr($source, $i, 1) ne ')';
        push @tokens, substr($source, $s, $i - $s);
    }
    return \@tokens;
}

package CodingAdventures::Twig::Parser;
use strict;
use warnings;
sub new { bless { tokens => $_[1], p => 0 }, $_[0] }
sub peek { $_[0]->{tokens}[ $_[0]->{p} ] }
sub next { my $self = shift; die 'unexpected end of Twig source' unless defined $self->peek; return $self->{tokens}[ $self->{p}++ ]; }
sub forms { my $self = shift; my @forms; push @forms, $self->expr while $self->{p} < @{ $self->{tokens} }; return \@forms; }
sub expr {
    my ($self) = @_;
    my $t = $self->next;
    if ($t eq '(') { my @list; while (!defined($self->peek) || $self->peek ne ')') { die 'unterminated Twig list' unless defined $self->peek; push @list, $self->expr; } $self->next; return \@list; }
    die 'unexpected )' if $t eq ')';
    return 0 + $t if $t =~ /^-?\d+$/;
    return 1 if $t eq '#t';
    return 0 if $t eq '#f';
    return undef if $t eq 'nil';
    return CodingAdventures::Twig::SymbolRef->new($t);
}

package CodingAdventures::Twig;
use strict;
use warnings;
sub parse_twig { CodingAdventures::Twig::Parser->new(tokenize_twig($_[0]))->forms }
sub _ctx { { instructions => [], n => 0, labels => 0 } }
sub _temp { my ($c) = @_; my $t = 't' . $c->{n}; $c->{n}++; return $t }
sub _label { my ($c, $prefix) = @_; my $l = $prefix . '_' . $c->{labels}; $c->{labels}++; return $l }
sub _emit { my ($c, $op, %args) = @_; push @{ $c->{instructions} }, CodingAdventures::InterpreterIr::IirInstr->of($op, %args) }
sub _copy_locals { +{ %{ $_[0] } } }

sub _compile_begin {
    my ($exprs, $c, $locals) = @_;
    my $r;
    $r = _compile_expr($_, $c, $locals) for @$exprs;
    if (defined $r) { return $r; }
    my $d = _temp($c); _emit($c, 'const', dest => $d, srcs => [], type_hint => CodingAdventures::InterpreterIr::Types::Nil); return $d;
}
sub _compile_if {
    my ($expr, $c, $locals) = @_;
    my $cond = _compile_expr($expr->[1], $c, $locals);
    my ($else, $end, $d) = (_label($c, 'else'), _label($c, 'endif'), _temp($c));
    _emit($c, 'jmp_if_false', srcs => [$cond, $else]);
    _emit($c, 'move', dest => $d, srcs => [_compile_expr($expr->[2], $c, $locals)], type_hint => CodingAdventures::InterpreterIr::Types::Any);
    _emit($c, 'jmp', srcs => [$end]);
    _emit($c, 'label', srcs => [$else]);
    _emit($c, 'move', dest => $d, srcs => [_compile_expr($expr->[3], $c, $locals)], type_hint => CodingAdventures::InterpreterIr::Types::Any);
    _emit($c, 'label', srcs => [$end]);
    return $d;
}
sub _compile_let {
    my ($expr, $c, $locals) = @_;
    my $bindings = $expr->[1];
    die 'let requires a binding list' unless ref($bindings) eq 'ARRAY';
    my $next = _copy_locals($locals);
    for my $b (@$bindings) {
        die 'let binding must be a pair' unless ref($b) eq 'ARRAY' && @$b == 2;
        my $name = _symbol_name($b->[0]);
        _emit($c, 'move', dest => $name, srcs => [_compile_expr($b->[1], $c, $next)], type_hint => CodingAdventures::InterpreterIr::Types::Any);
        $next->{$name} = 1;
    }
    return _compile_begin([ @$expr[2..$#$expr] ], $c, $next);
}
sub _compile_expr {
    my ($expr, $c, $locals) = @_;
    if (_is_symbol($expr)) {
        return $expr->{name} if $locals->{ $expr->{name} };
        my $d = _temp($c); _emit($c, 'call_builtin', dest => $d, srcs => ['global_get', $expr->{name}], type_hint => CodingAdventures::InterpreterIr::Types::Any); return $d;
    }
    if (!ref($expr)) {
        my $d = _temp($c);
        my @srcs = defined($expr) ? ($expr) : ();
        my $type = !defined($expr) ? CodingAdventures::InterpreterIr::Types::Nil : ($expr =~ /^-?\d+$/ ? CodingAdventures::InterpreterIr::Types::U64 : CodingAdventures::InterpreterIr::Types::Any);
        _emit($c, 'const', dest => $d, srcs => \@srcs, type_hint => $type);
        return $d;
    }
    if (@$expr == 0) { my $d = _temp($c); _emit($c, 'const', dest => $d, srcs => [], type_hint => CodingAdventures::InterpreterIr::Types::Nil); return $d; }
    my $head = $expr->[0];
    return _compile_if($expr, $c, $locals) if _is_symbol($head) && $head->{name} eq 'if';
    return _compile_begin([ @$expr[1..$#$expr] ], $c, $locals) if _is_symbol($head) && $head->{name} eq 'begin';
    return _compile_let($expr, $c, $locals) if _is_symbol($head) && $head->{name} eq 'let';
    die 'Twig applications require a symbol in operator position' unless _is_symbol($head);
    my @srcs = ($head->{name}, map { _compile_expr($_, $c, $locals) } @$expr[1..$#$expr]);
    my $d = _temp($c);
    _emit($c, $BUILTIN{$head->{name}} ? 'call_builtin' : 'call', dest => $d, srcs => \@srcs, type_hint => CodingAdventures::InterpreterIr::Types::Any);
    return $d;
}
sub _is_fn_define { ref($_[0]) eq 'ARRAY' && (_symbol_name_or_undef($_[0][0]) // '') eq 'define' && ref($_[0][1]) eq 'ARRAY' }
sub _is_value_define { ref($_[0]) eq 'ARRAY' && (_symbol_name_or_undef($_[0][0]) // '') eq 'define' && _is_symbol($_[0][1]) }
sub _compile_fn {
    my ($form) = @_;
    my $sig = $form->[1]; die 'function define requires a signature list' unless ref($sig) eq 'ARRAY' && @$sig;
    my $name = _symbol_name($sig->[0]);
    my (@params, %locals);
    for my $p (@$sig[1..$#$sig]) { my $name = _symbol_name($p); push @params, { name => $name, type => CodingAdventures::InterpreterIr::Types::Any }; $locals{$name} = 1; }
    my $c = _ctx(); my $result;
    $result = _compile_expr($_, $c, \%locals) for @$form[2..$#$form];
    if (!defined $result) { $result = _temp($c); _emit($c, 'const', dest => $result, srcs => [], type_hint => CodingAdventures::InterpreterIr::Types::Nil); }
    _emit($c, 'ret', srcs => [$result]);
    return CodingAdventures::InterpreterIr::IirFunction->new(name => $name, params => \@params, return_type => CodingAdventures::InterpreterIr::Types::Any, instructions => $c->{instructions}, register_count => $c->{n} > 64 ? $c->{n} : 64, type_status => CodingAdventures::InterpreterIr::FunctionTypeStatus::Untyped);
}
sub compile_twig {
    my ($source, $module_name) = @_;
    $module_name //= 'twig';
    my ($forms, @functions, @body) = (parse_twig($source));
    my $main = _ctx();
    for my $form (@$forms) {
        if (_is_fn_define($form)) { push @functions, _compile_fn($form); }
        elsif (_is_value_define($form)) { _emit($main, 'call_builtin', srcs => ['global_set', _symbol_name($form->[1]), _compile_expr($form->[2], $main, {})], type_hint => CodingAdventures::InterpreterIr::Types::Any); }
        else { push @body, $form; }
    }
    my $last; $last = _compile_expr($_, $main, {}) for @body;
    if (!defined $last) { $last = _temp($main); _emit($main, 'const', dest => $last, srcs => [], type_hint => CodingAdventures::InterpreterIr::Types::Nil); }
    _emit($main, 'ret', srcs => [$last]);
    push @functions, CodingAdventures::InterpreterIr::IirFunction->new(name => 'main', return_type => CodingAdventures::InterpreterIr::Types::Any, instructions => $main->{instructions}, register_count => $main->{n} > 64 ? $main->{n} : 64, type_status => CodingAdventures::InterpreterIr::FunctionTypeStatus::Untyped);
    my $mod = CodingAdventures::InterpreterIr::IirModule->new(name => $module_name, functions => \@functions, entry_point => 'main', language => 'twig');
    $mod->validate;
    return $mod;
}
sub _to_number { die 'expected number, got ' . format_twig_value($_[0]) unless defined($_[0]) && !ref($_[0]) && $_[0] =~ /^-?\d+$/; 0 + $_[0] }
sub _is_pair { ref($_[0]) eq 'ARRAY' && ($_[0][0] // '') eq 'cons' }
sub _as_pair { die 'expected pair, got ' . format_twig_value($_[0]) unless _is_pair($_[0]); $_[0] }
sub format_twig_value {
    my ($value) = @_;
    return 'nil' unless defined $value;
    return '(' . format_twig_value($value->[1]) . ' . ' . format_twig_value($value->[2]) . ')' if _is_pair($value);
    return "$value";
}
sub install_twig_builtins {
    my ($vm, $globals, $write) = @_;
    $vm->register_builtin('+' => sub { my $s = 0; $s += _to_number($_) for @{ $_[0] }; $s });
    $vm->register_builtin('-' => sub { my $a = $_[0]; my $r = _to_number($a->[0] // 0); return -$r if @$a == 1; $r -= _to_number($_) for @$a[1..$#$a]; $r });
    $vm->register_builtin('*' => sub { my $p = 1; $p *= _to_number($_) for @{ $_[0] }; $p });
    $vm->register_builtin('/' => sub { my $a = $_[0]; my $r = _to_number($a->[0] // 0); $r = int($r / _to_number($_)) for @$a[1..$#$a]; $r });
    $vm->register_builtin('=' => sub { $_[0][0] == $_[0][1] ? 1 : 0 });
    $vm->register_builtin('<' => sub { _to_number($_[0][0] // 0) < _to_number($_[0][1] // 0) ? 1 : 0 });
    $vm->register_builtin('>' => sub { _to_number($_[0][0] // 0) > _to_number($_[0][1] // 0) ? 1 : 0 });
    $vm->register_builtin(cons => sub { [ 'cons', $_[0][0], $_[0][1] ] });
    $vm->register_builtin(car => sub { _as_pair($_[0][0])->[1] });
    $vm->register_builtin(cdr => sub { _as_pair($_[0][0])->[2] });
    $vm->register_builtin('null?' => sub { defined($_[0][0]) ? 0 : 1 });
    $vm->register_builtin('pair?' => sub { _is_pair($_[0][0]) ? 1 : 0 });
    $vm->register_builtin('number?' => sub { defined($_[0][0]) && !ref($_[0][0]) && $_[0][0] =~ /^-?\d+$/ ? 1 : 0 });
    $vm->register_builtin(print => sub { $write->(format_twig_value($_[0][0]) . "\n"); undef });
    $vm->register_builtin(global_get => sub { my $name = "$_[0][0]"; die "undefined global: $name" unless exists $globals->{$name}; $globals->{$name} });
    $vm->register_builtin(global_set => sub { $globals->{"$_[0][0]"} = $_[0][1]; $_[0][1] });
    $vm->register_builtin(_move => sub { $_[0][0] });
}
sub run_twig_detailed {
    my ($source, $use_jit) = @_;
    my $module = compile_twig($source); my %globals; my $stdout = '';
    my $vm = CodingAdventures::VmCore::VMCore->new;
    install_twig_builtins($vm, \%globals, sub { $stdout .= $_[0] });
    my $value = $use_jit ? CodingAdventures::JitCore::JITCore->new(vm => $vm)->execute_with_jit($module) : $vm->execute($module);
    return { stdout => $stdout, value => $value, module => $module, vm => $vm };
}
sub run_twig { my $r = run_twig_detailed(@_); return ($r->{stdout}, $r->{value}); }
sub emit_twig { CodingAdventures::CodegenCore::BackendRegistry->default->compile(compile_twig($_[0]), $_[1]) }
1;
