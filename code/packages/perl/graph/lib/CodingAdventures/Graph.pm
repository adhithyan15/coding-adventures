package CodingAdventures::Graph;

use strict;
use warnings;
use List::Util qw(sum);
use Scalar::Util qw(looks_like_number);

our $VERSION = '0.01';

sub new {
    my ($class, %opts) = @_;
    my $repr = $opts{repr} // 'adjacency_list';
    die "repr must be adjacency_list or adjacency_matrix"
        unless $repr eq 'adjacency_list' || $repr eq 'adjacency_matrix';

    return bless {
        _repr       => $repr,
        _adj        => {},
        _node_list  => [],
        _node_index => {},
        _matrix     => [],
        _graph_properties => {},
        _node_properties  => {},
        _edge_properties  => {},
    }, $class;
}

sub repr { $_[0]->{_repr} }
sub size { $_[0]->{_repr} eq 'adjacency_list' ? scalar(keys %{ $_[0]->{_adj} }) : scalar(@{ $_[0]->{_node_list} }) }

sub add_node {
    my ($self, $node, $properties) = @_;
    $properties //= {};
    if ($self->{_repr} eq 'adjacency_list') {
        $self->{_adj}{$node} //= {};
        _merge_properties($self->{_node_properties}, $node, $properties);
        return 1;
    }

    if (exists $self->{_node_index}{$node}) {
        _merge_properties($self->{_node_properties}, $node, $properties);
        return 1;
    }

    my $index = scalar @{ $self->{_node_list} };
    push @{ $self->{_node_list} }, $node;
    $self->{_node_index}{$node} = $index;
    push @$_, undef for @{ $self->{_matrix} };
    push @{ $self->{_matrix} }, [ (undef) x ($index + 1) ];
    _merge_properties($self->{_node_properties}, $node, $properties);
    return 1;
}

sub remove_node {
    my ($self, $node) = @_;
    die "node not found: '$node'" unless $self->has_node($node);

    if ($self->{_repr} eq 'adjacency_list') {
        my $neighbors = $self->{_adj}{$node};
        for my $neighbor (keys %$neighbors) {
            delete $self->{_adj}{$neighbor}{$node};
            delete $self->{_edge_properties}{_edge_key($node, $neighbor)};
        }
        delete $self->{_adj}{$node};
        delete $self->{_node_properties}{$node};
        return 1;
    }

    my $index = delete $self->{_node_index}{$node};
    delete $self->{_edge_properties}{_edge_key($node, $_)} for @{ $self->{_node_list} };
    delete $self->{_node_properties}{$node};
    splice @{ $self->{_node_list} }, $index, 1;
    splice @{ $self->{_matrix} }, $index, 1;
    for my $row (@{ $self->{_matrix} }) {
        splice @$row, $index, 1;
    }
    for my $i (0 .. $#{ $self->{_node_list} }) {
        $self->{_node_index}{ $self->{_node_list}[$i] } = $i;
    }
    return 1;
}

sub has_node {
    my ($self, $node) = @_;
    return $self->{_repr} eq 'adjacency_list'
        ? exists $self->{_adj}{$node}
        : exists $self->{_node_index}{$node};
}

sub nodes {
    my ($self) = @_;
    my @nodes = $self->{_repr} eq 'adjacency_list' ? keys %{ $self->{_adj} } : @{ $self->{_node_list} };
    return [ sort @nodes ];
}

sub add_edge {
    my ($self, $left, $right, $weight, $properties) = @_;
    $weight = 1 unless defined $weight;
    $properties //= {};
    $self->add_node($left);
    $self->add_node($right);

    if ($self->{_repr} eq 'adjacency_list') {
        $self->{_adj}{$left}{$right} = $weight;
        $self->{_adj}{$right}{$left} = $weight;
        _merge_edge_properties($self, $left, $right, $weight, $properties);
        return 1;
    }

    my $li = $self->{_node_index}{$left};
    my $ri = $self->{_node_index}{$right};
    $self->{_matrix}[$li][$ri] = $weight;
    $self->{_matrix}[$ri][$li] = $weight;
    _merge_edge_properties($self, $left, $right, $weight, $properties);
    return 1;
}

sub remove_edge {
    my ($self, $left, $right) = @_;
    die "edge not found: '$left' -- '$right'" unless $self->has_edge($left, $right);

    if ($self->{_repr} eq 'adjacency_list') {
        delete $self->{_adj}{$left}{$right};
        delete $self->{_adj}{$right}{$left};
        delete $self->{_edge_properties}{_edge_key($left, $right)};
        return 1;
    }

    my $li = $self->{_node_index}{$left};
    my $ri = $self->{_node_index}{$right};
    $self->{_matrix}[$li][$ri] = undef;
    $self->{_matrix}[$ri][$li] = undef;
    delete $self->{_edge_properties}{_edge_key($left, $right)};
    return 1;
}

sub has_edge {
    my ($self, $left, $right) = @_;
    if ($self->{_repr} eq 'adjacency_list') {
        return exists $self->{_adj}{$left} && exists $self->{_adj}{$left}{$right} ? 1 : 0;
    }
    return 0 unless exists $self->{_node_index}{$left} && exists $self->{_node_index}{$right};
    return defined $self->{_matrix}[ $self->{_node_index}{$left} ][ $self->{_node_index}{$right} ] ? 1 : 0;
}

sub edge_weight {
    my ($self, $left, $right) = @_;
    die "edge not found: '$left' -- '$right'" unless $self->has_edge($left, $right);
    if ($self->{_repr} eq 'adjacency_list') {
        return $self->{_adj}{$left}{$right};
    }
    return $self->{_matrix}[ $self->{_node_index}{$left} ][ $self->{_node_index}{$right} ];
}

sub graph_properties {
    my ($self) = @_;
    return { %{ $self->{_graph_properties} } };
}

sub set_graph_property {
    my ($self, $key, $value) = @_;
    $self->{_graph_properties}{$key} = $value;
    return 1;
}

sub remove_graph_property {
    my ($self, $key) = @_;
    delete $self->{_graph_properties}{$key};
    return 1;
}

sub node_properties {
    my ($self, $node) = @_;
    die "node not found: '$node'" unless $self->has_node($node);
    return { %{ $self->{_node_properties}{$node} // {} } };
}

sub set_node_property {
    my ($self, $node, $key, $value) = @_;
    die "node not found: '$node'" unless $self->has_node($node);
    $self->{_node_properties}{$node} //= {};
    $self->{_node_properties}{$node}{$key} = $value;
    return 1;
}

sub remove_node_property {
    my ($self, $node, $key) = @_;
    die "node not found: '$node'" unless $self->has_node($node);
    delete $self->{_node_properties}{$node}{$key};
    return 1;
}

sub edge_properties {
    my ($self, $left, $right) = @_;
    die "edge not found: '$left' -- '$right'" unless $self->has_edge($left, $right);
    return {
        %{ $self->{_edge_properties}{_edge_key($left, $right)} // {} },
        weight => $self->edge_weight($left, $right),
    };
}

sub set_edge_property {
    my ($self, $left, $right, $key, $value) = @_;
    die "edge not found: '$left' -- '$right'" unless $self->has_edge($left, $right);
    if ($key eq 'weight') {
        die "edge property 'weight' must be numeric" unless looks_like_number($value);
        _set_edge_weight($self, $left, $right, $value + 0);
    }
    $self->{_edge_properties}{_edge_key($left, $right)} //= {};
    $self->{_edge_properties}{_edge_key($left, $right)}{$key} = $value;
    return 1;
}

sub remove_edge_property {
    my ($self, $left, $right, $key) = @_;
    die "edge not found: '$left' -- '$right'" unless $self->has_edge($left, $right);
    if ($key eq 'weight') {
        _set_edge_weight($self, $left, $right, 1);
        $self->{_edge_properties}{_edge_key($left, $right)} //= {};
        $self->{_edge_properties}{_edge_key($left, $right)}{weight} = 1;
    } else {
        delete $self->{_edge_properties}{_edge_key($left, $right)}{$key};
    }
    return 1;
}

sub edges {
    my ($self) = @_;
    my @result;
    if ($self->{_repr} eq 'adjacency_list') {
        my %seen;
        for my $left (keys %{ $self->{_adj} }) {
            for my $right (keys %{ $self->{_adj}{$left} }) {
                my ($first, $second) = $left le $right ? ($left, $right) : ($right, $left);
                my $key = "$first\0$second";
                next if $seen{$key}++;
                push @result, [$first, $second, $self->{_adj}{$left}{$right}];
            }
        }
    } else {
        for my $row (0 .. $#{ $self->{_node_list} }) {
            for my $col ($row .. $#{ $self->{_node_list} }) {
                my $weight = $self->{_matrix}[$row][$col];
                next unless defined $weight;
                push @result, [ $self->{_node_list}[$row], $self->{_node_list}[$col], $weight ];
            }
        }
    }
    @result = sort {
        $a->[2] <=> $b->[2] || $a->[0] cmp $b->[0] || $a->[1] cmp $b->[1]
    } @result;
    return \@result;
}

sub neighbors {
    my ($self, $node) = @_;
    die "node not found: '$node'" unless $self->has_node($node);
    if ($self->{_repr} eq 'adjacency_list') {
        return [ sort keys %{ $self->{_adj}{$node} } ];
    }
    my $idx = $self->{_node_index}{$node};
    my @neighbors;
    for my $col (0 .. $#{ $self->{_node_list} }) {
        push @neighbors, $self->{_node_list}[$col] if defined $self->{_matrix}[$idx][$col];
    }
    return [ sort @neighbors ];
}

sub neighbors_weighted {
    my ($self, $node) = @_;
    die "node not found: '$node'" unless $self->has_node($node);
    if ($self->{_repr} eq 'adjacency_list') {
        return { %{ $self->{_adj}{$node} } };
    }
    my $idx = $self->{_node_index}{$node};
    my %weights;
    for my $col (0 .. $#{ $self->{_node_list} }) {
        $weights{ $self->{_node_list}[$col] } = $self->{_matrix}[$idx][$col]
            if defined $self->{_matrix}[$idx][$col];
    }
    return \%weights;
}

sub degree {
    my ($self, $node) = @_;
    return scalar @{ $self->neighbors($node) };
}

sub bfs {
    my ($self, $start) = @_;
    die "node not found: '$start'" unless $self->has_node($start);
    my @queue = ($start);
    my %visited = ($start => 1);
    my @result;

    while (@queue) {
        my $node = shift @queue;
        push @result, $node;
        for my $neighbor (@{ $self->neighbors($node) }) {
            next if $visited{$neighbor};
            $visited{$neighbor} = 1;
            push @queue, $neighbor;
        }
    }

    return \@result;
}

sub dfs {
    my ($self, $start) = @_;
    die "node not found: '$start'" unless $self->has_node($start);
    my @stack = ($start);
    my %visited;
    my @result;

    while (@stack) {
        my $node = pop @stack;
        next if $visited{$node}++;
        push @result, $node;
        my @neighbors = reverse @{ $self->neighbors($node) };
        push @stack, grep { !$visited{$_} } @neighbors;
    }

    return \@result;
}

sub is_connected {
    my ($self) = @_;
    my $nodes = $self->nodes;
    return 1 unless @$nodes;
    return scalar(@{ $self->bfs($nodes->[0]) }) == $self->size ? 1 : 0;
}

sub connected_components {
    my ($self) = @_;
    my %remaining = map { $_ => 1 } @{ $self->nodes };
    my @components;

    while (keys %remaining) {
        my ($start) = sort keys %remaining;
        my $component = $self->bfs($start);
        push @components, $component;
        delete @remaining{@$component};
    }

    return \@components;
}

sub has_cycle {
    my ($self) = @_;
    my %visited;
    for my $start (@{ $self->nodes }) {
        next if $visited{$start};
        return 1 if _visit_cycle($self, $start, undef, \%visited);
    }
    return 0;
}

sub shortest_path {
    my ($self, $start, $finish) = @_;
    return [] unless $self->has_node($start) && $self->has_node($finish);
    return [$start] if $start eq $finish;

    my $all_unit = 1;
    for my $edge (@{ $self->edges }) {
        if ($edge->[2] != 1) {
            $all_unit = 0;
            last;
        }
    }

    return $all_unit ? _bfs_shortest_path($self, $start, $finish) : _dijkstra_shortest_path($self, $start, $finish);
}

sub minimum_spanning_tree {
    my ($self) = @_;
    return [] if $self->size <= 1 || !@{ $self->edges };
    die "graph is not connected" unless $self->is_connected;

    my %parent = map { $_ => $_ } @{ $self->nodes };
    my %rank = map { $_ => 0 } @{ $self->nodes };
    my @result;

    for my $edge (@{ $self->edges }) {
        my ($left, $right) = @$edge[0,1];
        if (_find(\%parent, $left) ne _find(\%parent, $right)) {
            _union(\%parent, \%rank, $left, $right);
            push @result, $edge;
        }
    }

    return \@result;
}

sub _visit_cycle {
    my ($self, $node, $parent, $visited) = @_;
    $visited->{$node} = 1;
    for my $neighbor (@{ $self->neighbors($node) }) {
        if (!$visited->{$neighbor}) {
            return 1 if _visit_cycle($self, $neighbor, $node, $visited);
        } elsif (!defined $parent || $neighbor ne $parent) {
            return 1;
        }
    }
    return 0;
}

sub _bfs_shortest_path {
    my ($self, $start, $finish) = @_;
    my @queue = ($start);
    my %parent = ($start => undef);

    while (@queue) {
        my $node = shift @queue;
        last if $node eq $finish;
        for my $neighbor (@{ $self->neighbors($node) }) {
            next if exists $parent{$neighbor};
            $parent{$neighbor} = $node;
            push @queue, $neighbor;
        }
    }

    return [] unless exists $parent{$finish};
    return _reconstruct_path(\%parent, $start, $finish);
}

sub _dijkstra_shortest_path {
    my ($self, $start, $finish) = @_;
    my %distance = map { $_ => 9**9**9 } @{ $self->nodes };
    my %parent;
    $distance{$start} = 0;
    my @queue = ([0, $start]);

    while (@queue) {
        @queue = sort { $a->[0] <=> $b->[0] || $a->[1] cmp $b->[1] } @queue;
        my ($distance, $node) = @{ shift @queue };
        next if $distance > $distance{$node};
        last if $node eq $finish;

        my $weighted = $self->neighbors_weighted($node);
        for my $neighbor (keys %$weighted) {
            my $next = $distance + $weighted->{$neighbor};
            if ($next < $distance{$neighbor}) {
                $distance{$neighbor} = $next;
                $parent{$neighbor} = $node;
                push @queue, [$next, $neighbor];
            }
        }
    }

    return [] if $distance{$finish} == 9**9**9;
    return _reconstruct_path(\%parent, $start, $finish);
}

sub _reconstruct_path {
    my ($parent, $start, $finish) = @_;
    my @path;
    my $current = $finish;
    while (defined $current) {
        push @path, $current;
        $current = $parent->{$current};
    }
    @path = reverse @path;
    return $path[0] eq $start ? \@path : [];
}

sub _find {
    my ($parent, $node) = @_;
    return $node if $parent->{$node} eq $node;
    $parent->{$node} = _find($parent, $parent->{$node});
    return $parent->{$node};
}

sub _union {
    my ($parent, $rank, $left, $right) = @_;
    my $left_root = _find($parent, $left);
    my $right_root = _find($parent, $right);
    return if $left_root eq $right_root;

    if ($rank->{$left_root} < $rank->{$right_root}) {
        ($left_root, $right_root) = ($right_root, $left_root);
    }

    $parent->{$right_root} = $left_root;
    $rank->{$left_root}++ if $rank->{$left_root} == $rank->{$right_root};
}

sub _edge_key {
    my ($left, $right) = @_;
    return $left le $right ? "$left\0$right" : "$right\0$left";
}

sub _merge_properties {
    my ($property_map, $owner, $properties) = @_;
    $property_map->{$owner} //= {};
    for my $key (keys %$properties) {
        $property_map->{$owner}{$key} = $properties->{$key};
    }
}

sub _merge_edge_properties {
    my ($self, $left, $right, $weight, $properties) = @_;
    my $key = _edge_key($left, $right);
    $self->{_edge_properties}{$key} //= {};
    for my $property_key (keys %$properties) {
        $self->{_edge_properties}{$key}{$property_key} = $properties->{$property_key};
    }
    $self->{_edge_properties}{$key}{weight} = $weight;
}

sub _set_edge_weight {
    my ($self, $left, $right, $weight) = @_;
    if ($self->{_repr} eq 'adjacency_list') {
        $self->{_adj}{$left}{$right} = $weight;
        $self->{_adj}{$right}{$left} = $weight;
        return;
    }

    my $li = $self->{_node_index}{$left};
    my $ri = $self->{_node_index}{$right};
    $self->{_matrix}[$li][$ri] = $weight;
    $self->{_matrix}[$ri][$li] = $weight;
}

1;
