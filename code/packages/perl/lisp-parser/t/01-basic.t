use strict;
use warnings;
use Test2::V0;

use CodingAdventures::LispParser;
use CodingAdventures::LispParser::ASTNode;

# ============================================================================
# Helpers
# ============================================================================

# Recursively count nodes with a given rule_name.
sub count_nodes {
    my ($node, $rule_name) = @_;
    return 0 unless ref $node && $node->can('rule_name');
    my $n = ($node->rule_name eq $rule_name) ? 1 : 0;
    for my $child (@{ $node->children }) {
        $n += count_nodes($child, $rule_name);
    }
    return $n;
}

# Find first node with a given rule_name (depth-first).
sub find_node {
    my ($node, $rule_name) = @_;
    return undef unless ref $node && $node->can('rule_name');
    return $node if $node->rule_name eq $rule_name;
    for my $child (@{ $node->children }) {
        my $found = find_node($child, $rule_name);
        return $found if defined $found;
    }
    return undef;
}

# ============================================================================
# Root node
# ============================================================================

subtest 'parse returns an ASTNode' => sub {
    my $ast = CodingAdventures::LispParser->parse('42');
    ok( defined $ast, 'parse returns a defined value' );
    ok( ref($ast),    'parse returns a reference' );
};

subtest 'root node has rule_name program' => sub {
    my $ast = CodingAdventures::LispParser->parse('42');
    is( $ast->rule_name, 'program', 'root rule_name is "program"' );
};

subtest 'root node has children arrayref' => sub {
    my $ast = CodingAdventures::LispParser->parse('42');
    ok( ref($ast->children) eq 'ARRAY', 'children is an arrayref' );
    ok( scalar @{ $ast->children } >= 1, 'children is non-empty' );
};

# ============================================================================
# Empty program
# ============================================================================

subtest 'empty input parses to program with no children' => sub {
    my $ast = CodingAdventures::LispParser->parse('');
    is( $ast->rule_name,              'program', 'rule_name is program' );
    is( scalar @{ $ast->children }, 0,           'no children for empty input' );
};

subtest 'whitespace-only parses to empty program' => sub {
    my $ast = CodingAdventures::LispParser->parse("   \n\t  ");
    is( $ast->rule_name,              'program', 'rule_name is program' );
    is( scalar @{ $ast->children }, 0,           'no children' );
};

subtest 'comment-only parses to empty program' => sub {
    my $ast = CodingAdventures::LispParser->parse("; just a comment");
    is( $ast->rule_name,              'program', 'rule_name is program' );
    is( scalar @{ $ast->children }, 0,           'no children' );
};

# ============================================================================
# Atom parsing
# ============================================================================

subtest 'parse bare NUMBER' => sub {
    my $ast  = CodingAdventures::LispParser->parse('42');
    my $atom = find_node($ast, 'atom');
    ok( defined $atom, 'found atom node' );
    my $leaf = $atom->children->[0];
    ok( $leaf->is_leaf,              'child is a leaf' );
    is( $leaf->token->{type},  'NUMBER', 'token type is NUMBER' );
    is( $leaf->token->{value}, '42',     'token value is 42' );
};

subtest 'parse negative integer' => sub {
    my $ast  = CodingAdventures::LispParser->parse('-7');
    my $atom = find_node($ast, 'atom');
    ok( defined $atom, 'found atom node' );
    is( $atom->children->[0]->token->{value}, '-7', 'value is -7' );
};

subtest 'parse SYMBOL' => sub {
    my $ast  = CodingAdventures::LispParser->parse('define');
    my $atom = find_node($ast, 'atom');
    ok( defined $atom, 'found atom node' );
    is( $atom->children->[0]->token->{type},  'SYMBOL', 'type is SYMBOL' );
    is( $atom->children->[0]->token->{value}, 'define', 'value is define' );
};

subtest 'parse operator symbol +' => sub {
    my $ast  = CodingAdventures::LispParser->parse('+');
    my $atom = find_node($ast, 'atom');
    ok( defined $atom, 'found atom' );
    is( $atom->children->[0]->token->{value}, '+', 'value is +' );
};

subtest 'parse STRING' => sub {
    my $ast  = CodingAdventures::LispParser->parse('"hello"');
    my $atom = find_node($ast, 'atom');
    ok( defined $atom, 'found atom' );
    is( $atom->children->[0]->token->{type},  'STRING',  'type is STRING' );
    is( $atom->children->[0]->token->{value}, '"hello"', 'value is "hello"' );
};

# ============================================================================
# List parsing
# ============================================================================

subtest 'parse empty list ()' => sub {
    my $ast  = CodingAdventures::LispParser->parse('()');
    my $list = find_node($ast, 'list');
    ok( defined $list, 'found list node' );
    # list has: LPAREN, list_body, RPAREN
    is( scalar @{ $list->children }, 3, 'list has 3 children' );
};

subtest 'parse (42)' => sub {
    my $ast  = CodingAdventures::LispParser->parse('(42)');
    my $list = find_node($ast, 'list');
    ok( defined $list, 'found list node' );
    is( count_nodes($ast, 'atom'), 1, 'one atom' );
};

subtest 'parse (+ 1 2)' => sub {
    my $ast = CodingAdventures::LispParser->parse('(+ 1 2)');
    ok( defined find_node($ast, 'list'), 'found list' );
    is( count_nodes($ast, 'atom'), 3, 'three atoms: +, 1, 2' );
};

subtest 'parse (define x 42)' => sub {
    my $ast = CodingAdventures::LispParser->parse('(define x 42)');
    ok( defined find_node($ast, 'list'), 'found list' );
    is( count_nodes($ast, 'atom'), 3, 'three atoms: define, x, 42' );
};

subtest 'parse list of strings' => sub {
    my $ast = CodingAdventures::LispParser->parse('("a" "b" "c")');
    ok( defined find_node($ast, 'list'), 'found list' );
    is( count_nodes($ast, 'atom'), 3, 'three atoms' );
};

# ============================================================================
# Nested lists
# ============================================================================

subtest 'parse (car (cdr x))' => sub {
    my $ast = CodingAdventures::LispParser->parse('(car (cdr x))');
    is( count_nodes($ast, 'list'), 2, 'two list nodes' );
    is( count_nodes($ast, 'atom'), 3, 'three atoms: car, cdr, x' );
};

subtest 'parse (+ (* 2 3) 4)' => sub {
    my $ast = CodingAdventures::LispParser->parse('(+ (* 2 3) 4)');
    is( count_nodes($ast, 'list'), 2, 'two list nodes' );
    is( count_nodes($ast, 'atom'), 5, 'five atoms: +, *, 2, 3, 4' );
};

subtest 'parse lambda (lambda (x) (* x x))' => sub {
    my $ast = CodingAdventures::LispParser->parse('(lambda (x) (* x x))');
    is( count_nodes($ast, 'list'), 3, 'three lists' );
    # lambda, (x), (* x x): atoms are lambda, x, *, x, x = 5
    is( count_nodes($ast, 'atom'), 5, 'five atoms' );
};

subtest 'deeply nested (a (b (c (d))))' => sub {
    my $ast = CodingAdventures::LispParser->parse('(a (b (c (d))))');
    is( count_nodes($ast, 'list'), 4, 'four lists' );
    is( count_nodes($ast, 'atom'), 4, 'four atoms: a, b, c, d' );
};

# ============================================================================
# Quoted forms
# ============================================================================
#
# 'x is shorthand for (quote x).
# Grammar: quoted = QUOTE sexpr

subtest "parse 'x" => sub {
    my $ast = CodingAdventures::LispParser->parse("'x");
    my $q = find_node($ast, 'quoted');
    ok( defined $q, 'found quoted node' );
    # quoted has two children: QUOTE token leaf + sexpr
    is( scalar @{ $q->children }, 2, 'quoted has 2 children' );
};

subtest "quoted node first child is QUOTE token" => sub {
    my $ast = CodingAdventures::LispParser->parse("'x");
    my $q   = find_node($ast, 'quoted');
    my $first = $q->children->[0];
    ok( $first->is_leaf,                   'first child is a leaf' );
    is( $first->token->{type}, 'QUOTE', 'first child is QUOTE token' );
};

subtest "'42 — quoted number" => sub {
    my $ast = CodingAdventures::LispParser->parse("'42");
    ok( defined find_node($ast, 'quoted'), 'found quoted node' );
    is( count_nodes($ast, 'atom'), 1, 'one atom under the quote' );
};

subtest "'(1 2 3) — quoted list" => sub {
    my $ast = CodingAdventures::LispParser->parse("'(1 2 3)");
    ok( defined find_node($ast, 'quoted'), 'found quoted node' );
    ok( defined find_node($ast, 'list'),   'found list node'   );
    is( count_nodes($ast, 'atom'), 3, 'three atoms' );
};

subtest "''x — nested quote" => sub {
    my $ast = CodingAdventures::LispParser->parse("''x");
    is( count_nodes($ast, 'quoted'), 2, 'two quoted nodes' );
};

# ============================================================================
# Dotted pairs
# ============================================================================
#
# Grammar: list_body = [ sexpr { sexpr } [ DOT sexpr ] ]

subtest 'parse (a . b)' => sub {
    my $ast = CodingAdventures::LispParser->parse('(a . b)');
    ok( defined find_node($ast, 'list'),      'found list node'      );
    ok( defined find_node($ast, 'list_body'), 'found list_body node' );
    is( count_nodes($ast, 'atom'), 2, 'two atoms: a, b' );
};

subtest 'parse (1 2 . 3)' => sub {
    # An improper list — the cdr of the last cons cell is 3, not nil.
    my $ast = CodingAdventures::LispParser->parse('(1 2 . 3)');
    ok( defined find_node($ast, 'list'), 'found list' );
    is( count_nodes($ast, 'atom'), 3, 'three atoms: 1, 2, 3' );
};

subtest "parse alist '((a . 1) (b . 2))" => sub {
    my $ast = CodingAdventures::LispParser->parse("'((a . 1) (b . 2))");
    ok( defined find_node($ast, 'quoted'), 'found quoted'     );
    is( count_nodes($ast, 'list'), 3, 'three lists (outer + two pairs)' );
    is( count_nodes($ast, 'atom'), 4, 'four atoms: a, 1, b, 2'         );
};

# ============================================================================
# Multi-expression programs
# ============================================================================

subtest 'two sequential atoms' => sub {
    my $ast = CodingAdventures::LispParser->parse('1 2');
    is( $ast->rule_name,              'program', 'root is program' );
    is( count_nodes($ast, 'sexpr'),  2,          'two sexpr nodes' );
};

subtest '(define x 42) (display x)' => sub {
    my $ast = CodingAdventures::LispParser->parse('(define x 42) (display x)');
    is( $ast->rule_name,             'program', 'root is program' );
    is( count_nodes($ast, 'list'), 2,            'two list nodes'  );
};

subtest 'three top-level definitions' => sub {
    my $src = "(define x 1)\n(define y 2)\n(+ x y)";
    my $ast = CodingAdventures::LispParser->parse($src);
    is( count_nodes($ast, 'list'), 3, 'three list nodes' );
};

subtest 'program with comments between expressions' => sub {
    my $src = <<'END_LISP';
;; Define a variable
(define x 42)
;; Display it
(display x)
END_LISP
    my $ast = CodingAdventures::LispParser->parse($src);
    is( $ast->rule_name,             'program', 'root is program' );
    is( count_nodes($ast, 'list'), 2,            'two list nodes'  );
};

subtest 'fibonacci function' => sub {
    # The classic Scheme fibonacci definition.
    my $src = <<'END_LISP';
(define (fib n)
  (if (< n 2)
      n
      (+ (fib (- n 1))
         (fib (- n 2)))))
END_LISP
    my $ast = CodingAdventures::LispParser->parse($src);
    is( $ast->rule_name, 'program', 'root is program' );
    ok( count_nodes($ast, 'list') >= 7, 'many lists in fib' );
    ok( count_nodes($ast, 'atom') >= 10, 'many atoms in fib' );
};

subtest 'let bindings' => sub {
    # (let ((x 1) (y 2)) (+ x y))
    my $ast = CodingAdventures::LispParser->parse('(let ((x 1) (y 2)) (+ x y))');
    ok( count_nodes($ast, 'list') >= 4, 'at least 4 lists in let' );
};

subtest 'program with quoted data' => sub {
    my $src = "(define colors '(red green blue))\n(car colors)";
    my $ast = CodingAdventures::LispParser->parse($src);
    is( count_nodes($ast, 'list'),   3, 'three lists (two top-level plus list inside quoted form)' );
    is( count_nodes($ast, 'quoted'), 1, 'one quoted form' );
};

# ============================================================================
# ASTNode methods
# ============================================================================

subtest 'ASTNode new and accessors' => sub {
    my $node = CodingAdventures::LispParser::ASTNode->new(
        rule_name => 'program',
        children  => [],
        is_leaf   => 0,
    );
    is( $node->rule_name, 'program', 'rule_name accessor' );
    is( $node->is_leaf,   0,         'is_leaf accessor (false)' );
    ok( ref($node->children) eq 'ARRAY', 'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok  = { type => 'NUMBER', value => '42', line => 1, col => 1 };
    my $leaf = CodingAdventures::LispParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => $tok,
    );
    is( $leaf->is_leaf,          1,        'is_leaf is true' );
    is( $leaf->token->{type},    'NUMBER', 'token type' );
    is( $leaf->token->{value},   '42',     'token value' );
};

subtest 'leaf nodes have empty children' => sub {
    my $tok  = { type => 'SYMBOL', value => '+', line => 1, col => 1 };
    my $leaf = CodingAdventures::LispParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => $tok,
    );
    is( scalar @{ $leaf->children }, 0, 'leaf has no children' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'unterminated list dies' => sub {
    ok(
        dies { CodingAdventures::LispParser->parse('(define x') },
        'unterminated list causes die'
    );
};

subtest 'unmatched RPAREN dies' => sub {
    ok(
        dies { CodingAdventures::LispParser->parse(')') },
        'lone ) causes die'
    );
};

subtest 'unexpected character dies' => sub {
    ok(
        dies { CodingAdventures::LispParser->parse('@bad') },
        'unexpected @ causes die'
    );
};

subtest 'unterminated string dies at lexer' => sub {
    ok(
        dies { CodingAdventures::LispParser->parse('"unterminated') },
        'unterminated string causes die'
    );
};

done_testing;
