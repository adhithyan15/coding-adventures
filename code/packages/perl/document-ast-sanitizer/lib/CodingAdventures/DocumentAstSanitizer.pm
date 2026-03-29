package CodingAdventures::DocumentAstSanitizer;

# ============================================================================
# CodingAdventures::DocumentAstSanitizer
# ============================================================================
#
# Sanitizes a Document AST by applying a policy that removes or neutralises
# dangerous or unwanted content. This is the "defense in depth" layer between
# a Markdown parser and an HTML renderer.
#
# === Why AST-level sanitization matters ===
#
# Sanitizing the AST (rather than the final HTML) is safer because:
#
#   1. No regex games against HTML strings — we work on typed nodes.
#   2. The sanitizer is renderer-agnostic: one policy works for HTML, PDF, etc.
#   3. We can express structural policies (e.g. "cap heading level to 3") that
#      are impossible to express as HTML-level transforms.
#
# === Design: pure and immutable ===
#
# Every function returns a brand-new data structure — input nodes are never
# modified in place. This means you can safely run the same document through
# multiple policies without fear of data corruption.
#
# === AST Node Format ===
#
# All nodes are plain Perl hashrefs. The "type" key identifies the node kind:
#
#   {type => "document",  children => [...]}
#   {type => "heading",   level => 2, children => [...]}
#   {type => "paragraph", children => [...]}
#   {type => "code_block", language => "perl", value => "say 'hi'"}
#   {type => "text",      value => "hello"}
#   {type => "link",      destination => "https://...", title => undef, children => [...]}
#   {type => "image",     destination => "...", alt => "...", title => undef}
#   ...
#
# === Policy Format ===
#
# A policy is a hashref with the following optional keys:
#
#   allowRawBlockFormats    "drop-all" | "passthrough" | [list of formats]
#   allowRawInlineFormats   same, for raw_inline nodes
#   allowedUrlSchemes       false (allow all) | [list of lowercase schemes]
#   dropLinks               boolean — promote link children to parent
#   dropImages              boolean — drop image nodes entirely
#   transformImageToText    boolean — replace image with alt text node
#   maxHeadingLevel         1-6 | "drop" — cap heading levels
#   minHeadingLevel         1-6 — floor heading levels
#   dropBlockquotes         boolean
#   dropCodeBlocks          boolean
#   transformCodeSpanToText boolean
#
# === Named Policy Presets ===
#
# Three presets are exported as package variables:
#
#   $STRICT      — for user-generated content (comments, forum posts)
#   $RELAXED     — for semi-trusted content (authenticated users)
#   $PASSTHROUGH — for fully trusted content (documentation, static sites)
#
# @module CodingAdventures::DocumentAstSanitizer

use strict;
use warnings;

our $VERSION = '0.01';

# ─── Named Policy Presets ────────────────────────────────────────────────────

# STRICT — for user-generated content (comments, forum posts, chat messages).
#
# Drops all raw HTML/format passthrough. Allows only http, https, mailto URLs.
# Images are converted to alt text. Links are kept but URL-sanitized.
# Headings are clamped to h2-h6 (h1 is reserved for page title).
our $STRICT = {
    allowRawBlockFormats    => 'drop-all',
    allowRawInlineFormats   => 'drop-all',
    allowedUrlSchemes       => [qw(http https mailto)],
    dropImages              => 0,
    transformImageToText    => 1,
    minHeadingLevel         => 2,
    maxHeadingLevel         => 6,
    dropLinks               => 0,
    dropBlockquotes         => 0,
    dropCodeBlocks          => 0,
    transformCodeSpanToText => 0,
};

# RELAXED — for semi-trusted content (authenticated users, internal wikis).
#
# Allows HTML raw blocks (but not other formats). Allows http, https, mailto,
# ftp. Images pass through unchanged. Headings unrestricted.
our $RELAXED = {
    allowRawBlockFormats    => ['html'],
    allowRawInlineFormats   => ['html'],
    allowedUrlSchemes       => [qw(http https mailto ftp)],
    dropImages              => 0,
    transformImageToText    => 0,
    minHeadingLevel         => 1,
    maxHeadingLevel         => 6,
    dropLinks               => 0,
    dropBlockquotes         => 0,
    dropCodeBlocks          => 0,
    transformCodeSpanToText => 0,
};

# PASSTHROUGH — for fully trusted content (documentation, static sites).
#
# No sanitization. Everything passes through unchanged.
our $PASSTHROUGH = {
    allowRawBlockFormats    => 'passthrough',
    allowRawInlineFormats   => 'passthrough',
    allowedUrlSchemes       => 0,  # 0 = allow any scheme
    dropImages              => 0,
    transformImageToText    => 0,
    minHeadingLevel         => 1,
    maxHeadingLevel         => 6,
    dropLinks               => 0,
    dropBlockquotes         => 0,
    dropCodeBlocks          => 0,
    transformCodeSpanToText => 0,
};

# ─── URL Utilities ────────────────────────────────────────────────────────────

# strip_control_chars($url) → string
#
# Removes C0 control characters (U+0000–U+001F), DEL (U+007F), and specific
# Unicode "invisible" code points that browsers silently ignore. This prevents
# bypass attacks like "java\x00script:alert(1)" where the null byte hides the
# "javascript:" scheme from naive checks.
sub _strip_control_chars {
    my ($url) = @_;
    $url =~ s/[\x00-\x1F\x7F]//g;      # C0 controls + DEL
    $url =~ s/\xe2\x80\x8b//g;          # U+200B ZERO WIDTH SPACE
    $url =~ s/\xe2\x80\x8c//g;          # U+200C ZERO WIDTH NON-JOINER
    $url =~ s/\xe2\x80\x8d//g;          # U+200D ZERO WIDTH JOINER
    $url =~ s/\xe2\x81\xa0//g;          # U+2060 WORD JOINER
    $url =~ s/\xef\xbb\xbf//g;          # U+FEFF BOM
    return $url;
}

# extract_scheme($url) → string or undef
#
# Extracts the URL scheme (the part before the first colon), lowercased.
# Returns undef if the URL is relative (no scheme). A colon that appears after
# a "/" or "?" is part of a path or query, not a scheme separator.
#
# Examples:
#   "https://example.com"  → "https"
#   "JAVASCRIPT:alert(1)"  → "javascript"
#   "/relative/path"       → undef
#   "?q=foo:bar"           → undef
sub _extract_scheme {
    my ($url) = @_;
    my $colon_pos = index($url, ':');
    return undef if $colon_pos < 0;   # no colon → relative URL

    # A colon at position 0 means the URL starts with ":" which is relative
    return undef if $colon_pos == 0;

    # Check for "/" or "?" before the colon — that makes the colon non-schematic
    my $slash_pos    = index($url, '/');
    my $question_pos = index($url, '?');

    return undef if $slash_pos    >= 0 && $slash_pos    < $colon_pos;
    return undef if $question_pos >= 0 && $question_pos < $colon_pos;

    return lc(substr($url, 0, $colon_pos));
}

# _is_scheme_allowed($url, $allowed_schemes) → bool
#
# Returns true if the URL's scheme is permitted by the policy.
#
#   $allowed_schemes = 0 (or falsy)  → all schemes allowed (passthrough)
#   $allowed_schemes = [list]        → allowlist of lowercase schemes
#
# Relative URLs always pass through regardless of scheme policy.
sub _is_scheme_allowed {
    my ($url, $allowed_schemes) = @_;

    # 0 (or undef/false) means "allow everything"
    return 1 if !$allowed_schemes;

    my $stripped = _strip_control_chars($url);
    my $scheme   = _extract_scheme($stripped);

    # Relative URLs always pass
    return 1 if !defined $scheme;

    # Linear scan over the allowlist
    for my $allowed (@$allowed_schemes) {
        return 1 if $scheme eq lc($allowed);
    }

    return 0;
}

# ─── Format Policy Helper ─────────────────────────────────────────────────────

# _format_allowed($node_format, $policy_field) → bool
#
# Determines whether a raw_block or raw_inline node should be kept, based on
# the policy's format allowlist.
#
#   "drop-all"    → always false
#   "passthrough" → always true
#   [list]        → true if node_format is in the list
sub _format_allowed {
    my ($node_format, $policy_field) = @_;
    return 0 if !defined $policy_field || $policy_field eq 'drop-all';
    return 1 if $policy_field eq 'passthrough';
    # It's an arrayref — check membership
    for my $allowed (@$policy_field) {
        return 1 if $node_format eq $allowed;
    }
    return 0;
}

# ─── Inline Node Sanitizer ────────────────────────────────────────────────────

# _sanitize_inline($node, $policy) → node or undef or {__promoted => [...]}
#
# Sanitizes a single inline node. Returns:
#   undef                    — drop this node
#   a node hashref           — use this replacement node
#   {__promoted => [...]}    — splice the given nodes into the parent (link children)
#
# The __promoted sentinel is used when dropLinks is true: rather than dropping
# the link entirely, we "promote" the link's text children to the parent level,
# preserving the visible content while removing the anchor.
sub _sanitize_inline {
    my ($node, $policy) = @_;
    my $t = $node->{type};

    # text — keep as-is (leaf node, no children)
    if ($t eq 'text') {
        return {type => 'text', value => $node->{value}};

    # emphasis — recurse into children; drop if empty result
    } elsif ($t eq 'emphasis') {
        my $children = _sanitize_inlines($node->{children} || [], $policy);
        return undef if @$children == 0;
        return {type => 'emphasis', children => $children};

    # strong — recurse into children; drop if empty
    } elsif ($t eq 'strong') {
        my $children = _sanitize_inlines($node->{children} || [], $policy);
        return undef if @$children == 0;
        return {type => 'strong', children => $children};

    # strikethrough — recurse into children; drop if empty
    } elsif ($t eq 'strikethrough') {
        my $children = _sanitize_inlines($node->{children} || [], $policy);
        return undef if @$children == 0;
        return {type => 'strikethrough', children => $children};

    # code_span — optionally transform to text
    } elsif ($t eq 'code_span') {
        if ($policy->{transformCodeSpanToText}) {
            return {type => 'text', value => $node->{value}};
        }
        return {type => 'code_span', value => $node->{value}};

    # link — drop (promoting children) or sanitize URL
    } elsif ($t eq 'link') {
        if ($policy->{dropLinks}) {
            # Promote children: return sentinel so caller splices them in
            my $children = _sanitize_inlines($node->{children} || [], $policy);
            return {__promoted => $children};
        }
        my $dest = $node->{destination};
        if (!_is_scheme_allowed($dest, $policy->{allowedUrlSchemes})) {
            $dest = '';
        }
        my $children = _sanitize_inlines($node->{children} || [], $policy);
        return {
            type        => 'link',
            destination => $dest,
            title       => $node->{title},
            children    => $children,
        };

    # image — drop, transform to alt text, or sanitize URL
    } elsif ($t eq 'image') {
        return undef if $policy->{dropImages};
        if ($policy->{transformImageToText}) {
            return {type => 'text', value => $node->{alt} // ''};
        }
        my $dest = $node->{destination};
        if (!_is_scheme_allowed($dest, $policy->{allowedUrlSchemes})) {
            $dest = '';
        }
        return {
            type        => 'image',
            destination => $dest,
            title       => $node->{title},
            alt         => $node->{alt} // '',
        };

    # autolink — drop if scheme not allowed; no children to promote
    } elsif ($t eq 'autolink') {
        my $dest = $node->{destination};
        if (!_is_scheme_allowed($dest, $policy->{allowedUrlSchemes})) {
            return undef;
        }
        return {type => 'autolink', destination => $dest, is_email => $node->{is_email}};

    # raw_inline — apply format policy
    } elsif ($t eq 'raw_inline') {
        my $field = $policy->{allowRawInlineFormats} // 'passthrough';
        return undef unless _format_allowed($node->{format}, $field);
        return {type => 'raw_inline', format => $node->{format}, value => $node->{value}};

    # hard_break — always keep
    } elsif ($t eq 'hard_break') {
        return {type => 'hard_break'};

    # soft_break — always keep
    } elsif ($t eq 'soft_break') {
        return {type => 'soft_break'};

    # Unknown node types are dropped — never silently pass through
    } else {
        return undef;
    }
}

# _sanitize_inlines(\@nodes, $policy) → \@nodes
#
# Sanitizes a list of inline nodes, flattening any promoted children from
# dropped links directly into the output list.
sub _sanitize_inlines {
    my ($nodes, $policy) = @_;
    my @result;
    for my $node (@$nodes) {
        my $sanitized = _sanitize_inline($node, $policy);
        next if !defined $sanitized;
        if (exists $sanitized->{__promoted}) {
            # Link promotion: splice children into the parent list
            push @result, @{$sanitized->{__promoted}};
        } else {
            push @result, $sanitized;
        }
    }
    return \@result;
}

# ─── Block Node Sanitizer ─────────────────────────────────────────────────────

# _sanitize_block($node, $policy) → node or undef
#
# Sanitizes a single block-level node. Returns undef to drop the node.
sub _sanitize_block {
    my ($node, $policy) = @_;
    my $t = $node->{type};

    # heading — clamp level or drop
    if ($t eq 'heading') {
        my $max = $policy->{maxHeadingLevel};
        my $min = $policy->{minHeadingLevel} // 1;

        # "drop" variant removes all headings
        return undef if defined $max && $max eq 'drop';

        my $effective_max = defined $max ? $max : 6;
        my $level = $node->{level};
        $level = $min if $level < $min;
        $level = $effective_max if $level > $effective_max;

        my $children = _sanitize_inlines($node->{children} || [], $policy);
        return undef if @$children == 0;
        return {type => 'heading', level => $level, children => $children};

    # paragraph — recurse; drop if empty
    } elsif ($t eq 'paragraph') {
        my $children = _sanitize_inlines($node->{children} || [], $policy);
        return undef if @$children == 0;
        return {type => 'paragraph', children => $children};

    # code_block — drop or keep as leaf
    } elsif ($t eq 'code_block') {
        return undef if $policy->{dropCodeBlocks};
        return {
            type     => 'code_block',
            language => $node->{language},
            value    => $node->{value},
        };

    # blockquote — drop or recurse
    } elsif ($t eq 'blockquote') {
        return undef if $policy->{dropBlockquotes};
        my $children = _sanitize_blocks($node->{children} || [], $policy);
        return undef if @$children == 0;
        return {type => 'blockquote', children => $children};

    # list — recurse; drop if empty
    } elsif ($t eq 'list') {
        my $children = _sanitize_blocks($node->{children} || [], $policy);
        return undef if @$children == 0;
        return {
            type     => 'list',
            ordered  => $node->{ordered},
            start    => $node->{start},
            tight    => $node->{tight},
            children => $children,
        };

    # list_item — recurse; drop if empty
    } elsif ($t eq 'list_item') {
        my $children = _sanitize_blocks($node->{children} || [], $policy);
        return undef if @$children == 0;
        return {type => 'list_item', children => $children};

    # task_item — recurse; drop if empty
    } elsif ($t eq 'task_item') {
        my $children = _sanitize_blocks($node->{children} || [], $policy);
        return undef if @$children == 0;
        return {
            type     => 'task_item',
            checked  => $node->{checked},
            children => $children,
        };

    # thematic_break — always keep (leaf node)
    } elsif ($t eq 'thematic_break') {
        return {type => 'thematic_break'};

    # raw_block — apply format policy
    } elsif ($t eq 'raw_block') {
        my $field = $policy->{allowRawBlockFormats} // 'passthrough';
        return undef unless _format_allowed($node->{format}, $field);
        return {
            type   => 'raw_block',
            format => $node->{format},
            value  => $node->{value},
        };

    # table — recurse; drop if empty
    } elsif ($t eq 'table') {
        my $children = _sanitize_blocks($node->{children} || [], $policy);
        return undef if @$children == 0;
        return {type => 'table', align => $node->{align}, children => $children};

    # table_row — recurse; drop if empty
    } elsif ($t eq 'table_row') {
        my $children = _sanitize_blocks($node->{children} || [], $policy);
        return undef if @$children == 0;
        return {type => 'table_row', is_header => $node->{is_header}, children => $children};

    # table_cell — recurse inline children; drop if empty
    } elsif ($t eq 'table_cell') {
        my $children = _sanitize_inlines($node->{children} || [], $policy);
        return undef if @$children == 0;
        return {type => 'table_cell', children => $children};

    # Unknown node types are dropped
    } else {
        return undef;
    }
}

# _sanitize_blocks(\@nodes, $policy) → \@nodes
#
# Sanitizes a list of block nodes. Returns a new arrayref with only the
# nodes that survived the policy.
sub _sanitize_blocks {
    my ($nodes, $policy) = @_;
    my @result;
    for my $node (@$nodes) {
        my $sanitized = _sanitize_block($node, $policy);
        push @result, $sanitized if defined $sanitized;
    }
    return \@result;
}

# ─── Public API ───────────────────────────────────────────────────────────────

# sanitize($document, $policy) → document
#
# Sanitizes a document node by applying the given policy. Returns a new
# document node — the input is never mutated. The result is always a valid
# document node (type="document"), even if all children are dropped.
#
# If $policy is omitted or empty, PASSTHROUGH defaults are used.
#
# Example:
#
#   use CodingAdventures::DocumentAstSanitizer;
#
#   my $safe = CodingAdventures::DocumentAstSanitizer::sanitize(
#       $doc,
#       $CodingAdventures::DocumentAstSanitizer::STRICT
#   );
sub sanitize {
    my ($document, $policy) = @_;
    $policy //= {};

    my $children = _sanitize_blocks($document->{children} || [], $policy);
    return {type => 'document', children => $children};
}

# sanitize_children(\@children, $policy) → \@children
#
# Convenience function: sanitizes a list of block nodes directly, without
# requiring a document wrapper. Useful for testing individual subtrees.
sub sanitize_children {
    my ($children, $policy) = @_;
    $policy //= {};
    return _sanitize_blocks($children, $policy);
}

# with_defaults(\%overrides) → \%policy
#
# Creates a complete policy by merging $overrides on top of PASSTHROUGH
# defaults. Callers can specify only the fields they care about:
#
#   my $p = CodingAdventures::DocumentAstSanitizer::with_defaults({
#       dropLinks       => 1,
#       minHeadingLevel => 2,
#   });
sub with_defaults {
    my ($overrides) = @_;
    my %result = %$PASSTHROUGH;
    if ($overrides) {
        for my $k (keys %$overrides) {
            $result{$k} = $overrides->{$k};
        }
    }
    return \%result;
}

1;

__END__

=head1 NAME

CodingAdventures::DocumentAstSanitizer - Pure-Perl document AST sanitizer

=head1 SYNOPSIS

    use CodingAdventures::DocumentAstSanitizer;

    # Sanitize user-generated content with the STRICT policy:
    my $safe = CodingAdventures::DocumentAstSanitizer::sanitize(
        $doc,
        $CodingAdventures::DocumentAstSanitizer::STRICT
    );

    # Build a custom policy:
    my $policy = CodingAdventures::DocumentAstSanitizer::with_defaults({
        dropImages      => 1,
        maxHeadingLevel => 3,
    });

=head1 DESCRIPTION

Sanitizes a Document AST (produced by a Markdown parser) by removing or
neutralising dangerous or unwanted content according to a policy. Works at
the AST level so it is renderer-agnostic and more robust than HTML-level
regex sanitization.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
