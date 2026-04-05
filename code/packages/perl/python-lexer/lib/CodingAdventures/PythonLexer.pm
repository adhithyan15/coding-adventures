package CodingAdventures::PythonLexer;

# ============================================================================
# CodingAdventures::PythonLexer — Grammar-driven Python tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools. It reads the shared `python.tokens`
# grammar file, compiles the token definitions into Perl regexes, and applies
# them in priority order to tokenize Python source code.
#
# # What is Python tokenization?
# =================================
#
# Given the input:  def foo(x):
#
# The tokenizer produces a flat list of token hashrefs:
#
#   { type => "DEF",    value => "def",  line => 1, col => 1  }
#   { type => "NAME",   value => "foo",  line => 1, col => 5  }
#   { type => "LPAREN", value => "(",    line => 1, col => 8  }
#   { type => "NAME",   value => "x",    line => 1, col => 9  }
#   { type => "RPAREN", value => ")",    line => 1, col => 10 }
#   { type => "COLON",  value => ":",    line => 1, col => 11 }
#   { type => "EOF",    value => "",     line => 1, col => 12 }
#
# Whitespace is consumed silently — skip patterns in `python.tokens`
# match whitespace and it is never emitted as a token.
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar()` opens `python.tokens`, parses it
#    with `CodingAdventures::GrammarTools::parse_token_grammar`, and caches
#    the result for the lifetime of the process.
#
# 2. **Pattern compilation** — `_build_rules()` converts every TokenDefinition
#    in the grammar into a `{ name => str, pat => qr/\G.../ }` hashref.
#    Regex definitions use `qr/\G(?:<pattern>)/`; literal definitions use
#    `qr/\G\Q<literal>\E/` to disable metacharacter interpretation.
#
# 3. **Tokenization** — `tokenize()` walks the source string using Perl's
#    `\G` + `pos()` mechanism, trying skip patterns first and then token
#    patterns in definition order. First match wins. On no match, dies with
#    position info.
#
# # Path navigation
# =================
#
# `__FILE__` resolves to `lib/CodingAdventures/PythonLexer.pm`.
# `dirname(__FILE__)` → `lib/CodingAdventures`
#
# From there we climb to the repo root (`code/`) then descend into
# `grammars/python.tokens`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → python-lexer/     (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/             ← repo root
#   + /grammars/python.tokens
#
# # Token types
# =============
#
# NAME        — identifiers; promoted to keyword type if in keywords section
# NUMBER      — integer literals (e.g. 42, 0)
# STRING      — double-quoted string literals
#
# Keyword tokens (promoted from NAME):
#   IF, ELIF, ELSE, WHILE, FOR, DEF, RETURN, CLASS, IMPORT, FROM,
#   AS, TRUE, FALSE, NONE
#
# Multi-char operators (matched before single-char ones):
#   EQUALS_EQUALS
#
# Single-char operators:
#   EQUALS, PLUS, MINUS, STAR, SLASH
#
# Delimiters:
#   LPAREN, RPAREN, COMMA, COLON
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use File::Basename qw(dirname);
use File::Spec;
use CodingAdventures::GrammarTools;

# DefaultVersion is the Python version used when no version is specified.
use constant DEFAULT_VERSION => '3.12';

# SupportedVersions lists all Python versions with grammar files.
our @SUPPORTED_VERSIONS = ('2.7', '3.0', '3.6', '3.8', '3.10', '3.12');

# ============================================================================
# Grammar loading and caching
# ============================================================================
#
# Reading and parsing the grammar file on every tokenize() call would be
# wasteful. We cache TokenGrammar objects and compiled rule lists keyed
# by version string. They are populated on first use and reused thereafter.

my %_grammar_cache;   # version => CodingAdventures::GrammarTools::TokenGrammar
my %_rules_cache;     # version => arrayref of { name => str, pat => qr// }
my %_skip_cache;      # version => arrayref of qr// patterns for skip definitions
my %_keyword_cache;   # version => hashref mapping keyword string => promoted type

# --- _grammars_dir() ----------------------------------------------------------
#
# Return the absolute path to the shared `grammars/` directory in the
# monorepo, computed relative to this module file.

sub _grammars_dir {
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ -> lib/ -> python-lexer/ -> perl/ -> packages/ -> code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _resolve_version($version) -----------------------------------------------
#
# Return the version string to use. If undef or empty, returns DEFAULT_VERSION.

sub _resolve_version {
    my ($version) = @_;
    return DEFAULT_VERSION if !defined($version) || $version eq '';
    return $version;
}

# --- _grammar_path($version) --------------------------------------------------
#
# Return the path to the .tokens file for the given Python version.

sub _grammar_path {
    my ($version) = @_;
    return File::Spec->catfile( _grammars_dir(), 'python', "python${version}.tokens" );
}

# --- _grammar($version) -------------------------------------------------------
#
# Load and parse the versioned grammar, caching per version.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.

sub _grammar {
    my ($version) = @_;
    my $v = _resolve_version($version);

    return $_grammar_cache{$v} if $_grammar_cache{$v};

    my $tokens_file = _grammar_path($v);
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::PythonLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::PythonLexer: failed to parse python${v}.tokens: $err"
        unless $grammar;

    $_grammar_cache{$v} = $grammar;
    return $grammar;
}

# --- _build_rules() -----------------------------------------------------------
#
# Convert TokenGrammar definitions into two lists of compiled Perl patterns:
#
#   $_rules      — token definitions, each { name => str, pat => qr/\G.../ }
#   $_skip_rules — skip definitions, each qr/\G.../
#
# Pattern compilation strategy:
#
#   is_regex == 1  →  treat `$defn->pattern` as a raw regex string.
#                     Wrap in qr/\G(?:<pattern>)/ to anchor at current pos.
#
#   is_regex == 0  →  treat `$defn->pattern` as a literal string.
#                     Use `\Q...\E` to disable regex metacharacters.
#                     This is critical for operators like ==, etc.
#
# The `\G` anchor forces the match to start exactly at `pos($source)`,
# preventing the regex engine from skipping ahead.
#
# Alias resolution: definitions with `-> ALIAS` emit the alias as type name.

sub _build_rules {
    my ($version) = @_;
    my $v = _resolve_version($version);

    return if $_rules_cache{$v};    # already built for this version

    my $grammar = _grammar($v);
    my (@rules, @skip_rules);

    # Build skip patterns
    for my $defn ( @{ $grammar->skip_definitions } ) {
        my $pat;
        if ( $defn->is_regex ) {
            $pat = qr/\G(?:${\$defn->pattern})/;
        } else {
            my $lit = $defn->pattern;
            $pat = qr/\G\Q$lit\E/;
        }
        push @skip_rules, $pat;
    }

    # Build token patterns
    for my $defn ( @{ $grammar->definitions } ) {
        my $pat;
        if ( $defn->is_regex ) {
            $pat = qr/\G(?:${\$defn->pattern})/;
        } else {
            my $lit = $defn->pattern;
            $pat = qr/\G\Q$lit\E/;
        }
        # Emit the alias if one exists, otherwise use the definition name.
        my $type = ( $defn->alias && $defn->alias ne '' )
                    ? $defn->alias
                    : $defn->name;
        push @rules, { name => $type, pat => $pat };
    }

    # If the grammar has no skip definitions (e.g. python.tokens has none),
    # add a default whitespace skip so that spaces, tabs, carriage returns, and
    # newlines between tokens are silently consumed.
    unless (@skip_rules) {
        push @skip_rules, qr/\G[ \t\r\n]+/;
    }

    # Build keyword lookup map from the grammar keywords section.
    my %kw_map;
    $kw_map{$_} = uc($_) for @{ $grammar->keywords };
    $_keyword_cache{$v} = \%kw_map;

    $_skip_cache{$v} = \@skip_rules;
    $_rules_cache{$v} = \@rules;
}

# ============================================================================
# Public API
# ============================================================================

# --- tokenize($source) --------------------------------------------------------
#
# Tokenize a Python source string.
#
# Algorithm:
#
#   1. Ensure grammar and compiled rules are loaded (_build_rules).
#   2. Walk the source from position 0 to end.
#   3. At each position, set pos($source) and try each skip pattern with /gc.
#      If a skip pattern matches, update line/col tracking and continue.
#   4. If no skip matched, try each token pattern in order.
#      The first match: record token, advance pos, update tracking, continue.
#   5. If nothing matched, die with a descriptive error message.
#   6. After exhausting the input, push an EOF sentinel and return.
#
# Line/column tracking:
#
#   - `$line` starts at 1, incremented for each '\n' in matched text.
#   - `$col`  starts at 1:
#       - If the match contains no newlines: col += length(match).
#       - If the match contains newlines: col = length of text after last '\n'.
#
# Return value:
#
#   An arrayref of hashrefs, each with keys: type, value, line, col.
#   The last element always has type 'EOF'.
#
# Raises:
#
#   `die` with a "LexerError" message on unexpected input.

sub tokenize {
    my ($class_or_self, $source) = @_;

    _build_rules();

    my @tokens;
    my $line = 1;
    my $col  = 1;
    my $pos  = 0;
    my $len  = length($source);

    while ($pos < $len) {
        pos($source) = $pos;

        # ---- Try skip patterns -----------------------------------------------
        #
        # Whitespace in Python source (spaces, tabs, and newlines between
        # tokens) is insignificant to the parser. We advance position without
        # emitting anything, but still update line/col so that token positions
        # after whitespace are accurate.

        my $skipped = 0;
        for my $spat (@$_skip_rules) {
            pos($source) = $pos;
            if ($source =~ /$spat/gc) {
                my $matched = $&;

                # Count newlines to update line/col
                my $nl_count = () = $matched =~ /\n/g;
                if ($nl_count) {
                    $line += $nl_count;
                    my $after_last_nl = $matched;
                    $after_last_nl =~ s/.*\n//s;
                    $col = length($after_last_nl) + 1;
                } else {
                    $col += length($matched);
                }

                $pos = pos($source);
                $skipped = 1;
                last;
            }
        }
        next if $skipped;

        # ---- Try token patterns ----------------------------------------------
        #
        # Each pattern is tried at the current pos() using /gc (keep pos on
        # failure, anchored to \G). First match wins.

        my $matched_tok = 0;
        for my $rule (@$_rules) {
            pos($source) = $pos;
            if ($source =~ /$rule->{pat}/gc) {
                my $value = $&;

                my $tok_type = $rule->{name};
                if ($tok_type eq 'NAME' && exists $_keyword_map->{$value}) {
                    $tok_type = $_keyword_map->{$value};
                }
                push @tokens, {
                    type  => $tok_type,
                    value => $value,
                    line  => $line,
                    col   => $col,
                };

                $pos = pos($source);

                # Update line/col tracking
                my $nl_count = () = $value =~ /\n/g;
                if ($nl_count) {
                    $line += $nl_count;
                    my $after_last_nl = $value;
                    $after_last_nl =~ s/.*\n//s;
                    $col = length($after_last_nl) + 1;
                } else {
                    $col += length($value);
                }

                $matched_tok = 1;
                last;
            }
        }

        # ---- No match — unexpected character ---------------------------------
        #
        # A well-formed Python source should rarely reach here. We emit a
        # descriptive error including position and the offending character.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::PythonLexer: LexerError at line %d col %d: "
              . "unexpected character '%s'",
                $line, $col, $ch
            );
        }
    }

    # Sentinel EOF token — always present as the last element.
    push @tokens, { type => 'EOF', value => '', line => $line, col => $col };

    return \@tokens;
}

1;

__END__

=head1 NAME

CodingAdventures::PythonLexer - Grammar-driven Python tokenizer

=head1 SYNOPSIS

    use CodingAdventures::PythonLexer;

    my $tokens = CodingAdventures::PythonLexer->tokenize('def foo(x):');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<python.tokens> file, compiles token definitions to Perl regexes,
and tokenizes Python source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace is silently consumed. The last token is always C<EOF>.

Token types include: NAME, NUMBER, STRING; keyword types: IF, ELIF, ELSE,
WHILE, FOR, DEF, RETURN, CLASS, IMPORT, FROM, AS, TRUE, FALSE, NONE;
operator types: EQUALS_EQUALS, EQUALS, PLUS, MINUS, STAR, SLASH; delimiter
types: LPAREN, RPAREN, COMMA, COLON.

=head1 METHODS

=head2 tokenize($source)

Tokenize a Python string. Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
