package CodingAdventures::DirectedGraph;

# ============================================================================
# CodingAdventures::DirectedGraph — Pure-Perl directed graph implementation
# ============================================================================
#
# A directed graph (or "digraph") is a set of nodes connected by edges,
# where each edge has a direction. Think of a one-way street map:
# you can travel from A to B, but that doesn't mean you can go B to A.
#
# In a build system, nodes are packages and edges are dependencies:
# if package A depends on package B, there's an edge from B to A
# (B must be built before A).
#
# Why a directed graph?
# =====================
#
# Dependency relationships form a DAG (Directed Acyclic Graph). A DAG has
# no cycles. Key algorithms on a DAG are:
#
#   - Topological sort: order nodes so every dependency comes before the
#     things that depend on it. This gives you a valid build order.
#
#   - Cycle detection: verify the graph has no circular dependencies.
#
#   - Shortest path (Dijkstra): find the minimum-weight path between nodes.
#
# Data structure
# ==============
#
# We store both forward edges (node -> its successors) and reverse edges
# (node -> its predecessors) for efficient lookups in both directions.
# This doubles memory usage but makes "who depends on X" queries O(V+E)
# instead of requiring a full graph reversal.
#
#   _forward{node} = { successor1 => 1, successor2 => 1, ... }
#   _reverse{node} = { predecessor1 => 1, predecessor2 => 1, ... }
#   _weights{from}{to} = weight   (optional, for Dijkstra)
#
# This is the coding-adventures monorepo standard directed graph, ported
# from the Lua and Ruby reference implementations.

use strict;
use warnings;
use List::Util qw(reduce);

our $VERSION = '0.01';

# ============================================================================
# Constructor
# ============================================================================

# new()
#
# Creates an empty directed graph.
#
# By default, self-loops (edges from a node to itself) are prohibited.
# A self-loop creates a trivial cycle and makes topological sorting impossible.
sub new {
    my ($class) = @_;
    return bless {
        _forward           => {},   # node -> { successor => 1, ... }
        _reverse           => {},   # node -> { predecessor => 1, ... }
        _weights           => {},   # from -> { to => weight }
        _allow_self_loops  => 0,
    }, $class;
}

# new_allow_self_loops()
#
# Creates a directed graph that allows self-loops (A->A).
# Useful for modeling state machines where a state can transition to itself.
# Note: has_cycle() will return 1 for such graphs.
sub new_allow_self_loops {
    my ($class) = @_;
    my $self = $class->new;
    $self->{_allow_self_loops} = 1;
    return $self;
}

# ============================================================================
# Node operations
# ============================================================================

# add_node($id)
#
# Adds a node to the graph. No-op if the node already exists.
# Both forward and reverse maps are initialized with empty hashes.
sub add_node {
    my ($self, $id) = @_;
    unless (exists $self->{_forward}{$id}) {
        $self->{_forward}{$id} = {};
        $self->{_reverse}{$id} = {};
    }
    return;
}

# has_node($id)
#
# Returns 1 if the node exists, 0 otherwise.
sub has_node {
    my ($self, $id) = @_;
    return exists $self->{_forward}{$id} ? 1 : 0;
}

# remove_node($id)
#
# Removes a node and all its incident edges.
# "Incident" means edges going TO or FROM this node.
#
# Returns 1 on success, dies if node not found.
sub remove_node {
    my ($self, $id) = @_;
    die "node not found: '$id'" unless $self->has_node($id);

    # Remove all edges TO this node: for each predecessor,
    # delete this node from their forward (successor) set.
    for my $pred (keys %{ $self->{_reverse}{$id} }) {
        delete $self->{_forward}{$pred}{$id};
        delete $self->{_weights}{$pred}{$id} if exists $self->{_weights}{$pred};
    }

    # Remove all edges FROM this node: for each successor,
    # delete this node from their reverse (predecessor) set.
    for my $succ (keys %{ $self->{_forward}{$id} }) {
        delete $self->{_reverse}{$succ}{$id};
    }

    # Remove the node itself from both maps.
    delete $self->{_forward}{$id};
    delete $self->{_reverse}{$id};
    delete $self->{_weights}{$id};
    return 1;
}

# nodes()
#
# Returns all nodes as a sorted arrayref. Sorting ensures determinism
# regardless of Perl's hash iteration order.
sub nodes {
    my ($self) = @_;
    return [sort keys %{ $self->{_forward} }];
}

# size()
#
# Returns the number of nodes in the graph.
sub size {
    my ($self) = @_;
    return scalar keys %{ $self->{_forward} };
}

# ============================================================================
# Edge operations
# ============================================================================

# add_edge($from, $to, $weight)
#
# Adds a directed edge from $from to $to.
#
# Both nodes are implicitly added if they don't exist.
# The optional $weight (default 1) is used by shortest_path / Dijkstra.
# Self-loops are rejected unless the graph was created with new_allow_self_loops().
sub add_edge {
    my ($self, $from, $to, $weight) = @_;
    $weight //= 1;

    if ($from eq $to && !$self->{_allow_self_loops}) {
        die "self-loop not allowed: '$from'";
    }

    $self->add_node($from);
    $self->add_node($to);

    $self->{_forward}{$from}{$to} = 1;
    $self->{_reverse}{$to}{$from} = 1;
    $self->{_weights}{$from}{$to} = $weight;

    return;
}

# has_edge($from, $to)
#
# Returns 1 if the edge exists, 0 otherwise.
sub has_edge {
    my ($self, $from, $to) = @_;
    return (exists $self->{_forward}{$from} && exists $self->{_forward}{$from}{$to}) ? 1 : 0;
}

# remove_edge($from, $to)
#
# Removes the edge from $from to $to (nodes remain in graph).
# Dies if the edge doesn't exist.
sub remove_edge {
    my ($self, $from, $to) = @_;
    die "edge not found: '$from' -> '$to'" unless $self->has_edge($from, $to);

    delete $self->{_forward}{$from}{$to};
    delete $self->{_reverse}{$to}{$from};
    delete $self->{_weights}{$from}{$to} if exists $self->{_weights}{$from};
    return 1;
}

# get_weight($from, $to)
#
# Returns the weight of the edge from $from to $to.
# Dies if the edge doesn't exist.
sub get_weight {
    my ($self, $from, $to) = @_;
    die "edge not found: '$from' -> '$to'" unless $self->has_edge($from, $to);
    return $self->{_weights}{$from}{$to} // 1;
}

# edges()
#
# Returns all edges as an arrayref of [$from, $to] pairs, sorted by from then to.
sub edges {
    my ($self) = @_;
    my @result;
    for my $from (sort keys %{ $self->{_forward} }) {
        for my $to (sort keys %{ $self->{_forward}{$from} }) {
            push @result, [$from, $to];
        }
    }
    return \@result;
}

# ============================================================================
# Neighbor queries
# ============================================================================

# successors($node)
#
# Returns the direct successors of $node (nodes this node has edges TO),
# as a sorted arrayref.
sub successors {
    my ($self, $node) = @_;
    die "node not found: '$node'" unless $self->has_node($node);
    return [sort keys %{ $self->{_forward}{$node} }];
}

# predecessors($node)
#
# Returns the direct predecessors of $node (nodes with edges TO this node),
# as a sorted arrayref.
sub predecessors {
    my ($self, $node) = @_;
    die "node not found: '$node'" unless $self->has_node($node);
    return [sort keys %{ $self->{_reverse}{$node} }];
}

# ============================================================================
# DFS — Depth-First Search
# ============================================================================
#
# DFS explores a graph by going as deep as possible before backtracking.
# Think of it like navigating a maze: you always take the first available
# turn and only backtrack when you hit a dead end.
#
# The order of discovery in DFS depends on the order you visit neighbors.
# We sort neighbors alphabetically for deterministic output.
#
# DFS is the foundation of many graph algorithms:
#   - Cycle detection (three-color marking, see has_cycle)
#   - Topological sort
#   - Connected component finding
#   - Strongly connected component algorithms (Tarjan, Kosaraju)

# dfs($start)
#
# Returns all nodes reachable from $start via DFS, as an arrayref in
# traversal order (preorder: node first, then its descendants).
sub dfs {
    my ($self, $start) = @_;
    die "node not found: '$start'" unless $self->has_node($start);

    my @result;
    my %visited;

    # Iterative DFS using an explicit stack (avoids Perl's recursion limit
    # on large graphs). We push neighbors in reverse-sorted order so that
    # the smallest neighbor is popped first (LIFO stack behavior).
    my @stack = ($start);

    while (@stack) {
        my $node = pop @stack;
        next if $visited{$node};
        $visited{$node} = 1;
        push @result, $node;

        # Push neighbors in reverse sorted order so smallest is popped first
        for my $succ (reverse sort keys %{ $self->{_forward}{$node} }) {
            push @stack, $succ unless $visited{$succ};
        }
    }

    return \@result;
}

# ============================================================================
# BFS — Breadth-First Search
# ============================================================================
#
# BFS explores a graph level by level, starting from the source node.
# Think of it like ripples spreading out from a stone dropped in a pond:
# all nodes at distance 1 are visited before any nodes at distance 2.
#
# BFS guarantees the shortest path in an unweighted graph. It's used to
# compute minimum hop count, find the closest node meeting some condition,
# and build level-based partitions (like independent_groups).

# bfs($start)
#
# Returns all nodes reachable from $start via BFS, as an arrayref in
# level-order (each level sorted alphabetically for determinism).
sub bfs {
    my ($self, $start) = @_;
    die "node not found: '$start'" unless $self->has_node($start);

    my @result;
    my %visited;
    my @queue = ($start);
    $visited{$start} = 1;

    while (@queue) {
        my $node = shift @queue;
        push @result, $node;

        for my $succ (sort keys %{ $self->{_forward}{$node} }) {
            unless ($visited{$succ}) {
                $visited{$succ} = 1;
                push @queue, $succ;
            }
        }
    }

    return \@result;
}

# ============================================================================
# Topological sort — Kahn's algorithm
# ============================================================================
#
# Kahn's algorithm produces a linear ordering of nodes where every edge
# goes from an earlier node to a later one. This is the "build order":
# process all dependencies before the things that depend on them.
#
# Algorithm:
#  1. Find all nodes with in-degree 0 (no predecessors = no dependencies)
#  2. Remove them from the graph (conceptually), add to result
#  3. Their successors may now have in-degree 0 — repeat
#  4. If all nodes are removed: valid ordering found
#  5. If some nodes remain: there's a cycle (mutual dependency)
#
# Why Kahn's instead of DFS-based topological sort?
#
# Both are O(V+E), but Kahn's sorts the zero-in-degree queue at each step,
# producing a DETERMINISTIC ordering. DFS-based approaches produce valid
# orderings but the exact order depends on hash iteration order (non-deterministic).

# topological_sort()
#
# Returns an arrayref of nodes in topological order, or dies if a cycle exists.
sub topological_sort {
    my ($self) = @_;

    # Step 1: Compute in-degrees from the reverse adjacency map.
    # In-degree = number of edges pointing TO a node.
    my %in_degree;
    for my $node (keys %{ $self->{_reverse} }) {
        $in_degree{$node} = scalar keys %{ $self->{_reverse}{$node} };
    }

    # Step 2: Collect all nodes with in-degree 0 (the "roots").
    my @queue = sort grep { $in_degree{$_} == 0 } keys %in_degree;

    # Step 3: Process the queue, decrementing in-degrees of successors.
    my @result;
    while (@queue) {
        my $node = shift @queue;   # FIFO: take from front
        push @result, $node;

        # For each successor, decrement its in-degree.
        # If it hits 0, it has no more unprocessed predecessors: add to queue.
        for my $succ (sort keys %{ $self->{_forward}{$node} }) {
            $in_degree{$succ}--;
            if ($in_degree{$succ} == 0) {
                # Insert in sorted order to maintain determinism
                @queue = sort (@queue, $succ);
            }
        }
    }

    # Step 4: If we processed all nodes, we're done. Otherwise there's a cycle.
    my $total = scalar keys %{ $self->{_forward} };
    die "graph contains a cycle" if @result != $total;

    return \@result;
}

# ============================================================================
# Cycle detection — DFS with three-color marking
# ============================================================================
#
# Three-color DFS (from CLRS "Introduction to Algorithms", Ch. 22):
#
#   white = 0: unvisited
#   gray  = 1: in current DFS path (on the recursion stack)
#   black = 2: fully processed (all descendants explored)
#
# Truth table for what happens when we visit a successor:
#
#   Successor color | Meaning                     | Action
#   ----------------+-----------------------------+------------------
#   white (0)       | Not yet visited             | Recurse into it
#   gray  (1)       | On current path = CYCLE!    | Return true
#   black (2)       | Already fully explored      | Skip it
#
# A "back edge" (gray -> gray) indicates we've found a cycle.

# has_cycle()
#
# Returns 1 if the graph contains a cycle, 0 otherwise.
sub has_cycle {
    my ($self) = @_;

    my %color;   # 0=white, 1=gray, 2=black

    # Recursive DFS via a local sub using $self via closure
    # Returns 1 if a cycle is found from $node, 0 otherwise
    my $dfs;
    $dfs = sub {
        my ($node) = @_;
        $color{$node} = 1;   # Mark gray: currently on the DFS stack

        for my $succ (keys %{ $self->{_forward}{$node} }) {
            my $c = $color{$succ} // 0;
            return 1 if $c == 1;   # Gray = back edge = cycle!
            if ($c == 0) {         # White = unvisited
                return 1 if $dfs->($succ);
            }
            # Black = fully explored: skip
        }

        $color{$node} = 2;   # Mark black: fully explored
        return 0;
    };

    # Run DFS from every unvisited node. We sort for determinism.
    for my $node (sort keys %{ $self->{_forward} }) {
        next if ($color{$node} // 0) != 0;   # Skip non-white nodes
        return 1 if $dfs->($node);
    }

    return 0;
}

# ============================================================================
# Shortest path — Dijkstra's algorithm
# ============================================================================
#
# Dijkstra's algorithm finds the minimum-weight path from a source node
# to all other reachable nodes. It works on graphs with non-negative weights.
#
# Algorithm (greedy approach):
#   1. Set distance[source] = 0, distance[all others] = infinity
#   2. Use a priority queue (min-heap) to always process the closest node
#   3. For each neighbor of the current node, if we found a shorter path,
#      update its distance and add it to the queue
#   4. Repeat until all reachable nodes are processed
#
# Why does this work?
# Once we extract a node from the priority queue (with minimum distance),
# we're guaranteed its distance is optimal because all edge weights are
# non-negative. Any other path would have to go through a node we haven't
# seen yet, which must be at least as far away.
#
# Time complexity: O((V + E) log V) with a binary heap.
# Our simple Perl implementation uses an array as a sorted queue.

# shortest_path($from, $to)
#
# Returns the minimum-weight path from $from to $to as:
#   ($distance, \@path)
#
# Where $distance is the total weight and @path is the sequence of nodes.
# Dies if $from or $to doesn't exist, or if no path exists.
sub shortest_path {
    my ($self, $from, $to) = @_;

    die "node not found: '$from'" unless $self->has_node($from);
    die "node not found: '$to'"   unless $self->has_node($to);

    # dist{node} = shortest distance from $from to node
    # prev{node} = the predecessor in the shortest path
    my %dist;
    my %prev;

    # Initialize all distances to "infinity" (use a very large number)
    for my $node (keys %{ $self->{_forward} }) {
        $dist{$node} = 9**9**9;  # Perl's infinity approximation
    }
    $dist{$from} = 0;

    # Priority queue: array of [distance, node], kept sorted by distance.
    # A real implementation would use a proper min-heap, but for correctness
    # and simplicity we sort the array. Performance is O(V^2) worst case.
    my @pq = ([0, $from]);

    my %visited;

    while (@pq) {
        # Pop the node with minimum distance (first element after sorting)
        @pq = sort { $a->[0] <=> $b->[0] } @pq;
        my $item = shift @pq;
        my ($d, $node) = @$item;

        next if $visited{$node};
        $visited{$node} = 1;

        # If we reached the destination, we're done
        last if $node eq $to;

        # Relax edges: if going through $node gives a shorter path to $succ,
        # update $succ's distance and remember $node as its predecessor.
        for my $succ (keys %{ $self->{_forward}{$node} }) {
            next if $visited{$succ};
            my $weight   = $self->{_weights}{$node}{$succ} // 1;
            my $new_dist = $dist{$node} + $weight;

            if ($new_dist < $dist{$succ}) {
                $dist{$succ} = $new_dist;
                $prev{$succ} = $node;
                push @pq, [$new_dist, $succ];
            }
        }
    }

    # If $to is still at infinity, there's no path
    die "no path from '$from' to '$to'"
        if $dist{$to} >= 9**9**9;

    # Reconstruct the path by following prev pointers backwards
    my @path;
    my $curr = $to;
    while (defined $curr) {
        unshift @path, $curr;
        $curr = $prev{$curr};
    }

    return ($dist{$to}, \@path);
}

# ============================================================================
# Transitive closure and affected nodes
# ============================================================================

# transitive_closure($node)
#
# Returns a hashref (node => 1) of all nodes reachable from $node by
# following forward edges. The starting node itself is NOT included
# (unless there's a self-loop or cycle back to it).
sub transitive_closure {
    my ($self, $node) = @_;
    die "node not found: '$node'" unless $self->has_node($node);

    my %visited;
    my @queue = ($node);

    while (@queue) {
        my $curr = shift @queue;
        for my $succ (keys %{ $self->{_forward}{$curr} }) {
            unless ($visited{$succ}) {
                $visited{$succ} = 1;
                push @queue, $succ;
            }
        }
    }

    return \%visited;
}

# affected_nodes(\%changed)
#
# Given a set of changed nodes (hashref: node => 1), returns a hashref of
# all affected nodes: the changed nodes themselves plus everything that
# transitively depends on any of them.
#
# Nodes in %changed that don't exist in the graph are silently ignored.
sub affected_nodes {
    my ($self, $changed) = @_;

    my %affected;
    for my $node (keys %$changed) {
        next unless $self->has_node($node);
        $affected{$node} = 1;
        my $deps = $self->transitive_closure($node);
        $affected{$_} = 1 for keys %$deps;
    }

    return \%affected;
}

# ============================================================================
# Independent groups — parallel execution levels
# ============================================================================
#
# Partitions nodes into levels by topological depth. Nodes at the same
# level have no dependency on each other and can run in parallel.
#
# Example for a diamond graph (A->B, A->C, B->D, C->D):
#
#   Level 0: [A]      — no dependencies
#   Level 1: [B, C]   — depend only on A, can run in parallel
#   Level 2: [D]      — depends on B and C
#
# This is a modified Kahn's algorithm: instead of processing one node at
# a time, we process all in-degree-0 nodes as a batch (one level).

# independent_groups()
#
# Returns an arrayref of arrayrefs. Each inner arrayref is a sorted list
# of nodes at that topological level. Dies if the graph contains a cycle.
sub independent_groups {
    my ($self) = @_;

    # Compute in-degrees
    my %in_degree;
    for my $node (keys %{ $self->{_reverse} }) {
        $in_degree{$node} = scalar keys %{ $self->{_reverse}{$node} };
    }

    # Initial batch: all nodes with in-degree 0
    my @queue = sort grep { $in_degree{$_} == 0 } keys %in_degree;

    my @levels;
    my $processed = 0;

    while (@queue) {
        # All nodes in the current queue form one level
        my @level = sort @queue;
        push @levels, \@level;
        $processed += @level;

        # Find next level by decrementing in-degrees
        my @next;
        for my $node (@queue) {
            for my $succ (keys %{ $self->{_forward}{$node} }) {
                $in_degree{$succ}--;
                push @next, $succ if $in_degree{$succ} == 0;
            }
        }
        @queue = sort @next;
    }

    my $total = scalar keys %{ $self->{_forward} };
    die "graph contains a cycle" if $processed != $total;

    return \@levels;
}

1;

__END__

=head1 NAME

CodingAdventures::DirectedGraph - Pure-Perl directed graph implementation

=head1 SYNOPSIS

    use CodingAdventures::DirectedGraph;

    my $g = CodingAdventures::DirectedGraph->new;

    $g->add_node('A');
    $g->add_edge('A', 'B');      # A -> B with weight 1
    $g->add_edge('B', 'C', 5);   # B -> C with weight 5

    my $order = $g->topological_sort;  # ['A', 'B', 'C']
    my $cycle  = $g->has_cycle;        # 0

    my ($dist, $path) = $g->shortest_path('A', 'C');
    # $dist = 6, $path = ['A', 'B', 'C']

    my $dfs = $g->dfs('A');   # ['A', 'B', 'C']
    my $bfs = $g->bfs('A');   # ['A', 'B', 'C']

=head1 DESCRIPTION

Directed graph with DFS, BFS, topological sort, cycle detection, Dijkstra's
shortest path, transitive closure, and independent group partitioning.

Used by the coding-adventures build tool to determine build order and
identify which packages need rebuilding when source files change.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
