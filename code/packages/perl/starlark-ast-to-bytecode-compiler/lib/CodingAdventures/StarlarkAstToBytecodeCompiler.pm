package CodingAdventures::StarlarkAstToBytecodeCompiler;

# ============================================================================
# CodingAdventures::StarlarkAstToBytecodeCompiler — Pure Perl Starlark compiler
# ============================================================================
#
# # What is this module?
#
# This module compiles Starlark AST nodes (produced by StarlarkParser) into
# flat bytecode instruction sequences for a stack-based virtual machine.
#
# # The Compilation Pipeline
#
#   Starlark source code
#       │ (StarlarkLexer)
#   Token stream
#       │ (StarlarkParser)
#   AST (hashrefs with rule_name and children)
#       │ (THIS MODULE)
#   CodeObject (bytecode instructions + constant pool + name pool)
#       │ (Starlark VM)
#   Execution result
#
# # How a Stack VM Works
#
# A stack VM maintains two things:
#   1. A program counter (PC) — index of current instruction
#   2. An operand stack — values waiting to be used
#
# Arithmetic example: "1 + 2"
#
#   Stack before: []
#   LOAD_CONST 0    ; push constants[0] = 1   → stack: [1]
#   LOAD_CONST 1    ; push constants[1] = 2   → stack: [1, 2]
#   ADD             ; pop 2, pop 1, push 3    → stack: [3]
#   STORE_NAME 0    ; pop 3, store in x       → stack: []
#
# # Opcode Categories
#
#   0x01-0x06  Stack operations
#   0x10-0x15  Variable operations
#   0x20-0x2D  Arithmetic & bitwise
#   0x30-0x38  Comparison & logical
#   0x40-0x44  Control flow (jumps)
#   0x50-0x53  Functions
#   0x60-0x64  Collections
#   0x70-0x74  Subscript & attributes
#   0x80-0x82  Iteration
#   0x90-0x91  Module
#   0xA0       I/O
#   0xFF       VM control (halt)
#
# ============================================================================

use strict;
use warnings;
use Carp qw(croak);

our $VERSION = '0.01';

# ============================================================================
# Opcode Constants
# ============================================================================

use constant {
    # Stack Operations (0x0_)
    OP_LOAD_CONST          => 0x01,
    OP_POP                 => 0x02,
    OP_DUP                 => 0x03,
    OP_LOAD_NONE           => 0x04,
    OP_LOAD_TRUE           => 0x05,
    OP_LOAD_FALSE          => 0x06,

    # Variable Operations (0x1_)
    OP_STORE_NAME          => 0x10,
    OP_LOAD_NAME           => 0x11,
    OP_STORE_LOCAL         => 0x12,
    OP_LOAD_LOCAL          => 0x13,
    OP_STORE_CLOSURE       => 0x14,
    OP_LOAD_CLOSURE        => 0x15,

    # Arithmetic (0x2_)
    OP_ADD                 => 0x20,
    OP_SUB                 => 0x21,
    OP_MUL                 => 0x22,
    OP_DIV                 => 0x23,
    OP_FLOOR_DIV           => 0x24,
    OP_MOD                 => 0x25,
    OP_POWER               => 0x26,
    OP_NEGATE              => 0x27,
    OP_BIT_AND             => 0x28,
    OP_BIT_OR              => 0x29,
    OP_BIT_XOR             => 0x2A,
    OP_BIT_NOT             => 0x2B,
    OP_LSHIFT              => 0x2C,
    OP_RSHIFT              => 0x2D,

    # Comparison & Boolean (0x3_)
    OP_CMP_EQ              => 0x30,
    OP_CMP_NE              => 0x31,
    OP_CMP_LT              => 0x32,
    OP_CMP_GT              => 0x33,
    OP_CMP_LE              => 0x34,
    OP_CMP_GE              => 0x35,
    OP_CMP_IN              => 0x36,
    OP_CMP_NOT_IN          => 0x37,
    OP_LOGICAL_NOT         => 0x38,

    # Control Flow (0x4_)
    OP_JUMP                => 0x40,
    OP_JUMP_IF_FALSE       => 0x41,
    OP_JUMP_IF_TRUE        => 0x42,
    OP_JUMP_IF_FALSE_OR_POP => 0x43,
    OP_JUMP_IF_TRUE_OR_POP  => 0x44,

    # Functions (0x5_)
    OP_MAKE_FUNCTION       => 0x50,
    OP_CALL_FUNCTION       => 0x51,
    OP_CALL_FUNCTION_KW    => 0x52,
    OP_RETURN              => 0x53,

    # Collections (0x6_)
    OP_BUILD_LIST          => 0x60,
    OP_BUILD_DICT          => 0x61,
    OP_BUILD_TUPLE         => 0x62,
    OP_LIST_APPEND         => 0x63,
    OP_DICT_SET            => 0x64,

    # Subscript & Attribute (0x7_)
    OP_LOAD_SUBSCRIPT      => 0x70,
    OP_STORE_SUBSCRIPT     => 0x71,
    OP_LOAD_ATTR           => 0x72,
    OP_STORE_ATTR          => 0x73,
    OP_LOAD_SLICE          => 0x74,

    # Iteration (0x8_)
    OP_GET_ITER            => 0x80,
    OP_FOR_ITER            => 0x81,
    OP_UNPACK_SEQUENCE     => 0x82,

    # Module (0x9_)
    OP_LOAD_MODULE         => 0x90,
    OP_IMPORT_FROM         => 0x91,

    # I/O (0xA_)
    OP_PRINT               => 0xA0,

    # VM Control (0xFF)
    OP_HALT                => 0xFF,
};

# ============================================================================
# Operator-to-Opcode Maps
# ============================================================================

my %BINARY_OP_MAP = (
    '+'  => OP_ADD,
    '-'  => OP_SUB,
    '*'  => OP_MUL,
    '/'  => OP_DIV,
    '//' => OP_FLOOR_DIV,
    '%'  => OP_MOD,
    '**' => OP_POWER,
    '&'  => OP_BIT_AND,
    '|'  => OP_BIT_OR,
    '^'  => OP_BIT_XOR,
    '<<' => OP_LSHIFT,
    '>>' => OP_RSHIFT,
);

my %COMPARE_OP_MAP = (
    '=='     => OP_CMP_EQ,
    '!='     => OP_CMP_NE,
    '<'      => OP_CMP_LT,
    '>'      => OP_CMP_GT,
    '<='     => OP_CMP_LE,
    '>='     => OP_CMP_GE,
    'in'     => OP_CMP_IN,
    'not in' => OP_CMP_NOT_IN,
);

my %AUGMENTED_ASSIGN_MAP = (
    '+='   => OP_ADD,
    '-='   => OP_SUB,
    '*='   => OP_MUL,
    '/='   => OP_DIV,
    '//='  => OP_FLOOR_DIV,
    '%='   => OP_MOD,
    '&='   => OP_BIT_AND,
    '|='   => OP_BIT_OR,
    '^='   => OP_BIT_XOR,
    '<<='  => OP_LSHIFT,
    '>>='  => OP_RSHIFT,
    '**='  => OP_POWER,
);

my %UNARY_OP_MAP = (
    '-' => OP_NEGATE,
    '~' => OP_BIT_NOT,
);

# Export maps for testing
sub binary_op_map   { \%BINARY_OP_MAP }
sub compare_op_map  { \%COMPARE_OP_MAP }
sub augmented_assign_map { \%AUGMENTED_ASSIGN_MAP }
sub unary_op_map    { \%UNARY_OP_MAP }

# ============================================================================
# Data Constructors
# ============================================================================

# instruction($opcode, $operand) → hashref
sub instruction {
    my ($opcode, $operand) = @_;
    return { opcode => $opcode, operand => $operand };
}

# code_object(\@instructions, \@constants, \@names) → hashref
sub code_object {
    my ($instructions, $constants, $names) = @_;
    return {
        instructions => $instructions // [],
        constants    => $constants    // [],
        names        => $names        // [],
    };
}

# token_node($type, $value) → hashref (for tests)
sub token_node {
    my ($type, $value) = @_;
    return { node_kind => 'token', type => $type, value => $value };
}

# ast_node($rule_name, \@children) → hashref (for tests)
sub ast_node {
    my ($rule_name, $children) = @_;
    return { node_kind => 'ast', rule_name => $rule_name, children => $children // [] };
}

# ============================================================================
# Compiler State
# ============================================================================
#
# The compiler walks the AST recursively, calling rule handlers for each node.
# State is stored in the compiler object:
#   - instructions: array of { opcode, operand } hashrefs
#   - constants:    deduplicated constant pool (strings, numbers, code objects)
#   - names:        deduplicated name pool (variable/function names)

sub new {
    my ($class) = @_;
    return bless {
        instructions => [],
        constants    => [],
        names        => [],
        _handlers    => {},
    }, $class;
}

# ============================================================================
# Instruction Emission
# ============================================================================

sub emit {
    my ($self, $opcode, $operand) = @_;
    push @{$self->{instructions}}, instruction($opcode, $operand);
    return $#{$self->{instructions}};  # 0-based index
}

sub emit_jump {
    my ($self, $opcode) = @_;
    return $self->emit($opcode, 0);  # placeholder target
}

sub patch_jump {
    my ($self, $index, $target) = @_;
    $target //= scalar @{$self->{instructions}};
    $self->{instructions}[$index]{operand} = $target;
}

sub current_offset {
    my ($self) = @_;
    return scalar @{$self->{instructions}};
}

# ============================================================================
# Pool Management
# ============================================================================

sub add_constant {
    my ($self, $value) = @_;
    my $pool = $self->{constants};
    for my $i (0 .. $#$pool) {
        # Use string equality for dedup (covers numbers and strings)
        return $i if defined $pool->[$i] && defined $value
            && ref($pool->[$i]) eq ref($value)
            && $pool->[$i] eq $value;
    }
    push @$pool, $value;
    return $#$pool;
}

sub add_name {
    my ($self, $name) = @_;
    my $pool = $self->{names};
    for my $i (0 .. $#$pool) {
        return $i if $pool->[$i] eq $name;
    }
    push @$pool, $name;
    return $#$pool;
}

# ============================================================================
# CodeObject export
# ============================================================================

sub to_code_object {
    my ($self) = @_;
    return code_object(
        [@{$self->{instructions}}],
        [@{$self->{constants}}],
        [@{$self->{names}}],
    );
}

# ============================================================================
# Rule Handler Registration & Dispatch
# ============================================================================

sub register_rule {
    my ($self, $rule_name, $handler) = @_;
    $self->{_handlers}{$rule_name} = $handler;
}

sub compile_node {
    my ($self, $node) = @_;
    return unless defined $node;

    if ($node->{node_kind} eq 'token') {
        # Tokens are handled by their parent's handler; no-op here.
        return;
    }

    croak "compile_node: unexpected node_kind '$node->{node_kind}'"
        unless $node->{node_kind} eq 'ast';

    my $handler = $self->{_handlers}{$node->{rule_name}};
    if ($handler) {
        $handler->($self, $node);
        return;
    }

    # Pass-through: single child, no handler.
    my @children = @{$node->{children} // []};
    if (@children == 1) {
        $self->compile_node($children[0]);
        return;
    }

    # Multiple children with no handler — compile non-token children in order.
    if (@children > 1) {
        for my $child (@children) {
            $self->compile_node($child) if $child->{node_kind} ne 'token';
        }
        return;
    }
}

# ============================================================================
# Top-Level Compilation
# ============================================================================

sub compile {
    my ($self, $ast) = @_;
    $self->compile_node($ast);
    $self->emit(OP_HALT);
    return $self->to_code_object();
}

# ============================================================================
# Compiler Factory — creates a compiler with all Starlark handlers
# ============================================================================

sub create_compiler {
    my ($class) = @_;
    my $self = $class->new();
    $self->_register_all_handlers();
    return $self;
}

sub _register_all_handlers {
    my ($self) = @_;

    # -- file / suite / wrappers ------------------------------------------

    $self->register_rule('file', sub {
        my ($c, $node) = @_;
        for my $child (@{$node->{children}}) {
            $c->compile_node($child);
        }
    });

    $self->register_rule('suite', sub {
        my ($c, $node) = @_;
        for my $child (@{$node->{children}}) {
            next if $child->{node_kind} eq 'token';
            $c->compile_node($child);
        }
    });

    for my $rule (qw(statement simple_stmt small_stmt compound_stmt)) {
        $self->register_rule($rule, sub {
            my ($c, $node) = @_;
            _pass_through($c, $node);
        });
    }

    # -- expression_stmt --------------------------------------------------
    $self->register_rule('expression_stmt', sub {
        my ($c, $node) = @_;
        $c->compile_node($node->{children}[0]);
        $c->emit(OP_POP);
    });

    # -- assign_stmt: x = expr ------------------------------------------
    $self->register_rule('assign_stmt', sub {
        my ($c, $node) = @_;
        my @ch = @{$node->{children}};
        my $rhs = $ch[-1];
        $c->compile_node($rhs);
        # Walk backwards, skip "=" tokens
        my $i = $#ch - 2;
        while ($i >= 0) {
            my $target = $ch[$i];
            _compile_store($c, $target);
            $i -= 2;
        }
    });

    # -- augmented_assign_stmt: x += expr --------------------------------
    $self->register_rule('augmented_assign_stmt', sub {
        my ($c, $node) = @_;
        my @ch = @{$node->{children}};
        my $lhs = $ch[0];
        my $rhs = $ch[-1];
        # Find operator token
        my $op_val = '';
        for my $child (@ch) {
            if ($child->{node_kind} eq 'token' && $child->{value} ne '') {
                next if $child->{value} eq $child->{value} && !defined $BINARY_OP_MAP{$child->{value}} && !defined $AUGMENTED_ASSIGN_MAP{$child->{value}};
                $op_val = $child->{value} if $AUGMENTED_ASSIGN_MAP{$child->{value}};
                last if $op_val;
            }
        }
        _compile_load($c, $lhs);
        $c->compile_node($rhs);
        my $opcode = $AUGMENTED_ASSIGN_MAP{$op_val};
        $c->emit($opcode) if defined $opcode;
        _compile_store($c, $lhs);
    });

    # -- return_stmt -----------------------------------------------------
    $self->register_rule('return_stmt', sub {
        my ($c, $node) = @_;
        my $has_expr = 0;
        for my $child (@{$node->{children}}) {
            next if $child->{node_kind} eq 'token';
            $c->compile_node($child);
            $has_expr = 1;
            last;
        }
        $c->emit(OP_LOAD_NONE) unless $has_expr;
        $c->emit(OP_RETURN);
    });

    # -- pass_stmt -------------------------------------------------------
    $self->register_rule('pass_stmt', sub { });

    # -- break/continue --------------------------------------------------
    $self->register_rule('break_stmt',    sub { $_[0]->emit_jump(OP_JUMP) });
    $self->register_rule('continue_stmt', sub { $_[0]->emit_jump(OP_JUMP) });

    # -- if_stmt ---------------------------------------------------------
    $self->register_rule('if_stmt', sub {
        my ($c, $node) = @_;
        _compile_if_chain($c, $node->{children}, 0);
    });

    $self->register_rule('elif_clause', sub {
        my ($c, $node) = @_;
        _compile_if_chain($c, $node->{children}, 0);
    });

    $self->register_rule('else_clause', sub {
        my ($c, $node) = @_;
        my $suite = _find_ast_child($node->{children}, 'suite')
                 // $node->{children}[-1];
        $c->compile_node($suite) if $suite;
    });

    # -- for_stmt --------------------------------------------------------
    $self->register_rule('for_stmt', sub {
        my ($c, $node) = @_;
        my @ch = @{$node->{children}};
        my $target   = _find_ast_child(\@ch, 'identifier');
        my $suite    = _find_ast_child(\@ch, 'suite');
        my $iterable = _find_iterable_in_for(\@ch);

        $c->compile_node($iterable) if $iterable;
        $c->emit(OP_GET_ITER);

        my $loop_start   = $c->current_offset();
        my $for_iter_idx = $c->emit_jump(OP_FOR_ITER);

        _compile_store($c, $target) if $target;
        $c->compile_node($suite)    if $suite;
        $c->emit(OP_JUMP, $loop_start);
        $c->patch_jump($for_iter_idx);
    });

    # -- load_stmt -------------------------------------------------------
    $self->register_rule('load_stmt', sub {
        my ($c, $node) = @_;
        my $first_str = _find_first_string($node->{children});
        if (defined $first_str) {
            my $idx = $c->add_constant($first_str);
            $c->emit(OP_LOAD_CONST, $idx);
            $c->emit(OP_LOAD_MODULE, 0);
        }
        my $sym_count = 0;
        for my $child (@{$node->{children}}) {
            next unless ref $child eq 'HASH' && $child->{node_kind} eq 'ast'
                     && $child->{rule_name} eq 'argument';
            my $name_val = _get_string_value($child);
            if (defined $name_val) {
                my $ni = $c->add_name($name_val);
                $c->emit(OP_IMPORT_FROM, $ni);
                $c->emit(OP_STORE_NAME, $ni);
            }
            $sym_count++;
        }
        $c->emit(OP_POP) unless $sym_count;
    });

    # -- def_stmt --------------------------------------------------------
    $self->register_rule('def_stmt', sub {
        my ($c, $node) = @_;
        my @ch = @{$node->{children}};
        my $func_name = _find_token_type(\@ch, 'NAME') // 'anonymous';
        my $param_list = _find_ast_child(\@ch, 'param_list');
        my $suite      = _find_ast_child(\@ch, 'suite');

        my @params = $param_list ? _collect_params($param_list) : ();

        # Create a nested compiler for the function body
        my $body_c = __PACKAGE__->new();
        # Copy handlers
        $body_c->{_handlers} = { %{$c->{_handlers}} };

        if ($suite) {
            for my $child (@{$suite->{children}}) {
                next if $child->{node_kind} eq 'token';
                $body_c->compile_node($child);
            }
        }
        $body_c->emit(OP_LOAD_NONE);
        $body_c->emit(OP_RETURN);

        my $func_co = $body_c->to_code_object();
        my $co_idx  = $c->add_constant($func_co);
        $c->emit(OP_LOAD_CONST, $co_idx);
        $c->emit(OP_MAKE_FUNCTION, 0);

        my $name_idx = $c->add_name($func_name);
        $c->emit(OP_STORE_NAME, $name_idx);
    });

    # -- param / param_list ----------------------------------------------
    $self->register_rule('param_list', sub { _pass_through($_[0], $_[1]) });
    $self->register_rule('param', sub {
        my ($c, $node) = @_;
        my $name = _find_token_type($node->{children}, 'NAME');
        if (defined $name) {
            my $idx = $c->add_name($name);
            $c->emit(OP_LOAD_NAME, $idx);
        }
    });

    # -- expressions (pass-through wrappers) ----------------------------
    for my $rule (qw(expr expression)) {
        $self->register_rule($rule, sub { _pass_through($_[0], $_[1]) });
    }

    # -- or_expr ---------------------------------------------------------
    $self->register_rule('or_expr', sub {
        my ($c, $node) = @_;
        my @ch = @{$node->{children}};
        if (@ch == 1) { $c->compile_node($ch[0]); return; }
        $c->compile_node($ch[0]);
        my $j = $c->emit_jump(OP_JUMP_IF_TRUE_OR_POP);
        $c->compile_node($ch[2]);
        $c->patch_jump($j);
    });

    # -- and_expr --------------------------------------------------------
    $self->register_rule('and_expr', sub {
        my ($c, $node) = @_;
        my @ch = @{$node->{children}};
        if (@ch == 1) { $c->compile_node($ch[0]); return; }
        $c->compile_node($ch[0]);
        my $j = $c->emit_jump(OP_JUMP_IF_FALSE_OR_POP);
        $c->compile_node($ch[2]);
        $c->patch_jump($j);
    });

    # -- not_expr --------------------------------------------------------
    $self->register_rule('not_expr', sub {
        my ($c, $node) = @_;
        my @ch = @{$node->{children}};
        if (@ch == 1) { $c->compile_node($ch[0]); return; }
        $c->compile_node($ch[1]);
        $c->emit(OP_LOGICAL_NOT);
    });

    # -- comparison ------------------------------------------------------
    $self->register_rule('comparison', sub {
        my ($c, $node) = @_;
        my @ch = @{$node->{children}};
        if (@ch == 1) { $c->compile_node($ch[0]); return; }
        $c->compile_node($ch[0]);
        $c->compile_node($ch[-1]);
        my $op = _get_compare_op(\@ch);
        my $opcode = $COMPARE_OP_MAP{$op};
        $c->emit($opcode) if defined $opcode;
    });

    # -- arith / term / shift / bitwise_* --------------------------------
    for my $rule (qw(arith term shift bitwise_and bitwise_xor bitwise_or)) {
        $self->register_rule($rule, sub {
            my ($c, $node) = @_;
            my @ch = @{$node->{children}};
            if (@ch == 1) { $c->compile_node($ch[0]); return; }
            _compile_binary_chain($c, \@ch);
        });
    }

    # -- factor / unary --------------------------------------------------
    for my $rule (qw(factor unary)) {
        $self->register_rule($rule, sub {
            my ($c, $node) = @_;
            my @ch = @{$node->{children}};
            if (@ch == 1) { $c->compile_node($ch[0]); return; }
            $c->compile_node($ch[1]);
            my $op_val = $ch[0]{value} // '';
            my $opcode = $UNARY_OP_MAP{$op_val};
            $c->emit($opcode) if defined $opcode;
        });
    }

    # -- power_expr ------------------------------------------------------
    $self->register_rule('power_expr', sub {
        my ($c, $node) = @_;
        my @ch = @{$node->{children}};
        if (@ch == 1) { $c->compile_node($ch[0]); return; }
        $c->compile_node($ch[0]);
        $c->compile_node($ch[2]);
        $c->emit(OP_POWER);
    });

    # -- primary ---------------------------------------------------------
    $self->register_rule('primary', sub {
        my ($c, $node) = @_;
        my @ch = @{$node->{children}};
        if (@ch == 1) { $c->compile_node($ch[0]); return; }
        $c->compile_node($ch[0]);
        my $second = $ch[1];
        if ($second && $second->{node_kind} eq 'token') {
            if ($second->{value} eq '.') {
                my $attr = $ch[2]{value} // '';
                my $idx  = $c->add_name($attr);
                $c->emit(OP_LOAD_ATTR, $idx);
            } elsif ($second->{value} eq '[') {
                $c->compile_node($ch[2]);
                $c->emit(OP_LOAD_SUBSCRIPT);
            }
        }
    });

    # -- call ------------------------------------------------------------
    $self->register_rule('call', sub {
        my ($c, $node) = @_;
        my @ch = @{$node->{children}};
        $c->compile_node($ch[0]);
        my $args_node  = _find_ast_child(\@ch, 'call_args');
        my $arg_count  = 0;
        if ($args_node) {
            for my $child (@{$args_node->{children}}) {
                next unless ref $child eq 'HASH' && $child->{node_kind} eq 'ast';
                $c->compile_node($child);
                $arg_count++;
            }
        }
        $c->emit(OP_CALL_FUNCTION, $arg_count);
    });

    $self->register_rule('call_args', sub { _pass_through($_[0], $_[1]) });

    $self->register_rule('argument', sub {
        my ($c, $node) = @_;
        for my $child (@{$node->{children}}) {
            if (ref $child eq 'HASH' && $child->{node_kind} eq 'ast') {
                $c->compile_node($child);
                last;
            }
        }
    });

    # -- dot_access ------------------------------------------------------
    $self->register_rule('dot_access', sub {
        my ($c, $node) = @_;
        $c->compile_node($node->{children}[0]);
        my $attr = _find_token_type($node->{children}, 'NAME') // '';
        my $idx  = $c->add_name($attr);
        $c->emit(OP_LOAD_ATTR, $idx);
    });

    # -- subscript -------------------------------------------------------
    $self->register_rule('subscript', sub {
        my ($c, $node) = @_;
        $c->compile_node($node->{children}[0]);
        $c->compile_node($node->{children}[2]);
        $c->emit(OP_LOAD_SUBSCRIPT);
    });

    # -- slice -----------------------------------------------------------
    $self->register_rule('slice', sub {
        my ($c, $node) = @_;
        $c->emit(OP_LOAD_SLICE, 0);
    });

    # -- atom (primitive literals) ---------------------------------------
    $self->register_rule('atom', sub {
        my ($c, $node) = @_;
        my @ch = @{$node->{children}};
        if (@ch == 1) {
            my $child = $ch[0];
            if ($child->{node_kind} eq 'token') {
                my $t = $child->{type};
                my $v = $child->{value};
                if ($t eq 'NAME') {
                    if ($v eq 'True' or $v eq 'true') {
                        $c->emit(OP_LOAD_TRUE);
                    } elsif ($v eq 'False' or $v eq 'false') {
                        $c->emit(OP_LOAD_FALSE);
                    } elsif ($v eq 'None' or $v eq 'nil') {
                        $c->emit(OP_LOAD_NONE);
                    } else {
                        my $idx = $c->add_name($v);
                        $c->emit(OP_LOAD_NAME, $idx);
                    }
                } elsif ($t eq 'INT' or $t eq 'FLOAT') {
                    my $idx = $c->add_constant($v + 0);
                    $c->emit(OP_LOAD_CONST, $idx);
                } elsif ($t eq 'STRING') {
                    my $idx = $c->add_constant(_strip_quotes($v));
                    $c->emit(OP_LOAD_CONST, $idx);
                }
            } else {
                $c->compile_node($child);
            }
        } elsif (@ch == 3) {
            # parenthesized: ( expr )
            $c->compile_node($ch[1]);
        } else {
            _pass_through($c, $node);
        }
    });

    # -- identifier ------------------------------------------------------
    $self->register_rule('identifier', sub {
        my ($c, $node) = @_;
        my $name = _get_token_value($node->{children});
        if (defined $name) {
            my $idx = $c->add_name($name);
            $c->emit(OP_LOAD_NAME, $idx);
        }
    });

    # -- number ----------------------------------------------------------
    $self->register_rule('number', sub {
        my ($c, $node) = @_;
        my $tok = _find_first_token($node->{children});
        if ($tok) {
            my $idx = $c->add_constant($tok->{value} + 0);
            $c->emit(OP_LOAD_CONST, $idx);
        }
    });

    # -- string_node -----------------------------------------------------
    $self->register_rule('string_node', sub {
        my ($c, $node) = @_;
        my $tok = _find_first_token($node->{children});
        if ($tok) {
            my $idx = $c->add_constant(_strip_quotes($tok->{value}));
            $c->emit(OP_LOAD_CONST, $idx);
        }
    });

    # -- list_expr -------------------------------------------------------
    $self->register_rule('list_expr', sub {
        my ($c, $node) = @_;
        my $count = 0;
        for my $child (@{$node->{children}}) {
            next unless ref $child eq 'HASH' && $child->{node_kind} eq 'ast';
            $c->compile_node($child);
            $count++;
        }
        $c->emit(OP_BUILD_LIST, $count);
    });

    # -- dict_expr -------------------------------------------------------
    $self->register_rule('dict_expr', sub {
        my ($c, $node) = @_;
        my $count = 0;
        for my $child (@{$node->{children}}) {
            next unless ref $child eq 'HASH' && $child->{node_kind} eq 'ast'
                     && $child->{rule_name} eq 'dict_entry';
            $c->compile_node($child);
            $count++;
        }
        $c->emit(OP_BUILD_DICT, $count);
    });

    # -- dict_entry: key: value ------------------------------------------
    $self->register_rule('dict_entry', sub {
        my ($c, $node) = @_;
        my ($key_expr, $val_expr);
        my $found_colon = 0;
        for my $child (@{$node->{children}}) {
            if (ref $child eq 'HASH') {
                if ($child->{node_kind} eq 'token' && $child->{value} eq ':') {
                    $found_colon = 1;
                } elsif ($child->{node_kind} eq 'ast') {
                    if (!$found_colon) { $key_expr = $child }
                    else              { $val_expr = $child }
                }
            }
        }
        $c->compile_node($key_expr) if $key_expr;
        $c->compile_node($val_expr) if $val_expr;
    });

    # -- tuple_expr ------------------------------------------------------
    $self->register_rule('tuple_expr', sub {
        my ($c, $node) = @_;
        my $count = 0;
        for my $child (@{$node->{children}}) {
            next unless ref $child eq 'HASH' && $child->{node_kind} eq 'ast';
            $c->compile_node($child);
            $count++;
        }
        $c->emit(OP_BUILD_TUPLE, $count);
    });

    # -- lambda_expr -----------------------------------------------------
    $self->register_rule('lambda_expr', sub {
        my ($c, $node) = @_;
        my @ch = @{$node->{children}};
        my $param_list = _find_ast_child(\@ch, 'param_list');
        my $body;
        for my $i (reverse 0 .. $#ch) {
            my $child = $ch[$i];
            if (ref $child eq 'HASH' && $child->{node_kind} eq 'ast'
                && ($child->{rule_name} // '') ne 'param_list') {
                $body = $child;
                last;
            }
        }

        my $body_c = __PACKAGE__->new();
        $body_c->{_handlers} = { %{$c->{_handlers}} };
        $body_c->compile_node($body) if $body;
        $body_c->emit(OP_RETURN);

        my $lambda_co = $body_c->to_code_object();
        my $co_idx    = $c->add_constant($lambda_co);
        $c->emit(OP_LOAD_CONST, $co_idx);
        $c->emit(OP_MAKE_FUNCTION, 0);
    });

    # -- list_comp -------------------------------------------------------
    $self->register_rule('list_comp', sub {
        my ($c, $node) = @_;
        $c->emit(OP_BUILD_LIST, 0);
        my @ch = @{$node->{children}};
        my $comp_clause = _find_ast_child(\@ch, 'comp_clause');
        if ($comp_clause) {
            my @cc = @{$comp_clause->{children}};
            my $iterable = _find_ast_child(\@cc, 'expr') // $cc[2];
            my $var_node = _find_ast_child(\@cc, 'identifier') // $cc[0];
            my $cond_node = _find_ast_child(\@ch, 'comp_if');

            $c->compile_node($iterable) if $iterable;
            $c->emit(OP_GET_ITER);
            my $loop_start = $c->current_offset();
            my $exit_jump  = $c->emit_jump(OP_FOR_ITER);
            _compile_store($c, $var_node) if $var_node;

            if ($cond_node) {
                my $cond_expr = _find_ast_child($cond_node->{children}, 'expr');
                if ($cond_expr) {
                    $c->compile_node($cond_expr);
                    my $skip_jump = $c->emit_jump(OP_JUMP_IF_FALSE);
                    my $elem = $ch[0];
                    $c->compile_node($elem);
                    $c->emit(OP_LIST_APPEND);
                    $c->patch_jump($skip_jump);
                }
            } else {
                my $elem = $ch[0];
                $c->compile_node($elem);
                $c->emit(OP_LIST_APPEND);
            }

            $c->emit(OP_JUMP, $loop_start);
            $c->patch_jump($exit_jump);
        }
    });

    # -- dict_comp -------------------------------------------------------
    $self->register_rule('dict_comp', sub {
        my ($c, $node) = @_;
        $c->emit(OP_BUILD_DICT, 0);
    });

    # -- comp_clause / comp_if / star_expr  -----------------------------
    for my $rule (qw(comp_clause comp_if star_expr)) {
        $self->register_rule($rule, sub { _pass_through($_[0], $_[1]) });
    }
}

# ============================================================================
# Private Helpers
# ============================================================================

sub _pass_through {
    my ($c, $node) = @_;
    my @ch = @{$node->{children}};
    if (@ch == 1) {
        $c->compile_node($ch[0]);
    } else {
        for my $child (@ch) {
            next if $child->{node_kind} eq 'token';
            $c->compile_node($child);
        }
    }
}

sub _compile_store {
    my ($c, $target) = @_;
    return unless $target;
    if ($target->{node_kind} eq 'token' && $target->{type} eq 'NAME') {
        my $idx = $c->add_name($target->{value});
        $c->emit(OP_STORE_NAME, $idx);
    } elsif ($target->{node_kind} eq 'ast') {
        my $name = undef;
        if ($target->{rule_name} eq 'identifier') {
            $name = _get_token_value($target->{children});
        } elsif ($target->{rule_name} eq 'atom' && @{$target->{children}} == 1) {
            my $child = $target->{children}[0];
            $name = $child->{value} if $child->{node_kind} eq 'token';
        }
        if (defined $name) {
            my $idx = $c->add_name($name);
            $c->emit(OP_STORE_NAME, $idx);
        }
    }
}

sub _compile_load {
    my ($c, $source) = @_;
    return unless $source;
    if ($source->{node_kind} eq 'token' && $source->{type} eq 'NAME') {
        my $idx = $c->add_name($source->{value});
        $c->emit(OP_LOAD_NAME, $idx);
    } elsif ($source->{node_kind} eq 'ast') {
        if ($source->{rule_name} eq 'identifier') {
            my $name = _get_token_value($source->{children});
            if (defined $name) {
                my $idx = $c->add_name($name);
                $c->emit(OP_LOAD_NAME, $idx);
                return;
            }
        }
        $c->compile_node($source);
    }
}

sub _compile_binary_chain {
    my ($c, $children) = @_;
    $c->compile_node($children->[0]);
    my $i = 1;
    while ($i <= $#$children) {
        my $op_tok = $children->[$i];
        my $right  = $children->[$i + 1];
        if ($op_tok && $op_tok->{node_kind} eq 'token' && $right) {
            $c->compile_node($right);
            my $opcode = $BINARY_OP_MAP{$op_tok->{value}};
            $c->emit($opcode) if defined $opcode;
        }
        $i += 2;
    }
}

sub _compile_if_chain {
    my ($c, $children, $start) = @_;
    my ($cond, $suite, $rest_start);
    my $i = $start;
    while ($i <= $#$children) {
        my $child = $children->[$i];
        if (ref $child eq 'HASH' && $child->{node_kind} eq 'ast') {
            if (!defined $cond && $child->{rule_name} ne 'suite'
                && $child->{rule_name} ne 'elif_clause'
                && $child->{rule_name} ne 'else_clause') {
                $cond = $child;
                $rest_start = $i + 1;
            } elsif ($child->{rule_name} eq 'suite') {
                $suite = $child;
                $rest_start = $i + 1;
                last;
            }
        }
        $i++;
    }

    $c->compile_node($cond) if $cond;
    my $jump_to_else = $c->emit_jump(OP_JUMP_IF_FALSE);
    $c->compile_node($suite) if $suite;

    # Check for elif/else
    if (defined $rest_start) {
        for my $j ($rest_start .. $#$children) {
            my $child = $children->[$j];
            next unless ref $child eq 'HASH' && $child->{node_kind} eq 'ast';
            my $r = $child->{rule_name} // '';
            if ($r eq 'elif_clause' || $r eq 'else_clause') {
                my $jump_over_else = $c->emit_jump(OP_JUMP);
                $c->patch_jump($jump_to_else);
                $c->compile_node($child);
                $c->patch_jump($jump_over_else);
                return;
            }
        }
    }

    $c->patch_jump($jump_to_else);
}

sub _find_iterable_in_for {
    my ($children) = @_;
    my $saw_in = 0;
    for my $child (@$children) {
        if ($child->{node_kind} eq 'token' && $child->{value} eq 'in') {
            $saw_in = 1;
        } elsif ($saw_in && $child->{node_kind} eq 'ast'
                 && ($child->{rule_name} // '') ne 'suite') {
            return $child;
        }
    }
    return undef;
}

sub _find_first_string {
    my ($children) = @_;
    for my $child (@$children) {
        if ($child->{node_kind} eq 'token' && $child->{type} eq 'STRING') {
            return _strip_quotes($child->{value});
        } elsif ($child->{node_kind} eq 'ast') {
            my $v = _find_first_string($child->{children} // []);
            return $v if defined $v;
        }
    }
    return undef;
}

sub _get_string_value {
    my ($node) = @_;
    for my $child (@{$node->{children} // []}) {
        if ($child->{node_kind} eq 'token') {
            return _strip_quotes($child->{value}) if $child->{type} eq 'STRING';
            return $child->{value}                if $child->{type} eq 'NAME';
        }
    }
    return undef;
}

sub _collect_params {
    my ($param_list) = @_;
    my @params;
    for my $child (@{$param_list->{children} // []}) {
        if (ref $child eq 'HASH' && $child->{node_kind} eq 'ast'
            && $child->{rule_name} eq 'param') {
            my $name = _find_token_type($child->{children}, 'NAME');
            push @params, $name if defined $name;
        } elsif (ref $child eq 'HASH' && $child->{node_kind} eq 'token'
                 && $child->{type} eq 'NAME') {
            push @params, $child->{value};
        }
    }
    return @params;
}

sub _get_compare_op {
    my ($children) = @_;
    for my $i (1 .. $#$children - 1) {
        my $tok = $children->[$i];
        if ($tok->{node_kind} eq 'token') {
            return 'not in' if $tok->{value} eq 'not';
            return $tok->{value};
        }
    }
    return '==';
}

sub _find_ast_child {
    my ($children, $rule_name) = @_;
    for my $child (@$children) {
        next unless ref $child eq 'HASH' && $child->{node_kind} eq 'ast';
        return $child if !defined $rule_name || $child->{rule_name} eq $rule_name;
    }
    return undef;
}

sub _find_token_type {
    my ($children, $token_type) = @_;
    for my $child (@{$children // []}) {
        if (ref $child eq 'HASH' && $child->{node_kind} eq 'token'
            && $child->{type} eq $token_type) {
            return $child->{value};
        }
    }
    return undef;
}

sub _get_token_value {
    my ($children) = @_;
    for my $child (@{$children // []}) {
        return $child->{value} if ref $child eq 'HASH' && $child->{node_kind} eq 'token';
    }
    return undef;
}

sub _find_first_token {
    my ($children) = @_;
    for my $child (@{$children // []}) {
        return $child if ref $child eq 'HASH' && $child->{node_kind} eq 'token';
    }
    return undef;
}

sub _strip_quotes {
    my ($s) = @_;
    return '' unless defined $s;
    # Triple-quoted
    if (substr($s, 0, 3) eq '"""' && substr($s, -3) eq '"""') {
        return substr($s, 3, length($s) - 6);
    }
    if (substr($s, 0, 3) eq "'''" && substr($s, -3) eq "'''") {
        return substr($s, 3, length($s) - 6);
    }
    # Single-quoted
    if ((substr($s, 0, 1) eq '"' && substr($s, -1) eq '"') ||
        (substr($s, 0, 1) eq "'" && substr($s, -1) eq "'")) {
        return substr($s, 1, length($s) - 2);
    }
    return $s;
}

# ============================================================================
# Public Convenience API
# ============================================================================

# compile_ast(\%ast_root) → hashref (code_object)
sub compile_ast {
    my ($class, $ast) = @_;
    my $compiler = $class->create_compiler();
    return $compiler->compile($ast);
}

1;

__END__

=head1 NAME

CodingAdventures::StarlarkAstToBytecodeCompiler - Compiles Starlark ASTs to bytecode

=head1 SYNOPSIS

    use CodingAdventures::StarlarkAstToBytecodeCompiler;

    my $tree = CodingAdventures::StarlarkAstToBytecodeCompiler::ast_node('file', [
        CodingAdventures::StarlarkAstToBytecodeCompiler::ast_node('statement', [
            CodingAdventures::StarlarkAstToBytecodeCompiler::ast_node('simple_stmt', [
                CodingAdventures::StarlarkAstToBytecodeCompiler::ast_node('assign_stmt', [
                    CodingAdventures::StarlarkAstToBytecodeCompiler::ast_node('identifier', [
                        CodingAdventures::StarlarkAstToBytecodeCompiler::token_node('NAME', 'x'),
                    ]),
                    CodingAdventures::StarlarkAstToBytecodeCompiler::token_node('OP', '='),
                    CodingAdventures::StarlarkAstToBytecodeCompiler::ast_node('atom', [
                        CodingAdventures::StarlarkAstToBytecodeCompiler::token_node('INT', '42'),
                    ]),
                ])
            ])
        ])
    ]);

    my $co = CodingAdventures::StarlarkAstToBytecodeCompiler->compile_ast($tree);
    # $co->{instructions}[0]{opcode} == 0x01  (OP_LOAD_CONST)
    # $co->{constants}[0] == 42
    # $co->{names}[0] eq 'x'

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
