use strict;
use warnings;
use Test2::V0;
use List::Util qw(sum);

use CodingAdventures::Graph;

sub make_graph {
    my ($repr) = @_;
    my $graph = CodingAdventures::Graph->new(repr => $repr);
    $graph->add_edge('London', 'Paris', 300.0);
    $graph->add_edge('London', 'Amsterdam', 520.0);
    $graph->add_edge('Paris', 'Berlin', 878.0);
    $graph->add_edge('Amsterdam', 'Berlin', 655.0);
    $graph->add_edge('Amsterdam', 'Brussels', 180.0);
    return $graph;
}

for my $repr (qw(adjacency_list adjacency_matrix)) {
    my $graph = CodingAdventures::Graph->new(repr => $repr);
    $graph->add_node('A');
    $graph->add_edge('A', 'B', 2.5);

    ok($graph->has_node('A'), "$repr has node");
    ok($graph->has_edge('A', 'B'), "$repr has edge");
    ok($graph->has_edge('B', 'A'), "$repr undirected edge");
    is($graph->edge_weight('A', 'B'), 2.5, "$repr edge weight");
    is($graph->degree('A'), 1, "$repr degree");
    is($graph->nodes, ['A', 'B'], "$repr nodes");
}

for my $repr (qw(adjacency_list adjacency_matrix)) {
    my $graph = make_graph($repr);
    is($graph->neighbors('Amsterdam'), ['Berlin', 'Brussels', 'London'], "$repr neighbors");
    is($graph->neighbors_weighted('Amsterdam'), {
        Berlin => 655.0,
        Brussels => 180.0,
        London => 520.0,
    }, "$repr weighted neighbors");
    is($graph->bfs('London'), ['London', 'Amsterdam', 'Paris', 'Berlin', 'Brussels'], "$repr bfs");
    is($graph->dfs('London'), ['London', 'Amsterdam', 'Berlin', 'Paris', 'Brussels'], "$repr dfs");
    ok($graph->is_connected, "$repr connected");
    ok($graph->has_cycle, "$repr cycle detection");
    is($graph->shortest_path('London', 'Berlin'), ['London', 'Amsterdam', 'Berlin'], "$repr shortest path");
    my $mst = $graph->minimum_spanning_tree;
    is(scalar(@$mst), $graph->size - 1, "$repr mst size");
    is(sum(map { $_->[2] } @$mst), 1655.0, "$repr mst weight");
}

for my $repr (qw(adjacency_list adjacency_matrix)) {
    my $graph = CodingAdventures::Graph->new(repr => $repr);
    $graph->add_edge('A', 'B');
    $graph->add_edge('B', 'C');
    $graph->add_edge('C', 'A');
    $graph->add_edge('D', 'E');
    ok($graph->has_cycle, "$repr cycle detection");
    my $components = $graph->connected_components;
    ok((grep { join(',', @$_) eq 'A,B,C' } @$components), "$repr abc component");
    ok((grep { join(',', @$_) eq 'D,E' } @$components), "$repr de component");
}

for my $repr (qw(adjacency_list adjacency_matrix)) {
    my $graph = CodingAdventures::Graph->new(repr => $repr);
    $graph->add_edge('A', 'B');
    $graph->add_node('C');
    like(dies { $graph->minimum_spanning_tree }, qr/not connected/, "$repr disconnected mst");
}

done_testing;
