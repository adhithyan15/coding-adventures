use strict;
use warnings;
use Test2::V0;

use CodingAdventures::MosaicParser;

# ============================================================================
# Helpers
# ============================================================================

# Return the rule_name of the root node.
sub root_rule {
    my ($src) = @_;
    my ($ast, $err) = CodingAdventures::MosaicParser->parse($src);
    die "parse error: $err" if $err;
    return $ast->{rule_name};
}

# Find first child node with given rule_name.
sub find_child {
    my ($node, $rule) = @_;
    for my $c (@{ $node->{children} // [] }) {
        return $c if ref $c eq 'HASH' && $c->{rule_name} eq $rule;
    }
    return undef;
}

# Collect all children with given rule_name.
sub find_children {
    my ($node, $rule) = @_;
    grep { ref $_ eq 'HASH' && $_->{rule_name} eq $rule } @{ $node->{children} // [] };
}

# Extract token value from a leaf node or a child leaf.
sub token_value {
    my ($node, $type) = @_;
    for my $c (@{ $node->{children} // [] }) {
        next unless ref $c eq 'HASH';
        if ($c->{is_leaf} && $c->{token}{type} eq $type) {
            return $c->{token}{value};
        }
    }
    return undef;
}

# ============================================================================
# Minimal component
# ============================================================================

subtest 'minimal component parses without error' => sub {
    my ($ast, $err) = CodingAdventures::MosaicParser->parse(
        'component Empty { Box {} }'
    );
    is($err, undef, 'no error');
    is($ast->{rule_name}, 'file', 'root is file');
};

subtest 'root rule is file' => sub {
    is(root_rule('component A { Box {} }'), 'file', 'root is file');
};

# ============================================================================
# Component name
# ============================================================================

subtest 'component name is captured' => sub {
    my ($ast) = CodingAdventures::MosaicParser->parse('component MyCard { Box {} }');
    my $comp = find_child($ast, 'component_decl');
    my $name = token_value($comp, 'NAME');
    is($name, 'MyCard', 'component name is MyCard');
};

# ============================================================================
# Slot declarations
# ============================================================================

subtest 'single text slot' => sub {
    my ($ast) = CodingAdventures::MosaicParser->parse(
        'component A { slot title: text; Box {} }'
    );
    my $comp  = find_child($ast, 'component_decl');
    my @slots = find_children($comp, 'slot_decl');
    is(scalar @slots, 1, 'one slot');
    is(token_value($slots[0], 'NAME'), 'title', 'slot name is title');
};

subtest 'multiple slots' => sub {
    my ($ast) = CodingAdventures::MosaicParser->parse(
        'component A { slot a: text; slot b: number; Box {} }'
    );
    my $comp  = find_child($ast, 'component_decl');
    my @slots = find_children($comp, 'slot_decl');
    is(scalar @slots, 2, 'two slots');
};

subtest 'slot with default value' => sub {
    my ($ast) = CodingAdventures::MosaicParser->parse(
        'component A { slot count: number = 0; Box {} }'
    );
    my $comp = find_child($ast, 'component_decl');
    my ($slot) = find_children($comp, 'slot_decl');
    my $def = find_child($slot, 'default_value');
    ok(defined $def, 'default_value node present');
};

subtest 'list slot type' => sub {
    my ($ast) = CodingAdventures::MosaicParser->parse(
        'component A { slot items: list<text>; Box {} }'
    );
    my $comp = find_child($ast, 'component_decl');
    my ($slot) = find_children($comp, 'slot_decl');
    # The list_type is a direct child of slot_decl (slot_type production returns it)
    my $list_type = find_child($slot, 'list_type');
    ok(defined $list_type, 'list_type node present as child of slot_decl');
};

# ============================================================================
# Node tree
# ============================================================================

subtest 'node tree captured' => sub {
    my ($ast) = CodingAdventures::MosaicParser->parse(
        'component A { Column {} }'
    );
    my $comp = find_child($ast, 'component_decl');
    my $tree = find_child($comp, 'node_tree');
    ok(defined $tree, 'node_tree present');
    my $elem = find_child($tree, 'node_element');
    ok(defined $elem, 'node_element present');
    is(token_value($elem, 'NAME'), 'Column', 'root element is Column');
};

subtest 'nested node elements' => sub {
    my ($ast) = CodingAdventures::MosaicParser->parse(
        'component A { Column { Text {} } }'
    );
    my $comp = find_child($ast, 'component_decl');
    my $tree = find_child($comp, 'node_tree');
    my $col  = find_child($tree, 'node_element');
    # Find node_content that contains a child_node
    my ($content) = grep {
        ref $_ eq 'HASH' && $_->{rule_name} eq 'node_content'
    } @{ $col->{children} };
    ok(defined $content, 'node_content present');
    my $child_node = find_child($content, 'child_node');
    ok(defined $child_node, 'child_node present');
};

# ============================================================================
# Property assignments
# ============================================================================

subtest 'property assignment captured' => sub {
    my ($ast) = CodingAdventures::MosaicParser->parse(
        'component A { Text { content: "hello"; } }'
    );
    my $comp  = find_child($ast, 'component_decl');
    my $tree  = find_child($comp, 'node_tree');
    my $elem  = find_child($tree, 'node_element');
    my ($cont) = grep {
        ref $_ eq 'HASH' && $_->{rule_name} eq 'node_content'
    } @{ $elem->{children} };
    my $prop = find_child($cont, 'property_assignment');
    ok(defined $prop, 'property_assignment present');
};

subtest 'slot ref as property value' => sub {
    my ($ast, $err) = CodingAdventures::MosaicParser->parse(
        'component A { slot title: text; Text { content: @title; } }'
    );
    is($err, undef, 'no error');
    ok(defined $ast, 'ast defined');
};

# ============================================================================
# when / each blocks
# ============================================================================

subtest 'when block parses' => sub {
    my ($ast, $err) = CodingAdventures::MosaicParser->parse(
        'component A { slot show: bool; Column { when @show { Text {} } } }'
    );
    is($err, undef, 'no error');
    ok(defined $ast, 'ast defined');
};

subtest 'each block parses' => sub {
    my ($ast, $err) = CodingAdventures::MosaicParser->parse(
        'component A { slot items: list<text>; Column { each @items as item { Text {} } } }'
    );
    is($err, undef, 'no error');
    ok(defined $ast, 'ast defined');
};

# ============================================================================
# Error cases
# ============================================================================

subtest 'missing component keyword returns error' => sub {
    my ($ast, $err) = CodingAdventures::MosaicParser->parse('Box {}');
    ok(defined $err, 'error returned');
    is($ast, undef, 'no ast');
};

subtest 'unclosed brace returns error' => sub {
    my ($ast, $err) = CodingAdventures::MosaicParser->parse(
        'component A { Box {'
    );
    ok(defined $err, 'error returned');
};

done_testing;
