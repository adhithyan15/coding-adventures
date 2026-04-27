package CodingAdventures::BuildTool::GitDiff;

# GitDiff.pm -- Git-Based Change Detection (Primary Build Mode)
# =============================================================
#
# Instead of rebuilding all packages on every run, the build tool computes
# which packages have changed since a base commit (usually origin/main).
# Only those packages — and their transitive dependents — need rebuilding.
#
# How it works:
#
#   1. Run `git diff --name-only <base>...HEAD` to get changed files.
#   2. Map each changed file to a package by finding its longest prefix
#      match among known package paths.
#   3. Use graph->affected_nodes() to find packages that depend on the
#      changed packages (transitively).
#   4. Return the union of directly changed and transitively affected.
#
# The three-dot diff (`base...HEAD`) uses the merge base, which is the
# correct comparison for PRs: it shows files changed since the branch
# diverged from main, not files different from the current main tip.
#
# If the three-dot diff fails (e.g., no common history), we fall back
# to the two-dot diff (`base..HEAD`).
#
# Perl advantages demonstrated here:
#   - Backtick operator (`) for shell commands — simpler than subprocess.
#   - qw() for flags list.
#   - Regex for path matching: `$path =~ /^\Q$pkg_path\E/`.
#   - \Q...\E escapes metacharacters in the path for safe regex use.

use strict;
use warnings;
use File::Spec ();
use CodingAdventures::BuildTool::CIWorkflow ();

our $VERSION = '0.01';

# new -- Constructor.
#
# Args:
#   root      => $path   -- Repository root directory.
#   diff_base => $ref    -- Git ref to diff against (default: "origin/main").
sub new {
    my ($class, %args) = @_;
    return bless {
        root      => $args{root}      // '.',
        diff_base => $args{diff_base} // 'origin/main',
    }, $class;
}

# changed_files -- Return a list of files changed since diff_base.
#
# Tries the three-dot diff first. If it fails (no common ancestor), falls
# back to the two-dot diff.
#
# @return list of file paths relative to the repository root.
sub changed_files {
    my ($self) = @_;
    my $base = $self->{diff_base};
    my $root = $self->{root};

    # Three-dot diff: changes since the merge base (correct for PRs).
    my $cmd3   = "git -C \Q$root\E diff --name-only $base...HEAD 2>/dev/null";
    my $output = `$cmd3`;

    if ($? != 0 || !defined $output) {
        # Fall back to two-dot diff.
        my $cmd2 = "git -C \Q$root\E diff --name-only $base..HEAD 2>/dev/null";
        $output  = `$cmd2`;
    }

    return () unless defined $output && $output ne '';

    # Split on newlines, strip whitespace, filter empties.
    my @files = grep { $_ ne '' } map { s/^\s+|\s+$//gr } split(/\n/, $output);
    return @files;
}

# affected_packages -- Return packages affected by the given changed files.
#
# Maps each changed file to its package (by path prefix), then uses the
# graph to find all transitively affected packages.
#
# @param \@packages -- All known packages.
# @param $graph     -- Dependency graph (CodingAdventures::BuildTool::Graph).
# @return list of package names that need rebuilding.
sub affected_packages {
    my ($self, $packages_ref, $graph) = @_;
    my $root = File::Spec->rel2abs($self->{root});

    my @changed_files = $self->changed_files();
    return () unless @changed_files;

    if (grep { $_ eq CodingAdventures::BuildTool::CIWorkflow::ci_workflow_path() } @changed_files) {
        my $ci_change = CodingAdventures::BuildTool::CIWorkflow::analyze_changes(
            $self->{root},
            $self->{diff_base},
        );

        if ($ci_change->{requires_full_rebuild}) {
            print "Git diff: ci.yml changed in shared ways -- rebuilding everything\n";
            return map { $_->{name} } @{$packages_ref};
        }

        my @toolchains =
            CodingAdventures::BuildTool::CIWorkflow::sorted_toolchains($ci_change->{toolchains});
        if (@toolchains) {
            print "Git diff: ci.yml changed only toolchain-scoped setup for "
                . join(', ', @toolchains) . "\n";
        }
    }

    # Map changed files to packages.
    my %directly_affected;
    for my $file (@changed_files) {
        # Make the file path absolute for prefix matching.
        my $abs_file = File::Spec->catfile($root, $file);
        $abs_file =~ s{\\}{/}g;

        for my $pkg (@{$packages_ref}) {
            my $pkg_path = $pkg->{path};
            $pkg_path =~ s{\\}{/}g;
            # Use \Q...\E to escape any regex metacharacters in the path.
            # The path separator is a metacharacter on Windows.
            if ($abs_file =~ /^\Q$pkg_path\E(\/|$)/) {
                $directly_affected{ $pkg->{name} } = 1;
            }
        }
    }

    # Add transitively affected packages (packages that depend on the changed ones).
    my @roots = keys %directly_affected;
    my @all_affected = $graph->affected_nodes(\@roots);

    return @all_affected;
}

1;

__END__

=head1 NAME

CodingAdventures::BuildTool::GitDiff - Git-based change detection

=head1 SYNOPSIS

  use CodingAdventures::BuildTool::GitDiff;

  my $gd = CodingAdventures::BuildTool::GitDiff->new(
      root      => '/repo',
      diff_base => 'origin/main',
  );

  my @affected = $gd->affected_packages(\@packages, $graph);

=head1 DESCRIPTION

Uses C<git diff> to determine which files changed since C<diff_base>. Maps
changed files to their packages and follows the dependency graph to include
transitively affected packages.

=cut
