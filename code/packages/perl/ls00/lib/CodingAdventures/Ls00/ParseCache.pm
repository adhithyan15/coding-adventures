package CodingAdventures::Ls00::ParseCache;

# ============================================================================
# CodingAdventures::Ls00::ParseCache -- Avoids re-parsing unchanged documents
# ============================================================================
#
# # Why Cache Parse Results?
#
# Parsing is the most expensive operation in a language server.  For a large
# file, parsing on every keystroke would lag the editor noticeably.
#
# The LSP protocol helps by sending a version number with every change.  If
# the document hasn't changed (same URI, same version), the parse result
# from the previous keystroke is still valid.
#
# # Cache Key Design
#
# The cache key is "$uri\0$version".  Version is a monotonically increasing
# integer that the editor increments with each change.  Using version in
# the key means:
#
#   Same (uri, version) -> cache hit  -> return cached result
#   Different version   -> cache miss -> re-parse and cache new result
#
# The old entry is evicted when a new version is cached for the same URI.
# This keeps memory bounded at O(open_documents) entries.
#
# # Thread Safety
#
# Perl's single-threaded event model means no locking is needed.

use strict;
use warnings;

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# new() -> ParseCache
# ---------------------------------------------------------------------------

sub new {
    my ($class) = @_;
    return bless { cache => {} }, $class;
}

# ---------------------------------------------------------------------------
# get_or_parse($uri, $version, $source, $bridge) -> $parse_result
#
# Return the parse result for ($uri, $version).
#
# If the result is already cached, it is returned immediately without
# calling the bridge again.  Otherwise, $bridge->parse($source) is called,
# the result is stored, and any previous cache entry for this URI is evicted.
#
# The result is a hashref:
#   { ast => $ast, diagnostics => \@diags, err => $err_or_undef }
# ---------------------------------------------------------------------------

sub get_or_parse {
    my ($self, $uri, $version, $source, $bridge) = @_;

    my $key = "$uri\0$version";

    # Cache hit: the document hasn't changed since last parse.
    if (exists $self->{cache}{$key}) {
        return $self->{cache}{$key};
    }

    # Cache miss: parse and store.  Evict any stale entry for this URI first.
    $self->_evict($uri);

    my ($ast, $diags, $err) = $bridge->parse($source);
    $diags //= [];

    my $result = {
        ast         => $ast,
        diagnostics => $diags,
        err         => $err,
    };

    $self->{cache}{$key} = $result;
    return $result;
}

# ---------------------------------------------------------------------------
# evict($uri)
#
# Remove all cached entries for a given URI.
# Called when a document is closed (didClose).
# ---------------------------------------------------------------------------

sub evict {
    my ($self, $uri) = @_;
    $self->_evict($uri);
}

# ---------------------------------------------------------------------------
# _evict($uri) -- internal eviction
# ---------------------------------------------------------------------------

sub _evict {
    my ($self, $uri) = @_;
    my $prefix = "$uri\0";
    for my $key (keys %{$self->{cache}}) {
        if (index($key, $prefix) == 0) {
            delete $self->{cache}{$key};
        }
    }
}

1;

__END__

=head1 NAME

CodingAdventures::Ls00::ParseCache -- Caches parse results by document version

=head1 SYNOPSIS

  my $cache = CodingAdventures::Ls00::ParseCache->new();
  my $result = $cache->get_or_parse($uri, $version, $source, $bridge);

=head1 DESCRIPTION

Stores the most recent parse result for each open document.  The cache key
is (uri, version) so that unchanged documents are never re-parsed.

=cut
