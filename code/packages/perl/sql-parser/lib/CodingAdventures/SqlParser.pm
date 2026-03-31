package CodingAdventures::SqlParser;

# ============================================================================
# CodingAdventures::SqlParser — Hand-written recursive-descent SQL parser
# ============================================================================
#
# This module parses a subset of ANSI SQL into an Abstract Syntax Tree (AST).
# It is hand-written using the recursive-descent technique — there is no
# grammar file; the grammar is encoded directly as Perl subroutines, one per
# grammar rule.
#
# # What is recursive descent?
# =============================
#
# Each grammar rule is represented by one Perl method.  When a rule references
# another rule, it calls that method.  This mirrors the grammar directly:
#
#   SQL grammar rule:        Perl method:
#   ─────────────────────    ────────────────────────────
#   statement = …            sub _parse_statement
#   select_stmt = …          sub _parse_select_stmt
#   where_clause = …         sub _parse_where_clause
#   expr = or_expr           sub _parse_expr → _parse_or_expr
#
# The parser maintains a position pointer (`_pos`) into the flat token array
# produced by CodingAdventures::SqlLexer.  Helper methods `_peek`, `_advance`,
# `_expect`, and `_match` navigate the token stream.
#
# # Supported SQL
# ===============
#
# Statement types:
#
#   SELECT [DISTINCT | ALL] col1, col2 FROM table
#          [JOIN …] [WHERE …] [GROUP BY …] [HAVING …]
#          [ORDER BY …] [LIMIT n [OFFSET m]]
#
#   INSERT INTO table [(col, …)] VALUES (expr, …) [, (expr, …)]
#
#   UPDATE table SET col = expr [, col = expr] [WHERE expr]
#
#   DELETE FROM table [WHERE expr]
#
# Multiple statements may be separated by semicolons.
#
# # Expressions
# ==============
#
# Full operator precedence (lowest to highest):
#
#   or_expr      — OR
#   and_expr     — AND
#   not_expr     — NOT
#   comparison   — = != < > <= >= BETWEEN IN LIKE IS NULL
#   additive     — + -
#   multiplicative — * / %
#   unary        — unary -
#   primary      — literal, column_ref, function_call, (expr)
#
# # AST node format
# =================
#
# Nodes are CodingAdventures::SqlParser::ASTNode instances:
#
#   Inner:  rule_name => string, children => [$node, ...]
#   Leaf:   rule_name => "token", is_leaf => 1, token => $tok_hashref
#
# # Path navigation
# =================
#
# This module does NOT need to read grammar files — the grammar is embedded
# in code.  No filesystem capability is required.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::SqlLexer;
use CodingAdventures::SqlParser::ASTNode;

# ============================================================================
# Constructor
# ============================================================================

# --- new($source) -------------------------------------------------------------
#
# Tokenize `$source` with SqlLexer and return a ready-to-parse parser.

sub new {
    my ($class, $source) = @_;
    my $tokens = CodingAdventures::SqlLexer->tokenize($source);
    return bless {
        _tokens => $tokens,
        _pos    => 0,
    }, $class;
}

# ============================================================================
# Token helpers
# ============================================================================

# Peek at the current token without consuming it.
sub _peek {
    my ($self) = @_;
    return $self->{_tokens}[ $self->{_pos} ]
        // { type => 'EOF', value => '', line => 0, col => 0 };
}

# Consume and return the current token.
sub _advance {
    my ($self) = @_;
    my $tok = $self->_peek();
    $self->{_pos}++ unless $tok->{type} eq 'EOF';
    return $tok;
}

# Expect a token of a specific type; die with a helpful message if mismatch.
# Returns the consumed token.
sub _expect {
    my ($self, $type) = @_;
    my $tok = $self->_peek();
    unless ($tok->{type} eq $type) {
        die sprintf(
            "CodingAdventures::SqlParser: parse error at line %d col %d: "
          . "expected %s but got %s ('%s')\n",
            $tok->{line}, $tok->{col}, $type, $tok->{type}, $tok->{value}
        );
    }
    return $self->_advance();
}

# Expect a token of type AND value; die if mismatch.
sub _expect_value {
    my ($self, $type, $value) = @_;
    my $tok = $self->_peek();
    unless ($tok->{type} eq $type && uc($tok->{value}) eq uc($value)) {
        die sprintf(
            "CodingAdventures::SqlParser: parse error at line %d col %d: "
          . "expected %s('%s') but got %s('%s')\n",
            $tok->{line}, $tok->{col},
            $type, $value,
            $tok->{type}, $tok->{value}
        );
    }
    return $self->_advance();
}

# Return 1 if current token matches type (and optionally value).
sub _check {
    my ($self, $type, $value) = @_;
    my $tok = $self->_peek();
    return 0 unless $tok->{type} eq $type;
    return 1 unless defined $value;
    return uc($tok->{value}) eq uc($value);
}

# Consume and return the current token if it matches; otherwise return undef.
sub _match {
    my ($self, $type, $value) = @_;
    return $self->_advance() if $self->_check($type, $value);
    return undef;
}

# Wrap a token as a leaf ASTNode.
sub _leaf {
    my ($self, $tok) = @_;
    return CodingAdventures::SqlParser::ASTNode->new_leaf($tok);
}

# Create an inner ASTNode.
sub _node {
    my ($self, $rule_name, @children) = @_;
    return CodingAdventures::SqlParser::ASTNode->new($rule_name, \@children);
}

# ============================================================================
# Public API
# ============================================================================

# --- parse() ------------------------------------------------------------------
#
# Parse the tokenized source and return the root AST node (rule_name "program").
# Dies on parse error.

sub parse {
    my ($self) = @_;
    return $self->_parse_program();
}

# ============================================================================
# Grammar rules — each method parses one grammar rule
# ============================================================================

# program = statement { ";" statement } [ ";" ] ;
sub _parse_program {
    my ($self) = @_;
    my @children;

    # Must have at least one statement (empty input is an error).
    if ($self->_check('EOF')) {
        die "CodingAdventures::SqlParser: empty input — expected a SQL statement\n";
    }

    push @children, $self->_parse_statement();

    while ($self->_match('SEMICOLON')) {
        push @children, $self->_leaf({ type => 'SEMICOLON', value => ';', line => 0, col => 0 });
        last if $self->_check('EOF');
        push @children, $self->_parse_statement();
    }

    unless ($self->_check('EOF')) {
        my $tok = $self->_peek();
        die sprintf(
            "CodingAdventures::SqlParser: unexpected token '%s' (type %s) at line %d col %d\n",
            $tok->{value}, $tok->{type}, $tok->{line}, $tok->{col}
        );
    }

    return $self->_node('program', @children);
}

# statement = select_stmt | insert_stmt | update_stmt | delete_stmt ;
sub _parse_statement {
    my ($self) = @_;
    my $tok = $self->_peek();
    my $type = $tok->{type};

    if ($type eq 'SELECT') {
        return $self->_node('statement', $self->_parse_select_stmt());
    } elsif ($type eq 'INSERT') {
        return $self->_node('statement', $self->_parse_insert_stmt());
    } elsif ($type eq 'UPDATE') {
        return $self->_node('statement', $self->_parse_update_stmt());
    } elsif ($type eq 'DELETE') {
        return $self->_node('statement', $self->_parse_delete_stmt());
    } else {
        die sprintf(
            "CodingAdventures::SqlParser: expected SELECT/INSERT/UPDATE/DELETE "
          . "at line %d col %d, got %s('%s')\n",
            $tok->{line}, $tok->{col}, $tok->{type}, $tok->{value}
        );
    }
}

# ── SELECT ───────────────────────────────────────────────────────────────────
#
# select_stmt = "SELECT" [ "DISTINCT" | "ALL" ] select_list
#               "FROM" table_ref { join_clause }
#               [ where_clause ] [ group_clause ] [ having_clause ]
#               [ order_clause ] [ limit_clause ] ;

sub _parse_select_stmt {
    my ($self) = @_;
    my @ch;

    push @ch, $self->_leaf($self->_expect('SELECT'));

    # Optional DISTINCT or ALL modifier
    if ($self->_check('DISTINCT')) {
        push @ch, $self->_leaf($self->_advance());
    } elsif ($self->_check('ALL')) {
        push @ch, $self->_leaf($self->_advance());
    }

    push @ch, $self->_parse_select_list();
    push @ch, $self->_leaf($self->_expect('FROM'));
    push @ch, $self->_parse_table_ref();

    # Zero or more JOIN clauses
    while ($self->_check('JOIN') || $self->_check('INNER') || $self->_check('LEFT')
           || $self->_check('RIGHT') || $self->_check('FULL') || $self->_check('CROSS')) {
        push @ch, $self->_parse_join_clause();
    }

    if ($self->_check('WHERE')) {
        push @ch, $self->_parse_where_clause();
    }
    if ($self->_check('GROUP')) {
        push @ch, $self->_parse_group_clause();
    }
    if ($self->_check('HAVING')) {
        push @ch, $self->_parse_having_clause();
    }
    if ($self->_check('ORDER')) {
        push @ch, $self->_parse_order_clause();
    }
    if ($self->_check('LIMIT')) {
        push @ch, $self->_parse_limit_clause();
    }

    return $self->_node('select_stmt', @ch);
}

# select_list = STAR | select_item { "," select_item } ;
sub _parse_select_list {
    my ($self) = @_;
    my @ch;

    if ($self->_check('STAR')) {
        push @ch, $self->_leaf($self->_advance());
    } else {
        push @ch, $self->_parse_select_item();
        while ($self->_check('COMMA')) {
            push @ch, $self->_leaf($self->_advance());
            push @ch, $self->_parse_select_item();
        }
    }

    return $self->_node('select_list', @ch);
}

# select_item = expr [ "AS" NAME ] ;
sub _parse_select_item {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_expr();
    if ($self->_check('AS')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('NAME'));
    }
    return $self->_node('select_item', @ch);
}

# table_ref = table_name [ "AS" NAME ] ;
# table_name = NAME [ "." NAME ] ;
sub _parse_table_ref {
    my ($self) = @_;
    my @ch;

    # table_name
    my @tn_ch;
    push @tn_ch, $self->_leaf($self->_expect('NAME'));
    if ($self->_check('DOT')) {
        push @tn_ch, $self->_leaf($self->_advance());
        push @tn_ch, $self->_leaf($self->_expect('NAME'));
    }
    push @ch, $self->_node('table_name', @tn_ch);

    if ($self->_check('AS')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('NAME'));
    }

    return $self->_node('table_ref', @ch);
}

# join_clause = join_type "JOIN" table_ref "ON" expr ;
# join_type   = "CROSS" | "INNER" | "LEFT" [ "OUTER" ] | "RIGHT" [ "OUTER" ] | "FULL" [ "OUTER" ] ;
sub _parse_join_clause {
    my ($self) = @_;
    my @ch;

    # join_type tokens
    my @jt_ch;
    if ($self->_check('CROSS') || $self->_check('INNER')) {
        push @jt_ch, $self->_leaf($self->_advance());
    } elsif ($self->_check('LEFT') || $self->_check('RIGHT') || $self->_check('FULL')) {
        push @jt_ch, $self->_leaf($self->_advance());
        if ($self->_check('OUTER')) {
            push @jt_ch, $self->_leaf($self->_advance());
        }
    }
    # JOIN keyword itself
    push @jt_ch, $self->_leaf($self->_expect('JOIN'));
    push @ch, $self->_node('join_type', @jt_ch);

    push @ch, $self->_parse_table_ref();
    push @ch, $self->_leaf($self->_expect('ON'));
    push @ch, $self->_parse_expr();

    return $self->_node('join_clause', @ch);
}

# where_clause = "WHERE" expr ;
sub _parse_where_clause {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('WHERE'));
    push @ch, $self->_parse_expr();
    return $self->_node('where_clause', @ch);
}

# group_clause = "GROUP" "BY" column_ref { "," column_ref } ;
sub _parse_group_clause {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('GROUP'));
    push @ch, $self->_leaf($self->_expect('BY'));
    push @ch, $self->_parse_column_ref();
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_column_ref();
    }
    return $self->_node('group_clause', @ch);
}

# having_clause = "HAVING" expr ;
sub _parse_having_clause {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('HAVING'));
    push @ch, $self->_parse_expr();
    return $self->_node('having_clause', @ch);
}

# order_clause = "ORDER" "BY" order_item { "," order_item } ;
# order_item   = expr [ "ASC" | "DESC" ] ;
sub _parse_order_clause {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('ORDER'));
    push @ch, $self->_leaf($self->_expect('BY'));

    push @ch, $self->_parse_order_item();
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_order_item();
    }
    return $self->_node('order_clause', @ch);
}

sub _parse_order_item {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_expr();
    if ($self->_check('ASC') || $self->_check('DESC')) {
        push @ch, $self->_leaf($self->_advance());
    }
    return $self->_node('order_item', @ch);
}

# limit_clause = "LIMIT" NUMBER [ "OFFSET" NUMBER ] ;
sub _parse_limit_clause {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LIMIT'));
    push @ch, $self->_leaf($self->_expect('NUMBER'));
    if ($self->_check('OFFSET')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('NUMBER'));
    }
    return $self->_node('limit_clause', @ch);
}

# ── INSERT ───────────────────────────────────────────────────────────────────
#
# insert_stmt = "INSERT" "INTO" NAME
#               [ "(" NAME { "," NAME } ")" ]
#               "VALUES" row_value { "," row_value } ;
# row_value   = "(" expr { "," expr } ")" ;

sub _parse_insert_stmt {
    my ($self) = @_;
    my @ch;

    push @ch, $self->_leaf($self->_expect('INSERT'));
    push @ch, $self->_leaf($self->_expect('INTO'));
    push @ch, $self->_leaf($self->_expect('NAME'));

    # Optional column list
    if ($self->_check('LPAREN')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('NAME'));
        while ($self->_check('COMMA')) {
            push @ch, $self->_leaf($self->_advance());
            push @ch, $self->_leaf($self->_expect('NAME'));
        }
        push @ch, $self->_leaf($self->_expect('RPAREN'));
    }

    push @ch, $self->_leaf($self->_expect('VALUES'));
    push @ch, $self->_parse_row_value();
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_row_value();
    }

    return $self->_node('insert_stmt', @ch);
}

# row_value = "(" expr { "," expr } ")" ;
sub _parse_row_value {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_parse_expr();
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expr();
    }
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    return $self->_node('row_value', @ch);
}

# ── UPDATE ───────────────────────────────────────────────────────────────────
#
# update_stmt = "UPDATE" NAME "SET" assignment { "," assignment }
#               [ where_clause ] ;
# assignment  = NAME "=" expr ;

sub _parse_update_stmt {
    my ($self) = @_;
    my @ch;

    push @ch, $self->_leaf($self->_expect('UPDATE'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('SET'));
    push @ch, $self->_parse_assignment();
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_assignment();
    }
    if ($self->_check('WHERE')) {
        push @ch, $self->_parse_where_clause();
    }

    return $self->_node('update_stmt', @ch);
}

# assignment = NAME "=" expr ;
sub _parse_assignment {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('EQUALS'));
    push @ch, $self->_parse_expr();
    return $self->_node('assignment', @ch);
}

# ── DELETE ───────────────────────────────────────────────────────────────────
#
# delete_stmt = "DELETE" "FROM" NAME [ where_clause ] ;

sub _parse_delete_stmt {
    my ($self) = @_;
    my @ch;

    push @ch, $self->_leaf($self->_expect('DELETE'));
    push @ch, $self->_leaf($self->_expect('FROM'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    if ($self->_check('WHERE')) {
        push @ch, $self->_parse_where_clause();
    }

    return $self->_node('delete_stmt', @ch);
}

# ── Expressions ───────────────────────────────────────────────────────────────
#
# Precedence (lowest to highest):
#   or_expr → and_expr → not_expr → comparison → additive → multiplicative
#   → unary → primary
#
# expr = or_expr ;

sub _parse_expr {
    my ($self) = @_;
    my $inner = $self->_parse_or_expr();
    return $self->_node('expr', $inner);
}

# or_expr = and_expr { "OR" and_expr } ;
sub _parse_or_expr {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_and_expr();
    while ($self->_check('OR')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_and_expr();
    }
    return $self->_node('or_expr', @ch);
}

# and_expr = not_expr { "AND" not_expr } ;
sub _parse_and_expr {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_not_expr();
    while ($self->_check('AND')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_not_expr();
    }
    return $self->_node('and_expr', @ch);
}

# not_expr = "NOT" not_expr | comparison ;
sub _parse_not_expr {
    my ($self) = @_;
    if ($self->_check('NOT')) {
        my @ch;
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_not_expr();
        return $self->_node('not_expr', @ch);
    }
    return $self->_node('not_expr', $self->_parse_comparison());
}

# comparison = additive [ cmp_op additive | BETWEEN … | IN … | LIKE … | IS NULL ] ;
#
# Note: cmp_op = "=" | NOT_EQUALS | "<" | ">" | "<=" | ">="
# Token types: EQUALS, NOT_EQUALS, LESS_THAN, GREATER_THAN, LESS_EQUALS, GREATER_EQUALS
sub _parse_comparison {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_additive();

    my $tok = $self->_peek();
    my $type = $tok->{type};

    if ($type eq 'EQUALS' || $type eq 'NOT_EQUALS' || $type eq 'LESS_THAN'
        || $type eq 'GREATER_THAN' || $type eq 'LESS_EQUALS' || $type eq 'GREATER_EQUALS') {
        # cmp_op additive
        push @ch, $self->_node('cmp_op', $self->_leaf($self->_advance()));
        push @ch, $self->_parse_additive();
    } elsif ($type eq 'BETWEEN') {
        push @ch, $self->_leaf($self->_advance());  # BETWEEN
        push @ch, $self->_parse_additive();
        push @ch, $self->_leaf($self->_expect('AND'));
        push @ch, $self->_parse_additive();
    } elsif ($type eq 'NOT' && do { my $next = $self->{_tokens}[$self->{_pos}+1]; $next && $next->{type} eq 'BETWEEN' }) {
        push @ch, $self->_leaf($self->_advance());  # NOT
        push @ch, $self->_leaf($self->_advance());  # BETWEEN
        push @ch, $self->_parse_additive();
        push @ch, $self->_leaf($self->_expect('AND'));
        push @ch, $self->_parse_additive();
    } elsif ($type eq 'IN') {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('LPAREN'));
        push @ch, $self->_parse_value_list();
        push @ch, $self->_leaf($self->_expect('RPAREN'));
    } elsif ($type eq 'NOT' && do { my $next = $self->{_tokens}[$self->{_pos}+1]; $next && $next->{type} eq 'IN' }) {
        push @ch, $self->_leaf($self->_advance());  # NOT
        push @ch, $self->_leaf($self->_advance());  # IN
        push @ch, $self->_leaf($self->_expect('LPAREN'));
        push @ch, $self->_parse_value_list();
        push @ch, $self->_leaf($self->_expect('RPAREN'));
    } elsif ($type eq 'LIKE') {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_additive();
    } elsif ($type eq 'NOT' && do { my $next = $self->{_tokens}[$self->{_pos}+1]; $next && $next->{type} eq 'LIKE' }) {
        push @ch, $self->_leaf($self->_advance());  # NOT
        push @ch, $self->_leaf($self->_advance());  # LIKE
        push @ch, $self->_parse_additive();
    } elsif ($type eq 'IS') {
        push @ch, $self->_leaf($self->_advance());  # IS
        if ($self->_check('NOT')) {
            push @ch, $self->_leaf($self->_advance());  # NOT
        }
        push @ch, $self->_leaf($self->_expect('NULL'));
    }

    return $self->_node('comparison', @ch);
}

# additive = multiplicative { ( "+" | "-" ) multiplicative } ;
sub _parse_additive {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_multiplicative();
    while ($self->_check('PLUS') || $self->_check('MINUS')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_multiplicative();
    }
    return $self->_node('additive', @ch);
}

# multiplicative = unary { ( STAR | "/" | "%" ) unary } ;
sub _parse_multiplicative {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_unary();
    while ($self->_check('STAR') || $self->_check('SLASH') || $self->_check('PERCENT')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_unary();
    }
    return $self->_node('multiplicative', @ch);
}

# unary = "-" unary | primary ;
sub _parse_unary {
    my ($self) = @_;
    if ($self->_check('MINUS')) {
        my @ch;
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_unary();
        return $self->_node('unary', @ch);
    }
    return $self->_node('unary', $self->_parse_primary());
}

# primary = NUMBER | STRING | "NULL" | "TRUE" | "FALSE"
#         | function_call | column_ref | "(" expr ")" ;
sub _parse_primary {
    my ($self) = @_;
    my $tok = $self->_peek();
    my $type = $tok->{type};

    if ($type eq 'NUMBER') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }
    if ($type eq 'STRING') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }
    if ($type eq 'NULL') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }
    if ($type eq 'TRUE') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }
    if ($type eq 'FALSE') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }
    if ($type eq 'LPAREN') {
        my @ch;
        push @ch, $self->_leaf($self->_advance());  # (
        push @ch, $self->_parse_expr();
        push @ch, $self->_leaf($self->_expect('RPAREN'));
        return $self->_node('primary', @ch);
    }
    if ($type eq 'NAME') {
        # Could be function_call or column_ref.
        # Look ahead: if next token is LPAREN, it's a function call.
        my $next = $self->{_tokens}[ $self->{_pos} + 1 ];
        if ($next && $next->{type} eq 'LPAREN') {
            return $self->_node('primary', $self->_parse_function_call());
        }
        return $self->_node('primary', $self->_parse_column_ref());
    }

    die sprintf(
        "CodingAdventures::SqlParser: unexpected token '%s' (type %s) "
      . "at line %d col %d in expression\n",
        $tok->{value}, $tok->{type}, $tok->{line}, $tok->{col}
    );
}

# column_ref = NAME [ "." NAME ] ;
sub _parse_column_ref {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    if ($self->_check('DOT')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('NAME'));
    }
    return $self->_node('column_ref', @ch);
}

# function_call = NAME "(" ( STAR | [ value_list ] ) ")" ;
sub _parse_function_call {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));

    if ($self->_check('STAR')) {
        push @ch, $self->_leaf($self->_advance());
    } elsif (!$self->_check('RPAREN')) {
        push @ch, $self->_parse_value_list();
    }

    push @ch, $self->_leaf($self->_expect('RPAREN'));
    return $self->_node('function_call', @ch);
}

# value_list = expr { "," expr } ;
sub _parse_value_list {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_expr();
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expr();
    }
    return $self->_node('value_list', @ch);
}

# ============================================================================
# Class-method convenience wrapper
# ============================================================================

# --- parse_sql($source) -------------------------------------------------------
#
# Convenience class method: tokenize and parse in one call.
# Returns the root ASTNode. Dies on error.

sub parse_sql {
    my ($class, $source) = @_;
    my $parser = $class->new($source);
    return $parser->parse();
}

1;

__END__

=head1 NAME

CodingAdventures::SqlParser - Hand-written recursive-descent SQL parser

=head1 SYNOPSIS

    use CodingAdventures::SqlParser;

    # Object-oriented
    my $parser = CodingAdventures::SqlParser->new("SELECT * FROM users");
    my $ast    = $parser->parse();
    print $ast->rule_name;   # "program"

    # Convenience class method
    my $ast = CodingAdventures::SqlParser->parse_sql("DELETE FROM t WHERE id = 1");

=head1 DESCRIPTION

A hand-written recursive-descent parser for a subset of ANSI SQL.
Tokenizes the input with C<CodingAdventures::SqlLexer> and builds an
Abstract Syntax Tree (AST) of C<CodingAdventures::SqlParser::ASTNode> nodes.

Supported statement types: C<SELECT>, C<INSERT INTO ... VALUES>, C<UPDATE ... SET>,
C<DELETE FROM>.

Full expression support including C<OR>, C<AND>, C<NOT>, comparison operators,
C<BETWEEN>, C<IN>, C<LIKE>, C<IS NULL>, arithmetic, and function calls.

=head1 METHODS

=head2 new($source)

Tokenize C<$source> and return a parser instance ready to call C<parse()>.

=head2 parse()

Parse the token stream and return the root AST node (rule_name C<"program">).
Dies with a descriptive message on parse errors.

=head2 parse_sql($source)

Class method — tokenize and parse in one call. Returns the root ASTNode.

=head1 AST NODE FORMAT

Each node is a C<CodingAdventures::SqlParser::ASTNode> instance:

    $node->rule_name   # string rule name, e.g. "select_stmt"
    $node->children    # arrayref of child nodes
    $node->is_leaf     # 1 for leaf (token) nodes, 0 for inner nodes
    $node->token       # token hashref (leaf nodes only)

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
