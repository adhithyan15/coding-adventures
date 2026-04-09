package CodingAdventures::Graph;

use strict;
use warnings;
use Carp qw(croak);

our $VERSION = '0.1.0';

=head1 NAME

CodingAdventures::Graph - Undirected weighted graph data structure

=head1 SYNOPSIS

    use CodingAdventures::Graph;

    my $g = CodingAdventures::Graph->new();  # adjacency list (default)
    # or
    my $g = CodingAdventures::Graph->new('matrix');  # adjacency matrix

    $g->add_node('A');
    $g->add_node('B');
    $g->add_edge('A', 'B', 1.0);

    my @neighbors = $g->neighbors('A');
    my @path = CodingAdventures::Graph::bfs($g, 'A');

=head1 DESCRIPTION

An undirected weighted graph library with two internal representations:

  - Adjacency list (default): O(V + E) space, O(degree) edge lookup
  - Adjacency matrix: O(V²) space, O(1) edge lookup

All public methods work identically on both representations.

Edges are undirected and weighted (default weight 1.0).

=head1 GRAPH THEORY CONCEPTS

A graph G = (V, E) consists of:
  V: vertices (nodes) — anything hashable
  E: edges — unordered pairs {u, v} with optional weights

Undirected means {u,v} == {v,u}.

=cut

# ============================================================================
# Constructor
# ============================================================================

=head2 new( [$repr] )

Create a new Graph. Optional $repr parameter:
  'list' or undef — adjacency list (default, sparse graphs)
  'matrix'        — adjacency matrix (dense graphs)

=cut

sub new {
    my ($class, $repr) = @_;
    $repr //= 'list';

    if ($repr eq 'list') {
        return bless {
            _repr => 'list',
            _adj => {},  # node -> { neighbour -> weight }
        }, $class;
    } elsif ($repr eq 'matrix') {
        return bless {
            _repr => 'matrix',
            _node_list => [],    # ordered list of nodes
            _node_idx => {},     # node -> index
            _matrix => [],       # V×V adjacency matrix
        }, $class;
    } else {
        croak "Invalid representation: $repr (use 'list' or 'matrix')";
    }
}

# ============================================================================
# Node Operations
# ============================================================================

=head2 add_node($node)

Add a node. No-op if the node already exists.

=cut

sub add_node {
    my ($self, $node) = @_;

    if ($self->{_repr} eq 'list') {
        return if exists $self->{_adj}{$node};
        $self->{_adj}{$node} = {};
    } else {
        return if exists $self->{_node_idx}{$node};
        my $idx = @{$self->{_node_list}};
        push @{$self->{_node_list}}, $node;
        $self->{_node_idx}{$node} = $idx;

        # Expand matrix: add column to existing rows
        for my $row (@{$self->{_matrix}}) {
            push @$row, 0.0;
        }
        # Add new row
        push @{$self->{_matrix}}, [ (0.0) x ($idx + 1) ];
    }
}

=head2 remove_node($node)

Remove a node and all its incident edges. Raises an error if node doesn't exist.

=cut

sub remove_node {
    my ($self, $node) = @_;

    if ($self->{_repr} eq 'list') {
        croak "Node not found: $node" unless exists $self->{_adj}{$node};

        # Remove all edges touching this node
        for my $neighbour (keys %{$self->{_adj}{$node}}) {
            delete $self->{_adj}{$neighbour}{$node};
        }
        delete $self->{_adj}{$node};
    } else {
        croak "Node not found: $node" unless exists $self->{_node_idx}{$node};

        my $idx = delete $self->{_node_idx}{$node};
        splice @{$self->{_node_list}}, $idx, 1;

        # Update indices for nodes that shifted down
        for my $i ($idx .. $#{$self->{_node_list}}) {
            $self->{_node_idx}{$self->{_node_list}[$i]} = $i;
        }

        # Remove row
        splice @{$self->{_matrix}}, $idx, 1;

        # Remove column from remaining rows
        for my $row (@{$self->{_matrix}}) {
            splice @$row, $idx, 1;
        }
    }
}

=head2 has_node($node)

Return 1 if node exists, 0 otherwise.

=cut

sub has_node {
    my ($self, $node) = @_;

    return exists $self->{_adj}{$node} if $self->{_repr} eq 'list';
    return exists $self->{_node_idx}{$node};
}

=head2 nodes()

Return list of all nodes.

=cut

sub nodes {
    my ($self) = @_;

    return keys %{$self->{_adj}} if $self->{_repr} eq 'list';
    return @{$self->{_node_list}};
}

# ============================================================================
# Edge Operations
# ============================================================================

=head2 add_edge($u, $v, [$weight])

Add an undirected edge between u and v (default weight 1.0).
Both nodes are added if they don't exist.

=cut

sub add_edge {
    my ($self, $u, $v, $weight) = @_;
    $weight //= 1.0;

    $self->add_node($u);
    $self->add_node($v);

    if ($self->{_repr} eq 'list') {
        $self->{_adj}{$u}{$v} = $weight;
        $self->{_adj}{$v}{$u} = $weight;
    } else {
        my $i = $self->{_node_idx}{$u};
        my $j = $self->{_node_idx}{$v};
        $self->{_matrix}[$i][$j] = $weight;
        $self->{_matrix}[$j][$i] = $weight;
    }
}

=head2 remove_edge($u, $v)

Remove the edge between u and v. Raises an error if edge doesn't exist.

=cut

sub remove_edge {
    my ($self, $u, $v) = @_;

    if ($self->{_repr} eq 'list') {
        croak "Edge not found: ($u, $v)"
            unless exists $self->{_adj}{$u} && exists $self->{_adj}{$u}{$v};

        delete $self->{_adj}{$u}{$v};
        delete $self->{_adj}{$v}{$u};
    } else {
        croak "Edge not found: ($u, $v)"
            unless exists $self->{_node_idx}{$u} && exists $self->{_node_idx}{$v};

        my $i = $self->{_node_idx}{$u};
        my $j = $self->{_node_idx}{$v};

        croak "Edge not found: ($u, $v)" if $self->{_matrix}[$i][$j] == 0.0;

        $self->{_matrix}[$i][$j] = 0.0;
        $self->{_matrix}[$j][$i] = 0.0;
    }
}

=head2 has_edge($u, $v)

Return 1 if edge exists, 0 otherwise.

=cut

sub has_edge {
    my ($self, $u, $v) = @_;

    if ($self->{_repr} eq 'list') {
        return (exists $self->{_adj}{$u} && exists $self->{_adj}{$u}{$v}) ? 1 : 0;
    } else {
        return 0 unless exists $self->{_node_idx}{$u} && exists $self->{_node_idx}{$v};
        my $i = $self->{_node_idx}{$u};
        my $j = $self->{_node_idx}{$v};
        return ($self->{_matrix}[$i][$j] != 0.0) ? 1 : 0;
    }
}

=head2 edges()

Return list of (u, v, weight) triples, each edge appearing exactly once.

=cut

sub edges {
    my ($self) = @_;
    my @result;

    if ($self->{_repr} eq 'list') {
        my %seen;
        for my $u (keys %{$self->{_adj}}) {
            for my $v (keys %{$self->{_adj}{$u}}) {
                my $key = join(',', sort ($u, $v));
                next if $seen{$key};
                $seen{$key} = 1;
                push @result, [$u, $v, $self->{_adj}{$u}{$v}];
            }
        }
    } else {
        my $n = @{$self->{_node_list}};
        for my $i (0 .. $n - 1) {
            for my $j ($i + 1 .. $n - 1) {
                my $w = $self->{_matrix}[$i][$j];
                if ($w != 0.0) {
                    push @result, [$self->{_node_list}[$i], $self->{_node_list}[$j], $w];
                }
            }
        }
    }

    return @result;
}

=head2 edge_weight($u, $v)

Return the weight of edge (u, v). Raises an error if edge doesn't exist.

=cut

sub edge_weight {
    my ($self, $u, $v) = @_;

    if ($self->{_repr} eq 'list') {
        croak "Edge not found: ($u, $v)"
            unless exists $self->{_adj}{$u} && exists $self->{_adj}{$u}{$v};
        return $self->{_adj}{$u}{$v};
    } else {
        croak "Edge not found: ($u, $v)"
            unless exists $self->{_node_idx}{$u} && exists $self->{_node_idx}{$v};
        my $i = $self->{_node_idx}{$u};
        my $j = $self->{_node_idx}{$v};
        my $w = $self->{_matrix}[$i][$j];
        croak "Edge not found: ($u, $v)" if $w == 0.0;
        return $w;
    }
}

# ============================================================================
# Neighbourhood Queries
# ============================================================================

=head2 neighbors($node)

Return list of neighbouring nodes. Raises an error if node doesn't exist.

=cut

sub neighbors {
    my ($self, $node) = @_;

    if ($self->{_repr} eq 'list') {
        croak "Node not found: $node" unless exists $self->{_adj}{$node};
        return keys %{$self->{_adj}{$node}};
    } else {
        croak "Node not found: $node" unless exists $self->{_node_idx}{$node};
        my $idx = $self->{_node_idx}{$node};
        my @neighbors;
        for my $j (0 .. $#{$self->{_matrix}[$idx]}) {
            push @neighbors, $self->{_node_list}[$j] if $self->{_matrix}[$idx][$j] != 0.0;
        }
        return @neighbors;
    }
}

=head2 neighbors_weighted($node)

Return hash of { neighbor => weight }. Raises an error if node doesn't exist.

=cut

sub neighbors_weighted {
    my ($self, $node) = @_;

    if ($self->{_repr} eq 'list') {
        croak "Node not found: $node" unless exists $self->{_adj}{$node};
        return %{$self->{_adj}{$node}};
    } else {
        croak "Node not found: $node" unless exists $self->{_node_idx}{$node};
        my $idx = $self->{_node_idx}{$node};
        my %result;
        for my $j (0 .. $#{$self->{_matrix}[$idx]}) {
            my $w = $self->{_matrix}[$idx][$j];
            if ($w != 0.0) {
                $result{$self->{_node_list}[$j]} = $w;
            }
        }
        return %result;
    }
}

=head2 degree($node)

Return the number of incident edges. Raises an error if node doesn't exist.

=cut

sub degree {
    my ($self, $node) = @_;
    return scalar($self->neighbors($node));
}

# ============================================================================
# Dunder Methods
# ============================================================================

=head2 len($graph) / scalar($graph)

Return the number of nodes. Usage: my $count = scalar keys $graph->nodes;

=cut

sub len {
    my ($self) = @_;
    return scalar($self->nodes);
}

# ============================================================================
# Algorithms (exported as functions)
# ============================================================================

=head2 bfs($graph, $start)

Breadth-first search from start node. Returns list of nodes in BFS order.
Time: O(V + E). Space: O(V).

=cut

sub bfs {
    my ($graph, $start) = @_;

    my %visited = ($start => 1);
    my @queue = ($start);
    my @result;

    while (@queue) {
        my $node = shift @queue;
        push @result, $node;

        for my $neighbor (sort $graph->neighbors($node)) {
            unless ($visited{$neighbor}) {
                $visited{$neighbor} = 1;
                push @queue, $neighbor;
            }
        }
    }

    return @result;
}

=head2 dfs($graph, $start)

Depth-first search from start node. Returns list of nodes in DFS order.
Time: O(V + E). Space: O(V).

=cut

sub dfs {
    my ($graph, $start) = @_;

    my %visited;
    my @stack = ($start);
    my @result;

    while (@stack) {
        my $node = pop @stack;
        next if $visited{$node};

        $visited{$node} = 1;
        push @result, $node;

        # Push in reverse order so first (alphabetically) is on top
        for my $neighbor (reverse sort $graph->neighbors($node)) {
            push @stack, $neighbor unless $visited{$neighbor};
        }
    }

    return @result;
}

=head2 is_connected($graph)

Return 1 if graph is connected (all nodes reachable from any node), 0 otherwise.
Time: O(V + E).

=cut

sub is_connected {
    my ($graph) = @_;

    return 1 if $graph->len == 0;

    my @nodes = $graph->nodes;
    my $start = $nodes[0];
    my @reachable = bfs($graph, $start);

    return (scalar @reachable == $graph->len) ? 1 : 0;
}

=head2 connected_components($graph)

Return list of connected components. Each component is a list of nodes.
Time: O(V + E).

=cut

sub connected_components {
    my ($graph) = @_;

    my %unvisited = map { $_ => 1 } $graph->nodes;
    my @components;

    while (keys %unvisited) {
        my $start = (keys %unvisited)[0];
        my @component = bfs($graph, $start);
        push @components, [@component];

        for my $node (@component) {
            delete $unvisited{$node};
        }
    }

    return @components;
}

=head2 has_cycle($graph)

Return 1 if graph contains a cycle, 0 otherwise.
Uses iterative DFS. Time: O(V + E).

=cut

sub has_cycle {
    my ($graph) = @_;

    my %visited;

    for my $start ($graph->nodes) {
        next if $visited{$start};

        my @stack = ([$start, undef]);  # [node, parent]

        while (@stack) {
            my ($node, $parent) = @{pop @stack};
            next if $visited{$node};

            $visited{$node} = 1;

            for my $neighbor ($graph->neighbors($node)) {
                unless ($visited{$neighbor}) {
                    push @stack, [$neighbor, $node];
                } elsif ($neighbor ne ($parent // '')) {
                    # Back edge to visited node that isn't parent -> cycle
                    return 1;
                }
            }
        }
    }

    return 0;
}

=head2 shortest_path($graph, $start, $end)

Return list of nodes in shortest (lowest-weight) path from start to end.
Returns empty list if no path exists.

Uses BFS for unweighted graphs (O(V+E)) or Dijkstra for weighted (O((V+E) log V)).

=cut

sub shortest_path {
    my ($graph, $start, $end) = @_;

    return [$start] if $start eq $end && $graph->has_node($start);
    return [] if $start eq $end;

    # Check if all weights are 1.0 (unweighted)
    my $all_unit = 1;
    for my $edge ($graph->edges) {
        if ($edge->[2] != 1.0) {
            $all_unit = 0;
            last;
        }
    }

    if ($all_unit) {
        return _bfs_path($graph, $start, $end);
    } else {
        return _dijkstra($graph, $start, $end);
    }
}

sub _bfs_path {
    my ($graph, $start, $end) = @_;

    my %parent = ($start => undef);
    my @queue = ($start);

    while (@queue) {
        my $node = shift @queue;
        last if $node eq $end;

        for my $neighbor ($graph->neighbors($node)) {
            unless (exists $parent{$neighbor}) {
                $parent{$neighbor} = $node;
                push @queue, $neighbor;
            }
        }
    }

    return [] unless exists $parent{$end};

    # Trace back from end to start
    my @path;
    my $cur = $end;
    while (defined $cur) {
        unshift @path, $cur;
        $cur = $parent{$cur};
    }

    return \@path;
}

sub _dijkstra {
    my ($graph, $start, $end) = @_;

    return [] unless $graph->has_node($end);

    my %dist = map { $_ => 'inf' } $graph->nodes;
    my %parent;
    $dist{$start} = 0;

    my @heap = ([$start, 0]);  # [node, distance]
    my $counter = 0;

    while (@heap) {
        @heap = sort { $a->[1] <=> $b->[1] } @heap;
        my ($node, $d) = @{shift @heap};

        next if $d > $dist{$node};
        last if $node eq $end;

        my %neighbors = $graph->neighbors_weighted($node);
        for my $neighbor (keys %neighbors) {
            my $weight = $neighbors{$neighbor};
            my $new_dist = $dist{$node} + $weight;

            if ($dist{$neighbor} eq 'inf' || $new_dist < $dist{$neighbor}) {
                $dist{$neighbor} = $new_dist;
                $parent{$neighbor} = $node;
                push @heap, [$neighbor, $new_dist];
            }
        }
    }

    return [] if $dist{$end} eq 'inf';

    # Trace back
    my @path;
    my $cur = $end;
    while (defined $cur) {
        unshift @path, $cur;
        $cur = $parent{$cur};
    }

    return \@path;
}

=head2 minimum_spanning_tree($graph)

Return list of (u, v, weight) edges forming the minimum spanning tree.
Uses Kruskal's algorithm with Union-Find. Time: O(E log E).

Raises error if graph is not connected.

=cut

sub minimum_spanning_tree {
    my ($graph) = @_;

    my @nodes = $graph->nodes;
    return [] unless @nodes;

    my @edges = $graph->edges;
    @edges = sort { $a->[2] <=> $b->[2] } @edges;

    my %uf_parent = map { $_ => $_ } @nodes;
    my %uf_rank = map { $_ => 0 } @nodes;

    my @mst;
    for my $edge (@edges) {
        my ($u, $v, $w) = @$edge;
        my $root_u = _find(\%uf_parent, $u);
        my $root_v = _find(\%uf_parent, $v);

        if ($root_u ne $root_v) {
            _union(\%uf_parent, \%uf_rank, $u, $v);
            push @mst, [$u, $v, $w];
            last if @mst == @nodes - 1;
        }
    }

    if (@mst < @nodes - 1 && @nodes > 1) {
        croak "Graph is not connected — no spanning tree exists";
    }

    return @mst;
}

sub _find {
    my ($parent, $x) = @_;
    if ($parent->{$x} ne $x) {
        $parent->{$x} = _find($parent, $parent->{$x});  # path compression
    }
    return $parent->{$x};
}

sub _union {
    my ($parent, $rank, $a, $b) = @_;
    my $ra = _find($parent, $a);
    my $rb = _find($parent, $b);
    return if $ra eq $rb;

    if ($rank->{$ra} < $rank->{$rb}) {
        ($ra, $rb) = ($rb, $ra);
    }
    $parent->{$rb} = $ra;
    $rank->{$ra}++ if $rank->{$ra} == $rank->{$rb};
}

=head2 is_bipartite($graph)

Return 1 if graph is bipartite (can be 2-colored), 0 otherwise.
Time: O(V + E).

=cut

sub is_bipartite {
    my ($graph) = @_;

    my %color;  # node -> 0 or 1 (or undef if unvisited)

    for my $start ($graph->nodes) {
        next if exists $color{$start};

        my @queue = ($start);
        $color{$start} = 0;

        while (@queue) {
            my $node = shift @queue;
            my $node_color = $color{$node};

            for my $neighbor ($graph->neighbors($node)) {
                if (!exists $color{$neighbor}) {
                    $color{$neighbor} = 1 - $node_color;
                    push @queue, $neighbor;
                } elsif ($color{$neighbor} == $node_color) {
                    # Adjacent nodes have same color -> not bipartite
                    return 0;
                }
            }
        }
    }

    return 1;
}

=head1 LICENSE

MIT

=cut

1;
