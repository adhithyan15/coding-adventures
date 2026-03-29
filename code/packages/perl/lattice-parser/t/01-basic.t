use strict;
use warnings;
use Test2::V0;

use CodingAdventures::LatticeParser;
use CodingAdventures::LatticeParser::ASTNode;

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
# Helper: count nodes with given rule_name (depth-first)
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
# Root node and empty stylesheet
# ============================================================================

subtest 'parse returns an ASTNode' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('h1 { color: red; }');
    ok( defined $ast, 'parse returns defined' );
    ok( ref($ast),    'parse returns a reference' );
};

subtest 'root node has rule_name stylesheet' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('h1 { color: red; }');
    is( $ast->rule_name, 'stylesheet', 'root rule_name is stylesheet' );
};

subtest 'root node children is arrayref' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('h1 { color: red; }');
    ok( ref($ast->children) eq 'ARRAY', 'children is arrayref' );
};

subtest 'empty stylesheet' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('');
    is( $ast->rule_name, 'stylesheet', 'empty stylesheet root' );
    is( scalar @{ $ast->children }, 0, 'empty stylesheet has no children' );
};

# ============================================================================
# Plain CSS rules
# ============================================================================

subtest 'simple CSS type selector rule' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('h1 { color: red; }');
    my $qr = find_node($ast, 'qualified_rule');
    ok( defined $qr, 'found qualified_rule' );
};

subtest 'CSS class selector' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('.box { display: block; }');
    ok( defined find_node($ast, 'qualified_rule'), 'found qualified_rule' );
    ok( defined find_node($ast, 'class_selector'), 'found class_selector' );
};

subtest 'CSS declaration node' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('p { margin: 0; padding: 0; }');
    ok( count_nodes($ast, 'declaration') >= 2, 'at least two declarations' );
};

subtest 'CSS @media at-rule' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('@media screen { body { margin: 0; } }');
    ok( defined find_node($ast, 'at_rule'), 'found at_rule for @media' );
};

subtest 'CSS id selector' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('#main { width: 100%; }');
    ok( defined find_node($ast, 'id_selector'), 'found id_selector' );
};

subtest 'CSS attribute selector' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('input[type="text"] { border: 1px; }');
    ok( defined find_node($ast, 'attribute_selector'), 'found attribute_selector' );
};

subtest 'CSS pseudo-class' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('a:hover { color: blue; }');
    ok( defined find_node($ast, 'pseudo_class'), 'found pseudo_class' );
};

subtest 'CSS pseudo-element' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('p::first-line { font-weight: bold; }');
    ok( defined find_node($ast, 'pseudo_element'), 'found pseudo_element' );
};

subtest 'multiple declarations in one block' => sub {
    my $src = 'body { font-size: 16px; line-height: 1.5; color: #333; }';
    my $ast = CodingAdventures::LatticeParser->parse($src);
    ok( count_nodes($ast, 'declaration') >= 3, 'at least three declarations' );
};

# ============================================================================
# Variable declarations
# ============================================================================

subtest 'simple variable declaration' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('$primary: #4a90d9;');
    my $vd = find_node($ast, 'variable_declaration');
    ok( defined $vd, 'found variable_declaration' );
};

subtest 'variable with dimension value' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('$size: 16px;');
    ok( defined find_node($ast, 'variable_declaration'), 'found variable_declaration' );
};

subtest 'multiple variable declarations' => sub {
    my $src = '$a: 1px; $b: 2em; $c: red;';
    my $ast = CodingAdventures::LatticeParser->parse($src);
    is( count_nodes($ast, 'variable_declaration'), 3, 'three variable_declarations' );
};

subtest 'variable used in declaration value' => sub {
    my $ast = CodingAdventures::LatticeParser->parse(
        '$brand: #4a90d9; .btn { color: $brand; }'
    );
    ok( defined find_node($ast, 'variable_declaration'), 'found variable_declaration' );
    ok( defined find_node($ast, 'declaration'),          'found declaration' );
};

# ============================================================================
# Nested rules
# ============================================================================

subtest 'nested child rule' => sub {
    my $src = '.parent { .child { color: blue; } }';
    my $ast = CodingAdventures::LatticeParser->parse($src);
    ok( count_nodes($ast, 'qualified_rule') >= 2, 'at least two qualified_rules' );
};

subtest 'rule with declaration and nested rule' => sub {
    my $src = '.nav { display: flex; .item { color: white; } }';
    my $ast = CodingAdventures::LatticeParser->parse($src);
    ok( defined find_node($ast, 'declaration'), 'found declaration' );
    ok( count_nodes($ast, 'qualified_rule') >= 2, 'at least two qualified_rules' );
};

# ============================================================================
# Mixin definitions
# ============================================================================

subtest 'simple no-param mixin (IDENT form)' => sub {
    my $ast = CodingAdventures::LatticeParser->parse(
        '@mixin clearfix { &::after { content: ""; clear: both; display: table; } }'
    );
    ok( defined find_node($ast, 'mixin_definition'), 'found mixin_definition' );
};

subtest 'mixin with parameters (FUNCTION form)' => sub {
    my $ast = CodingAdventures::LatticeParser->parse(
        '@mixin button($bg, $fg) { background: $bg; color: $fg; }'
    );
    ok( defined find_node($ast, 'mixin_definition'), 'found mixin_definition' );
    ok( defined find_node($ast, 'mixin_params'),     'found mixin_params' );
};

subtest 'mixin with default parameter' => sub {
    my $ast = CodingAdventures::LatticeParser->parse(
        '@mixin shadow($blur: 4px) { box-shadow: 0 $blur black; }'
    );
    ok( defined find_node($ast, 'mixin_definition'), 'found mixin_definition' );
};

# ============================================================================
# @include directives
# ============================================================================

subtest '@include without arguments (IDENT form)' => sub {
    my $ast = CodingAdventures::LatticeParser->parse(
        '.box { @include clearfix; }'
    );
    ok( defined find_node($ast, 'include_directive'), 'found include_directive' );
};

subtest '@include with arguments (FUNCTION form)' => sub {
    my $ast = CodingAdventures::LatticeParser->parse(
        '.btn { @include button(red); }'
    );
    ok( defined find_node($ast, 'include_directive'), 'found include_directive' );
};

# ============================================================================
# @if control flow
# ============================================================================

subtest 'simple @if directive' => sub {
    my $ast = CodingAdventures::LatticeParser->parse(
        '@if $debug { color: red; }'
    );
    ok( defined find_node($ast, 'if_directive'), 'found if_directive' );
};

subtest '@if with comparison operator' => sub {
    my $ast = CodingAdventures::LatticeParser->parse(
        '@if $size == large { font-size: 24px; }'
    );
    ok( defined find_node($ast, 'if_directive'),     'found if_directive' );
    ok( defined find_node($ast, 'comparison_op'),    'found comparison_op' );
};

subtest '@if ... @else' => sub {
    my $ast = CodingAdventures::LatticeParser->parse(
        '@if $dark { background: #1a1a1a; } @else { background: white; }'
    );
    ok( defined find_node($ast, 'if_directive'), 'found if_directive' );
};

# ============================================================================
# @for loops
# ============================================================================

subtest '@for ... through loop' => sub {
    my $ast = CodingAdventures::LatticeParser->parse(
        '@for $i from 1 through 3 { .item { margin: 0; } }'
    );
    ok( defined find_node($ast, 'for_directive'), 'found for_directive' );
};

subtest '@for ... to loop (exclusive)' => sub {
    my $ast = CodingAdventures::LatticeParser->parse(
        '@for $i from 0 to 5 { .step { color: red; } }'
    );
    ok( defined find_node($ast, 'for_directive'), 'found for_directive' );
};

# ============================================================================
# @each loops
# ============================================================================

subtest '@each loop over a list' => sub {
    my $ast = CodingAdventures::LatticeParser->parse(
        '@each $color in red, green, blue { .text { color: $color; } }'
    );
    ok( defined find_node($ast, 'each_directive'), 'found each_directive' );
};

# ============================================================================
# @function definitions
# ============================================================================

subtest 'function definition with @return (FUNCTION form)' => sub {
    my $ast = CodingAdventures::LatticeParser->parse(
        '@function spacing($n) { @return $n * 8px; }'
    );
    ok( defined find_node($ast, 'function_definition'), 'found function_definition' );
    ok( defined find_node($ast, 'return_directive'),    'found return_directive' );
};

subtest 'no-param function (IDENT form)' => sub {
    my $ast = CodingAdventures::LatticeParser->parse(
        '@function pi { @return 3.14159; }'
    );
    ok( defined find_node($ast, 'function_definition'), 'found function_definition' );
};

# ============================================================================
# @use directives
# ============================================================================

subtest '@use simple' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('@use "colors";');
    ok( defined find_node($ast, 'use_directive'), 'found use_directive' );
};

subtest '@use with as alias' => sub {
    my $ast = CodingAdventures::LatticeParser->parse('@use "utils/mixins" as m;');
    ok( defined find_node($ast, 'use_directive'), 'found use_directive' );
};

# ============================================================================
# Multi-rule stylesheets
# ============================================================================

subtest 'realistic multi-rule stylesheet' => sub {
    my $src = <<'LATTICE';
$primary: #4a90d9;
$font-stack: Helvetica, sans-serif;

@mixin flex-center {
  display: flex;
  align-items: center;
  justify-content: center;
}

body {
  font-family: $font-stack;
  background: white;
}

.hero {
  @include flex-center;
  color: $primary;
}
LATTICE

    my $ast = CodingAdventures::LatticeParser->parse($src);
    is( $ast->rule_name, 'stylesheet', 'root is stylesheet' );
    ok( count_nodes($ast, 'variable_declaration') >= 2, 'at least two variable_declarations' );
    ok( count_nodes($ast, 'mixin_definition')     >= 1, 'at least one mixin_definition' );
    ok( count_nodes($ast, 'qualified_rule')       >= 2, 'at least two qualified_rules' );
    ok( count_nodes($ast, 'include_directive')    >= 1, 'at least one include_directive' );
};

# ============================================================================
# ASTNode methods
# ============================================================================

subtest 'ASTNode new and accessors' => sub {
    my $node = CodingAdventures::LatticeParser::ASTNode->new(
        rule_name => 'variable_declaration',
        children  => [],
        is_leaf   => 0,
    );
    is( $node->rule_name, 'variable_declaration', 'rule_name accessor' );
    is( $node->is_leaf,   0,                       'is_leaf is false' );
    ok( ref($node->children) eq 'ARRAY', 'children is arrayref' );
};

subtest 'ASTNode leaf node' => sub {
    my $tok = { type => 'VARIABLE', value => '$primary', line => 1, col => 1 };
    my $leaf = CodingAdventures::LatticeParser::ASTNode->new(
        rule_name => 'token',
        is_leaf   => 1,
        token     => $tok,
    );
    is( $leaf->is_leaf,        1,          'is_leaf is true' );
    is( $leaf->token->{type},  'VARIABLE', 'token type is VARIABLE' );
    is( $leaf->token->{value}, '$primary', 'token value is $primary' );
    is( scalar @{ $leaf->children }, 0, 'leaf has no children' );
};

# ============================================================================
# Error handling
# ============================================================================

subtest 'block missing closing brace raises die' => sub {
    ok(
        dies { CodingAdventures::LatticeParser->parse('h1 { color: red;') },
        'unclosed block causes die'
    );
};

subtest 'variable declaration missing semicolon raises die' => sub {
    ok(
        dies { CodingAdventures::LatticeParser->parse('$x: 1') },
        'missing semicolon causes die'
    );
};

subtest '@for missing "from" raises die' => sub {
    ok(
        dies { CodingAdventures::LatticeParser->parse('@for $i 1 through 3 { }') },
        'missing "from" in @for causes die'
    );
};

done_testing;
