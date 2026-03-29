package CodingAdventures::DocumentHtmlSanitizer;

# ============================================================================
# CodingAdventures::DocumentHtmlSanitizer — HTML Sanitizer to Prevent XSS
# ============================================================================
#
# This module sanitizes HTML strings to prevent Cross-Site Scripting (XSS)
# attacks. It removes or neutralizes dangerous HTML constructs while
# preserving safe, innocuous markup.
#
# === WHAT IS XSS? ===
#
# Cross-Site Scripting (XSS) is an attack where malicious JavaScript is
# injected into a web page and executed in the victim's browser. For example:
#
#   <script>document.cookie</script>     — steals cookies
#   <img onerror="alert(1)" src="x">    — event handler attack
#   <a href="javascript:alert(1)">link   — javascript: URI
#
# === APPROACH: PATTERN-BASED SANITIZATION ===
#
# This sanitizer uses Perl regular expressions rather than a DOM parser.
# The spec (TE02) mandates this approach for portability across languages
# that lack a shared DOM API (Go, Rust, Elixir, Lua, etc.).
#
# Pattern-based sanitization has limitations with severely malformed HTML,
# but handles the common XSS vectors defined in the spec's test categories.
#
# === ALGORITHM ===
#
# The sanitizer performs these passes in order:
#
#   1. Drop comments        — strip <!-- … -->
#   2. Drop elements        — strip <tagname>…</tagname> (content included)
#   3. Sanitize attributes  — per open-tag pass:
#        a. Strip on* event handler attributes
#        b. Strip explicit drop_attributes entries
#        c. Sanitize href/src URL attributes
#        d. Strip dangerous style attributes (expression(), url(non-http))
#
# === SECURITY NOTE ON ELEMENT DROPPING ===
#
# Element dropping removes the open tag, ALL inner content, AND the close tag.
# We do NOT just strip the tags and keep text content. The content of
# <script>alert(1)</script> could still be rendered as text by some browsers.
#
# === POLICY PRESETS ===
#
#   HTML_STRICT      — for untrusted HTML from external sources
#   HTML_RELAXED     — for authenticated users / internal tools
#   HTML_PASSTHROUGH — no sanitization (for trusted content or testing)
#
# === USAGE ===
#
#   use CodingAdventures::DocumentHtmlSanitizer;
#
#   my $clean = CodingAdventures::DocumentHtmlSanitizer::sanitize(
#       '<p>Hello <script>alert(1)</script></p>',
#       CodingAdventures::DocumentHtmlSanitizer::HTML_STRICT
#   );
#   # → "<p>Hello </p>"
#
#   # Escape HTML entities
#   my $escaped = CodingAdventures::DocumentHtmlSanitizer::escape_html('<b>bold</b>');
#   # → "&lt;b&gt;bold&lt;/b&gt;"
#
# ============================================================================

use strict;
use warnings;
use Exporter 'import';

our $VERSION = '0.01';

our @EXPORT_OK = qw(
    sanitize
    escape_html
    strip_tags
    allowed_tags
    allowed_attributes
    HTML_STRICT
    HTML_RELAXED
    HTML_PASSTHROUGH
);

# ============================================================================
# POLICY CONSTANTS
# ============================================================================
#
# A policy is a hashref controlling what gets stripped.
#
# Fields:
#   drop_elements        { string... } — element names to remove entirely
#   drop_attributes      { string... } | false — attribute names to strip
#   allowed_url_schemes  { string... } | false — URL schemes for href/src
#   drop_comments        bool — strip HTML comments
#   sanitize_style_attributes bool — strip dangerous CSS

# HTML_STRICT — for untrusted HTML from external sources.
#
# Drops all scripting, frames, form controls, and metadata.
# Strips event handlers (on*), srcdoc, formaction.
# Allows only http, https, mailto URLs.
# Strips HTML comments.
# Strips CSS expressions.
use constant HTML_STRICT => {
    drop_elements => [qw(
        script style iframe object embed applet
        form input button select textarea
        noscript meta link base
    )],
    drop_attributes     => [],   # on* stripped by default sanitize logic
    allowed_url_schemes => [qw(http https mailto)],
    drop_comments       => 1,
    sanitize_style_attributes => 1,
};

# HTML_RELAXED — for authenticated users / internal tools.
#
# Drops only scripting and frame elements. Allows comments.
# Still strips CSS expressions.
use constant HTML_RELAXED => {
    drop_elements => [qw(script iframe object embed applet)],
    drop_attributes     => [],
    allowed_url_schemes => [qw(http https mailto ftp)],
    drop_comments       => 0,
    sanitize_style_attributes => 1,
};

# HTML_PASSTHROUGH — no sanitization.
#
# Nothing is stripped. Useful for trusted content or testing.
# Uses undef/false for drop_attributes and allowed_url_schemes to signal
# "skip all attribute sanitization".
use constant HTML_PASSTHROUGH => {
    drop_elements       => [],
    drop_attributes     => undef,   # undef = skip all attribute sanitization
    allowed_url_schemes => undef,   # undef = allow any scheme
    drop_comments       => 0,
    sanitize_style_attributes => 0,
};

# ============================================================================
# ALLOWED TAGS AND ATTRIBUTES
# ============================================================================

# allowed_tags() — return the list of tags considered safe in strict mode.
#
# These are standard HTML elements that are harmless when their event
# handlers and dangerous attributes are stripped.
sub allowed_tags {
    return [qw(
        a b i em strong u s del ins
        p div span br hr
        h1 h2 h3 h4 h5 h6
        ul ol li dl dt dd
        blockquote pre code
        table thead tbody tr th td
        img figure figcaption
        header footer main nav section article aside
    )];
}

# allowed_attributes() — return the list of attributes considered safe.
#
# This is a general allowlist. Additional context-specific validation
# (like URL scheme checking on href/src) is performed separately.
sub allowed_attributes {
    return [qw(
        href src alt title id class style
        width height align valign
        colspan rowspan
        type start reversed
        lang dir
    )];
}

# ============================================================================
# HTML ESCAPING
# ============================================================================

# escape_html(text) — escape HTML special characters.
#
# Converts &, <, >, ", and ' to their named HTML entity equivalents.
# This is the fundamental defense against XSS when rendering user content
# as text (not HTML).
#
# Truth table:
#   & → &amp;
#   < → &lt;
#   > → &gt;
#   " → &quot;
#   ' → &#39;
#
# @param text  string — raw text that might contain HTML special chars
# @return string — escaped text safe to embed in HTML
sub escape_html {
    my ($text) = @_;
    return '' unless defined $text;
    $text =~ s/&/&amp;/g;
    $text =~ s/</&lt;/g;
    $text =~ s/>/&gt;/g;
    $text =~ s/"/&quot;/g;
    $text =~ s/'/&#39;/g;
    return $text;
}

# ============================================================================
# STRIP TAGS
# ============================================================================

# strip_tags(html) — remove ALL HTML tags from a string.
#
# This is a simple but effective way to extract plain text from HTML.
# Use when you want no markup at all (e.g., for plain-text email).
#
# Note: This does NOT decode HTML entities — use a separate decoder
# for that (e.g., s/&amp;/&/g).
#
# @param html  string — HTML string
# @return string — plain text with all tags removed
sub strip_tags {
    my ($html) = @_;
    return '' unless defined $html;
    # Remove all tags (including attributes, spanning multiple lines)
    $html =~ s/<[^>]*>//gs;
    return $html;
}

# ============================================================================
# CORE SANITIZATION ENGINE
# ============================================================================

# --- Step 1: Comment removal -------------------------------------------------

# _remove_comments(html) — strip all HTML comments from the string.
#
# Matches <!-- … --> pairs, handling multiple comments correctly.
# We use an iterative approach because Perl's greedy .* would match
# from the first <!-- to the LAST --> in the string.
#
# @param html  string — HTML input
# @return string — HTML with comments removed
sub _remove_comments {
    my ($html) = @_;
    my @parts;
    my $pos = 0;
    while (1) {
        # Find the start of a comment
        my $s = index($html, '<!--', $pos);
        if ( $s == -1 ) {
            push @parts, substr($html, $pos);
            last;
        }
        # Keep content before the comment
        push @parts, substr($html, $pos, $s - $pos);
        # Find the end of the comment
        my $e = index($html, '-->', $s + 4);
        if ( $e == -1 ) {
            # Unclosed comment — drop everything from here to end
            last;
        }
        # Skip past the closing -->
        $pos = $e + 3;
    }
    return join('', @parts);
}

# --- Step 2: Element dropping ------------------------------------------------

# _drop_element(html, tagname) — remove all occurrences of a specific element.
#
# Handles three cases:
#   1. <tagname …>…</tagname>  — removes open tag + content + close tag
#   2. <tagname …/>            — removes self-closing tag
#   3. <tagname …>             — removes open tag if no close tag found
#
# Case-insensitive matching via lc().
#
# @param html     string — HTML input
# @param tagname  string — lowercase tag name to drop
# @return string — HTML with all instances removed
sub _drop_element {
    my ($html, $tagname) = @_;
    my @parts;
    my $pos   = 0;
    my $lower = lc($html);

    my $open_pat  = '<' . quotemeta($tagname) . '(?=[\s/>])';
    my $close_pat = '</' . quotemeta($tagname) . '\s*>';

    while (1) {
        # Find the next open tag (case-insensitive via $lower)
        if ( $lower !~ /\G.*?($open_pat)/gsc ) {
            push @parts, substr($html, $pos);
            last;
        }
        my $s = pos($lower) - length($1);
        pos($lower) = $s;  # reset to start of match

        # Use index-based search for reliability
        my $found = -1;
        my $search_pos = $pos;
        while ( $search_pos <= length($html) ) {
            my $try = index($lower, '<' . $tagname, $search_pos);
            last if $try == -1;
            # Check that the character after tagname is space, /, or >
            my $next_char = substr($html, $try + length($tagname) + 1, 1);
            if ( $next_char =~ /[\s\/>]/ || $try + length($tagname) + 1 >= length($html) ) {
                $found = $try;
                last;
            }
            $search_pos = $try + 1;
        }

        if ( $found == -1 ) {
            push @parts, substr($html, $pos);
            last;
        }

        # Keep content before the open tag
        push @parts, substr($html, $pos, $found - $pos);

        # Find the end of the open tag (the closing >)
        my $open_end = index($html, '>', $found);
        if ( $open_end == -1 ) {
            # Malformed: no > found, stop
            last;
        }

        # Check if self-closing
        my $self_close = substr($html, $open_end - 1, 1) eq '/';

        if ( $self_close ) {
            # Self-closing: just skip the tag
            $pos = $open_end + 1;
        } else {
            # Look for the matching close tag
            my $lc_close = '</' . $tagname;
            my $cs = index($lower, $lc_close, $open_end + 1);
            if ( $cs != -1 ) {
                my $ce = index($html, '>', $cs);
                $pos = $ce != -1 ? $ce + 1 : $cs + length($lc_close);
            } else {
                # No close tag: skip just the open tag
                $pos = $open_end + 1;
            }
        }

        # Reset $lower position tracking
        $lower = lc($html);  # rebuild (we're done with pos())
    }

    return join('', @parts);
}

# _drop_elements(html, policy) — drop all elements listed in the policy.
sub _drop_elements {
    my ($html, $policy) = @_;
    my $elements = $policy->{drop_elements} || [];
    for my $tagname ( @$elements ) {
        $html = _drop_element($html, lc($tagname));
    }
    return $html;
}

# --- Step 3: Attribute sanitization ------------------------------------------

# _remove_attr(attrs, attr_name) — remove a named attribute from a tag's attrs.
#
# Handles all attribute forms:
#   name="value"   — double-quoted
#   name='value'   — single-quoted
#   name=value     — unquoted
#   name           — boolean (no value)
#
# @param attrs     string — attribute string from a tag
# @param attr_name string — attribute name to remove (lowercase)
# @return string — attrs with the attribute removed
sub _remove_attr {
    my ($attrs, $attr_name) = @_;
    my $n = quotemeta($attr_name);

    # Double-quoted: name="value"
    $attrs =~ s/\s*$n\s*=\s*"[^"]*"//gi;
    # Single-quoted: name='value'
    $attrs =~ s/\s*$n\s*=\s*'[^']*'//gi;
    # Unquoted: name=value
    $attrs =~ s/\s*$n\s*=\s*[^\s>]*//gi;
    # Boolean: name (followed by space, /, or >)
    $attrs =~ s/\s+$n(?=[\s\/>])//gi;
    # Boolean at start of attrs string
    $attrs =~ s/^$n(?=[\s\/>])//gi;

    return $attrs;
}

# _remove_event_handlers(attrs) — strip all on* event handler attributes.
#
# Event handlers start with "on" followed by one or more letters:
# onclick, onload, onerror, onmouseover, etc.
# These are always dangerous — they execute JavaScript in the browser.
#
# @param attrs  string — attribute portion of a tag
# @return string — attrs with all on* attributes removed
sub _remove_event_handlers {
    my ($attrs) = @_;

    # Double-quoted: onclick="..."
    $attrs =~ s/\s+on[a-z]+\s*=\s*"[^"]*"//gi;
    # Single-quoted: onclick='...'
    $attrs =~ s/\s+on[a-z]+\s*=\s*'[^']*'//gi;
    # Unquoted: onclick=handler
    $attrs =~ s/\s+on[a-z]+\s*=\s*[^\s>]*//gi;
    # Boolean form (rare)
    $attrs =~ s/\s+on[a-z]+(?=[\s>])//gi;
    # At start of attrs string (no leading whitespace)
    $attrs =~ s/^on[a-z]+\s*=\s*"[^"]*"//gi;
    $attrs =~ s/^on[a-z]+\s*=\s*'[^']*'//gi;
    $attrs =~ s/^on[a-z]+\s*=\s*[^\s>]*//gi;
    # Boolean form at start: oninput (no value, no equals sign)
    $attrs =~ s/^on[a-z]+(?=[\s\/>])//gi;

    return $attrs;
}

# _strip_control_chars(url) — remove control characters from a URL string.
#
# Browsers silently strip invisible code points. An attacker can smuggle
# a dangerous scheme past naive pattern-matching:
#
#   java\x00script:alert(1)   — null byte stripped → javascript:
#   \u200bjavascript:alert(1) — zero-width space stripped → javascript:
#
# We strip: ASCII C0 controls (0x00-0x1F), DEL (0x7F), and common
# zero-width Unicode characters.
#
# @param url  string — raw URL
# @return string — URL with invisible chars removed
sub _strip_control_chars {
    my ($url) = @_;
    # ASCII C0 controls and DEL
    $url =~ s/[\x00-\x1f\x7f]//g;
    # Zero-width Unicode characters (UTF-8 encoded)
    $url =~ s/\xe2\x80\x8b//g;    # U+200B ZERO WIDTH SPACE
    $url =~ s/\xe2\x80\x8c//g;    # U+200C ZERO WIDTH NON-JOINER
    $url =~ s/\xe2\x80\x8d//g;    # U+200D ZERO WIDTH JOINER
    $url =~ s/\xe2\x81\xa0//g;    # U+2060 WORD JOINER
    $url =~ s/\xef\xbb\xbf//g;    # U+FEFF BOM
    return $url;
}

# _extract_scheme(url) — extract the URL scheme (e.g., "https").
#
# Returns undef for relative URLs (no scheme component).
#
# Rules:
#   * Scheme is everything before the first colon.
#   * If a / or ? appears before the colon, the URL is relative.
#   * Scheme is lowercased before return.
#
# @param url  string — URL with control chars already stripped
# @return string|undef — lowercase scheme, or undef if relative
sub _extract_scheme {
    my ($url) = @_;
    my $colon_pos = index($url, ':');
    return undef if $colon_pos <= 0;

    my $slash_pos    = index($url, '/');
    my $question_pos = index($url, '?');

    return undef if $slash_pos    != -1 && $slash_pos    < $colon_pos;
    return undef if $question_pos != -1 && $question_pos < $colon_pos;

    return lc(substr($url, 0, $colon_pos));
}

# _is_scheme_allowed(url, allowed_schemes) — check if a URL has an allowed scheme.
#
# @param url              string        — attribute value (href, src, etc.)
# @param allowed_schemes  arrayref | undef
#   undef → any scheme is allowed (passthrough policy)
#   arrayref → allowlist of lowercase scheme strings
# @return bool — 1 if safe to pass through, 0 otherwise
sub _is_scheme_allowed {
    my ($url, $allowed_schemes) = @_;
    return 1 unless defined $allowed_schemes;  # passthrough

    my $stripped = _strip_control_chars($url);
    my $scheme   = _extract_scheme($stripped);

    # Relative URLs always pass (no scheme to check)
    return 1 unless defined $scheme;

    for my $allowed ( @$allowed_schemes ) {
        return 1 if $scheme eq lc($allowed);
    }
    return 0;
}

# _sanitize_url_attrs(attrs, allowed_schemes) — validate href and src values.
#
# If the scheme is not in the allowlist, replace the attribute value with "".
#
# @param attrs           string         — attribute string of a tag
# @param allowed_schemes arrayref|undef — scheme allowlist
# @return string — attrs with href/src values sanitized
sub _sanitize_url_attrs {
    my ($attrs, $allowed_schemes) = @_;
    return $attrs unless defined $allowed_schemes;

    for my $attr_name ( qw(href src) ) {
        my $n = quotemeta($attr_name);
        # Double-quoted: href="url"
        $attrs =~ s{($n\s*=\s*")([^"]*)(")}
                   { my ($pre,$url,$suf) = ($1,$2,$3);
                     _is_scheme_allowed($url,$allowed_schemes)
                         ? $pre.$url.$suf
                         : $pre.$suf
                   }gei;
        # Single-quoted: href='url'
        $attrs =~ s{($n\s*=\s*')([^']*)(')}
                   { my ($pre,$url,$suf) = ($1,$2,$3);
                     _is_scheme_allowed($url,$allowed_schemes)
                         ? $pre.$url.$suf
                         : $pre.$suf
                   }gei;
    }
    return $attrs;
}

# _sanitize_style(attrs) — strip dangerous style attributes.
#
# Strips the ENTIRE style attribute if it contains:
#   expression(      — IE CSS expression(), executes JavaScript
#   url(             with a non-http/https argument
#
# @param attrs  string — attribute portion of a tag
# @return string — attrs with dangerous style attributes removed
sub _sanitize_style {
    my ($attrs) = @_;

    # Double-quoted style="value"
    $attrs =~ s{(style\s*=\s*")([^"]*)(")}
               { my ($pre,$val,$suf) = ($1,$2,$3);
                 _style_is_dangerous($val) ? '' : $pre.$val.$suf
               }gei;

    # Single-quoted style='value'
    $attrs =~ s{(style\s*=\s*')([^']*)(')}
               { my ($pre,$val,$suf) = ($1,$2,$3);
                 _style_is_dangerous($val) ? '' : $pre.$val.$suf
               }gei;

    return $attrs;
}

# _style_is_dangerous(style_val) — check if a CSS value contains XSS vectors.
#
# Returns 1 (dangerous) if the value contains:
#   expression(  — IE executes arbitrary JS
#   url( with a non-http/https argument
sub _style_is_dangerous {
    my ($val) = @_;
    my $lower = lc($val);

    # Check for expression()
    return 1 if $lower =~ /expression\s*\(/;

    # Check for url() with non-safe content
    if ( $lower =~ /url\s*\(/ ) {
        # Extract each url(...) call and check its argument
        while ( $lower =~ /url\s*\(\s*(['"]?)([^)]*?)\1\s*\)/g ) {
            my $url = $2;
            $url =~ s/^\s+|\s+$//g;  # trim
            unless ( _is_scheme_allowed($url, ['http', 'https']) ) {
                return 1;
            }
        }
    }

    return 0;
}

# _sanitize_open_tag(tag, policy) — apply all attribute sanitization to one tag.
#
# @param tag     string — the full open tag, e.g. <a href="..." onclick="...">
# @param policy  hashref — sanitization policy
# @return string — sanitized open tag
sub _sanitize_open_tag {
    my ($tag, $policy) = @_;

    # Extract tag name and attribute string
    my ($tagname, $attrs) = $tag =~ /^<(\w+)(.*)/s;
    return $tag unless defined $tagname;

    my $drop_attrs = $policy->{drop_attributes};

    if ( defined $drop_attrs ) {
        # (a) Strip on* event handler attributes — always when policy is active
        $attrs = _remove_event_handlers($attrs);

        # (b) Strip explicit drop_attributes entries
        if ( ref($drop_attrs) eq 'ARRAY' ) {
            for my $attr ( @$drop_attrs ) {
                $attrs = _remove_attr($attrs, lc($attr));
            }
        }

        # Always strip srcdoc and formaction (spec requirement)
        $attrs = _remove_attr($attrs, 'srcdoc');
        $attrs = _remove_attr($attrs, 'formaction');
    }

    # (c) Sanitize href and src URL values
    my $allowed_schemes = $policy->{allowed_url_schemes};
    if ( defined $allowed_schemes ) {
        $attrs = _sanitize_url_attrs($attrs, $allowed_schemes);
    }

    # (d) Sanitize style attributes
    if ( $policy->{sanitize_style_attributes} ) {
        $attrs = _sanitize_style($attrs);
    }

    return '<' . $tagname . $attrs;
}

# _sanitize_attributes(html, policy) — walk every open tag and sanitize attrs.
#
# We iterate over every <…> region that looks like an open tag (starts with
# a letter, not a close tag) and call _sanitize_open_tag on it.
#
# @param html    string — HTML input (after element dropping)
# @param policy  hashref — sanitization policy
# @return string — HTML with all open tags sanitized
sub _sanitize_attributes {
    my ($html, $policy) = @_;
    # Match <tagname...attrs...> — open tags only (letter after <)
    $html =~ s{<(\w[^>]*)>}
              { my $inner = $1;
                my $rebuilt = _sanitize_open_tag('<' . $inner, $policy);
                # Ensure the result ends with >
                $rebuilt .= '>' unless $rebuilt =~ />$/;
                $rebuilt
              }ge;
    return $html;
}

# ============================================================================
# PUBLIC API
# ============================================================================

# sanitize(html, policy) — sanitize an HTML string.
#
# Performs a multi-pass string transformation:
#   1. Strip HTML comments (if policy->{drop_comments})
#   2. Drop dangerous elements and their content
#   3. Strip on* attributes, srcdoc, formaction
#   4. Sanitize href/src URL values
#   5. Strip dangerous style attributes
#
# @param html    string  — HTML input (may be undef, returns "")
# @param policy  hashref — sanitization policy (default: HTML_STRICT)
# @return string — sanitized HTML string
sub sanitize {
    my ($html, $policy) = @_;
    return '' unless defined $html && $html ne '';
    $policy //= HTML_STRICT;

    # Step 1: Remove HTML comments
    if ( $policy->{drop_comments} ) {
        $html = _remove_comments($html);
    }

    # Step 2: Drop dangerous elements (tag + content + close tag)
    $html = _drop_elements($html, $policy);

    # Steps 3-5: Sanitize attributes in remaining open tags
    $html = _sanitize_attributes($html, $policy);

    return $html;
}

1;

__END__

=head1 NAME

CodingAdventures::DocumentHtmlSanitizer - HTML sanitizer to prevent XSS attacks

=head1 SYNOPSIS

    use CodingAdventures::DocumentHtmlSanitizer;

    # Sanitize with strict policy
    my $clean = CodingAdventures::DocumentHtmlSanitizer::sanitize(
        '<p>Hello <script>alert(1)</script></p>',
        CodingAdventures::DocumentHtmlSanitizer::HTML_STRICT
    );
    # → "<p>Hello </p>"

    # Escape HTML entities
    my $escaped = CodingAdventures::DocumentHtmlSanitizer::escape_html('<b>bold</b>');
    # → "&lt;b&gt;bold&lt;/b&gt;"

    # Strip all tags
    my $text = CodingAdventures::DocumentHtmlSanitizer::strip_tags('<p>Hello</p>');
    # → "Hello"

=head1 DESCRIPTION

Pattern-based HTML sanitizer that removes dangerous HTML to prevent XSS attacks.
Implements three policy presets: HTML_STRICT, HTML_RELAXED, and HTML_PASSTHROUGH.

=head1 FUNCTIONS

=head2 sanitize($html, $policy)

Sanitize an HTML string using the given policy.

=head2 escape_html($text)

Escape HTML special characters (&, <, >, ", ').

=head2 strip_tags($html)

Remove all HTML tags from a string.

=head2 allowed_tags()

Return the list of safe HTML tags.

=head2 allowed_attributes()

Return the list of safe HTML attributes.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
