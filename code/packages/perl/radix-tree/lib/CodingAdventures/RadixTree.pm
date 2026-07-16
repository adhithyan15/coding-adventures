package CodingAdventures::RadixTree;

use strict;
use warnings;
use utf8;
use overload '""' => 'as_string', fallback => 1;

our $VERSION = '0.1.0';

sub _new_node {
    my ($terminal, $value) = @_;
    return {
        children => {},
        terminal => $terminal ? 1 : 0,
        value    => $value,
    };
}

sub _assert_string {
    my ($value, $name) = @_;
    die "$name must be a string\n" if !defined($value) || ref($value);
}

sub _characters {
    my ($value) = @_;
    return [split //u, $value];
}

sub _slice {
    my ($characters, $start) = @_;
    return [] if $start > $#$characters;
    return [@$characters[$start .. $#$characters]];
}

sub _concatenate {
    my ($left, $right) = @_;
    return [@$left, @$right];
}

sub _common_prefix_length {
    my ($left, $right) = @_;
    my $limit = @$left < @$right ? scalar @$left : scalar @$right;
    my $index = 0;
    $index++ while $index < $limit && $left->[$index] eq $right->[$index];
    return $index;
}

sub _child_count {
    my ($node) = @_;
    return scalar keys %{$node->{children}};
}

sub _only_child {
    my ($node) = @_;
    return (values %{$node->{children}})[0];
}

sub _insert_recursive {
    my ($node, $key, $value) = @_;
    if (!@$key) {
        my $added = !$node->{terminal};
        $node->{terminal} = 1;
        $node->{value} = $value;
        return $added;
    }

    my $first = $key->[0];
    my $edge = $node->{children}{$first};
    if (!defined $edge) {
        $node->{children}{$first} = {
            label => $key,
            child => _new_node(1, $value),
        };
        return 1;
    }

    my $common = _common_prefix_length($key, $edge->{label});
    if ($common == @{$edge->{label}}) {
        return _insert_recursive($edge->{child}, _slice($key, $common), $value);
    }

    my $common_label = [@{$edge->{label}}[0 .. $common - 1]];
    my $old_rest = _slice($edge->{label}, $common);
    my $key_rest = _slice($key, $common);
    my $split_node = _new_node();
    $split_node->{children}{$old_rest->[0]} = {
        label => $old_rest,
        child => $edge->{child},
    };

    if (!@$key_rest) {
        $split_node->{terminal} = 1;
        $split_node->{value} = $value;
    } else {
        $split_node->{children}{$key_rest->[0]} = {
            label => $key_rest,
            child => _new_node(1, $value),
        };
    }

    $node->{children}{$first} = {
        label => $common_label,
        child => $split_node,
    };
    return 1;
}

sub _find_node {
    my ($self, $key) = @_;
    my $characters = _characters($key);
    my $node = $self->{root};
    my $index = 0;

    while ($index < @$characters) {
        my $edge = $node->{children}{$characters->[$index]};
        return undef if !defined $edge;
        for my $offset (0 .. $#{$edge->{label}}) {
            return undef
                if !defined($characters->[$index + $offset])
                || $characters->[$index + $offset] ne $edge->{label}[$offset];
        }
        $index += @{$edge->{label}};
        $node = $edge->{child};
    }
    return $node;
}

sub _delete_recursive {
    my ($node, $key) = @_;
    if (!@$key) {
        return (0, 0) if !$node->{terminal};
        $node->{terminal} = 0;
        $node->{value} = undef;
        return (1, _child_count($node) == 1 ? 1 : 0);
    }

    my $first = $key->[0];
    my $edge = $node->{children}{$first};
    return (0, 0) if !defined $edge;

    my $common = _common_prefix_length($key, $edge->{label});
    return (0, 0) if $common < @{$edge->{label}};

    my ($deleted, $child_mergeable) = _delete_recursive(
        $edge->{child},
        _slice($key, $common),
    );
    return (0, 0) if !$deleted;

    if ($child_mergeable) {
        my $grandchild = _only_child($edge->{child});
        $node->{children}{$first} = {
            label => _concatenate($edge->{label}, $grandchild->{label}),
            child => $grandchild->{child},
        };
    } elsif (!$edge->{child}{terminal} && _child_count($edge->{child}) == 0) {
        delete $node->{children}{$first};
    }

    my $mergeable = !$node->{terminal} && _child_count($node) == 1;
    return (1, $mergeable ? 1 : 0);
}

sub _collect_entries {
    my ($node, $current, $results) = @_;
    push @$results, [$current, $node->{value}] if $node->{terminal};
    for my $first (sort keys %{$node->{children}}) {
        my $edge = $node->{children}{$first};
        _collect_entries(
            $edge->{child},
            $current . join('', @{$edge->{label}}),
            $results,
        );
    }
}

sub _count_nodes {
    my ($node) = @_;
    my $count = 1;
    $count += _count_nodes($_->{child}) for values %{$node->{children}};
    return $count;
}

sub _validate_node {
    my ($node, $is_root) = @_;
    my $endpoints = $node->{terminal} ? 1 : 0;
    my $children = 0;

    while (my ($first, $edge) = each %{$node->{children}}) {
        $children++;
        return (0, 0)
            if ref($edge->{label}) ne 'ARRAY'
            || !@{$edge->{label}}
            || $edge->{label}[0] ne $first
            || ref($edge->{child}) ne 'HASH';
        my ($valid, $child_endpoints) = _validate_node($edge->{child}, 0);
        return (0, 0) if !$valid;
        $endpoints += $child_endpoints;
    }

    return (0, 0) if !$is_root && !$node->{terminal} && $children <= 1;
    return (1, $endpoints);
}

sub new {
    my ($class, $entries) = @_;
    $entries = [] if !defined $entries;
    die "entries must be an array reference\n" if ref($entries) ne 'ARRAY';

    my $self = bless {
        root => _new_node(),
        size => 0,
    }, $class;

    for my $index (0 .. $#$entries) {
        my $entry = $entries->[$index];
        die "entry at index $index must contain a key\n"
            if ref($entry) ne 'ARRAY' || !@$entry;
        my $value = @$entry >= 2 ? $entry->[1] : 1;
        $self->insert($entry->[0], $value);
    }
    return $self;
}

sub insert {
    my ($self, $key) = @_;
    my $value = @_ >= 3 ? $_[2] : 1;
    _assert_string($key, 'key');
    $self->{size}++ if _insert_recursive($self->{root}, _characters($key), $value);
    return $self;
}

sub search {
    my ($self, $key) = @_;
    _assert_string($key, 'key');
    my $node = $self->_find_node($key);
    return defined($node) && $node->{terminal} ? $node->{value} : undef;
}

sub contains_key {
    my ($self, $key) = @_;
    _assert_string($key, 'key');
    my $node = $self->_find_node($key);
    return defined($node) && $node->{terminal} ? 1 : 0;
}

sub key_exists { return shift->contains_key(@_); }
sub contains   { return shift->contains_key(@_); }

sub delete {
    my ($self, $key) = @_;
    _assert_string($key, 'key');
    my ($deleted) = _delete_recursive($self->{root}, _characters($key));
    $self->{size}-- if $deleted;
    return $deleted;
}

sub starts_with {
    my ($self, $prefix) = @_;
    _assert_string($prefix, 'prefix');
    return $self->{size} > 0 ? 1 : 0 if $prefix eq '';

    my $characters = _characters($prefix);
    my $node = $self->{root};
    my $index = 0;
    while ($index < @$characters) {
        my $edge = $node->{children}{$characters->[$index]};
        return 0 if !defined $edge;

        my $remaining = @$characters - $index;
        my $limit = $remaining < @{$edge->{label}}
            ? $remaining
            : scalar @{$edge->{label}};
        for my $offset (0 .. $limit - 1) {
            return 0 if $characters->[$index + $offset] ne $edge->{label}[$offset];
        }
        return 1 if $remaining <= @{$edge->{label}};
        $index += @{$edge->{label}};
        $node = $edge->{child};
    }
    return $node->{terminal} || _child_count($node) ? 1 : 0;
}

sub entries {
    my ($self) = @_;
    my $results = [];
    _collect_entries($self->{root}, '', $results);
    return $results;
}

sub all_entries { return shift->entries(@_); }

sub keys {
    my ($self) = @_;
    return [map { $_->[0] } @{$self->entries}];
}

sub all_words { return shift->keys(@_); }

sub words_with_prefix {
    my ($self, $prefix) = @_;
    _assert_string($prefix, 'prefix');
    return $self->keys if $prefix eq '';

    my $characters = _characters($prefix);
    my $node = $self->{root};
    my $index = 0;
    my $path = '';

    while ($index < @$characters) {
        my $edge = $node->{children}{$characters->[$index]};
        return [] if !defined $edge;

        my $remaining = @$characters - $index;
        my $limit = $remaining < @{$edge->{label}}
            ? $remaining
            : scalar @{$edge->{label}};
        for my $offset (0 .. $limit - 1) {
            return [] if $characters->[$index + $offset] ne $edge->{label}[$offset];
        }

        $path .= join('', @{$edge->{label}});
        if ($remaining <= @{$edge->{label}}) {
            my $entries = [];
            _collect_entries($edge->{child}, $path, $entries);
            return [map { $_->[0] } @$entries];
        }
        $index += @{$edge->{label}};
        $node = $edge->{child};
    }

    my $entries = [];
    _collect_entries($node, $path, $entries);
    return [map { $_->[0] } @$entries];
}

sub longest_prefix_match {
    my ($self, $input) = @_;
    _assert_string($input, 'input');
    my $characters = _characters($input);
    my $node = $self->{root};
    my $index = 0;
    my $path = '';
    my $best = $node->{terminal} ? '' : undef;

    while ($index < @$characters) {
        my $edge = $node->{children}{$characters->[$index]};
        last if !defined $edge;

        my $matches = 1;
        for my $offset (0 .. $#{$edge->{label}}) {
            if (!defined($characters->[$index + $offset])
                || $characters->[$index + $offset] ne $edge->{label}[$offset]) {
                $matches = 0;
                last;
            }
        }
        last if !$matches;

        $path .= join('', @{$edge->{label}});
        $index += @{$edge->{label}};
        $node = $edge->{child};
        $best = $path if $node->{terminal};
    }
    return $best;
}

sub to_hash {
    my ($self) = @_;
    my $result = {};
    $result->{$_->[0]} = $_->[1] for @{$self->entries};
    return $result;
}

sub to_map { return shift->to_hash(@_); }

sub each {
    my ($self, $callback) = @_;
    die "callback must be a code reference\n" if ref($callback) ne 'CODE';
    $callback->(@$_) for @{$self->entries};
    return $self;
}

sub size   { return $_[0]->{size}; }
sub length { return $_[0]->{size}; }

sub is_empty {
    my ($self) = @_;
    return $self->{size} == 0 ? 1 : 0;
}

sub node_count {
    my ($self) = @_;
    return _count_nodes($self->{root});
}

sub is_valid {
    my ($self) = @_;
    my ($valid, $endpoints) = _validate_node($self->{root}, 1);
    return $valid && $endpoints == $self->{size} ? 1 : 0;
}

sub as_string {
    my ($self) = @_;
    return sprintf('RadixTree(%d keys, %d nodes)', $self->{size}, $self->node_count);
}

1;

__END__

=head1 NAME

CodingAdventures::RadixTree - path-compressed radix tree for Unicode keys

=head1 SYNOPSIS

  my $tree = CodingAdventures::RadixTree->new;
  $tree->insert('app', 1)->insert('apple', 2);
  my $match = $tree->longest_prefix_match('apples');

=cut
