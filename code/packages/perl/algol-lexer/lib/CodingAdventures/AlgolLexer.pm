package CodingAdventures::AlgolLexer;

# ============================================================================
# CodingAdventures::AlgolLexer — Grammar-driven ALGOL 60 tokenizer
# ============================================================================
#
# This module tokenizes ALGOL 60 source text into a flat list of token
# hashrefs.  It is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools.  It reads the shared `algol.tokens`
# grammar file, compiles token definitions into Perl regexes, and applies
# them in priority order.
#
# # What is ALGOL 60 tokenization?
# =================================
#
# ALGOL 60 (ALGOrithmic Language, 1960) was the first programming language to
# be formally specified using BNF (Backus-Naur Form).  It introduced block
# structure, lexical scoping, recursion, and the call stack — concepts every
# modern language inherits.
#
# Given the input: begin integer x; x := 42 end
#
# The tokenizer produces a flat list of token hashrefs:
#
#   { type => "BEGIN",       value => "begin", line => 1, col => 1  }
#   { type => "INTEGER",     value => "integer",line=> 1, col => 7  }
#   { type => "NAME",       value => "x",     line => 1, col => 15 }
#   { type => "SEMICOLON",   value => ";",     line => 1, col => 16 }
#   { type => "NAME",       value => "x",     line => 1, col => 18 }
#   { type => "ASSIGN",      value => ":=",    line => 1, col => 20 }
#   { type => "INTEGER_LIT", value => "42",    line => 1, col => 23 }
#   { type => "END",         value => "end",   line => 1, col => 26 }
#   { type => "EOF",         value => "",      line => 1, col => 29 }
#
# Whitespace and comments are consumed silently via skip patterns.
#
# # Keywords
# ===========
#
# The `algol.tokens` grammar uses a `keywords:` section.  Any IDENT token
# whose lowercase value appears in the keyword table is reclassified to the
# corresponding keyword type (uppercased).  Keywords are case-insensitive:
# BEGIN, Begin, and begin all produce type "BEGIN".
#
# Examples:
#   begin       → BEGIN
#   end         → END
#   integer     → INTEGER
#   beginning   → IDENT   (partial match does NOT qualify)
#
# # Comments
# ===========
#
# ALGOL 60 comment syntax: the word `comment` followed by any text up to and
# including the next semicolon.  Example:
#
#   comment this is ignored;
#
# Comments are handled by a COMMENT skip pattern in `algol.tokens` that
# matches `/comment[^;]*;/`.  This means the entire comment (from the word
# `comment` through the closing `;`) is consumed silently.
#
# # Multi-character operators
# ===========================
#
# ALGOL 60 uses several multi-character operators that must be tokenized
# before their single-character components:
#
#   :=   ASSIGN   (assignment; must precede : COLON)
#   **   POWER    (exponentiation; must precede * STAR)
#   <=   LEQ      (less-or-equal; must precede < LT)
#   >=   GEQ      (greater-or-equal; must precede > GT)
#   !=   NEQ      (not-equal; must precede any ! use)
#
# The `algol.tokens` file lists these in the correct priority order.
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar()` opens `algol.tokens`, parses it with
#    `CodingAdventures::GrammarTools::parse_token_grammar`, and caches the
#    resulting `TokenGrammar` object for the lifetime of the process.
#
# 2. **Pattern compilation** — `_build_rules()` converts every `TokenDefinition`
#    in the grammar into a `{ name => ..., pat => qr/\G.../ }` hashref.
#    Regex definitions get `qr/\G(?:<pattern>)/` directly; literal definitions
#    get `qr/\G\Q<literal>\E/`.
#
# 3. **Keyword table** — `_build_rules()` also extracts the keyword list from
#    `$grammar->keywords` (a hashref of lowercase word → token type).
#
# 4. **Tokenization** — `tokenize()` walks the source string using Perl's
#    `\G` + `pos()` mechanism:
#      a. Try skip patterns (whitespace, comments); advance silently on match.
#      b. Try token patterns in definition order; first match wins.
#      c. After matching IDENT, consult the keyword table; reclassify if found.
#      d. Die on no match.
#
# # Path navigation
# =================
#
# `__FILE__` resolves to `lib/CodingAdventures/AlgolLexer.pm`.
# `dirname(__FILE__)` → `lib/CodingAdventures`
#
# From there we need to climb to the repo root (`code/`) then descend
# into `grammars/algol.tokens`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → algol-lexer/       (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/             ← repo root
#   + /grammars/algol.tokens
#
# This is identical depth to json-lexer (same monorepo structure).
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use File::Basename qw(dirname);
use File::Spec;
use CodingAdventures::GrammarTools;

# ============================================================================
# Grammar loading and caching
# ============================================================================
#
# Reading and parsing the grammar file on every tokenize() call would be
# wasteful.  We cache the TokenGrammar object, the compiled rule list, and
# the keyword table in package-level variables.  They are populated on the
# first call and reused thereafter.

my $_grammar;      # CodingAdventures::GrammarTools::TokenGrammar
my $_rules;        # arrayref of { name => str, pat => qr// }
my $_skip_rules;   # arrayref of qr// patterns for skip definitions
my $_keywords;     # hashref of lowercase_word => TOKEN_TYPE
my $_grammar_version;

# --- _grammars_dir() ----------------------------------------------------------
#
# Return the absolute path to the shared `grammars/` directory in the
# monorepo, computed relative to this module file.
#
# We use File::Spec for cross-platform path construction and
# File::Basename::dirname to strip the filename component.

sub _grammars_dir {
    # __FILE__ = .../code/packages/perl/algol-lexer/lib/CodingAdventures/AlgolLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → algol-lexer/ → perl/ → packages/ → code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _grammar() ---------------------------------------------------------------
#
# Load and parse `algol.tokens`, caching the result.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.

sub _normalize_version {
    my ($version) = @_;
    $version = 'algol60' if !defined($version) || $version eq '';
    die "CodingAdventures::AlgolLexer: unknown ALGOL version '$version' (valid: algol60)"
        unless $version eq 'algol60';
    return $version;
}

sub _grammar {
    my ($version) = @_;
    $version = _normalize_version($version);
    return $_grammar if $_grammar && defined $_grammar_version && $_grammar_version eq $version;

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'algol', "$version.tokens" );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::AlgolLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::AlgolLexer: failed to parse $version.tokens: $err"
        unless $grammar;

    $_grammar = $grammar;
    $_grammar_version = $version;
    return $_grammar;
}

# --- _build_rules() -----------------------------------------------------------
#
# Convert TokenGrammar definitions into compiled Perl patterns and a keyword
# lookup table.
#
#   $_rules      — token definitions, each { name => str, pat => qr/\G.../ }
#   $_skip_rules — skip definitions, each qr/\G.../
#   $_keywords   — hashref: lowercase_word → uppercase_token_type
#
# Pattern compilation strategy:
#
#   is_regex == 1  →  wrap in qr/\G(?:<pattern>)/  (raw regex string)
#   is_regex == 0  →  use qr/\G\Q<literal>\E/       (literal text)
#
# The `\G` anchor forces the match to start exactly at `pos($source)`,
# preventing the regex engine from skipping ahead.
#
# Keyword table construction:
#   `$grammar->keywords` returns a hashref of lowercase_word → token_type.
#   We store this directly; keywords are checked only after an IDENT match.
#
# Token type resolution:
#   If a definition has an alias (the `-> ALIAS` syntax in .tokens files),
#   we emit the alias as the token type; otherwise we emit the definition name.

sub _build_rules {
    my ($version) = @_;
    $version = _normalize_version($version);
    return if $_rules && defined $_grammar_version && $_grammar_version eq $version;

    $_rules = undef;
    $_skip_rules = undef;
    $_keywords = undef;

    my $grammar = _grammar($version);
    my (@rules, @skip_rules);

    # Build skip patterns
    for my $defn ( @{ $grammar->skip_definitions } ) {
        my $pat;
        if ( $defn->is_regex ) {
            # Security: reject patterns containing Perl code-execution constructs.
            # (?{ ... }) and (??{ ... }) allow arbitrary Perl code to run inside
            # a regex match. These constructs should never appear in a grammar
            # file from disk. Die early rather than silently execute injected code.
            # Fixed: 2026-04-10 security review.
            my $raw_pat = $defn->pattern;
            if ( $raw_pat =~ /\(\?{|\(\?\?{/ ) {
                die "Security error: unsafe Perl regex code construct in grammar pattern '$raw_pat'
";
            }
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
            # Security: reject patterns containing Perl code-execution constructs.
            # (?{ ... }) and (??{ ... }) allow arbitrary Perl code to run inside
            # a regex match. These constructs should never appear in a grammar
            # file from disk. Die early rather than silently execute injected code.
            # Fixed: 2026-04-10 security review.
            my $raw_pat = $defn->pattern;
            if ( $raw_pat =~ /\(\?{|\(\?\?{/ ) {
                die "Security error: unsafe Perl regex code construct in grammar pattern '$raw_pat'
";
            }
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

    # Build the keyword lookup table.
    # $grammar->keywords returns an arrayref of lowercase keyword strings:
    #   ['begin', 'end', 'if', 'then', ...]
    # We convert this to a hashref for O(1) lookup.
    # Each keyword's token type is the uppercased form of the keyword:
    #   'begin' → 'BEGIN', 'integer' → 'INTEGER', etc.
    my %kw_table;
    for my $kw ( @{ $grammar->keywords } ) {
        $kw_table{$kw} = uc($kw);
    }
    $_keywords = \%kw_table;

    $_skip_rules = \@skip_rules;
    $_rules      = \@rules;
}

# ============================================================================
# Public API
# ============================================================================

# --- tokenize($source) --------------------------------------------------------
#
# Tokenize an ALGOL 60 source string.
#
# Algorithm:
#
#   1. Ensure the grammar and compiled rules are loaded (_build_rules).
#   2. Walk the source from position 0 to end.
#   3. At each position, try each skip pattern (whitespace, comments) with /gc.
#      If a skip pattern matches, update line/col tracking and continue.
#   4. If no skip matched, try each token pattern in order.
#      The first match: check if IDENT needs keyword reclassification.
#      Record token, advance pos, update tracking, continue.
#   5. If nothing matched, die with a descriptive error message.
#   6. After exhausting the input, push an EOF sentinel and return.
#
# Keyword reclassification:
#   When a token matches the IDENT pattern, we look up its lowercase value
#   in the keyword table.  If found, the token type is replaced with the
#   keyword type.  This implements case-insensitive keyword matching:
#   BEGIN, Begin, and begin all become type "BEGIN".
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
    my ($class_or_self, $source, %opts) = @_;
    my $version = _normalize_version(delete($opts{version}));
    die "CodingAdventures::AlgolLexer: unknown options: " . join(", ", sort keys %opts)
        if %opts;

    _build_rules($version);

    my @tokens;
    my $line = 1;
    my $col  = 1;
    my $pos  = 0;
    my $len  = length($source);

    while ($pos < $len) {
        pos($source) = $pos;

        # ---- Try skip patterns -----------------------------------------------
        #
        # Skip patterns are tried before token patterns.  In ALGOL 60:
        #   - Whitespace (spaces, tabs, carriage returns, newlines) is ignored.
        #   - Comments begin with the keyword `comment` and end at the next `;`.
        # We update line/col tracking so token positions are accurate.

        my $skipped = 0;
        for my $spat (@$_skip_rules) {
            pos($source) = $pos;
            if ($source =~ /$spat/gc) {
                my $matched = $&;

                # Count newlines to update line/col
                my $nl_count = () = $matched =~ /\n/g;
                if ($nl_count) {
                    $line += $nl_count;
                    # Column resets to 1 plus whatever follows the last newline
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
        # failure, anchored to \G).  The first match wins.
        #
        # After matching IDENT, we check if the lowercased value is a keyword.
        # If so, the token type is reclassified to the keyword type.
        # This makes keyword matching case-insensitive:
        #   BEGIN → BEGIN, begin → BEGIN, Begin → BEGIN.

        my $matched_tok = 0;
        for my $rule (@$_rules) {
            pos($source) = $pos;
            if ($source =~ /$rule->{pat}/gc) {
                my $value = $&;
                my $type  = $rule->{name};

                # Keyword reclassification: IDENT may actually be a keyword.
                # The keyword table maps lowercase word to uppercase token type.
                if ($type eq 'NAME') {
                    my $lc_val = lc($value);
                    if ( exists $_keywords->{$lc_val} ) {
                        $type = uc($lc_val);
                    }
                }

                push @tokens, {
                    type  => $type,
                    value => $value,
                    line  => $line,
                    col   => $col,
                };

                # Advance position
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
        # Well-formed ALGOL 60 source should never reach here.  We emit a
        # descriptive error that includes the position and the offending
        # character to aid debugging.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::AlgolLexer: LexerError at line %d col %d: "
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

CodingAdventures::AlgolLexer - Grammar-driven ALGOL 60 tokenizer

=head1 SYNOPSIS

    use CodingAdventures::AlgolLexer;

    my $tokens = CodingAdventures::AlgolLexer->tokenize('begin integer x; x := 42 end');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<algol.tokens> file, compiles token definitions to Perl regexes,
and tokenizes ALGOL 60 source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace and comments (C<comment ... ;>) are silently consumed.
Keywords are case-insensitive: C<BEGIN>, C<Begin>, and C<begin> all produce
token type C<BEGIN>.  The last token is always C<EOF>.

=head1 METHODS

=head2 tokenize($source)

Tokenize an ALGOL 60 string.  Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
