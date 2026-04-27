package CodingAdventures::VhdlLexer;

# ============================================================================
# CodingAdventures::VhdlLexer — Grammar-driven VHDL tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools. It reads the shared `vhdl.tokens`
# grammar file, compiles the token definitions into Perl regexes, and applies
# them in priority order to tokenize VHDL (IEEE 1076-2008) source code.
#
# # What is VHDL tokenization?
# ================================
#
# VHDL (VHSIC Hardware Description Language) was designed by the US Department
# of Defense for documenting and simulating digital systems. Where Verilog is
# terse and C-like, VHDL is verbose and Ada-like: strongly typed, explicitly
# declared, and case-insensitive. ENTITY, Entity, and entity are all identical.
#
# Given the input:  entity adder is port (a : in std_logic);
#
# The tokenizer produces:
#
#   { type => "ENTITY",    value => "entity",    line => 1, col => 1  }
#   { type => "NAME",      value => "adder",     line => 1, col => 8  }
#   { type => "IS",        value => "is",        line => 1, col => 14 }
#   { type => "PORT",      value => "port",      line => 1, col => 17 }
#   { type => "LPAREN",    value => "(",         line => 1, col => 22 }
#   { type => "NAME",      value => "a",         line => 1, col => 23 }
#   { type => "COLON",     value => ":",         line => 1, col => 25 }
#   { type => "IN",        value => "in",        line => 1, col => 27 }
#   { type => "NAME",      value => "std_logic", line => 1, col => 30 }
#   { type => "RPAREN",    value => ")",         line => 1, col => 39 }
#   { type => "SEMICOLON", value => ";",         line => 1, col => 40 }
#   { type => "EOF",       value => "",          line => 1, col => 41 }
#
# VHDL is case-insensitive: `vhdl.tokens` sets `case_sensitive: false`,
# so the grammar lowercases all input before matching. All token values
# in the output are lowercase.
#
# Whitespace and single-line comments (-- ...) are consumed silently.
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar()` opens `vhdl.tokens`, parses it
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
# `__FILE__` resolves to `lib/CodingAdventures/VhdlLexer.pm`.
#
# From there we climb to the repo root (`code/`) then descend into
# `grammars/vhdl.tokens`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → vhdl-lexer/    (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/           ← repo root
#   + /grammars/vhdl.tokens
#
# # Token types
# =============
#
# Keyword tokens (NAME values with lowercased text matched against keyword list):
#   ABS, ACCESS, AFTER, ALIAS, ALL, AND, ARCHITECTURE, ARRAY, ASSERT,
#   ATTRIBUTE, BEGIN, BLOCK, BODY, BUFFER, BUS, CASE, COMPONENT,
#   CONFIGURATION, CONSTANT, DISCONNECT, DOWNTO, ELSE, ELSIF, END, ENTITY,
#   EXIT, FILE, FOR, FUNCTION, GENERATE, GENERIC, GROUP, GUARDED, IF,
#   IMPURE, IN, INOUT, IS, LABEL, LIBRARY, LINKAGE, LITERAL, LOOP, MAP,
#   MOD, NAND, NEW, NEXT, NOR, NOT, NULL, OF, ON, OPEN, OR, OTHERS, OUT,
#   PACKAGE, PORT, POSTPONED, PROCEDURE, PROCESS, PURE, RANGE, RECORD,
#   REGISTER, REJECT, REM, REPORT, RETURN, ROL, ROR, SELECT, SEVERITY,
#   SIGNAL, SHARED, SLA, SLL, SRA, SRL, SUBTYPE, THEN, TO, TRANSPORT,
#   TYPE, UNAFFECTED, UNITS, UNTIL, USE, VARIABLE, WAIT, WHEN, WHILE,
#   WITH, XNOR, XOR
#
# Literal/regex tokens:
#   BASED_LITERAL  — e.g. 16#FF#, 2#1010#
#   REAL_NUMBER    — e.g. 3.14, 1.0E-3
#   NUMBER         — plain integers like 42, 1_000
#   STRING         — double-quoted string (use "" for embedded quote)
#   BIT_STRING     — prefix + quoted: X"FF", B"1010", O"77"
#   CHAR_LITERAL   — std_logic char: '0', '1', 'X', 'Z'
#   EXTENDED_IDENT — backslash-delimited: \my odd name\
#   NAME           — regular identifier
#
# Two-char operators: VAR_ASSIGN (:=), LESS_EQUALS (<=), GREATER_EQUALS (>=),
#                     ARROW (=>), NOT_EQUALS (/=), POWER (**), BOX (<>)
# Single-char operators: PLUS, MINUS, STAR, SLASH, AMPERSAND,
#                        LESS_THAN, GREATER_THAN, EQUALS, TICK, PIPE
# Delimiters: LPAREN, RPAREN, LBRACKET, RBRACKET, SEMICOLON, COMMA, DOT, COLON
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';
our $DEFAULT_VERSION = '2008';
our @SUPPORTED_VERSIONS = qw(1987 1993 2002 2008 2019);

use File::Basename qw(dirname);
use File::Spec;
use CodingAdventures::GrammarTools;

# ============================================================================
# Grammar loading and caching
# ============================================================================
#
# Reading and parsing the grammar file on every tokenize() call would be
# wasteful. We cache the TokenGrammar object and compiled rule lists in
# package-level variables. They are populated on the first call and reused.

my %_grammar_cache;      # version -> CodingAdventures::GrammarTools::TokenGrammar
my %_rules_cache;        # version -> arrayref of { name => str, pat => qr// }
my %_skip_rules_cache;   # version -> arrayref of qr// patterns for skip definitions
my %_keyword_map_cache;  # version -> hashref mapping keyword string -> promoted token type

# --- _grammars_dir() ----------------------------------------------------------
#
# Return the absolute path to the shared `grammars/` directory in the
# monorepo, computed relative to this module file.

sub _grammars_dir {
    # __FILE__ = .../code/packages/perl/vhdl-lexer/lib/CodingAdventures/VhdlLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → vhdl-lexer/ → perl/ → packages/ → code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

sub _resolve_version {
    my ($version) = @_;
    return $DEFAULT_VERSION unless defined $version && length $version;
    return $version if grep { $_ eq $version } @SUPPORTED_VERSIONS;
    die sprintf(
        "CodingAdventures::VhdlLexer: unknown VHDL version '%s' (expected one of: %s)",
        $version,
        join(', ', @SUPPORTED_VERSIONS)
    );
}

sub default_version {
    return $DEFAULT_VERSION;
}

sub supported_versions {
    return [ @SUPPORTED_VERSIONS ];
}

# --- _grammar() ---------------------------------------------------------------
#
# Load and parse `vhdl.tokens`, caching the result.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.

sub _grammar {
    my ($version) = @_;
    $version = _resolve_version($version);

    return $_grammar_cache{$version} if exists $_grammar_cache{$version};

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'vhdl', "vhdl${version}.tokens" );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::VhdlLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::VhdlLexer: failed to parse vhdl${version}.tokens: $err"
        unless $grammar;

    $_grammar_cache{$version} = $grammar;
    return $_grammar_cache{$version};
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
#   is_regex == 1  →  wrap raw pattern string in qr/\G(?:<pattern>)/
#   is_regex == 0  →  treat as literal; use qr/\G\Q<literal>\E/
#
# The `\G` anchor forces the match to start exactly at `pos($source)`.
# Alias resolution: definitions with `-> ALIAS` emit the alias as type name.

sub _build_rules {
    my ($version) = @_;
    $version = _resolve_version($version);
    return if exists $_rules_cache{$version};    # already built

    my $grammar = _grammar($version);
    my (@rules, @skip_rules);

    # Build skip patterns
    # VHDL has two skip types: -- line comments and whitespace.
    for my $defn ( @{ $grammar->skip_definitions } ) {
        my $pat;
        if ( $defn->is_regex ) {
            # Security: reject patterns containing Perl code-execution constructs.
            # (?{ ... }) and (??{ ... }) allow arbitrary Perl code to run inside
            # a regex match. These constructs should never appear in a grammar
            # file from disk. Die early rather than silently execute injected code.
            # Fixed: 2026-04-10 security review.
            my $raw_pat = $defn->pattern;
            if ( $raw_pat =~ /\(\?\{|\(\?\?\{/ ) {
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
            if ( $raw_pat =~ /\(\?\{|\(\?\?\{/ ) {
                die "Security error: unsafe Perl regex code construct in grammar pattern '$raw_pat'
";
            }
            $pat = qr/\G(?:${\$defn->pattern})/;
        } else {
            my $lit = $defn->pattern;
            $pat = qr/\G\Q$lit\E/;
        }
        my $type = ( $defn->alias && $defn->alias ne '' )
                    ? $defn->alias
                    : $defn->name;
        push @rules, { name => $type, pat => $pat };
    }

    unless (@skip_rules) {
        push @skip_rules, qr/\G[ \t\r\n]+/;
    }

    # Build keyword lookup map from the grammar keywords section.
    my %kw_map;
    $kw_map{$_} = uc($_) for @{ $grammar->keywords };
    $_keyword_map_cache{$version} = \%kw_map;

    $_skip_rules_cache{$version} = \@skip_rules;
    $_rules_cache{$version}      = \@rules;
}

# ============================================================================
# Public API
# ============================================================================

# --- tokenize($source) --------------------------------------------------------
#
# Tokenize a VHDL source string.
#
# VHDL is case-insensitive: `vhdl.tokens` sets `case_sensitive: false`,
# which causes the GrammarTools parser to lowercase all input text before
# applying patterns. This means all returned token values are lowercase:
# "ENTITY" input → value "entity", "Entity" input → value "entity".
#
# Algorithm:
#
#   1. Ensure grammar and compiled rules are loaded (_build_rules).
#   2. Walk the source from position 0 to end.
#   3. At each position, try each skip pattern with /gc.
#      If a skip pattern matches, update line/col tracking and continue.
#   4. If no skip matched, try each token pattern in order.
#      The first match: record token, advance pos, update tracking, continue.
#   5. If nothing matched, die with a descriptive error message.
#   6. After exhausting the input, push an EOF sentinel and return.
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
    my ($class_or_self, $source, $version) = @_;

    $version = _resolve_version($version);
    _build_rules($version);

    my $rules       = $_rules_cache{$version};
    my $skip_rules  = $_skip_rules_cache{$version};
    my $keyword_map = $_keyword_map_cache{$version};

    # Normalize to lowercase for case-insensitive matching.
    # VHDL is case-insensitive (ENTITY = Entity = entity).
    $source = lc($source);

    my @tokens;
    my $line = 1;
    my $col  = 1;
    my $pos  = 0;
    my $len  = length($source);

    while ($pos < $len) {
        pos($source) = $pos;

        # ---- Try skip patterns -----------------------------------------------
        #
        # VHDL whitespace and -- comments are insignificant between tokens.
        # We advance position without emitting, but track line/col for accuracy.

        my $skipped = 0;
        for my $spat (@$skip_rules) {
            pos($source) = $pos;
            if ($source =~ /$spat/gc) {
                my $matched = $&;

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

        my $matched_tok = 0;
        for my $rule (@$rules) {
            pos($source) = $pos;
            if ($source =~ /$rule->{pat}/gc) {
                my $value = $&;

                my $tok_type = $rule->{name};
                if ($tok_type eq 'NAME' && exists $keyword_map->{$value}) {
                    $tok_type = $keyword_map->{$value};
                }
                push @tokens, {
                    type  => $tok_type,
                    value => $value,
                    line  => $line,
                    col   => $col,
                };

                $pos = pos($source);

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

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::VhdlLexer: LexerError at line %d col %d: "
              . "unexpected character '%s'",
                $line, $col, $ch
            );
        }
    }

    push @tokens, { type => 'EOF', value => '', line => $line, col => $col };

    return \@tokens;
}

1;

__END__

=head1 NAME

CodingAdventures::VhdlLexer - Grammar-driven VHDL tokenizer

=head1 SYNOPSIS

    use CodingAdventures::VhdlLexer;

    my $tokens = CodingAdventures::VhdlLexer->tokenize('entity adder is');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<vhdl.tokens> file, compiles token definitions to Perl regexes,
and tokenizes VHDL (IEEE 1076-2008) source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

VHDL is case-insensitive. The C<vhdl.tokens> grammar sets C<case_sensitive: false>,
so the lexer lowercases all input before matching. All returned token values are
lowercase: C<"ENTITY"> input gives value C<"entity">.

Whitespace and C<-- > line comments are silently consumed. The last token is
always C<EOF>.

=head1 METHODS

=head2 tokenize($source)

Tokenize a VHDL string. Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
