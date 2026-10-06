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
# Not all files in a package directory are source files. The installed
# language-source-input registry supplies seven exact selector roles; a
# package's language controls its suffixes and metadata names. Generated
# directory components are pruned before either declared or extension mode.
#
# Algorithm
# ---------
#
#   1. Walk the package directory recursively with File::Find.
#   2. Select package-local inputs through the same registry projection that
#      the neutral inert-candidate tests exercise.
#   3. Sort normalized relative paths by raw UTF-8 bytes.
#   4. Frame each path and content body with unsigned 64-bit byte lengths.
#   5. SHA256-hash the framed stream (empty selection = SHA256 of empty bytes).
#
# Including the path in the hash means that renaming a file changes the
# hash even if the content is identical. This is intentional — file
# renames are structural changes.
#
# Perl advantages demonstrated here:
#   - Digest::SHA (core) for cryptographic hashing.
#   - File::Find for recursive traversal without external dependencies.
#   - JSON::PP for a complete installed registry snapshot.

use strict;
use warnings;
use Digest::SHA ();
use File::Find ();
use File::Spec ();
use File::Basename ();
use Encode qw(encode);
use JSON::PP ();
use CodingAdventures::BuildTool::GlobMatch ();
use CodingAdventures::BuildTool::SourceInputRegistry ();

our $VERSION = '0.01';

# new -- Constructor.
sub new {
    my ($class) = @_;
    return bless { registry => bless({}, 'CodingAdventures::BuildTool::SourceInputRegistry') }, $class;
}

sub registry_bytes  { return $_[0]->{registry}->bytes() }
sub registry_digest { return $_[0]->{registry}->digest() }

# Resolve the language encoded by a discovered package. A test-only isolated
# directory may omit a canonical root; it never gains exact-package rules.
sub _identity {
    my ($self, $pkg) = @_;
    my $root = $pkg->{package_root};
    if (!defined($root) && defined($pkg->{path})) {
        my $path = $pkg->{path};
        $path =~ s{\\}{/}g;
        $root = $1 if $path =~ m{(?:\A|/)(code/(?:packages|programs)/[^/]+/[^/]+(?:/[^/]+)*|code/sites/[^/]+)\z};
    }
    my $language = $pkg->{language};
    $language = (split m{/}, $pkg->{name} // '')[0] unless defined $language;
    $language = 'typescript' if defined($root) && $root =~ m{\Acode/sites/(?:blog|landing-page)\z}
        && $language eq 'unknown';
    return ($language, $root);
}

sub select_source_paths {
    my ($self, $pkg, $candidates) = @_;
    my ($language, $root) = $self->_identity($pkg);
    my $mode = $pkg->{source_mode} // 'extension';
    my $srcs = $pkg->{declared_srcs} // [];
    my @compiled;
    if ($mode eq 'declared_sources') {
        die 'DECLARED_GLOB_INVALID: expected an array' unless ref($srcs) eq 'ARRAY';
        die 'DECLARED_GLOB_INVALID: too many patterns' if @$srcs > 256;
        my $matcher = CodingAdventures::BuildTool::GlobMatch->new();
        for my $pattern (@$srcs) {
            die 'DECLARED_GLOB_INVALID: unsafe pattern'
                unless defined($pattern) && !ref($pattern)
                && length($pattern) > 0 && length($pattern) <= 4096
                && $pattern =~ m{\A[A-Za-z0-9_.+/*?\[\]!^-]+\z}
                && $pattern !~ m{(?:\A|/)\.\.?(?:/|\z)|//|\A/|/\z|\*\*\*|--|&&|~~|\|\|};
            my $compiled = eval { $matcher->glob_to_regex($pattern) };
            die 'DECLARED_GLOB_INVALID: malformed pattern' if $@ || !defined $compiled;
            push @compiled, $compiled;
        }
    }
    return $self->{registry}->select(
        $language, $root, $mode, $srcs, $candidates,
        sub { my ($path) = @_; return scalar(grep { $path =~ $_ } @compiled) },
    );
}

sub hash_source_records {
    my ($self, $records) = @_;
    die 'SOURCE_RECORDS_INVALID: expected array' unless ref($records) eq 'ARRAY';
    my $sha = Digest::SHA->new(256);
    for my $record (sort {
        encode('UTF-8', $a->{path}) cmp encode('UTF-8', $b->{path})
    } @$records) {
        die 'SOURCE_RECORDS_INVALID: record' unless ref($record) eq 'HASH'
            && defined($record->{path}) && defined($record->{content});
        my $path = encode('UTF-8', $record->{path});
        my $bytes = $record->{content};
        die 'SOURCE_HASH_LIMIT_EXCEEDED: file' if length($bytes) > 64 * 1024 * 1024;
        $sha->add(pack('Q>', length($path)), $path,
            pack('Q>', length($bytes)), $bytes);
    }
    return $sha->hexdigest();
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
    return $self->_hash_files($pkg, @files);
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
    die 'SOURCE_ROOT_INVALID: missing path' unless defined($root) && -d $root;
    # Validate language, mode, and every declared pattern before enumeration.
    $self->select_source_paths($pkg, []);
    my @candidates;
    my %physical;
    File::Find::find(
        {
            wanted => sub {
                return if $File::Find::name eq $root;
                my $relative = File::Spec->abs2rel($File::Find::name, $root);
                $relative =~ s{\\}{/}g;
                if (-l $File::Find::name) {
                    push @candidates, {path => $relative, kind => 'symlink'};
                    $File::Find::prune = 1 if -d $File::Find::name;
                    return;
                }
                if (-d $File::Find::name) {
                    $File::Find::prune = 1
                        if $self->{registry}->generated_component(File::Basename::basename($File::Find::name));
                    return;
                }
                return unless -f $File::Find::name;
                push @candidates, {path => $relative, kind => 'file'};
                $physical{$relative} = $File::Find::name;
                die "SOURCE_HASH_LIMIT_EXCEEDED: candidates\n" if @candidates > 100_000;
            },
            no_chdir => 1,
        },
        $root,
    );
    my @selected = $self->select_source_paths($pkg, \@candidates);
    return map { $physical{$_} } @selected;
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
    my ($self, $pkg, @files) = @_;
    my $root = $pkg->{path};
    my ($language, $canonical_root) = $self->_identity($pkg);
    if (!defined($canonical_root) || $canonical_root eq '') {
        my @name = split m{/}, $pkg->{name} // '';
        die "SOURCE_PACKAGE_ROOT_INVALID: cannot derive root\n" unless @name >= 2;
        $canonical_root = @name == 3 && $name[1] eq 'programs'
            ? join('/', 'code', 'programs', $name[0], $name[2])
            : join('/', 'code', 'packages', $language, @name[1 .. $#name]);
    }
    my $sha = Digest::SHA->new(256);
    my $total = 0;
    for my $file (@files) {
        my $rel = File::Spec->abs2rel($file, $root);
        $rel =~ s{\\}{/}g;
        my $path_bytes = encode('UTF-8', "$canonical_root/$rel");
        open my $fh, '<:raw', $file or die "SOURCE_READ_FAILED: $!";
        my $size = -s $fh;
        die "SOURCE_READ_FAILED: size unavailable\n" unless defined $size;
        die "SOURCE_HASH_LIMIT_EXCEEDED: file\n" if $size > 64 * 1024 * 1024;
        $total += $size;
        die "SOURCE_HASH_LIMIT_EXCEEDED: package\n" if $total > 1024 * 1024 * 1024;
        $sha->add(pack('Q>', length($path_bytes)), $path_bytes, pack('Q>', $size));
        my $read_total = 0;
        while (1) {
            my $read = read($fh, my $chunk, 8192);
            die "SOURCE_READ_FAILED: $!" unless defined $read;
            last if !$read;
            $read_total += $read;
            die "SOURCE_READ_FAILED: file changed\n" if $read_total > $size;
            $sha->add($chunk);
        }
        close $fh or die "SOURCE_READ_FAILED: $!";
        die "SOURCE_READ_FAILED: file changed\n" if $read_total != $size;
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
    my ($self, $ext, $language) = @_;
    $language //= 'perl';
    my $entry = $self->{registry}->entry($language);
    return scalar(grep { $_ eq $ext } @{ $entry->{recursive_suffixes} }) ? 1 : 0;
}

# is_special_filename -- Predicate: is $name a recognised special filename?
#
# @param $name -- Basename without path (e.g. "cpanfile").
# @return 1 or 0.
sub is_special_filename {
    my ($self, $name, $language) = @_;
    $language //= 'perl';
    my $entry = $self->{registry}->entry($language);
    my $registry = JSON::PP->new->utf8->decode($self->registry_bytes());
    return 1 if grep { $_ eq $name } @{ $registry->{universal_inputs}{build_filenames} };
    return 1 if grep { $_ eq $name } @{ $entry->{root_exact_basenames} };
    return 1 if grep { $_ eq $name } @{ $entry->{recursive_exact_basenames} };
    return 0;
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
