package CodingAdventures::BuildTool::Cache;

# Cache.pm -- JSON-Based Hash Cache for Incremental Builds
# =========================================================
#
# The cache stores SHA256 hashes of packages after a successful build.
# On the next run, we compare current hashes against cached hashes. Only
# packages whose hash has changed (or is missing from the cache) are rebuilt.
#
# Cache file format (JSON):
#
#   {
#     "perl/logic-gates": "a1b2c3d4...",
#     "python/arithmetic": "e5f6a7b8...",
#     ...
#   }
#
# The cache file lives at the root of the repository: .build-cache.json.
# This is the same location used by the Go, Python, and Ruby implementations.
#
# Why JSON::PP instead of a faster alternative?
# ---------------------------------------------
#
# JSON::PP is a pure Perl JSON encoder/decoder bundled with Perl since 5.14.
# The build cache is small (one line per package) and is read/written once
# per build. Performance is irrelevant. Using a core module keeps the
# installation footprint at zero.
#
# Perl advantages demonstrated here:
#   - JSON::PP (core) for serialisation.
#   - Hash slices for set arithmetic: @changed_packages.
#   - die/eval for error handling without CPAN dependencies.

use strict;
use warnings;
use JSON::PP ();

our $VERSION = '0.01';

# CACHE_FILENAME -- default name for the cache file.
my $CACHE_FILENAME = '.build-cache.json';

# new -- Constructor.
#
# Args:
#   path => $filepath   -- Full path to the cache file.
#                          Defaults to CACHE_FILENAME in the current directory.
sub new {
    my ($class, %args) = @_;
    return bless {
        path  => $args{path} // $CACHE_FILENAME,
        cache => {},          # loaded cache data: name => hash
    }, $class;
}

# load -- Read the cache file from disk.
#
# If the file does not exist, treats the cache as empty (no error). If the
# file is corrupt (invalid JSON), logs a warning and treats as empty.
#
# @return $self (for chaining).
sub load {
    my ($self) = @_;
    my $path = $self->{path};

    unless (-f $path) {
        $self->{cache} = {};
        return $self;
    }

    open(my $fh, '<', $path) or do {
        warn "Could not read cache file $path: $!\n";
        $self->{cache} = {};
        return $self;
    };
    local $/;
    my $json_text = <$fh>;
    close $fh;

    # eval { } catches die() from JSON::PP on invalid JSON.
    my $data = eval { JSON::PP::decode_json($json_text) };
    if ($@) {
        warn "Cache file $path is corrupt (invalid JSON): $@\nTreating as empty cache.\n";
        $self->{cache} = {};
        return $self;
    }

    # Validate: we expect a hash (object), not an array or scalar.
    unless (ref($data) eq 'HASH') {
        warn "Cache file $path has unexpected format. Treating as empty.\n";
        $self->{cache} = {};
        return $self;
    }

    $self->{cache} = $data;
    return $self;
}

# save -- Write the current hash map to the cache file.
#
# @param \%hashes -- Hash mapping package name => SHA256 hex string.
# @return $self.
sub save {
    my ($self, $hashes_ref) = @_;

    # JSON::PP->new->pretty(1)->encode produces human-readable JSON.
    # Sorted keys make git diffs readable.
    my $json = JSON::PP->new->pretty(1)->canonical(1)->encode($hashes_ref);

    open(my $fh, '>', $self->{path}) or do {
        warn "Could not write cache file $self->{path}: $!\n";
        return $self;
    };
    print $fh $json;
    close $fh;

    $self->{cache} = {%{$hashes_ref}};
    return $self;
}

# changed_packages -- Return packages whose hash has changed since last build.
#
# Compares the current hashes against the cached hashes. A package is
# "changed" if:
#   - It is new (not in the cache).
#   - Its current hash differs from the cached hash.
#
# Packages in the cache that are no longer in the current list are ignored
# (they were deleted).
#
# @param \@packages       -- arrayref of package hashrefs (from Discovery).
# @param \%current_hashes -- Hash mapping package name => current SHA256.
# @return list of package names that need rebuilding.
sub changed_packages {
    my ($self, $packages_ref, $current_hashes_ref) = @_;

    my @changed;
    for my $pkg (@{$packages_ref}) {
        my $name = $pkg->{name};
        my $current = $current_hashes_ref->{$name} // '';
        my $cached  = $self->{cache}{$name}         // '';

        push @changed, $name if $current ne $cached;
    }

    return @changed;
}

# get -- Return the cached hash for a package name.
#
# @param $name -- Package name.
# @return SHA256 hex string or undef.
sub get {
    my ($self, $name) = @_;
    return $self->{cache}{$name};
}

# all -- Return the full cache as a hashref.
sub all {
    my ($self) = @_;
    return $self->{cache};
}

1;

__END__

=head1 NAME

CodingAdventures::BuildTool::Cache - JSON-based hash cache for incremental builds

=head1 SYNOPSIS

  use CodingAdventures::BuildTool::Cache;

  my $cache = CodingAdventures::BuildTool::Cache->new(path => '.build-cache.json');
  $cache->load();

  my @to_build = $cache->changed_packages(\@packages, \%current_hashes);
  # ... build @to_build ...
  $cache->save(\%current_hashes);

=head1 DESCRIPTION

Stores SHA256 package hashes in a JSON file. On each build, compares current
hashes against cached hashes to determine which packages need rebuilding.

=cut
