use strict;
use warnings;

# ============================================================================
# CodingAdventures::UrlParser — comprehensive test suite
# ============================================================================
#
# This test file mirrors every test from the Rust url-parser crate, plus
# additional edge cases.  It exercises:
#
#   1. Basic URL parsing (scheme, host, port, path, query, fragment)
#   2. Case normalisation (scheme + host lowercased, path preserved)
#   3. Effective port (explicit vs. scheme default)
#   4. Authority string reconstruction
#   5. Invalid URL detection (missing scheme, bad scheme, bad port)
#   6. Percent-encoding and decoding (ASCII, UTF-8, round-trip, errors)
#   7. Relative resolution (same dir, parent, absolute, scheme-relative, …)
#   8. Dot-segment removal
#   9. to_url_string round-tripping
#  10. Historical URLs, IPv6, and edge cases
#
# Total: 44+ tests.
# ============================================================================

# Try Test2::V0 first (preferred); fall back to Test::More.
# The BUILD installs Test2::V0 via cpanm, but on some systems (Windows MSYS)
# only Test::More is available out of the box.
BEGIN {
    if (eval { require Test2::V0; 1 }) {
        Test2::V0->import;
    } else {
        require Test::More;
        Test::More->import;
    }
}

use CodingAdventures::UrlParser qw(
    parse resolve effective_port authority
    to_url_string percent_encode percent_decode
);

# Helper: "dies" block that works with both Test2 and Test::More.
# Returns the exception text, or undef if no exception was thrown.
sub _dies_ok (&) {
    my ($code) = @_;
    my $died;
    eval { $code->(); 1 } or do { $died = $@ };
    return $died;
}

# ─── 1. Basic parsing ──────────────────────────────────────────────────────

subtest 'parse simple HTTP URL' => sub {
    # The simplest possible HTTP URL — just scheme and host.
    # Path defaults to "/" because HTTP URLs always have a path.
    my $url = parse("http://www.example.com");
    is($url->{scheme},   'http',            'scheme is http');
    is($url->{host},     'www.example.com', 'host extracted');
    is($url->{port},     undef,             'no explicit port');
    is($url->{path},     '/',               'path defaults to /');
    is($url->{query},    undef,             'no query');
    is($url->{fragment}, undef,             'no fragment');
};

subtest 'parse HTTP with path' => sub {
    my $url = parse("http://www.example.com/docs/page.html");
    is($url->{scheme}, 'http',            'scheme');
    is($url->{host},   'www.example.com', 'host');
    is($url->{path},   '/docs/page.html', 'path extracted');
};

subtest 'parse all components' => sub {
    # A URL with every single component filled in — the "full house".
    my $url = parse("http://alice:secret\@www.example.com:8080/docs/page.html?q=hello#section2");
    is($url->{scheme},   'http',            'scheme');
    is($url->{userinfo}, 'alice:secret',    'userinfo');
    is($url->{host},     'www.example.com', 'host');
    is($url->{port},     8080,              'port');
    is($url->{path},     '/docs/page.html', 'path');
    is($url->{query},    'q=hello',         'query');
    is($url->{fragment}, 'section2',        'fragment');
};

subtest 'parse HTTPS URL' => sub {
    my $url = parse("https://secure.example.com/login");
    is($url->{scheme}, 'https',              'scheme is https');
    is($url->{host},   'secure.example.com', 'host');
    is(effective_port($url), 443,            'effective port is 443');
};

subtest 'parse FTP URL' => sub {
    my $url = parse("ftp://files.example.com/pub/readme.txt");
    is($url->{scheme}, 'ftp', 'scheme is ftp');
    is(effective_port($url), 21, 'effective port is 21');
};

subtest 'parse mailto URL' => sub {
    # mailto: is a "scheme:path" URL — no authority (no "://").
    # The entire "alice@example.com" is the path, not userinfo+host.
    my $url = parse("mailto:alice\@example.com");
    is($url->{scheme}, 'mailto',              'scheme is mailto');
    is($url->{host},   undef,                 'no host for mailto');
    is($url->{path},   'alice@example.com',   'path is the email address');
};

# ─── 2. Case normalisation ─────────────────────────────────────────────────

subtest 'scheme and host are lowercased, path case preserved' => sub {
    # RFC says scheme and host are case-insensitive, but path is case-sensitive.
    my $url = parse("HTTP://WWW.EXAMPLE.COM/PATH");
    is($url->{scheme}, 'http',            'scheme lowercased');
    is($url->{host},   'www.example.com', 'host lowercased');
    is($url->{path},   '/PATH',           'path case preserved');
};

# ─── 3. Effective port ─────────────────────────────────────────────────────

subtest 'effective port — HTTP default' => sub {
    my $url = parse("http://example.com");
    is($url->{port},          undef, 'no explicit port');
    is(effective_port($url),  80,    'default port is 80');
};

subtest 'effective port — explicit overrides default' => sub {
    my $url = parse("http://example.com:9090");
    is($url->{port},         9090, 'explicit port 9090');
    is(effective_port($url), 9090, 'effective port uses explicit');
};

# ─── 4. Authority ──────────────────────────────────────────────────────────

subtest 'authority with all parts' => sub {
    my $url = parse("http://user:pass\@host.com:8080/path");
    is(authority($url), 'user:pass@host.com:8080', 'full authority');
};

subtest 'authority host only' => sub {
    my $url = parse("http://host.com/path");
    is(authority($url), 'host.com', 'host-only authority');
};

# ─── 5. Invalid URLs ───────────────────────────────────────────────────────

subtest 'missing scheme' => sub {
    # "www.example.com" has no scheme — should die.
    like(
        _dies_ok { parse("www.example.com") },
        qr/missing_scheme/,
        'dies on missing scheme'
    );
};

subtest 'invalid scheme — starts with digit' => sub {
    like(
        _dies_ok { parse("1http://x.com") },
        qr/invalid_scheme/,
        'dies on digit-prefixed scheme'
    );
};

subtest 'invalid port — too large' => sub {
    # Port 99999 exceeds the u16 range (0-65535).
    like(
        _dies_ok { parse("http://host:99999") },
        qr/invalid_port/,
        'dies on port > 65535'
    );
};

# ─── 6. Percent-encoding ───────────────────────────────────────────────────

subtest 'encode space' => sub {
    is(percent_encode("hello world"), 'hello%20world', 'space → %20');
};

subtest 'encode preserves unreserved characters' => sub {
    # Unreserved chars should pass through untouched: A-Z a-z 0-9 - _ . ~
    is(percent_encode("abc-def_ghi.jkl~mno"), 'abc-def_ghi.jkl~mno',
       'unreserved chars preserved');
};

subtest 'encode preserves slashes' => sub {
    # Slashes are included in our "unreserved" set for path safety.
    is(percent_encode("/path/to/file"), '/path/to/file', 'slashes preserved');
};

subtest 'decode space' => sub {
    is(percent_decode("hello%20world"), 'hello world', '%20 → space');
};

subtest 'decode UTF-8' => sub {
    # 日 = U+65E5 = E6 97 A5 in UTF-8
    is(percent_decode("%E6%97%A5"), "\x{65E5}", 'decodes UTF-8 kanji');
};

subtest 'encode/decode round-trip' => sub {
    # Encoding then decoding should give back the original string.
    my $original = "hello world/\x{65E5}\x{672C}\x{8A9E}";  # "hello world/日本語"
    my $encoded  = percent_encode($original);
    my $decoded  = percent_decode($encoded);
    is($decoded, $original, 'round-trip preserves original');
};

subtest 'decode malformed — truncated' => sub {
    # "%2" is only one hex digit — needs two.
    like(
        _dies_ok { percent_decode("%2") },
        qr/invalid_percent_encoding/,
        'dies on truncated percent sequence'
    );
};

subtest 'decode malformed — bad hex' => sub {
    like(
        _dies_ok { percent_decode("%GG") },
        qr/invalid_percent_encoding/,
        'dies on non-hex digits'
    );
};

# ─── 7. Relative resolution ────────────────────────────────────────────────

subtest 'resolve same directory' => sub {
    my $base     = parse("http://host/a/b/c.html");
    my $resolved = resolve($base, "d.html");
    is($resolved->{scheme}, 'http',        'inherits scheme');
    is($resolved->{host},   'host',        'inherits host');
    is($resolved->{path},   '/a/b/d.html', 'replaces filename in same dir');
};

subtest 'resolve parent directory (..)' => sub {
    my $base     = parse("http://host/a/b/c.html");
    my $resolved = resolve($base, "../d.html");
    is($resolved->{path}, '/a/d.html', 'goes up one level');
};

subtest 'resolve grandparent directory (../../)' => sub {
    my $base     = parse("http://host/a/b/c.html");
    my $resolved = resolve($base, "../../d.html");
    is($resolved->{path}, '/d.html', 'goes up two levels');
};

subtest 'resolve absolute path' => sub {
    my $base     = parse("http://host/a/b/c.html");
    my $resolved = resolve($base, "/x/y.html");
    is($resolved->{path}, '/x/y.html', 'absolute path replaces entirely');
    is($resolved->{host}, 'host',      'inherits host');
};

subtest 'resolve scheme-relative (//other.com/path)' => sub {
    my $base     = parse("http://host/a/b");
    my $resolved = resolve($base, "//other.com/path");
    is($resolved->{scheme}, 'http',      'inherits scheme');
    is($resolved->{host},   'other.com', 'uses new host');
    is($resolved->{path},   '/path',     'uses new path');
};

subtest 'resolve already absolute URL' => sub {
    my $base     = parse("http://host/a/b");
    my $resolved = resolve($base, "https://other.com/x");
    is($resolved->{scheme}, 'https',     'uses new scheme');
    is($resolved->{host},   'other.com', 'uses new host');
    is($resolved->{path},   '/x',        'uses new path');
};

subtest 'resolve dot segment (./)' => sub {
    my $base     = parse("http://host/a/b/c");
    my $resolved = resolve($base, "./d");
    is($resolved->{path}, '/a/b/d', 'dot-slash stays in same directory');
};

subtest 'resolve empty string — returns base without fragment' => sub {
    my $base     = parse("http://host/a/b?q=1#frag");
    my $resolved = resolve($base, "");
    is($resolved->{path},     '/a/b', 'path preserved');
    is($resolved->{query},    'q=1',  'query preserved');
    is($resolved->{fragment}, undef,  'fragment stripped');
};

subtest 'resolve fragment only (#sec)' => sub {
    my $base     = parse("http://host/a/b");
    my $resolved = resolve($base, "#sec");
    is($resolved->{path},     '/a/b', 'path preserved');
    is($resolved->{fragment}, 'sec',  'fragment set');
};

subtest 'resolve with query' => sub {
    my $base     = parse("http://host/a/b");
    my $resolved = resolve($base, "c?key=val");
    is($resolved->{path},  '/a/c',    'merged path');
    is($resolved->{query}, 'key=val', 'query extracted');
};

# ─── 8. Dot-segment removal ────────────────────────────────────────────────
#
# These tests exercise the internal _remove_dot_segments function
# indirectly through resolve().

subtest 'resolve removes single dot' => sub {
    my $base     = parse("http://host/");
    my $resolved = resolve($base, "/a/./b");
    is($resolved->{path}, '/a/b', 'single dot removed');
};

subtest 'resolve removes double dot' => sub {
    my $base     = parse("http://host/");
    my $resolved = resolve($base, "/a/b/../c");
    is($resolved->{path}, '/a/c', 'double dot goes up');
};

subtest 'resolve removes multiple double dots' => sub {
    my $base     = parse("http://host/");
    my $resolved = resolve($base, "/a/b/../../c");
    is($resolved->{path}, '/c', 'two double-dots go up two levels');
};

subtest 'double dot above root' => sub {
    # Can't go above root — extra ".." segments are discarded.
    my $base     = parse("http://host/");
    my $resolved = resolve($base, "/a/../../../c");
    is($resolved->{path}, '/c', 'cannot go above root');
};

# ─── 9. to_url_string / round-tripping ─────────────────────────────────────

subtest 'round-trip full URL' => sub {
    my $input = "http://user:pass\@host.com:8080/path?q=1#frag";
    my $url   = parse($input);
    is(to_url_string($url), $input, 'full URL round-trips');
};

subtest 'round-trip simple URL' => sub {
    my $input = "http://example.com/path";
    my $url   = parse($input);
    is(to_url_string($url), $input, 'simple URL round-trips');
};

# ─── 10. Historical / real-world URLs ──────────────────────────────────────

subtest 'parse CERN original URL' => sub {
    # Tim Berners-Lee's original web page — the first URL ever.
    my $url = parse("http://info.cern.ch/hypertext/WWW/TheProject.html");
    is($url->{scheme}, 'http',          'scheme');
    is($url->{host},   'info.cern.ch',  'host');
    is($url->{path},   '/hypertext/WWW/TheProject.html', 'path');
    is(effective_port($url), 80,        'port defaults to 80');
};

subtest 'parse NCSA Mosaic URL' => sub {
    my $url = parse("http://www.ncsa.uiuc.edu/SDG/Software/Mosaic/");
    is($url->{host}, 'www.ncsa.uiuc.edu',     'host');
    is($url->{path}, '/SDG/Software/Mosaic/',  'trailing slash preserved');
};

# ─── 11. IPv6 ──────────────────────────────────────────────────────────────

subtest 'parse IPv6 localhost' => sub {
    # IPv6 addresses are enclosed in brackets per RFC 2732.
    my $url = parse("http://[::1]:8080/path");
    is($url->{host}, '[::1]', 'IPv6 host preserved with brackets');
    is($url->{port}, 8080,    'port after bracket');
    is($url->{path}, '/path', 'path');
};

# ─── 12. Edge cases ────────────────────────────────────────────────────────

subtest 'parse trailing slash' => sub {
    my $url = parse("http://host/");
    is($url->{path}, '/', 'trailing slash is the path');
};

subtest 'parse query without path' => sub {
    # "http://host?q=1" — the query attaches to the implicit "/" path.
    my $url = parse("http://host?q=1");
    is($url->{host},  'host', 'host extracted before query');
    is($url->{path},  '/',    'path defaults to /');
    is($url->{query}, 'q=1',  'query extracted');
};

subtest 'parse fragment without path' => sub {
    my $url = parse("http://host#frag");
    is($url->{host},     'host',  'host extracted before fragment');
    is($url->{path},     '/',     'path defaults to /');
    is($url->{fragment}, 'frag',  'fragment extracted');
};

subtest 'percent_encode uppercase hex' => sub {
    # Verify that encoded sequences use uppercase hex (e.g., %2F not %2f).
    my $encoded = percent_encode("@");
    is($encoded, '%40', 'at-sign encoded with uppercase hex');
};

subtest 'unknown scheme has no default port' => sub {
    my $url = parse("custom://host/path");
    is($url->{port},          undef, 'no explicit port');
    is(effective_port($url),  undef, 'no default port for custom scheme');
};

done_testing;
