package CodingAdventures::Brainfuck;

# ============================================================================
# CodingAdventures::Brainfuck — Brainfuck interpreter and bytecode compiler
# ============================================================================
#
# Brainfuck (Urban Müller, 1993) is a Turing-complete language with exactly
# 8 commands.  Its educational value is enormous despite (or because of) its
# absurd minimalism: it strips a computer down to its bare essentials.
#
# ## The 8 Commands
#
#   Command  | C equivalent      | Description
#   ---------+-------------------+--------------------------------
#   >        | ++ptr             | Move data pointer right
#   <        | --ptr             | Move data pointer left
#   +        | (*ptr)++          | Increment current cell
#   -        | (*ptr)--          | Decrement current cell
#   .        | putchar(*ptr)     | Output current cell as ASCII
#   ,        | *ptr = getchar()  | Read input into current cell
#   [        | while (*ptr) {    | Jump past ] if cell == 0
#   ]        | }                 | Jump back to [ if cell != 0
#
# All other characters are treated as comments.
#
# ## The Tape (Memory Model)
#
# The tape is an array of 30,000 byte cells, all initialised to 0.  Cells
# wrap: 255 + 1 = 0, 0 - 1 = 255.  The data pointer (dp) starts at cell 0.
#
# ## Two-Phase Execution
#
# Phase 1 — compile_to_opcodes: parse source, assign opcodes, pre-compute
# jump targets for [ and ] using a stack.  This converts O(n) runtime bracket
# searches into O(1) array lookups.
#
# Phase 2 — run_opcodes: execute the opcode array in a simple while loop.
#
# ## Usage
#
#   use CodingAdventures::Brainfuck qw(validate compile_to_opcodes run_opcodes interpret);
#
#   # High-level
#   my ($out, $err) = interpret("+++++++++[>++++++++<-]>.", "");
#   print $out;   # "H"
#
#   # Low-level
#   my ($ops, $err2) = compile_to_opcodes(",[.,]");
#   die $err2 if $err2;
#   my $result = run_opcodes($ops, "hello");   # "hello"
#
# ============================================================================

use strict;
use warnings;
use Exporter 'import';

our $VERSION = '0.01';

our @EXPORT_OK = qw(validate compile_to_opcodes run_opcodes interpret);

# ============================================================================
# Opcode Constants
# ============================================================================
#
# We map each Brainfuck character to an integer opcode.  Using the same
# hex values as the Go and Lua implementations makes the bytecode conceptually
# portable across language implementations.

use constant {
    OP_RIGHT      => 0x01,   # >
    OP_LEFT       => 0x02,   # <
    OP_INC        => 0x03,   # +
    OP_DEC        => 0x04,   # -
    OP_OUTPUT     => 0x05,   # .
    OP_INPUT      => 0x06,   # ,
    OP_LOOP_START => 0x07,   # [
    OP_LOOP_END   => 0x08,   # ]
    OP_HALT       => 0xFF,   # end of program
};

# Map source characters to opcodes.
my %CHAR_TO_OP = (
    '>' => OP_RIGHT,
    '<' => OP_LEFT,
    '+' => OP_INC,
    '-' => OP_DEC,
    '.' => OP_OUTPUT,
    ',' => OP_INPUT,
    '[' => OP_LOOP_START,
    ']' => OP_LOOP_END,
);

# Tape size matches the original specification.
use constant TAPE_SIZE => 30_000;

# ============================================================================
# validate — Check bracket balance
# ============================================================================

=head2 validate($program)

Check that all C<[> and C<]> brackets in C<$program> are balanced.

  my ($ok, $err) = validate("[[][]]");   # (1, undef)
  my ($ok2, $err2) = validate("[");      # (0, "1 unclosed ...")

Returns C<(1, undef)> if valid, or C<(0, $error_message)> if not.

=cut

sub validate {
    my ($program) = @_;
    my $depth = 0;
    for my $i (0..length($program)-1) {
        my $ch = substr($program, $i, 1);
        if ($ch eq '[') {
            $depth++;
        }
        elsif ($ch eq ']') {
            $depth--;
            if ($depth < 0) {
                return (0, sprintf("unmatched ']' at position %d", $i + 1));
            }
        }
    }
    if ($depth > 0) {
        return (0, sprintf("%d unclosed '[' bracket(s)", $depth));
    }
    return (1, undef);
}

# ============================================================================
# compile_to_opcodes — Translate source to opcode array with jump targets
# ============================================================================

=head2 compile_to_opcodes($program)

Compile C<$program> to an array-ref of instruction hash-refs:

  [ { op => OP_INC, operand => undef }, ... ]

For OP_LOOP_START, C<operand> is the index AFTER the matching C<]>.
For OP_LOOP_END,   C<operand> is the index of the matching C<[>.

The array is 0-indexed.  A HALT instruction is appended at the end.

Returns C<($opcodes_arrayref, undef)> on success, or C<(undef, $error)> on
unbalanced brackets.

=cut

sub compile_to_opcodes {
    my ($program) = @_;

    my ($ok, $err) = validate($program);
    return (undef, $err) unless $ok;

    # First pass: build opcode array (no jump targets yet).
    my @opcodes;
    for my $i (0..length($program)-1) {
        my $ch = substr($program, $i, 1);
        my $op = $CHAR_TO_OP{$ch};
        if (defined $op) {
            push @opcodes, { op => $op, operand => undef };
        }
    }
    push @opcodes, { op => OP_HALT, operand => undef };

    # Second pass: resolve [ ] jump targets using a stack.
    my @stack;
    for my $i (0..$#opcodes) {
        if ($opcodes[$i]{op} == OP_LOOP_START) {
            push @stack, $i;
        }
        elsif ($opcodes[$i]{op} == OP_LOOP_END) {
            my $open_idx = pop @stack;
            # [ jumps to instruction AFTER ] (i.e. index i+1)
            $opcodes[$open_idx]{operand} = $i + 1;
            # ] jumps back to [
            $opcodes[$i]{operand} = $open_idx;
        }
    }

    return (\@opcodes, undef);
}

# ============================================================================
# run_opcodes — Execute compiled opcodes
# ============================================================================

=head2 run_opcodes($opcodes, $input_str)

Execute a compiled opcode array-ref and return the output string.

  my $output = run_opcodes($opcodes, "hello");

EOF convention: when C<,> is executed and input is exhausted, the cell is
set to 0.  This is the most common convention and makes the cat program
C<,[.,]> terminate naturally.

=cut

sub run_opcodes {
    my ($opcodes, $input_str) = @_;
    $input_str //= '';

    # Initialise tape: 30,000 zero bytes.
    my @tape      = (0) x TAPE_SIZE;
    my $dp        = 0;    # data pointer (0-based)
    my $pc        = 0;    # program counter (0-based)
    my $input_pos = 0;    # next input byte (0-based)
    my @output;           # collected output characters

    while ($pc < scalar @$opcodes) {
        my $instr = $opcodes->[$pc];
        my $op    = $instr->{op};

        # > — Move data pointer right
        if ($op == OP_RIGHT) {
            $dp++;
            if ($dp >= TAPE_SIZE) {
                die sprintf("BrainfuckError: data pointer past end of tape at pc=%d\n", $pc);
            }
            $pc++;
        }
        # < — Move data pointer left
        elsif ($op == OP_LEFT) {
            $dp--;
            if ($dp < 0) {
                die "BrainfuckError: data pointer before start of tape\n";
            }
            $pc++;
        }
        # + — Increment current cell (wraps 255 → 0)
        elsif ($op == OP_INC) {
            $tape[$dp] = ($tape[$dp] + 1) % 256;
            $pc++;
        }
        # - — Decrement current cell (wraps 0 → 255)
        elsif ($op == OP_DEC) {
            $tape[$dp] = ($tape[$dp] - 1 + 256) % 256;
            $pc++;
        }
        # . — Output current cell as ASCII character
        elsif ($op == OP_OUTPUT) {
            push @output, chr($tape[$dp]);
            $pc++;
        }
        # , — Read one byte of input into current cell; 0 on EOF
        elsif ($op == OP_INPUT) {
            if ($input_pos < length($input_str)) {
                $tape[$dp] = ord(substr($input_str, $input_pos, 1));
                $input_pos++;
            }
            else {
                $tape[$dp] = 0;    # EOF → 0
            }
            $pc++;
        }
        # [ — Jump past ] if cell is 0
        elsif ($op == OP_LOOP_START) {
            if ($tape[$dp] == 0) {
                $pc = $instr->{operand};    # skip loop
            }
            else {
                $pc++;                       # enter loop body
            }
        }
        # ] — Jump back to [ if cell is nonzero
        elsif ($op == OP_LOOP_END) {
            if ($tape[$dp] != 0) {
                $pc = $instr->{operand};    # loop back
            }
            else {
                $pc++;                       # exit loop
            }
        }
        # HALT — stop
        elsif ($op == OP_HALT) {
            last;
        }
        else {
            die sprintf("BrainfuckError: unknown opcode 0x%02x at pc=%d\n", $op, $pc);
        }
    }

    return join('', @output);
}

# ============================================================================
# interpret — High-level one-call interface
# ============================================================================

=head2 interpret($program, $input_str)

Validate, compile, and execute C<$program> in one call.

  my ($output, $err) = interpret("+++.", "");

Returns C<($output, undef)> on success or C<(undef, $error)> on failure.

=cut

sub interpret {
    my ($program, $input_str) = @_;
    $input_str //= '';

    my ($opcodes, $err) = compile_to_opcodes($program);
    return (undef, $err) if $err;

    my $result = eval { run_opcodes($opcodes, $input_str) };
    if ($@) {
        return (undef, $@);
    }
    return ($result, undef);
}

1;

__END__

=head1 NAME

CodingAdventures::Brainfuck - Brainfuck interpreter and bytecode compiler

=head1 VERSION

0.01

=head1 SYNOPSIS

  use CodingAdventures::Brainfuck qw(interpret validate compile_to_opcodes run_opcodes);

  my ($out, $err) = interpret("+++++++++[>++++++++<-]>.", "");
  print $out;  # "H"

=head1 DESCRIPTION

Two-phase Brainfuck implementation: compile source to opcodes with
pre-computed jump targets, then execute in a simple eval loop.

=head1 EXPORTS

Nothing by default.  Import explicitly:

  use CodingAdventures::Brainfuck qw(validate compile_to_opcodes run_opcodes interpret);

=head1 LICENSE

MIT
