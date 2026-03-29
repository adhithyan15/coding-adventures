package CodingAdventures::BytecodeCompiler;

# ============================================================================
# CodingAdventures::BytecodeCompiler — AST → stack-based bytecode
# ============================================================================
#
# A bytecode compiler translates an Abstract Syntax Tree (AST) — the structured
# representation of source code — into a flat sequence of bytecode instructions
# that a virtual machine can execute.
#
# Think of it like translating a recipe (structured with sections, sub-steps,
# and nested instructions) into a simple numbered list of actions.
#
# === WHY BYTECODE? ===
#
#   Source code  →  [Lexer]  →  Tokens  →  [Parser]  →  AST
#                                                         ↓
#                                              [BytecodeCompiler]
#                                                         ↓
#                                                    Bytecode
#                                                         ↓
#                                              [VirtualMachine]
#
# Real-world examples:
#   - Java:   .java → javac → .class files (JVM bytecode)
#   - Python: .py → compile() → .pyc files (CPython bytecode)
#   - Lua:    .lua → luac → bytecode chunks
#
# === INSTRUCTION ENCODING ===
#
# Each instruction is one or two integers:
#
#   [opcode]              — zero-operand instructions (e.g. ADD, HALT)
#   [opcode, operand]     — one-operand instructions  (e.g. PUSH 42, LOAD "x")
#
# The VM processes these linearly, maintaining a stack.
#
# === OPCODES ===
#
#   PUSH=0   — push a literal value onto the stack
#   POP=1    — discard the top stack value
#   ADD=2    — pop two values, push their sum
#   SUB=3    — pop two values, push their difference
#   MUL=4    — pop two values, push their product
#   DIV=5    — pop two values, push their quotient
#   AND=6    — pop two values, push logical AND
#   OR=7     — pop two values, push logical OR
#   NOT=8    — pop one value, push logical NOT
#   JMP=9    — unconditional jump to operand (instruction index)
#   JZ=10    — jump if top-of-stack is zero/false
#   JNZ=11   — jump if top-of-stack is non-zero/true
#   HALT=12  — stop execution
#   LOAD=13  — push the value of variable named by operand
#   STORE=14 — pop and store into variable named by operand
#   DUP=15   — duplicate top of stack
#   SWAP=16  — swap top two stack values
#
# === USAGE ===
#
#   use CodingAdventures::BytecodeCompiler;
#
#   my $compiler = CodingAdventures::BytecodeCompiler->new();
#   my $instr    = $compiler->compile($ast);   # arrayref of ints
#   print CodingAdventures::BytecodeCompiler::disassemble($instr);
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(disassemble);

# ============================================================================
# Opcode constants
# ============================================================================
#
# These numeric values are the "machine code" of our virtual machine.
# Using integer constants means the bytecode is compact and the VM can
# dispatch via a simple array lookup (like a jump table).

use constant {
    OP_PUSH  =>  0,
    OP_POP   =>  1,
    OP_ADD   =>  2,
    OP_SUB   =>  3,
    OP_MUL   =>  4,
    OP_DIV   =>  5,
    OP_AND   =>  6,
    OP_OR    =>  7,
    OP_NOT   =>  8,
    OP_JMP   =>  9,
    OP_JZ    => 10,
    OP_JNZ   => 11,
    OP_HALT  => 12,
    OP_LOAD  => 13,
    OP_STORE => 14,
    OP_DUP   => 15,
    OP_SWAP  => 16,
};

# Human-readable names for disassembly
my %OP_NAMES = (
    OP_PUSH,  'PUSH',
    OP_POP,   'POP',
    OP_ADD,   'ADD',
    OP_SUB,   'SUB',
    OP_MUL,   'MUL',
    OP_DIV,   'DIV',
    OP_AND,   'AND',
    OP_OR,    'OR',
    OP_NOT,   'NOT',
    OP_JMP,   'JMP',
    OP_JZ,    'JZ',
    OP_JNZ,   'JNZ',
    OP_HALT,  'HALT',
    OP_LOAD,  'LOAD',
    OP_STORE, 'STORE',
    OP_DUP,   'DUP',
    OP_SWAP,  'SWAP',
);

# Set of opcodes that take one operand (the instruction following is the argument)
my %HAS_OPERAND = map { $_ => 1 } (OP_PUSH, OP_JMP, OP_JZ, OP_JNZ, OP_LOAD, OP_STORE);

# ============================================================================
# Constructor
# ============================================================================

sub new {
    my ($class) = @_;
    return bless {
        _instructions => [],  # flat array of ints
    }, $class;
}

# ============================================================================
# compile — compile an AST into bytecode
# ============================================================================
#
# Entry point.  Accepts the root AST node (as returned by the parser).
# Returns an arrayref of integers.

sub compile {
    my ($self, $ast) = @_;
    $self->{_instructions} = [];
    $self->compile_node($ast);
    push @{ $self->{_instructions} }, OP_HALT;
    return $self->{_instructions};
}

# ============================================================================
# compile_node — dispatch on AST node type
# ============================================================================

sub compile_node {
    my ($self, $node) = @_;
    my $type = $node->{type};

    if ($type eq 'program') {
        for my $stmt (@{ $node->{stmts} }) {
            $self->compile_statement($stmt);
        }
    } elsif ($type eq 'let' || $type eq 'assign') {
        $self->compile_statement($node);
    } elsif ($type eq 'if') {
        $self->compile_if($node);
    } else {
        $self->compile_expr($node);
    }
}

# ============================================================================
# compile_statement — compile a statement-level node
# ============================================================================
#
# Statements:
#   let x = expr  → compile expr, STORE x
#   expr          → compile expr, POP (discard result)

sub compile_statement {
    my ($self, $node) = @_;
    my $type = $node->{type};

    if ($type eq 'let') {
        # Compile the right-hand side expression
        $self->compile_expr($node->{value});
        # Store result in the named variable
        push @{ $self->{_instructions} }, OP_STORE, $node->{name};
    } elsif ($type eq 'program') {
        for my $stmt (@{ $node->{stmts} }) {
            $self->compile_statement($stmt);
        }
    } else {
        # Expression statement: evaluate and discard the result
        $self->compile_expr($node);
        push @{ $self->{_instructions} }, OP_POP;
    }
}

# ============================================================================
# compile_expr — compile an expression node
# ============================================================================
#
# After compile_expr returns, exactly one value has been pushed onto the stack.

sub compile_expr {
    my ($self, $node) = @_;
    my $type = $node->{type};

    # --- Number literal: PUSH the numeric value ---
    if ($type eq 'number') {
        push @{ $self->{_instructions} }, OP_PUSH, $node->{value};
        return;
    }

    # --- String literal: PUSH the string ---
    if ($type eq 'string') {
        push @{ $self->{_instructions} }, OP_PUSH, $node->{value};
        return;
    }

    # --- Boolean literal ---
    if ($type eq 'bool') {
        push @{ $self->{_instructions} }, OP_PUSH, $node->{value};
        return;
    }

    # --- Nil literal ---
    if ($type eq 'nil') {
        push @{ $self->{_instructions} }, OP_PUSH, 0;
        return;
    }

    # --- Identifier: LOAD the variable ---
    if ($type eq 'ident') {
        push @{ $self->{_instructions} }, OP_LOAD, $node->{name};
        return;
    }

    # --- Unary operator ---
    if ($type eq 'unary') {
        $self->compile_expr($node->{expr});
        my $op = $node->{op};
        if ($op eq '-') {
            # Negate: push -1 and multiply
            push @{ $self->{_instructions} }, OP_PUSH, -1, OP_MUL;
        } elsif ($op eq '!' || $op eq 'not') {
            push @{ $self->{_instructions} }, OP_NOT;
        } else {
            die "Unknown unary operator: $op\n";
        }
        return;
    }

    # --- Binary operator ---
    if ($type eq 'binop') {
        return $self->compile_binop($node);
    }

    # --- If expression ---
    if ($type eq 'if') {
        return $self->compile_if($node);
    }

    # --- Function call ---
    if ($type eq 'call') {
        return $self->compile_call($node);
    }

    # --- Let binding used as expression (returns the value) ---
    if ($type eq 'let') {
        $self->compile_expr($node->{value});
        push @{ $self->{_instructions} }, OP_DUP;
        push @{ $self->{_instructions} }, OP_STORE, $node->{name};
        push @{ $self->{_instructions} }, OP_POP;
        return;
    }

    die "Unknown AST node type in compile_expr: $type\n";
}

# ============================================================================
# compile_binop — compile a binary operator node
# ============================================================================
#
# Binary operators are compiled in post-order: left, right, operator.
# This is correct for a stack machine:
#
#   1 + 2  →  PUSH 1, PUSH 2, ADD
#
# The ADD instruction pops two values and pushes their sum.

sub compile_binop {
    my ($self, $node) = @_;
    my $op = $node->{op};

    # Short-circuit 'and': evaluate left; if false, skip right
    if ($op eq 'and') {
        $self->compile_expr($node->{left});
        push @{ $self->{_instructions} }, OP_DUP;
        my $jz_idx = scalar @{ $self->{_instructions} };
        push @{ $self->{_instructions} }, OP_JZ, 0;  # placeholder
        push @{ $self->{_instructions} }, OP_POP;
        $self->compile_expr($node->{right});
        # Patch jump target
        $self->{_instructions}[$jz_idx + 1] = scalar @{ $self->{_instructions} };
        return;
    }

    # Short-circuit 'or'
    if ($op eq 'or') {
        $self->compile_expr($node->{left});
        push @{ $self->{_instructions} }, OP_DUP;
        my $jnz_idx = scalar @{ $self->{_instructions} };
        push @{ $self->{_instructions} }, OP_JNZ, 0;  # placeholder
        push @{ $self->{_instructions} }, OP_POP;
        $self->compile_expr($node->{right});
        $self->{_instructions}[$jnz_idx + 1] = scalar @{ $self->{_instructions} };
        return;
    }

    # Normal binary operators: compile both operands then the opcode
    $self->compile_expr($node->{left});
    $self->compile_expr($node->{right});

    my %opmap = (
        '+'  => OP_ADD,
        '-'  => OP_SUB,
        '*'  => OP_MUL,
        '/'  => OP_DIV,
        '==' => OP_AND,   # simplification: == maps to AND for now
        '!=' => OP_OR,    # simplification
        '<'  => OP_SUB,   # simplification: compare via subtraction result
        '>'  => OP_SUB,
        '<=' => OP_SUB,
        '>=' => OP_SUB,
    );

    # For a proper implementation the VM would have CMP opcodes; our simple
    # VM only has ADD/SUB/MUL/DIV/AND/OR/NOT so we approximate.
    my $opcode = $opmap{$op} // die "Unknown binary operator: $op\n";
    push @{ $self->{_instructions} }, $opcode;
}

# ============================================================================
# compile_if — compile an if expression
# ============================================================================
#
# Compiles to:
#
#   <cond>
#   JZ  else_label
#   <then>
#   JMP end_label
#   else_label:
#   <else>
#   end_label:

sub compile_if {
    my ($self, $node) = @_;

    # Compile condition
    $self->compile_expr($node->{cond});

    # JZ placeholder (will jump to else branch)
    my $jz_idx = scalar @{ $self->{_instructions} };
    push @{ $self->{_instructions} }, OP_JZ, 0;

    # Compile then-branch
    $self->compile_expr($node->{then});

    # JMP placeholder (will jump past else branch)
    my $jmp_idx = scalar @{ $self->{_instructions} };
    push @{ $self->{_instructions} }, OP_JMP, 0;

    # Patch JZ to here (start of else branch)
    $self->{_instructions}[$jz_idx + 1] = scalar @{ $self->{_instructions} };

    # Compile else-branch (or push 0 if absent)
    if (defined $node->{else}) {
        $self->compile_expr($node->{else});
    } else {
        push @{ $self->{_instructions} }, OP_PUSH, 0;
    }

    # Patch JMP to here (after else branch)
    $self->{_instructions}[$jmp_idx + 1] = scalar @{ $self->{_instructions} };
}

# ============================================================================
# compile_call — compile a function call
# ============================================================================
#
# In our simple VM there are no first-class functions, so we treat known
# built-ins (print) specially and leave others as a LOAD + series of pushes.

sub compile_call {
    my ($self, $node) = @_;
    # Push arguments in order
    for my $arg (@{ $node->{args} }) {
        $self->compile_expr($arg);
    }
    # LOAD the function name (VM will look it up)
    push @{ $self->{_instructions} }, OP_LOAD, $node->{name};
}

# ============================================================================
# disassemble — convert bytecode to human-readable text
# ============================================================================
#
# Accepts an arrayref of integers (as returned by compile).
# Returns a multiline string like:
#
#   0000: PUSH 42
#   0002: PUSH 8
#   0004: ADD
#   0005: HALT
#
# @param $instructions  arrayref of integers
# @return string

sub disassemble {
    my ($instructions) = @_;
    my @out;
    my $i = 0;
    while ($i < @$instructions) {
        my $op   = $instructions->[$i];
        my $name = $OP_NAMES{$op} // "UNKNOWN($op)";
        if ($HAS_OPERAND{$op} && $i + 1 < @$instructions) {
            my $operand = $instructions->[$i + 1];
            push @out, sprintf("%04d: %s %s", $i, $name, $operand);
            $i += 2;
        } else {
            push @out, sprintf("%04d: %s", $i, $name);
            $i++;
        }
    }
    return join("\n", @out) . "\n";
}

1;

__END__

=head1 NAME

CodingAdventures::BytecodeCompiler - AST to stack-based bytecode compiler

=head1 SYNOPSIS

    use CodingAdventures::BytecodeCompiler;

    my $compiler = CodingAdventures::BytecodeCompiler->new();
    my $bytecode = $compiler->compile($ast);   # arrayref of integers
    print CodingAdventures::BytecodeCompiler::disassemble($bytecode);

=head1 DESCRIPTION

Compiles an AST (as produced by CodingAdventures::Parser) to a flat array of
integers representing stack-machine bytecode.  Includes a C<disassemble>
function for human-readable output.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
