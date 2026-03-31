package CodingAdventures::GrammarTools;

# ============================================================================
# CodingAdventures::GrammarTools — BNF grammar utilities for lexer/parser
# ============================================================================
#
# This module parses and validates two kinds of grammar files:
#
#   1. Token grammars (.tokens files) — define the lexical tokens a lexer
#      recognizes: identifiers, numbers, operators, string literals, etc.
#
#   2. Parser grammars (.grammar files) — define the syntactic structure of a
#      language using EBNF-like rules that reference the tokens from (1).
#
# It also provides classic LL(1) parsing theory functions:
#
#   - is_nullable:      Can a symbol derive the empty string?
#   - compute_first:    What terminal tokens can begin a symbol's derivation?
#   - compute_follow:   What tokens can appear immediately after a symbol?
#   - build_parse_table: Build the LL(1) parsing table for a grammar.
#
# # Background: Why LL(1)?
# =========================
#
# LL(1) means:
#   L  = scan input Left to right
#   L  = construct Leftmost derivation
#   (1) = use 1 token of lookahead
#
# An LL(1) parser is a top-down recursive descent parser that decides which
# production rule to use by looking at just one token. This is the simplest
# kind of parser and is easy to implement by hand.
#
# For an LL(1) grammar, the "parse table" maps (non-terminal, terminal) to
# the production rule to use. If there's ever more than one rule for a
# (non-terminal, terminal) pair, the grammar is ambiguous for LL(1).
#
# # Grammar representation
# =========================
#
# A grammar is a hashref where keys are non-terminal symbols and values are
# arrayrefs of alternative productions. Each production is an arrayref of
# symbols (strings). Upper-case symbols are terminals; lower-case are
# non-terminals. The special string '' (empty string) represents epsilon (ε).
#
# Example grammar (arithmetic expressions):
#
#   {
#     'E'  => [['T', "E'"]],
#     "E'" => [['+', 'T', "E'"], ['']],
#     'T'  => [['F', "T'"]],
#     "T'" => [['*', 'F', "T'"], ['']],
#     'F'  => [['(', 'E', ')'], ['id']],
#   }
#
# In this grammar:
#   - 'E', "E'", 'T', "T'", 'F' are non-terminals (defined in the grammar)
#   - '+', '*', '(', ')', 'id' are terminals (token names)
#   - [''] is the empty (epsilon) production
#
# # FIRST and FOLLOW sets
# ========================
#
# FIRST(X) = the set of terminal symbols that can begin any string derived
#             from symbol X.
#
# FOLLOW(A) = the set of terminal symbols that can appear immediately to the
#             right of A in any sentential form. Also includes '$' (end-of-input)
#             if A can appear at the end.
#
# These are computed according to the standard textbook algorithm from
# "Compilers: Principles, Techniques, and Tools" by Aho, Lam, Sethi, Ullman
# (the "Dragon Book"), Chapter 4.
#
# # Port lineage
# ==============
#
# This is a Perl port of the Lua/Ruby implementations in the
# coding-adventures monorepo. Data structures follow the Lua conventions
# adapted to idiomatic Perl.

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# TokenDefinition — a single token rule from a .tokens file
# ============================================================================
#
# Fields:
#   name        — token name, e.g. "NUMBER" or "PLUS"
#   pattern     — the pattern string (without delimiters)
#   is_regex    — 1 if written as /regex/, 0 if "literal"
#   line_number — 1-based line number where this definition appears
#   alias       — optional type alias (e.g. STRING_DQ -> STRING)

package CodingAdventures::GrammarTools::TokenDefinition;

sub new {
    my ($class, %fields) = @_;
    return bless {
        name        => $fields{name}        // '',
        pattern     => $fields{pattern}     // '',
        is_regex    => $fields{is_regex}    // 0,
        line_number => $fields{line_number} // 0,
        alias       => $fields{alias}       // '',
    }, $class;
}

sub name        { $_[0]->{name}        }
sub pattern     { $_[0]->{pattern}     }
sub is_regex    { $_[0]->{is_regex}    }
sub line_number { $_[0]->{line_number} }
sub alias       { $_[0]->{alias}       }

# ============================================================================
# PatternGroup — a named set of token definitions for context-sensitive lexing
# ============================================================================
#
# When a pattern group is at the top of the lexer's group stack, only its
# patterns are tried during token matching. This enables context-sensitive
# lexing (e.g., different patterns inside XML tags vs. outside).

package CodingAdventures::GrammarTools::PatternGroup;

sub new {
    my ($class, $name, $definitions) = @_;
    return bless {
        name        => $name,
        definitions => $definitions // [],
    }, $class;
}

sub name        { $_[0]->{name}        }
sub definitions { $_[0]->{definitions} }

# ============================================================================
# TokenGrammar — the complete contents of a parsed .tokens file
# ============================================================================
#
# Fields:
#   definitions       — ordered arrayref of TokenDefinition objects
#   keywords          — arrayref of keyword strings
#   mode              — lexer mode, e.g. "indentation"
#   escape_mode       — escape processing mode, e.g. "none"
#   skip_definitions  — arrayref of skip-pattern TokenDefinition objects
#   error_definitions — arrayref of error-recovery TokenDefinition objects
#   reserved_keywords — arrayref of keywords that cause lex errors
#   groups            — hashref of group name => PatternGroup

package CodingAdventures::GrammarTools::TokenGrammar;

sub new {
    my ($class) = @_;
    return bless {
        definitions       => [],
        keywords          => [],
        mode              => '',
        escape_mode       => '',
        skip_definitions  => [],
        error_definitions => [],
        reserved_keywords => [],
        groups            => {},
    }, $class;
}

sub definitions       { $_[0]->{definitions}       }
sub keywords          { $_[0]->{keywords}          }
sub mode              { $_[0]->{mode}              }
sub escape_mode       { $_[0]->{escape_mode}       }
sub skip_definitions  { $_[0]->{skip_definitions}  }
sub error_definitions { $_[0]->{error_definitions} }
sub reserved_keywords { $_[0]->{reserved_keywords} }
sub groups            { $_[0]->{groups}            }

# token_names()
#
# Returns a hashref (name => 1) of all defined token names, including aliases
# and names from all pattern groups.
sub token_names {
    my ($self) = @_;
    my %names;

    my @all_defs = @{ $self->{definitions} };
    for my $group (values %{ $self->{groups} }) {
        push @all_defs, @{ $group->definitions };
    }

    for my $d (@all_defs) {
        $names{ $d->name } = 1;
        $names{ $d->alias } = 1 if $d->alias ne '';
    }
    return \%names;
}

# effective_token_names()
#
# Returns a hashref of token names as the parser will see them.
# For definitions with aliases, returns the alias; otherwise the name.
sub effective_token_names {
    my ($self) = @_;
    my %names;

    my @all_defs = @{ $self->{definitions} };
    for my $group (values %{ $self->{groups} }) {
        push @all_defs, @{ $group->definitions };
    }

    for my $d (@all_defs) {
        if ($d->alias ne '') {
            $names{ $d->alias } = 1;
        } else {
            $names{ $d->name } = 1;
        }
    }
    return \%names;
}

# ============================================================================
# GrammarTools main package — parsing and LL(1) algorithms
# ============================================================================

package CodingAdventures::GrammarTools;

# Reserved group names that cannot be used as custom group names
my %RESERVED_GROUP_NAMES = map { $_ => 1 }
    qw(default skip keywords reserved errors);

# ============================================================================
# parse_definition($pattern_part, $name_part, $line_number)
# ============================================================================
#
# Parses a single token pattern from a .tokens file line.
# Patterns come in two forms:
#   /regex/    — a Perl-compatible regular expression
#   "literal"  — an exact literal string
#
# Either form can have an optional alias suffix:
#   /regex/   -> ALIAS
#   "literal" -> ALIAS
#
# Returns: (TokenDefinition, undef) on success
#          (undef, $error_string)   on failure

sub _parse_definition {
    my ($pattern_part, $name_part, $line_number) = @_;

    my $defn = CodingAdventures::GrammarTools::TokenDefinition->new(
        name        => $name_part,
        line_number => $line_number,
    );

    if (substr($pattern_part, 0, 1) eq '/') {
        # Regex pattern — find the closing /
        my $last_slash;
        for my $i (reverse 1 .. length($pattern_part) - 1) {
            if (substr($pattern_part, $i, 1) eq '/') {
                $last_slash = $i;
                last;
            }
        }

        if (!defined $last_slash || $last_slash == 0) {
            return (undef, "Line $line_number: Unclosed regex pattern for token '$name_part'");
        }

        my $regex_body = substr($pattern_part, 1, $last_slash - 1);
        if ($regex_body eq '') {
            return (undef, "Line $line_number: Empty regex pattern for token '$name_part'");
        }

        $defn->{pattern}  = $regex_body;
        $defn->{is_regex} = 1;

        my $remainder = substr($pattern_part, $last_slash + 1);
        $remainder =~ s/^\s+|\s+$//g;

        if (substr($remainder, 0, 2) eq '->') {
            my $alias = substr($remainder, 2);
            $alias =~ s/^\s+|\s+$//g;
            return (undef, "Line $line_number: Missing alias after '->' for token '$name_part'")
                if $alias eq '';
            $defn->{alias} = $alias;
        }
        elsif ($remainder ne '') {
            return (undef, "Line $line_number: Unexpected text after pattern for token '$name_part': '$remainder'");
        }

    }
    elsif (substr($pattern_part, 0, 1) eq '"') {
        # Literal pattern — find the closing "
        my $close_pos = index($pattern_part, '"', 1);
        if ($close_pos < 0) {
            return (undef, "Line $line_number: Unclosed literal pattern for token '$name_part'");
        }

        my $literal_body = substr($pattern_part, 1, $close_pos - 1);
        if ($literal_body eq '') {
            return (undef, "Line $line_number: Empty literal pattern for token '$name_part'");
        }

        $defn->{pattern}  = $literal_body;
        $defn->{is_regex} = 0;

        my $remainder = substr($pattern_part, $close_pos + 1);
        $remainder =~ s/^\s+|\s+$//g;

        if (substr($remainder, 0, 2) eq '->') {
            my $alias = substr($remainder, 2);
            $alias =~ s/^\s+|\s+$//g;
            return (undef, "Line $line_number: Missing alias after '->' for token '$name_part'")
                if $alias eq '';
            $defn->{alias} = $alias;
        }
        elsif ($remainder ne '') {
            return (undef, "Line $line_number: Unexpected text after pattern for token '$name_part': '$remainder'");
        }

    }
    else {
        return (undef, "Line $line_number: Pattern must be /regex/ or \"literal\"");
    }

    return ($defn, undef);
}

# ============================================================================
# parse_token_grammar($source)
# ============================================================================
#
# Parses the contents of a .tokens file into a TokenGrammar object.
#
# Handles:
#   - mode: and escapes: directives
#   - keywords:, reserved:, skip:, errors: sections
#   - group NAME: sections for context-sensitive lexing
#   - Top-level token definitions (NAME = pattern)
#   - Comments (lines starting with #) and blank lines
#   - -> ALIAS syntax on definitions
#
# Parameters:
#   $source — string contents of a .tokens file
#
# Returns: (TokenGrammar, undef) on success
#          (undef, $error_string) on failure
sub parse_token_grammar {
    my ($class_or_self, $source) = @_;

    # Handle both class and object method call
    unless (defined $source) {
        $source = $class_or_self;
    }

    my $grammar = CodingAdventures::GrammarTools::TokenGrammar->new;
    my @lines = split /\n/, $source;

    my $current_section = '';   # "keywords", "reserved", "skip", "errors", or "group:NAME"

    for my $i (0 .. $#lines) {
        my $line_number = $i + 1;
        my $line = $lines[$i];
        $line =~ s/\s+$//;   # Strip trailing whitespace
        my $stripped = $line;
        $stripped =~ s/^\s+|\s+$//g;

        # Skip blank lines and comments
        next if $stripped eq '' || substr($stripped, 0, 1) eq '#';

        # mode: directive
        if (substr($stripped, 0, 5) eq 'mode:') {
            my $mode_value = substr($stripped, 5);
            $mode_value =~ s/^\s+|\s+$//g;
            return (undef, "Line $line_number: Missing value after 'mode:'")
                if $mode_value eq '';
            $grammar->{mode} = $mode_value;
            $current_section = '';
            next;
        }

        # case_sensitive: directive — silently accepted, ignored
        if ($stripped =~ /^case_sensitive\s*:/) {
            $current_section = '';
            next;
        }

        # escapes: directive
        if (substr($stripped, 0, 8) eq 'escapes:') {
            my $escape_value = substr($stripped, 8);
            $escape_value =~ s/^\s+|\s+$//g;
            return (undef, "Line $line_number: Missing value after 'escapes:'")
                if $escape_value eq '';
            $grammar->{escape_mode} = $escape_value;
            $current_section = '';
            next;
        }

        # Group header: "group NAME:"
        if ($stripped =~ /^group\s+/ && substr($stripped, -1) eq ':') {
            my $group_name = substr($stripped, 6, length($stripped) - 7);
            $group_name =~ s/^\s+|\s+$//g;
            return (undef, "Line $line_number: Missing group name after 'group'")
                if $group_name eq '';
            unless ($group_name =~ /^[a-z_][a-z0-9_]*$/) {
                return (undef, "Line $line_number: Invalid group name: '$group_name' (must be a lowercase identifier)");
            }
            if ($RESERVED_GROUP_NAMES{$group_name}) {
                return (undef, "Line $line_number: Reserved group name: '$group_name'");
            }
            if (exists $grammar->{groups}{$group_name}) {
                return (undef, "Line $line_number: Duplicate group name: '$group_name'");
            }
            $grammar->{groups}{$group_name} =
                CodingAdventures::GrammarTools::PatternGroup->new($group_name);
            $current_section = "group:$group_name";
            next;
        }

        # Section headers
        if ($stripped eq 'keywords:' || $stripped eq 'keywords :') {
            $current_section = 'keywords';
            next;
        }
        if ($stripped eq 'reserved:' || $stripped eq 'reserved :') {
            $current_section = 'reserved';
            next;
        }
        if ($stripped eq 'skip:' || $stripped eq 'skip :') {
            $current_section = 'skip';
            next;
        }
        if ($stripped eq 'errors:' || $stripped eq 'errors :') {
            $current_section = 'errors';
            next;
        }

        # Inside a section: lines must be indented
        if ($current_section ne '') {
            my $first_char = substr($line, 0, 1);
            if ($first_char eq ' ' || $first_char eq "\t") {
                if ($current_section eq 'keywords') {
                    push @{ $grammar->{keywords} }, $stripped if $stripped ne '';
                }
                elsif ($current_section eq 'reserved') {
                    push @{ $grammar->{reserved_keywords} }, $stripped if $stripped ne '';
                }
                elsif ($current_section eq 'skip') {
                    my $eq_pos = index($stripped, '=');
                    return (undef, "Line $line_number: Expected skip pattern (NAME = pattern), got: '$stripped'")
                        if $eq_pos < 0;
                    my $skip_name    = substr($stripped, 0, $eq_pos);
                    my $skip_pattern = substr($stripped, $eq_pos + 1);
                    $skip_name    =~ s/^\s+|\s+$//g;
                    $skip_pattern =~ s/^\s+|\s+$//g;
                    return (undef, "Line $line_number: Incomplete skip pattern definition: '$stripped'")
                        if $skip_name eq '' || $skip_pattern eq '';
                    my ($defn, $err) = _parse_definition($skip_pattern, $skip_name, $line_number);
                    return (undef, $err) if $err;
                    push @{ $grammar->{skip_definitions} }, $defn;
                }
                elsif ($current_section eq 'errors') {
                    my $eq_pos = index($stripped, '=');
                    return (undef, "Line $line_number: Expected error pattern (NAME = pattern), got: '$stripped'")
                        if $eq_pos < 0;
                    my $err_name    = substr($stripped, 0, $eq_pos);
                    my $err_pattern = substr($stripped, $eq_pos + 1);
                    $err_name    =~ s/^\s+|\s+$//g;
                    $err_pattern =~ s/^\s+|\s+$//g;
                    return (undef, "Line $line_number: Incomplete error pattern definition: '$stripped'")
                        if $err_name eq '' || $err_pattern eq '';
                    my ($defn, $parse_err) = _parse_definition($err_pattern, $err_name, $line_number);
                    return (undef, $parse_err) if $parse_err;
                    push @{ $grammar->{error_definitions} }, $defn;
                }
                elsif (substr($current_section, 0, 6) eq 'group:') {
                    my $group_name = substr($current_section, 6);
                    my $eq_pos = index($stripped, '=');
                    return (undef, "Line $line_number: Expected token definition in group '$group_name' (NAME = pattern), got: '$stripped'")
                        if $eq_pos < 0;
                    my $g_name    = substr($stripped, 0, $eq_pos);
                    my $g_pattern = substr($stripped, $eq_pos + 1);
                    $g_name    =~ s/^\s+|\s+$//g;
                    $g_pattern =~ s/^\s+|\s+$//g;
                    return (undef, "Line $line_number: Incomplete definition in group '$group_name': '$stripped'")
                        if $g_name eq '' || $g_pattern eq '';
                    my ($defn, $parse_err) = _parse_definition($g_pattern, $g_name, $line_number);
                    return (undef, $parse_err) if $parse_err;
                    push @{ $grammar->{groups}{$group_name}->{definitions} }, $defn;
                }
                next;
            }
            # Non-indented line — exit current section
            $current_section = '';
        }

        # Top-level token definition: NAME = pattern
        my $eq_pos = index($line, '=');
        return (undef, "Line $line_number: Expected token definition (NAME = pattern)")
            if $eq_pos < 0;

        my $name_part    = substr($line, 0, $eq_pos);
        my $pattern_part = substr($line, $eq_pos + 1);
        $name_part    =~ s/^\s+|\s+$//g;
        $pattern_part =~ s/^\s+|\s+$//g;

        return (undef, "Line $line_number: Missing token name")    if $name_part eq '';
        return (undef, "Line $line_number: Missing pattern after '='") if $pattern_part eq '';

        my ($defn, $err) = _parse_definition($pattern_part, $name_part, $line_number);
        return (undef, $err) if $err;
        push @{ $grammar->{definitions} }, $defn;
    }

    return ($grammar, undef);
}

# ============================================================================
# validate_token_grammar($grammar)
# ============================================================================
#
# Validates a parsed TokenGrammar for semantic issues.
# Returns an arrayref of issue strings (empty = all clear).
sub validate_token_grammar {
    my ($class_or_self, $grammar) = @_;

    # Handle both class method and object call
    unless (ref($grammar) && $grammar->isa('CodingAdventures::GrammarTools::TokenGrammar')) {
        $grammar = $class_or_self;
    }

    my @issues;

    # Validate regular definitions
    push @issues, @{ _validate_definitions($grammar->definitions, 'token') };

    # Validate skip definitions
    push @issues, @{ _validate_definitions($grammar->skip_definitions, 'skip pattern') };

    # Validate error definitions
    push @issues, @{ _validate_definitions($grammar->error_definitions, 'error pattern') };

    # Validate lexer mode
    if ($grammar->mode ne '' && $grammar->mode ne 'indentation') {
        push @issues, "Unknown lexer mode '" . $grammar->mode . "' (only 'indentation' is supported)";
    }

    # Validate escape mode
    if ($grammar->escape_mode ne '' && $grammar->escape_mode ne 'none') {
        push @issues, "Unknown escape mode '" . $grammar->escape_mode . "' (only 'none' is supported)";
    }

    # Validate pattern groups
    for my $group_name (sort keys %{ $grammar->groups }) {
        my $group = $grammar->groups->{$group_name};

        unless ($group_name =~ /^[a-z_][a-z0-9_]*$/) {
            push @issues, "Invalid group name '$group_name' (must be a lowercase identifier)";
        }

        if (@{ $group->definitions } == 0) {
            push @issues, "Empty pattern group '$group_name' (has no token definitions)";
        }

        push @issues, @{ _validate_definitions($group->definitions, "group '$group_name' token") };
    }

    return \@issues;
}

# _validate_definitions(\@defs, $label)
#
# Internal helper: validates a list of TokenDefinition objects.
# Checks for duplicates, empty patterns, and naming conventions.
sub _validate_definitions {
    my ($definitions, $label) = @_;
    my @issues;
    my %seen_names;

    for my $defn (@$definitions) {
        # Duplicate check
        if (exists $seen_names{ $defn->name }) {
            push @issues, sprintf(
                "Line %d: Duplicate %s name '%s' (first defined on line %d)",
                $defn->line_number, $label, $defn->name, $seen_names{ $defn->name }
            );
        }
        else {
            $seen_names{ $defn->name } = $defn->line_number;
        }

        # Empty pattern
        if ($defn->pattern eq '') {
            push @issues, sprintf("Line %d: Empty pattern for %s '%s'",
                $defn->line_number, $label, $defn->name);
        }

        # Naming convention: token names should be UPPER_CASE
        if ($defn->name ne uc($defn->name)) {
            push @issues, sprintf("Line %d: Token name '%s' should be UPPER_CASE",
                $defn->line_number, $defn->name);
        }

        # Alias convention
        if ($defn->alias ne '' && $defn->alias ne uc($defn->alias)) {
            push @issues, sprintf("Line %d: Alias '%s' for token '%s' should be UPPER_CASE",
                $defn->line_number, $defn->alias, $defn->name);
        }
    }

    return \@issues;
}

# ============================================================================
# LL(1) parsing theory functions
# ============================================================================
#
# These implement the classic algorithms from compiler theory for analyzing
# context-free grammars. The input grammar format is a hashref:
#
#   {
#     NonTerminal => [ [symbol, symbol, ...], [symbol, ...], ... ],
#   }
#
# Where:
#   - Each key is a non-terminal symbol name
#   - Each value is an arrayref of productions (alternative right-hand sides)
#   - Each production is an arrayref of symbols
#   - A production of [''] represents epsilon (the empty string)
#   - Terminals are symbols NOT present as keys in the grammar hashref

# ============================================================================
# is_nullable(\%grammar, $symbol)
# ============================================================================
#
# Returns 1 if $symbol can derive the empty string (epsilon), 0 otherwise.
#
# A symbol is nullable if:
#   1. It is a non-terminal with at least one epsilon production ([''])
#   2. It is a non-terminal where ALL symbols in some production are nullable
#
# Algorithm: iterative fixpoint — keep computing until no new nullable symbols
# are found. This handles mutual recursion (A derives B derives empty, etc.).
#
# Examples:
#   E' => [['+', 'T', "E'"], ['']]   # nullable (has epsilon production)
#   E  => [['T', "E'"]]              # NOT nullable (T is not nullable)
#
# Parameters:
#   \%grammar — the grammar hashref
#   $symbol   — the symbol to check
#
# Returns: 1 if nullable, 0 otherwise

sub is_nullable {
    my ($class_or_self, $grammar, $symbol) = @_;

    # Handle both class and object call
    unless (ref $grammar eq 'HASH') {
        ($grammar, $symbol) = ($class_or_self, $grammar);
    }

    # Compute the full set of nullable symbols using fixpoint iteration
    my %nullable = _compute_nullable_set($grammar);
    return $nullable{$symbol} ? 1 : 0;
}

# _compute_nullable_set(\%grammar)
#
# Returns a hash of all nullable symbols (symbol => 1).
# Uses iterative fixpoint algorithm.
sub _compute_nullable_set {
    my ($grammar) = @_;
    my %nullable;

    # Keep iterating until nothing changes
    my $changed = 1;
    while ($changed) {
        $changed = 0;
        for my $nt (keys %$grammar) {
            next if $nullable{$nt};   # Already known nullable
            # Check each production
            PROD: for my $prod (@{ $grammar->{$nt} }) {
                # An empty production (epsilon) makes the symbol nullable
                if (@$prod == 1 && $prod->[0] eq '') {
                    $nullable{$nt} = 1;
                    $changed = 1;
                    last PROD;
                }
                # A production where ALL symbols are nullable also makes it nullable
                my $all_nullable = 1;
                for my $sym (@$prod) {
                    unless ($nullable{$sym}) {
                        $all_nullable = 0;
                        last;
                    }
                }
                if ($all_nullable && @$prod > 0) {
                    $nullable{$nt} = 1;
                    $changed = 1;
                    last PROD;
                }
            }
        }
    }

    return %nullable;
}

# ============================================================================
# compute_first(\%grammar, $symbol)
# ============================================================================
#
# Computes FIRST($symbol) — the set of terminal tokens that can begin
# any string derived from $symbol.
#
# Rules (from the Dragon Book, Section 4.4.2):
#
#   1. If $symbol is a terminal (not a key in %grammar):
#      FIRST($symbol) = { $symbol }
#
#   2. If $symbol is a non-terminal with production X -> Y1 Y2 ... Yk:
#      - Add FIRST(Y1) (minus epsilon) to FIRST(X)
#      - If Y1 is nullable, also add FIRST(Y2) (minus epsilon), etc.
#      - If ALL of Y1..Yk are nullable, add epsilon to FIRST(X)
#
#   3. If X -> epsilon (empty production):
#      Add epsilon ('') to FIRST(X)
#
# This function handles indirect left recursion by memoization.
#
# Parameters:
#   \%grammar — the grammar hashref
#   $symbol   — the symbol to compute FIRST for
#
# Returns: a hashref { terminal => 1, ... } representing the FIRST set

sub compute_first {
    my ($class_or_self, $grammar, $symbol) = @_;

    # Handle both class and object call
    unless (ref $grammar eq 'HASH') {
        ($grammar, $symbol) = ($class_or_self, $grammar);
    }

    my %memo;    # Memoization cache: symbol => { first set }
    my %in_progress;  # Cycle detection

    my $first_of;
    $first_of = sub {
        my ($sym) = @_;

        # Return memoized result if available
        return $memo{$sym} if exists $memo{$sym};

        # If not in grammar, it's a terminal: FIRST = { sym }
        unless (exists $grammar->{$sym}) {
            $memo{$sym} = { $sym => 1 };
            return $memo{$sym};
        }

        # Guard against cycles (left recursion)
        return {} if $in_progress{$sym};
        $in_progress{$sym} = 1;

        my %result;
        my %nullable = _compute_nullable_set($grammar);

        for my $prod (@{ $grammar->{$sym} }) {
            # Epsilon production
            if (@$prod == 1 && $prod->[0] eq '') {
                $result{''} = 1;
                next;
            }

            # For production Y1 Y2 ... Yk:
            # Add FIRST(Yi) - {epsilon} for each Yi, stopping when Yi is not nullable
            my $all_nullable = 1;
            for my $y (@$prod) {
                my $first_y = $first_of->($y);
                # Add all non-epsilon terminals from FIRST(Y)
                for my $t (keys %$first_y) {
                    $result{$t} = 1 unless $t eq '';
                }
                # If Y is not nullable, stop here
                unless ($nullable{$y} || (exists $first_y->{''} && $first_y->{''})) {
                    $all_nullable = 0;
                    last;
                }
            }
            # If all symbols in the production are nullable, add epsilon
            $result{''} = 1 if $all_nullable;
        }

        delete $in_progress{$sym};
        $memo{$sym} = \%result;
        return \%result;
    };

    return $first_of->($symbol);
}

# ============================================================================
# compute_follow(\%grammar, $start, \%first_sets)
# ============================================================================
#
# Computes FOLLOW($symbol) for all non-terminals in the grammar.
#
# FOLLOW(A) = the set of terminal symbols that can appear immediately to
# the right of A in some sentential form of the grammar.
#
# Rules (from the Dragon Book, Section 4.4.2):
#
#   1. Put '$' (end-of-input marker) in FOLLOW(start symbol).
#
#   2. For each production A -> alpha B beta:
#      - Add FIRST(beta) - {epsilon} to FOLLOW(B)
#      - If beta is nullable (or empty), add FOLLOW(A) to FOLLOW(B)
#
# We iterate until fixpoint (no more changes).
#
# Parameters:
#   \%grammar    — the grammar hashref
#   $start       — the start symbol of the grammar
#   \%first_sets — optional precomputed FIRST sets (will compute if omitted)
#
# Returns: a hashref { NonTerminal => { terminal => 1, ... }, ... }

sub compute_follow {
    my ($class_or_self, $grammar, $start, $first_sets) = @_;

    # Handle both class and object call
    unless (ref $grammar eq 'HASH') {
        ($grammar, $start, $first_sets) = ($class_or_self, $grammar, $start);
    }

    # Compute nullable set
    my %nullable = _compute_nullable_set($grammar);

    # Initialize FOLLOW sets: start symbol gets '$' (end-of-input)
    my %follow;
    for my $nt (keys %$grammar) {
        $follow{$nt} = {};
    }
    $follow{$start}{'$'} = 1;

    # Iterate until fixpoint
    my $changed = 1;
    while ($changed) {
        $changed = 0;

        for my $lhs (keys %$grammar) {
            for my $prod (@{ $grammar->{$lhs} }) {
                next if @$prod == 1 && $prod->[0] eq '';  # Skip epsilon

                for my $i (0 .. $#$prod) {
                    my $b = $prod->[$i];
                    next unless exists $grammar->{$b};  # Only process non-terminals

                    # Compute FIRST of the remaining tail: prod[i+1 .. end]
                    my @tail = @{$prod}[$i+1 .. $#$prod];

                    # Add FIRST(tail) - epsilon to FOLLOW(B)
                    my $tail_nullable = 1;
                    for my $sym (@tail) {
                        my $first_sym = compute_first($grammar, $sym);
                        for my $t (keys %$first_sym) {
                            next if $t eq '';
                            unless (exists $follow{$b}{$t}) {
                                $follow{$b}{$t} = 1;
                                $changed = 1;
                            }
                        }
                        # If this symbol is not nullable, stop
                        unless ($nullable{$sym} || exists $first_sym->{''}) {
                            $tail_nullable = 0;
                            last;
                        }
                    }

                    # If tail is nullable (or empty), add FOLLOW(A) to FOLLOW(B)
                    if ($tail_nullable) {
                        for my $t (keys %{ $follow{$lhs} }) {
                            unless (exists $follow{$b}{$t}) {
                                $follow{$b}{$t} = 1;
                                $changed = 1;
                            }
                        }
                    }
                }
            }
        }
    }

    return \%follow;
}

# ============================================================================
# build_parse_table(\%grammar, $start)
# ============================================================================
#
# Builds the LL(1) parsing table for the given grammar.
#
# The parse table maps (NonTerminal, Terminal) => production rule, telling
# the parser "when you're trying to derive $NT and you see token $T, use
# this production."
#
# Construction algorithm (Dragon Book, Algorithm 4.31):
#
#   For each production A -> alpha:
#     1. For each terminal `a` in FIRST(alpha):
#        Add A -> alpha to M[A, a]
#     2. If epsilon is in FIRST(alpha):
#        For each terminal `b` in FOLLOW(A):
#          Add A -> alpha to M[A, b]
#
# The result is a hashref of hashrefs:
#   { NonTerminal => { terminal => \@production, ... }, ... }
#
# If any cell has more than one production, the grammar is NOT LL(1).
# We return all productions (as an arrayref) in that cell.
#
# Parameters:
#   \%grammar — the grammar hashref
#   $start    — the start symbol
#
# Returns: hashref { NT => { terminal => [\@production, ...], ... }, ... }

sub build_parse_table {
    my ($class_or_self, $grammar, $start) = @_;

    # Handle both class and object call
    unless (ref $grammar eq 'HASH') {
        ($grammar, $start) = ($class_or_self, $grammar);
    }

    # Compute FOLLOW sets (which internally uses FIRST)
    my $follow = compute_follow($grammar, $start);

    my %table;
    for my $nt (keys %$grammar) {
        $table{$nt} = {};
    }

    for my $nt (keys %$grammar) {
        for my $prod (@{ $grammar->{$nt} }) {
            # Compute FIRST of this production (treat it as a sequence)
            my %first_prod;

            if (@$prod == 1 && $prod->[0] eq '') {
                # Epsilon production: FIRST = { '' }
                $first_prod{''} = 1;
            }
            else {
                # FIRST of a sequence: add FIRST(sym) - epsilon while nullable
                my $all_nullable = 1;
                for my $sym (@$prod) {
                    my $fs = compute_first($grammar, $sym);
                    for my $t (keys %$fs) {
                        $first_prod{$t} = 1 unless $t eq '';
                    }
                    # If sym is not nullable, stop
                    my %nullable_set = _compute_nullable_set($grammar);
                    unless ($nullable_set{$sym} || exists $fs->{''}) {
                        $all_nullable = 0;
                        last;
                    }
                }
                $first_prod{''} = 1 if $all_nullable;
            }

            # Rule 1: For each terminal in FIRST(prod), add to table
            for my $terminal (keys %first_prod) {
                next if $terminal eq '';
                push @{ $table{$nt}{$terminal} }, $prod;
            }

            # Rule 2: If epsilon in FIRST(prod), use FOLLOW(NT)
            if (exists $first_prod{''}) {
                for my $terminal (keys %{ $follow->{$nt} }) {
                    push @{ $table{$nt}{$terminal} }, $prod;
                }
            }
        }
    }

    return \%table;
}

# ============================================================================
# is_ll1(\%grammar, $start)
# ============================================================================
#
# Returns 1 if the grammar is LL(1) (no cell in parse table has conflicts),
# 0 otherwise.
#
# A grammar is LL(1) if every cell in the parse table has at most one
# production. Conflicts arise from:
#   - First/First conflicts: two alternatives can start with the same token
#   - First/Follow conflicts: an alternative can derive epsilon AND its
#     follow set overlaps with another alternative's first set

sub is_ll1 {
    my ($class_or_self, $grammar, $start) = @_;

    unless (ref $grammar eq 'HASH') {
        ($grammar, $start) = ($class_or_self, $grammar);
    }

    my $table = build_parse_table($grammar, $start);

    for my $nt (keys %$table) {
        for my $terminal (keys %{ $table->{$nt} }) {
            return 0 if @{ $table->{$nt}{$terminal} } > 1;
        }
    }

    return 1;
}

1;

__END__

=head1 NAME

CodingAdventures::GrammarTools - BNF grammar utilities for lexer/parser construction

=head1 SYNOPSIS

    use CodingAdventures::GrammarTools;

    # Parse a .tokens file
    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($source);
    my $issues = CodingAdventures::GrammarTools->validate_token_grammar($grammar);

    # LL(1) analysis on a context-free grammar
    my $cfg = {
        "E"  => [["T", "E'"]],
        "E'" => [["+", "T", "E'"], [""]],
        "T"  => [["F", "T'"]],
        "T'" => [["*", "F", "T'"], [""]],
        "F"  => [["(", "E", ")"], ["id"]],
    };

    my $nullable = CodingAdventures::GrammarTools->is_nullable($cfg, "E'");  # 1
    my $first_E  = CodingAdventures::GrammarTools->compute_first($cfg, "E"); # { id=>1, '('=>1 }
    my $follow   = CodingAdventures::GrammarTools->compute_follow($cfg, "E");
    my $table    = CodingAdventures::GrammarTools->build_parse_table($cfg, "E");
    my $ok       = CodingAdventures::GrammarTools->is_ll1($cfg, "E");  # 1

=head1 DESCRIPTION

Grammar tools for lexer and parser construction. Provides:

=over 4

=item * Token grammar parsing and validation (.tokens files)

=item * LL(1) grammar analysis: nullable check, FIRST sets, FOLLOW sets, parse table

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
