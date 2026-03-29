use strict;
use warnings;
use Test2::V0;

# Load the module
ok( eval { require CodingAdventures::DocumentHtmlSanitizer; 1 },
    'DocumentHtmlSanitizer module loads' ) or diag($@);

use CodingAdventures::DocumentHtmlSanitizer qw(
    sanitize escape_html strip_tags allowed_tags allowed_attributes
    HTML_STRICT HTML_RELAXED HTML_PASSTHROUGH
);

# ============================================================================
# escape_html tests
# ============================================================================

# Test 1: escape_html — basic entities
{
    is( escape_html('&'),  '&amp;',  'escape_html encodes &' );
    is( escape_html('<'),  '&lt;',   'escape_html encodes <' );
    is( escape_html('>'),  '&gt;',   'escape_html encodes >' );
    is( escape_html('"'),  '&quot;', 'escape_html encodes "' );
    is( escape_html("'"),  '&#39;',  "escape_html encodes '" );
}

# Test 2: escape_html — mixed content
{
    my $input  = '<b>Hello & "World"</b>';
    my $result = escape_html($input);
    ok( $result =~ /&lt;b&gt;/, 'escape_html escapes < and >' );
    ok( $result =~ /&amp;/,     'escape_html escapes & in mixed content' );
    ok( $result =~ /&quot;/,    'escape_html escapes double quotes' );
}

# Test 3: escape_html — undef returns empty string
{
    is( escape_html(undef), '', 'escape_html(undef) returns empty string' );
}

# Test 4: escape_html — plain text unchanged
{
    is( escape_html('hello world'), 'hello world', 'escape_html leaves plain text alone' );
}

# ============================================================================
# strip_tags tests
# ============================================================================

# Test 5: strip_tags — removes tags
{
    is( strip_tags('<p>Hello</p>'),     'Hello',  'strip_tags removes paragraph tags' );
    is( strip_tags('<b>bold</b>'),      'bold',   'strip_tags removes bold tags' );
    is( strip_tags('<br/>'),            '',       'strip_tags removes self-closing tags' );
}

# Test 6: strip_tags — preserves text content
{
    my $result = strip_tags('<p>Hello <b>world</b>!</p>');
    is( $result, 'Hello world!', 'strip_tags preserves text content' );
}

# Test 7: strip_tags — undef/empty
{
    is( strip_tags(undef), '', 'strip_tags(undef) returns empty string' );
    is( strip_tags(''),    '', 'strip_tags("") returns empty string' );
}

# ============================================================================
# sanitize — script tag removal
# ============================================================================

# Test 8: sanitize removes <script> tags and their content
{
    my $html   = '<p>Hello <script>alert(1)</script> world</p>';
    my $clean  = sanitize($html, HTML_STRICT);
    ok( $clean !~ /script/i,   'sanitize removes script tag' );
    ok( $clean !~ /alert/,     'sanitize removes script content' );
    ok( $clean =~ /Hello/,     'sanitize preserves surrounding content' );
}

# Test 9: sanitize removes <iframe> tags
{
    my $html  = '<p><iframe src="evil.com"></iframe></p>';
    my $clean = sanitize($html, HTML_STRICT);
    ok( $clean !~ /iframe/i, 'sanitize removes iframe tag' );
}

# Test 10: sanitize removes <style> tags
{
    my $html  = '<p>text</p><style>body{color:red}</style>';
    my $clean = sanitize($html, HTML_STRICT);
    ok( $clean !~ /<style/i, 'sanitize removes style element' );
}

# ============================================================================
# sanitize — event handler removal
# ============================================================================

# Test 11: sanitize strips onclick
{
    my $html  = '<div onclick="evil()">click me</div>';
    my $clean = sanitize($html, HTML_STRICT);
    ok( $clean !~ /onclick/i, 'sanitize strips onclick attribute' );
    ok( $clean =~ /click me/, 'sanitize preserves text content' );
}

# Test 12: sanitize strips onerror
{
    my $html  = '<img src="x" onerror="alert(1)">';
    my $clean = sanitize($html, HTML_STRICT);
    ok( $clean !~ /onerror/i, 'sanitize strips onerror attribute' );
}

# Test 13: sanitize strips onload
{
    my $html  = '<body onload="stealCookies()">content</body>';
    my $clean = sanitize($html, HTML_STRICT);
    ok( $clean !~ /onload/i, 'sanitize strips onload attribute' );
}

# ============================================================================
# sanitize — javascript: URI removal
# ============================================================================

# Test 14: sanitize strips javascript: href
{
    my $html  = '<a href="javascript:alert(1)">click</a>';
    my $clean = sanitize($html, HTML_STRICT);
    ok( $clean !~ /javascript:/i, 'sanitize removes javascript: URI from href' );
    ok( $clean =~ /click/,        'sanitize preserves link text' );
}

# Test 15: javascript: in src attribute
{
    my $html  = '<img src="javascript:alert(1)">';
    my $clean = sanitize($html, HTML_STRICT);
    ok( $clean !~ /javascript:/i, 'sanitize removes javascript: URI from src' );
}

# ============================================================================
# sanitize — safe content preserved
# ============================================================================

# Test 16: safe HTML passes through HTML_STRICT
{
    my $html  = '<p class="greeting"><b>Hello</b> <i>world</i>!</p>';
    my $clean = sanitize($html, HTML_STRICT);
    ok( $clean =~ /<p/, 'sanitize preserves <p> tag' );
    ok( $clean =~ /<b>/, 'sanitize preserves <b> tag' );
    ok( $clean =~ /<i>/, 'sanitize preserves <i> tag' );
    ok( $clean =~ /Hello/, 'sanitize preserves text' );
}

# Test 17: safe https href passes through
{
    my $html  = '<a href="https://example.com">link</a>';
    my $clean = sanitize($html, HTML_STRICT);
    ok( $clean =~ /https:\/\/example\.com/, 'safe https href passes through' );
}

# Test 18: safe http href passes through
{
    my $html  = '<a href="http://example.com">link</a>';
    my $clean = sanitize($html, HTML_STRICT);
    ok( $clean =~ /href="http:\/\/example\.com"/, 'safe http href preserved' );
}

# ============================================================================
# sanitize — comments
# ============================================================================

# Test 19: HTML_STRICT removes comments
{
    my $html  = '<!-- secret -->Hello<!-- another -->';
    my $clean = sanitize($html, HTML_STRICT);
    ok( $clean !~ /<!--/, 'HTML_STRICT removes comments' );
    ok( $clean =~ /Hello/, 'text outside comments preserved' );
}

# Test 20: HTML_RELAXED keeps comments
{
    my $html  = '<!-- keep me -->Hello';
    my $clean = sanitize($html, HTML_RELAXED);
    ok( $clean =~ /<!--/, 'HTML_RELAXED preserves comments' );
}

# ============================================================================
# sanitize — HTML_PASSTHROUGH
# ============================================================================

# Test 21: HTML_PASSTHROUGH does not strip script tags
{
    my $html  = '<script>alert(1)</script>';
    my $clean = sanitize($html, HTML_PASSTHROUGH);
    ok( $clean =~ /script/i, 'HTML_PASSTHROUGH does not strip script' );
}

# ============================================================================
# sanitize — style attribute sanitization
# ============================================================================

# Test 22: dangerous CSS expression() stripped
{
    my $html  = '<p style="color: expression(alert(1))">text</p>';
    my $clean = sanitize($html, HTML_STRICT);
    ok( $clean !~ /expression/i, 'sanitize strips CSS expression()' );
}

# ============================================================================
# allowed_tags and allowed_attributes
# ============================================================================

# Test 23: allowed_tags returns an arrayref of tag names
{
    my $tags = allowed_tags();
    ok( ref($tags) eq 'ARRAY', 'allowed_tags returns arrayref' );
    ok( scalar(@$tags) > 0,    'allowed_tags is non-empty' );
    my %tag_set = map { $_ => 1 } @$tags;
    ok( $tag_set{a},  'allowed_tags includes a' );
    ok( $tag_set{p},  'allowed_tags includes p' );
    ok( $tag_set{img}, 'allowed_tags includes img' );
}

# Test 24: allowed_attributes returns an arrayref
{
    my $attrs = allowed_attributes();
    ok( ref($attrs) eq 'ARRAY', 'allowed_attributes returns arrayref' );
    ok( scalar(@$attrs) > 0,    'allowed_attributes is non-empty' );
    my %attr_set = map { $_ => 1 } @$attrs;
    ok( $attr_set{href},  'allowed_attributes includes href' );
    ok( $attr_set{class}, 'allowed_attributes includes class' );
    ok( $attr_set{src},   'allowed_attributes includes src' );
}

# Test 25: sanitize(undef, ...) returns empty string
{
    is( sanitize(undef, HTML_STRICT), '', 'sanitize(undef) returns empty string' );
    is( sanitize('',    HTML_STRICT), '', 'sanitize("") returns empty string' );
}

# Test 26: HTML_RELAXED allows ftp URLs
{
    my $html  = '<a href="ftp://example.com/file">download</a>';
    my $clean = sanitize($html, HTML_RELAXED);
    ok( $clean =~ /ftp:\/\//, 'HTML_RELAXED allows ftp URLs' );
}

# Test 27: HTML_STRICT removes form elements
{
    my $html  = '<form action="/submit"><input type="text" name="q"></form>';
    my $clean = sanitize($html, HTML_STRICT);
    ok( $clean !~ /<form/i,  'HTML_STRICT removes form tag' );
    ok( $clean !~ /<input/i, 'HTML_STRICT removes input tag' );
}

done_testing;
