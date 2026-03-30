package CodingAdventures::LatticeAstToCss;

# ============================================================================
# CodingAdventures::LatticeAstToCss — Lattice AST → CSS compiler
# ============================================================================
#
# This module walks a Lattice AST (produced by CodingAdventures::LatticeParser)
# and emits plain CSS text.  It is the semantic core of the Lattice transpiler.
#
# # What is Lattice?
#
# Lattice is a CSS superset language.  Every valid CSS file is valid Lattice.
# On top of CSS3, Lattice adds:
#
#   Variables:     $primary: #4a90d9;   color: $primary;
#   Mixins:        @mixin button($bg) { background: $bg; }
#                  .btn { @include button(red); }
#   Control flow:  @if $debug { color: red; }
#                  @for $i from 1 through 3 { .col-#{$i} { ... } }
#                  @each $c in red, blue { .t { color: $c; } }
#   Functions:     @function spacing($n) { @return $n * 8px; }
#   Nesting:       .parent { .child { color: blue; } }
#
# # Architecture
#
# The compiler runs in two conceptual passes:
#
#   Pass 1 — Symbol Collection:
#     Walk the top-level stylesheet and collect all variable declarations,
#     mixin definitions, and function definitions into a root $env.
#
#   Pass 2 — Expansion and Emission:
#     Walk remaining AST nodes, expanding $var references, expanding
#     @include directives, unrolling @if/@for/@each, evaluating @function
#     calls, and flattening nested CSS rules.
#
# # Environment (Scope) Chain
#
# Each scope is a hashref:
#
#   $env = {
#     vars      => {},     # local variable bindings (name → string value)
#     mixins    => {},     # mixin definitions (only populated in root env)
#     functions => {},     # function definitions (only in root env)
#     parent    => undef,  # parent env ref (undef at root)
#   }
#
# Variable lookup walks up the parent chain.
# Mixin/function lookup always walks to the root.
#
# # AST Structure
#
# ASTNode objects have:
#   $node->rule_name  — grammar rule name ("qualified_rule", "declaration", …)
#   $node->children   — arrayref of child ASTNode or Token objects
#
# Token objects have:
#   $token->type      — token type ("VARIABLE", "IDENT", "FUNCTION", …)
#   $token->value     — raw token text ("$primary", "red", "spacing(", …)

use strict;
use warnings;

our $VERSION = '0.1.0';

# ============================================================================
# CSS built-in function names
# ============================================================================
#
# When we encounter a function_call node, we check if its name is a known
# CSS built-in.  If so, we emit it as-is (never treat it as a Lattice function).

my %CSS_FUNCTIONS = map { $_ => 1 } qw(
    rgb rgba hsl hsla hwb lab lch oklch oklab color color-mix
    calc min max clamp abs sign round mod rem
    sin cos tan asin acos atan atan2 pow sqrt hypot log exp
    var env url format local
    linear-gradient radial-gradient conic-gradient
    repeating-linear-gradient repeating-radial-gradient
    repeating-conic-gradient
    counter counters attr element
    translate translateX translateY translateZ
    rotate rotateX rotateY rotateZ
    scale scaleX scaleY scaleZ
    skew skewX skewY
    matrix matrix3d perspective
    cubic-bezier steps
    path polygon circle ellipse inset
    image-set cross-fade
    fit-content minmax repeat
    blur brightness contrast drop-shadow grayscale
    hue-rotate invert opacity saturate sepia
);

sub _is_css_function {
    my ($name) = @_;
    # FUNCTION tokens include "(" at the end: "rgb(" → check "rgb"
    (my $clean = $name) =~ s/\(+$//;
    return exists $CSS_FUNCTIONS{$clean};
}

# ============================================================================
# Environment helpers
# ============================================================================

sub _new_env {
    return { vars => {}, mixins => {}, functions => {}, parent => undef };
}

sub _child_env {
    my ($parent) = @_;
    return { vars => {}, mixins => {}, functions => {}, parent => $parent };
}

sub _lookup_var {
    my ($env, $name) = @_;
    my $e = $env;
    while ($e) {
        return $e->{vars}{$name} if exists $e->{vars}{$name};
        $e = $e->{parent};
    }
    return undef;
}

sub _root_env {
    my ($env) = @_;
    my $e = $env;
    $e = $e->{parent} while $e->{parent};
    return $e;
}

sub _lookup_mixin {
    my ($env, $name) = @_;
    return _root_env($env)->{mixins}{$name};
}

sub _lookup_function {
    my ($env, $name) = @_;
    return _root_env($env)->{functions}{$name};
}

sub _set_var {
    my ($env, $name, $val) = @_;
    $env->{vars}{$name} = $val;
}

# ============================================================================
# AST navigation helpers
# ============================================================================

sub _is_token {
    my ($node) = @_;
    return ref($node) && $node->can('is_leaf') && $node->is_leaf;
}

sub _is_node {
    my ($node) = @_;
    return ref($node) && $node->can('rule_name') && !$node->is_leaf;
}

sub _find_child {
    my ($children, $rule_name) = @_;
    for my $c (@$children) {
        return $c if _is_node($c) && $c->rule_name eq $rule_name;
    }
    return undef;
}

sub _find_children {
    my ($children, $rule_name) = @_;
    return grep { _is_node($_) && $_->rule_name eq $rule_name } @$children;
}

sub _find_token {
    my ($children, $tok_type) = @_;
    for my $c (@$children) {
        return $c if _is_token($c) && $c->type eq $tok_type;
    }
    return undef;
}

sub _find_tokens {
    my ($children, $tok_type) = @_;
    return grep { _is_token($_) && $_->type eq $tok_type } @$children;
}

# Collect all token text from a subtree.
sub _collect_text {
    my ($node) = @_;
    return '' unless defined $node;
    if (_is_token($node)) {
        return '"' . $node->value . '"' if $node->type eq 'STRING';
        return $node->value;
    }
    if (_is_node($node)) {
        return join('', map { _collect_text($_) } @{ $node->children // [] });
    }
    return '';
}

# ============================================================================
# Numeric helpers
# ============================================================================

sub _parse_numeric {
    my ($val) = @_;
    return ($val, '') if !defined $val;
    if ($val =~ /^(-?\d+\.?\d*)(.*)$/) {
        return ($1 + 0, $2);
    }
    return (undef, undef);
}

# ============================================================================
# Expression evaluation
# ============================================================================
#
# Lattice expressions appear in @if conditions, @for bounds, and @return.
# We evaluate them at compile time.

sub _eval_expression;  # forward
sub _eval_function_call;

sub _eval_primary {
    my ($node, $env) = @_;
    my $children = $node->children // [];

    if (@$children == 1) {
        my $c = $children->[0];
        if (_is_token($c)) {
            if ($c->type eq 'VARIABLE') {
                my $name = substr($c->value, 1);  # strip '$'
                my $val  = _lookup_var($env, $name);
                return defined $val ? $val : $c->value;
            }
            if ($c->type eq 'NUMBER')     { return $c->value + 0 }
            if ($c->type eq 'DIMENSION')  { return $c->value }
            if ($c->type eq 'PERCENTAGE') { return $c->value }
            if ($c->type eq 'STRING')     { return $c->value }
            if ($c->type eq 'IDENT') {
                return 1  if $c->value eq 'true';
                return 0  if $c->value eq 'false';
                return '' if $c->value eq 'null';
                return $c->value;
            }
            if ($c->type eq 'HASH') { return $c->value }
        }
        if (_is_node($c)) {
            if ($c->rule_name eq 'function_call') {
                return _eval_function_call($c, $env);
            }
            if ($c->rule_name eq 'map_literal') {
                return _collect_text($c);
            }
        }
    }

    # Parenthesized expression: LPAREN lattice_expression RPAREN
    for my $c (@$children) {
        if (_is_node($c) && $c->rule_name eq 'lattice_expression') {
            return _eval_expression($c, $env);
        }
    }

    # "true" / "false" / "null" as bare IDENT tokens in children
    for my $c (@$children) {
        if (_is_token($c)) {
            return 1  if $c->value eq 'true';
            return 0  if $c->value eq 'false';
            return '' if $c->value eq 'null';
        }
    }

    return _collect_text($node);
}

sub _eval_function_call {
    my ($node, $env) = @_;
    my $children = $node->children // [];
    my $func_tok = _find_token($children, 'FUNCTION');
    return _collect_text($node) unless $func_tok;

    my $func_name_raw = $func_tok->value;
    (my $func_name = $func_name_raw) =~ s/\(+$//;

    return _collect_text($node) if _is_css_function($func_name_raw);

    my $func_def = _lookup_function($env, $func_name);
    return _collect_text($node) unless $func_def;

    # Evaluate arguments
    my $args_node = _find_child($children, 'function_args');
    my @arg_vals  = _eval_include_args($args_node, $env);

    # Build call scope
    my $call_env = _child_env($env);
    my @params = @{ $func_def->{params} // [] };
    for my $i (0 .. $#params) {
        my $param_name = $params[$i];
        (my $param_key = $param_name) =~ s/^\$//;
        my $val = $arg_vals[$i];
        unless (defined $val) {
            my $default = $func_def->{defaults}{$param_name};
            $val = $default if defined $default;
        }
        _set_var($call_env, $param_key, $val) if defined $val;
    }

    return _exec_function_body($func_def->{body}, $call_env) // '';
}

sub _exec_function_body {
    my ($body_node, $env) = @_;
    my $items = $body_node->children // [];
    for my $item (@$items) {
        next unless _is_node($item);
        my @inner = $item->rule_name eq 'function_body_item'
                    ? @{ $item->children // [] }
                    : ($item);
        for my $inner (@inner) {
            next unless _is_node($inner);
            if ($inner->rule_name eq 'return_directive') {
                my $expr = _find_child($inner->children, 'lattice_expression');
                return _eval_expression($expr, $env) if $expr;
            }
            elsif ($inner->rule_name eq 'variable_declaration') {
                _exec_variable_decl($inner, $env);
            }
        }
    }
    return undef;
}

sub _exec_variable_decl {
    my ($node, $env) = @_;
    my $children = $node->children // [];
    my $var_tok  = _find_token($children, 'VARIABLE') or return;
    (my $name = $var_tok->value) =~ s/^\$//;
    my $vl_node  = _find_child($children, 'value_list');
    if ($vl_node) {
        my $val = _eval_value_list_node($vl_node, $env);
        _set_var($env, $name, $val);
    }
}

sub _eval_value_list_node {
    my ($node, $env) = @_;
    return '' unless _is_node($node);
    return join(' ', map { _eval_value_node($_, $env) } @{ $node->children // [] });
}

sub _eval_value_node {
    my ($node, $env) = @_;
    if (_is_token($node)) {
        if ($node->type eq 'VARIABLE') {
            (my $name = $node->value) =~ s/^\$//;
            my $val = _lookup_var($env, $name);
            return defined $val ? $val : $node->value;
        }
        return '"' . $node->value . '"' if $node->type eq 'STRING';
        return $node->value;
    }
    if (_is_node($node)) {
        if ($node->rule_name eq 'value') {
            return join('', map { _eval_value_node($_, $env) } @{ $node->children // [] });
        }
        if ($node->rule_name eq 'function_call') {
            return _eval_function_call($node, $env);
        }
        if ($node->rule_name =~ /^(?:mixin_value_list|value_list)$/) {
            return _eval_value_list_node($node, $env);
        }
        return join(' ', map { _eval_value_node($_, $env) } @{ $node->children // [] });
    }
    return '';
}

sub _eval_include_args {
    my ($args_node, $env) = @_;
    return () unless defined $args_node;
    my @vals;
    for my $child (@{ $args_node->children // [] }) {
        next unless _is_node($child);
        if ($child->rule_name =~ /^(?:include_arg|function_arg)$/) {
            my $vl = _find_child($child->children, 'value_list');
            if ($vl) {
                push @vals, _eval_value_list_node($vl, $env);
            } else {
                push @vals, _eval_value_node($child, $env);
            }
        }
        elsif ($child->rule_name eq 'value_list') {
            push @vals, _eval_value_list_node($child, $env);
        }
        # Skip COMMA tokens
    }
    return @vals;
}

sub _eval_unary {
    my ($node, $env) = @_;
    my $children = $node->children // [];
    if (@$children == 2) {
        my $tok = $children->[0];
        if (_is_token($tok) && $tok->type eq 'MINUS') {
            my $val = _eval_unary($children->[1], $env);
            my ($n, $unit) = _parse_numeric($val);
            if (defined $n) {
                my $r = -$n;
                return ($unit && $unit ne '') ? "$r$unit" : $r;
            }
            return '-' . $val;
        }
    }
    for my $c (@$children) {
        return _eval_primary($c, $env) if _is_node($c) && $c->rule_name eq 'lattice_primary';
    }
    return _eval_primary($node, $env);
}

sub _eval_multiplicative {
    my ($node, $env) = @_;
    my $children = $node->children // [];
    return '' unless @$children;

    my ($result, $i);
    for ($i = 0; $i < @$children; $i++) {
        my $c = $children->[$i];
        if (_is_node($c) && $c->rule_name eq 'lattice_unary') {
            $result = _eval_unary($c, $env);
            $i++;
            last;
        }
    }

    while ($i < @$children) {
        my $op  = $children->[$i];
        my $rhs_node = $children->[$i + 1];
        if (_is_token($op) && $rhs_node && _is_node($rhs_node) && $rhs_node->rule_name eq 'lattice_unary') {
            my $rhs = _eval_unary($rhs_node, $env);
            my ($ln, $lunit) = _parse_numeric($result);
            my ($rn)         = _parse_numeric($rhs);
            if (defined $ln && defined $rn) {
                my $res = $op->type eq 'STAR' ? $ln * $rn
                        : ($rn != 0 ? $ln / $rn : 0);
                $result = ($lunit && $lunit ne '') ? "$res$lunit" : $res;
            } else {
                $result = ($result // '') . $op->value . ($rhs // '');
            }
        }
        $i += 2;
    }
    return $result;
}

sub _eval_additive {
    my ($node, $env) = @_;
    my $children = $node->children // [];
    return '' unless @$children;

    my ($result, $i);
    for ($i = 0; $i < @$children; $i++) {
        my $c = $children->[$i];
        if (_is_node($c) && $c->rule_name eq 'lattice_multiplicative') {
            $result = _eval_multiplicative($c, $env);
            $i++;
            last;
        }
    }

    while ($i < @$children) {
        my $op  = $children->[$i];
        my $rhs_node = $children->[$i + 1];
        if (_is_token($op) && $rhs_node && _is_node($rhs_node) && $rhs_node->rule_name eq 'lattice_multiplicative') {
            my $rhs = _eval_multiplicative($rhs_node, $env);
            my ($ln, $lunit) = _parse_numeric($result);
            my ($rn)         = _parse_numeric($rhs);
            if (defined $ln && defined $rn) {
                my $res = $op->type eq 'PLUS' ? $ln + $rn : $ln - $rn;
                $result = ($lunit && $lunit ne '') ? "$res$lunit" : $res;
            } else {
                my $sep = $op->type eq 'PLUS' ? '' : ' - ';
                $result = ($result // '') . $sep . ($rhs // '');
            }
        }
        $i += 2;
    }
    return $result;
}

sub _eval_comparison {
    my ($node, $env) = @_;
    my $children = $node->children // [];
    return '' unless @$children;

    my ($lhs_node, $op_node, $rhs_node);
    for my $c (@$children) {
        if (_is_node($c)) {
            if ($c->rule_name eq 'lattice_additive') {
                if (!$lhs_node) { $lhs_node = $c }
                else            { $rhs_node = $c }
            }
            elsif ($c->rule_name eq 'comparison_op') {
                $op_node = $c;
            }
        }
    }

    return '' unless $lhs_node;
    my $lhs = _eval_additive($lhs_node, $env);
    return $lhs unless $op_node && $rhs_node;

    my $rhs = _eval_additive($rhs_node, $env);

    my $op_tok;
    for my $c (@{ $op_node->children // [] }) {
        $op_tok = $c if _is_token($c);
        last if $op_tok;
    }
    return $lhs unless $op_tok;

    my ($ln) = _parse_numeric($lhs);
    my ($rn) = _parse_numeric($rhs);

    my $t = $op_tok->type;
    if    ($t eq 'EQUALS_EQUALS')  { return (defined $ln && defined $rn) ? ($ln == $rn ? 1 : 0)  : ($lhs eq $rhs ? 1 : 0) }
    elsif ($t eq 'NOT_EQUALS')     { return (defined $ln && defined $rn) ? ($ln != $rn ? 1 : 0)  : ($lhs ne $rhs ? 1 : 0) }
    elsif ($t eq 'GREATER')        { return (defined $ln && defined $rn) ? ($ln > $rn  ? 1 : 0)  : 0 }
    elsif ($t eq 'GREATER_EQUALS') { return (defined $ln && defined $rn) ? ($ln >= $rn ? 1 : 0)  : 0 }
    elsif ($t eq 'LESS')           { return (defined $ln && defined $rn) ? ($ln < $rn  ? 1 : 0)  : 0 }
    elsif ($t eq 'LESS_EQUALS')    { return (defined $ln && defined $rn) ? ($ln <= $rn ? 1 : 0)  : 0 }
    return $lhs;
}

sub _eval_and_expr {
    my ($node, $env) = @_;
    my $result = 1;
    for my $c (@{ $node->children // [] }) {
        if (_is_node($c) && $c->rule_name eq 'lattice_comparison') {
            my $val = _eval_comparison($c, $env);
            return 0 unless $val;
            $result = $val;
        }
    }
    return $result;
}

sub _eval_or_expr {
    my ($node, $env) = @_;
    for my $c (@{ $node->children // [] }) {
        if (_is_node($c) && $c->rule_name eq 'lattice_and_expr') {
            my $val = _eval_and_expr($c, $env);
            return $val if $val;
        }
    }
    return 0;
}

sub _eval_expression {
    my ($node, $env) = @_;
    return '' unless _is_node($node);
    for my $c (@{ $node->children // [] }) {
        return _eval_or_expr($c, $env) if _is_node($c) && $c->rule_name eq 'lattice_or_expr';
    }
    return '';
}

# ============================================================================
# Selector emission
# ============================================================================

sub _emit_compound_selector {
    my ($node) = @_;
    my @parts;
    for my $c (@{ $node->children // [] }) {
        if (_is_token($c)) {
            push @parts, $c->value;
        }
        elsif (_is_node($c)) {
            if ($c->rule_name eq 'simple_selector') {
                push @parts, map { _is_token($_) ? $_->value : '' }
                             @{ $c->children // [] };
            }
            elsif ($c->rule_name eq 'subclass_selector') {
                for my $sc (@{ $c->children // [] }) {
                    if (_is_node($sc)) {
                        if ($sc->rule_name eq 'class_selector') {
                            my $ident = _find_token($sc->children, 'IDENT');
                            push @parts, '.' . $ident->value if $ident;
                        }
                        elsif ($sc->rule_name eq 'id_selector') {
                            my $h = _find_token($sc->children, 'HASH');
                            push @parts, $h->value if $h;
                        }
                        elsif ($sc->rule_name eq 'pseudo_class') {
                            my $func  = _find_token($sc->children, 'FUNCTION');
                            my $ident = _find_token($sc->children, 'IDENT');
                            if ($func) {
                                push @parts, ':' . $func->value;
                                my $args = _find_child($sc->children, 'pseudo_class_args');
                                push @parts, _collect_text($args) if $args;
                                push @parts, ')';
                            }
                            elsif ($ident) {
                                push @parts, ':' . $ident->value;
                            }
                        }
                        elsif ($sc->rule_name eq 'pseudo_element') {
                            my $ident = _find_token($sc->children, 'IDENT');
                            push @parts, '::' . $ident->value if $ident;
                        }
                        elsif ($sc->rule_name eq 'attribute_selector') {
                            push @parts, _collect_text($sc);
                        }
                        elsif ($sc->rule_name eq 'placeholder_selector') {
                            my $ph = _find_token($sc->children, 'PLACEHOLDER');
                            push @parts, $ph->value if $ph;
                        }
                        else {
                            push @parts, _collect_text($sc);
                        }
                    }
                    elsif (_is_token($sc)) {
                        push @parts, $sc->value;
                    }
                }
            }
            elsif ($c->rule_name eq 'placeholder_selector') {
                my $ph = _find_token($c->children, 'PLACEHOLDER');
                push @parts, $ph->value if $ph;
            }
            else {
                push @parts, _collect_text($c);
            }
        }
    }
    return join('', @parts);
}

sub _emit_complex_selector {
    my ($node) = @_;
    my @parts;
    for my $c (@{ $node->children // [] }) {
        if (_is_node($c)) {
            if ($c->rule_name eq 'compound_selector') {
                push @parts, _emit_compound_selector($c);
            }
            elsif ($c->rule_name eq 'combinator') {
                my $tok;
                for my $t (@{ $c->children // [] }) {
                    $tok = $t if _is_token($t);
                    last if $tok;
                }
                push @parts, ' ' . $tok->value if $tok;
            }
        }
        elsif (_is_token($c)) {
            push @parts, $c->value;
        }
    }
    my $sel = join(' ', @parts);
    $sel =~ s/\s+/ /g;
    $sel =~ s/^\s+|\s+$//g;
    return $sel;
}

sub _emit_selector_list {
    my ($node) = @_;
    my @parts;
    for my $c (@{ $node->children // [] }) {
        push @parts, _emit_complex_selector($c)
            if _is_node($c) && $c->rule_name eq 'complex_selector';
    }
    return join(', ', @parts);
}

sub _resolve_selector {
    my ($parent, $child) = @_;
    return $child unless $parent;
    if ($child =~ /&/) {
        $child =~ s/&/$parent/g;
        return $child;
    }
    return "$parent $child";
}

# ============================================================================
# CSS text emission
# ============================================================================

sub _emit_value_list {
    my ($node, $env) = @_;
    return '' unless _is_node($node);
    return join(' ', map { _eval_value_node($_, $env) } @{ $node->children // [] });
}

sub _emit_declaration {
    my ($node, $env, $indent) = @_;
    $indent //= '  ';
    my $children  = $node->children // [];
    my $prop_node = _find_child($children, 'property') or return '';
    my $vl_node   = _find_child($children, 'value_list');
    my $pri_node  = _find_child($children, 'priority');

    my $prop      = _collect_text($prop_node);
    my $val       = $vl_node ? _emit_value_list($vl_node, $env) : '';
    my $important = $pri_node ? ' !important' : '';

    return "$indent$prop: $val$important;\n";
}

sub _extract_mixin_params {
    my ($params_node, $env) = @_;
    return ([], {}) unless defined $params_node;
    my (@params, %defaults);
    for my $child (@{ $params_node->children // [] }) {
        next unless _is_node($child) && $child->rule_name eq 'mixin_param';
        my $var_tok = _find_token($child->children, 'VARIABLE') or next;
        push @params, $var_tok->value;
        my $default_node = _find_child($child->children, 'mixin_value_list')
                        // _find_child($child->children, 'value_list');
        if ($default_node) {
            $defaults{ $var_tok->value } = _eval_value_list_node($default_node, $env);
        }
    }
    return (\@params, \%defaults);
}

# _compile_block — expand a block node into declarations and nested rule strings.
sub _compile_block {
    my ($block_node, $env, $parent_sel, $declarations, $nested_rules) = @_;
    # block = LBRACE block_contents RBRACE
    my $contents = _find_child($block_node->children, 'block_contents');
    return unless $contents;

    for my $item (@{ $contents->children // [] }) {
        _compile_block_item($item, $env, $parent_sel, $declarations, $nested_rules)
            if _is_node($item) && $item->rule_name eq 'block_item';
    }
}

sub _compile_block_item {
    my ($item, $env, $parent_sel, $declarations, $nested_rules) = @_;
    for my $inner (@{ $item->children // [] }) {
        next unless _is_node($inner);
        if ($inner->rule_name eq 'lattice_block_item') {
            _compile_lattice_block_item($inner, $env, $parent_sel, $declarations, $nested_rules);
        }
        elsif ($inner->rule_name eq 'declaration_or_nested') {
            _compile_declaration_or_nested($inner, $env, $parent_sel, $declarations, $nested_rules);
        }
        elsif ($inner->rule_name eq 'at_rule') {
            push @$nested_rules, _emit_at_rule($inner, $parent_sel, $env);
        }
        elsif ($inner->rule_name eq 'declaration') {
            push @$declarations, _emit_declaration($inner, $env, '  ');
        }
        elsif ($inner->rule_name eq 'qualified_rule') {
            _compile_nested_rule($inner, $env, $parent_sel, $nested_rules);
        }
    }
}

sub _compile_lattice_block_item {
    my ($node, $env, $parent_sel, $declarations, $nested_rules) = @_;
    for my $child (@{ $node->children // [] }) {
        next unless _is_node($child);
        if ($child->rule_name eq 'variable_declaration') {
            _exec_variable_decl($child, $env);
        }
        elsif ($child->rule_name eq 'include_directive') {
            _compile_include($child, $env, $parent_sel, $declarations, $nested_rules);
        }
        elsif ($child->rule_name eq 'lattice_control') {
            _compile_control($child, $env, $parent_sel, $declarations, $nested_rules);
        }
    }
}

sub _compile_declaration_or_nested {
    my ($node, $env, $parent_sel, $declarations, $nested_rules) = @_;
    for my $child (@{ $node->children // [] }) {
        next unless _is_node($child);
        if ($child->rule_name eq 'declaration') {
            push @$declarations, _emit_declaration($child, $env, '  ');
        }
        elsif ($child->rule_name eq 'qualified_rule') {
            _compile_nested_rule($child, $env, $parent_sel, $nested_rules);
        }
    }
}

sub _compile_nested_rule {
    my ($node, $env, $parent_sel, $output) = @_;
    my $children = $node->children // [];
    my $sel_node = _find_child($children, 'selector_list') or return;
    my $blk_node = _find_child($children, 'block')         or return;

    my $raw_sel  = _emit_selector_list($sel_node);
    my $full_sel = _resolve_selector($parent_sel, $raw_sel);

    my $child_scope = _child_env($env);
    my (@decls, @nested);
    _compile_block($blk_node, $child_scope, $full_sel, \@decls, \@nested);

    if (@decls) {
        push @$output, "$full_sel {\n" . join('', @decls) . "}\n";
    }
    push @$output, @nested;
}

sub _compile_include {
    my ($node, $env, $parent_sel, $declarations, $nested_rules) = @_;
    my $children = $node->children // [];

    my $func_tok  = _find_token($children, 'FUNCTION');
    my $ident_tok = _find_token($children, 'IDENT');
    my $mixin_name;
    if ($func_tok) {
        ($mixin_name = $func_tok->value) =~ s/\(+$//;
    }
    elsif ($ident_tok) {
        $mixin_name = $ident_tok->value;
    }
    return unless $mixin_name;

    my $mixin_def = _lookup_mixin($env, $mixin_name) or return;

    my $args_node = _find_child($children, 'include_args');
    my @arg_vals  = _eval_include_args($args_node, $env);

    my $call_env = _child_env($env);
    my @params   = @{ $mixin_def->{params} // [] };
    for my $i (0 .. $#params) {
        my $param_name = $params[$i];
        (my $param_key = $param_name) =~ s/^\$//;
        my $val = $arg_vals[$i];
        unless (defined $val) {
            $val = $mixin_def->{defaults}{$param_name};
        }
        _set_var($call_env, $param_key, $val) if defined $val;
    }

    _compile_block($mixin_def->{body}, $call_env, $parent_sel, $declarations, $nested_rules);
}

sub _compile_control {
    my ($node, $env, $parent_sel, $declarations, $nested_rules) = @_;
    for my $child (@{ $node->children // [] }) {
        next unless _is_node($child);
        if    ($child->rule_name eq 'if_directive')    { _compile_if   ($child, $env, $parent_sel, $declarations, $nested_rules) }
        elsif ($child->rule_name eq 'for_directive')   { _compile_for  ($child, $env, $parent_sel, $declarations, $nested_rules) }
        elsif ($child->rule_name eq 'each_directive')  { _compile_each ($child, $env, $parent_sel, $declarations, $nested_rules) }
        elsif ($child->rule_name eq 'while_directive') { _compile_while($child, $env, $parent_sel, $declarations, $nested_rules) }
    }
}

sub _compile_if {
    my ($node, $env, $parent_sel, $declarations, $nested_rules) = @_;
    my $children = $node->children // [];

    # Rebuild branches from children
    my @branches;
    my $cur_cond;
    my $expect_cond = 1;

    for my $c (@$children) {
        if (_is_token($c)) {
            if ($c->value eq '@else') { $expect_cond = 0 }
            elsif ($c->value eq '@if') { $expect_cond = 1 }
            elsif ($c->type eq 'IDENT' && $c->value eq 'if') { $expect_cond = 1 }
        }
        elsif (_is_node($c)) {
            if ($c->rule_name eq 'lattice_expression') {
                $cur_cond = $c;
            }
            elsif ($c->rule_name eq 'block') {
                push @branches, { cond => $cur_cond, block => $c };
                $cur_cond = undef;
                $expect_cond = 1;
            }
        }
    }

    for my $branch (@branches) {
        my $take = 1;
        if ($branch->{cond}) {
            my $val = _eval_expression($branch->{cond}, $env);
            $take = ($val && $val ne 'false' && $val ne 'null') ? 1 : 0;
        }
        if ($take) {
            my $branch_env = _child_env($env);
            _compile_block($branch->{block}, $branch_env, $parent_sel, $declarations, $nested_rules);
            return;
        }
    }
}

sub _compile_for {
    my ($node, $env, $parent_sel, $declarations, $nested_rules) = @_;
    my $children = $node->children // [];

    my $var_tok  = _find_token($children, 'VARIABLE') or return;
    (my $var_name = $var_tok->value) =~ s/^\$//;

    my @exprs;
    my $blk_node;
    my $exclusive = 0;

    for my $c (@$children) {
        if (_is_node($c)) {
            if ($c->rule_name eq 'lattice_expression') { push @exprs, $c }
            elsif ($c->rule_name eq 'block')           { $blk_node = $c }
        }
        elsif (_is_token($c) && $c->value eq 'to') {
            $exclusive = 1;
        }
    }

    return unless @exprs >= 2 && $blk_node;

    my $start_val  = _eval_expression($exprs[0], $env);
    my $finish_val = _eval_expression($exprs[1], $env);
    my ($start_n)  = _parse_numeric($start_val);
    my ($finish_n) = _parse_numeric($finish_val);
    return unless defined $start_n && defined $finish_n;

    my $step  = ($start_n <= $finish_n) ? 1 : -1;
    my $i_val = int($start_n);
    my $end   = int($finish_n);
    my $count = 0;
    my $limit = abs($end - $i_val) + 2;
    $limit = 1000 if $limit > 1000;

    while ($count < $limit) {
        if ($exclusive) {
            last if $step > 0 && $i_val >= $end;
            last if $step < 0 && $i_val <= $end;
        } else {
            last if $step > 0 && $i_val > $end;
            last if $step < 0 && $i_val < $end;
        }

        my $iter_env = _child_env($env);
        _set_var($iter_env, $var_name, $i_val);
        _compile_block($blk_node, $iter_env, $parent_sel, $declarations, $nested_rules);

        $i_val += $step;
        $count++;
    }
}

sub _compile_each {
    my ($node, $env, $parent_sel, $declarations, $nested_rules) = @_;
    my $children = $node->children // [];

    my @var_toks = _find_tokens($children, 'VARIABLE');
    return unless @var_toks;
    (my $var_name = $var_toks[0]->value) =~ s/^\$//;

    my $each_list = _find_child($children, 'each_list') or return;
    my $blk_node  = _find_child($children, 'block')     or return;

    my @values;
    for my $c (@{ $each_list->children // [] }) {
        if (_is_node($c) && $c->rule_name eq 'value') {
            push @values, _eval_value_node($c, $env);
        }
        elsif (_is_token($c) && $c->type ne 'COMMA') {
            push @values, $c->value;
        }
    }

    for my $val (@values) {
        my $iter_env = _child_env($env);
        _set_var($iter_env, $var_name, $val);
        _compile_block($blk_node, $iter_env, $parent_sel, $declarations, $nested_rules);
    }
}

sub _compile_while {
    my ($node, $env, $parent_sel, $declarations, $nested_rules) = @_;
    my $children  = $node->children // [];
    my $cond_node = _find_child($children, 'lattice_expression') or return;
    my $blk_node  = _find_child($children, 'block')              or return;

    my $count = 0;
    while ($count < 1000) {
        my $val = _eval_expression($cond_node, $env);
        last unless $val && $val ne 'false';
        _compile_block($blk_node, $env, $parent_sel, $declarations, $nested_rules);
        $count++;
    }
}

sub _emit_at_rule {
    my ($node, $parent_sel, $env) = @_;
    my $children  = $node->children // [];
    my $kw        = _find_token($children, 'AT_KEYWORD') or return '';

    my $prelude_node = _find_child($children, 'at_prelude');
    my $prelude      = $prelude_node ? _collect_text($prelude_node) : '';
    $prelude =~ s/^\s+|\s+$//g;

    my $blk = _find_child($children, 'block');
    if ($blk) {
        if ($parent_sel) {
            my (@inner_decls, @inner_nested);
            _compile_block($blk, _child_env($env), $parent_sel, \@inner_decls, \@inner_nested);
            my @body_parts;
            if (@inner_decls) {
                push @body_parts, "  $parent_sel {\n";
                push @body_parts, map { "  $_" } @inner_decls;
                push @body_parts, "  }\n";
            }
            push @body_parts, @inner_nested;
            if (@body_parts) {
                return $kw->value . ($prelude ? " $prelude" : '') . " {\n" . join('', @body_parts) . "}\n";
            }
            return '';
        }
        else {
            my (@inner_decls, @inner_nested);
            _compile_block($blk, _child_env($env), '', \@inner_decls, \@inner_nested);
            my $body = join('', @inner_decls, @inner_nested);
            if ($body) {
                return $kw->value . ($prelude ? " $prelude" : '') . " {\n$body}\n";
            }
            return $kw->value . ($prelude ? " $prelude" : '') . " {}\n";
        }
    }
    else {
        return $kw->value . ($prelude ? " $prelude" : '') . ";\n";
    }
}

# ============================================================================
# Pass 1: Symbol Collection
# ============================================================================

sub _collect_top_level_symbols {
    my ($stylesheet, $env) = @_;
    my @remaining;

    for my $rule (@{ $stylesheet->children // [] }) {
        next unless _is_node($rule);
        my $collected = 0;

        if ($rule->rule_name eq 'rule') {
            my $inner = $rule->children->[0];
            if (_is_node($inner) && $inner->rule_name eq 'lattice_rule') {
                my $lat = $inner->children->[0];
                if (_is_node($lat)) {
                    if ($lat->rule_name eq 'variable_declaration') {
                        _exec_variable_decl($lat, $env);
                        $collected = 1;
                    }
                    elsif ($lat->rule_name eq 'mixin_definition') {
                        _collect_mixin_def($lat, $env);
                        $collected = 1;
                    }
                    elsif ($lat->rule_name eq 'function_definition') {
                        _collect_function_def($lat, $env);
                        $collected = 1;
                    }
                    elsif ($lat->rule_name eq 'use_directive') {
                        $collected = 1;  # skip @use for now
                    }
                }
            }
        }

        push @remaining, $rule unless $collected;
    }

    return @remaining;
}

sub _collect_mixin_def {
    my ($node, $env) = @_;
    my $children  = $node->children // [];
    my $func_tok  = _find_token($children, 'FUNCTION');
    my $ident_tok = _find_token($children, 'IDENT');
    my $name;
    if ($func_tok) { ($name = $func_tok->value) =~ s/\(+$// }
    elsif ($ident_tok) { $name = $ident_tok->value }
    return unless $name;

    my $params_node = _find_child($children, 'mixin_params');
    my $body_node   = _find_child($children, 'block') or return;

    my ($params, $defaults) = _extract_mixin_params($params_node, $env);
    _root_env($env)->{mixins}{$name} = { params => $params, defaults => $defaults, body => $body_node };
}

sub _collect_function_def {
    my ($node, $env) = @_;
    my $children  = $node->children // [];
    my $func_tok  = _find_token($children, 'FUNCTION');
    my $ident_tok = _find_token($children, 'IDENT');
    my $name;
    if ($func_tok) { ($name = $func_tok->value) =~ s/\(+$// }
    elsif ($ident_tok) { $name = $ident_tok->value }
    return unless $name;

    my $params_node = _find_child($children, 'mixin_params');
    my $body_node   = _find_child($children, 'function_body') or return;

    my ($params, $defaults) = _extract_mixin_params($params_node, $env);
    _root_env($env)->{functions}{$name} = { params => $params, defaults => $defaults, body => $body_node };
}

# ============================================================================
# Pass 2: Top-level expansion
# ============================================================================

sub _compile_top_level_rule {
    my ($rule_node, $env) = @_;
    return '' unless _is_node($rule_node);

    my $inner = $rule_node->rule_name eq 'rule'
                ? $rule_node->children->[0]
                : $rule_node;
    return '' unless _is_node($inner);

    if ($inner->rule_name eq 'qualified_rule') {
        my $children = $inner->children // [];
        my $sel_node = _find_child($children, 'selector_list') or return '';
        my $blk_node = _find_child($children, 'block')         or return '';

        my $selector  = _emit_selector_list($sel_node);
        my $rule_env  = _child_env($env);
        my (@decls, @nested);
        _compile_block($blk_node, $rule_env, $selector, \@decls, \@nested);

        my @result;
        push @result, "$selector {\n" . join('', @decls) . "}\n" if @decls;
        push @result, @nested;
        return join("\n", @result);
    }

    if ($inner->rule_name eq 'at_rule') {
        return _emit_at_rule($inner, '', $env);
    }

    if ($inner->rule_name eq 'lattice_rule') {
        my $lat = $inner->children->[0];
        if (_is_node($lat) && $lat->rule_name eq 'lattice_control') {
            my (@decls, @nested);
            _compile_control($lat, $env, '', \@decls, \@nested);
            return join("\n", @nested);
        }
    }

    return '';
}

# ============================================================================
# Public API
# ============================================================================

=head1 NAME

CodingAdventures::LatticeAstToCss - Lattice AST to CSS compiler

=head1 SYNOPSIS

    use CodingAdventures::LatticeParser;
    use CodingAdventures::LatticeAstToCss;

    my $ast = CodingAdventures::LatticeParser->parse('$c: red; h1 { color: $c; }');
    my $css = CodingAdventures::LatticeAstToCss->compile($ast);
    # $css eq "h1 {\n  color: red;\n}\n"

=head1 DESCRIPTION

Walks a Lattice AST and emits CSS text.  Handles variable expansion,
nested rule flattening, mixin expansion, @if/@for/@each control flow,
and @function evaluation.

=head1 METHODS

=head2 compile($ast) -> $css_string

Compile a Lattice AST (root C<stylesheet> ASTNode) to CSS text.

=cut

sub compile {
    my ($class, $ast) = @_;
    die "LatticeAstToCss->compile: expected an ASTNode\n"
        unless _is_node($ast);

    my $env = _new_env();

    # Pass 1: collect definitions
    my @remaining = _collect_top_level_symbols($ast, $env);

    # Pass 2: compile
    my @parts;
    for my $rule_node (@remaining) {
        my $css = _compile_top_level_rule($rule_node, $env);
        push @parts, $css if $css;
    }

    my $result = join('', @parts);
    $result .= "\n" if $result && $result !~ /\n$/;
    return $result;
}

1;
