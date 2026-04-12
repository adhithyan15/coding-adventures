package CodingAdventures::BuildTool::Discovery;

# Discovery.pm -- Package Discovery via Recursive BUILD File Walk
# ================================================================
#
# This module walks a monorepo directory tree and discovers "packages" — any
# directory that contains a BUILD file. The walk is recursive: starting from
# the root, we visit every subdirectory, skip known non-source directories
# (.git, node_modules, etc.), and register each directory that has a BUILD
# file as a package.
#
# This mirrors Bazel/Buck/Pants: no configuration files enumerate what
# packages exist. The BUILD file is self-declaring.
#
# Platform-specific BUILD files
# -----------------------------
#
# If we are on macOS and a BUILD_mac file exists alongside BUILD, we use
# BUILD_mac. If we are on Linux and BUILD_linux exists, we use BUILD_linux.
# If neither platform file exists, we fall back to BUILD. This lets packages
# provide OS-specific commands while sharing a generic fallback.
#
# Language inference
# ------------------
#
# We infer the language by scanning the directory path for a known language
# directory name. A path like /repo/code/packages/perl/logic-gates contains
# "perl", so the language is "perl" and the name is "perl/logic-gates".
#
# Perl advantages demonstrated here:
#   - File::Find for recursive walks (core module, no dependencies).
#   - grep {} LIST for filtering — concise and idiomatic.
#   - qw() for literal string lists without commas or quotes.

use strict;
use warnings;
use File::Find ();
use File::Spec ();
use File::Basename ();
use Cwd ();

our $VERSION = '0.01';

# KNOWN_LANGUAGES -- the language directory names we recognise when inferring
# which ecosystem a package belongs to. The order does not matter for
# correctness but we list them alphabetically for readability.
my @KNOWN_LANGUAGES = qw(
    python ruby go rust typescript elixir lua perl swift wasm haskell starlark
    java kotlin csharp fsharp dotnet
);

# SKIP_DIRS -- directory names that we never descend into during the walk.
# These directories contain caches, compiled artifacts, installed dependencies,
# or version-control metadata — none of which contain BUILD files we care about.
#
# We use a hash for O(1) membership testing: `exists $SKIP_DIRS{$name}`.
my %SKIP_DIRS = map { $_ => 1 } qw(
    .git .hg .svn .venv .tox .mypy_cache .pytest_cache .ruff_cache
    __pycache__ node_modules vendor dist build target .claude Pods
    _build blib .build .gradle gradle-build
);

# new -- Constructor.
#
# Args:
#   root => $path   -- Repository root to start the walk from (default: '.').
#
# Returns a blessed hashref with the discovered packages list initially empty.
#
# Example:
#   my $d = CodingAdventures::BuildTool::Discovery->new(root => '/repo');
sub new {
    my ($class, %args) = @_;
    return bless {
        root     => $args{root} // '.',
        packages => [],
    }, $class;
}

# packages -- Accessor returning the discovered packages list.
#
# Each package is a hash reference:
#   {
#     name           => "perl/logic-gates",
#     path           => "/abs/path/to/logic-gates",
#     language       => "perl",
#     build_commands => ["cpanm --installdeps --quiet .", "prove -l -v t/"],
#   }
#
# @return arrayref
sub packages { return $_[0]->{packages} }

# discover -- Walk the repository root and collect all packages.
#
# Populates $self->{packages} with one entry per BUILD file found.
# Packages are sorted by name for deterministic output.
#
# @return $self (for chaining)
sub discover {
    my ($self) = @_;
    my @found;

    # File::Find::find($wanted, @dirs) calls $wanted->() for every file
    # and directory encountered. We use the $File::Find::dir and $_ variables
    # that File::Find sets before each call.
    #
    # The $wanted sub controls traversal via $File::Find::prune: setting it
    # to 1 prevents descending into the current directory.
    #
    # We do two passes conceptually:
    #   Pass 1 (preprocess): prune skip directories before descending.
    #   Pass 2 (wanted):     when we find a BUILD file, register the package.
    #
    # File::Find processes directories BEFORE their contents by default when
    # $File::Find::name is a directory. Setting `no_chdir => 1` keeps the
    # working directory stable (easier to reason about).

    my $root = File::Spec->rel2abs($self->{root});

    File::Find::find(
        {
            wanted => sub {
                my $dir      = $File::Find::dir;
                my $fullpath = $File::Find::name;  # absolute path to current item
                my $basename = File::Basename::basename($fullpath);

                # Prune known non-source directories.
                # $basename is just the last component (e.g. ".git", "node_modules").
                if (-d $fullpath && exists $SKIP_DIRS{$basename}) {
                    $File::Find::prune = 1;
                    return;
                }

                # Only process BUILD files (and their platform variants).
                return unless -f $fullpath;
                return unless $basename =~ /^BUILD(_mac|_linux|_windows|_mac_and_linux)?$/;

                # Choose the right platform BUILD file.
                my $build_file = _choose_build_file($dir);
                return unless defined $build_file;

                # Only register when this is the canonical file for the dir.
                # (Avoids registering the same package multiple times when
                # both BUILD and BUILD_mac exist.)
                my $canonical = File::Spec->catfile($dir, $build_file);
                my $normalized_fullpath = $fullpath;
                my $normalized_canonical = $canonical;
                $normalized_fullpath =~ s{\\}{/}g;
                $normalized_canonical =~ s{\\}{/}g;
                return unless $normalized_fullpath eq $normalized_canonical;

                # Read the BUILD commands — non-blank, non-comment lines.
                my @commands = _read_commands($canonical);

                # Infer language and name from the directory path.
                my $language = _infer_language($dir);
                my $pkg_name = _infer_package_name($dir, $language);

                push @found, {
                    name           => $pkg_name,
                    path           => $dir,
                    language       => $language,
                    build_commands => \@commands,
                };
            },
            no_chdir => 1,
        },
        $root,
    );

    # Sort for deterministic output — same as Go/Python/Ruby implementations.
    @found = sort { $a->{name} cmp $b->{name} } @found;
    $self->{packages} = \@found;
    return $self;
}

# _choose_build_file -- Return the name of the BUILD file to use for $dir.
#
# Priority (most specific wins):
#   1. BUILD_mac     — macOS only
#   2. BUILD_linux   — Linux only
#   3. BUILD_windows — Windows only
#   4. BUILD_mac_and_linux — macOS or Linux (shared Unix file)
#   5. BUILD         — all platforms (generic fallback)
#
# Returns the basename of the chosen file, or undef if none exists.
sub _choose_build_file {
    my ($dir) = @_;
    my $os = _detect_os();

    # Step 1: Platform-specific files.
    if ($os eq 'darwin') {
        my $f = File::Spec->catfile($dir, 'BUILD_mac');
        return 'BUILD_mac' if -f $f;
    }
    if ($os eq 'linux') {
        my $f = File::Spec->catfile($dir, 'BUILD_linux');
        return 'BUILD_linux' if -f $f;
    }
    if ($os eq 'windows') {
        my $f = File::Spec->catfile($dir, 'BUILD_windows');
        return 'BUILD_windows' if -f $f;
    }

    # Step 2: Shared Unix file.
    if ($os eq 'darwin' || $os eq 'linux') {
        my $f = File::Spec->catfile($dir, 'BUILD_mac_and_linux');
        return 'BUILD_mac_and_linux' if -f $f;
    }

    # Step 3: Generic fallback.
    my $f = File::Spec->catfile($dir, 'BUILD');
    return 'BUILD' if -f $f;

    return undef;
}

# _detect_os -- Return a normalised OS name for platform BUILD selection.
#
# We read $^O — Perl's built-in operating system identifier. On macOS it is
# "darwin", on Linux it is "linux", on Windows it is "MSWin32".
#
# Truth table:
#   $^O value    | _detect_os() result
#   -------------|--------------------
#   "darwin"     | "darwin"
#   "linux"      | "linux"
#   "MSWin32"    | "windows"
#   anything else| "unknown"
sub _detect_os {
    return 'darwin'  if $^O eq 'darwin';
    return 'linux'   if $^O eq 'linux';
    return 'windows' if $^O eq 'MSWin32';
    return 'unknown';
}

# _detect_os_for_platform -- Like _detect_os but accepts an explicit OS name.
#
# This is used by tests to verify platform-specific BUILD selection without
# needing to run on the target OS.
#
# @param $os -- One of "darwin", "linux", "windows", "unknown".
# @return Same string (used for testing only).
sub _detect_os_for_platform { return $_[0] }

# choose_build_file_for_platform -- Public version that accepts explicit $os.
#
# Used by tests.
sub choose_build_file_for_platform {
    my ($dir, $os) = @_;
    if ($os eq 'darwin') {
        my $f = File::Spec->catfile($dir, 'BUILD_mac');
        return 'BUILD_mac' if -f $f;
    }
    if ($os eq 'linux') {
        my $f = File::Spec->catfile($dir, 'BUILD_linux');
        return 'BUILD_linux' if -f $f;
    }
    if ($os eq 'windows') {
        my $f = File::Spec->catfile($dir, 'BUILD_windows');
        return 'BUILD_windows' if -f $f;
    }
    if ($os eq 'darwin' || $os eq 'linux') {
        my $f = File::Spec->catfile($dir, 'BUILD_mac_and_linux');
        return 'BUILD_mac_and_linux' if -f $f;
    }
    my $f = File::Spec->catfile($dir, 'BUILD');
    return 'BUILD' if -f $f;
    return undef;
}

# _read_commands -- Read a BUILD file and return executable lines.
#
# We strip blank lines and comment lines (starting with '#'). This is the
# same filtering used by the Go, Python, and Ruby implementations.
#
# @param $path -- Absolute path to the BUILD file.
# @return list of non-blank, non-comment lines.
sub _read_commands {
    my ($path) = @_;
    open(my $fh, '<', $path) or return ();
    my @lines;
    while (<$fh>) {
        chomp;
        s/^\s+|\s+$//g;   # trim whitespace (Perl's equivalent of strip())
        next if $_ eq '';
        next if /^#/;
        push @lines, $_;
    }
    close $fh;
    return @lines;
}

# _infer_language -- Detect the programming language from the directory path.
#
# We split the path on the directory separator and look for the first path
# component that matches a known language name. For example:
#
#   /repo/code/packages/perl/logic-gates  -->  "perl"
#   /repo/code/programs/go/build-tool    -->  "go"
#   /repo/code/packages/python/foo       -->  "python"
#
# If no known language is found, we return "unknown". This keeps the
# implementation lenient — future languages can be added without breaking
# existing packages.
#
# @param $path -- Absolute path to the package directory.
# @return language string or "unknown".
sub _infer_language {
    my ($path) = @_;
    my @parts = File::Spec->splitdir($path);

    # Perl's `grep { COND } LIST` returns matching elements. Here we check
    # each path component against the known language list.
    for my $lang (@KNOWN_LANGUAGES) {
        return $lang if grep { $_ eq $lang } @parts;
    }
    return 'unknown';
}

# _infer_package_name -- Build the qualified package name.
#
# The name follows `{language}/{dir-basename}`. For example:
#   language = "perl", dir = "/repo/code/packages/perl/logic-gates"
#   --> "perl/logic-gates"
#
# @param $path     -- Absolute path to the package directory.
# @param $language -- Inferred language string.
# @return qualified package name.
sub _infer_package_name {
    my ($path, $language) = @_;
    my $basename = File::Basename::basename($path);
    return "$language/$basename";
}

1;

__END__

=head1 NAME

CodingAdventures::BuildTool::Discovery - Package discovery via BUILD file walk

=head1 SYNOPSIS

  use CodingAdventures::BuildTool::Discovery;

  my $d = CodingAdventures::BuildTool::Discovery->new(root => '/repo');
  $d->discover();

  for my $pkg (@{ $d->packages() }) {
      printf "Found: %s (%s)\n", $pkg->{name}, $pkg->{language};
  }

=head1 DESCRIPTION

Recursively walks the monorepo from C<root>, finds directories containing
BUILD files, and returns them as package hash references. Skips known
non-source directories (.git, node_modules, etc.). Infers language from
the directory path. Supports platform-specific BUILD files (BUILD_mac,
BUILD_linux, BUILD_windows).

=cut
