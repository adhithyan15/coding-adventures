package CodingAdventures::BuildTool;

# BuildTool.pm -- Main Orchestrator: Discovery → Resolution → Execution
# ======================================================================
#
# This is the top-level module that assembles all the pieces into a
# complete build pipeline:
#
#   1. Discovery   -- Find all packages by walking the repository tree.
#   2. Resolver    -- Read dependency files and build the dependency graph.
#   3. Change detection -- Either git diff (default) or hash-based cache.
#   4. Executor    -- Run BUILD commands in dependency order, in parallel.
#   5. Reporter    -- Print human-readable results.
#
# All business logic lives in the sub-modules. BuildTool.pm is "the glue"
# that wires them together and manages the CLI flags.
#
# Pipeline diagram:
#
#   --force mode:
#     Discovery -> Resolver -> all packages -> Executor -> Reporter
#
#   --diff-base mode (default):
#     Discovery -> Resolver -> GitDiff -> affected packages -> Executor -> Reporter
#
#   --dry-run mode:
#     Discovery -> Resolver -> GitDiff -> Plan -> print plan (no execution)
#
# Perl advantages demonstrated here:
#   - Module loading with `use` at compile time.
#   - Chained method calls: $d->discover()->packages().
#   - Array filtering with grep{}.

use strict;
use warnings;
use Cwd ();
use File::Spec ();
use List::Util qw(any);

use CodingAdventures::BuildTool::Discovery  ();
use CodingAdventures::BuildTool::Resolver   ();
use CodingAdventures::BuildTool::Hasher     ();
use CodingAdventures::BuildTool::Executor   ();
use CodingAdventures::BuildTool::Cache      ();
use CodingAdventures::BuildTool::GitDiff    ();
use CodingAdventures::BuildTool::Reporter   ();
use CodingAdventures::BuildTool::Plan       ();
use CodingAdventures::BuildTool::Validator  ();

our $VERSION = '0.01';

# new -- Constructor.
#
# Args:
#   root      => $path   -- Repository root (default: cwd).
#   force     => $bool   -- Rebuild everything, ignore cache.
#   dry_run   => $bool   -- Show plan without executing.
#   jobs      => $n      -- Max parallel builds (default: CPU count).
#   language  => $lang   -- Build only this language.
#   diff_base => $ref    -- Git ref for change detection (default: "origin/main").
#   verbose   => $bool   -- Extra output.
sub new {
    my ($class, %args) = @_;
    return bless {
        root      => $args{root}      // Cwd::cwd(),
        force     => $args{force}     // 0,
        dry_run   => $args{dry_run}   // 0,
        jobs      => $args{jobs}      // undef,
        language  => $args{language}  // undef,
        diff_base => $args{diff_base} // 'origin/main',
        verbose   => $args{verbose}   // 0,
        validate_build_files => $args{validate_build_files} // 0,
    }, $class;
}

# run -- Execute the full build pipeline.
#
# Returns 0 (all passed), 1 (some failed), or 2 (configuration error).
sub run {
    my ($self) = @_;

    # Step 1: Discover all packages.
    my $discovery = CodingAdventures::BuildTool::Discovery->new(root => $self->{root});
    $discovery->discover();
    my @all_packages = @{ $discovery->packages() };

    if (!@all_packages) {
        warn "No packages found under $self->{root}\n";
        return 2;
    }

    # Step 2: Filter by language (if --language was specified).
    if (defined $self->{language}) {
        @all_packages = grep { $_->{language} eq $self->{language} } @all_packages;
    }

    if ($self->{validate_build_files}) {
        my $validation_error =
            CodingAdventures::BuildTool::Validator::validate_build_contracts(
                $self->{root},
                \@all_packages,
            );
        if (defined $validation_error) {
            warn "BUILD/CI validation failed:\n";
            warn "  - $validation_error\n";
            warn "Fix the BUILD file or CI workflow so isolated and full-build runs stay correct.\n";
            return 1;
        }
    }

    # Step 3: Resolve dependencies.
    my $resolver = CodingAdventures::BuildTool::Resolver->new();
    my $graph    = $resolver->resolve(\@all_packages);

    # Step 4: Determine which packages need building.
    my @to_build;

    if ($self->{force}) {
        @to_build = @all_packages;
    } else {
        # Use git diff to find changed packages.
        my $gitdiff = CodingAdventures::BuildTool::GitDiff->new(
            root      => $self->{root},
            diff_base => $self->{diff_base},
        );
        my @affected_names = $gitdiff->affected_packages(\@all_packages, $graph);

        # Filter to packages in the affected set.
        my %affected = map { $_ => 1 } @affected_names;
        @to_build = grep { $affected{ $_->{name} } } @all_packages;

        if (!@to_build) {
            print "No packages changed. Nothing to build.\n";
            return 0;
        }
    }

    # Step 5: Dry-run mode — print the plan and exit.
    if ($self->{dry_run}) {
        my $planner = CodingAdventures::BuildTool::Plan->new();
        my %pkg_map = map { $_->{name} => $_ } @to_build;
        my $plan    = $planner->build(\@to_build, $graph, \%pkg_map);
        $planner->print_plan($plan);
        return 0;
    }

    # Step 6: Execute builds.
    my %executor_args = (dry_run => 0, verbose => $self->{verbose});
    $executor_args{max_jobs} = $self->{jobs} if defined $self->{jobs};

    my $executor = CodingAdventures::BuildTool::Executor->new(%executor_args);
    my $ok       = $executor->execute(\@to_build, $graph);
    my @results  = @{ $executor->results() };

    # Step 7: Report.
    my $reporter = CodingAdventures::BuildTool::Reporter->new(
        verbose => $self->{verbose},
    );
    $reporter->report(\@results);
    $reporter->summary(\@results);

    return $ok ? 0 : 1;
}

# plan -- Return the build plan without executing.
#
# @return Plan hashref (from CodingAdventures::BuildTool::Plan->build).
sub plan {
    my ($self) = @_;

    my $discovery = CodingAdventures::BuildTool::Discovery->new(root => $self->{root});
    $discovery->discover();
    my @packages = @{ $discovery->packages() };

    my $resolver = CodingAdventures::BuildTool::Resolver->new();
    my $graph    = $resolver->resolve(\@packages);

    my $planner = CodingAdventures::BuildTool::Plan->new();
    my %pkg_map = map { $_->{name} => $_ } @packages;
    return $planner->build(\@packages, $graph, \%pkg_map);
}

1;

__END__

=head1 NAME

CodingAdventures::BuildTool - Incremental parallel monorepo build system

=head1 VERSION

0.01

=head1 SYNOPSIS

  use CodingAdventures::BuildTool;

  my $bt = CodingAdventures::BuildTool->new(
      root      => '/path/to/repo',
      force     => 0,
      dry_run   => 0,
      jobs      => 4,
      diff_base => 'origin/main',
  );
  exit $bt->run();

=head1 DESCRIPTION

The Perl port of the coding-adventures monorepo build tool. Discovers
packages by walking the repository tree, resolves dependencies, detects
changes via git diff, and executes BUILD commands in parallel using fork().

This is an educational implementation that demonstrates:
  - File::Find for recursive directory walking
  - Digest::SHA for content hashing
  - JSON::PP for cache serialisation
  - fork() for parallel execution
  - Native regex for config file parsing

=head1 OPTIONS

=over 4

=item B<root> => $path

Repository root directory. Defaults to the current working directory.

=item B<force> => $bool

Rebuild all packages, ignoring git diff.

=item B<dry_run> => $bool

Print the build plan without executing any commands.

=item B<jobs> => $n

Maximum number of parallel build processes. Defaults to the number of
logical CPUs.

=item B<language> => $lang

Build only packages of the given language (e.g. "perl", "python").

=item B<diff_base> => $ref

Git ref to diff against. Defaults to "origin/main".

=item B<verbose> => $bool

Print extra output including build command output for passing packages.

=back

=head1 EXIT CODES

  0  All builds succeeded (or nothing to build)
  1  One or more builds failed
  2  Configuration error

=cut
