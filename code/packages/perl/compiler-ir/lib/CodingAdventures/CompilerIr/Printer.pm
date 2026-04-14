package CodingAdventures::CompilerIr::Printer;

# ============================================================================
# CodingAdventures::CompilerIr::Printer — IrProgram → canonical text
# ============================================================================
#
# The printer converts an IrProgram into its canonical text format.
# This format serves three purposes:
#
#   1. Debugging — humans can read the IR to understand what the compiler did
#   2. Golden-file tests — expected IR output is committed as .ir text files
#   3. Roundtrip — parse(print(program)) == program is a testable invariant
#
# ## Text Format
#
#   .version 1
#
#   .data tape 30000 0
#
#   .entry _start
#
#   _start:
#     LOAD_ADDR   v0, tape          ; #0
#     LOAD_IMM    v1, 0             ; #1
#     HALT                          ; #2
#
# ## Formatting rules
#
#   - .version N is always the first non-comment line
#   - .data declarations come before .entry
#   - Labels are on their own line with a trailing colon, no indent
#   - Instructions are indented with two spaces
#   - ; #N comments show instruction IDs (informational, not semantic)
#   - COMMENT instructions emit as "; <text>" on their own line
#   - The opcode field is left-padded to 11 characters for alignment
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(print_ir);

use CodingAdventures::CompilerIr::IrOp qw(op_name);

# print_ir($program) — convert an IrProgram to its canonical text string.
#
# Returns a string.  The string ends with a newline after the last instruction.
sub print_ir {
    my ($program) = @_;

    my @lines;

    # ── .version directive ─────────────────────────────────────────────────
    push @lines, ".version $program->{version}";

    # ── .data declarations ─────────────────────────────────────────────────
    # Each data declaration gets its own block with a blank line before it.
    for my $decl (@{ $program->{data} }) {
        push @lines, '';
        push @lines, ".data $decl->{label} $decl->{size} $decl->{init}";
    }

    # ── .entry directive ───────────────────────────────────────────────────
    push @lines, '';
    push @lines, ".entry $program->{entry_label}";

    # ── instructions ───────────────────────────────────────────────────────
    for my $instr (@{ $program->{instructions} }) {
        my $opcode = $instr->{opcode};
        my $ops    = $instr->{operands};
        my $id     = $instr->{id};

        # LABEL — unindented name with colon, preceded by blank line
        if ($opcode == CodingAdventures::CompilerIr::IrOp::LABEL()) {
            push @lines, '';
            my $name = $ops->[0]->to_string;
            push @lines, "${name}:";
            next;
        }

        # COMMENT — emit as "; <text>" (no indent for comments)
        if ($opcode == CodingAdventures::CompilerIr::IrOp::COMMENT()) {
            my $text = @$ops ? $ops->[0]->to_string : '';
            push @lines, "  ; $text";
            next;
        }

        # Regular instruction: "  OPCODE     operand, operand  ; #ID"
        my $opname = op_name($opcode);

        # Format operands, comma-separated
        my $operand_str = join(', ', map { $_->to_string } @$ops);

        # Left-pad opcode to 11 characters for column alignment
        my $line = sprintf('  %-11s', $opname);
        $line .= $operand_str if length($operand_str);
        $line .= "  ; #$id";

        push @lines, $line;
    }

    return join("\n", @lines) . "\n";
}

1;

__END__

=head1 NAME

CodingAdventures::CompilerIr::Printer - print an IrProgram as canonical IR text

=head1 SYNOPSIS

  use CodingAdventures::CompilerIr::Printer qw(print_ir);

  my $text = print_ir($program);
  print $text;

=head1 DESCRIPTION

Converts an C<IrProgram> to its canonical text representation.  The format
is human-readable and round-trips through the parser:
C<parse_ir(print_ir($p))> reconstructs an equivalent program.

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
