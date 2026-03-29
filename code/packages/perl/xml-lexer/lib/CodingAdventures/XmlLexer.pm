package CodingAdventures::XmlLexer;

# ============================================================================
# CodingAdventures::XmlLexer — Context-sensitive XML tokenizer
# ============================================================================
#
# This module tokenizes XML source text using the shared `xml.tokens` grammar
# file.  It follows the same architecture as CodingAdventures::JsonLexer but
# adds a **group stack** to handle XML's context-sensitive lexical rules.
#
# # Why XML tokenization is context-sensitive
# =============================================
#
# In XML, the character `=` means different things depending on context:
#
#   Inside a tag:      <div class="main">   — `=` is an attribute delimiter
#   In text content:   1 + 1 = 2            — `=` is plain text
#
# The `xml.tokens` grammar handles this with **pattern groups**.  Each group
# defines which patterns are active.  A group stack tracks the current context:
#
#   default (implicit) — text between tags, entity refs, tag openers
#   tag                — inside <...> or </...>: names, attr values, closers
#   comment            — inside <!-- ... -->
#   cdata              — inside <![CDATA[ ... ]]>
#   pi                 — inside <? ... ?>
#
# # Group stack protocol
# =======================
#
# We maintain `@_group_stack` (package-level, reset on each `tokenize` call).
# After every token match, we check the token type and push/pop accordingly:
#
#   OPEN_TAG_START  or CLOSE_TAG_START → push "tag"
#   TAG_CLOSE       or SELF_CLOSE      → pop
#   COMMENT_START                      → push "comment"
#   COMMENT_END                        → pop
#   CDATA_START                        → push "cdata"
#   CDATA_END                          → pop
#   PI_START                           → push "pi"
#   PI_END                             → pop
#
# # Architecture
# ==============
#
# Grammar loading:  `_grammar()` reads `xml.tokens` once and caches the
#   TokenGrammar object.  `_build_rules()` compiles each TokenDefinition into
#   a `{ name => str, pat => qr/\G.../ }` hashref, and separates skip patterns.
#
# Rules are organised into groups: `%_group_rules` maps group name to an
# arrayref of compiled rule hashrefs.  The active group's rules are tried
# at each position.  If the active group fails to match, we fall back to
# the default (top-level) rules.
#
# Tokenization:  `tokenize()` walks the source with `pos()` / `\G`, applying
#   the active group's rules.  After each token, it calls `_update_group_stack`
#   to push or pop groups.
#
# # Path navigation
# =================
#
# __FILE__  = lib/CodingAdventures/XmlLexer.pm
# dirname   = lib/CodingAdventures/
# Up 5 levels: CodingAdventures → lib → xml-lexer → perl → packages → code
# Then: /grammars/xml.tokens

use strict;
use warnings;

our $VERSION = '0.01';

use File::Basename qw(dirname);
use File::Spec;
use CodingAdventures::GrammarTools qw(parse_token_grammar);

# ============================================================================
# Grammar and rule caches (package-level)
# ============================================================================

my $_grammar;       # TokenGrammar object
my $_default_rules; # arrayref of { name, pat } for top-level definitions
my $_group_rules;   # hashref: group_name → arrayref of { name, pat }
my $_skip_rules;    # arrayref of qr// for skip definitions
my $_rules_built;   # flag

# ============================================================================
# Path resolution
# ============================================================================

sub _grammars_dir {
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb: CodingAdventures → lib → xml-lexer → perl → packages → code
    for (1..5) { $dir = dirname($dir); }
    return File::Spec->catdir($dir, 'grammars');
}

# ============================================================================
# Grammar loading
# ============================================================================

sub _grammar {
    return $_grammar if $_grammar;

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'xml.tokens' );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::XmlLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = parse_token_grammar($content);
    die "CodingAdventures::XmlLexer: failed to parse xml.tokens: $err"
        unless $grammar;

    $_grammar = $grammar;
    return $_grammar;
}

# ============================================================================
# Rule compilation
# ============================================================================
#
# compile_defn($defn) → { name => str, pat => qr/\G.../ }
#
# For regex definitions (is_regex == 1) we embed the pattern string directly.
# For literal definitions (is_regex == 0) we use \Q...\E to escape it.
# The type emitted is the alias if present, otherwise the definition name.

sub _compile_defn {
    my ($defn) = @_;
    my $pat;
    if ( $defn->is_regex ) {
        $pat = qr/\G(?:${\$defn->pattern})/;
    } else {
        my $lit = $defn->pattern;
        $pat = qr/\G\Q$lit\E/;
    }
    my $type = ( $defn->alias && $defn->alias ne '' )
                ? $defn->alias
                : $defn->name;
    return { name => $type, pat => $pat };
}

sub _build_rules {
    return if $_rules_built;

    my $grammar = _grammar();

    # Skip patterns
    my @skip_rules;
    for my $defn ( @{ $grammar->skip_definitions } ) {
        if ( $defn->is_regex ) {
            push @skip_rules, qr/\G(?:${\$defn->pattern})/;
        } else {
            my $lit = $defn->pattern;
            push @skip_rules, qr/\G\Q$lit\E/;
        }
    }

    # Top-level (default group) token rules
    my @default_rules = map { _compile_defn($_) } @{ $grammar->definitions };

    # Per-group token rules
    my %group_rules;
    if ( $grammar->groups ) {
        for my $group_name ( keys %{ $grammar->groups } ) {
            my $group = $grammar->groups->{$group_name};
            $group_rules{$group_name} = [
                map { _compile_defn($_) } @{ $group->definitions }
            ];
        }
    }

    $_skip_rules    = \@skip_rules;
    $_default_rules = \@default_rules;
    $_group_rules   = \%group_rules;
    $_rules_built   = 1;
}

# ============================================================================
# Group stack management
# ============================================================================
#
# The group stack tracks which pattern set is currently active.  "default"
# is always at the bottom; other groups are pushed on top.
#
# `_active_rules()` returns the rules for the current top-of-stack group,
# falling back to the default rules if the group has no compiled rules.

my @_group_stack;   # reset at the start of each tokenize() call

sub _push_group { push @_group_stack, $_[0] }
sub _pop_group  { pop  @_group_stack if @_group_stack > 1 }

sub _active_rules {
    my $top = $_group_stack[-1];
    if ( $top ne 'default' && $_group_rules->{$top} ) {
        return $_group_rules->{$top};
    }
    return $_default_rules;
}

# --- _update_group_stack($type) -----------------------------------------------
#
# Given the type of the token just emitted, push or pop the group stack.
# This implements the callback protocol from the xml.tokens grammar comment.

sub _update_group_stack {
    my ($type) = @_;

    if ( $type eq 'OPEN_TAG_START' || $type eq 'CLOSE_TAG_START' ) {
        _push_group('tag');

    } elsif ( $type eq 'TAG_CLOSE' || $type eq 'SELF_CLOSE' ) {
        _pop_group();

    } elsif ( $type eq 'COMMENT_START' ) {
        _push_group('comment');
    } elsif ( $type eq 'COMMENT_END' ) {
        _pop_group();

    } elsif ( $type eq 'CDATA_START' ) {
        _push_group('cdata');
    } elsif ( $type eq 'CDATA_END' ) {
        _pop_group();

    } elsif ( $type eq 'PI_START' ) {
        _push_group('pi');
    } elsif ( $type eq 'PI_END' ) {
        _pop_group();
    }
}

# ============================================================================
# Public API
# ============================================================================

# --- tokenize($source) --------------------------------------------------------
#
# Tokenize an XML source string.
#
# Returns an arrayref of token hashrefs (type, value, line, col).
# The last element always has type 'EOF'.
#
# Dies on unexpected input with a descriptive "LexerError" message.

sub tokenize {
    my ($class_or_self, $source) = @_;

    _build_rules();

    # Reset the group stack: start in the default (content) context.
    @_group_stack = ('default');

    my @tokens;
    my $line = 1;
    my $col  = 1;
    my $pos  = 0;
    my $len  = length($source);

    while ($pos < $len) {
        pos($source) = $pos;

        # ---- Skip patterns ---------------------------------------------------
        #
        # Whitespace is significant inside comment/cdata/pi groups (it becomes
        # part of the content token).  In the default and tag groups, whitespace
        # is insignificant and should be consumed silently.
        #
        # The xml.tokens grammar only declares a WHITESPACE skip pattern for
        # the implicit top-level group.  When we are inside comment/cdata/pi,
        # those groups' own patterns (which match anything up to the closing
        # delimiter) consume whitespace as part of the text token.
        #
        # So applying skip patterns only when we are in the default or tag
        # groups is safe and matches the grammar's intent.

        my $active_group = $_group_stack[-1];
        if ( $active_group eq 'default' || $active_group eq 'tag' ) {
            my $skipped = 0;
            for my $spat (@$_skip_rules) {
                pos($source) = $pos;
                if ($source =~ /$spat/gc) {
                    my $matched = $&;
                    my $nl_count = () = $matched =~ /\n/g;
                    if ($nl_count) {
                        $line += $nl_count;
                        my $after = $matched; $after =~ s/.*\n//s;
                        $col = length($after) + 1;
                    } else {
                        $col += length($matched);
                    }
                    $pos = pos($source);
                    $skipped = 1;
                    last;
                }
            }
            next if $skipped;
        }

        # ---- Token patterns --------------------------------------------------
        #
        # Try each rule in the active group.  On match: record token, advance,
        # update line/col, update group stack.

        my $rules = _active_rules();
        my $matched_tok = 0;

        for my $rule (@$rules) {
            pos($source) = $pos;
            if ($source =~ /$rule->{pat}/gc) {
                my $value = $&;

                push @tokens, {
                    type  => $rule->{name},
                    value => $value,
                    line  => $line,
                    col   => $col,
                };

                $pos = pos($source);

                my $nl_count = () = $value =~ /\n/g;
                if ($nl_count) {
                    $line += $nl_count;
                    my $after = $value; $after =~ s/.*\n//s;
                    $col = length($after) + 1;
                } else {
                    $col += length($value);
                }

                # Update the group stack based on the token just emitted.
                _update_group_stack( $rule->{name} );

                $matched_tok = 1;
                last;
            }
        }

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::XmlLexer: LexerError at line %d col %d: "
              . "unexpected character '%s' (group: %s)",
                $line, $col, $ch, $_group_stack[-1]
            );
        }
    }

    push @tokens, { type => 'EOF', value => '', line => $line, col => $col };
    return \@tokens;
}

1;

__END__

=head1 NAME

CodingAdventures::XmlLexer - Context-sensitive XML tokenizer

=head1 SYNOPSIS

    use CodingAdventures::XmlLexer;

    my $tokens = CodingAdventures::XmlLexer->tokenize('<root attr="v">text</root>');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A grammar-driven XML tokenizer that handles XML's context-sensitive lexical
structure using a pattern-group stack.  Reads the shared C<xml.tokens> grammar
file, compiles token definitions to Perl regexes grouped by context (default,
tag, comment, cdata, pi), and tokenizes XML source into a flat list of token
hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.
The last token is always C<EOF>.

=head1 METHODS

=head2 tokenize($source)

Tokenize an XML string.  Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive "LexerError" message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
