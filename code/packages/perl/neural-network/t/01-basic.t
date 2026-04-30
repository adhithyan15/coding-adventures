use strict;
use warnings;
use Test::More;
use CodingAdventures::NeuralNetwork qw(create_neural_graph add_input add_constant add_weighted_sum add_activation add_output create_xor_network wi);

my $graph = create_neural_graph('tiny');
add_input($graph, 'x0');
add_input($graph, 'x1');
add_constant($graph, 'bias', 1.0);
add_weighted_sum($graph, 'sum', [wi('x0', 0.25, 'x0_to_sum'), wi('x1', 0.75, 'x1_to_sum'), wi('bias', -1.0, 'bias_to_sum')]);
add_activation($graph, 'relu', 'sum', 'relu', {}, 'sum_to_relu');
add_output($graph, 'out', 'relu', 'prediction', {}, 'relu_to_out');
is(scalar @{$graph->incoming_edges('sum')}, 3, 'sum has three weighted inputs');
is($graph->topological_sort->[-1], 'out', 'output sorts last');
ok((grep { $_->{id} eq 'h_or_to_out' } @{create_xor_network()->graph->edges}), 'xor includes hidden output edge');
done_testing;
