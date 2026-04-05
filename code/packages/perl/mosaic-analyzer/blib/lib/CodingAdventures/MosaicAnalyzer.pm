package CodingAdventures::MosaicAnalyzer;

# ============================================================================
# CodingAdventures::MosaicAnalyzer — Walks a Mosaic AST and produces a typed IR
# ============================================================================
#
# The analyzer is the third stage of the Mosaic compiler pipeline:
#
#   Source text → Lexer → Tokens → Parser → AST → **Analyzer** → MosaicIR
#
# What the Analyzer Does
# ----------------------
#
# The AST from the parser is a faithful, unvalidated representation of the
# source text. The analyzer:
#
#   1. Strips syntax noise (braces, semicolons, colons) and retains only
#      semantically meaningful tokens.
#   2. Resolves types — converts keyword strings to typed hashrefs:
#      { kind => "text" }, { kind => "number" }, etc.
#   3. Normalizes values — "16dp" → { kind => "dimension", value => 16, unit => "dp" }
#   4. Determines required/optional — slots with defaults are optional.
#   5. Identifies primitives — Row/Column/Text/etc. are primitive nodes.
#
# MosaicIR structure (all hashrefs)
# ----------------------------------
#
#   component = {
#     name  => "ProfileCard",
#     slots => [ slot, ... ],
#     tree  => node,
#   }
#
#   slot = {
#     name         => "title",
#     type         => { kind => "text" },
#     default_value => undef | value,
#     required     => 1 | 0,
#   }
#
#   type kinds: text, number, bool, image, color, node, component, list
#   list: { kind => "list", element_type => type }
#   component: { kind => "component", name => "Button" }
#
#   node = {
#     tag          => "Column",
#     is_primitive => 1 | 0,
#     properties   => [ { name => "padding", value => value }, ... ],
#     children     => [ child, ... ],
#   }
#
#   child kinds: node, slot_ref, when, each
#
#   value kinds: string, number, bool, dimension, color_hex, ident, slot_ref, enum
#
# Public API
# ----------
#
#   my ($component, $error) = CodingAdventures::MosaicAnalyzer->analyze($source);

use strict;
use warnings;

use CodingAdventures::MosaicParser;

our $VERSION = '0.01';

# ============================================================================
# Primitive node registry
# ============================================================================
#
# These are the built-in layout and display elements. Any node tag not in
# this set is treated as a composite component (is_primitive => 0).

my %PRIMITIVE_NODES = map { $_ => 1 } qw(
    Row Column Box Stack Text Image Icon Spacer Divider Scroll
);

# ============================================================================
# Public API
# ============================================================================

sub analyze {
    my ($class, $source) = @_;

    my ($ast, $parse_err) = CodingAdventures::MosaicParser->parse($source);
    return (undef, $parse_err) if $parse_err;

    my $component = eval { _analyze_file($ast) };
    return (undef, $@) if $@;
    return ($component, undef);
}

# ============================================================================
# File-level analysis
# ============================================================================

sub _analyze_file {
    my ($ast) = @_;
    unless ($ast->{rule_name} eq 'file') {
        die "AnalysisError: Expected root rule 'file', got '$ast->{rule_name}'";
    }

    my $comp_decl;
    for my $child (@{ $ast->{children} }) {
        next unless ref $child eq 'HASH';
        $comp_decl = $child if $child->{rule_name} eq 'component_decl';
    }

    unless ($comp_decl) {
        die "AnalysisError: No component declaration found in file";
    }

    return _analyze_component($comp_decl);
}

# ============================================================================
# Component analysis
# ============================================================================

sub _analyze_component {
    my ($node) = @_;

    my $name = _first_token_value($node, 'NAME')
        or die "AnalysisError: component_decl missing name";

    my @slots;
    my $tree_node;

    for my $child (@{ $node->{children} }) {
        next unless ref $child eq 'HASH';
        if ($child->{rule_name} eq 'slot_decl') {
            push @slots, _analyze_slot($child);
        }
        elsif ($child->{rule_name} eq 'node_tree') {
            $tree_node = $child;
        }
    }

    unless ($tree_node) {
        die "AnalysisError: component '$name' has no node tree";
    }

    return {
        name  => $name,
        slots => \@slots,
        tree  => _analyze_node_tree($tree_node),
    };
}

# ============================================================================
# Slot analysis
# ============================================================================

sub _analyze_slot {
    my ($node) = @_;

    my $name = _first_token_value($node, 'NAME')
        or die "AnalysisError: slot_decl missing name";

    my $type = _analyze_slot_type($node);
    my $default_value = _analyze_slot_default($node);
    my $required = !defined $default_value;

    return {
        name          => $name,
        type          => $type,
        default_value => $default_value,
        required      => $required ? 1 : 0,
    };
}

sub _analyze_slot_type {
    my ($slot_node) = @_;

    # Look for list_type as a direct child first
    for my $child (@{ $slot_node->{children} }) {
        next unless ref $child eq 'HASH';
        if ($child->{rule_name} eq 'list_type') {
            return _analyze_list_type($child);
        }
        # slot_type node wraps the actual type token
        if ($child->{rule_name} eq 'slot_type') {
            return _analyze_slot_type_node($child);
        }
    }

    die "AnalysisError: slot_decl missing type";
}

sub _analyze_slot_type_node {
    my ($node) = @_;

    # Could contain a list_type or a direct KEYWORD/NAME token
    for my $child (@{ $node->{children} }) {
        next unless ref $child eq 'HASH';
        if ($child->{rule_name} eq 'list_type') {
            return _analyze_list_type($child);
        }
        if ($child->{is_leaf}) {
            my $tok = $child->{token};
            if ($tok->{type} eq 'KEYWORD') {
                return _parse_primitive_type($tok->{value});
            }
            if ($tok->{type} eq 'NAME') {
                return { kind => 'component', name => $tok->{value} };
            }
        }
    }

    die "AnalysisError: slot_type has no recognizable content";
}

sub _analyze_list_type {
    my ($node) = @_;

    # list_type = "list" "<" slot_type ">"
    # Find the inner slot_type node
    for my $child (@{ $node->{children} }) {
        next unless ref $child eq 'HASH';
        if ($child->{rule_name} eq 'slot_type') {
            my $element_type = _analyze_slot_type_node($child);
            return { kind => 'list', element_type => $element_type };
        }
    }

    die "AnalysisError: list_type missing element type";
}

sub _parse_primitive_type {
    my ($kw) = @_;
    my %TYPES = (
        text   => { kind => 'text'   },
        number => { kind => 'number' },
        bool   => { kind => 'bool'   },
        image  => { kind => 'image'  },
        color  => { kind => 'color'  },
        node   => { kind => 'node'   },
    );
    return $TYPES{$kw} if exists $TYPES{$kw};
    die "AnalysisError: Unknown primitive type keyword: '$kw'";
}

sub _analyze_slot_default {
    my ($slot_node) = @_;
    for my $child (@{ $slot_node->{children} }) {
        next unless ref $child eq 'HASH';
        if ($child->{rule_name} eq 'default_value') {
            return _analyze_default_value($child);
        }
    }
    return undef;
}

sub _analyze_default_value {
    my ($node) = @_;
    for my $child (@{ $node->{children} }) {
        next unless ref $child eq 'HASH' && $child->{is_leaf};
        my $tok = $child->{token};
        return _token_to_value($tok);
    }
    die "AnalysisError: default_value has no recognizable content";
}

# ============================================================================
# Node tree analysis
# ============================================================================

sub _analyze_node_tree {
    my ($node) = @_;
    for my $child (@{ $node->{children} }) {
        next unless ref $child eq 'HASH';
        if ($child->{rule_name} eq 'node_element') {
            return _analyze_node_element($child);
        }
    }
    die "AnalysisError: node_tree missing node_element";
}

sub _analyze_node_element {
    my ($node) = @_;

    my $tag = _first_token_value($node, 'NAME')
        or die "AnalysisError: node_element missing tag name";

    my $is_primitive = exists $PRIMITIVE_NODES{$tag} ? 1 : 0;
    my @properties;
    my @children;

    for my $child (@{ $node->{children} }) {
        next unless ref $child eq 'HASH';
        if ($child->{rule_name} eq 'node_content') {
            my ($prop, $child_item) = _analyze_node_content($child);
            push @properties, $prop  if defined $prop;
            push @children,   $child_item if defined $child_item;
        }
    }

    return {
        tag          => $tag,
        is_primitive => $is_primitive,
        properties   => \@properties,
        children     => \@children,
    };
}

sub _analyze_node_content {
    my ($node) = @_;

    for my $child (@{ $node->{children} }) {
        next unless ref $child eq 'HASH';

        if ($child->{rule_name} eq 'property_assignment') {
            return (_analyze_property_assignment($child), undef);
        }
        if ($child->{rule_name} eq 'child_node') {
            my $elem = _find_child($child, 'node_element');
            return (undef, { kind => 'node', node => _analyze_node_element($elem) }) if $elem;
        }
        if ($child->{rule_name} eq 'slot_reference') {
            my $name = _first_token_value($child, 'NAME');
            return (undef, { kind => 'slot_ref', slot_name => $name }) if $name;
        }
        if ($child->{rule_name} eq 'when_block') {
            return (undef, _analyze_when_block($child));
        }
        if ($child->{rule_name} eq 'each_block') {
            return (undef, _analyze_each_block($child));
        }
    }

    return (undef, undef);
}

# ============================================================================
# Property analysis
# ============================================================================

sub _analyze_property_assignment {
    my ($node) = @_;

    # Property name may be a NAME or KEYWORD token
    my $name;
    for my $child (@{ $node->{children} }) {
        next unless ref $child eq 'HASH' && $child->{is_leaf};
        if ($child->{token}{type} =~ /^(NAME|KEYWORD)$/) {
            $name = $child->{token}{value};
            last;
        }
    }
    die "AnalysisError: property_assignment missing name" unless defined $name;

    my $value_node = _find_child($node, 'property_value')
        or die "AnalysisError: property '$name' missing value";

    return { name => $name, value => _analyze_property_value($value_node) };
}

sub _analyze_property_value {
    my ($node) = @_;

    for my $child (@{ $node->{children} }) {
        next unless ref $child eq 'HASH';

        if ($child->{rule_name} eq 'slot_ref') {
            my $name = _first_token_value($child, 'NAME');
            return { kind => 'slot_ref', slot_name => $name } if $name;
        }
        if ($child->{rule_name} eq 'enum_value') {
            my @names;
            for my $c (@{ $child->{children} }) {
                next unless ref $c eq 'HASH' && $c->{is_leaf};
                push @names, $c->{token}{value} if $c->{token}{type} eq 'NAME';
            }
            return { kind => 'enum', namespace => $names[0], member => $names[1] }
                if @names >= 2;
        }
        if ($child->{is_leaf}) {
            return _token_to_value($child->{token});
        }
    }

    die "AnalysisError: property_value has no recognizable content";
}

# ============================================================================
# When / Each block analysis
# ============================================================================

sub _analyze_when_block {
    my ($node) = @_;

    my $slot_ref = _find_child($node, 'slot_ref')
        or die "AnalysisError: when_block missing slot_ref";

    my $slot_name = _first_token_value($slot_ref, 'NAME')
        or die "AnalysisError: when_block slot_ref missing name";

    my @children = _collect_node_contents($node);

    return { kind => 'when', slot_name => $slot_name, children => \@children };
}

sub _analyze_each_block {
    my ($node) = @_;

    my $slot_ref = _find_child($node, 'slot_ref')
        or die "AnalysisError: each_block missing slot_ref";

    my $slot_name = _first_token_value($slot_ref, 'NAME')
        or die "AnalysisError: each_block slot_ref missing name";

    # The loop variable (item_name) is a NAME token AFTER the "as" keyword,
    # as a direct child of the each_block node (not inside slot_ref).
    my $item_name = _find_loop_variable($node, $slot_ref);
    die "AnalysisError: each_block missing loop variable name" unless defined $item_name;

    my @children = _collect_node_contents($node);

    return {
        kind      => 'each',
        slot_name => $slot_name,
        item_name => $item_name,
        children  => \@children,
    };
}

# Find the loop variable name in an each_block.
# Grammar: "each" slot_ref "as" NAME "{" ... "}"
# We scan direct children of each_block, skipping the slot_ref subtree,
# and find the NAME token after the "as" keyword.
sub _find_loop_variable {
    my ($each_block, $slot_ref) = @_;
    my $after_as = 0;
    for my $child (@{ $each_block->{children} }) {
        next unless ref $child eq 'HASH';
        next if $child == $slot_ref;   # skip slot_ref subtree
        next if $child->{rule_name} eq 'slot_ref';

        if ($child->{is_leaf}) {
            my $tok = $child->{token};
            if ($tok->{type} eq 'KEYWORD' && $tok->{value} eq 'as') {
                $after_as = 1;
                next;
            }
            if ($after_as && $tok->{type} eq 'NAME') {
                return $tok->{value};
            }
        }
    }
    return undef;
}

sub _collect_node_contents {
    my ($node) = @_;
    my @results;
    for my $child (@{ $node->{children} }) {
        next unless ref $child eq 'HASH';
        if ($child->{rule_name} eq 'node_content') {
            my ($prop, $child_item) = _analyze_node_content($child);
            push @results, $child_item if defined $child_item;
        }
    }
    return @results;
}

# ============================================================================
# Value helpers
# ============================================================================

sub _token_to_value {
    my ($tok) = @_;
    my $type = $tok->{type};
    my $val  = $tok->{value};

    if ($type eq 'STRING')   { return { kind => 'string', value => $val } }
    if ($type eq 'DIMENSION') {
        if ($val =~ /^(-?[0-9]*\.?[0-9]+)([a-zA-Z%]+)$/) {
            return { kind => 'dimension', value => $1 + 0, unit => $2 };
        }
        die "AnalysisError: Invalid DIMENSION token: '$val'";
    }
    if ($type eq 'NUMBER')   { return { kind => 'number', value => $val + 0 } }
    if ($type eq 'HEX_COLOR') { return { kind => 'color_hex', value => $val } }
    if ($type eq 'KEYWORD') {
        return { kind => 'bool', value => 1 } if $val eq 'true';
        return { kind => 'bool', value => 0 } if $val eq 'false';
        return { kind => 'ident', value => $val };
    }
    if ($type eq 'NAME') { return { kind => 'ident', value => $val } }

    die "AnalysisError: Unrecognized token type '$type' in value context";
}

# ============================================================================
# AST traversal helpers
# ============================================================================

# Find first direct child with given rule_name.
sub _find_child {
    my ($node, $rule) = @_;
    for my $child (@{ $node->{children} // [] }) {
        return $child if ref $child eq 'HASH' && $child->{rule_name} eq $rule;
    }
    return undef;
}

# Get the value of the first direct-child token with the given type.
sub _first_token_value {
    my ($node, $type) = @_;
    for my $child (@{ $node->{children} // [] }) {
        next unless ref $child eq 'HASH' && $child->{is_leaf};
        return $child->{token}{value} if $child->{token}{type} eq $type;
    }
    return undef;
}

1;

__END__

=head1 NAME

CodingAdventures::MosaicAnalyzer - Walks a Mosaic AST and produces a typed IR

=head1 SYNOPSIS

    use CodingAdventures::MosaicAnalyzer;

    my ($component, $error) = CodingAdventures::MosaicAnalyzer->analyze($source);
    die $error if $error;
    print $component->{name};
    for my $slot (@{ $component->{slots} }) {
        printf "  slot %s: %s\n", $slot->{name}, $slot->{type}{kind};
    }

=head1 DESCRIPTION

Port of the TypeScript mosaic-analyzer to Perl. Walks the AST produced by
CodingAdventures::MosaicParser and produces a structured MosaicIR hashref
with typed slots, resolved values, and a normalized node tree.

=head1 METHODS

=head2 analyze($source)

Analyze Mosaic source text. Returns C<($component_hashref, undef)> on success
or C<(undef, $error_string)> on failure.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
