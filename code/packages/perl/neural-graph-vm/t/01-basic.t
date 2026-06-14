use strict;
use warnings;
use Test::More;
use lib '../neural-network/lib';
use CodingAdventures::NeuralNetwork qw(create_neural_graph add_input add_constant add_weighted_sum add_activation add_output create_xor_network wi);
use CodingAdventures::NeuralGraphVM qw(compile_neural_graph_to_bytecode compile_neural_network_to_bytecode run_neural_bytecode_forward);

sub tiny_graph {
    my $graph = create_neural_graph('tiny');
    add_input($graph, 'x0'); add_input($graph, 'x1'); add_constant($graph, 'bias', 1.0);
    add_weighted_sum($graph, 'sum', [wi('x0', 0.25, 'x0_to_sum'), wi('x1', 0.75, 'x1_to_sum'), wi('bias', -1.0, 'bias_to_sum')]);
    add_activation($graph, 'relu', 'sum', 'relu', {}, 'sum_to_relu'); add_output($graph, 'out', 'relu', 'prediction', {}, 'relu_to_out');
    return $graph;
}
my $outputs = run_neural_bytecode_forward(compile_neural_graph_to_bytecode(tiny_graph()), { x0 => 4.0, x1 => 8.0 });
ok(abs($outputs->{prediction} - 6.0) < 1e-9, 'tiny weighted sum predicts 6');
my $xor = compile_neural_network_to_bytecode(create_xor_network());
for my $case ([0,0,0], [0,1,1], [1,0,1], [1,1,0]) {
    my ($x0, $x1, $expected) = @$case;
    my $prediction = run_neural_bytecode_forward($xor, { x0 => $x0, x1 => $x1 })->{prediction};
    ok($expected ? $prediction > 0.99 : $prediction < 0.01, "xor $x0,$x1");
}
done_testing;
