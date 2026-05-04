package CodingAdventures::NeuralGraphVM;
use strict;
use warnings;
use Exporter 'import';
use CodingAdventures::NeuralNetwork;
our $VERSION = '0.1.0';
our @EXPORT_OK = qw(compile_neural_graph_to_bytecode compile_neural_network_to_bytecode run_neural_bytecode_forward apply_neural_activation);

sub compile_neural_network_to_bytecode { return compile_neural_graph_to_bytecode($_[0]->graph); }
sub compile_neural_graph_to_bytecode {
    my ($graph) = @_;
    my %values;
    my $next_value_id = 0;
    my $alloc = sub { return 'v' . $next_value_id++; };
    my @instructions;
    for my $node (@{$graph->topological_sort}) {
        my $props = $graph->node_properties($node);
        my $op = $props->{'nn.op'} // 'weighted_sum';
        if ($op eq 'input') {
            my $dst = $alloc->(); $values{$node} = $dst;
            push @instructions, { op => 'LOAD_INPUT', dst => $dst, input_name => $props->{'nn.input'} // $node, source_node => $node };
        } elsif ($op eq 'constant') {
            my $dst = $alloc->(); $values{$node} = $dst;
            push @instructions, { op => 'LOAD_CONST', dst => $dst, value => $props->{'nn.value'}, source_node => $node };
        } elsif ($op eq 'weighted_sum') {
            my @terms;
            for my $edge (sort { $a->{id} cmp $b->{id} } @{$graph->incoming_edges($node)}) {
                my $weight_value = $alloc->(); my $term_value = $alloc->();
                push @instructions, { op => 'LOAD_EDGE_WEIGHT', dst => $weight_value, edge_id => $edge->{id}, source_edge => $edge->{id} };
                push @instructions, { op => 'MUL', dst => $term_value, left => $values{$edge->{from}}, right => $weight_value, source_edge => $edge->{id} };
                push @terms, $term_value;
            }
            my $dst = $alloc->(); $values{$node} = $dst;
            push @instructions, @terms ? { op => 'ADD', dst => $dst, inputs => \@terms, source_node => $node } : { op => 'LOAD_CONST', dst => $dst, value => 0.0, source_node => $node };
        } elsif ($op eq 'activation') {
            my $dst = $alloc->(); $values{$node} = $dst;
            push @instructions, { op => 'ACTIVATE', dst => $dst, input => single_input_value($graph, \%values, $node), activation => $props->{'nn.activation'} // 'relu', source_node => $node };
        } elsif ($op eq 'output') {
            my $input = single_input_value($graph, \%values, $node); $values{$node} = $input;
            push @instructions, { op => 'STORE_OUTPUT', output_name => $props->{'nn.output'} // $node, input => $input, source_node => $node };
        } else { die "unsupported neural graph op: $op"; }
    }
    return { magic => 'CANN', version => 0, nodes => $graph->nodes, edges => [map { { id => $_->{id}, from => $_->{from}, to => $_->{to}, weight => $_->{weight} } } @{$graph->edges}], functions => [{ id => 'forward', kind => 'forward', instructions => \@instructions }] };
}
sub run_neural_bytecode_forward {
    my ($module, $inputs) = @_;
    my %values; my %edge_weights = map { $_->{id} => $_->{weight} } @{$module->{edges}}; my %outputs;
    my ($forward) = grep { $_->{kind} eq 'forward' } @{$module->{functions}};
    die 'neural bytecode module has no forward function' unless $forward;
    for my $inst (@{$forward->{instructions}}) {
        my $op = $inst->{op};
        if ($op eq 'LOAD_INPUT') { $values{$inst->{dst}} = $inputs->{$inst->{input_name}}; }
        elsif ($op eq 'LOAD_CONST') { $values{$inst->{dst}} = $inst->{value} // 0.0; }
        elsif ($op eq 'LOAD_EDGE_WEIGHT') { $values{$inst->{dst}} = $edge_weights{$inst->{edge_id}} // 1.0; }
        elsif ($op eq 'MUL') { $values{$inst->{dst}} = $values{$inst->{left}} * $values{$inst->{right}}; }
        elsif ($op eq 'ADD') { my $sum = 0.0; $sum += $values{$_} for @{$inst->{inputs} // []}; $values{$inst->{dst}} = $sum; }
        elsif ($op eq 'ACTIVATE') { $values{$inst->{dst}} = apply_neural_activation($values{$inst->{input}}, $inst->{activation} // 'relu'); }
        elsif ($op eq 'STORE_OUTPUT') { $outputs{$inst->{output_name} // 'output'} = $values{$inst->{input}}; }
        else { die "unsupported opcode: $op"; }
    }
    return \%outputs;
}
sub apply_neural_activation {
    my ($value, $activation) = @_;
    return $value > 0 ? $value : 0.0 if $activation eq 'relu';
    return 1.0 / (1.0 + exp(-$value)) if $activation eq 'sigmoid';
    return (exp($value) - exp(-$value)) / (exp($value) + exp(-$value)) if $activation eq 'tanh';
    return $value;
}
sub single_input_value {
    my ($graph, $values, $node) = @_;
    my $incoming = $graph->incoming_edges($node);
    die "node $node expects exactly one input" unless @$incoming == 1;
    return $values->{$incoming->[0]{from}};
}
1;
