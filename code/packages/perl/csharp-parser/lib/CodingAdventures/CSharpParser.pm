package CodingAdventures::CSharpParser;

# ============================================================================
# CodingAdventures::CSharpParser — Hand-written recursive-descent C# parser
# ============================================================================
#
# This module parses a subset of C# into an Abstract Syntax Tree (AST).
# The parser is hand-written using the recursive-descent technique: each
# grammar rule is encoded as one Perl method, and rules call each other
# recursively.
#
# # What language do we parse?
# =============================
#
# A practical subset covering the constructs most commonly taught first:
#
#   int x = 5;                           — variable declaration
#   x = 10;                              — assignment
#   42;                                  — expression statement
#   public int Add(int a, int b) { return a+b; }  — method declaration
#   if (x > 0) { … } else { … }         — if/else statement
#   for (int i = 0; i < 10; i++) { … }  — for loop
#   foreach (var item in list) { … }    — foreach loop
#   return x + 1;                        — return statement
#   f(a, b)                              — method call
#   1 + 2 * 3                            — binary expression (precedence)
#   a ?? b                               — null-coalescing (C# 2.0+)
#   obj?.Member                          — null-conditional access (C# 6.0+)
#
# # Operator precedence (lowest to highest)
# ==========================================
#
#   Null-coalescing  →  ??
#   Equality         →  ==  !=
#   Comparison       →  <  >  <=  >=
#   Additive         →  +  -
#   Multiplicative   →  *  /
#   Unary            →  !  -  (unary)
#   Primary          →  literals, identifiers, (expr), call, null-conditional
#
# # AST node types (rule_name values)
# ====================================
#
#   program          — root; contains a list of statement nodes
#   statement        — wrapper for one statement
#   var_declaration  — type name = expr ;
#   assignment_stmt  — name = expr ;
#   expression_stmt  — expr ;
#   if_stmt          — if (cond) block [else block/if_stmt]
#   for_stmt         — for (init; cond; update) block
#   foreach_stmt     — foreach (type name in expr) block
#   return_stmt      — return [expr] ;
#   block            — { statement* }
#   expression       — full expression (lowest-precedence entry point)
#   binary_expr      — left op right
#   unary_expr       — op expr  (unary ! or -)
#   null_coalesce    — left ?? right
#   call_expr        — callee(args)
#   primary          — literal, identifier, grouped expression
#   param_list       — comma-separated parameter names
#   arg_list         — comma-separated argument expressions
#   for_init         — initializer part of for header
#   for_condition    — condition part of for header
#   for_update       — update part of for header
#   token            — leaf node wrapping a single lexer token
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::CSharpLexer;
use CodingAdventures::CSharpParser::ASTNode;

my %TYPE_KEYWORDS = map { $_ => 1 } qw(
    BOOL BYTE CHAR DECIMAL DOUBLE DYNAMIC FLOAT INT LONG NINT NUINT
    OBJECT SBYTE SHORT STRING UINT ULONG USHORT
);

# ============================================================================
# Constructor
# ============================================================================

# --- new($source, $version) ---------------------------------------------------
#
# Tokenize `$source` with CSharpLexer (using the specified $version)
# and return a ready-to-parse parser.
#
# $version is optional. Valid values: "1.0", "2.0", "3.0", "4.0", "5.0",
# "6.0", "7.0", "8.0", "9.0", "10.0", "11.0", "12.0",
# or undef/"" for C# 12.0 (default).

sub new {
    my ($class, $source, $version) = @_;
    my $tokens = CodingAdventures::CSharpLexer->tokenize($source, $version);
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

# Peek N positions ahead from the current position (0 = current).
sub _peek_ahead {
    my ($self, $n) = @_;
    $n //= 0;
    return $self->{_tokens}[ $self->{_pos} + $n ]
        // { type => 'EOF', value => '', line => 0, col => 0 };
}

# Consume and return the current token.
sub _advance {
    my ($self) = @_;
    my $tok = $self->_peek();
    $self->{_pos}++ unless $tok->{type} eq 'EOF';
    return $tok;
}

# Expect a specific token type; die with a helpful message on mismatch.
sub _expect {
    my ($self, $type) = @_;
    my $tok = $self->_peek();
    unless ($tok->{type} eq $type) {
        die sprintf(
            "CodingAdventures::CSharpParser: parse error at line %d col %d: "
          . "expected %s but got %s ('%s')\n",
            $tok->{line}, $tok->{col}, $type, $tok->{type}, $tok->{value}
        );
    }
    return $self->_advance();
}

# Return 1 if current token matches the given type (and optionally value).
sub _check {
    my ($self, $type, $value) = @_;
    my $tok = $self->_peek();
    return 0 unless $tok->{type} eq $type;
    return 1 unless defined $value;
    return $tok->{value} eq $value;
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
    return CodingAdventures::CSharpParser::ASTNode->new_leaf($tok);
}

# Create an inner ASTNode.
sub _node {
    my ($self, $rule_name, @children) = @_;
    return CodingAdventures::CSharpParser::ASTNode->new($rule_name, \@children);
}

sub _is_type_token {
    my ($self, $tok) = @_;
    return 0 unless $tok;
    return 1 if $tok->{type} eq 'NAME';
    return 1 if $TYPE_KEYWORDS{$tok->{type}};
    return 0;
}

sub _consume_type_token {
    my ($self) = @_;
    my $tok = $self->_peek();
    unless ($self->_is_type_token($tok)) {
        die sprintf(
            "CodingAdventures::CSharpParser: parse error at line %d col %d: "
          . "expected type name but got %s ('%s')\n",
            $tok->{line}, $tok->{col}, $tok->{type}, $tok->{value}
        );
    }
    return $self->_advance();
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

# program = { statement } ;
sub _parse_program {
    my ($self) = @_;
    my @children;
    while (!$self->_check('EOF')) {
        push @children, $self->_parse_statement();
    }
    return $self->_node('program', @children);
}

# statement = var_declaration | if_stmt | for_stmt | foreach_stmt
#           | return_stmt | block | assignment_stmt | expression_stmt ;
#
# Statement dispatch: inspect the current token to select the right rule.
# For C#, type keywords (int, string, etc.) or NAME followed by NAME
# indicates a variable declaration.
sub _parse_statement {
    my ($self) = @_;
    my $tok = $self->_peek();
    my $type = $tok->{type};

    # if statement
    if ($type eq 'IF') {
        return $self->_node('statement', $self->_parse_if_stmt());
    }

    # for loop
    if ($type eq 'FOR') {
        return $self->_node('statement', $self->_parse_for_stmt());
    }

    # foreach loop — C# has a dedicated foreach statement
    # foreach (type name in expr) { ... }
    if ($type eq 'FOREACH') {
        return $self->_node('statement', $self->_parse_foreach_stmt());
    }

    # return statement
    if ($type eq 'RETURN') {
        return $self->_node('statement', $self->_parse_return_stmt());
    }

    # block
    if ($type eq 'LBRACE') {
        return $self->_node('statement', $self->_parse_block());
    }

    # Variable declaration: NAME NAME EQUALS ... (type name = expr ;)
    # Also handles keyword types like INT, STRING, BOOL, etc.
    if ($self->_is_type_token($tok)) {
        my $next = $self->_peek_ahead(1);
        # If a type token is followed by NAME, it's a variable declaration.
        if ($next->{type} eq 'NAME') {
            return $self->_node('statement', $self->_parse_var_declaration());
        }
    }

    if ($type eq 'NAME') {
        my $next = $self->_peek_ahead(1);
        # If NAME is followed by EQUALS, it's an assignment.
        if ($next->{type} eq 'EQUALS') {
            return $self->_node('statement', $self->_parse_assignment_stmt());
        }
    }

    # expression statement (default — catches method calls, literals, etc.)
    return $self->_node('statement', $self->_parse_expression_stmt());
}

# var_declaration = NAME NAME EQUALS expression SEMICOLON ;
#
# Example:  int x = 5;   string y = "hello";   var z = new Foo();
sub _parse_var_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_consume_type_token());
    push @ch, $self->_leaf($self->_expect('NAME'));      # variable name
    push @ch, $self->_leaf($self->_expect('EQUALS'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('var_declaration', @ch);
}

# assignment_stmt = NAME EQUALS expression SEMICOLON ;
#
# Example:  x = 10;
sub _parse_assignment_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('EQUALS'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('assignment_stmt', @ch);
}

# expression_stmt = expression SEMICOLON ;
#
# Example:  42;   f(x);
sub _parse_expression_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('expression_stmt', @ch);
}

# if_stmt = IF LPAREN expression RPAREN block [ ELSE ( if_stmt | block ) ] ;
#
# Example:  if (x > 0) { return x; } else { return 0; }
sub _parse_if_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('IF'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_parse_block();
    if ($self->_check('ELSE')) {
        push @ch, $self->_leaf($self->_advance());
        if ($self->_check('IF')) {
            push @ch, $self->_parse_if_stmt();       # else if chain
        } else {
            push @ch, $self->_parse_block();          # else block
        }
    }
    return $self->_node('if_stmt', @ch);
}

# for_stmt = FOR LPAREN for_header RPAREN block ;
sub _parse_for_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('FOR'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));

    # --- Initializer ---
    if ($self->_is_type_token($self->_peek()) && $self->_peek_ahead(1)->{type} eq 'NAME') {
        # type name = expr
        my @init_ch;
        push @init_ch, $self->_leaf($self->_consume_type_token());
        push @init_ch, $self->_leaf($self->_expect('NAME'));
        push @init_ch, $self->_leaf($self->_expect('EQUALS'));
        push @init_ch, $self->_parse_expression();
        push @ch, $self->_node('for_init', @init_ch);
    } elsif (!$self->_check('SEMICOLON')) {
        push @ch, $self->_node('for_init', $self->_parse_expression());
    } else {
        push @ch, $self->_node('for_init');
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));

    # --- Condition ---
    if (!$self->_check('SEMICOLON')) {
        push @ch, $self->_node('for_condition', $self->_parse_expression());
    } else {
        push @ch, $self->_node('for_condition');
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));

    # --- Update expression ---
    if (!$self->_check('RPAREN')) {
        my $next = $self->_peek_ahead(1);
        if ($self->_check('NAME') && $next && $next->{type} eq 'EQUALS') {
            my @upd_ch;
            push @upd_ch, $self->_leaf($self->_advance());    # NAME
            push @upd_ch, $self->_leaf($self->_advance());    # EQUALS
            push @upd_ch, $self->_parse_expression();
            push @ch, $self->_node('for_update', $self->_node('assign_expr', @upd_ch));
        } else {
            push @ch, $self->_node('for_update', $self->_parse_expression());
        }
    } else {
        push @ch, $self->_node('for_update');
    }

    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_parse_block();
    return $self->_node('for_stmt', @ch);
}

# foreach_stmt = FOREACH LPAREN NAME NAME IN expression RPAREN block ;
#
# C# foreach is a unique construct not present in Java (Java 5 enhanced for
# is similar but uses `for`, not `foreach`). It iterates over any IEnumerable.
#
# Example:  foreach (string item in list) { Console.WriteLine(item); }
sub _parse_foreach_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('FOREACH'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_leaf($self->_consume_type_token());  # element type
    push @ch, $self->_leaf($self->_expect('NAME'));         # element variable name
    push @ch, $self->_leaf($self->_expect('IN'));           # in keyword
    push @ch, $self->_parse_expression();                   # iterable expression
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_parse_block();
    return $self->_node('foreach_stmt', @ch);
}

# return_stmt = RETURN [ expression ] SEMICOLON ;
sub _parse_return_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('RETURN'));
    if (!$self->_check('SEMICOLON') && !$self->_check('RBRACE') && !$self->_check('EOF')) {
        push @ch, $self->_parse_expression();
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('return_stmt', @ch);
}

# block = LBRACE { statement } RBRACE ;
sub _parse_block {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LBRACE'));
    while (!$self->_check('RBRACE') && !$self->_check('EOF')) {
        push @ch, $self->_parse_statement();
    }
    push @ch, $self->_leaf($self->_expect('RBRACE'));
    return $self->_node('block', @ch);
}

# ============================================================================
# Expression parsing — precedence climbing
# ============================================================================
#
# The precedence hierarchy (lowest to highest) is:
#
#   null_coalesce   ??
#   equality        ==  !=
#   comparison      <  >  <=  >=
#   additive        +  -
#   multiplicative  *  /
#   unary           !  - (unary)
#   primary         literals, names, calls, (expr), null-conditional
#
# Each level is one method that calls the next higher level.

# expression = null_coalesce ;
sub _parse_expression {
    my ($self) = @_;
    my $inner = $self->_parse_null_coalesce();
    return $self->_node('expression', $inner);
}

# null_coalesce = equality { ?? equality } ;
#
# The null-coalescing operator (??) is unique to C#.
# It evaluates to the left operand if it is non-null; otherwise the right.
# This is left-associative: a ?? b ?? c == (a ?? b) ?? c.
sub _parse_null_coalesce {
    my ($self) = @_;
    my $left = $self->_parse_equality();
    while ($self->_check('NULL_COALESCE') || $self->_check('QUESTION_QUESTION')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_equality();
        $left = $self->_node('null_coalesce', $left, $op, $right);
    }
    return $left;
}

# equality = comparison { ( EQUALS_EQUALS | NOT_EQUALS ) comparison } ;
sub _parse_equality {
    my ($self) = @_;
    my $left = $self->_parse_comparison();
    while ($self->_check('EQUALS_EQUALS') || $self->_check('NOT_EQUALS')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_comparison();
        $left = $self->_node('binary_expr', $left, $op, $right);
    }
    return $left;
}

# comparison = additive { ( LESS_THAN | GREATER_THAN
#                          | LESS_EQUALS | GREATER_EQUALS ) additive } ;
sub _parse_comparison {
    my ($self) = @_;
    my $left = $self->_parse_additive();
    while ($self->_check('LESS_THAN') || $self->_check('GREATER_THAN')
           || $self->_check('LESS_EQUALS') || $self->_check('GREATER_EQUALS')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_additive();
        $left = $self->_node('binary_expr', $left, $op, $right);
    }
    return $left;
}

# additive = multiplicative { ( PLUS | MINUS ) multiplicative } ;
sub _parse_additive {
    my ($self) = @_;
    my $left = $self->_parse_multiplicative();
    while ($self->_check('PLUS') || $self->_check('MINUS')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_multiplicative();
        $left = $self->_node('binary_expr', $left, $op, $right);
    }
    return $left;
}

# multiplicative = unary { ( STAR | SLASH ) unary } ;
sub _parse_multiplicative {
    my ($self) = @_;
    my $left = $self->_parse_unary();
    while ($self->_check('STAR') || $self->_check('SLASH')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_unary();
        $left = $self->_node('binary_expr', $left, $op, $right);
    }
    return $left;
}

# unary = ( BANG | MINUS ) unary | primary ;
sub _parse_unary {
    my ($self) = @_;
    if ($self->_check('BANG') || $self->_check('MINUS')) {
        my $op   = $self->_leaf($self->_advance());
        my $expr = $self->_parse_unary();
        return $self->_node('unary_expr', $op, $expr);
    }
    return $self->_parse_primary();
}

# primary = NUMBER | STRING | TRUE | FALSE | NULL
#         | NAME [ LPAREN arg_list RPAREN ]
#         | LPAREN expression RPAREN
#
# After any primary, we check for a null-conditional member access (?.)
# and chain it as needed.
sub _parse_primary {
    my ($self) = @_;
    my $tok = $self->_peek();
    my $type = $tok->{type};

    # --- Numeric literal ---
    if ($type eq 'NUMBER') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # --- String literal ---
    if ($type eq 'STRING') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # --- Boolean literals ---
    if ($type eq 'TRUE' || $type eq 'FALSE') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # --- null literal ---
    if ($type eq 'NULL') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # --- Identifier or method call ---
    if ($type eq 'NAME') {
        my $name_leaf = $self->_leaf($self->_advance());
        if ($self->_check('LPAREN')) {
            my @ch = ($name_leaf);
            push @ch, $self->_leaf($self->_advance());       # (
            push @ch, $self->_parse_arg_list();
            push @ch, $self->_leaf($self->_expect('RPAREN'));
            return $self->_node('call_expr', @ch);
        }
        return $self->_node('primary', $name_leaf);
    }

    # --- Parenthesized expression ---
    if ($type eq 'LPAREN') {
        my $lparen = $self->_leaf($self->_advance());
        my $inner  = $self->_parse_expression();
        my $rparen = $self->_leaf($self->_expect('RPAREN'));
        return $self->_node('primary', $lparen, $inner, $rparen);
    }

    die sprintf(
        "CodingAdventures::CSharpParser: unexpected token '%s' (type %s) "
      . "at line %d col %d\n",
        $tok->{value}, $tok->{type}, $tok->{line}, $tok->{col}
    );
}

# arg_list = [ expression { COMMA expression } ] ;
sub _parse_arg_list {
    my ($self) = @_;
    my @ch;
    if (!$self->_check('RPAREN')) {
        push @ch, $self->_parse_expression();
        while ($self->_check('COMMA')) {
            push @ch, $self->_leaf($self->_advance());
            push @ch, $self->_parse_expression();
        }
    }
    return $self->_node('arg_list', @ch);
}

# ============================================================================
# Class-method convenience wrappers
# ============================================================================

# --- parse_csharp($source, $version) -----------------------------------------
#
# Convenience class method: tokenize and parse in one call.
# Returns the root ASTNode. Dies on error.

sub parse_csharp {
    my ($class, $source, $version) = @_;
    my $parser = $class->new($source, $version);
    return $parser->parse();
}

# --- new_csharp_parser($source, $version) ------------------------------------
#
# Alias for new(). Provided so callers can use a named constructor without
# the OO -> syntax.

sub new_csharp_parser {
    my ($class, $source, $version) = @_;
    return $class->new($source, $version);
}

1;

__END__

=head1 NAME

CodingAdventures::CSharpParser - Hand-written recursive-descent C# parser

=head1 SYNOPSIS

    use CodingAdventures::CSharpParser;

    # Object-oriented
    my $parser = CodingAdventures::CSharpParser->new("int x = 5;");
    my $ast    = $parser->parse();
    print $ast->rule_name;   # "program"

    # Convenience class method
    my $ast = CodingAdventures::CSharpParser->parse_csharp("int y = x + 1;");

    # Version-specific
    my $ast = CodingAdventures::CSharpParser->parse_csharp('a ?? b;', '2.0');

=head1 DESCRIPTION

A hand-written recursive-descent parser for a practical subset of C#.
Tokenizes input with C<CodingAdventures::CSharpLexer> and builds an
Abstract Syntax Tree (AST) of C<CodingAdventures::CSharpParser::ASTNode>
nodes.

Supported constructs: variable declarations, assignments, expression statements,
if/else statements, for loops, foreach loops, return statements, blocks,
method calls, and full expression parsing with correct operator precedence
(null-coalesce > equality > comparison > additive > multiplicative > unary >
primary).

C#-specific features beyond basic Java parsing:

=over 4

=item C<null_coalesce> — the C<??> operator (C# 2.0+)

Parses left ?? right as a C<null_coalesce> AST node.

=item C<foreach_stmt> — the C<foreach> loop (C# only)

Parses foreach (type name in expr) { ... }

=back

=head1 METHODS

=head2 new($source, $version)

Tokenize C<$source> with C<CSharpLexer> and return a parser instance.

=head2 parse()

Parse and return the root AST node (rule_name C<"program">). Dies on error.

=head2 parse_csharp($source, $version)

Class method — tokenize and parse in one call. Returns the root ASTNode.

=head2 new_csharp_parser($source, $version)

Alias for C<new()>.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
