package CodingAdventures::BinaryTree::Node;

use strict;
use warnings;

sub new {
    my ($class, $value, $left, $right) = @_;
    die "node value must be defined\n" if !defined $value;
    for my $pair ([$left, 'left'], [$right, 'right']) {
        my ($child, $name) = @$pair;
        die "$name must be a CodingAdventures::BinaryTree::Node or undef\n"
            if defined($child) && !$child->isa(__PACKAGE__);
    }
    return bless {value => $value, left => $left, right => $right}, $class;
}

sub value { return $_[0]->{value}; }
sub left  { return $_[0]->{left}; }
sub right { return $_[0]->{right}; }

package CodingAdventures::BinaryTree;

use strict;
use warnings;
use Scalar::Util qw(refaddr);
use overload '""' => 'to_string', fallback => 1;

our $VERSION = '0.1.0';

sub _is_node {
    my ($value) = @_;
    return defined($value)
        && ref($value)
        && eval { $value->isa('CodingAdventures::BinaryTree::Node') };
}

sub _values_equal {
    my ($left, $right) = @_;
    return 0 if !defined($left) || !defined($right);
    return refaddr($left) == refaddr($right) if ref($left) && ref($right);
    return 0 if ref($left) || ref($right);
    return $left eq $right;
}

sub new {
    my ($class, $root) = @_;
    die "root must be a CodingAdventures::BinaryTree::Node or undef\n"
        if defined($root) && !_is_node($root);
    return bless {root => $root}, $class;
}

sub with_root {
    my ($class, $root) = @_;
    return $class->new($root);
}

sub singleton {
    my ($class, $value) = @_;
    return $class->new(CodingAdventures::BinaryTree::Node->new($value));
}

sub from_level_order {
    my ($class, $values) = @_;
    die "values must be an array reference\n" if ref($values) ne 'ARRAY';
    return $class->new(_build_from_level_order($values, 0));
}

sub _build_from_level_order {
    my ($values, $index) = @_;
    return undef if $index >= @$values || !defined $values->[$index];
    return CodingAdventures::BinaryTree::Node->new(
        $values->[$index],
        _build_from_level_order($values, 2 * $index + 1),
        _build_from_level_order($values, 2 * $index + 2),
    );
}

sub root { return $_[0]->{root}; }

sub find {
    my ($self, $value) = @_;
    return _find($self->{root}, $value);
}

sub _find {
    my ($root, $value) = @_;
    return undef if !defined $root;
    return $root if _values_equal($root->{value}, $value);
    return _find($root->{left}, $value) || _find($root->{right}, $value);
}

sub left_child {
    my ($self, $value) = @_;
    my $node = $self->find($value);
    return defined($node) ? $node->{left} : undef;
}

sub right_child {
    my ($self, $value) = @_;
    my $node = $self->find($value);
    return defined($node) ? $node->{right} : undef;
}

sub height {
    my ($self) = @_;
    return _height($self->{root});
}

sub _height {
    my ($root) = @_;
    return -1 if !defined $root;
    my $left = _height($root->{left});
    my $right = _height($root->{right});
    return 1 + ($left > $right ? $left : $right);
}

sub size {
    my ($self) = @_;
    return _size($self->{root});
}

sub _size {
    my ($root) = @_;
    return 0 if !defined $root;
    return 1 + _size($root->{left}) + _size($root->{right});
}

sub is_full {
    my ($self) = @_;
    return _is_full($self->{root});
}

sub _is_full {
    my ($root) = @_;
    return 1 if !defined $root;
    return 1 if !defined($root->{left}) && !defined($root->{right});
    return 0 if !defined($root->{left}) || !defined($root->{right});
    return _is_full($root->{left}) && _is_full($root->{right});
}

sub is_complete {
    my ($self) = @_;
    my @queue = ($self->{root});
    my $seen_empty = 0;

    while (@queue) {
        my $node = shift @queue;
        if (!defined $node) {
            $seen_empty = 1;
            next;
        }
        return 0 if $seen_empty;
        push @queue, $node->{left}, $node->{right};
    }
    return 1;
}

sub is_perfect {
    my ($self) = @_;
    my $height = $self->height;
    return $self->size == 0 if $height < 0;
    return $self->size == (2 ** ($height + 1)) - 1;
}

sub inorder {
    my ($self) = @_;
    my @out;
    _append_inorder($self->{root}, \@out);
    return \@out;
}

sub _append_inorder {
    my ($root, $out) = @_;
    return if !defined $root;
    _append_inorder($root->{left}, $out);
    push @$out, $root->{value};
    _append_inorder($root->{right}, $out);
}

sub preorder {
    my ($self) = @_;
    my @out;
    _append_preorder($self->{root}, \@out);
    return \@out;
}

sub _append_preorder {
    my ($root, $out) = @_;
    return if !defined $root;
    push @$out, $root->{value};
    _append_preorder($root->{left}, $out);
    _append_preorder($root->{right}, $out);
}

sub postorder {
    my ($self) = @_;
    my @out;
    _append_postorder($self->{root}, \@out);
    return \@out;
}

sub _append_postorder {
    my ($root, $out) = @_;
    return if !defined $root;
    _append_postorder($root->{left}, $out);
    _append_postorder($root->{right}, $out);
    push @$out, $root->{value};
}

sub level_order {
    my ($self) = @_;
    return [] if !defined $self->{root};

    my @out;
    my @queue = ($self->{root});
    while (@queue) {
        my $node = shift @queue;
        push @out, $node->{value};
        push @queue, $node->{left} if defined $node->{left};
        push @queue, $node->{right} if defined $node->{right};
    }
    return \@out;
}

sub to_array {
    my ($self) = @_;
    my $height = $self->height;
    return [] if $height < 0;

    my $length = (2 ** ($height + 1)) - 1;
    my @out = (undef) x $length;
    _fill_array($self->{root}, 0, \@out);
    return \@out;
}

sub _fill_array {
    my ($root, $index, $out) = @_;
    return if !defined($root) || $index >= @$out;
    $out->[$index] = $root->{value};
    _fill_array($root->{left}, 2 * $index + 1, $out);
    _fill_array($root->{right}, 2 * $index + 2, $out);
}

sub to_ascii {
    my ($self) = @_;
    return '' if !defined $self->{root};
    my @lines;
    _render_ascii($self->{root}, '', 1, \@lines);
    return join("\n", @lines);
}

sub _render_ascii {
    my ($node, $prefix, $is_tail, $lines) = @_;
    push @$lines, $prefix . ($is_tail ? '`-- ' : '|-- ') . $node->{value};

    my @children = grep { defined } ($node->{left}, $node->{right});
    my $next_prefix = $prefix . ($is_tail ? '    ' : '|   ');
    for my $index (0 .. $#children) {
        _render_ascii($children[$index], $next_prefix, $index == $#children, $lines);
    }
}

sub to_string {
    my ($self) = @_;
    my $root = defined($self->{root}) ? $self->{root}{value} : 'undef';
    return sprintf('BinaryTree(root=%s, size=%d)', $root, $self->size);
}

1;
