package CodingAdventures::MosaicVm;

# ============================================================================
# CodingAdventures::MosaicVm — Generic tree-walking driver for Mosaic backends
# ============================================================================
#
# The MosaicVM is the fourth stage of the Mosaic compiler pipeline:
#
#   Source → Lexer → Parser → Analyzer → MosaicIR → **VM** → Backend → Code
#
# The VM's responsibilities:
#   1. Traverse the MosaicIR component tree depth-first.
#   2. Normalize every MosaicValue into a ResolvedValue:
#        - color_hex  → parsed RGBA integers
#        - dimension  → { value => N, unit => "dp" }
#        - ident      → folded into string
#        - slot_ref   → enriched with slot type + is_loop_var flag
#   3. Track slot context (component slots + active each-loop scopes).
#   4. Call renderer methods in strict open-before-close order.
#
# Renderer duck-typing
# --------------------
#
# The renderer is any object/hashref that supports these methods:
#   begin_component($name, $slots_arrayref)
#   end_component()
#   begin_node($tag, $is_primitive, $resolved_props_arrayref, $ctx)
#   end_node($tag)
#   render_slot_child($slot_name, $slot_type, $ctx)
#   begin_when($slot_name, $ctx)
#   end_when()
#   begin_each($slot_name, $item_name, $element_type, $ctx)
#   end_each()
#   emit()   → returns whatever the backend wants to return
#
# We check with can() if the renderer supports each method, following
# Perl duck-typing conventions.
#
# Slot context hashref
# --------------------
#
#   {
#     component_slots => { slot_name => slot_hashref, ... },
#     loop_scopes     => [ { item_name => "item", element_type => type }, ... ],
#   }
#
# Color parsing rules
# -------------------
#
#   #rgb      → r=rr, g=gg, b=bb, a=255
#   #rrggbb   → r, g, b, a=255
#   #rrggbbaa → r, g, b, a
#
# Public API
# ----------
#
#   my ($result, $error) = CodingAdventures::MosaicVm->run($component, $renderer);

use strict;
use warnings;

use CodingAdventures::MosaicAnalyzer;

our $VERSION = '0.01';

# ============================================================================
# Public API
# ============================================================================

# run($component, $renderer)
#
# Traverse the MosaicIR component, calling renderer methods depth-first.
# Returns ($emit_result, undef) on success, (undef, $error) on failure.
#
# $component — hashref from CodingAdventures::MosaicAnalyzer->analyze()
# $renderer  — any object/hashref supporting the renderer duck-type contract

sub run {
    my ($class, $component, $renderer) = @_;

    # Validate renderer has the required methods
    for my $method (qw(begin_component end_component begin_node end_node
                       render_slot_child begin_when end_when
                       begin_each end_each emit)) {
        unless ($renderer->can($method)) {
            return (undef, "MosaicVm: renderer is missing method '$method'");
        }
    }

    my $ctx = {
        component_slots => { map { $_->{name} => $_ } @{ $component->{slots} } },
        loop_scopes     => [],
    };

    my $result = eval {
        $renderer->begin_component($component->{name}, $component->{slots});
        _walk_node($component->{tree}, $ctx, $renderer);
        $renderer->end_component();
        $renderer->emit();
    };
    return (undef, $@) if $@;
    return ($result, undef);
}

# ============================================================================
# Tree traversal
# ============================================================================

sub _walk_node {
    my ($node, $ctx, $r) = @_;

    # Resolve all properties before calling begin_node
    my @resolved = map { { name => $_->{name}, value => _resolve_value($_->{value}, $ctx) } }
                   @{ $node->{properties} };

    $r->begin_node($node->{tag}, $node->{is_primitive}, \@resolved, $ctx);

    for my $child (@{ $node->{children} }) {
        _walk_child($child, $ctx, $r);
    }

    $r->end_node($node->{tag});
}

sub _walk_child {
    my ($child, $ctx, $r) = @_;
    my $kind = $child->{kind};

    if ($kind eq 'node') {
        _walk_node($child->{node}, $ctx, $r);
        return;
    }

    if ($kind eq 'slot_ref') {
        my $slot = _resolve_slot($child->{slot_name}, $ctx);
        $r->render_slot_child($child->{slot_name}, $slot->{type}, $ctx);
        return;
    }

    if ($kind eq 'when') {
        $r->begin_when($child->{slot_name}, $ctx);
        _walk_child($_, $ctx, $r) for @{ $child->{children} };
        $r->end_when();
        return;
    }

    if ($kind eq 'each') {
        my $list_slot = $ctx->{component_slots}{$child->{slot_name}};
        unless ($list_slot) {
            die "MosaicVmError: Unknown list slot: \@$child->{slot_name}";
        }
        unless ($list_slot->{type}{kind} eq 'list') {
            die "MosaicVmError: each block references \@$child->{slot_name} but it is not a list type";
        }
        my $element_type = $list_slot->{type}{element_type};

        $r->begin_each($child->{slot_name}, $child->{item_name}, $element_type, $ctx);

        # Push loop scope so @item references inside the block resolve correctly
        my $inner_ctx = {
            component_slots => $ctx->{component_slots},
            loop_scopes     => [
                @{ $ctx->{loop_scopes} },
                { item_name => $child->{item_name}, element_type => $element_type },
            ],
        };

        _walk_child($_, $inner_ctx, $r) for @{ $child->{children} };
        $r->end_each();
        return;
    }

    die "MosaicVmError: Unknown child kind '$kind'";
}

# ============================================================================
# Value resolution
# ============================================================================

sub _resolve_value {
    my ($v, $ctx) = @_;
    my $kind = $v->{kind};

    return { kind => 'string',    value => $v->{value}              } if $kind eq 'string';
    return { kind => 'number',    value => $v->{value}              } if $kind eq 'number';
    return { kind => 'bool',      value => $v->{value}              } if $kind eq 'bool';
    return { kind => 'string',    value => $v->{value}              } if $kind eq 'ident';
    return { kind => 'enum', namespace => $v->{namespace}, member => $v->{member} } if $kind eq 'enum';
    return _parse_dimension($v->{value}, $v->{unit})                  if $kind eq 'dimension';
    return _parse_color($v->{value})                                   if $kind eq 'color_hex';
    return _resolve_slot_ref($v->{slot_name}, $ctx)                   if $kind eq 'slot_ref';

    die "MosaicVmError: Unknown value kind '$kind'";
}

sub _parse_dimension {
    my ($value, $unit) = @_;
    # Pass through — unit is already validated by the analyzer.
    return { kind => 'dimension', value => $value, unit => $unit };
}

# Parse a hex color string into RGBA integer components.
#
# Three-digit (#rgb)    → double each digit, alpha = 255
# Six-digit  (#rrggbb)  → alpha = 255
# Eight-digit (#rrggbbaa) → all four channels
sub _parse_color {
    my ($hex) = @_;
    my $h = substr($hex, 1);  # strip leading '#'
    my ($r, $g, $b, $a);
    $a = 255;

    if (length($h) == 3) {
        $r = hex(substr($h,0,1) x 2);
        $g = hex(substr($h,1,1) x 2);
        $b = hex(substr($h,2,1) x 2);
    } elsif (length($h) == 6) {
        $r = hex(substr($h,0,2));
        $g = hex(substr($h,2,2));
        $b = hex(substr($h,4,2));
    } elsif (length($h) == 8) {
        $r = hex(substr($h,0,2));
        $g = hex(substr($h,2,2));
        $b = hex(substr($h,4,2));
        $a = hex(substr($h,6,2));
    } else {
        die "MosaicVmError: Invalid color hex: $hex";
    }

    return { kind => 'color', r => $r, g => $g, b => $b, a => $a };
}

sub _resolve_slot_ref {
    my ($slot_name, $ctx) = @_;

    # Check loop scopes innermost-first
    for my $scope (reverse @{ $ctx->{loop_scopes} }) {
        if ($scope->{item_name} eq $slot_name) {
            return {
                kind        => 'slot_ref',
                slot_name   => $slot_name,
                slot_type   => $scope->{element_type},
                is_loop_var => 1,
            };
        }
    }

    # Fall back to component slots
    my $slot = $ctx->{component_slots}{$slot_name};
    unless ($slot) {
        die "MosaicVmError: Unresolved slot reference: \@$slot_name";
    }
    return {
        kind        => 'slot_ref',
        slot_name   => $slot_name,
        slot_type   => $slot->{type},
        is_loop_var => 0,
    };
}

sub _resolve_slot {
    my ($slot_name, $ctx) = @_;
    my $slot = $ctx->{component_slots}{$slot_name};
    unless ($slot) {
        die "MosaicVmError: Unknown slot: \@$slot_name";
    }
    return $slot;
}

1;

__END__

=head1 NAME

CodingAdventures::MosaicVm - Generic tree-walking driver for Mosaic compiler backends

=head1 SYNOPSIS

    use CodingAdventures::MosaicVm;
    use CodingAdventures::MosaicAnalyzer;

    my ($component, $err) = CodingAdventures::MosaicAnalyzer->analyze($source);
    die $err if $err;

    my ($result, $vm_err) = CodingAdventures::MosaicVm->run($component, $renderer);
    die $vm_err if $vm_err;

=head1 DESCRIPTION

Port of the TypeScript MosaicVM to Perl. Traverses a MosaicIR component tree
depth-first, normalizes values (hex colors to RGBA, dimensions to value+unit),
and drives any renderer that implements the duck-typed renderer interface.

=head1 METHODS

=head2 run($component, $renderer)

Run the VM. C<$component> is a hashref from C<MosaicAnalyzer::analyze>.
C<$renderer> is any object with the required renderer methods.
Returns C<($result, undef)> on success or C<(undef, $error)> on failure.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
