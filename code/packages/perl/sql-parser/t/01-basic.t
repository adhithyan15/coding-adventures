use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::SqlParser; 1 }, 'module loads' );
ok( eval { require CodingAdventures::SqlParser::ASTNode; 1 }, 'ASTNode loads' );

# ============================================================================
# Helpers
# ============================================================================

# Parse SQL and return the root AST node.
sub parse_sql {
    my ($sql) = @_;
    return CodingAdventures::SqlParser->parse_sql($sql);
}

# Find the first node with the given rule_name (depth-first).
sub find_node {
    my ($node, $rule_name) = @_;
    return undef unless ref($node);
    return $node if $node->rule_name eq $rule_name;
    for my $child (@{ $node->children }) {
        my $found = find_node($child, $rule_name);
        return $found if defined $found;
    }
    return undef;
}

# Count all nodes with the given rule_name.
sub count_nodes {
    my ($node, $rule_name) = @_;
    return 0 unless ref($node);
    my $n = ($node->rule_name eq $rule_name) ? 1 : 0;
    for my $child (@{ $node->children }) {
        $n += count_nodes($child, $rule_name);
    }
    return $n;
}

# Collect all rule_names in pre-order.
sub collect_rule_names {
    my ($node) = @_;
    return () unless ref($node);
    my @names = ($node->rule_name);
    for my $child (@{ $node->children }) {
        push @names, collect_rule_names($child);
    }
    return @names;
}

# ============================================================================
# ASTNode unit tests
# ============================================================================

subtest 'ASTNode inner node' => sub {
    my $node = CodingAdventures::SqlParser::ASTNode->new('select_stmt', []);
    is( $node->rule_name, 'select_stmt', 'rule_name' );
    is( $node->is_leaf,   0,             'not a leaf' );
    is( ref($node->children), 'ARRAY',   'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok  = { type => 'SELECT', value => 'SELECT', line => 1, col => 1 };
    my $leaf = CodingAdventures::SqlParser::ASTNode->new_leaf($tok);
    is( $leaf->rule_name,    'token',   'rule_name is token' );
    is( $leaf->is_leaf,      1,         'is_leaf returns 1' );
    is( $leaf->token->{type}, 'SELECT', 'token type' );
};

# ============================================================================
# Root node
# ============================================================================

subtest 'root rule_name is program' => sub {
    my $ast = parse_sql("SELECT * FROM users");
    is( $ast->rule_name, 'program', 'root is program' );
};

subtest 'program contains statement' => sub {
    my $ast  = parse_sql("SELECT * FROM t");
    my $stmt = find_node($ast, 'statement');
    ok( defined $stmt, 'statement node found' );
};

# ============================================================================
# SELECT statements
# ============================================================================

subtest 'SELECT * FROM users' => sub {
    my $ast = parse_sql("SELECT * FROM users");
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'select_stmt'), 'select_stmt node' );
    ok( defined find_node($ast, 'select_list'), 'select_list node' );
    ok( defined find_node($ast, 'table_ref'),   'table_ref node' );
};

subtest 'SELECT name, age FROM users WHERE age > 18' => sub {
    my $ast = parse_sql("SELECT name, age FROM users WHERE age > 18");
    ok( defined find_node($ast, 'select_stmt'),  'select_stmt' );
    ok( defined find_node($ast, 'where_clause'), 'where_clause' );
    ok( defined find_node($ast, 'comparison'),   'comparison' );
    my $sc = count_nodes($ast, 'select_item');
    ok( $sc >= 2, "at least 2 select_item nodes (got $sc)" );
};

subtest 'SELECT DISTINCT' => sub {
    my $ast = parse_sql("SELECT DISTINCT name FROM users");
    ok( defined find_node($ast, 'select_stmt'), 'select_stmt' );
};

subtest 'SELECT with AND condition' => sub {
    my $ast = parse_sql(
        "SELECT * FROM employees WHERE salary > 50000 AND active = TRUE"
    );
    ok( defined find_node($ast, 'where_clause'), 'where_clause' );
    ok( defined find_node($ast, 'and_expr'),     'and_expr' );
};

subtest 'SELECT with OR condition' => sub {
    my $ast = parse_sql("SELECT * FROM t WHERE a > 1 OR b < 2");
    ok( defined find_node($ast, 'or_expr'), 'or_expr' );
};

subtest 'SELECT with ORDER BY' => sub {
    my $ast = parse_sql("SELECT * FROM users ORDER BY name");
    ok( defined find_node($ast, 'order_clause'), 'order_clause' );
};

subtest 'SELECT with ORDER BY DESC' => sub {
    my $ast = parse_sql("SELECT * FROM t ORDER BY id DESC");
    ok( defined find_node($ast, 'order_clause'), 'order_clause' );
    ok( defined find_node($ast, 'order_item'),   'order_item' );
};

subtest 'SELECT with LIMIT' => sub {
    my $ast = parse_sql("SELECT * FROM t LIMIT 10");
    ok( defined find_node($ast, 'limit_clause'), 'limit_clause' );
};

subtest 'SELECT with LIMIT OFFSET' => sub {
    my $ast = parse_sql("SELECT * FROM t LIMIT 10 OFFSET 5");
    ok( defined find_node($ast, 'limit_clause'), 'limit_clause' );
};

subtest 'SELECT with GROUP BY' => sub {
    my $ast = parse_sql("SELECT dept FROM employees GROUP BY dept");
    ok( defined find_node($ast, 'group_clause'), 'group_clause' );
};

subtest 'SELECT with HAVING' => sub {
    my $ast = parse_sql(
        "SELECT dept FROM employees GROUP BY dept HAVING COUNT(*) > 5"
    );
    ok( defined find_node($ast, 'group_clause'),  'group_clause' );
    ok( defined find_node($ast, 'having_clause'), 'having_clause' );
    ok( defined find_node($ast, 'function_call'), 'function_call for COUNT' );
};

subtest 'SELECT with INNER JOIN' => sub {
    my $ast = parse_sql(
        "SELECT * FROM users INNER JOIN orders ON users.id = orders.user_id"
    );
    ok( defined find_node($ast, 'join_clause'), 'join_clause' );
};

subtest 'SELECT with column_ref using dot notation' => sub {
    my $ast = parse_sql("SELECT t.id FROM t");
    ok( defined find_node($ast, 'column_ref'), 'column_ref' );
};

subtest 'SELECT with function call' => sub {
    my $ast = parse_sql("SELECT COUNT(*) FROM t");
    ok( defined find_node($ast, 'function_call'), 'function_call' );
};

subtest 'SELECT with AS alias' => sub {
    my $ast = parse_sql("SELECT name AS n FROM t");
    ok( defined find_node($ast, 'select_item'), 'select_item' );
};

# ============================================================================
# INSERT statements
# ============================================================================

subtest 'INSERT INTO orders VALUES (1, item, 9.99)' => sub {
    my $ast = parse_sql("INSERT INTO orders VALUES (1, 'item', 9.99)");
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'insert_stmt'), 'insert_stmt' );
    ok( defined find_node($ast, 'row_value'),   'row_value' );
};

subtest 'INSERT with column list' => sub {
    my $ast = parse_sql(
        "INSERT INTO users (id, name) VALUES (1, 'Alice')"
    );
    ok( defined find_node($ast, 'insert_stmt'), 'insert_stmt' );
    ok( defined find_node($ast, 'row_value'),   'row_value' );
};

subtest 'INSERT with multiple row values' => sub {
    my $ast = parse_sql(
        "INSERT INTO t VALUES (1, 2), (3, 4)"
    );
    my $rv_count = count_nodes($ast, 'row_value');
    is( $rv_count, 2, '2 row_value nodes' );
};

# ============================================================================
# UPDATE statements
# ============================================================================

subtest "UPDATE users SET name = 'Bob' WHERE id = 1" => sub {
    my $ast = parse_sql("UPDATE users SET name = 'Bob' WHERE id = 1");
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'update_stmt'),  'update_stmt' );
    ok( defined find_node($ast, 'assignment'),   'assignment' );
    ok( defined find_node($ast, 'where_clause'), 'where_clause' );
};

subtest 'UPDATE with multiple assignments' => sub {
    my $ast = parse_sql(
        "UPDATE users SET name = 'Alice', age = 30 WHERE id = 1"
    );
    my $ac = count_nodes($ast, 'assignment');
    ok( $ac >= 2, "at least 2 assignment nodes (got $ac)" );
};

subtest 'UPDATE without WHERE' => sub {
    my $ast = parse_sql("UPDATE t SET col = 42");
    ok( defined find_node($ast, 'update_stmt'), 'update_stmt' );
    ok( !defined find_node($ast, 'where_clause'), 'no where_clause' );
};

# ============================================================================
# DELETE statements
# ============================================================================

subtest 'DELETE FROM temp WHERE expired = TRUE' => sub {
    my $ast = parse_sql("DELETE FROM temp WHERE expired = TRUE");
    is( $ast->rule_name, 'program', 'root is program' );
    ok( defined find_node($ast, 'delete_stmt'),  'delete_stmt' );
    ok( defined find_node($ast, 'where_clause'), 'where_clause' );
};

subtest 'DELETE without WHERE' => sub {
    my $ast = parse_sql("DELETE FROM temp");
    ok( defined find_node($ast, 'delete_stmt'), 'delete_stmt' );
    ok( !defined find_node($ast, 'where_clause'), 'no where_clause' );
};

# ============================================================================
# Expressions
# ============================================================================

subtest 'comparison with =' => sub {
    my $ast = parse_sql("SELECT * FROM t WHERE a = 1");
    ok( defined find_node($ast, 'comparison'), 'comparison node' );
};

subtest 'comparison with !=' => sub {
    my $ast = parse_sql("SELECT * FROM t WHERE a != 1");
    ok( defined find_node($ast, 'comparison'), 'comparison node' );
};

subtest 'comparison with BETWEEN AND' => sub {
    my $ast = parse_sql("SELECT * FROM t WHERE age BETWEEN 18 AND 65");
    ok( defined find_node($ast, 'comparison'), 'comparison node for BETWEEN' );
};

subtest 'comparison with IN' => sub {
    my $ast = parse_sql("SELECT * FROM t WHERE id IN (1, 2, 3)");
    ok( defined find_node($ast, 'comparison'), 'comparison node for IN' );
};

subtest 'comparison with LIKE' => sub {
    my $ast = parse_sql("SELECT * FROM t WHERE name LIKE '%Alice%'");
    ok( defined find_node($ast, 'comparison'), 'comparison node for LIKE' );
};

subtest 'comparison with IS NULL' => sub {
    my $ast = parse_sql("SELECT * FROM t WHERE col IS NULL");
    ok( defined find_node($ast, 'comparison'), 'comparison for IS NULL' );
};

subtest 'comparison with IS NOT NULL' => sub {
    my $ast = parse_sql("SELECT * FROM t WHERE col IS NOT NULL");
    ok( defined find_node($ast, 'comparison'), 'comparison for IS NOT NULL' );
};

subtest 'arithmetic additive' => sub {
    my $ast = parse_sql("SELECT a + b FROM t");
    ok( defined find_node($ast, 'additive'), 'additive node' );
};

subtest 'arithmetic multiplicative' => sub {
    my $ast = parse_sql("SELECT a * b FROM t");
    ok( defined find_node($ast, 'multiplicative'), 'multiplicative node' );
};

subtest 'unary minus in expression' => sub {
    my $ast = parse_sql("SELECT * FROM t WHERE x > -1");
    ok( defined find_node($ast, 'unary'), 'unary node' );
};

subtest 'NULL literal in expr' => sub {
    my $ast = parse_sql("INSERT INTO t VALUES (NULL)");
    ok( defined find_node($ast, 'primary'), 'primary node contains NULL' );
};

subtest 'TRUE / FALSE literals' => sub {
    my $ast = parse_sql("DELETE FROM t WHERE active = TRUE");
    ok( defined find_node($ast, 'primary'), 'primary node' );
};

subtest 'NOT expression' => sub {
    my $ast = parse_sql("SELECT * FROM t WHERE NOT active = TRUE");
    ok( defined find_node($ast, 'not_expr'), 'not_expr node' );
};

# ============================================================================
# Multiple statements
# ============================================================================

subtest 'two statements separated by semicolon' => sub {
    my $ast = parse_sql("SELECT * FROM a; SELECT * FROM b");
    is( $ast->rule_name, 'program', 'root is program' );
    my $sc = count_nodes($ast, 'select_stmt');
    is( $sc, 2, '2 select_stmt nodes' );
};

subtest 'statement with trailing semicolon' => sub {
    my $ast = parse_sql("SELECT 1 FROM t;");
    ok( defined find_node($ast, 'select_stmt'), 'select_stmt' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'empty input raises die' => sub {
    ok(
        dies { CodingAdventures::SqlParser->parse_sql('') },
        'empty input causes die'
    );
};

subtest 'incomplete SELECT raises die' => sub {
    ok(
        dies { CodingAdventures::SqlParser->parse_sql('SELECT name') },
        'missing FROM causes die'
    );
};

subtest 'garbage input raises die' => sub {
    ok(
        dies { CodingAdventures::SqlParser->parse_sql('@@@ GARBAGE') },
        'garbage causes die'
    );
};

subtest 'SELECT without FROM raises die' => sub {
    ok(
        dies { CodingAdventures::SqlParser->parse_sql('SELECT * LIMIT 5') },
        'missing FROM raises die'
    );
};

done_testing;
