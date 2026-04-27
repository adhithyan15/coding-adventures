#!/usr/bin/env perl

use strict;
use warnings;
use Test2::V0;
use CodingAdventures::Graph;

# ============================================================================
# Test Construction
# ============================================================================

subtest "Graph construction" => sub {
    my $g = CodingAdventures::Graph->new();
    ok(defined $g, 'new() creates a graph');

    my $g_matrix = CodingAdventures::Graph->new('matrix');
    ok(defined $g_matrix, 'new(matrix) creates matrix-backed graph');

    is($g->len, 0, 'empty graph has 0 nodes');
    is(scalar($g->nodes), 0, 'empty graph has no nodes');
    is(scalar($g->edges), 0, 'empty graph has no edges');
};

# ============================================================================
# Test Node Operations
# ============================================================================

subtest "Node operations - adjacency list" => sub {
    my $g = CodingAdventures::Graph->new('list');

    $g->add_node('A');
    ok($g->has_node('A'), 'has_node returns true after add_node');
    is($g->len, 1, 'len returns 1 after adding one node');

    my @nodes = sort $g->nodes;
    is(scalar @nodes, 1, 'nodes returns 1 element');
    is($nodes[0], 'A', 'nodes contains A');

    $g->add_node('B');
    $g->add_node('C');
    is($g->len, 3, 'multiple nodes added correctly');

    ok($g->has_node('B'), 'has_node(B) returns true');
    ok(!$g->has_node('Z'), 'has_node(Z) returns false');

    # Duplicate add is a no-op
    $g->add_node('A');
    is($g->len, 3, 'adding duplicate node is no-op');
};

subtest "Node operations - adjacency matrix" => sub {
    my $g = CodingAdventures::Graph->new('matrix');

    $g->add_node('X');
    ok($g->has_node('X'), 'has_node returns true');
    is($g->len, 1, 'len returns 1');

    $g->add_node('Y');
    $g->add_node('Z');
    is($g->len, 3, 'len returns 3');
};

subtest "remove_node" => sub {
    my $g = CodingAdventures::Graph->new('list');
    $g->add_node('A');
    $g->add_node('B');
    $g->add_edge('A', 'B', 1.0);

    $g->remove_node('A');
    ok(!$g->has_node('A'), 'node removed');
    is($g->len, 1, 'len decreased');
    ok(!$g->has_edge('A', 'B'), 'edge involving removed node is gone');

    my $removed = 0;
    eval { $g->remove_node('NONEXISTENT') };
    ok($@, 'remove_node raises error for nonexistent node');
};

# ============================================================================
# Test Edge Operations
# ============================================================================

subtest "Edge operations" => sub {
    my $g = CodingAdventures::Graph->new('list');

    $g->add_edge('A', 'B', 1.5);
    ok($g->has_node('A'), 'add_edge creates nodes');
    ok($g->has_node('B'), 'add_edge creates both nodes');
    ok($g->has_edge('A', 'B'), 'has_edge returns true');
    ok($g->has_edge('B', 'A'), 'edge is undirected');

    is($g->edge_weight('A', 'B'), 1.5, 'edge_weight correct');
    is($g->edge_weight('B', 'A'), 1.5, 'edge_weight symmetric');

    my @edges = $g->edges;
    is(scalar @edges, 1, 'edges returns 1 edge');
};

subtest "remove_edge" => sub {
    my $g = CodingAdventures::Graph->new('list');
    $g->add_edge('A', 'B', 1.0);
    $g->add_edge('B', 'C', 1.0);

    $g->remove_edge('A', 'B');
    ok(!$g->has_edge('A', 'B'), 'edge removed');
    ok(!$g->has_edge('B', 'A'), 'undirected edge removed both ways');
    ok($g->has_edge('B', 'C'), 'other edges unaffected');

    eval { $g->remove_edge('A', 'Z') };
    ok($@, 'remove_edge raises error for nonexistent edge');
};

subtest "Weighted edges" => sub {
    my $g = CodingAdventures::Graph->new('list');

    $g->add_edge('A', 'B');  # default weight 1.0
    is($g->edge_weight('A', 'B'), 1.0, 'default weight is 1.0');

    $g->add_edge('C', 'D', 2.5);
    is($g->edge_weight('C', 'D'), 2.5, 'custom weight stored');

    $g->add_edge('A', 'B', 3.0);  # update weight
    is($g->edge_weight('A', 'B'), 3.0, 'weight updated');
};

# ============================================================================
# Test Neighbour Queries
# ============================================================================

subtest "neighbors and degree" => sub {
    my $g = CodingAdventures::Graph->new('list');
    $g->add_edge('A', 'B', 1.0);
    $g->add_edge('A', 'C', 2.0);
    $g->add_edge('B', 'C', 3.0);

    my @a_neighbors = sort $g->neighbors('A');
    is(scalar @a_neighbors, 2, 'neighbors returns 2 neighbors');
    ok((grep { $_ eq 'B' } @a_neighbors), 'neighbors includes B');
    ok((grep { $_ eq 'C' } @a_neighbors), 'neighbors includes C');

    is($g->degree('A'), 2, 'degree is count of neighbors');
    is($g->degree('B'), 2, 'degree correct');
    is($g->degree('C'), 2, 'degree correct');

    eval { $g->neighbors('NONEXISTENT') };
    ok($@, 'neighbors raises error for nonexistent node');
};

subtest "neighbors_weighted" => sub {
    my $g = CodingAdventures::Graph->new('list');
    $g->add_edge('A', 'B', 1.5);
    $g->add_edge('A', 'C', 2.5);

    my %neighbors = $g->neighbors_weighted('A');
    is($neighbors{B}, 1.5, 'weight for B correct');
    is($neighbors{C}, 2.5, 'weight for C correct');
};

# ============================================================================
# Test BFS
# ============================================================================

subtest "BFS" => sub {
    my $g = CodingAdventures::Graph->new('list');
    $g->add_edge('A', 'B', 1.0);
    $g->add_edge('B', 'C', 1.0);
    $g->add_edge('A', 'D', 1.0);

    my @bfs_result = CodingAdventures::Graph::bfs($g, 'A');
    # BFS from A should visit in level order
    is($bfs_result[0], 'A', 'start node first');
    is(scalar @bfs_result, 4, 'visits all reachable nodes');
};

subtest "DFS" => sub {
    my $g = CodingAdventures::Graph->new('list');
    $g->add_edge('A', 'B', 1.0);
    $g->add_edge('B', 'C', 1.0);
    $g->add_edge('A', 'D', 1.0);

    my @dfs_result = CodingAdventures::Graph::dfs($g, 'A');
    is($dfs_result[0], 'A', 'start node first');
    is(scalar @dfs_result, 4, 'visits all reachable nodes');
};

# ============================================================================
# Test Connectivity
# ============================================================================

subtest "is_connected" => sub {
    my $g = CodingAdventures::Graph->new('list');
    ok(CodingAdventures::Graph::is_connected($g), 'empty graph is connected');

    $g->add_node('A');
    ok(CodingAdventures::Graph::is_connected($g), 'single node is connected');

    $g->add_node('B');
    ok(!CodingAdventures::Graph::is_connected($g), 'disconnected nodes return false');

    $g->add_edge('A', 'B', 1.0);
    ok(CodingAdventures::Graph::is_connected($g), 'connected after adding edge');
};

subtest "connected_components" => sub {
    my $g = CodingAdventures::Graph->new('list');
    $g->add_edge('A', 'B', 1.0);
    $g->add_edge('C', 'D', 1.0);
    $g->add_node('E');

    my @components = CodingAdventures::Graph::connected_components($g);
    is(scalar @components, 3, 'three components');
};

# ============================================================================
# Test Cycles
# ============================================================================

subtest "has_cycle" => sub {
    my $g = CodingAdventures::Graph->new('list');
    $g->add_edge('A', 'B', 1.0);
    $g->add_edge('B', 'C', 1.0);
    ok(!CodingAdventures::Graph::has_cycle($g), 'tree has no cycle');

    $g->add_edge('C', 'A', 1.0);  # create cycle
    ok(CodingAdventures::Graph::has_cycle($g), 'triangle has cycle');
};

# ============================================================================
# Test Shortest Path
# ============================================================================

subtest "shortest_path" => sub {
    my $g = CodingAdventures::Graph->new('list');
    $g->add_edge('A', 'B', 1.0);
    $g->add_edge('B', 'C', 1.0);
    $g->add_edge('A', 'C', 10.0);

    my @path = @{CodingAdventures::Graph::shortest_path($g, 'A', 'C')};
    is(scalar @path, 3, 'shortest path has 3 nodes');
    is($path[0], 'A', 'shortest path starts with A');
    is($path[1], 'B', 'shortest path goes through B');
    is($path[2], 'C', 'shortest path ends with C');

    my @no_path = @{CodingAdventures::Graph::shortest_path($g, 'A', 'Z')};
    is(scalar @no_path, 0, 'no path returns empty list');
};

# ============================================================================
# Test Minimum Spanning Tree
# ============================================================================

subtest "minimum_spanning_tree" => sub {
    my $g = CodingAdventures::Graph->new('list');
    $g->add_edge('A', 'B', 1.0);
    $g->add_edge('B', 'C', 2.0);
    $g->add_edge('A', 'C', 3.0);

    my @mst = CodingAdventures::Graph::minimum_spanning_tree($g);
    is(scalar @mst, 2, 'MST has V-1 edges');

    # Total weight should be 3 (1.0 + 2.0)
    my $total_weight = 0;
    for my $edge (@mst) {
        $total_weight += $edge->[2];
    }
    is($total_weight, 3.0, 'MST has correct total weight');
};

# ============================================================================
# Test Bipartite
# ============================================================================

subtest "is_bipartite" => sub {
    my $g = CodingAdventures::Graph->new('list');
    $g->add_edge('A', 'B', 1.0);
    $g->add_edge('B', 'C', 1.0);
    ok(CodingAdventures::Graph::is_bipartite($g), 'path is bipartite');

    $g->add_edge('C', 'A', 1.0);  # create triangle
    ok(!CodingAdventures::Graph::is_bipartite($g), 'triangle is not bipartite');
};

# ============================================================================
# Test Both Representations
# ============================================================================

subtest "adjacency_matrix representation" => sub {
    my $g = CodingAdventures::Graph->new('matrix');
    $g->add_edge('A', 'B', 1.5);
    $g->add_edge('B', 'C', 2.0);

    ok($g->has_edge('A', 'B'), 'has_edge works on matrix');
    is($g->edge_weight('B', 'C'), 2.0, 'edge_weight works on matrix');

    my @neighbors = sort $g->neighbors('B');
    is(scalar @neighbors, 2, 'neighbors returns 2 neighbors');
    ok((grep { $_ eq 'A' } @neighbors), 'neighbors includes A');
    ok((grep { $_ eq 'C' } @neighbors), 'neighbors includes C');

    my @bfs = CodingAdventures::Graph::bfs($g, 'A');
    is(scalar @bfs, 3, 'BFS works on matrix representation');
};

done_testing();
