use strict;
use warnings;
use Test2::V0;

use CodingAdventures::TomlParser;
use CodingAdventures::TomlParser::ASTNode;

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
    my $ast = CodingAdventures::TomlParser->parse("key = 42\n");
    ok( defined $ast, 'parse returns defined' );
    ok( ref($ast), 'parse returns reference' );
};

subtest 'root node has rule_name document' => sub {
    my $ast = CodingAdventures::TomlParser->parse("key = 42\n");
    is( $ast->rule_name, 'document', 'root rule_name is document' );
};

subtest 'root node children is arrayref' => sub {
    my $ast = CodingAdventures::TomlParser->parse("key = 42\n");
    ok( ref($ast->children) eq 'ARRAY', 'children is arrayref' );
};

# ============================================================================
# Key-value pairs
# ============================================================================

subtest 'parse string value' => sub {
    my $ast = CodingAdventures::TomlParser->parse("name = \"Alice\"\n");
    is( $ast->rule_name, 'document', 'root is document' );
    my $kv = find_node($ast, 'keyval');
    ok( defined $kv, 'found keyval node' );
};

subtest 'parse integer value' => sub {
    my $ast = CodingAdventures::TomlParser->parse("port = 8080\n");
    ok( defined find_node($ast, 'keyval'), 'found keyval' );
};

subtest 'parse float value' => sub {
    my $ast = CodingAdventures::TomlParser->parse("pi = 3.14\n");
    ok( defined find_node($ast, 'keyval'), 'found keyval' );
};

subtest 'parse boolean true' => sub {
    my $ast = CodingAdventures::TomlParser->parse("debug = true\n");
    ok( defined find_node($ast, 'keyval'), 'found keyval' );
};

subtest 'parse boolean false' => sub {
    my $ast = CodingAdventures::TomlParser->parse("enabled = false\n");
    ok( defined find_node($ast, 'keyval'), 'found keyval' );
};

subtest 'parse multiple key-value pairs' => sub {
    my $src = "host = \"localhost\"\nport = 9000\ndebug = true\n";
    my $ast = CodingAdventures::TomlParser->parse($src);
    is( $ast->rule_name, 'document', 'root is document' );
    is( count_nodes($ast, 'keyval'), 3, 'three keyval nodes' );
};

# ============================================================================
# Keys
# ============================================================================

subtest 'bare key' => sub {
    my $ast = CodingAdventures::TomlParser->parse("my_key = 1\n");
    ok( defined find_node($ast, 'key'), 'found key node' );
    ok( defined find_node($ast, 'simple_key'), 'found simple_key node' );
};

subtest 'quoted key' => sub {
    my $ast = CodingAdventures::TomlParser->parse("\"my key\" = 1\n");
    ok( defined find_node($ast, 'key'), 'found key node' );
};

subtest 'dotted key a.b' => sub {
    my $ast = CodingAdventures::TomlParser->parse("a.b = 1\n");
    my $key = find_node($ast, 'key');
    ok( defined $key, 'found key node' );
    is( count_nodes($key, 'simple_key'), 2, 'two simple_key nodes for a.b' );
};

subtest 'dotted key a.b.c' => sub {
    my $ast = CodingAdventures::TomlParser->parse("a.b.c = 1\n");
    my $key = find_node($ast, 'key');
    ok( defined $key, 'found key node' );
    is( count_nodes($key, 'simple_key'), 3, 'three simple_key nodes for a.b.c' );
};

# ============================================================================
# Table headers
# ============================================================================

subtest 'simple table header [section]' => sub {
    my $ast = CodingAdventures::TomlParser->parse("[server]\n");
    my $th = find_node($ast, 'table_header');
    ok( defined $th, 'found table_header node' );
};

subtest 'table header with key-value pair' => sub {
    my $src = "[server]\nhost = \"localhost\"\n";
    my $ast = CodingAdventures::TomlParser->parse($src);
    ok( defined find_node($ast, 'table_header'), 'found table_header' );
    ok( defined find_node($ast, 'keyval'), 'found keyval' );
};

subtest 'dotted table header [a.b]' => sub {
    my $ast = CodingAdventures::TomlParser->parse("[a.b]\n");
    ok( defined find_node($ast, 'table_header'), 'found table_header' );
};

subtest 'multiple table sections' => sub {
    my $src = "[alpha]\nx = 1\n[beta]\ny = 2\n";
    my $ast = CodingAdventures::TomlParser->parse($src);
    is( count_nodes($ast, 'table_header'), 2, 'two table_header nodes' );
};

# ============================================================================
# Array-of-tables headers
# ============================================================================

subtest '[[products]] header' => sub {
    my $ast = CodingAdventures::TomlParser->parse("[[products]]\n");
    my $ath = find_node($ast, 'array_table_header');
    ok( defined $ath, 'found array_table_header' );
};

subtest 'multiple [[array]] headers' => sub {
    my $src = "[[fruits]]\nname = \"apple\"\n[[fruits]]\nname = \"banana\"\n";
    my $ast = CodingAdventures::TomlParser->parse($src);
    is( count_nodes($ast, 'array_table_header'), 2, 'two array_table_header nodes' );
};

# ============================================================================
# Inline arrays
# ============================================================================

subtest 'empty inline array' => sub {
    my $ast = CodingAdventures::TomlParser->parse("ports = []\n");
    ok( defined find_node($ast, 'array'), 'found array node' );
};

subtest 'inline array of integers' => sub {
    my $ast = CodingAdventures::TomlParser->parse("ports = [8001, 8002, 8003]\n");
    ok( defined find_node($ast, 'array'), 'found array node' );
};

subtest 'inline array of strings' => sub {
    my $ast = CodingAdventures::TomlParser->parse("colors = [\"red\", \"green\"]\n");
    ok( defined find_node($ast, 'array'), 'found array node' );
};

subtest 'nested inline array' => sub {
    my $ast = CodingAdventures::TomlParser->parse("matrix = [[1, 2], [3, 4]]\n");
    ok( count_nodes($ast, 'array') >= 2, 'at least two array nodes (nested)' );
};

# ============================================================================
# Inline tables
# ============================================================================

subtest 'empty inline table {}' => sub {
    my $ast = CodingAdventures::TomlParser->parse("empty = {}\n");
    ok( defined find_node($ast, 'inline_table'), 'found inline_table' );
};

subtest 'inline table with one pair' => sub {
    my $ast = CodingAdventures::TomlParser->parse("point = {x = 1}\n");
    ok( defined find_node($ast, 'inline_table'), 'found inline_table' );
};

subtest 'inline table with multiple pairs' => sub {
    my $ast = CodingAdventures::TomlParser->parse("point = {x = 1, y = 2}\n");
    ok( defined find_node($ast, 'inline_table'), 'found inline_table' );
    # outer keyval (point=…) + inner keyvals (x=1, y=2)
    ok( count_nodes($ast, 'keyval') >= 3, 'at least 3 keyval nodes' );
};

# ============================================================================
# Multi-section document
# ============================================================================

subtest 'realistic TOML config' => sub {
    my $src = <<'TOML';
[database]
server = "192.168.1.1"
ports = [5432, 5433]
enabled = true

[servers.alpha]
ip = "10.0.0.1"

[[products]]
name = "Widget"
sku = 738594937

[[products]]
name = "Gadget"
sku = 284758393
TOML
    my $ast = CodingAdventures::TomlParser->parse($src);
    is( $ast->rule_name, 'document', 'root is document' );
    ok( count_nodes($ast, 'table_header') >= 2, 'at least two table_header nodes' );
    is( count_nodes($ast, 'array_table_header'), 2, 'two array_table_header nodes' );
    ok( count_nodes($ast, 'keyval') >= 5, 'at least 5 keyval nodes' );
};

# ============================================================================
# ASTNode methods
# ============================================================================

subtest 'ASTNode new and accessors' => sub {
    my $node = CodingAdventures::TomlParser::ASTNode->new(
        rule_name => 'keyval',
        children  => [],
        is_leaf   => 0,
    );
    is( $node->rule_name, 'keyval', 'rule_name accessor' );
    is( $node->is_leaf,   0,        'is_leaf is false' );
    ok( ref($node->children) eq 'ARRAY', 'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok = { type => 'INTEGER', value => '42', line => 1, col => 7 };
    my $leaf = CodingAdventures::TomlParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => $tok,
    );
    is( $leaf->is_leaf,         1,         'is_leaf is true' );
    is( $leaf->token->{type},   'INTEGER', 'token type is INTEGER' );
    is( $leaf->token->{value},  '42',      'token value is 42' );
    is( scalar @{ $leaf->children }, 0, 'leaf has no children' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'missing equals raises die' => sub {
    ok(
        dies { CodingAdventures::TomlParser->parse("key value\n") },
        'missing equals causes die'
    );
};

subtest 'unterminated inline array raises die' => sub {
    ok(
        dies { CodingAdventures::TomlParser->parse("ports = [1, 2\n") },
        'unterminated array causes die'
    );
};

subtest 'unterminated table header raises die' => sub {
    ok(
        dies { CodingAdventures::TomlParser->parse("[server\n") },
        'unterminated table header causes die'
    );
};

done_testing;
