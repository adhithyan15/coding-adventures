package CodingAdventures::StarlarkLexer;

# ============================================================================
# CodingAdventures::StarlarkLexer — Grammar-driven Starlark tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools. It reads the shared `starlark.tokens`
# grammar file, compiles the token definitions into Perl regexes, and applies
# them in priority order to tokenize Starlark source code.
#
# # What is Starlark?
# ====================
#
# Starlark is a deterministic subset of Python designed for use as a
# configuration language. It is used in Bazel BUILD files. Key differences
# from Python:
#   - No while loops, classes, try/except/raise, global/nonlocal
#   - Significant indentation (INDENT/DEDENT/NEWLINE tokens)
#   - Certain Python keywords are reserved but disallowed
#   - Reserved words cause a tokenization error when used as identifiers
#
# # What is Starlark tokenization?
# ==================================
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
#   { type => "NEWLINE",value => "\n",   line => 1, col => 12 }  (if newline follows)
#   { type => "EOF",    value => "",     line => 1, col => 12 }
#
# Whitespace between tokens is consumed silently. Comments (# to end of line)
# are also consumed silently.
#
# # Indentation mode
# ===================
#
# starlark.tokens declares `mode: indentation`. This module implements
# indentation tracking manually (mirroring what the Lua GrammarLexer does
# automatically):
#
#   1. Maintain an indentation stack (starts at [0]).
#   2. At the start of each logical line, measure leading spaces.
#   3. If indent > top of stack: emit INDENT, push new level.
#   4. If indent < top of stack: emit one DEDENT per stack level popped
#      until the stack matches the new level.
#   5. If indent == top of stack: no structural token.
#   6. Emit NEWLINE at each logical line boundary.
#   7. Track bracket depth: inside (, [, { — suppress INDENT/DEDENT/NEWLINE.
#   8. Reject tab characters in leading whitespace.
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar()` opens `starlark.tokens`, parses it
#    with `CodingAdventures::GrammarTools::parse_token_grammar`, and caches
#    the result for the lifetime of the process.
#
# 2. **Pattern compilation** — `_build_rules()` converts every TokenDefinition
#    into a `{ name => str, pat => qr/\G.../ }` hashref. Skip patterns become
#    `qr/\G.../` entries in a separate list.
#
# 3. **Tokenization** — `tokenize()` processes the source line by line,
#    emitting NEWLINE/INDENT/DEDENT at logical line boundaries and calling
#    the token scanner on each line's non-whitespace content.
#
# # Path navigation
# =================
#
# `__FILE__` resolves to `lib/CodingAdventures/StarlarkLexer.pm`.
# `dirname(__FILE__)` → `lib/CodingAdventures`
#
# From there we climb to the repo root (`code/`) then descend into
# `grammars/starlark.tokens`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → starlark-lexer/   (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/             ← repo root
#   + /grammars/starlark.tokens
#
# # Token types
# =============
#
# NAME        — identifiers; promoted to keyword type if in keywords section
# INT         — integer literals: decimal, hex (0xFF), octal (0o77) via -> INT aliases
# FLOAT       — floating-point literals
# STRING      — all string variants via -> STRING aliases
#
# Keyword tokens (promoted from NAME):
#   AND, BREAK, CONTINUE, DEF, ELIF, ELSE, FOR, IF, IN, LAMBDA,
#   LOAD, NOT, OR, PASS, RETURN, TRUE, FALSE, NONE
#
# Three-char operators:
#   DOUBLE_STAR_EQUALS, LEFT_SHIFT_EQUALS, RIGHT_SHIFT_EQUALS, FLOOR_DIV_EQUALS
#
# Two-char operators:
#   DOUBLE_STAR, FLOOR_DIV, LEFT_SHIFT, RIGHT_SHIFT,
#   EQUALS_EQUALS, NOT_EQUALS, LESS_EQUALS, GREATER_EQUALS,
#   PLUS_EQUALS, MINUS_EQUALS, STAR_EQUALS, SLASH_EQUALS,
#   PERCENT_EQUALS, AMP_EQUALS, PIPE_EQUALS, CARET_EQUALS
#
# Single-char operators:
#   PLUS, MINUS, STAR, SLASH, PERCENT, EQUALS, LESS_THAN, GREATER_THAN,
#   AMP, PIPE, CARET, TILDE
#
# Delimiters:
#   LPAREN, RPAREN, LBRACKET, RBRACKET, LBRACE, RBRACE,
#   COMMA, COLON, SEMICOLON, DOT
#
# Indentation tokens:
#   INDENT, DEDENT, NEWLINE
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
# wasteful. We cache the TokenGrammar object and compiled rule lists in
# package-level variables. They are populated on the first call and reused.

my $_grammar;      # CodingAdventures::GrammarTools::TokenGrammar
my $_rules;        # arrayref of { name => str, pat => qr// }
my $_skip_rules;   # arrayref of qr// patterns for skip definitions
my $_keyword_map;  # hashref mapping keyword string → promoted token type

# --- _grammars_dir() ----------------------------------------------------------
#
# Return the absolute path to the shared `grammars/` directory in the
# monorepo, computed relative to this module file.

sub _grammars_dir {
    # __FILE__ = .../code/packages/perl/starlark-lexer/lib/CodingAdventures/StarlarkLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → starlark-lexer/ → perl/ → packages/ → code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _grammar() ---------------------------------------------------------------
#
# Load and parse `starlark.tokens`, caching the result.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.

sub _grammar {
    return $_grammar if $_grammar;

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'starlark.tokens' );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::StarlarkLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::StarlarkLexer: failed to parse starlark.tokens: $err"
        unless $grammar;

    $_grammar = $grammar;
    return $_grammar;
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
#
# The `\G` anchor forces the match to start exactly at `pos($source)`,
# preventing the regex engine from skipping ahead.
#
# Alias resolution: definitions with `-> ALIAS` emit the alias as type name.

sub _build_rules {
    return if $_rules;    # already built

    my $grammar = _grammar();
    my (@rules, @skip_rules);

    # Build skip patterns — these consume text without emitting tokens.
    # For Starlark, skip patterns include COMMENT (# to end of line) and
    # WHITESPACE (spaces and tabs between tokens on the same line).
    # Leading whitespace is handled by the indentation algorithm, not here.
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

    # If no skip rules exist in the grammar, add a default whitespace skip.
    unless (@skip_rules) {
        push @skip_rules, qr/\G[ \t]+/;
    }

    # Build token patterns — these consume text and emit a token.
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

    # Build keyword lookup map from the grammar keywords section.
    my %kw_map;
    $kw_map{$_} = uc($_) for @{ $grammar->keywords };
    $_keyword_map = \%kw_map;

    $_skip_rules = \@skip_rules;
    $_rules      = \@rules;
}

# ============================================================================
# Public API
# ============================================================================

# --- tokenize($source) --------------------------------------------------------
#
# Tokenize a Starlark source string.
#
# Starlark uses significant indentation, so this implementation processes the
# source in two phases:
#
#   Phase 1 — Line splitting
#     Split the source on newlines. Each physical line is a unit for
#     indentation measurement. Logical line continuation (\ at end of line)
#     is not handled here (Starlark does not support \ continuation).
#
#   Phase 2 — Indentation tracking + token scanning per line
#     For each line:
#       a. Measure leading spaces (tabs are an error in leading whitespace).
#       b. If bracket_depth > 0: skip indentation logic entirely.
#       c. If bracket_depth == 0 AND the line is not blank (after stripping
#          comments and whitespace):
#          - Compare indent level to stack top.
#          - Emit INDENT/DEDENT as needed.
#          - Emit NEWLINE at end of line.
#       d. Scan non-indentation content using the compiled token rules.
#       e. Track ( [ { → bracket_depth++, ) ] } → bracket_depth--.
#
# Return value:
#
#   An arrayref of hashrefs, each with keys: type, value, line, col.
#   The last element always has type 'EOF'.

sub tokenize {
    my ($class_or_self, $source) = @_;

    _build_rules();

    my @tokens;
    my $line_num       = 1;
    my $col            = 1;
    my @indent_stack   = (0);    # Stack of indentation levels; starts at [0]
    my $bracket_depth  = 0;      # Depth inside (), [], {}; suppresses INDENT/DEDENT

    # The open bracket token types that increase bracket depth.
    my %open_brackets  = ( LPAREN => 1, LBRACKET => 1, LBRACE => 1 );
    my %close_brackets = ( RPAREN => 1, RBRACKET => 1, RBRACE => 1 );

    # Ensure the source ends with a newline so that the final DEDENT(s)
    # are emitted. If source is empty or already ends with \n, this is a no-op.
    $source .= "\n" unless $source =~ /\n\z/;

    # Split into lines, preserving the newline at the end of each line.
    # We use a lookbehind so that each line retains its trailing \n.
    my @lines = split /(?<=\n)/, $source;

    for my $raw_line (@lines) {
        # Measure leading whitespace (for indentation tracking).
        # Leading tabs are disallowed in Starlark.
        my ($leading) = $raw_line =~ /^([ \t]*)/;
        if ($leading =~ /\t/) {
            die sprintf(
                "CodingAdventures::StarlarkLexer: SyntaxError at line %d: "
              . "tab character in leading whitespace",
                $line_num
            );
        }
        my $indent_level = length($leading);

        # Strip the trailing newline for token scanning purposes.
        # We track it separately so we can emit NEWLINE at the right position.
        (my $scan_line = $raw_line) =~ s/\n\z//;

        # Determine whether the logical line is blank (empty or comment-only).
        # A blank line does not trigger indentation changes or NEWLINE emission.
        my $stripped = $scan_line;
        $stripped =~ s/^\s+//;
        $stripped =~ s/#.*//;
        $stripped =~ s/\s+\z//;
        my $is_blank = ($stripped eq '');

        # ---- Indentation logic (only when at top level, not in brackets) -----
        #
        # When bracket_depth > 0, we are inside an implicit continuation
        # (a multi-line expression inside (), [], {}) — indentation is
        # insignificant there, just as in Python/Starlark.

        if ( $bracket_depth == 0 && !$is_blank ) {
            # Compare to the current indentation level.
            my $current_indent = $indent_stack[-1];

            if ( $indent_level > $current_indent ) {
                # Indentation increased: emit INDENT and push new level.
                push @tokens, {
                    type  => 'INDENT',
                    value => ' ' x $indent_level,
                    line  => $line_num,
                    col   => 1,
                };
                push @indent_stack, $indent_level;

            } elsif ( $indent_level < $current_indent ) {
                # Indentation decreased: emit DEDENT(s) until we match.
                while ( @indent_stack && $indent_stack[-1] > $indent_level ) {
                    pop @indent_stack;
                    push @tokens, {
                        type  => 'DEDENT',
                        value => '',
                        line  => $line_num,
                        col   => 1,
                    };
                }
                # Verify that the new level matches an existing level exactly.
                if ( !@indent_stack || $indent_stack[-1] != $indent_level ) {
                    die sprintf(
                        "CodingAdventures::StarlarkLexer: IndentationError at line %d: "
                      . "unindent does not match any outer indentation level",
                        $line_num
                    );
                }
            }
            # If indent_level == $current_indent: no structural token needed.
        }

        # ---- Token scanning --------------------------------------------------
        #
        # Walk the scan_line from the indentation position forward, trying
        # skip patterns then token patterns in definition order.

        my $pos = $indent_level;    # Start after the leading whitespace
        my $line_len = length($scan_line);
        $col = $indent_level + 1;   # 1-based column, right after the indent

        while ( $pos < $line_len ) {
            pos($scan_line) = $pos;

            # ---- Try skip patterns -------------------------------------------
            my $skipped = 0;
            for my $spat (@$_skip_rules) {
                pos($scan_line) = $pos;
                if ( $scan_line =~ /$spat/gc ) {
                    my $matched = $&;
                    $col += length($matched);
                    $pos = pos($scan_line);
                    $skipped = 1;
                    last;
                }
            }
            next if $skipped;

            # ---- Try token patterns ------------------------------------------
            my $matched_tok = 0;
            for my $rule (@$_rules) {
                pos($scan_line) = $pos;
                if ( $scan_line =~ /$rule->{pat}/gc ) {
                    my $value = $&;

                    my $tok_type = $rule->{name};
                    if ($tok_type eq 'NAME' && exists $_keyword_map->{$value}) {
                        $tok_type = $_keyword_map->{$value};
                    }
                    push @tokens, {
                        type  => $tok_type,
                        value => $value,
                        line  => $line_num,
                        col   => $col,
                    };

                    # Track bracket depth for implicit line continuation.
                    if    ( $open_brackets{ $rule->{name} }  ) { $bracket_depth++ }
                    elsif ( $close_brackets{ $rule->{name} } ) { $bracket_depth-- }

                    $col += length($value);
                    $pos = pos($scan_line);
                    $matched_tok = 1;
                    last;
                }
            }

            # ---- No match — unexpected character ----------------------------
            unless ($matched_tok) {
                my $ch = substr($scan_line, $pos, 1);
                die sprintf(
                    "CodingAdventures::StarlarkLexer: LexerError at line %d col %d: "
                  . "unexpected character '%s'",
                    $line_num, $col, $ch
                );
            }
        }

        # ---- NEWLINE at end of logical line ----------------------------------
        #
        # Emit NEWLINE at the end of each non-blank top-level line.
        # Inside brackets, newlines are part of the implicit continuation and
        # are not emitted as tokens.

        if ( $bracket_depth == 0 && !$is_blank ) {
            push @tokens, {
                type  => 'NEWLINE',
                value => "\n",
                line  => $line_num,
                col   => $col,
            };
        }

        $line_num++;
        $col = 1;
    }

    # ---- Emit remaining DEDENT tokens ----------------------------------------
    #
    # At end of file, pop any remaining indentation levels off the stack.
    # Each level that is not 0 gets a DEDENT token.

    while ( @indent_stack && $indent_stack[-1] > 0 ) {
        pop @indent_stack;
        push @tokens, {
            type  => 'DEDENT',
            value => '',
            line  => $line_num,
            col   => 1,
        };
    }

    # Sentinel EOF token — always present as the last element.
    # $line_num was incremented after processing the last line, so subtract 1.
    push @tokens, { type => 'EOF', value => '', line => $line_num - 1, col => $col // 1 };

    return \@tokens;
}

1;

__END__

=head1 NAME

CodingAdventures::StarlarkLexer - Grammar-driven Starlark tokenizer

=head1 SYNOPSIS

    use CodingAdventures::StarlarkLexer;

    my $tokens = CodingAdventures::StarlarkLexer->tokenize('def foo(x):');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<starlark.tokens> file, compiles token definitions to Perl regexes,
and tokenizes Starlark source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Starlark uses significant indentation (C<mode: indentation> in the grammar).
This module implements the indentation algorithm producing INDENT, DEDENT, and
NEWLINE tokens. INDENT/DEDENT/NEWLINE are suppressed inside (), [], {}.

Whitespace and C<#> comments are silently consumed. The last token is always C<EOF>.

=head1 METHODS

=head2 tokenize($source)

Tokenize a Starlark string. Returns an arrayref of token hashrefs.
Dies on unexpected input, indentation errors, or tab characters in leading
whitespace with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
