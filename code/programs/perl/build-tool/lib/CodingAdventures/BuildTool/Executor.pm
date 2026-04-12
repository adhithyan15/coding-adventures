package CodingAdventures::BuildTool::Executor;

# Executor.pm -- Parallel Build Execution via fork()
# ===================================================
#
# This module executes BUILD commands for packages, honouring the
# dependency order produced by the graph's independent_groups() method.
# It uses Unix fork() for parallelism.
#
# Why fork() instead of threads?
# --------------------------------
#
# Perl's thread model (`use threads`) is heavyweight: each thread gets its
# own copy of the Perl interpreter, including all loaded modules. This is
# expensive in memory and startup time.
#
# fork() is lighter on Unix:
#   - The kernel uses copy-on-write — child processes share pages until
#     they write to them.
#   - Each child process is independent: no shared state to protect.
#   - Shell command execution (system()) is the bottleneck anyway, not
#     Perl code.
#
# The parent process forks one child per package in the current independent
# group (up to --jobs children at a time). Each child runs its BUILD
# commands, then exits with 0 (all passed) or 1 (any failed).
#
# Semaphore pattern:
#
#   while packages remain in the group:
#       if active_children < max_jobs:
#           fork a child for the next package
#       else:
#           waitpid(-1, 0)  # reap one child
#       check exit status of reaped child
#   waitpid all remaining children
#
# Windows fallback:
#
# fork() is unavailable on native Windows (without WSL). We detect this at
# runtime and fall back to sequential execution. CI runs on Linux/macOS, so
# this is a graceful degradation rather than a blocking limitation.
#
# Perl advantages demonstrated here:
#   - fork() / waitpid() / POSIX::WIFEXITED() for process control.
#   - $$ (PID), $! (errno), $? (child exit status).
#   - Local variables as closure captures (for child process isolation).
#   - time() for timing each build.

use strict;
use warnings;
use POSIX qw(WIFEXITED WEXITSTATUS WNOHANG);
use Cwd ();
use File::Spec ();

our $VERSION = '0.01';
our $WINDOWS_BASH;

# new -- Constructor.
#
# Args:
#   max_jobs => $n    -- Max concurrent build processes (default: number of CPUs).
#   dry_run  => $bool -- Print commands but don't execute them.
#   verbose  => $bool -- Extra output.
#
# Example:
#   my $e = CodingAdventures::BuildTool::Executor->new(max_jobs => 4);
sub new {
    my ($class, %args) = @_;
    return bless {
        max_jobs => $args{max_jobs} // _cpu_count(),
        dry_run  => $args{dry_run}  // 0,
        verbose  => $args{verbose}  // 0,
        results  => [],
    }, $class;
}

# results -- Return the list of build results accumulated by execute().
#
# Each result is a hashref:
#   {
#     package  => "perl/logic-gates",
#     status   => "pass" | "fail" | "skip",
#     duration => 3.14,              # seconds (float)
#     output   => "...",             # combined stdout+stderr
#   }
sub results { return $_[0]->{results} }

# execute -- Run all packages in dependency order.
#
# Processes independent groups one at a time. Within each group, packages
# can run in parallel (up to max_jobs). Failed packages cause their
# dependents to be skipped.
#
# @param \@packages  -- arrayref of package hashrefs.
# @param $graph      -- CodingAdventures::BuildTool::Graph instance.
# @return 1 if all builds passed, 0 if any failed.
sub execute {
    my ($self, $packages_ref, $graph) = @_;
    $self->{results} = [];

    # Build a lookup from package name to hashref.
    my %pkg_map = map { $_->{name} => $_ } @{$packages_ref};

    # Track which packages have failed (so we can skip their dependents).
    my %failed;

    my @groups = $graph->independent_groups();

    for my $group (@groups) {
        # Filter to packages we actually know about (graph may include
        # packages not in the current build set, e.g. when --language filters).
        my @to_build = grep { exists $pkg_map{$_} } @{$group};

        for my $name (@to_build) {
            my $pkg = $pkg_map{$name};

            # Check if any dependency failed.
            my @deps = $graph->predecessors($name);
            my $dep_failed = grep { $failed{$_} } @deps;

            if ($dep_failed) {
                push @{ $self->{results} }, {
                    package  => $name,
                    status   => 'skip',
                    duration => 0,
                    output   => "Skipped: a dependency failed.",
                };
                $failed{$name} = 1;
                next;
            }
        }

        # Build the packages that are not already skipped.
        my @runnable = grep {
            my $name = $_;
            !grep { $_->{package} eq $name } @{ $self->{results} }
        } @to_build;

        if (@runnable) {
            my @build_results = $self->_run_group(\@runnable, \%pkg_map);
            push @{ $self->{results} }, @build_results;

            for my $r (@build_results) {
                $failed{ $r->{package} } = 1 if $r->{status} eq 'fail';
            }
        }
    }

    my $any_failed = grep { $_->{status} eq 'fail' } @{ $self->{results} };
    return $any_failed ? 0 : 1;
}

# _run_group -- Execute a group of packages, possibly in parallel.
#
# Forks up to max_jobs children. Each child runs all BUILD commands for one
# package. The parent waits for all children.
#
# @param \@names   -- package names to build.
# @param \%pkg_map -- name => package hashref.
# @return list of result hashrefs.
sub _run_group {
    my ($self, $names_ref, $pkg_map_ref) = @_;

    # If fork() is not available (Windows), fall back to sequential.
    my $can_fork = ($^O ne 'MSWin32');

    if (!$can_fork || $self->{max_jobs} == 1 || @{$names_ref} == 1) {
        return map { $self->_run_single($pkg_map_ref->{$_}) } @{$names_ref};
    }

    # Parallel execution via fork().
    #
    # We use a pipe to communicate results from child to parent:
    #   child writes JSON result hash to the write end.
    #   parent reads from the read end after waitpid().
    #
    # Why a pipe? Because after fork(), the child and parent have separate
    # memory spaces. The child cannot directly modify the parent's %results.

    my %children;  # pid => { name, pipe_read_fh }
    my @results;
    my @pending = @{$names_ref};

    while (@pending || %children) {
        # Fork new children up to max_jobs.
        while (@pending && scalar(keys %children) < $self->{max_jobs}) {
            my $name = shift @pending;
            my $pkg  = $pkg_map_ref->{$name};

            # Create a pipe: $rd is for reading, $wr is for writing.
            pipe(my $rd, my $wr) or die "pipe: $!";

            my $pid = fork();
            die "fork: $!" unless defined $pid;

            if ($pid == 0) {
                # === Child process ===
                close $rd;

                my $result = $self->_run_single($pkg);

                # Serialise the result as a simple text record.
                # We use a simple key=value format to avoid requiring JSON::PP
                # in the child (though it's available — this is simpler).
                my $output = $result->{output} // '';
                $output =~ s/\|/\\|/g;  # escape our delimiter in output
                my $line = join('|',
                    $result->{status},
                    $result->{duration},
                    $output,
                );
                print $wr $line;
                close $wr;
                POSIX::_exit($result->{status} eq 'pass' ? 0 : 1);
            }

            # === Parent process ===
            close $wr;
            $children{$pid} = { name => $name, rd => $rd };
        }

        # Reap one child.
        last unless %children;
        my $pid = waitpid(-1, 0);
        next if $pid <= 0;

        my $child = delete $children{$pid};
        next unless defined $child;

        # Read the result from the pipe.
        my $rd = $child->{rd};
        local $/;
        my $line = <$rd>;
        close $rd;

        my ($status, $duration, $output) = defined $line
            ? split(/\|/, $line, 3)
            : ('fail', 0, 'No output from child process');
        $output //= '';
        $output =~ s/\\\|/|/g;  # unescape

        push @results, {
            package  => $child->{name},
            status   => $status // 'fail',
            duration => $duration // 0,
            output   => $output,
        };
    }

    return @results;
}

# _run_single -- Execute all BUILD commands for one package sequentially.
#
# Records the combined output and total duration. Returns a result hashref.
#
# @param $pkg -- Package hashref.
# @return result hashref.
sub _run_single {
    my ($self, $pkg) = @_;
    my $name  = $pkg->{name};
    my $path  = $pkg->{path};
    my @cmds  = @{ $pkg->{build_commands} // [] };

    if (!@cmds) {
        return {
            package  => $name,
            status   => 'pass',
            duration => 0,
            output   => 'No build commands.',
        };
    }

    if ($self->{dry_run}) {
        my $preview = join("\n", @cmds);
        return {
            package  => $name,
            status   => 'pass',
            duration => 0,
            output   => "[dry-run] Would run:\n$preview",
        };
    }

    my $start  = time();
    my @output;
    my $status = 'pass';

    for my $cmd (@cmds) {
        my ($out, $exit_code) = $self->_run_command_in_dir($path, $cmd);

        push @output, "\$ $cmd\n$out";  # \$ is a literal $ ($ $cmd would deref $cmd)

        if ($exit_code != 0) {
            $status = 'fail';
            last;  # stop on first failure (same as Go implementation)
        }
    }

    my $duration = time() - $start;

    return {
        package  => $name,
        status   => $status,
        duration => $duration,
        output   => join("\n", @output),
    };
}

sub _run_command_in_dir {
    my ($self, $path, $cmd) = @_;

    my $cwd = Cwd::getcwd();
    if (!chdir $path) {
        return ("chdir $path failed: $!\n", 1);
    }

    my @shell = _shell_argv($cmd);
    my $output = '';

    my $opened = open(my $fh, '-|', @shell);
    if (!$opened) {
        my $err = "failed to start command: $!\n";
        chdir $cwd or die "chdir $cwd failed: $!";
        return ($err, 1);
    }

    local $/;
    $output = <$fh> // '';
    close $fh;
    my $exit_code = $? >> 8;

    chdir $cwd or die "chdir $cwd failed: $!";
    return ($output, $exit_code);
}

sub _shell_argv {
    my ($cmd) = @_;
    my $redirected = "$cmd 2>&1";

    if ($^O eq 'MSWin32') {
        my $bash = _windows_bash();
        return ($bash, '-lc', _command_for_windows_bash($redirected)) if defined $bash;
        return ('cmd', '/d', '/s', '/c', $redirected);
    }

    return ('sh', '-lc', $redirected);
}

sub _command_for_windows_bash {
    my ($cmd) = @_;
    $cmd =~ s{([A-Za-z]):([\\/][^\s'"]*)}{_bashify_windows_path($1, $2)}ge;
    return $cmd;
}

sub _bashify_windows_path {
    my ($drive, $rest) = @_;
    $rest =~ s{\\}{/}g;
    return '/' . lc($drive) . $rest;
}

sub _windows_bash {
    return $WINDOWS_BASH if defined $WINDOWS_BASH;

    my @candidates = grep { defined $_ && $_ ne '' } (
        File::Spec->catfile($ENV{ProgramFiles} // '', 'Git', 'bin', 'bash.exe'),
        File::Spec->catfile($ENV{'ProgramFiles(x86)'} // '', 'Git', 'bin', 'bash.exe'),
    );

    for my $candidate (@candidates) {
        if (-f $candidate) {
            $WINDOWS_BASH = $candidate;
            return $WINDOWS_BASH;
        }
    }

    $WINDOWS_BASH = undef;
    return $WINDOWS_BASH;
}

# _cpu_count -- Detect the number of logical CPUs.
#
# We try several approaches in order:
#   1. /proc/cpuinfo (Linux)
#   2. sysctl hw.logicalcpu (macOS)
#   3. Fall back to 1
#
# This mirrors Go's runtime.NumCPU() and Python's os.cpu_count().
sub _cpu_count {
    if ($^O eq 'linux' && -f '/proc/cpuinfo') {
        open(my $fh, '<', '/proc/cpuinfo') or return 1;
        my $count = 0;
        while (<$fh>) {
            $count++ if /^processor\s*:/;
        }
        close $fh;
        return $count || 1;
    }

    if ($^O eq 'darwin') {
        my $n = `sysctl -n hw.logicalcpu 2>/dev/null`;
        chomp $n;
        return $n + 0 if $n =~ /^\d+$/;
    }

    return 1;
}

1;

__END__

=head1 NAME

CodingAdventures::BuildTool::Executor - Parallel build execution via fork()

=head1 SYNOPSIS

  use CodingAdventures::BuildTool::Executor;

  my $e = CodingAdventures::BuildTool::Executor->new(max_jobs => 4);
  my $ok = $e->execute(\@packages, $graph);

  for my $r (@{ $e->results() }) {
      printf "%s: %s (%.1fs)\n", $r->{package}, $r->{status}, $r->{duration};
  }

=head1 DESCRIPTION

Executes package BUILD commands in dependency order. Uses C<fork()> for
parallelism on Unix systems, falling back to sequential execution on Windows.
Respects the C<--jobs> concurrency limit.

=cut
