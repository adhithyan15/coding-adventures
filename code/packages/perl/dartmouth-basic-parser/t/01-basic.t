use strict;
use warnings;
use Test2::V0;

use CodingAdventures::DartmouthBasicParser;
use CodingAdventures::DartmouthBasicParser::ASTNode;

# ============================================================================
# Test suite for CodingAdventures::DartmouthBasicParser
# ============================================================================
#
# Tests all 17 BASIC statement types, expression precedence rules,
# multi-line programs, and error cases.
#
# The 17 statement types from the 1964 Dartmouth BASIC specification:
#   LET, PRINT, INPUT, IF, GOTO, GOSUB, RETURN, FOR, NEXT,
#   END, STOP, REM, READ, DATA, RESTORE, DIM, DEF

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
# Helper: count nodes with given rule_name (full traversal)
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
# Helper: find first leaf token with given type (and optional value)
# ============================================================================

sub find_token {
    my ($node, $type, $value) = @_;
    return undef unless ref $node && $node->can('rule_name');
    if ($node->is_leaf && $node->token) {
        my $tok = $node->token;
        if ($tok->{type} eq $type) {
            return $node if !defined($value) || $tok->{value} eq $value;
        }
    }
    for my $child (@{ $node->children }) {
        my $found = find_token($child, $type, $value);
        return $found if defined $found;
    }
    return undef;
}

# ============================================================================
# Root node
# ============================================================================

subtest 'parse returns an ASTNode' => sub {
    my $ast = CodingAdventures::DartmouthBasicParser->parse("10 END\n");
    ok( defined $ast, 'parse returns a defined value' );
    ok( ref($ast),    'parse returns a reference' );
};

subtest 'root node has rule_name program' => sub {
    my $ast = CodingAdventures::DartmouthBasicParser->parse("10 END\n");
    is( $ast->rule_name, 'program', 'root rule_name is "program"' );
};

subtest 'root node has children' => sub {
    my $ast = CodingAdventures::DartmouthBasicParser->parse("10 END\n");
    ok( ref($ast->children) eq 'ARRAY', 'children is an arrayref' );
    ok( scalar @{ $ast->children } >= 1, 'children is non-empty' );
};

subtest 'empty input produces program with no line children' => sub {
    my $ast = CodingAdventures::DartmouthBasicParser->parse('');
    is( $ast->rule_name, 'program', 'root is program' );
    my $line = find_node($ast, 'line');
    ok( !defined $line, 'no line nodes for empty input' );
};

# ============================================================================
# Empty line: bare LINE_NUM
# ============================================================================

subtest 'bare line number is valid: "10\n"' => sub {
    my $ast = CodingAdventures::DartmouthBasicParser->parse("10\n");
    is( $ast->rule_name, 'program', 'root is program' );
    my $line = find_node($ast, 'line');
    ok( defined $line, 'found a line node' );
    # Bare line: only LINE_NUM and NEWLINE children (no statement)
    my $stmt = find_node($ast, 'statement');
    ok( !defined $stmt, 'no statement node for bare line' );
};

# ============================================================================
# Statement type 1: LET
# ============================================================================

subtest 'LET scalar assignment: 10 LET X = 5' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 LET X = 5\n");
    my $stmt = find_node($ast, 'let_stmt');
    ok( defined $stmt, 'found let_stmt node' );
};

subtest 'LET array assignment: 10 LET A(3) = 7' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 LET A(3) = 7\n");
    my $stmt = find_node($ast, 'let_stmt');
    ok( defined $stmt, 'found let_stmt node' );
};

subtest 'LET expression: 10 LET X = X + 1' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 LET X = X + 1\n");
    my $stmt = find_node($ast, 'let_stmt');
    ok( defined $stmt, 'found let_stmt node' );
    my $expr = find_node($ast, 'expr');
    ok( defined $expr, 'found expr node' );
};

# ============================================================================
# Statement type 2: PRINT
# ============================================================================

subtest 'PRINT bare: 10 PRINT' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 PRINT\n");
    my $stmt = find_node($ast, 'print_stmt');
    ok( defined $stmt, 'found print_stmt node' );
    my $list = find_node($ast, 'print_list');
    ok( !defined $list, 'no print_list for bare PRINT' );
};

subtest 'PRINT string: 10 PRINT "HELLO, WORLD"' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 PRINT \"HELLO, WORLD\"\n");
    my $stmt = find_node($ast, 'print_stmt');
    ok( defined $stmt, 'found print_stmt node' );
};

subtest 'PRINT expression: 10 PRINT X' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 PRINT X\n");
    my $stmt = find_node($ast, 'print_stmt');
    ok( defined $stmt, 'found print_stmt node' );
};

subtest 'PRINT comma-separated: 10 PRINT X, Y' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 PRINT X, Y\n");
    my $stmt = find_node($ast, 'print_stmt');
    ok( defined $stmt, 'found print_stmt node' );
    my $list = find_node($ast, 'print_list');
    ok( defined $list, 'found print_list node' );
};

subtest 'PRINT semicolon: 10 PRINT X; Y' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 PRINT X; Y\n");
    my $stmt = find_node($ast, 'print_stmt');
    ok( defined $stmt, 'found print_stmt' );
};

# ============================================================================
# Statement type 3: INPUT
# ============================================================================

subtest 'INPUT single variable: 10 INPUT X' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 INPUT X\n");
    my $stmt = find_node($ast, 'input_stmt');
    ok( defined $stmt, 'found input_stmt node' );
};

subtest 'INPUT multiple variables: 10 INPUT A, B, C' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 INPUT A, B, C\n");
    my $stmt = find_node($ast, 'input_stmt');
    ok( defined $stmt, 'found input_stmt node' );
};

# ============================================================================
# Statement type 4: IF ... THEN
# ============================================================================

subtest 'IF EQ: 10 IF X = 0 THEN 100' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 IF X = 0 THEN 100\n");
    my $stmt = find_node($ast, 'if_stmt');
    ok( defined $stmt, 'found if_stmt node' );
    my $relop = find_node($ast, 'relop');
    ok( defined $relop, 'found relop node' );
};

subtest 'IF LT: 10 IF X < 5 THEN 20' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 IF X < 5 THEN 20\n");
    my $stmt = find_node($ast, 'if_stmt');
    ok( defined $stmt, 'found if_stmt' );
};

subtest 'IF GT: 10 IF A > B THEN 50' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 IF A > B THEN 50\n");
    my $stmt = find_node($ast, 'if_stmt');
    ok( defined $stmt, 'found if_stmt' );
};

subtest 'IF LE: 10 IF X <= 10 THEN 20' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 IF X <= 10 THEN 20\n");
    my $stmt = find_node($ast, 'if_stmt');
    ok( defined $stmt, 'found if_stmt' );
};

subtest 'IF GE: 10 IF X >= 0 THEN 30' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 IF X >= 0 THEN 30\n");
    my $stmt = find_node($ast, 'if_stmt');
    ok( defined $stmt, 'found if_stmt' );
};

subtest 'IF NE: 10 IF X <> Y THEN 70' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 IF X <> Y THEN 70\n");
    my $stmt = find_node($ast, 'if_stmt');
    ok( defined $stmt, 'found if_stmt' );
};

# ============================================================================
# Statement type 5: GOTO
# ============================================================================

subtest 'GOTO: 10 GOTO 50' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 GOTO 50\n");
    my $stmt = find_node($ast, 'goto_stmt');
    ok( defined $stmt, 'found goto_stmt node' );
};

# ============================================================================
# Statement type 6: GOSUB
# ============================================================================

subtest 'GOSUB: 10 GOSUB 200' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 GOSUB 200\n");
    my $stmt = find_node($ast, 'gosub_stmt');
    ok( defined $stmt, 'found gosub_stmt node' );
};

# ============================================================================
# Statement type 7: RETURN
# ============================================================================

subtest 'RETURN: 10 RETURN' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 RETURN\n");
    my $stmt = find_node($ast, 'return_stmt');
    ok( defined $stmt, 'found return_stmt node' );
};

# ============================================================================
# Statement type 8: FOR
# ============================================================================

subtest 'FOR without STEP: 10 FOR I = 1 TO 10' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 FOR I = 1 TO 10\n");
    my $stmt = find_node($ast, 'for_stmt');
    ok( defined $stmt, 'found for_stmt node' );
};

subtest 'FOR with STEP: 10 FOR I = 10 TO 1 STEP -1' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 FOR I = 10 TO 1 STEP -1\n");
    my $stmt = find_node($ast, 'for_stmt');
    ok( defined $stmt, 'found for_stmt node' );
};

# ============================================================================
# Statement type 9: NEXT
# ============================================================================

subtest 'NEXT: 30 NEXT I' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("30 NEXT I\n");
    my $stmt = find_node($ast, 'next_stmt');
    ok( defined $stmt, 'found next_stmt node' );
};

# ============================================================================
# Statement type 10: END
# ============================================================================

subtest 'END: 10 END' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 END\n");
    my $stmt = find_node($ast, 'end_stmt');
    ok( defined $stmt, 'found end_stmt node' );
};

# ============================================================================
# Statement type 11: STOP
# ============================================================================

subtest 'STOP: 10 STOP' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 STOP\n");
    my $stmt = find_node($ast, 'stop_stmt');
    ok( defined $stmt, 'found stop_stmt node' );
};

# ============================================================================
# Statement type 12: REM
# ============================================================================

subtest 'REM comment: 10 REM THIS IS A COMMENT' => sub {
    # The lexer suppresses all tokens after REM, so the token stream is:
    #   LINE_NUM(10) KEYWORD(REM) NEWLINE
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 REM THIS IS A COMMENT\n");
    my $stmt = find_node($ast, 'rem_stmt');
    ok( defined $stmt, 'found rem_stmt node' );
};

# ============================================================================
# Statement type 13: READ
# ============================================================================

subtest 'READ single: 10 READ X' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 READ X\n");
    my $stmt = find_node($ast, 'read_stmt');
    ok( defined $stmt, 'found read_stmt node' );
};

subtest 'READ multiple: 10 READ A, B, C' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 READ A, B, C\n");
    my $stmt = find_node($ast, 'read_stmt');
    ok( defined $stmt, 'found read_stmt node' );
};

# ============================================================================
# Statement type 14: DATA
# ============================================================================

subtest 'DATA single: 10 DATA 42' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 DATA 42\n");
    my $stmt = find_node($ast, 'data_stmt');
    ok( defined $stmt, 'found data_stmt node' );
};

subtest 'DATA multiple: 10 DATA 1, 2, 3, 4, 5' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 DATA 1, 2, 3, 4, 5\n");
    my $stmt = find_node($ast, 'data_stmt');
    ok( defined $stmt, 'found data_stmt node' );
};

# ============================================================================
# Statement type 15: RESTORE
# ============================================================================

subtest 'RESTORE: 10 RESTORE' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 RESTORE\n");
    my $stmt = find_node($ast, 'restore_stmt');
    ok( defined $stmt, 'found restore_stmt node' );
};

# ============================================================================
# Statement type 16: DIM
# ============================================================================

subtest 'DIM single array: 10 DIM A(10)' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 DIM A(10)\n");
    my $stmt = find_node($ast, 'dim_stmt');
    ok( defined $stmt, 'found dim_stmt node' );
};

subtest 'DIM multiple arrays: 10 DIM A(10), B(20)' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 DIM A(10), B(20)\n");
    my $stmt = find_node($ast, 'dim_stmt');
    ok( defined $stmt, 'found dim_stmt node' );
    is( count_nodes($ast, 'dim_decl'), 2, 'two dim_decl nodes' );
};

# ============================================================================
# Statement type 17: DEF
# ============================================================================

subtest 'DEF user function: 10 DEF FNA(X) = X * X' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 DEF FNA(X) = X * X\n");
    my $stmt = find_node($ast, 'def_stmt');
    ok( defined $stmt, 'found def_stmt node' );
};

subtest 'DEF with builtin: 10 DEF FNB(T) = SIN(T)' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 DEF FNB(T) = SIN(T)\n");
    my $stmt = find_node($ast, 'def_stmt');
    ok( defined $stmt, 'found def_stmt node' );
};

# ============================================================================
# Expression rules
# ============================================================================

subtest 'expr: addition A + B' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 LET X = A + B\n");
    my $expr = find_node($ast, 'expr');
    ok( defined $expr, 'found expr node' );
};

subtest 'term: multiplication A * B' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 LET X = A * B\n");
    my $term = find_node($ast, 'term');
    ok( defined $term, 'found term node' );
};

subtest 'power: exponentiation A ^ B' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 LET X = A ^ B\n");
    my $pow  = find_node($ast, 'power');
    ok( defined $pow, 'found power node' );
};

subtest 'unary: unary minus -5' => sub {
    my $ast   = CodingAdventures::DartmouthBasicParser->parse("10 LET X = -5\n");
    my $unary = find_node($ast, 'unary');
    ok( defined $unary, 'found unary node' );
};

subtest 'primary: NUMBER token' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 LET X = 42\n");
    my $prim = find_node($ast, 'primary');
    ok( defined $prim, 'found primary node' );
    my $leaf = find_token($ast, 'NUMBER', '42');
    ok( defined $leaf, 'NUMBER token present' );
};

subtest 'primary: builtin function SIN(X)' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 LET X = SIN(A)\n");
    my $prim = find_node($ast, 'primary');
    ok( defined $prim, 'found primary node' );
};

subtest 'primary: user function FNA(Y)' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 LET X = FNA(Y)\n");
    my $prim = find_node($ast, 'primary');
    ok( defined $prim, 'found primary node' );
};

subtest 'primary: parenthesised (A + B) * C' => sub {
    my $ast  = CodingAdventures::DartmouthBasicParser->parse("10 LET X = (A + B) * C\n");
    ok( defined find_node($ast, 'expr'), 'found expr node' );
    ok( defined find_node($ast, 'term'), 'found term node' );
};

subtest 'complex expression: A + B * C ^ 2' => sub {
    my $ast = CodingAdventures::DartmouthBasicParser->parse("10 LET X = A + B * C ^ 2\n");
    ok( defined find_node($ast, 'expr'),  'found expr node' );
    ok( defined find_node($ast, 'term'),  'found term node' );
    ok( defined find_node($ast, 'power'), 'found power node' );
};

# ============================================================================
# Multi-line programs
# ============================================================================

subtest '"HELLO, WORLD" program' => sub {
    my $ast = CodingAdventures::DartmouthBasicParser->parse(
        "10 PRINT \"HELLO, WORLD\"\n20 END\n"
    );
    is( $ast->rule_name, 'program', 'root is program' );
    is( count_nodes($ast, 'line'), 2, 'two line nodes' );
};

subtest 'FOR loop program' => sub {
    my $src = "10 FOR I = 1 TO 5\n20 PRINT I\n30 NEXT I\n40 END\n";
    my $ast = CodingAdventures::DartmouthBasicParser->parse($src);
    is( $ast->rule_name, 'program', 'root is program' );
    is( count_nodes($ast, 'line'), 4, 'four line nodes' );
    ok( defined find_node($ast, 'for_stmt'),  'found for_stmt' );
    ok( defined find_node($ast, 'next_stmt'), 'found next_stmt' );
};

subtest 'counting loop with GOTO' => sub {
    my $src =
        "10 LET X = 1\n"   .
        "20 PRINT X\n"     .
        "30 LET X = X + 1\n" .
        "40 IF X <= 10 THEN 20\n" .
        "50 END\n";
    my $ast = CodingAdventures::DartmouthBasicParser->parse($src);
    is( $ast->rule_name, 'program', 'root is program' );
    is( count_nodes($ast, 'line'), 5, 'five line nodes' );
};

subtest 'GOSUB / RETURN program' => sub {
    my $src =
        "10 GOSUB 100\n" .
        "20 END\n"       .
        "100 PRINT \"IN SUBROUTINE\"\n" .
        "110 RETURN\n";
    my $ast = CodingAdventures::DartmouthBasicParser->parse($src);
    is( $ast->rule_name, 'program', 'root is program' );
    is( count_nodes($ast, 'line'), 4, 'four line nodes' );
    ok( defined find_node($ast, 'gosub_stmt'),  'found gosub_stmt' );
    ok( defined find_node($ast, 'return_stmt'), 'found return_stmt' );
};

subtest 'READ / DATA program' => sub {
    my $src =
        "10 READ X\n"   .
        "20 PRINT X\n"  .
        "30 DATA 42\n"  .
        "40 END\n";
    my $ast = CodingAdventures::DartmouthBasicParser->parse($src);
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'read_stmt'), 'found read_stmt' );
    ok( defined find_node($ast, 'data_stmt'), 'found data_stmt' );
};

subtest 'program with REM comment' => sub {
    my $src =
        "10 REM DARTMOUTH BASIC EXAMPLE\n" .
        "20 LET X = 42\n"                  .
        "30 END\n";
    my $ast = CodingAdventures::DartmouthBasicParser->parse($src);
    is( $ast->rule_name, 'program', 'root is program' );
    is( count_nodes($ast, 'line'), 3, 'three line nodes' );
    ok( defined find_node($ast, 'rem_stmt'), 'found rem_stmt' );
};

# ============================================================================
# ASTNode methods
# ============================================================================

subtest 'ASTNode new and accessors (internal node)' => sub {
    my $node = CodingAdventures::DartmouthBasicParser::ASTNode->new(
        rule_name => 'program',
        children  => [],
        is_leaf   => 0,
    );
    is( $node->rule_name, 'program', 'rule_name accessor' );
    is( $node->is_leaf,   0,         'is_leaf is false' );
    ok( ref($node->children) eq 'ARRAY', 'children is arrayref' );
};

subtest 'ASTNode new and accessors (leaf node)' => sub {
    my $tok  = { type => 'LINE_NUM', value => '10', line => 1, col => 1 };
    my $leaf = CodingAdventures::DartmouthBasicParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => $tok,
    );
    is( $leaf->is_leaf,         1,         'is_leaf is true' );
    is( $leaf->token->{type},   'LINE_NUM', 'token type' );
    is( $leaf->token->{value},  '10',       'token value' );
};

subtest 'leaf nodes have empty children' => sub {
    my $tok  = { type => 'KEYWORD', value => 'END', line => 1, col => 4 };
    my $leaf = CodingAdventures::DartmouthBasicParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => $tok,
    );
    is( scalar @{ $leaf->children }, 0, 'leaf has no children' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'IF without THEN raises die' => sub {
    ok(
        dies { CodingAdventures::DartmouthBasicParser->parse("10 IF X > 0\n") },
        'IF without THEN causes die'
    );
};

subtest 'incomplete LET (no expression) raises die' => sub {
    ok(
        dies { CodingAdventures::DartmouthBasicParser->parse("10 LET X =\n") },
        'incomplete LET causes die'
    );
};

subtest 'incomplete FOR (no TO) raises die' => sub {
    ok(
        dies { CodingAdventures::DartmouthBasicParser->parse("10 FOR I = 1\n") },
        'FOR without TO causes die'
    );
};

done_testing;
