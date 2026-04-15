package CodingAdventures::VerilogLexer;

# ============================================================================
# CodingAdventures::VerilogLexer — Grammar-driven Verilog tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools. It reads the shared `verilog.tokens`
# grammar file, compiles the token definitions into Perl regexes, and applies
# them in priority order to tokenize Verilog (IEEE 1364-2005) source code.
#
# # What is Verilog tokenization?
# ==================================
#
# Verilog is a Hardware Description Language (HDL). Unlike software languages
# that describe sequential computations on a processor, Verilog describes
# physical structures: gates, wires, flip-flops that exist simultaneously and
# operate in parallel. A Verilog module is a blueprint for a hardware
# component with named inputs, outputs, and internal logic.
#
# Given the input:  module adder(input a, output y);
#
# The tokenizer produces:
#
#   { type => "MODULE",   value => "module", line => 1, col => 1  }
#   { type => "NAME",     value => "adder",  line => 1, col => 8  }
#   { type => "LPAREN",   value => "(",      line => 1, col => 13 }
#   { type => "INPUT",    value => "input",  line => 1, col => 14 }
#   { type => "NAME",     value => "a",      line => 1, col => 20 }
#   { type => "COMMA",    value => ",",      line => 1, col => 21 }
#   { type => "OUTPUT",   value => "output", line => 1, col => 23 }
#   { type => "NAME",     value => "y",      line => 1, col => 30 }
#   { type => "RPAREN",   value => ")",      line => 1, col => 31 }
#   { type => "SEMICOLON",value => ";",      line => 1, col => 32 }
#   { type => "EOF",      value => "",       line => 1, col => 33 }
#
# Whitespace and comments (// and /* */) are consumed silently — skip patterns
# in `verilog.tokens` match them and they are never emitted as tokens.
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar()` opens `verilog.tokens`, parses it
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
# `__FILE__` resolves to `lib/CodingAdventures/VerilogLexer.pm`.
#
# From there we climb to the repo root (`code/`) then descend into
# `grammars/verilog.tokens`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → verilog-lexer/   (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/            ← repo root
#   + /grammars/verilog.tokens
#
# # Token types
# =============
#
# Keyword tokens (NAME values promoted to their uppercase keyword type):
#   MODULE, ENDMODULE, INPUT, OUTPUT, INOUT, WIRE, REG, INTEGER, REAL,
#   SIGNED, UNSIGNED, TRI, SUPPLY0, SUPPLY1, ALWAYS, INITIAL, BEGIN, END,
#   IF, ELSE, CASE, CASEX, CASEZ, ENDCASE, DEFAULT, FOR, ASSIGN, DEFPARAM,
#   PARAMETER, LOCALPARAM, GENERATE, ENDGENERATE, GENVAR, POSEDGE, NEGEDGE,
#   OR, FUNCTION, ENDFUNCTION, TASK, ENDTASK, AND, NAND, NOR, NOT, BUF,
#   XOR, XNOR
#
# Literal/regex tokens:
#   SIZED_NUMBER — e.g. 4'b1010, 8'hFF, 32'd42
#   REAL_NUMBER  — e.g. 3.14, 1.5e-3
#   NUMBER       — plain integers like 42, 1_000
#   STRING       — double-quoted string like "hello\n"
#   SYSTEM_ID    — $display, $time, $finish
#   DIRECTIVE    — `define, `ifdef, `include
#   ESCAPED_IDENT — \my.odd.name
#   NAME         — regular identifier
#
# Three-char operators: ARITH_LEFT_SHIFT (<<<), ARITH_RIGHT_SHIFT (>>>),
#                       CASE_EQ (===), CASE_NEQ (!==)
# Two-char operators:   LOGIC_AND (&&), LOGIC_OR (||), LEFT_SHIFT (<<),
#                       RIGHT_SHIFT (>>), EQUALS_EQUALS (==), NOT_EQUALS (!=),
#                       LESS_EQUALS (<=), GREATER_EQUALS (>=), POWER (**),
#                       TRIGGER (->)
# Single-char operators: PLUS, MINUS, STAR, SLASH, PERCENT, AMP, PIPE,
#                        CARET, TILDE, BANG, LESS_THAN, GREATER_THAN,
#                        EQUALS, QUESTION, COLON
# Delimiters: LPAREN, RPAREN, LBRACKET, RBRACKET, LBRACE, RBRACE,
#             SEMICOLON, COMMA, DOT, HASH, AT
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';
our $DEFAULT_VERSION = '2005';
our @SUPPORTED_VERSIONS = qw(1995 2001 2005);

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
#
# We use File::Spec for cross-platform path construction and
# File::Basename::dirname to strip the filename component.

sub _grammars_dir {
    # __FILE__ = .../code/packages/perl/verilog-lexer/lib/CodingAdventures/VerilogLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → verilog-lexer/ → perl/ → packages/ → code/
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
        "CodingAdventures::VerilogLexer: unknown Verilog version '%s' (expected one of: %s)",
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
# Load and parse `verilog.tokens`, caching the result.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.

sub _grammar {
    my ($version) = @_;
    $version = _resolve_version($version);

    return $_grammar_cache{$version} if exists $_grammar_cache{$version};

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'verilog', "verilog${version}.tokens" );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::VerilogLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::VerilogLexer: failed to parse verilog${version}.tokens: $err"
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
#   is_regex == 1  →  treat `$defn->pattern` as a raw regex string.
#                     Wrap in qr/\G(?:<pattern>)/ to anchor at current pos.
#
#   is_regex == 0  →  treat `$defn->pattern` as a literal string.
#                     Use `\Q...\E` to disable regex metacharacters.
#                     Critical for operators like ===, !==, <<<, >>>, etc.
#
# The `\G` anchor forces the match to start exactly at `pos($source)`,
# preventing the regex engine from skipping ahead.
#
# Alias resolution: definitions with `-> ALIAS` emit the alias as type name.

sub _build_rules {
    my ($version) = @_;
    $version = _resolve_version($version);
    return if exists $_rules_cache{$version};    # already built

    my $grammar = _grammar($version);
    my (@rules, @skip_rules);

    # Build skip patterns
    # Verilog has two skip types: // line comments, /* */ block comments,
    # and plain whitespace. All are declared in the grammar's skip: section.
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
        # Emit the alias if one exists, otherwise use the definition name.
        my $type = ( $defn->alias && $defn->alias ne '' )
                    ? $defn->alias
                    : $defn->name;
        push @rules, { name => $type, pat => $pat };
    }

    # If the grammar has no skip definitions, fall back to whitespace skip.
    # (Verilog.tokens does have skip: entries, so this is a safety net.)
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
# Tokenize a Verilog source string.
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
    my ($class_or_self, $source, $version) = @_;

    $version = _resolve_version($version);
    _build_rules($version);

    my $rules       = $_rules_cache{$version};
    my $skip_rules  = $_skip_rules_cache{$version};
    my $keyword_map = $_keyword_map_cache{$version};

    my @tokens;
    my $line = 1;
    my $col  = 1;
    my $pos  = 0;
    my $len  = length($source);

    while ($pos < $len) {
        pos($source) = $pos;

        # ---- Try skip patterns -----------------------------------------------
        #
        # Verilog whitespace and comments are insignificant between tokens.
        # We advance position without emitting anything, but still track
        # line/col so that subsequent token positions are accurate.

        my $skipped = 0;
        for my $spat (@$skip_rules) {
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
        # A well-formed Verilog source should rarely reach here. We emit a
        # descriptive error including position and the offending character.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::VerilogLexer: LexerError at line %d col %d: "
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

CodingAdventures::VerilogLexer - Grammar-driven Verilog tokenizer

=head1 SYNOPSIS

    use CodingAdventures::VerilogLexer;

    my $tokens = CodingAdventures::VerilogLexer->tokenize('module adder(input a, output y);');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<verilog.tokens> file, compiles token definitions to Perl regexes,
and tokenizes Verilog (IEEE 1364-2005) source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace and comments (// and /* */) are silently consumed. The last token is
always C<EOF>.

Token types include: NAME, SIZED_NUMBER, REAL_NUMBER, NUMBER, STRING, SYSTEM_ID,
DIRECTIVE, ESCAPED_IDENT; keyword types: MODULE, ENDMODULE, INPUT, OUTPUT, INOUT,
WIRE, REG, INTEGER, REAL, SIGNED, UNSIGNED, TRI, SUPPLY0, SUPPLY1, ALWAYS, INITIAL,
BEGIN, END, IF, ELSE, CASE, CASEX, CASEZ, ENDCASE, DEFAULT, FOR, ASSIGN, DEFPARAM,
PARAMETER, LOCALPARAM, GENERATE, ENDGENERATE, GENVAR, POSEDGE, NEGEDGE, OR,
FUNCTION, ENDFUNCTION, TASK, ENDTASK, AND, NAND, NOR, NOT, BUF, XOR, XNOR;
three-char operators: ARITH_LEFT_SHIFT, ARITH_RIGHT_SHIFT, CASE_EQ, CASE_NEQ;
two-char operators: LOGIC_AND, LOGIC_OR, LEFT_SHIFT, RIGHT_SHIFT, EQUALS_EQUALS,
NOT_EQUALS, LESS_EQUALS, GREATER_EQUALS, POWER, TRIGGER; single-char operators:
PLUS, MINUS, STAR, SLASH, PERCENT, AMP, PIPE, CARET, TILDE, BANG, LESS_THAN,
GREATER_THAN, EQUALS, QUESTION, COLON; delimiters: LPAREN, RPAREN, LBRACKET,
RBRACKET, LBRACE, RBRACE, SEMICOLON, COMMA, DOT, HASH, AT.

=head1 METHODS

=head2 tokenize($source)

Tokenize a Verilog string. Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
