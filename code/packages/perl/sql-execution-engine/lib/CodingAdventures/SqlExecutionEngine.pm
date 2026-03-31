package CodingAdventures::SqlExecutionEngine;

# sql_execution_engine — SELECT-Only SQL Execution Engine (Perl)
# ==============================================================
#
# This module implements a complete SELECT-only SQL execution engine that
# evaluates SQL queries against pluggable data sources.
#
# # Architecture: The Materialized Pipeline
# ==========================================
#
# The engine uses a MATERIALIZED PIPELINE: each stage reads all rows from
# the previous stage into memory, transforms them, and passes the result
# forward.  This makes the intermediate state visible and debuggable.
#
#   SQL string
#       │  tokenize()
#       ▼
#   tokens
#       │  Parser->parse()
#       ▼
#   AST (hashref)
#       │  execute($ast, $ds)
#       ▼
#   Stage 1: FROM + JOINs
#   Stage 2: WHERE
#   Stage 3: GROUP BY
#   Stage 4: HAVING
#   Stage 5: SELECT
#   Stage 6: DISTINCT
#   Stage 7: ORDER BY
#   Stage 8: LIMIT / OFFSET
#       ▼
#   { columns => [...], rows => [[...], ...] }
#
# # DataSource Protocol
# =====================
#
# Any data source must provide:
#   $ds->schema($table_name)  → arrayref of column-name strings
#   $ds->scan($table_name)    → arrayref of row hashrefs { col => value }
#
# # NULL Handling
# ==============
#
# SQL uses three-valued logic: TRUE, FALSE, and UNKNOWN (NULL).
# We represent NULL as Perl's undef.  Comparisons involving NULL return undef
# (not 0), which the WHERE stage treats as falsy (row excluded).

use strict;
use warnings;
use Scalar::Util qw(looks_like_number);
use POSIX        qw(floor);
use List::Util   qw(sum min max);

our $VERSION = '0.01';

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

# execute($sql, $ds) → ($ok, $result_or_error)
# Run a single SQL statement.  Returns (1, {columns,rows}) on success or
# (0, "error message") on failure.
sub execute {
    my ($class, $sql, $ds) = @_;
    my $result = eval {
        my $tokens = _tokenize($sql);
        my $parser = CodingAdventures::SqlExecutionEngine::Parser->new($tokens);
        my $ast    = $parser->parse_statement();
        _execute_select($ast, $ds);
    };
    if ($@) {
        my $err = $@;
        $err =~ s/ at .+ line \d+.*//s;
        return (0, $err);
    }
    return (1, $result);
}

# execute_all($sql, $ds) → (\@results, undef) or (undef, $error)
# Run multiple semicolon-separated statements.
sub execute_all {
    my ($class, $sql, $ds) = @_;
    my @statements = grep { /\S/ } split /;/, $sql;
    my @results;
    for my $stmt (@statements) {
        my ($ok, $res) = $class->execute($stmt, $ds);
        return (undef, $res) unless $ok;
        push @results, $res;
    }
    return (\@results, undef);
}

# ---------------------------------------------------------------------------
# SQL Lexer
# ---------------------------------------------------------------------------
#
# Converts a SQL string into a flat list of token hashrefs:
#   { type => 'KEYWORD'|'IDENT'|'NUMBER'|'STRING'|'OP'|'PUNCT'|'EOF',
#     value => "..." }
#
# Keywords are uppercased so the parser can compare case-insensitively.

my %KEYWORDS = map { $_ => 1 } qw(
    SELECT FROM WHERE GROUP BY HAVING ORDER LIMIT OFFSET DISTINCT ALL
    JOIN INNER LEFT RIGHT FULL OUTER CROSS ON AS
    AND OR NOT IN BETWEEN LIKE IS NULL
    TRUE FALSE ASC DESC
    COUNT SUM AVG MIN MAX UPPER LOWER LENGTH
);

sub _tokenize {
    my ($sql) = @_;
    my @tokens;
    my $i = 0;
    my $n = length($sql);

    while ($i < $n) {
        my $c = substr($sql, $i, 1);

        # Whitespace — skip
        if ($c =~ /\s/) { $i++; next }

        # Line comment -- ...
        if (substr($sql, $i, 2) eq '--') {
            while ($i < $n && substr($sql, $i, 1) ne "\n") { $i++ }
            next;
        }

        # String literal: single-quoted
        if ($c eq "'") {
            my $j = $i + 1;
            my $s = '';
            while ($j < $n) {
                my $ch = substr($sql, $j, 1);
                if ($ch eq "'" && $j + 1 < $n && substr($sql, $j + 1, 1) eq "'") {
                    $s .= "'"; $j += 2; next;
                }
                last if $ch eq "'";
                $s .= $ch; $j++;
            }
            $j++;  # consume closing quote
            push @tokens, { type => 'STRING', value => $s };
            $i = $j; next;
        }

        # Number
        if ($c =~ /\d/ || ($c eq '.' && $i + 1 < $n && substr($sql, $i + 1, 1) =~ /\d/)) {
            my $j = $i;
            while ($j < $n && substr($sql, $j, 1) =~ /[\d.]/) { $j++ }
            my $num_str = substr($sql, $i, $j - $i);
            my $num = ($num_str =~ /\./) ? $num_str + 0.0 : $num_str + 0;
            push @tokens, { type => 'NUMBER', value => $num };
            $i = $j; next;
        }

        # Identifier or keyword
        if ($c =~ /[a-zA-Z_]/) {
            my $j = $i;
            while ($j < $n && substr($sql, $j, 1) =~ /[a-zA-Z0-9_]/) { $j++ }
            my $word = substr($sql, $i, $j - $i);
            my $upper = uc($word);
            if ($KEYWORDS{$upper}) {
                push @tokens, { type => 'KEYWORD', value => $upper };
            } else {
                push @tokens, { type => 'IDENT', value => $word };
            }
            $i = $j; next;
        }

        # Two-character operators: !=  <>  <=  >=
        if ($i + 1 < $n) {
            my $two = substr($sql, $i, 2);
            if ($two =~ /^(!=|<>|<=|>=|\|\|)$/) {
                push @tokens, { type => 'OP', value => $two };
                $i += 2; next;
            }
        }

        # Single-character operator or punctuation
        if ($c =~ /[=<>+\-*\/%]/) {
            push @tokens, { type => 'OP',    value => $c }; $i++; next;
        }
        if ($c =~ /[(),.]/) {
            push @tokens, { type => 'PUNCT', value => $c }; $i++; next;
        }

        # Unknown — skip
        $i++;
    }

    push @tokens, { type => 'EOF', value => '' };
    return \@tokens;
}

# ---------------------------------------------------------------------------
# Recursive-Descent SQL Parser
# ---------------------------------------------------------------------------
#
# Produces an AST hashref for a single SELECT statement.
# Grammar (simplified):
#
#   statement  → SELECT [DISTINCT] select_list FROM table_ref [join*]
#                [WHERE expr] [GROUP BY expr_list] [HAVING expr]
#                [ORDER BY order_list] [LIMIT n [OFFSET m]]
#
#   expr       → or_expr
#   or_expr    → and_expr (OR and_expr)*
#   and_expr   → not_expr (AND not_expr)*
#   not_expr   → NOT not_expr | comparison
#   comparison → additive [IS [NOT] NULL | BETWEEN a AND b |
#                          [NOT] IN (...) | [NOT] LIKE pat |
#                          op additive]
#   additive   → mult ([+-] mult)*
#   mult       → unary ([*/%] unary)*
#   unary      → [-] primary | primary
#   primary    → number | string | TRUE | FALSE | NULL | * |
#                ident(dot ident)? | func_call | ( expr )

package CodingAdventures::SqlExecutionEngine::Parser;

sub new {
    my ($class, $tokens) = @_;
    return bless { tokens => $tokens, pos => 0 }, $class;
}

sub _peek { my $s = shift; $s->{tokens}[$s->{pos}] }
sub _advance {
    my $s = shift;
    my $t = $s->{tokens}[$s->{pos}];
    $s->{pos}++ if $t->{type} ne 'EOF';
    return $t;
}

sub _expect {
    my ($s, $type, $value) = @_;
    my $t = $s->_peek();
    if (defined $value) {
        die "Expected $value, got '$t->{value}'\n"
            unless $t->{type} eq $type && $t->{value} eq $value;
    } else {
        die "Expected $type, got '$t->{type}' ('$t->{value}')\n"
            unless $t->{type} eq $type;
    }
    return $s->_advance();
}

sub _peek_kw { my ($s, $kw) = @_; my $t = $s->_peek(); $t->{type} eq 'KEYWORD' && $t->{value} eq $kw }
sub _peek_op { my ($s, $op) = @_; my $t = $s->_peek(); $t->{type} eq 'OP'      && $t->{value} eq $op }
sub _peek_pt { my ($s, $pt) = @_; my $t = $s->_peek(); $t->{type} eq 'PUNCT'   && $t->{value} eq $pt }

sub _maybe_kw {
    my ($s, $kw) = @_;
    return $s->_advance() if $s->_peek_kw($kw);
    return undef;
}

sub parse_statement {
    my ($s) = @_;
    $s->_expect('KEYWORD', 'SELECT');
    my $distinct = $s->_maybe_kw('DISTINCT') ? 1 : 0;
    my @cols = $s->_parse_select_list();
    $s->_expect('KEYWORD', 'FROM');
    my $from = $s->_parse_table_ref();
    my @joins = $s->_parse_joins();
    my $where  = $s->_maybe_kw('WHERE')  ? $s->_parse_expr() : undef;
    my @group_by;
    if ($s->_peek_kw('GROUP')) {
        $s->_advance(); $s->_expect('KEYWORD', 'BY');
        @group_by = $s->_parse_expr_list();
    }
    my $having = $s->_maybe_kw('HAVING') ? $s->_parse_expr() : undef;
    my @order_by;
    if ($s->_peek_kw('ORDER')) {
        $s->_advance(); $s->_expect('KEYWORD', 'BY');
        @order_by = $s->_parse_order_list();
    }
    my $limit  = $s->_maybe_kw('LIMIT')  ? $s->_advance()->{value} : undef;
    my $offset = $s->_maybe_kw('OFFSET') ? $s->_advance()->{value} : undef;

    return {
        type     => 'SELECT',
        distinct => $distinct,
        columns  => \@cols,
        from     => $from,
        joins    => \@joins,
        where    => $where,
        group_by => \@group_by,
        having   => $having,
        order_by => \@order_by,
        limit    => $limit,
        offset   => $offset,
    };
}

sub _parse_table_ref {
    my ($s) = @_;
    my $name = $s->_expect('IDENT')->{value};
    my $alias = $name;
    if ($s->_maybe_kw('AS')) {
        $alias = $s->_expect('IDENT')->{value};
    } elsif ($s->_peek()->{type} eq 'IDENT') {
        $alias = $s->_advance()->{value};
    }
    return { name => $name, alias => $alias };
}

my %JOIN_TYPES = (INNER => 'INNER', LEFT => 'LEFT', RIGHT => 'RIGHT',
                  FULL  => 'FULL',  CROSS => 'CROSS', JOIN => 'INNER');

sub _parse_joins {
    my ($s) = @_;
    my @joins;
    while (1) {
        my $t = $s->_peek();
        last unless $t->{type} eq 'KEYWORD';
        my $jtype;
        if ($t->{value} =~ /^(INNER|LEFT|RIGHT|FULL|CROSS)$/) {
            $jtype = $t->{value};
            $s->_advance();
            $s->_maybe_kw('OUTER');
            $s->_expect('KEYWORD', 'JOIN');
        } elsif ($t->{value} eq 'JOIN') {
            $jtype = 'INNER';
            $s->_advance();
        } else { last }
        my $tref = $s->_parse_table_ref();
        my $on   = undef;
        if ($s->_maybe_kw('ON')) {
            $on = $s->_parse_expr();
        }
        push @joins, { type => $jtype, table => $tref, on => $on };
    }
    return @joins;
}

sub _parse_select_list {
    my ($s) = @_;
    my @cols;
    # SELECT *
    if ($s->_peek_op('*')) {
        $s->_advance();
        return ({ expr => { type => 'STAR' }, alias => undef });
    }
    push @cols, $s->_parse_select_col();
    while ($s->_peek_pt(',')) {
        $s->_advance();
        push @cols, $s->_parse_select_col();
    }
    return @cols;
}

sub _parse_select_col {
    my ($s) = @_;
    my $expr  = $s->_parse_expr();
    my $alias = undef;
    if ($s->_maybe_kw('AS')) {
        $alias = $s->_expect('IDENT')->{value};
    } elsif ($s->_peek()->{type} eq 'IDENT') {
        $alias = $s->_advance()->{value};
    }
    return { expr => $expr, alias => $alias };
}

sub _parse_expr_list {
    my ($s) = @_;
    my @exprs = ($s->_parse_expr());
    while ($s->_peek_pt(',')) { $s->_advance(); push @exprs, $s->_parse_expr() }
    return @exprs;
}

sub _parse_order_list {
    my ($s) = @_;
    my @items;
    push @items, $s->_parse_order_item();
    while ($s->_peek_pt(',')) { $s->_advance(); push @items, $s->_parse_order_item() }
    return @items;
}

sub _parse_order_item {
    my ($s) = @_;
    my $expr = $s->_parse_expr();
    my $dir  = 'ASC';
    $dir = $s->_advance()->{value} if $s->_peek_kw('ASC') || $s->_peek_kw('DESC');
    return { expr => $expr, dir => $dir };
}

# Expression parsing — precedence climbing

sub _parse_expr { $_[0]->_parse_or() }

sub _parse_or {
    my ($s) = @_;
    my $left = $s->_parse_and();
    while ($s->_peek_kw('OR')) {
        $s->_advance();
        $left = { type => 'OR', left => $left, right => $s->_parse_and() };
    }
    return $left;
}

sub _parse_and {
    my ($s) = @_;
    my $left = $s->_parse_not();
    while ($s->_peek_kw('AND')) {
        $s->_advance();
        $left = { type => 'AND', left => $left, right => $s->_parse_not() };
    }
    return $left;
}

sub _parse_not {
    my ($s) = @_;
    if ($s->_peek_kw('NOT')) {
        $s->_advance();
        return { type => 'NOT', expr => $s->_parse_not() };
    }
    return $s->_parse_comparison();
}

sub _parse_comparison {
    my ($s) = @_;
    my $left = $s->_parse_additive();
    my $t    = $s->_peek();

    # IS [NOT] NULL
    if ($t->{type} eq 'KEYWORD' && $t->{value} eq 'IS') {
        $s->_advance();
        my $neg = $s->_maybe_kw('NOT') ? 1 : 0;
        $s->_expect('KEYWORD', 'NULL');
        return { type => ($neg ? 'IS_NOT_NULL' : 'IS_NULL'), expr => $left };
    }

    # [NOT] BETWEEN
    if ($t->{type} eq 'KEYWORD' && $t->{value} eq 'BETWEEN') {
        $s->_advance();
        my $lo = $s->_parse_additive();
        $s->_expect('KEYWORD', 'AND');
        my $hi = $s->_parse_additive();
        return { type => 'BETWEEN', expr => $left, lo => $lo, hi => $hi };
    }

    # [NOT] IN (...)
    if ($t->{type} eq 'KEYWORD' && $t->{value} eq 'IN') {
        $s->_advance();
        $s->_expect('PUNCT', '(');
        my @vals = ($s->_parse_expr());
        while ($s->_peek_pt(',')) { $s->_advance(); push @vals, $s->_parse_expr() }
        $s->_expect('PUNCT', ')');
        return { type => 'IN', expr => $left, values => \@vals };
    }

    # [NOT] LIKE
    if ($t->{type} eq 'KEYWORD' && $t->{value} eq 'LIKE') {
        $s->_advance();
        return { type => 'LIKE', expr => $left, pattern => $s->_parse_additive() };
    }

    # NOT BETWEEN / NOT IN / NOT LIKE
    if ($t->{type} eq 'KEYWORD' && $t->{value} eq 'NOT') {
        $s->_advance();
        my $t2 = $s->_peek();
        if ($t2->{value} eq 'BETWEEN') {
            $s->_advance();
            my $lo = $s->_parse_additive();
            $s->_expect('KEYWORD', 'AND');
            my $hi = $s->_parse_additive();
            return { type => 'NOT', expr => { type => 'BETWEEN', expr => $left, lo => $lo, hi => $hi } };
        }
        if ($t2->{value} eq 'IN') {
            $s->_advance();
            $s->_expect('PUNCT', '(');
            my @vals = ($s->_parse_expr());
            while ($s->_peek_pt(',')) { $s->_advance(); push @vals, $s->_parse_expr() }
            $s->_expect('PUNCT', ')');
            return { type => 'NOT', expr => { type => 'IN', expr => $left, values => \@vals } };
        }
        if ($t2->{value} eq 'LIKE') {
            $s->_advance();
            return { type => 'NOT', expr => { type => 'LIKE', expr => $left, pattern => $s->_parse_additive() } };
        }
    }

    # Binary comparison operators
    if ($t->{type} eq 'OP' && $t->{value} =~ /^(=|!=|<>|<|>|<=|>=)$/) {
        $s->_advance();
        return { type => 'BINOP', op => $t->{value}, left => $left, right => $s->_parse_additive() };
    }

    return $left;
}

sub _parse_additive {
    my ($s) = @_;
    my $left = $s->_parse_mult();
    while ($s->_peek()->{type} eq 'OP' && $s->_peek()->{value} =~ /^[+\-]$/) {
        my $op = $s->_advance()->{value};
        $left = { type => 'BINOP', op => $op, left => $left, right => $s->_parse_mult() };
    }
    return $left;
}

sub _parse_mult {
    my ($s) = @_;
    my $left = $s->_parse_unary();
    while ($s->_peek()->{type} eq 'OP' && $s->_peek()->{value} =~ /^[*\/%]$/) {
        my $op = $s->_advance()->{value};
        $left = { type => 'BINOP', op => $op, left => $left, right => $s->_parse_unary() };
    }
    return $left;
}

sub _parse_unary {
    my ($s) = @_;
    if ($s->_peek_op('-')) {
        $s->_advance();
        return { type => 'UNARY_MINUS', expr => $s->_parse_primary() };
    }
    return $s->_parse_primary();
}

sub _parse_primary {
    my ($s) = @_;
    my $t = $s->_peek();

    # Parenthesised expression
    if ($t->{type} eq 'PUNCT' && $t->{value} eq '(') {
        $s->_advance();
        my $e = $s->_parse_expr();
        $s->_expect('PUNCT', ')');
        return $e;
    }

    # Literals
    if ($t->{type} eq 'NUMBER')  { $s->_advance(); return { type => 'LITERAL', value => $t->{value} } }
    if ($t->{type} eq 'STRING')  { $s->_advance(); return { type => 'LITERAL', value => $t->{value} } }
    if ($t->{type} eq 'KEYWORD' && $t->{value} eq 'NULL')  { $s->_advance(); return { type => 'NULL' } }
    if ($t->{type} eq 'KEYWORD' && $t->{value} eq 'TRUE')  { $s->_advance(); return { type => 'LITERAL', value => 1 } }
    if ($t->{type} eq 'KEYWORD' && $t->{value} eq 'FALSE') { $s->_advance(); return { type => 'LITERAL', value => 0 } }

    # Star (COUNT(*))
    if ($t->{type} eq 'OP' && $t->{value} eq '*') {
        $s->_advance();
        return { type => 'STAR' };
    }

    # Aggregate / function call
    my %AGG_FUNCS = map { $_ => 1 } qw(COUNT SUM AVG MIN MAX);
    my %STR_FUNCS = map { $_ => 1 } qw(UPPER LOWER LENGTH);
    if ($t->{type} eq 'KEYWORD' && ($AGG_FUNCS{$t->{value}} || $STR_FUNCS{$t->{value}})) {
        my $fname = $s->_advance()->{value};
        $s->_expect('PUNCT', '(');
        my $arg;
        if ($fname eq 'COUNT' && $s->_peek_op('*')) {
            $s->_advance();
            $arg = { type => 'STAR' };
        } else {
            $arg = $s->_parse_expr();
        }
        $s->_expect('PUNCT', ')');
        my $ftype = $AGG_FUNCS{$fname} ? 'AGG' : 'FUNC';
        return { type => $ftype, name => $fname, arg => $arg };
    }

    # Identifier (possibly qualified: table.col)
    if ($t->{type} eq 'IDENT') {
        my $name = $s->_advance()->{value};
        if ($s->_peek_pt('.')) {
            $s->_advance();
            my $col = $s->_expect('IDENT')->{value};
            return { type => 'COLUMN', table => $name, name => $col };
        }
        return { type => 'COLUMN', table => undef, name => $name };
    }

    die "Unexpected token: type=$t->{type} value='$t->{value}'\n";
}

# ---------------------------------------------------------------------------
# Expression Evaluator
# ---------------------------------------------------------------------------
#
# eval_expr($node, $row, $group_rows, $aggregated) → scalar value or undef
#
# $row          = current row hashref (bare + qualified names)
# $group_rows   = all rows in the current GROUP (for aggregates)
# $aggregated   = 1 if we are computing a GROUP result

package CodingAdventures::SqlExecutionEngine;

sub _eval_expr {
    my ($node, $row, $group_rows) = @_;
    my $type = $node->{type};

    # ---- Literals and constants ----
    return undef                if $type eq 'NULL';
    return $node->{value}       if $type eq 'LITERAL';

    # ---- Columns ----
    if ($type eq 'COLUMN') {
        my $col = $node->{name};
        if (defined $node->{table}) {
            my $qkey = "$node->{table}.$col";
            return $row->{$qkey} if exists $row->{$qkey};
        }
        return $row->{$col} if exists $row->{$col};
        # Check qualified names with any prefix
        for my $k (keys %$row) {
            return $row->{$k} if $k =~ /\.\Q$col\E$/;
        }
        return undef;
    }

    # ---- Unary minus ----
    if ($type eq 'UNARY_MINUS') {
        my $v = _eval_expr($node->{expr}, $row, $group_rows);
        return defined $v ? -$v : undef;
    }

    # ---- Binary operators ----
    if ($type eq 'BINOP') {
        my $op = $node->{op};
        my $l  = _eval_expr($node->{left},  $row, $group_rows);
        my $r  = _eval_expr($node->{right}, $row, $group_rows);
        return undef unless defined $l && defined $r;
        if ($op eq '+')  { return $l + $r }
        if ($op eq '-')  { return $l - $r }
        if ($op eq '*')  { return $l * $r }
        if ($op eq '/')  { die "Division by zero\n" if $r == 0; return $l / $r }
        if ($op eq '%')  { return $l % $r }
        if ($op eq '=')  { return _sql_eq($l, $r) }
        if ($op eq '!=' || $op eq '<>') { my $eq = _sql_eq($l, $r); return defined $eq ? !$eq : undef }
        if ($op eq '<')  { return _sql_cmp($l, $r) < 0  ? 1 : 0 }
        if ($op eq '>')  { return _sql_cmp($l, $r) > 0  ? 1 : 0 }
        if ($op eq '<=') { return _sql_cmp($l, $r) <= 0 ? 1 : 0 }
        if ($op eq '>=') { return _sql_cmp($l, $r) >= 0 ? 1 : 0 }
        die "Unknown operator: $op\n";
    }

    # ---- Logical operators ----
    if ($type eq 'AND') {
        my $l = _eval_expr($node->{left},  $row, $group_rows);
        return 0 if defined $l && !$l;    # short-circuit FALSE
        my $r = _eval_expr($node->{right}, $row, $group_rows);
        return 0 if defined $r && !$r;
        return undef unless defined $l && defined $r;
        return $l && $r ? 1 : 0;
    }
    if ($type eq 'OR') {
        my $l = _eval_expr($node->{left},  $row, $group_rows);
        return 1 if defined $l && $l;     # short-circuit TRUE
        my $r = _eval_expr($node->{right}, $row, $group_rows);
        return 1 if defined $r && $r;
        return undef unless defined $l && defined $r;
        return $l || $r ? 1 : 0;
    }
    if ($type eq 'NOT') {
        my $v = _eval_expr($node->{expr}, $row, $group_rows);
        return undef unless defined $v;
        return $v ? 0 : 1;
    }

    # ---- IS NULL / IS NOT NULL ----
    if ($type eq 'IS_NULL')     { return !defined(_eval_expr($node->{expr}, $row, $group_rows)) ? 1 : 0 }
    if ($type eq 'IS_NOT_NULL') { return  defined(_eval_expr($node->{expr}, $row, $group_rows)) ? 1 : 0 }

    # ---- BETWEEN ----
    if ($type eq 'BETWEEN') {
        my $v  = _eval_expr($node->{expr}, $row, $group_rows);
        my $lo = _eval_expr($node->{lo},   $row, $group_rows);
        my $hi = _eval_expr($node->{hi},   $row, $group_rows);
        return undef unless defined $v && defined $lo && defined $hi;
        return (_sql_cmp($v, $lo) >= 0 && _sql_cmp($v, $hi) <= 0) ? 1 : 0;
    }

    # ---- IN ----
    if ($type eq 'IN') {
        my $v = _eval_expr($node->{expr}, $row, $group_rows);
        return undef unless defined $v;
        for my $val_node (@{$node->{values}}) {
            my $val = _eval_expr($val_node, $row, $group_rows);
            next unless defined $val;
            return 1 if _sql_eq($v, $val);
        }
        return 0;
    }

    # ---- LIKE ----
    if ($type eq 'LIKE') {
        my $v   = _eval_expr($node->{expr},    $row, $group_rows);
        my $pat = _eval_expr($node->{pattern}, $row, $group_rows);
        return undef unless defined $v && defined $pat;
        # Convert SQL LIKE pattern to Perl regex
        my $re = _like_to_regex($pat);
        return $v =~ /^$re$/ ? 1 : 0;
    }

    # ---- Aggregate functions ----
    if ($type eq 'AGG') {
        my $fname = $node->{name};
        die "Aggregate $fname used outside GROUP BY context\n" unless defined $group_rows;
        if ($fname eq 'COUNT') {
            if ($node->{arg}{type} eq 'STAR') {
                return scalar @$group_rows;
            }
            my $cnt = 0;
            for my $r (@$group_rows) {
                $cnt++ if defined _eval_expr($node->{arg}, $r, undef);
            }
            return $cnt;
        }
        my @vals = grep { defined $_ }
                   map  { _eval_expr($node->{arg}, $_, undef) } @$group_rows;
        return undef unless @vals;
        if ($fname eq 'SUM') { my $s = 0; $s += $_ for @vals; return $s }
        if ($fname eq 'AVG') { my $s = 0; $s += $_ for @vals; return $s / @vals }
        if ($fname eq 'MIN') { return (sort { _sql_cmp($a,$b) } @vals)[0]  }
        if ($fname eq 'MAX') { return (sort { _sql_cmp($b,$a) } @vals)[0]  }
        die "Unknown aggregate: $fname\n";
    }

    # ---- Scalar functions ----
    if ($type eq 'FUNC') {
        my $fname = $node->{name};
        my $v     = _eval_expr($node->{arg}, $row, $group_rows);
        return undef unless defined $v;
        if ($fname eq 'UPPER')  { return uc($v) }
        if ($fname eq 'LOWER')  { return lc($v) }
        if ($fname eq 'LENGTH') { return length($v) }
        die "Unknown function: $fname\n";
    }

    die "Unknown expression type: $type\n";
}

sub _sql_eq {
    my ($a, $b) = @_;
    return undef unless defined $a && defined $b;
    if (looks_like_number($a) && looks_like_number($b)) {
        return $a == $b ? 1 : 0;
    }
    return $a eq $b ? 1 : 0;
}

sub _sql_cmp {
    my ($a, $b) = @_;
    if (looks_like_number($a) && looks_like_number($b)) { return $a <=> $b }
    return $a cmp $b;
}

sub _like_to_regex {
    my ($pat) = @_;
    my $re = '';
    for my $ch (split //, $pat) {
        if    ($ch eq '%') { $re .= '.*' }
        elsif ($ch eq '_') { $re .= '.' }
        else { $re .= quotemeta($ch) }
    }
    return $re;
}

# ---------------------------------------------------------------------------
# Query Executor
# ---------------------------------------------------------------------------
#
# Runs the 8-stage materialized pipeline against a DataSource.

sub _execute_select {
    my ($ast, $ds) = @_;
    die "Only SELECT statements are supported\n" unless $ast->{type} eq 'SELECT';

    # ------------------------------------------------------------------
    # Stage 1: FROM + JOINs
    # ------------------------------------------------------------------
    my $from_table = $ast->{from};
    my $rows = _scan_table($ds, $from_table->{name}, $from_table->{alias});

    for my $join (@{$ast->{joins}}) {
        my $right = _scan_table($ds, $join->{table}{name}, $join->{table}{alias});
        my $jtype = $join->{type};
        my $on    = $join->{on};
        $rows = _apply_join($jtype, $rows, $right, $on);
    }

    # ------------------------------------------------------------------
    # Stage 2: WHERE
    # ------------------------------------------------------------------
    if (defined $ast->{where}) {
        my $expr = $ast->{where};
        $rows = [ grep { my $v = eval { _eval_expr($expr, $_, undef) }; defined $v && $v } @$rows ];
    }

    # ------------------------------------------------------------------
    # Stage 3: GROUP BY + aggregates
    # ------------------------------------------------------------------
    my $grouped;
    if (@{$ast->{group_by}}) {
        $grouped = {};
        my @group_keys_order;
        for my $row (@$rows) {
            my @key_parts = map { _eval_expr($_, $row, undef) // '__NULL__' } @{$ast->{group_by}};
            my $key = join("\x00", @key_parts);
            unless (exists $grouped->{$key}) {
                $grouped->{$key} = { group_row => $row, rows => [], key => \@key_parts };
                push @group_keys_order, $key;
            }
            push @{$grouped->{$key}{rows}}, $row;
        }
        # Re-order grouped as list
        $rows = [ map { $grouped->{$_} } @group_keys_order ];
    } elsif (_has_aggregate($ast->{columns}) || defined $ast->{having}) {
        # Implicit single-group aggregation
        $rows = [{ group_row => ($rows->[0] // {}), rows => $rows, key => [] }];
        $grouped = 1;
    }

    # ------------------------------------------------------------------
    # Stage 4: HAVING
    # ------------------------------------------------------------------
    if (defined $ast->{having} && defined $grouped) {
        my $expr = $ast->{having};
        $rows = [ grep {
            my $v = eval { _eval_expr($expr, $_->{group_row}, $_->{rows}) };
            defined $v && $v;
        } @$rows ];
    }

    # ------------------------------------------------------------------
    # Stage 5: SELECT — project columns
    # ------------------------------------------------------------------
    my (@out_cols, @out_rows);
    my $first = 1;

    # Handle SELECT *
    if (@{$ast->{columns}} == 1 && $ast->{columns}[0]{expr}{type} eq 'STAR') {
        for my $item (@$rows) {
            my ($row, $group_rows) = defined $grouped
                ? ($item->{group_row}, $item->{rows})
                : ($item, undef);
            if ($first) {
                @out_cols = sort keys %$row;
                $first = 0;
            }
            push @out_rows, [ map { $row->{$_} } @out_cols ];
        }
    } else {
        # Compute column names from first row
        for my $col_spec (@{$ast->{columns}}) {
            my $alias = $col_spec->{alias};
            unless (defined $alias) {
                my $expr = $col_spec->{expr};
                if ($expr->{type} eq 'COLUMN') {
                    $alias = $expr->{name};
                } elsif ($expr->{type} eq 'AGG' || $expr->{type} eq 'FUNC') {
                    $alias = "$expr->{name}(...)";
                } else {
                    $alias = '?';
                }
            }
            push @out_cols, $alias;
        }

        for my $item (@$rows) {
            my ($row, $group_rows) = defined $grouped
                ? ($item->{group_row}, $item->{rows})
                : ($item, undef);
            push @out_rows, [
                map { _eval_expr($_->{expr}, $row, $group_rows) } @{$ast->{columns}}
            ];
        }
    }

    # ------------------------------------------------------------------
    # Stage 6: DISTINCT
    # ------------------------------------------------------------------
    if ($ast->{distinct}) {
        my %seen;
        @out_rows = grep {
            my $key = join("\x00", map { defined $_ ? $_ : '__NULL__' } @$_);
            !$seen{$key}++;
        } @out_rows;
    }

    # ------------------------------------------------------------------
    # Stage 7: ORDER BY
    # ------------------------------------------------------------------
    if (@{$ast->{order_by}}) {
        # Re-evaluate ORDER BY exprs against the *original* rows (before SELECT)
        # We need to sort the output rows; we attach keys for sort
        my @keyed;
        for my $i (0 .. $#out_rows) {
            # Try to get the original row for key computation
            my $orig_item = $rows->[$i];
            my $row = (defined $grouped && ref $orig_item eq 'HASH' && exists $orig_item->{group_row})
                ? $orig_item->{group_row}
                : (ref $orig_item eq 'HASH' ? $orig_item : {});
            # Build sort key from output row (column index lookup or re-eval)
            my @sort_vals;
            for my $ob (@{$ast->{order_by}}) {
                my $expr = $ob->{expr};
                # Try to find matching output column
                my $val = _eval_expr($expr, $row, undef);
                # Fallback: if expr is a column name matching an output col
                if (!defined $val && $expr->{type} eq 'COLUMN') {
                    my $col_idx = _find_col_idx(\@out_cols, $expr->{name});
                    $val = defined $col_idx ? $out_rows[$i][$col_idx] : undef;
                }
                push @sort_vals, $val;
            }
            push @keyed, { row => $out_rows[$i], keys => \@sort_vals, idx => $i };
        }
        @keyed = sort {
            for my $j (0 .. $#{$ast->{order_by}}) {
                my $dir = $ast->{order_by}[$j]{dir} // 'ASC';
                my $av  = $a->{keys}[$j];
                my $bv  = $b->{keys}[$j];
                # NULLs sort last
                if (!defined $av && !defined $bv) { next }
                if (!defined $av) { return 1 }
                if (!defined $bv) { return -1 }
                my $cmp = _sql_cmp($av, $bv);
                $cmp = -$cmp if $dir eq 'DESC';
                return $cmp if $cmp != 0;
            }
            return 0;
        } @keyed;
        @out_rows = map { $_->{row} } @keyed;
    }

    # ------------------------------------------------------------------
    # Stage 8: LIMIT / OFFSET
    # ------------------------------------------------------------------
    if (defined $ast->{offset}) {
        my $off = int($ast->{offset});
        splice(@out_rows, 0, $off) if $off > 0;
    }
    if (defined $ast->{limit}) {
        my $lim = int($ast->{limit});
        @out_rows = @out_rows[0 .. ($lim < @out_rows ? $lim - 1 : $#out_rows)];
    }

    return { columns => \@out_cols, rows => \@out_rows };
}

sub _find_col_idx {
    my ($cols, $name) = @_;
    for my $i (0 .. $#$cols) {
        return $i if $cols->[$i] eq $name;
    }
    return undef;
}

sub _has_aggregate {
    my ($cols) = @_;
    for my $col (@$cols) {
        return 1 if _expr_has_agg($col->{expr});
    }
    return 0;
}

sub _expr_has_agg {
    my ($node) = @_;
    return 0 unless defined $node;
    return 1 if $node->{type} eq 'AGG';
    for my $key (qw(left right expr arg lo hi)) {
        return 1 if exists $node->{$key} && _expr_has_agg($node->{$key});
    }
    return 0;
}

sub _scan_table {
    my ($ds, $table_name, $alias) = @_;
    my $schema = $ds->schema($table_name);
    my $raw    = $ds->scan($table_name);
    my @rows;
    for my $r (@$raw) {
        my %row;
        for my $col (@$schema) {
            $row{$col}            = $r->{$col};
            $row{"$alias.$col"}   = $r->{$col};
            $row{"$table_name.$col"} = $r->{$col};
        }
        push @rows, \%row;
    }
    return \@rows;
}

sub _apply_join {
    my ($jtype, $left, $right, $on) = @_;
    my @result;
    if ($jtype eq 'CROSS') {
        for my $l (@$left) {
            for my $r (@$right) {
                push @result, { %$l, %$r };
            }
        }
        return \@result;
    }
    # INNER / LEFT / RIGHT / FULL
    my %right_matched;
    for my $i (0 .. $#$left) {
        my $l = $left->[$i];
        my $matched = 0;
        for my $j (0 .. $#$right) {
            my $r    = $right->[$j];
            my $row  = { %$l, %$r };
            my $pass = !defined $on || do {
                my $v = eval { _eval_expr($on, $row, undef) };
                defined $v && $v;
            };
            if ($pass) {
                push @result, $row;
                $right_matched{$j} = 1;
                $matched = 1;
            }
        }
        if (!$matched && ($jtype eq 'LEFT' || $jtype eq 'FULL')) {
            # Left row with NULLs for right columns
            my %null_row = %$l;
            push @result, \%null_row;
        }
    }
    if ($jtype eq 'RIGHT' || $jtype eq 'FULL') {
        for my $j (0 .. $#$right) {
            next if $right_matched{$j};
            push @result, { %{$right->[$j]} };
        }
    }
    return \@result;
}

# ---------------------------------------------------------------------------
# InMemoryDataSource
# ---------------------------------------------------------------------------
#
# A simple data source backed by Perl arrayrefs of hashrefs.
# Used for testing and examples.

package CodingAdventures::SqlExecutionEngine::InMemoryDataSource;

sub new {
    my ($class, $tables) = @_;
    return bless { tables => $tables }, $class;
}

sub schema {
    my ($self, $table_name) = @_;
    die "Unknown table: $table_name\n" unless exists $self->{tables}{$table_name};
    my $rows = $self->{tables}{$table_name};
    return [] unless @$rows;
    return [ sort keys %{$rows->[0]} ];
}

sub scan {
    my ($self, $table_name) = @_;
    die "Unknown table: $table_name\n" unless exists $self->{tables}{$table_name};
    return $self->{tables}{$table_name};
}

1;
__END__

=head1 NAME

CodingAdventures::SqlExecutionEngine - SELECT-only SQL execution engine

=head1 SYNOPSIS

  use CodingAdventures::SqlExecutionEngine;
  use CodingAdventures::SqlExecutionEngine::InMemoryDataSource;

  my $ds = CodingAdventures::SqlExecutionEngine::InMemoryDataSource->new({
    employees => [
      { id => 1, name => 'Alice', dept => 'Engineering', salary => 95000 },
      { id => 2, name => 'Bob',   dept => 'Marketing',   salary => 72000 },
    ],
  });

  my ($ok, $result) = CodingAdventures::SqlExecutionEngine->execute(
    "SELECT name, salary FROM employees WHERE salary > 80000 ORDER BY salary DESC",
    $ds,
  );

  if ($ok) {
    print join(', ', @{$result->{columns}}), "\n";
    for my $row (@{$result->{rows}}) {
      print join(', ', map { defined $_ ? $_ : 'NULL' } @$row), "\n";
    }
  }

=head1 DESCRIPTION

Implements a complete SELECT-only SQL execution engine using a materialized
pipeline model.

=cut
