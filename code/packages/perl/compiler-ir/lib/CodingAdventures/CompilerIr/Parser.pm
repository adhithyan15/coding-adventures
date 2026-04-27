package CodingAdventures::CompilerIr::Parser;

# ============================================================================
# CodingAdventures::CompilerIr::Parser — IR text → IrProgram
# ============================================================================
#
# The parser reads the canonical IR text format (produced by Printer) and
# reconstructs an IrProgram.  This enables:
#
#   1. Golden-file testing — load an expected .ir file, parse it, compare
#   2. Roundtrip verification — parse(print(program)) == program
#   3. Manual IR authoring — write IR by hand for testing backends
#
# ## Parsing strategy
#
# The parser processes the text line by line:
#
#   1. Lines starting with ".version" set the program version
#   2. Lines starting with ".data" add a data declaration
#   3. Lines starting with ".entry" set the entry label
#   4. Lines ending with ":" define a label
#   5. Lines starting with whitespace are instructions
#   6. Lines starting with ";" are standalone comments
#   7. Blank lines are skipped
#
# Each instruction line is split into: opcode, operands, and optional
# "; #N" ID comment.  Operands are parsed as registers (v0, v1, ...),
# immediates (42, -1), or labels (any other identifier).
#
# ## Security limits
#
# To prevent denial-of-service from adversarial input:
#   - Maximum 1,000,000 lines
#   - Maximum 16 operands per instruction
#   - Maximum register index 65535
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(parse_ir);

use CodingAdventures::CompilerIr::IrOp        qw(parse_op op_name);
use CodingAdventures::CompilerIr::IrProgram;
use CodingAdventures::CompilerIr::IrInstruction;
use CodingAdventures::CompilerIr::IrDataDecl;
use CodingAdventures::CompilerIr::IrRegister;
use CodingAdventures::CompilerIr::IrImmediate;
use CodingAdventures::CompilerIr::IrLabel;

use constant MAX_LINES          => 1_000_000;
use constant MAX_OPERANDS       => 16;
use constant MAX_REGISTER_INDEX => 65535;

# parse_ir($text) — convert IR text to an IrProgram.
#
# Returns the IrProgram on success.
# Dies with a descriptive error message on malformed input.
sub parse_ir {
    my ($text) = @_;

    my $program = CodingAdventures::CompilerIr::IrProgram->new('');
    $program->{version} = 1;

    my @lines = split /\n/, $text, -1;

    if (@lines > MAX_LINES) {
        die sprintf(
            "parse_ir: input too large: %d lines (max %d)",
            scalar @lines, MAX_LINES
        );
    }

    my $line_num = 0;
    for my $line (@lines) {
        $line_num++;
        my $trimmed = $line;
        $trimmed =~ s/^\s+|\s+$//g;  # trim both ends

        # Skip blank lines
        next if $trimmed eq '';

        # .version directive
        if ($trimmed =~ /^\.version\b/) {
            my @parts = split /\s+/, $trimmed;
            unless (@parts == 2) {
                die "parse_ir line $line_num: invalid .version directive: '$line'";
            }
            unless ($parts[1] =~ /^\d+$/) {
                die "parse_ir line $line_num: invalid version number: '$parts[1]'";
            }
            $program->{version} = int($parts[1]);
            next;
        }

        # .data directive
        if ($trimmed =~ /^\.data\b/) {
            my @parts = split /\s+/, $trimmed;
            unless (@parts == 4) {
                die "parse_ir line $line_num: invalid .data directive: '$line'";
            }
            unless ($parts[2] =~ /^\d+$/) {
                die "parse_ir line $line_num: invalid data size: '$parts[2]'";
            }
            unless ($parts[3] =~ /^-?\d+$/) {
                die "parse_ir line $line_num: invalid data init: '$parts[3]'";
            }
            $program->add_data(
                CodingAdventures::CompilerIr::IrDataDecl->new(
                    label => $parts[1],
                    size  => int($parts[2]),
                    init  => int($parts[3]),
                )
            );
            next;
        }

        # .entry directive
        if ($trimmed =~ /^\.entry\b/) {
            my @parts = split /\s+/, $trimmed;
            unless (@parts == 2) {
                die "parse_ir line $line_num: invalid .entry directive: '$line'";
            }
            $program->{entry_label} = $parts[1];
            next;
        }

        # Label definition — line ends with ':' and doesn't start with ';'
        if ($trimmed =~ /^([^;].*):\s*$/) {
            my $label_name = $1;
            $label_name =~ s/\s+$//;
            $program->add_instruction(
                CodingAdventures::CompilerIr::IrInstruction->new(
                    opcode   => CodingAdventures::CompilerIr::IrOp::LABEL(),
                    operands => [
                        CodingAdventures::CompilerIr::IrLabel->new($label_name)
                    ],
                    id       => -1,
                )
            );
            next;
        }

        # Standalone comment line — starts with ';'
        if ($trimmed =~ /^;(.*)$/) {
            my $comment_text = $1;
            $comment_text =~ s/^\s+//;
            # Skip ID comments ("; #N") — those are instruction annotations
            unless ($comment_text =~ /^#/) {
                $program->add_instruction(
                    CodingAdventures::CompilerIr::IrInstruction->new(
                        opcode   => CodingAdventures::CompilerIr::IrOp::COMMENT(),
                        operands => [
                            CodingAdventures::CompilerIr::IrLabel->new($comment_text)
                        ],
                        id       => -1,
                    )
                );
            }
            next;
        }

        # Regular instruction line (starts with whitespace or opcode)
        my $instr = _parse_instruction_line($trimmed, $line_num);
        $program->add_instruction($instr);
    }

    return $program;
}

# _parse_instruction_line($line, $line_num) — parse one instruction.
#
# Format:  "OPCODE  operand, operand  ; #ID"
#
# The "; #N" ID comment is optional.  Operands are comma-separated.
sub _parse_instruction_line {
    my ($line, $line_num) = @_;

    # Split off the "; #N" ID comment if present
    my $id = -1;
    my $instruction_part = $line;
    if ($line =~ /^(.*?);\s*#(\d+)\s*$/) {
        my ($instr_part, $id_str) = ($1, $2);
        $id = int($id_str);
        $instruction_part = $instr_part;
        $instruction_part =~ s/\s+$//;
    }

    # Split into opcode and remainder
    my @fields = split /\s+/, $instruction_part;
    unless (@fields) {
        die "parse_ir line $line_num: empty instruction";
    }

    my $opcode_name = $fields[0];
    my ($opcode, $ok) = parse_op($opcode_name);
    unless ($ok) {
        die "parse_ir line $line_num: unknown opcode '$opcode_name'";
    }

    # Parse operands (everything after the opcode, comma-separated)
    my @operands;
    if (@fields > 1) {
        # Rejoin field 1..N and split by comma
        my $operand_str = join(' ', @fields[1..$#fields]);
        my @parts = split /,/, $operand_str;
        if (@parts > MAX_OPERANDS) {
            die sprintf(
                "parse_ir line $line_num: too many operands (%d, max %d)",
                scalar @parts, MAX_OPERANDS
            );
        }
        for my $part (@parts) {
            $part =~ s/^\s+|\s+$//g;
            next if $part eq '';
            my $operand = _parse_operand($part, $line_num);
            push @operands, $operand;
        }
    }

    return CodingAdventures::CompilerIr::IrInstruction->new(
        opcode   => $opcode,
        operands => \@operands,
        id       => $id,
    );
}

# _parse_operand($str, $line_num) — parse a single operand token.
#
# Parsing rules (in order):
#   1. Starts with 'v' followed by digits → IrRegister{index => N}
#   2. Parseable as integer (with optional minus sign) → IrImmediate{value => N}
#   3. Anything else → IrLabel{name => $str}
sub _parse_operand {
    my ($str, $line_num) = @_;

    # Register: v0, v1, v2, ...
    if ($str =~ /^v(\d+)$/) {
        my $idx = int($1);
        if ($idx > MAX_REGISTER_INDEX) {
            die sprintf(
                "parse_ir line $line_num: register index %d out of range (max %d)",
                $idx, MAX_REGISTER_INDEX
            );
        }
        return CodingAdventures::CompilerIr::IrRegister->new($idx);
    }

    # Immediate: 42, -1, 255, ...
    if ($str =~ /^-?\d+$/) {
        return CodingAdventures::CompilerIr::IrImmediate->new(int($str));
    }

    # Label: _start, loop_0_end, tape, ...
    return CodingAdventures::CompilerIr::IrLabel->new($str);
}

1;

__END__

=head1 NAME

CodingAdventures::CompilerIr::Parser - parse IR text into an IrProgram

=head1 SYNOPSIS

  use CodingAdventures::CompilerIr::Parser qw(parse_ir);

  my $program = parse_ir($text);   # or dies on error

=head1 DESCRIPTION

Converts canonical IR text (as produced by
C<CodingAdventures::CompilerIr::Printer>) back into an C<IrProgram>.
Enables golden-file tests and roundtrip verification.

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
