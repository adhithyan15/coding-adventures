package CodingAdventures::Brainfuck::Parser;

# ============================================================================
# CodingAdventures::Brainfuck::Parser — Hand-written recursive descent parser
# ============================================================================
#
# This module parses tokenized Brainfuck into an Abstract Syntax Tree (AST).
# It is a hand-written recursive descent parser — the same approach used by
# CodingAdventures::JsonParser — because Brainfuck's grammar is so simple
# that a 4-function recursive descent implementation is cleaner and more
# educational than setting up the full grammar-driven machinery.
#
# # The Brainfuck Grammar
# ========================
#
# (This matches the formal grammar in code/grammars/brainfuck.grammar)
#
#   program     = { instruction } ;
#   instruction = loop | command ;
#   loop        = LOOP_START { instruction } LOOP_END ;
#   command     = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT ;
#
# There are exactly 4 rules. The key insight:
#
#   - A "program" is just a sequence of instructions.
#   - An "instruction" is either a loop or a simple command.
#   - A "loop" is `[` followed by more instructions, followed by `]`.
#   - A "command" is one of the 6 non-bracket operators.
#
# Loops nest arbitrarily deep because `instruction` can be a `loop`,
# and `loop` contains `{ instruction }`.
#
# # AST Node Structure
# =====================
#
# Each node is a hashref with these keys:
#
#   {
#     type     => 'program'|'instruction'|'loop'|'command',
#     children => [...],        # child nodes (for program, instruction, loop)
#     token    => { ... },      # the token this node wraps (for command nodes)
#     line     => N,            # source line number
#     col      => N,            # source column number
#   }
#
# Example AST for "++[>+<-]":
#
#   { type => 'program', children => [
#       { type => 'instruction', children => [
#           { type => 'command', token => { type => 'INC', value => '+', ... } }
#       ]},
#       { type => 'instruction', children => [
#           { type => 'command', token => { type => 'INC', value => '+', ... } }
#       ]},
#       { type => 'instruction', children => [
#           { type => 'loop', children => [
#               { type => 'instruction', children => [
#                   { type => 'command', token => { type => 'RIGHT', ... } }
#               ]},
#               ... etc ...
#           ]}
#       ]},
#   ]}
#
# # Recursive Descent
# ===================
#
# Recursive descent parsing works by having one function per grammar rule.
# Each function "consumes" tokens from the stream and returns a node:
#
#   _parse_program    → loops over _parse_instruction until EOF
#   _parse_instruction → peeks at next token:
#                          LOOP_START → calls _parse_loop
#                          otherwise  → calls _parse_command
#   _parse_loop       → consumes LOOP_START, loops _parse_instruction
#                        until LOOP_END, consumes LOOP_END
#   _parse_command    → consumes one of the 6 command tokens
#
# Unmatched brackets are detected naturally:
#   - LOOP_END without LOOP_START: _parse_instruction is called, sees ']',
#     falls through to _parse_command, which dies because ']' is not a command.
#   - LOOP_START without LOOP_END: _parse_loop consumes instructions until
#     it hits EOF (not LOOP_END), then dies with a descriptive error.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::Brainfuck::Lexer;

# ============================================================================
# Public API
# ============================================================================

=head2 parse($source)

Parse a Brainfuck source string and return an AST node hashref.

  my $ast = CodingAdventures::Brainfuck::Parser->parse("++[>+<-]");
  # $ast->{type}  → 'program'

Returns the root AST node (type => 'program').
Dies with a descriptive error message on unmatched brackets.

=cut

sub parse {
    my ($class, $source) = @_;

    # Step 1: Tokenize using the grammar-driven lexer.
    # Comment characters are silently consumed, so we receive only the
    # 8 command tokens plus EOF.
    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize($source);

    # Step 2: Set up the parser state.
    # We use a simple index into the token array as our "cursor".
    # This is more efficient than shifting off the front of an array.
    my $pos = 0;

    # Step 3: Parse the program.
    return _parse_program($tokens, \$pos);
}

# ============================================================================
# Internal: recursive descent parsing functions
# ============================================================================

# --- _peek($tokens, $pos_ref) -------------------------------------------------
#
# Look at the current token without consuming it.
# Returns the token hashref.

sub _peek {
    my ($tokens, $pos_ref) = @_;
    return $tokens->[$$pos_ref];
}

# --- _consume($tokens, $pos_ref, $expected_type) ------------------------------
#
# Consume the current token and advance the position.
# Dies if the current token's type does not match $expected_type.
# Returns the consumed token hashref.

sub _consume {
    my ($tokens, $pos_ref, $expected_type) = @_;
    my $tok = $tokens->[$$pos_ref];
    unless ($tok && $tok->{type} eq $expected_type) {
        my $got  = $tok ? "$tok->{type} ('$tok->{value}')" : "end of input";
        my $line = $tok ? $tok->{line} : '?';
        my $col  = $tok ? $tok->{col}  : '?';
        die sprintf(
            "CodingAdventures::Brainfuck::Parser: Parse error at line %s col %s: "
          . "expected %s but got %s",
            $line, $col, $expected_type, $got
        );
    }
    $$pos_ref++;
    return $tok;
}

# --- _parse_program($tokens, $pos_ref) ----------------------------------------
#
# Grammar rule: program = { instruction } ;
#
# A program is zero or more instructions. We collect instructions until we
# hit EOF. Any token other than EOF is the start of an instruction.
#
# Returns: { type => 'program', children => [...instructions...], line => 1, col => 1 }

sub _parse_program {
    my ($tokens, $pos_ref) = @_;

    my @children;
    my $first_tok = _peek($tokens, $pos_ref);

    # Keep parsing instructions until we reach EOF.
    while (1) {
        my $tok = _peek($tokens, $pos_ref);
        last unless $tok && $tok->{type} ne 'EOF';

        # A LOOP_END here means an unmatched ']' at the top level.
        # _parse_instruction will catch this and die with a clear message.
        push @children, _parse_instruction($tokens, $pos_ref);
    }

    return {
        type     => 'program',
        children => \@children,
        line     => $first_tok ? $first_tok->{line} : 1,
        col      => $first_tok ? $first_tok->{col}  : 1,
    };
}

# --- _parse_instruction($tokens, $pos_ref) ------------------------------------
#
# Grammar rule: instruction = loop | command ;
#
# An instruction is either a loop (starts with LOOP_START) or a command
# (any of the six non-bracket operators).
#
# Returns: { type => 'instruction', children => [loop_or_command], ... }

sub _parse_instruction {
    my ($tokens, $pos_ref) = @_;

    my $tok = _peek($tokens, $pos_ref);
    unless ($tok && $tok->{type} ne 'EOF') {
        die "CodingAdventures::Brainfuck::Parser: Parse error: unexpected end of input";
    }

    # Unmatched ']' at statement level — no matching '[' was seen.
    if ($tok->{type} eq 'LOOP_END') {
        die sprintf(
            "CodingAdventures::Brainfuck::Parser: Parse error at line %d col %d: "
          . "unexpected ']' without matching '['",
            $tok->{line}, $tok->{col}
        );
    }

    my $child;
    if ($tok->{type} eq 'LOOP_START') {
        # Dispatch to the loop production.
        $child = _parse_loop($tokens, $pos_ref);
    } else {
        # Dispatch to the command production.
        $child = _parse_command($tokens, $pos_ref);
    }

    return {
        type     => 'instruction',
        children => [$child],
        line     => $tok->{line},
        col      => $tok->{col},
    };
}

# --- _parse_loop($tokens, $pos_ref) -------------------------------------------
#
# Grammar rule: loop = LOOP_START { instruction } LOOP_END ;
#
# A loop begins with '[', contains zero or more instructions, and ends with ']'.
# An empty loop '[]' is legal (and useful as a "wait for zero" idiom: [-]).
#
# Unmatched '[' is detected when we hit EOF before finding ']'.
#
# Returns: { type => 'loop', children => [...instructions...], ... }

sub _parse_loop {
    my ($tokens, $pos_ref) = @_;

    # Consume the opening '['.
    my $open_tok = _consume($tokens, $pos_ref, 'LOOP_START');

    my @children;

    # Parse instructions until we see ']' or run out of tokens.
    while (1) {
        my $tok = _peek($tokens, $pos_ref);

        # Hit EOF without a closing ']' — unmatched '['.
        if (!$tok || $tok->{type} eq 'EOF') {
            die sprintf(
                "CodingAdventures::Brainfuck::Parser: Parse error at line %d col %d: "
              . "'[' without matching ']'",
                $open_tok->{line}, $open_tok->{col}
            );
        }

        # Found the matching ']' — stop collecting loop body instructions.
        last if $tok->{type} eq 'LOOP_END';

        push @children, _parse_instruction($tokens, $pos_ref);
    }

    # Consume the closing ']'.
    _consume($tokens, $pos_ref, 'LOOP_END');

    return {
        type     => 'loop',
        children => \@children,
        line     => $open_tok->{line},
        col      => $open_tok->{col},
    };
}

# --- _parse_command($tokens, $pos_ref) ----------------------------------------
#
# Grammar rule: command = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT ;
#
# A command is a single token from the six non-bracket operators.
# LOOP_START and LOOP_END are intentionally not handled here — they are
# consumed by _parse_loop and _parse_instruction respectively.
#
# Returns: { type => 'command', token => {...}, ... }

my %COMMAND_TYPES = map { $_ => 1 } qw(RIGHT LEFT INC DEC OUTPUT INPUT);

sub _parse_command {
    my ($tokens, $pos_ref) = @_;

    my $tok = _peek($tokens, $pos_ref);

    unless ($tok && $COMMAND_TYPES{ $tok->{type} }) {
        my $got  = $tok ? "$tok->{type} ('$tok->{value}')" : "end of input";
        my $line = $tok ? $tok->{line} : '?';
        my $col  = $tok ? $tok->{col}  : '?';
        die sprintf(
            "CodingAdventures::Brainfuck::Parser: Parse error at line %s col %s: "
          . "expected a command (> < + - . ,) but got %s",
            $line, $col, $got
        );
    }

    # Consume and return the command token.
    $$pos_ref++;
    return {
        type  => 'command',
        token => $tok,
        line  => $tok->{line},
        col   => $tok->{col},
    };
}

1;

__END__

=head1 NAME

CodingAdventures::Brainfuck::Parser - Hand-written recursive descent Brainfuck parser

=head1 SYNOPSIS

    use CodingAdventures::Brainfuck::Parser;

    my $ast = CodingAdventures::Brainfuck::Parser->parse("++[>+<-]");
    print $ast->{type};   # 'program'
    print scalar @{ $ast->{children} };  # 3 (two INC instructions + one loop instruction)

=head1 DESCRIPTION

A recursive descent parser for Brainfuck source code. Uses
C<CodingAdventures::Brainfuck::Lexer> for tokenization, then builds an AST
from the token stream following the Brainfuck grammar:

    program     = { instruction } ;
    instruction = loop | command ;
    loop        = LOOP_START { instruction } LOOP_END ;
    command     = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT ;

Each AST node is a hashref with keys: C<type>, C<children>, C<token>,
C<line>, C<col>. Node types are: C<program>, C<instruction>, C<loop>,
C<command>.

Unmatched brackets cause the parser to die with a descriptive error message
including the line and column of the offending bracket.

=head1 METHODS

=head2 parse($source)

Parse a Brainfuck source string and return the root AST node
(C<type => 'program'>). Dies on unmatched brackets.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
