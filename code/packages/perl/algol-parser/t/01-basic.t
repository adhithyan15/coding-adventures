use strict;
use warnings;
use Test2::V0;

use CodingAdventures::AlgolParser;
use CodingAdventures::AlgolParser::ASTNode;

# ============================================================================
# Helper: find first node with given rule_name (depth-first)
# ============================================================================

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
# Helper: count all nodes with given rule_name (depth-first)
# ============================================================================

sub count_nodes {
    my ($node, $rule_name) = @_;
    return 0 unless ref $node && $node->can('rule_name');
    my $n = ($node->rule_name eq $rule_name) ? 1 : 0;
    for my $child (@{ $node->children }) {
        $n += count_nodes($child, $rule_name);
    }
    return $n;
}

# ============================================================================
# Helper: collect token values of given type (depth-first)
# ============================================================================

sub find_tokens_of_type {
    my ($node, $type) = @_;
    my @results;
    return @results unless ref $node && $node->can('is_leaf');
    if ($node->is_leaf && $node->token->{type} eq $type) {
        push @results, $node->token->{value};
    }
    for my $child (@{ $node->children }) {
        push @results, find_tokens_of_type($child, $type);
    }
    return @results;
}

# ============================================================================
# Root node
# ============================================================================

subtest 'parse returns an ASTNode' => sub {
    my $ast = CodingAdventures::AlgolParser->parse('begin end');
    ok( defined $ast, 'parse returns a defined value' );
    ok( ref($ast),    'parse returns a reference' );
};

subtest 'root node has rule_name program' => sub {
    my $ast = CodingAdventures::AlgolParser->parse('begin end');
    is( $ast->rule_name, 'program', 'root rule_name is "program"' );
};

subtest 'root node has children' => sub {
    my $ast = CodingAdventures::AlgolParser->parse('begin end');
    ok( ref($ast->children) eq 'ARRAY', 'children is an arrayref' );
    ok( scalar @{ $ast->children } >= 1, 'children is non-empty' );
};

# ============================================================================
# Minimal program: begin end (empty block)
# ============================================================================

subtest 'minimal program: begin end' => sub {
    my $ast = CodingAdventures::AlgolParser->parse('begin end');
    is( $ast->rule_name, 'program', 'root is program' );
    my $block = find_node($ast, 'block');
    ok( defined $block, 'found block node' );
};

# ============================================================================
# Minimal program with declaration and assignment
# ============================================================================

subtest 'minimal program: begin integer x; x := 42 end' => sub {
    my $ast = CodingAdventures::AlgolParser->parse('begin integer x; x := 42 end');
    is( $ast->rule_name, 'program', 'root is program' );

    # Should have a block
    my $block = find_node($ast, 'block');
    ok( defined $block, 'found block node' );

    # Should have a type_decl
    my $type_decl = find_node($ast, 'type_decl');
    ok( defined $type_decl, 'found type_decl node' );

    # Should have an assign_stmt
    my $assign = find_node($ast, 'assign_stmt');
    ok( defined $assign, 'found assign_stmt node' );
};

# ============================================================================
# Type declarations
# ============================================================================

subtest 'type_decl: integer x' => sub {
    my $ast = CodingAdventures::AlgolParser->parse('begin integer x; x := 0 end');
    my $type_decl = find_node($ast, 'type_decl');
    ok( defined $type_decl, 'found type_decl' );
    # type_decl has a type child and an ident_list child
    is( scalar @{ $type_decl->children }, 2, 'type_decl has 2 children (type + ident_list)' );
};

subtest 'type_decl: real x, y' => sub {
    my $ast = CodingAdventures::AlgolParser->parse('begin real x, y; x := 0 end');
    my $ident_list = find_node($ast, 'ident_list');
    ok( defined $ident_list, 'found ident_list' );
    # ident_list: IDENT COMMA IDENT = 3 children
    is( scalar @{ $ident_list->children }, 3, 'ident_list has 3 children for x, y' );
};

subtest 'type_decl: boolean flag' => sub {
    my $ast = CodingAdventures::AlgolParser->parse('begin boolean flag; flag := true end');
    my $type_decl = find_node($ast, 'type_decl');
    ok( defined $type_decl, 'found type_decl for boolean' );
};

# ============================================================================
# Assignment statements
# ============================================================================

subtest 'simple assignment: x := 42' => sub {
    my $ast = CodingAdventures::AlgolParser->parse('begin integer x; x := 42 end');
    my $assign = find_node($ast, 'assign_stmt');
    ok( defined $assign, 'found assign_stmt' );
    # assign_stmt has: left_part + expression
    ok( scalar @{ $assign->children } >= 2, 'assign_stmt has children' );
};

subtest 'assignment with arithmetic: x := a + b * c' => sub {
    my $ast = CodingAdventures::AlgolParser->parse(
        'begin integer x, a, b, c; x := a + b * c end'
    );
    my $assign = find_node($ast, 'assign_stmt');
    ok( defined $assign, 'found assign_stmt' );
    # Should have term nodes (for multiplication) inside
    my $term = find_node($ast, 'term');
    ok( defined $term, 'found term node (for b * c)' );
};

subtest 'assignment: x := 3.14' => sub {
    my $ast = CodingAdventures::AlgolParser->parse('begin real x; x := 3.14 end');
    my $assign = find_node($ast, 'assign_stmt');
    ok( defined $assign, 'found assign_stmt for real literal' );
    # Find the REAL_LIT token
    my @reals = find_tokens_of_type($ast, 'REAL_LIT');
    is( scalar @reals, 1,      'one REAL_LIT token' );
    is( $reals[0],     '3.14', 'REAL_LIT value is 3.14' );
};

# ============================================================================
# If/then/else
# ============================================================================

subtest 'if/then: if x = 0 then x := 1' => sub {
    my $ast = CodingAdventures::AlgolParser->parse(
        'begin integer x; x := 0; if x = 0 then x := 1 end'
    );
    my $cond = find_node($ast, 'cond_stmt');
    ok( defined $cond, 'found cond_stmt' );
    # cond_stmt children: IF bool_expr THEN unlabeled_stmt = 4
    ok( scalar @{ $cond->children } >= 4, 'cond_stmt has IF, bool_expr, THEN, body' );
};

subtest 'if/then/else: if x = 0 then x := 1 else x := 2' => sub {
    my $ast = CodingAdventures::AlgolParser->parse(
        'begin integer x; if x = 0 then x := 1 else x := 2 end'
    );
    my $cond = find_node($ast, 'cond_stmt');
    ok( defined $cond, 'found cond_stmt' );
    # cond_stmt with else: IF bool_expr THEN unlabeled_stmt ELSE statement = 6
    is( scalar @{ $cond->children }, 6, 'cond_stmt has 6 children (with else)' );
};

subtest 'if/then with boolean literal' => sub {
    my $ast = CodingAdventures::AlgolParser->parse(
        'begin integer x; if true then x := 1 end'
    );
    my $cond = find_node($ast, 'cond_stmt');
    ok( defined $cond, 'found cond_stmt' );
};

# ============================================================================
# Arithmetic expressions
# ============================================================================

subtest 'unary minus: x := -42' => sub {
    my $ast = CodingAdventures::AlgolParser->parse('begin integer x; x := -42 end');
    my $simple_arith = find_node($ast, 'simple_arith');
    ok( defined $simple_arith, 'found simple_arith' );
    # First child should be MINUS (unary)
    ok( $simple_arith->children->[0]->is_leaf, 'first child is leaf' );
    is( $simple_arith->children->[0]->token->{type}, 'MINUS', 'unary minus token' );
};

subtest 'exponentiation: x := a ** 2' => sub {
    my $ast = CodingAdventures::AlgolParser->parse(
        'begin integer x, a; x := a ** 2 end'
    );
    my $factor = find_node($ast, 'factor');
    ok( defined $factor, 'found factor node' );
    # factor has: primary POWER primary = 3 children
    is( scalar @{ $factor->children }, 3, 'factor has 3 children (a ** 2)' );
};

subtest 'div and mod operators' => sub {
    my $ast = CodingAdventures::AlgolParser->parse(
        'begin integer x, a, b; x := a div b end'
    );
    my $term = find_node($ast, 'term');
    ok( defined $term, 'found term node' );
    my @divs = find_tokens_of_type($ast, 'DIV');
    is( scalar @divs, 1, 'one DIV token' );
};

# ============================================================================
# For loop
# ============================================================================

subtest 'for loop: step/until' => sub {
    my $ast = CodingAdventures::AlgolParser->parse(
        'begin integer i, s; s := 0; for i := 1 step 1 until 10 do s := s + i end'
    );
    my $for_stmt = find_node($ast, 'for_stmt');
    ok( defined $for_stmt, 'found for_stmt' );
    my $for_elem = find_node($ast, 'for_elem_step');
    ok( defined $for_elem, 'found for_elem_step' );
};

subtest 'for loop: simple value' => sub {
    my $ast = CodingAdventures::AlgolParser->parse(
        'begin integer x; for x := 5 do x := x + 1 end'
    );
    my $for_stmt = find_node($ast, 'for_stmt');
    ok( defined $for_stmt, 'found for_stmt' );
};

# ============================================================================
# ASTNode methods
# ============================================================================

subtest 'ASTNode new and accessors' => sub {
    my $node = CodingAdventures::AlgolParser::ASTNode->new(
        rule_name => 'block',
        children  => [],
        is_leaf   => 0,
    );
    is( $node->rule_name, 'block', 'rule_name accessor' );
    is( $node->is_leaf,   0,       'is_leaf accessor (false)' );
    ok( ref($node->children) eq 'ARRAY', 'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok  = { type => 'INTEGER_LIT', value => '42', line => 1, col => 1 };
    my $leaf = CodingAdventures::AlgolParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => $tok,
    );
    is( $leaf->is_leaf,          1,            'is_leaf is true' );
    is( $leaf->token->{type},    'INTEGER_LIT', 'token type' );
    is( $leaf->token->{value},   '42',          'token value' );
};

subtest 'leaf nodes have empty children by default' => sub {
    my $tok  = { type => 'BEGIN', value => 'begin', line => 1, col => 1 };
    my $leaf = CodingAdventures::AlgolParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => $tok,
    );
    is( scalar @{ $leaf->children }, 0, 'leaf has no children' );
};

# ============================================================================
# Block structure
# ============================================================================

subtest 'multiple declarations in block' => sub {
    my $src = 'begin integer x, y; real z; x := 0; y := 0; z := 0.0 end';
    my $ast = CodingAdventures::AlgolParser->parse($src);
    is( count_nodes($ast, 'type_decl'), 2, 'two type_decl nodes' );
};

subtest 'comment is skipped by lexer before parsing' => sub {
    my $src = 'begin comment initialize; integer x; x := 1 end';
    my $ast = CodingAdventures::AlgolParser->parse($src);
    is( $ast->rule_name, 'program', 'parsed successfully past comment' );
    my $type_decl = find_node($ast, 'type_decl');
    ok( defined $type_decl, 'type_decl found after comment' );
};

subtest 'labeled statement' => sub {
    my $src = 'begin integer x; start: x := 0 end';
    my $ast = CodingAdventures::AlgolParser->parse($src);
    is( $ast->rule_name, 'program', 'parsed labeled statement successfully' );
    # The statement should have a label (IDENT) and COLON as first children
    my $stmt = find_node($ast, 'statement');
    ok( defined $stmt, 'found statement node' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'missing END raises die' => sub {
    ok(
        dies { CodingAdventures::AlgolParser->parse('begin integer x; x := 1') },
        'missing END causes die'
    );
};

subtest 'missing BEGIN raises die' => sub {
    ok(
        dies { CodingAdventures::AlgolParser->parse('integer x; x := 1 end') },
        'missing BEGIN causes die'
    );
};

subtest 'missing semicolon after declaration raises die' => sub {
    ok(
        dies { CodingAdventures::AlgolParser->parse('begin integer x x := 1 end') },
        'missing semicolon after declaration causes die'
    );
};

subtest 'missing ASSIGN in assignment raises die' => sub {
    ok(
        dies { CodingAdventures::AlgolParser->parse('begin integer x; x 42 end') },
        'missing := in assignment causes die'
    );
};

subtest 'empty input raises die' => sub {
    ok(
        dies { CodingAdventures::AlgolParser->parse('') },
        'empty input causes die'
    );
};

done_testing;
