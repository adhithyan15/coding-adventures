use strict;
use warnings;
use Test2::V0;

use CodingAdventures::JsonParser;
use CodingAdventures::JsonParser::ASTNode;

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
# Helper: count nodes with given rule_name
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
# Root node
# ============================================================================

subtest 'parse returns an ASTNode' => sub {
    my $ast = CodingAdventures::JsonParser->parse('42');
    ok( defined $ast, 'parse returns a defined value' );
    ok( ref($ast), 'parse returns a reference' );
};

subtest 'root node has rule_name value' => sub {
    my $ast = CodingAdventures::JsonParser->parse('42');
    is( $ast->rule_name, 'value', 'root rule_name is "value"' );
};

subtest 'root node has children' => sub {
    my $ast = CodingAdventures::JsonParser->parse('42');
    ok( ref($ast->children) eq 'ARRAY', 'children is an arrayref' );
    ok( scalar @{ $ast->children } >= 1, 'children is non-empty' );
};

# ============================================================================
# Scalar values
# ============================================================================

subtest 'parse bare number' => sub {
    my $ast = CodingAdventures::JsonParser->parse('42');
    is( $ast->rule_name, 'value', 'rule_name is value' );
    my $leaf = $ast->children->[0];
    ok( $leaf->is_leaf, 'child is a leaf' );
    is( $leaf->token->{type}, 'NUMBER', 'token type is NUMBER' );
    is( $leaf->token->{value}, '42', 'token value is 42' );
};

subtest 'parse negative float' => sub {
    my $ast = CodingAdventures::JsonParser->parse('-3.14');
    is( $ast->rule_name, 'value', 'rule_name is value' );
    my $leaf = $ast->children->[0];
    is( $leaf->token->{type}, 'NUMBER', 'token type is NUMBER' );
};

subtest 'parse string' => sub {
    my $ast = CodingAdventures::JsonParser->parse('"hello"');
    is( $ast->rule_name, 'value', 'rule_name is value' );
    my $leaf = $ast->children->[0];
    ok( $leaf->is_leaf, 'leaf' );
    is( $leaf->token->{type}, 'STRING', 'token type is STRING' );
    is( $leaf->token->{value}, '"hello"', 'token value is "hello"' );
};

subtest 'parse true' => sub {
    my $ast = CodingAdventures::JsonParser->parse('true');
    my $leaf = $ast->children->[0];
    is( $leaf->token->{type}, 'TRUE', 'token type is TRUE' );
};

subtest 'parse false' => sub {
    my $ast = CodingAdventures::JsonParser->parse('false');
    my $leaf = $ast->children->[0];
    is( $leaf->token->{type}, 'FALSE', 'token type is FALSE' );
};

subtest 'parse null' => sub {
    my $ast = CodingAdventures::JsonParser->parse('null');
    my $leaf = $ast->children->[0];
    is( $leaf->token->{type}, 'NULL', 'token type is NULL' );
};

# ============================================================================
# Empty containers
# ============================================================================

subtest 'parse empty object {}' => sub {
    my $ast = CodingAdventures::JsonParser->parse('{}');
    is( $ast->rule_name, 'value', 'root is value' );
    my $obj = find_node($ast, 'object');
    ok( defined $obj, 'found object node' );
    # Empty object: only LBRACE and RBRACE leaf children
    is( scalar @{ $obj->children }, 2, 'object has 2 children (braces only)' );
};

subtest 'parse empty array []' => sub {
    my $ast = CodingAdventures::JsonParser->parse('[]');
    is( $ast->rule_name, 'value', 'root is value' );
    my $arr = find_node($ast, 'array');
    ok( defined $arr, 'found array node' );
    is( scalar @{ $arr->children }, 2, 'array has 2 children (brackets only)' );
};

# ============================================================================
# Simple object
# ============================================================================

subtest 'parse {"key": 42}' => sub {
    my $ast = CodingAdventures::JsonParser->parse('{"key": 42}');
    is( $ast->rule_name, 'value', 'root is value' );
    my $obj  = find_node($ast, 'object');
    my $pair = find_node($ast, 'pair');
    ok( defined $obj,  'found object node' );
    ok( defined $pair, 'found pair node' );
    # Pair has STRING, COLON, value
    is( scalar @{ $pair->children }, 3, 'pair has 3 children' );
};

subtest 'object with multiple pairs' => sub {
    my $ast = CodingAdventures::JsonParser->parse('{"a": 1, "b": 2, "c": 3}');
    my $pair_count = count_nodes($ast, 'pair');
    is( $pair_count, 3, 'three pair nodes' );
};

subtest 'object with boolean and null values' => sub {
    my $ast = CodingAdventures::JsonParser->parse('{"ok": true, "data": null}');
    is( count_nodes($ast, 'pair'), 2, 'two pair nodes' );
};

# ============================================================================
# Simple array
# ============================================================================

subtest 'parse [1, 2, 3]' => sub {
    my $ast = CodingAdventures::JsonParser->parse('[1, 2, 3]');
    is( $ast->rule_name, 'value', 'root is value' );
    my $arr = find_node($ast, 'array');
    ok( defined $arr, 'found array node' );
    # children: LBRACKET value COMMA value COMMA value RBRACKET = 7
    is( scalar @{ $arr->children }, 7, 'array has 7 children' );
};

subtest 'parse array of strings' => sub {
    my $ast = CodingAdventures::JsonParser->parse('["a", "b"]');
    my $arr = find_node($ast, 'array');
    ok( defined $arr, 'found array node' );
    # LBRACKET value COMMA value RBRACKET = 5
    is( scalar @{ $arr->children }, 5, 'array has 5 children' );
};

subtest 'parse array of mixed types' => sub {
    my $ast = CodingAdventures::JsonParser->parse('[1, "two", true, false, null]');
    my $arr = find_node($ast, 'array');
    ok( defined $arr, 'found array' );
    # 5 values + 4 commas + 2 brackets = 11
    is( scalar @{ $arr->children }, 11, 'array has 11 children' );
};

# ============================================================================
# Nested structures
# ============================================================================

subtest 'nested object' => sub {
    my $ast = CodingAdventures::JsonParser->parse('{"a": {"b": 2}}');
    is( count_nodes($ast, 'pair'), 2, 'two pair nodes (nested)' );
};

subtest 'array inside object' => sub {
    my $ast = CodingAdventures::JsonParser->parse('{"tags": ["lua", "perl"]}');
    ok( defined find_node($ast, 'array'), 'found array node' );
};

subtest 'object inside array' => sub {
    my $ast = CodingAdventures::JsonParser->parse('[{"id": 1}, {"id": 2}]');
    is( count_nodes($ast, 'pair'), 2, 'two pair nodes' );
};

subtest 'deeply nested {"a": [1, 2, {"b": true}]}' => sub {
    my $ast = CodingAdventures::JsonParser->parse('{"a": [1, 2, {"b": true}]}');
    is( $ast->rule_name, 'value', 'root is value' );
    ok( defined find_node($ast, 'array'), 'found array' );
    is( count_nodes($ast, 'pair'), 2, 'two pairs (outer a:… and inner b:true)' );
};

subtest 'realistic JSON document' => sub {
    my $src = <<'END_JSON';
{
  "name": "Alice",
  "age": 30,
  "active": true,
  "score": -1.5,
  "tags": ["perl", "json"],
  "address": {
    "city": "Metropolis",
    "zip": null
  }
}
END_JSON
    my $ast = CodingAdventures::JsonParser->parse($src);
    is( $ast->rule_name, 'value', 'root is value' );
    ok( defined find_node($ast, 'array'), 'found array (tags)' );
    ok( count_nodes($ast, 'pair') >= 7, 'at least 7 pairs' );
};

# ============================================================================
# ASTNode methods
# ============================================================================

subtest 'ASTNode new and accessors' => sub {
    my $node = CodingAdventures::JsonParser::ASTNode->new(
        rule_name => 'value',
        children  => [],
        is_leaf   => 0,
    );
    is( $node->rule_name, 'value', 'rule_name accessor' );
    is( $node->is_leaf,   0,       'is_leaf accessor (false)' );
    ok( ref($node->children) eq 'ARRAY', 'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok  = { type => 'NUMBER', value => '42', line => 1, col => 1 };
    my $leaf = CodingAdventures::JsonParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => $tok,
    );
    is( $leaf->is_leaf,          1,        'is_leaf is true' );
    is( $leaf->token->{type},    'NUMBER', 'token type' );
    is( $leaf->token->{value},   '42',     'token value' );
};

subtest 'leaf nodes have empty children' => sub {
    my $tok  = { type => 'TRUE', value => 'true', line => 1, col => 1 };
    my $leaf = CodingAdventures::JsonParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => $tok,
    );
    is( scalar @{ $leaf->children }, 0, 'leaf has no children' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'trailing garbage raises die' => sub {
    ok(
        dies { CodingAdventures::JsonParser->parse('42 garbage') },
        'trailing content causes die'
    );
};

subtest 'unterminated object raises die' => sub {
    ok(
        dies { CodingAdventures::JsonParser->parse('{"key": 1') },
        'unterminated object causes die'
    );
};

subtest 'unterminated array raises die' => sub {
    ok(
        dies { CodingAdventures::JsonParser->parse('[1, 2') },
        'unterminated array causes die'
    );
};

subtest 'missing colon raises die' => sub {
    ok(
        dies { CodingAdventures::JsonParser->parse('{"key" 1}') },
        'missing colon causes die'
    );
};

subtest 'bare identifier raises die' => sub {
    ok(
        dies { CodingAdventures::JsonParser->parse('undefined') },
        'bare identifier causes die'
    );
};

subtest 'empty input raises die' => sub {
    ok(
        dies { CodingAdventures::JsonParser->parse('') },
        'empty input causes die'
    );
};

done_testing;
