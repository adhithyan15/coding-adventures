package CodingAdventures::BuildTool::Hasher;

# Hasher.pm -- SHA256-Based Content Hashing for Change Detection
# ==============================================================
#
# This module computes a stable hash for each package. The hash is a
# SHA256 digest of all source files in the package, sorted by relative
# path. If any file changes — even a comment — the hash changes, and
# the package is marked dirty and rebuilt.
#
# This is the same strategy used by the Go, Python, and Ruby build tools.
# The hash serves as a "content fingerprint":
#
#   same hash  → no rebuild needed (cache hit)
#   different hash → content changed, rebuild required
#
# What counts as "source"?
# ------------------------
#
# Not all files in a package directory are source files. Build artifacts,
# editor swap files, and log files should not affect the hash. We use two
# allowlists:
#
#   SOURCE_EXTENSIONS -- file suffixes that count as source code.
#   SPECIAL_FILENAMES -- specific filenames that count as source regardless
#                        of extension (e.g., BUILD, Makefile, cpanfile).
#
# Algorithm
# ---------
#
#   1. Walk the package directory recursively with File::Find.
#   2. For each file, check if its extension or basename is on the allowlist.
#   3. Sort the qualifying files by relative path for determinism.
#   4. For each file, compute SHA256 of its contents.
#   5. Feed all hashes (including the relative path) into a final SHA256.
#   6. Return the hex digest.
#
# Including the path in the hash means that renaming a file changes the
# hash even if the content is identical. This is intentional — file
# renames are structural changes.
#
# Perl advantages demonstrated here:
#   - Digest::SHA (core) for cryptographic hashing.
#   - File::Find for recursive traversal without external dependencies.
#   - Hash slices for fast membership testing: $EXTS{$ext}.

use strict;
use warnings;
use Digest::SHA ();
use File::Find ();
use File::Spec ();
use File::Basename ();

our $VERSION = '0.01';

# SOURCE_EXTENSIONS -- file suffixes that are considered source code.
#
# Organised by language. Each key is a language name; each value is a hash
# of extensions (with leading dot) that count as source for that language.
# We use a flat merged hash for fast lookup by extension alone.
my %SOURCE_EXTENSIONS = map { $_ => 1 } (
    # Python
    qw(.py .pyi .toml .cfg .ini),
    # Ruby
    qw(.rb .rake .gemspec .ru),
    # Go
    qw(.go .mod .sum),
    # TypeScript / JavaScript
    qw(.ts .tsx .js .jsx .mjs .cjs .json),
    # Rust
    qw(.rs),
    # Elixir / Erlang
    qw(.ex .exs),
    # Lua
    qw(.lua .rockspec),
    # Perl
    qw(.pm .pl .t .xs .pod),
    # Starlark / BUILD-adjacent
    qw(.star .bzl),
    # C / C++ (for XS extensions and native code)
    qw(.c .h .cpp .cc .cxx .hh .hpp),
    # Documentation and markup (changes to docs trigger rebuilds too)
    qw(.md .rst .txt),
    # YAML / TOML (config files that affect build)
    qw(.yaml .yml),
);

# SPECIAL_FILENAMES -- specific filenames that count as source regardless
# of extension. These are top-level build and config files.
my %SPECIAL_FILENAMES = map { $_ => 1 } qw(
    BUILD BUILD_mac BUILD_linux BUILD_windows BUILD_mac_and_linux
    Makefile.PL Build.PL cpanfile MANIFEST META.json META.yml
    Makefile makefile GNUmakefile
    Gemfile Gemfile.lock .gemspec
    pyproject.toml setup.py setup.cfg requirements.txt
    go.mod go.sum
    package.json package-lock.json tsconfig.json vitest.config.ts
    Cargo.toml Cargo.lock
    mix.exs mix.lock
    .luarocks rockspec
    Dockerfile docker-compose.yml .dockerignore
);

# SKIP_DIRS -- directories to never recurse into when collecting source files.
# Same list as Discovery — avoids double-counting installed deps.
my %SKIP_DIRS = map { $_ => 1 } qw(
    .git .hg .svn .venv .tox __pycache__ node_modules vendor dist
    build target .claude _build blib .mypy_cache .pytest_cache .ruff_cache
);

# new -- Constructor.
sub new {
    my ($class) = @_;
    return bless {}, $class;
}

# hash_package -- Compute a SHA256 fingerprint for a package.
#
# Returns a 64-character hex string (SHA256). Two calls on the same
# package with the same file contents always return the same string.
#
# @param $pkg -- Package hashref with {path => '/abs/path'}.
# @return 64-char hex string.
sub hash_package {
    my ($self, $pkg) = @_;
    my @files = $self->collect_source_files($pkg);
    return $self->_hash_files($pkg->{path}, @files);
}

# collect_source_files -- Return sorted list of source file paths for a package.
#
# Walks the package directory recursively, skipping SKIP_DIRS. Returns
# absolute paths of files that match the source extension or special
# filename allowlists.
#
# @param $pkg -- Package hashref.
# @return sorted list of absolute paths.
sub collect_source_files {
    my ($self, $pkg) = @_;
    my $root = $pkg->{path};
    my @files;

    File::Find::find(
        {
            wanted => sub {
                # Prune skip directories.
                if (-d $_ && exists $SKIP_DIRS{ File::Basename::basename($_) }) {
                    $File::Find::prune = 1;
                    return;
                }
                return unless -f $_;
                my $basename = File::Basename::basename($_);
                my ($ext)    = ($basename =~ /(\.[^.]+)$/);
                $ext //= '';

                if (exists $SOURCE_EXTENSIONS{$ext} || exists $SPECIAL_FILENAMES{$basename}) {
                    push @files, $File::Find::name;
                }
            },
            no_chdir => 1,
        },
        $root,
    );

    # Sort for determinism — same order on every call.
    return sort @files;
}

# _hash_files -- Compute the combined SHA256 of a set of files.
#
# For each file (in sorted order), we feed:
#   1. The relative path of the file (so renames change the hash).
#   2. The file's contents.
# into a single running SHA256 digest.
#
# This means the final hash captures both file names and file contents.
#
# @param $root  -- Package root directory (for computing relative paths).
# @param @files -- Sorted list of absolute file paths.
# @return 64-char hex string.
sub _hash_files {
    my ($self, $root, @files) = @_;

    # Digest::SHA->new(256) creates a SHA256 context.
    # ->add($data) feeds data into the digest.
    # ->hexdigest() finalises and returns the hex string.
    my $sha = Digest::SHA->new(256);

    if (!@files) {
        # Empty package: hash the string "empty" so we still return a valid
        # hex string rather than the hash of nothing.
        $sha->add('empty');
        return $sha->hexdigest();
    }

    for my $file (@files) {
        # Compute the relative path from the package root.
        # File::Spec->abs2rel handles cross-platform path normalisation.
        my $rel = File::Spec->abs2rel($file, $root);

        # Feed the relative path into the digest (including the separator).
        $sha->add($rel);
        $sha->add("\0");  # null byte as separator

        # Feed the file contents.
        open(my $fh, '<:raw', $file) or next;
        $sha->addfile($fh);  # Digest::SHA::addfile reads the filehandle efficiently
        close $fh;
    }

    return $sha->hexdigest();
}

# is_source_extension -- Predicate: is $ext a recognised source extension?
#
# Used in tests to verify the allowlist.
#
# @param $ext -- File extension including the leading dot (e.g. ".pm").
# @return 1 or 0.
sub is_source_extension {
    my ($self, $ext) = @_;
    return exists $SOURCE_EXTENSIONS{$ext} ? 1 : 0;
}

# is_special_filename -- Predicate: is $name a recognised special filename?
#
# @param $name -- Basename without path (e.g. "cpanfile").
# @return 1 or 0.
sub is_special_filename {
    my ($self, $name) = @_;
    return exists $SPECIAL_FILENAMES{$name} ? 1 : 0;
}

1;

__END__

=head1 NAME

CodingAdventures::BuildTool::Hasher - SHA256 content hashing for packages

=head1 SYNOPSIS

  use CodingAdventures::BuildTool::Hasher;

  my $h = CodingAdventures::BuildTool::Hasher->new();
  my $hash = $h->hash_package({ path => '/repo/code/packages/perl/logic-gates' });
  print "Hash: $hash\n";  # 64-char hex string

=head1 DESCRIPTION

Computes a deterministic SHA256 fingerprint for each package based on the
content of its source files. Uses C<Digest::SHA> (a Perl core module). The
hash changes if any source file is added, modified, renamed, or removed.

=cut
