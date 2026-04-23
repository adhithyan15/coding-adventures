package CodingAdventures::UrlParser;

# ============================================================================
# CodingAdventures::UrlParser — RFC 1738 URL parser with relative resolution
#                                and percent-encoding
# ============================================================================
#
# This module is a faithful port of the Rust url-parser crate.  It parses
# URLs into their component parts, resolves relative URLs against a base,
# and handles percent-encoding/decoding.
#
# ## URL anatomy
#
#   http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2
#   └─┬─┘ └────┬─────┘└──────┬───────┘└─┬─┘└─────┬───────┘└──┬───┘└───┬───┘
#  scheme  userinfo        host       port     path         query   fragment
#
# - **scheme**: how to deliver (http, ftp, mailto)
# - **host**: which server (www.example.com)
# - **port**: which door (8080; defaults to 80 for http)
# - **path**: which resource (/docs/page.html)
# - **query**: parameters (?q=hello)
# - **fragment**: client-side anchor (#section2) — never sent to server
# - **userinfo**: credentials (rare today, common in early web)
#
# ## Parsing algorithm
#
# The URL is parsed left-to-right in a single pass, no backtracking:
#
# 1. Find `://` → extract scheme (lowercased)
# 2. Find `#` from right → extract fragment
# 3. Find `?` → extract query
# 4. Find first `/` → extract path
# 5. Find `@` → extract userinfo
# 6. Find last `:` → extract port
# 7. Remainder → host (lowercased)
#
# ## Usage
#
#   use CodingAdventures::UrlParser qw(
#       parse resolve effective_port authority
#       to_url_string percent_encode percent_decode
#   );
#
#   my $url = parse("http://www.example.com:8080/docs?q=1#s2");
#   # $url = { scheme => "http", host => "www.example.com",
#   #          port => 8080, path => "/docs", query => "q=1",
#   #          fragment => "s2", userinfo => undef, raw => "..." }
#
# ============================================================================

use strict;
use warnings;
use Exporter 'import';

our $VERSION = '0.01';

our @EXPORT_OK = qw(
    parse
    resolve
    effective_port
    authority
    to_url_string
    percent_encode
    percent_decode
);

# ============================================================================
# Default ports — well-known scheme → port mappings
# ============================================================================
#
# These are the defaults assigned when no explicit port appears in the URL.
# Just like a postal address has a default mailbox, HTTP defaults to port 80.
#
# | Scheme | Default Port |
# |--------|-------------|
# | http   | 80          |
# | https  | 443         |
# | ftp    | 21          |

my %DEFAULT_PORTS = (
    http  => 80,
    https => 443,
    ftp   => 21,
);

# ============================================================================
# parse($input) → hashref or die
# ============================================================================
#
# Parse an absolute URL string into its component parts.
#
# Returns a hashref with keys:
#   scheme, userinfo, host, port, path, query, fragment, raw
#
# Dies with a descriptive message on invalid input:
#   "missing_scheme"          — no scheme found
#   "invalid_scheme"          — scheme doesn't match [a-z][a-z0-9+.-]*
#   "invalid_port"            — port is not a valid 0-65535 number
#   "invalid_percent_encoding" — malformed %XX sequence
#
# ## Algorithm — single-pass, left-to-right:
#
# ```text
# "http://alice:secret@www.example.com:8080/docs/page.html?q=hello#sec2"
#  ^^^^                                                              ^^^^
#  Step 1: scheme = "http"                            Step 2: fragment = "sec2"
#                                                   ^^^^^^^^
#                                           Step 3: query = "q=hello"
#                                    ^^^^^^^^^^^^^^^
#                            Step 4: path = "/docs/page.html"
#        ^^^^^^^^^^^^
#    Step 5: userinfo = "alice:secret"
#                                ^^^^
#                    Step 6: port = 8080
#                       ^^^^^^^^^^^^^^^
#               Step 7: host = "www.example.com"
# ```

sub parse {
    my ($input) = @_;

    die "missing_scheme\n" unless defined $input;

    my $raw = $input;
    $input =~ s/^\s+//;
    $input =~ s/\s+$//;

    # ── Step 1: Extract scheme ──────────────────────────────────────────
    #
    # Look for "://".  If found, everything before it is the scheme.
    # If not found, try the "scheme:path" form (e.g., "mailto:alice@ex.com").
    # If neither pattern matches, we have no scheme — that's an error.

    my $scheme_sep = index($input, '://');

    if ($scheme_sep >= 0) {
        # Authority-based URL: "scheme://authority/path?query#fragment"
        my $scheme = lc(substr($input, 0, $scheme_sep));
        _validate_scheme($scheme);
        my $after_scheme = substr($input, $scheme_sep + 3);

        # ── Step 2: Extract fragment (find "#") ─────────────────────────
        my ($after_frag, $fragment) = _split_fragment($after_scheme);

        # ── Step 3: Extract query (find "?") ────────────────────────────
        my ($after_query, $query) = _split_query($after_frag);

        # ── Step 4: Split authority from path (find first "/") ──────────
        my ($authority_str, $path);
        my $slash_pos = index($after_query, '/');
        if ($slash_pos >= 0) {
            $authority_str = substr($after_query, 0, $slash_pos);
            $path          = substr($after_query, $slash_pos);
        } else {
            $authority_str = $after_query;
            $path          = '/';
        }

        # ── Step 5: Extract userinfo (find "@" in authority) ────────────
        my ($userinfo, $host_port);
        my $at_pos = rindex($authority_str, '@');
        if ($at_pos >= 0) {
            $userinfo  = substr($authority_str, 0, $at_pos);
            $host_port = substr($authority_str, $at_pos + 1);
        } else {
            $userinfo  = undef;
            $host_port = $authority_str;
        }

        # ── Step 6 & 7: Extract port and host ──────────────────────────
        #
        # IPv6 addresses are enclosed in brackets: [::1]:8080
        # For IPv6, the port delimiter is the ":" AFTER the closing "]"
        # For IPv4 / hostnames, the LAST ":" separates host from port,
        # but only if everything after it is all digits.

        my ($host, $port);

        if ($host_port =~ /^\[/) {
            # IPv6: find closing bracket
            my $bracket_pos = index($host_port, ']');
            if ($bracket_pos >= 0) {
                $host = substr($host_port, 0, $bracket_pos + 1);
                my $after_bracket = substr($host_port, $bracket_pos + 1);
                if ($after_bracket =~ /^:(.+)/) {
                    $port = _parse_port($1);
                } else {
                    $port = undef;
                }
            } else {
                # Malformed IPv6 — treat whole thing as host
                $host = $host_port;
                $port = undef;
            }
        } else {
            # IPv4 or hostname: last ":" separates host from port
            my $colon_pos = rindex($host_port, ':');
            if ($colon_pos >= 0) {
                my $maybe_port = substr($host_port, $colon_pos + 1);
                # Only treat as port if it's non-empty and all digits
                if (length($maybe_port) > 0 && $maybe_port =~ /^\d+$/) {
                    $host = substr($host_port, 0, $colon_pos);
                    $port = _parse_port($maybe_port);
                } else {
                    $host = $host_port;
                    $port = undef;
                }
            } else {
                $host = $host_port;
                $port = undef;
            }
        }

        # Empty host → undef; otherwise lowercase
        $host = (length($host) > 0) ? lc($host) : undef;

        return {
            scheme   => $scheme,
            userinfo => $userinfo,
            host     => $host,
            port     => $port,
            path     => $path,
            query    => $query,
            fragment => $fragment,
            raw      => $raw,
        };
    }

    # ── "scheme:path" form (e.g., "mailto:alice@example.com") ───────────
    #
    # No "://" found.  Check for a bare "scheme:rest" pattern.
    # The colon must exist, the part before it must not contain "/" (to
    # avoid confusing "relative/path:stuff" with a scheme), and the
    # scheme must be at least 1 character and start with a letter.

    my $colon_pos = index($input, ':');
    if ($colon_pos > 0 && index(substr($input, 0, $colon_pos), '/') < 0) {
        my $scheme = lc(substr($input, 0, $colon_pos));
        _validate_scheme($scheme);
        my $rest = substr($input, $colon_pos + 1);

        # Still split fragment and query from the path
        my ($after_frag, $fragment) = _split_fragment($rest);
        my ($path, $query)          = _split_query($after_frag);

        return {
            scheme   => $scheme,
            userinfo => undef,
            host     => undef,
            port     => undef,
            path     => $path,
            query    => $query,
            fragment => $fragment,
            raw      => $raw,
        };
    }

    die "missing_scheme\n";
}

# ============================================================================
# resolve($base_url, $relative) → hashref or die
# ============================================================================
#
# Resolve a relative URL against a base URL (hashref from parse()).
#
# Implements RFC 1808 relative resolution:
#
#   if R has scheme     → R is absolute, return as-is
#   if R starts with // → inherit scheme only
#   if R starts with /  → inherit scheme + authority, replace path
#   otherwise           → merge paths, resolve . and ..
#
# Examples:
#   my $base = parse("http://host/a/b/c.html");
#   resolve($base, "d.html")->{path}     # → "/a/b/d.html"
#   resolve($base, "../d.html")->{path}  # → "/a/d.html"
#   resolve($base, "/x/y.html")->{path}  # → "/x/y.html"

sub resolve {
    my ($base, $relative) = @_;

    $relative =~ s/^\s+//;
    $relative =~ s/\s+$//;

    # Empty relative → return base without fragment
    if ($relative eq '') {
        my $result = { %$base };
        $result->{fragment} = undef;
        $result->{raw} = to_url_string($result);
        return $result;
    }

    # Fragment-only: "#section"
    if ($relative =~ /^#(.*)/) {
        my $result = { %$base };
        $result->{fragment} = $1;
        $result->{raw} = to_url_string($result);
        return $result;
    }

    # If R has a scheme, it's already absolute
    if (index($relative, '://') >= 0
        || ($relative =~ /:/ && $relative !~ m{^/}))
    {
        # Check if the part before ":" looks like a scheme
        if ($relative =~ /^([^:]+):/) {
            my $maybe_scheme = $1;
            if (length($maybe_scheme) > 0
                && $maybe_scheme =~ /^[a-zA-Z][a-zA-Z0-9+.\-]*$/)
            {
                return parse($relative);
            }
        }
    }

    # Scheme-relative: "//host/path"
    if ($relative =~ m{^//}) {
        my $full = $base->{scheme} . ':' . $relative;
        return parse($full);
    }

    # Absolute path: "/path"
    if ($relative =~ m{^/}) {
        my ($path_part, $fragment) = _split_fragment($relative);
        my ($path, $query)         = _split_query($path_part);
        my $result = { %$base };
        $result->{path}     = _remove_dot_segments($path);
        $result->{query}    = $query;
        $result->{fragment} = $fragment;
        $result->{raw}      = to_url_string($result);
        return $result;
    }

    # Relative path: merge with base
    my ($rel_part, $fragment) = _split_fragment($relative);
    my ($rel_path, $query)    = _split_query($rel_part);

    # Merge: take base path up to last "/", append relative
    my $merged        = _merge_paths($base->{path}, $rel_path);
    my $resolved_path = _remove_dot_segments($merged);

    my $result = { %$base };
    $result->{path}     = $resolved_path;
    $result->{query}    = $query;
    $result->{fragment} = $fragment;
    $result->{raw}      = to_url_string($result);
    return $result;
}

# ============================================================================
# effective_port($url) → number or undef
# ============================================================================
#
# Returns the explicit port if set, otherwise the well-known default for the
# scheme.  Returns undef for unknown schemes with no explicit port.
#
#   effective_port(parse("http://host"))        # → 80
#   effective_port(parse("http://host:9090"))   # → 9090
#   effective_port(parse("custom://host"))      # → undef

sub effective_port {
    my ($url) = @_;
    return $url->{port} if defined $url->{port};
    return $DEFAULT_PORTS{ $url->{scheme} };
}

# ============================================================================
# authority($url) → string
# ============================================================================
#
# Build the authority component: [userinfo@]host[:port]
#
#   authority(parse("http://user:pass@host:8080/p"))  # → "user:pass@host:8080"
#   authority(parse("http://host/p"))                 # → "host"

sub authority {
    my ($url) = @_;
    my $auth = '';
    if (defined $url->{userinfo}) {
        $auth .= $url->{userinfo} . '@';
    }
    if (defined $url->{host}) {
        $auth .= $url->{host};
    }
    if (defined $url->{port}) {
        $auth .= ':' . $url->{port};
    }
    return $auth;
}

# ============================================================================
# to_url_string($url) → string
# ============================================================================
#
# Serialize a parsed URL hashref back to its string representation.
#
#   to_url_string(parse("http://host/path?q=1#f"))  # → "http://host/path?q=1#f"

sub to_url_string {
    my ($url) = @_;
    my $s = $url->{scheme};

    if (defined $url->{host}) {
        $s .= '://' . authority($url);
    } else {
        $s .= ':';
    }

    $s .= $url->{path};

    if (defined $url->{query}) {
        $s .= '?' . $url->{query};
    }
    if (defined $url->{fragment}) {
        $s .= '#' . $url->{fragment};
    }

    return $s;
}

# ============================================================================
# percent_encode($input) → string
# ============================================================================
#
# Percent-encode a string for safe inclusion in a URL.
#
# Characters that do NOT need encoding (RFC 1738 unreserved + path slash):
#   A-Z  a-z  0-9  -  _  .  ~  /
#
# Everything else becomes %XX where XX is the uppercase hex byte value.
#
# Examples:
#   percent_encode("hello world")    # → "hello%20world"
#   percent_encode("/path/to/file")  # → "/path/to/file"     (slashes preserved)
#   percent_encode("café")           # → "caf%C3%A9"         (UTF-8 bytes encoded)

sub percent_encode {
    my ($input) = @_;
    # Encode as UTF-8 bytes, then encode each non-unreserved byte
    my @bytes = unpack('C*', Encode::encode('UTF-8', $input));
    my $result = '';
    for my $byte (@bytes) {
        if (_is_unreserved($byte)) {
            $result .= chr($byte);
        } else {
            $result .= sprintf('%%%02X', $byte);
        }
    }
    return $result;
}

# ============================================================================
# percent_decode($input) → string or die
# ============================================================================
#
# Decode percent-encoded sequences back to their original characters.
# Each %XX is replaced by the byte with that hex value.  The resulting
# byte sequence is interpreted as UTF-8.
#
# Examples:
#   percent_decode("hello%20world")   # → "hello world"
#   percent_decode("%E6%97%A5")       # → "日"  (UTF-8: E6 97 A5)
#
# Dies with "invalid_percent_encoding" on malformed input ("%2", "%GG").

sub percent_decode {
    my ($input) = @_;
    my @bytes;
    my $len = length($input);
    my $i   = 0;

    while ($i < $len) {
        my $ch = substr($input, $i, 1);
        if ($ch eq '%') {
            # Need at least 2 more hex digits
            die "invalid_percent_encoding\n" if $i + 2 >= $len;
            my $hi = _hex_digit(substr($input, $i + 1, 1));
            my $lo = _hex_digit(substr($input, $i + 2, 1));
            push @bytes, ($hi << 4) | $lo;
            $i += 3;
        } else {
            push @bytes, ord($ch);
            $i += 1;
        }
    }

    my $raw_bytes = pack('C*', @bytes);
    return Encode::decode('UTF-8', $raw_bytes);
}

# ============================================================================
# Internal helpers
# ============================================================================

# We need Encode for UTF-8 handling in percent_encode / percent_decode.
use Encode ();

# _is_unreserved($byte) → bool
#
# Returns true if the given byte value represents an "unreserved" character
# that does NOT need percent-encoding.  The unreserved set is:
#   A-Z  a-z  0-9  -  _  .  ~  /
#
# This matches the Rust implementation's `is_unreserved()` function.

sub _is_unreserved {
    my ($byte) = @_;
    return 1 if $byte >= 0x41 && $byte <= 0x5A;  # A-Z
    return 1 if $byte >= 0x61 && $byte <= 0x7A;  # a-z
    return 1 if $byte >= 0x30 && $byte <= 0x39;  # 0-9
    return 1 if $byte == 0x2D;                    # -
    return 1 if $byte == 0x5F;                    # _
    return 1 if $byte == 0x2E;                    # .
    return 1 if $byte == 0x7E;                    # ~
    return 1 if $byte == 0x2F;                    # /
    return 0;
}

# _hex_digit($char) → number or die
#
# Convert a single hex character ('0'-'9', 'a'-'f', 'A'-'F') to its
# numeric value (0-15).  Dies with "invalid_percent_encoding" otherwise.

sub _hex_digit {
    my ($ch) = @_;
    my $ord = ord($ch);
    return $ord - ord('0') if $ord >= ord('0') && $ord <= ord('9');
    return $ord - ord('a') + 10 if $ord >= ord('a') && $ord <= ord('f');
    return $ord - ord('A') + 10 if $ord >= ord('A') && $ord <= ord('F');
    die "invalid_percent_encoding\n";
}

# _validate_scheme($scheme) → void or die
#
# Check that a scheme matches the regex [a-z][a-z0-9+.-]*.
# The scheme has already been lowercased by the caller.
#
# Dies with "invalid_scheme" if the check fails.

sub _validate_scheme {
    my ($scheme) = @_;
    die "invalid_scheme\n" if length($scheme) == 0;
    die "invalid_scheme\n" unless $scheme =~ /^[a-z][a-z0-9+.\-]*$/;
    return;
}

# _parse_port($string) → number or die
#
# Parse a port string to an integer in the range 0-65535.
# Dies with "invalid_port" if the value is out of range or non-numeric.

sub _parse_port {
    my ($s) = @_;
    die "invalid_port\n" unless $s =~ /^\d+$/;
    my $port = 0 + $s;
    die "invalid_port\n" if $port > 65535;
    return $port;
}

# _split_fragment($input) → ($before, $after_or_undef)
#
# Split a string at the first '#'.
# Returns (everything_before, everything_after) or (input, undef).

sub _split_fragment {
    my ($input) = @_;
    my $pos = index($input, '#');
    if ($pos >= 0) {
        return (substr($input, 0, $pos), substr($input, $pos + 1));
    }
    return ($input, undef);
}

# _split_query($input) → ($before, $after_or_undef)
#
# Split a string at the first '?'.
# Returns (everything_before, everything_after) or (input, undef).

sub _split_query {
    my ($input) = @_;
    my $pos = index($input, '?');
    if ($pos >= 0) {
        return (substr($input, 0, $pos), substr($input, $pos + 1));
    }
    return ($input, undef);
}

# _merge_paths($base_path, $relative_path) → string
#
# Merge a base path and a relative path.  Takes everything in base_path
# up to and including the last "/", then appends relative_path.
#
# Examples:
#   _merge_paths("/a/b/c", "d")  → "/a/b/d"
#   _merge_paths("/a/b/",  "d")  → "/a/b/d"
#   _merge_paths("/a",     "d")  → "/d"

sub _merge_paths {
    my ($base_path, $relative_path) = @_;
    my $pos = rindex($base_path, '/');
    if ($pos >= 0) {
        return substr($base_path, 0, $pos + 1) . $relative_path;
    }
    return '/' . $relative_path;
}

# _remove_dot_segments($path) → string
#
# Remove "." and ".." segments from a path, implementing RFC 3986 §5.2.4.
#
# "." means "current directory" — skip it.
# ".." means "parent directory" — pop the last segment.
# Can't go above root — extra ".." segments are simply discarded.
#
# Examples:
#   /a/b/../c      → /a/c
#   /a/./b         → /a/b
#   /a/b/../../c   → /c
#   /a/../../../c  → /c   (can't go above root)

sub _remove_dot_segments {
    my ($path) = @_;
    my @segments = split(m{/}, $path, -1);  # -1 preserves trailing empty strings
    my @output;

    for my $seg (@segments) {
        if ($seg eq '.') {
            # Current directory — skip
            next;
        } elsif ($seg eq '..') {
            # Parent directory — pop last segment (if any)
            pop @output;
        } else {
            push @output, $seg;
        }
    }

    my $result = join('/', @output);

    # Ensure the path starts with "/" if the input did
    if ($path =~ m{^/} && $result !~ m{^/}) {
        $result = '/' . $result;
    }

    return $result;
}

1;

__END__

=head1 NAME

CodingAdventures::UrlParser - RFC 1738 URL parser with relative resolution and percent-encoding

=head1 SYNOPSIS

    use CodingAdventures::UrlParser qw(parse resolve effective_port
                                        authority to_url_string
                                        percent_encode percent_decode);

    my $url = parse("http://www.example.com:8080/docs?q=hello#s2");
    # $url->{scheme}   → "http"
    # $url->{host}     → "www.example.com"
    # $url->{port}     → 8080
    # $url->{path}     → "/docs"
    # $url->{query}    → "q=hello"
    # $url->{fragment} → "s2"

    my $base     = parse("http://host/a/b/c.html");
    my $resolved = resolve($base, "../d.html");
    # $resolved->{path} → "/a/d.html"

    say percent_encode("hello world");     # "hello%20world"
    say percent_decode("hello%20world");   # "hello world"

=head1 DESCRIPTION

RFC 1738 URL parser with relative resolution and percent-encoding.
A single-pass, left-to-right parser that decomposes URLs into scheme,
userinfo, host, port, path, query, and fragment components.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
