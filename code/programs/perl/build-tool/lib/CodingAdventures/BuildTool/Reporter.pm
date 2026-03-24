package CodingAdventures::BuildTool::Reporter;

# Reporter.pm -- Human-Readable Build Output
# ==========================================
#
# This module formats the build results for human consumption. It prints
# a per-package status line and a summary at the end.
#
# Output format:
#
#   [PASS] perl/logic-gates (2.1s)
#   [FAIL] perl/arithmetic (3.4s)
#     $ prove -l -v t/
#     not ok 1 - addition
#   [SKIP] perl/bitset (dep failed)
#
#   Build Summary:
#     Total:   3
#     Passed:  1
#     Failed:  1
#     Skipped: 1
#
# Perl advantages demonstrated here:
#   - printf / sprintf for formatted output.
#   - Heredocs (<<~"END") for multi-line strings with clean indentation.
#   - String repetition operator (x) for separator lines.

use strict;
use warnings;

our $VERSION = '0.01';

# STATUS_LABELS -- ANSI colour codes for status labels.
#
# Green for PASS, red for FAIL, yellow for SKIP. These are standard ANSI
# escape sequences:
#   \e[32m   -- foreground green
#   \e[31m   -- foreground red
#   \e[33m   -- foreground yellow
#   \e[0m    -- reset to default
#
# We only apply colours when the output is a terminal (STDOUT is a tty).
my %STATUS_LABEL = (
    pass => 'PASS',
    fail => 'FAIL',
    skip => 'SKIP',
);

my %STATUS_COLOUR = (
    pass => "\e[32m",
    fail => "\e[31m",
    skip => "\e[33m",
);

my $RESET = "\e[0m";

# new -- Constructor.
#
# Args:
#   colour => $bool  -- Emit ANSI colour codes (default: auto-detect from isatty).
#   verbose => $bool -- Print full build output for each package.
sub new {
    my ($class, %args) = @_;
    my $colour = exists $args{colour} ? $args{colour} : (-t STDOUT ? 1 : 0);
    return bless {
        colour  => $colour,
        verbose => $args{verbose} // 0,
    }, $class;
}

# report -- Print a line for each build result.
#
# @param \@results -- list of result hashrefs from Executor.
sub report {
    my ($self, $results_ref) = @_;

    if (!@{$results_ref}) {
        print "Nothing to build.\n";
        return;
    }

    for my $r (@{$results_ref}) {
        $self->_print_result($r);
    }
}

# summary -- Print the final summary line.
#
# @param \@results -- list of result hashrefs.
sub summary {
    my ($self, $results_ref) = @_;

    my $total   = scalar @{$results_ref};
    my $passed  = scalar grep { $_->{status} eq 'pass' } @{$results_ref};
    my $failed  = scalar grep { $_->{status} eq 'fail' } @{$results_ref};
    my $skipped = scalar grep { $_->{status} eq 'skip' } @{$results_ref};

    my $sep = '-' x 40;

    # Heredoc with interpolation. The <<~"END" form strips leading whitespace,
    # so we can indent the body for readability without affecting output.
    print <<~"END";
        $sep
        Build Summary:
          Total:   $total
          Passed:  $passed
          Failed:  $failed
          Skipped: $skipped
        END

    if ($failed > 0) {
        print "\nFailed packages:\n";
        for my $r (grep { $_->{status} eq 'fail' } @{$results_ref}) {
            printf "  - %s\n", $r->{package};
        }
    }
}

# _print_result -- Print one result line (and optionally the build output).
sub _print_result {
    my ($self, $r) = @_;

    my $status  = $r->{status} // 'fail';
    my $name    = $r->{package};
    my $dur     = $r->{duration} // 0;
    my $label   = $STATUS_LABEL{$status} // uc($status);

    if ($self->{colour}) {
        my $colour = $STATUS_COLOUR{$status} // '';
        printf "%s[%s]%s %s (%.1fs)\n", $colour, $label, $RESET, $name, $dur;
    } else {
        printf "[%s] %s (%.1fs)\n", $label, $name, $dur;
    }

    # In verbose mode (or on failure), print the build output.
    if ($self->{verbose} || $status eq 'fail') {
        my $output = $r->{output} // '';
        if ($output && $output ne '') {
            # Indent each output line with two spaces.
            for my $line (split /\n/, $output) {
                print "  $line\n";
            }
        }
    }
}

1;

__END__

=head1 NAME

CodingAdventures::BuildTool::Reporter - Human-readable build output

=head1 SYNOPSIS

  use CodingAdventures::BuildTool::Reporter;

  my $r = CodingAdventures::BuildTool::Reporter->new(colour => 1);
  $r->report(\@results);
  $r->summary(\@results);

=head1 DESCRIPTION

Formats build results as human-readable terminal output with optional ANSI
colours. Prints per-package status lines and a summary.

=cut
