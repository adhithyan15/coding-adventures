use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::CssParser; 1 }, 'module loads' );

# ============================================================================
# Helpers
# ============================================================================

# Walk the AST depth-first and collect all leaf token values.
sub collect_values {
    my ($node) = @_;
    my @out;
    _walk_values($node, \@out);
    return \@out;
}

sub _walk_values {
    my ($node, $out) = @_;
    if ($node->is_leaf && $node->token) {
        push @$out, $node->token->{value};
    } elsif ($node->children) {
        _walk_values($_, $out) for @{ $node->children };
    }
}

# Walk the AST depth-first and collect all leaf token types.
sub collect_types {
    my ($node) = @_;
    my @out;
    _walk_types($node, \@out);
    return \@out;
}

sub _walk_types {
    my ($node, $out) = @_;
    if ($node->is_leaf && $node->token) {
        push @$out, $node->token->{type};
    } elsif ($node->children) {
        _walk_types($_, $out) for @{ $node->children };
    }
}

# Breadth-first search for first node with given rule_name.
sub find_node {
    my ($node, $rule) = @_;
    my @queue = ($node);
    while (@queue) {
        my $n = shift @queue;
        return $n if $n->rule_name eq $rule;
        push @queue, @{ $n->children } if $n->children;
    }
    return undef;
}

# ============================================================================
# Empty and trivial inputs
# ============================================================================

subtest 'empty string parses to stylesheet' => sub {
    my $ast = CodingAdventures::CssParser->parse('');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

subtest 'whitespace-only parses to stylesheet' => sub {
    my $ast = CodingAdventures::CssParser->parse("   \n\t  ");
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

subtest 'comment-only parses to stylesheet' => sub {
    my $ast = CodingAdventures::CssParser->parse('/* just a comment */');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

# ============================================================================
# Qualified rules
# ============================================================================

subtest 'simple rule: h1 { color: red; }' => sub {
    my $ast = CodingAdventures::CssParser->parse('h1 { color: red; }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );

    my $qr = find_node($ast, 'qualified_rule');
    ok( $qr, 'has qualified_rule node' );

    my $sl = find_node($ast, 'selector_list');
    ok( $sl, 'has selector_list node' );

    my $block = find_node($ast, 'block');
    ok( $block, 'has block node' );

    my $decl = find_node($ast, 'declaration');
    ok( $decl, 'has declaration node' );

    # Token values should include h1, {, color, :, red, ;, }
    my $vals = collect_values($ast);
    my %val_set = map { $_ => 1 } @$vals;
    ok( $val_set{h1},    'AST contains h1' );
    ok( $val_set{color}, 'AST contains color' );
    ok( $val_set{red},   'AST contains red' );
};

subtest 'class selector rule: .active { display: block; }' => sub {
    my $ast = CodingAdventures::CssParser->parse('.active { display: block; }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
    ok( find_node($ast, 'class_selector'), 'has class_selector node' );
};

# ============================================================================
# Selectors
# ============================================================================

subtest 'ID selector: #header { }' => sub {
    my $ast = CodingAdventures::CssParser->parse('#header { }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

subtest 'attribute selector: [disabled] { }' => sub {
    my $ast = CodingAdventures::CssParser->parse('[disabled] { }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
    ok( find_node($ast, 'attribute_selector'), 'has attribute_selector node' );
};

subtest 'attribute selector with value: [type="text"] { }' => sub {
    my $ast = CodingAdventures::CssParser->parse('[type="text"] { }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

subtest 'pseudo-class: a:hover { }' => sub {
    my $ast = CodingAdventures::CssParser->parse('a:hover { }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
    ok( find_node($ast, 'pseudo_class'), 'has pseudo_class node' );
};

subtest 'pseudo-element: p::before { }' => sub {
    my $ast = CodingAdventures::CssParser->parse('p::before { }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
    ok( find_node($ast, 'pseudo_element'), 'has pseudo_element node' );
};

subtest 'child combinator: div > p { }' => sub {
    my $ast = CodingAdventures::CssParser->parse('div > p { }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

subtest 'comma selector list: h1, h2, h3 { }' => sub {
    my $ast = CodingAdventures::CssParser->parse('h1, h2, h3 { }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
    ok( find_node($ast, 'selector_list'), 'has selector_list node' );
};

# ============================================================================
# Declarations with compound tokens
# ============================================================================

subtest 'DIMENSION in value: font-size: 16px' => sub {
    my $ast = CodingAdventures::CssParser->parse('p { font-size: 16px; }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
    my $types = collect_types($ast);
    my %type_set = map { $_ => 1 } @$types;
    ok( $type_set{DIMENSION}, 'DIMENSION token in AST' );
};

subtest 'PERCENTAGE in value: width: 50%' => sub {
    my $ast = CodingAdventures::CssParser->parse('div { width: 50%; }');
    my $types = collect_types($ast);
    my %type_set = map { $_ => 1 } @$types;
    ok( $type_set{PERCENTAGE}, 'PERCENTAGE token in AST' );
};

subtest 'HASH in value: color: #333' => sub {
    my $ast = CodingAdventures::CssParser->parse('p { color: #333; }');
    my $types = collect_types($ast);
    my %type_set = map { $_ => 1 } @$types;
    ok( $type_set{HASH}, 'HASH token in AST' );
};

subtest 'multi-value: margin: 10px 20px 10px 20px' => sub {
    my $ast = CodingAdventures::CssParser->parse('div { margin: 10px 20px 10px 20px; }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

subtest '!important: color: red !important' => sub {
    my $ast = CodingAdventures::CssParser->parse('p { color: red !important; }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
    ok( find_node($ast, 'priority'), 'has priority node' );
};

# ============================================================================
# Function values
# ============================================================================

subtest 'rgba() function value' => sub {
    my $ast = CodingAdventures::CssParser->parse('p { color: rgba(255, 0, 0, 0.5); }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
    ok( find_node($ast, 'function_call'), 'has function_call node' );
};

subtest 'calc() with mixed units' => sub {
    my $ast = CodingAdventures::CssParser->parse('div { width: calc(100% - 20px); }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

subtest 'url() token in value' => sub {
    my $ast = CodingAdventures::CssParser->parse('div { background: url(./bg.png); }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

subtest 'var() CSS variable reference' => sub {
    my $ast = CodingAdventures::CssParser->parse('p { color: var(--main-color); }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

# ============================================================================
# At-rules
# ============================================================================

subtest '@import with semicolon' => sub {
    my $ast = CodingAdventures::CssParser->parse('@import "style.css";');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
    ok( find_node($ast, 'at_rule'), 'has at_rule node' );
};

subtest '@charset with semicolon' => sub {
    my $ast = CodingAdventures::CssParser->parse('@charset "UTF-8";');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

subtest '@media with block' => sub {
    my $ast = CodingAdventures::CssParser->parse('@media screen { h1 { color: red; } }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
    ok( find_node($ast, 'at_rule'), 'has at_rule node' );
};

subtest '@media with min-width query' => sub {
    my $ast = CodingAdventures::CssParser->parse('@media (min-width: 768px) { }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

subtest '@keyframes block' => sub {
    my $ast = CodingAdventures::CssParser->parse(
        '@keyframes fade { from { opacity: 0; } to { opacity: 1; } }'
    );
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

subtest '@font-face block' => sub {
    my $ast = CodingAdventures::CssParser->parse(
        '@font-face { font-family: "MyFont"; }'
    );
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

# ============================================================================
# Custom properties
# ============================================================================

subtest 'custom property declaration: --main-color: #333' => sub {
    my $ast = CodingAdventures::CssParser->parse(':root { --main-color: #333; }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

subtest 'custom property token in AST' => sub {
    my $ast = CodingAdventures::CssParser->parse(':root { --bg: white; }');
    my $types = collect_types($ast);
    my %type_set = map { $_ => 1 } @$types;
    ok( $type_set{CUSTOM_PROPERTY}, 'CUSTOM_PROPERTY token in AST' );
};

# ============================================================================
# Multiple rules
# ============================================================================

subtest 'two qualified rules' => sub {
    my $ast = CodingAdventures::CssParser->parse(
        'h1 { color: red; } p { color: blue; }'
    );
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

subtest 'three rules' => sub {
    my $ast = CodingAdventures::CssParser->parse('h1 { } h2 { } h3 { }');
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

subtest 'mixed at-rule and qualified rule' => sub {
    my $ast = CodingAdventures::CssParser->parse(
        '@import "reset.css"; h1 { color: red; }'
    );
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
};

# ============================================================================
# ASTNode interface
# ============================================================================

subtest 'ASTNode interface' => sub {
    my $ast = CodingAdventures::CssParser->parse('h1 { color: red; }');

    # Root is a rule node (not a leaf)
    ok( !$ast->is_leaf,     'root is_leaf is false' );
    ok( $ast->children,     'root has children' );
    is( ref($ast->children), 'ARRAY', 'children is arrayref' );

    # Walk to a leaf
    my $leaf = undef;
    my @queue = ($ast);
    while (@queue && !$leaf) {
        my $n = shift @queue;
        if ($n->is_leaf) {
            $leaf = $n;
        } else {
            push @queue, @{ $n->children };
        }
    }
    ok( $leaf,             'found a leaf node' );
    ok( $leaf->token,      'leaf has a token' );
    ok( $leaf->token->{type},  'token has type' );
    ok( $leaf->token->{value}, 'token has value' );
    ok( $leaf->token->{line},  'token has line' );
    ok( $leaf->token->{col},   'token has col' );
};

done_testing;
