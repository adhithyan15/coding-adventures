use strict;
use warnings;
use Test2::V0;

use CodingAdventures::DirectedGraph;

# ============================================================================
# Tests for CodingAdventures::DirectedGraph
# ============================================================================

# ============================================================================
# Node operations
# ============================================================================

subtest 'Node: add_node and has_node' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    ok(!$g->has_node('A'), 'A not present initially');
    $g->add_node('A');
    ok($g->has_node('A'), 'A present after add_node');
    $g->add_node('A');  # no-op
    is($g->size, 1, 'size still 1 after duplicate add');
};

subtest 'Node: size and nodes list' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_node('C');
    $g->add_node('A');
    $g->add_node('B');
    is($g->size, 3, 'size == 3');
    is($g->nodes, ['A', 'B', 'C'], 'nodes returned in sorted order');
};

subtest 'Node: remove_node cleans up edges' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'B');
    $g->add_edge('B', 'C');
    $g->remove_node('B');
    ok(!$g->has_node('B'), 'B removed');
    ok(!$g->has_edge('A', 'B'), 'edge A->B removed');
    ok(!$g->has_edge('B', 'C'), 'edge B->C removed');
    ok($g->has_node('A'), 'A still present');
    ok($g->has_node('C'), 'C still present');
};

# ============================================================================
# Edge operations
# ============================================================================

subtest 'Edge: add_edge and has_edge' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    ok(!$g->has_edge('A', 'B'), 'no edge before adding');
    $g->add_edge('A', 'B');
    ok($g->has_edge('A', 'B'), 'edge A->B exists after add');
    ok(!$g->has_edge('B', 'A'), 'edge B->A does not exist (directed)');
};

subtest 'Edge: add_edge implicitly adds nodes' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('X', 'Y');
    ok($g->has_node('X'), 'X auto-added');
    ok($g->has_node('Y'), 'Y auto-added');
};

subtest 'Edge: self-loops rejected by default' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    ok(dies { $g->add_edge('A', 'A') }, 'self-loop rejected in default graph');
};

subtest 'Edge: self-loops allowed with new_allow_self_loops' => sub {
    my $g = CodingAdventures::DirectedGraph->new_allow_self_loops;
    $g->add_edge('A', 'A');
    ok($g->has_edge('A', 'A'), 'self-loop allowed with new_allow_self_loops');
};

subtest 'Edge: remove_edge' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'B');
    $g->remove_edge('A', 'B');
    ok(!$g->has_edge('A', 'B'), 'edge removed');
    ok($g->has_node('A'), 'nodes remain after edge removal');
    ok($g->has_node('B'), 'nodes remain after edge removal');
};

subtest 'Edge: weighted edge and get_weight' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'B', 5);
    is($g->get_weight('A', 'B'), 5, 'weight retrieved correctly');
};

subtest 'Edge: default weight is 1' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'B');
    is($g->get_weight('A', 'B'), 1, 'default weight is 1');
};

subtest 'Edge: edges() returns sorted list' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('B', 'C');
    $g->add_edge('A', 'B');
    $g->add_edge('A', 'C');
    my $edges = $g->edges;
    is($edges->[0], ['A', 'B'], 'first edge: A->B');
    is($edges->[1], ['A', 'C'], 'second edge: A->C');
    is($edges->[2], ['B', 'C'], 'third edge: B->C');
};

# ============================================================================
# Neighbor queries
# ============================================================================

subtest 'Neighbors: successors' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'B');
    $g->add_edge('A', 'C');
    is($g->successors('A'), ['B', 'C'], 'successors of A are B and C (sorted)');
    is($g->successors('B'), [], 'B has no successors');
};

subtest 'Neighbors: predecessors' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'C');
    $g->add_edge('B', 'C');
    is($g->predecessors('C'), ['A', 'B'], 'predecessors of C are A and B (sorted)');
    is($g->predecessors('A'), [], 'A has no predecessors');
};

# ============================================================================
# DFS and BFS
# ============================================================================

subtest 'DFS: traversal order from A in linear graph' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'B');
    $g->add_edge('B', 'C');
    $g->add_edge('C', 'D');
    my $result = $g->dfs('A');
    is($result->[0], 'A', 'DFS starts at A');
    ok(scalar(grep { $_ eq 'D' } @$result) == 1, 'DFS reaches D');
};

subtest 'BFS: level-order from A' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'B');
    $g->add_edge('A', 'C');
    $g->add_edge('B', 'D');
    $g->add_edge('C', 'D');
    my $result = $g->bfs('A');
    is($result->[0], 'A', 'BFS starts at A');
    # B and C are at level 1, D at level 2
    ok(scalar @$result == 4, 'all 4 nodes visited');
};

# ============================================================================
# Topological sort
# ============================================================================

subtest 'Topological sort: linear chain A->B->C' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'B');
    $g->add_edge('B', 'C');
    my $order = $g->topological_sort;
    is($order, ['A', 'B', 'C'], 'topological order A, B, C');
};

subtest 'Topological sort: diamond graph' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'B');
    $g->add_edge('A', 'C');
    $g->add_edge('B', 'D');
    $g->add_edge('C', 'D');
    my $order = $g->topological_sort;
    # A must come first, D must come last
    is($order->[0], 'A', 'A is first');
    is($order->[-1], 'D', 'D is last');
    is(scalar @$order, 4, 'all 4 nodes in result');
};

subtest 'Topological sort: dies on cycle' => sub {
    my $g = CodingAdventures::DirectedGraph->new_allow_self_loops;
    $g->add_edge('A', 'B');
    $g->add_edge('B', 'A');  # cycle
    ok(dies { $g->topological_sort }, 'topological_sort dies on cycle');
};

# ============================================================================
# Cycle detection
# ============================================================================

subtest 'has_cycle: DAG has no cycle' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'B');
    $g->add_edge('B', 'C');
    is($g->has_cycle, 0, 'linear chain has no cycle');
};

subtest 'has_cycle: detects cycle' => sub {
    my $g = CodingAdventures::DirectedGraph->new_allow_self_loops;
    $g->add_edge('A', 'B');
    $g->add_edge('B', 'C');
    $g->add_edge('C', 'A');  # cycle: A->B->C->A
    is($g->has_cycle, 1, 'cycle A->B->C->A detected');
};

# ============================================================================
# Shortest path (Dijkstra)
# ============================================================================

subtest 'Shortest path: simple chain' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'B', 1);
    $g->add_edge('B', 'C', 2);
    my ($dist, $path) = $g->shortest_path('A', 'C');
    is($dist, 3, 'shortest distance A to C is 3');
    is($path, ['A', 'B', 'C'], 'path is A -> B -> C');
};

subtest 'Shortest path: chooses cheaper route' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'B', 1);
    $g->add_edge('B', 'C', 1);
    $g->add_edge('A', 'C', 10);   # direct but expensive
    my ($dist, $path) = $g->shortest_path('A', 'C');
    is($dist, 2, 'shortest distance via B is 2');
    is($path, ['A', 'B', 'C'], 'path goes through B');
};

subtest 'Shortest path: same node' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_node('A');
    my ($dist, $path) = $g->shortest_path('A', 'A');
    is($dist, 0, 'distance from A to A is 0');
    is($path, ['A'], 'path is just [A]');
};

# ============================================================================
# Transitive closure and independent groups
# ============================================================================

subtest 'Transitive closure' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'B');
    $g->add_edge('B', 'C');
    $g->add_edge('B', 'D');
    my $tc = $g->transitive_closure('A');
    ok($tc->{B}, 'B reachable from A');
    ok($tc->{C}, 'C reachable from A');
    ok($tc->{D}, 'D reachable from A');
    ok(!$tc->{A}, 'A not in its own closure');
};

subtest 'Independent groups: diamond' => sub {
    my $g = CodingAdventures::DirectedGraph->new;
    $g->add_edge('A', 'B');
    $g->add_edge('A', 'C');
    $g->add_edge('B', 'D');
    $g->add_edge('C', 'D');
    my $levels = $g->independent_groups;
    is($levels->[0], ['A'],      'level 0: just A');
    is($levels->[1], ['B', 'C'], 'level 1: B and C (can run in parallel)');
    is($levels->[2], ['D'],      'level 2: just D');
};

subtest 'Independent groups: dies on cycle' => sub {
    my $g = CodingAdventures::DirectedGraph->new_allow_self_loops;
    $g->add_edge('A', 'B');
    $g->add_edge('B', 'A');
    ok(dies { $g->independent_groups }, 'independent_groups dies on cycle');
};

done_testing;
