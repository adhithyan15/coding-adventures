package CodingAdventures::NeuralNetwork;
use strict;
use warnings;
use Exporter 'import';
our $VERSION = '0.1.0';
our @EXPORT_OK = qw(create_neural_graph create_neural_network add_input add_constant add_weighted_sum add_activation add_output create_xor_network wi);

sub create_neural_graph { return CodingAdventures::NeuralNetwork::Graph->new(@_); }
sub create_neural_network { return CodingAdventures::NeuralNetwork::Network->new(@_); }
sub wi { return { from => $_[0], weight => $_[1], edge_id => $_[2], properties => {} }; }

sub add_input {
    my ($graph, $node, $input_name, $properties) = @_;
    $input_name //= $node; $properties //= {};
    $graph->add_node($node, { %$properties, 'nn.op' => 'input', 'nn.input' => $input_name });
}
sub add_constant {
    my ($graph, $node, $value, $properties) = @_;
    $properties //= {};
    die 'constant value must be finite' if $value != $value;
    $graph->add_node($node, { %$properties, 'nn.op' => 'constant', 'nn.value' => 0.0 + $value });
}
sub add_weighted_sum {
    my ($graph, $node, $inputs, $properties) = @_;
    $properties //= {};
    $graph->add_node($node, { %$properties, 'nn.op' => 'weighted_sum' });
    for my $input (@$inputs) {
        $graph->add_edge($input->{from}, $node, $input->{weight} // 1.0, $input->{properties} // {}, $input->{edge_id});
    }
}
sub add_activation {
    my ($graph, $node, $input, $activation, $properties, $edge_id) = @_;
    $properties //= {};
    $graph->add_node($node, { %$properties, 'nn.op' => 'activation', 'nn.activation' => $activation });
    return $graph->add_edge($input, $node, 1.0, {}, $edge_id);
}
sub add_output {
    my ($graph, $node, $input, $output_name, $properties, $edge_id) = @_;
    $output_name //= $node; $properties //= {};
    $graph->add_node($node, { %$properties, 'nn.op' => 'output', 'nn.output' => $output_name });
    return $graph->add_edge($input, $node, 1.0, {}, $edge_id);
}
sub create_xor_network {
    my ($name) = @_; $name //= 'xor';
    my $network = create_neural_network($name);
    $network->input('x0')->input('x1')->constant('bias', 1.0, { 'nn.role' => 'bias' })
        ->weighted_sum('h_or_sum', [wi('x0', 20, 'x0_to_h_or'), wi('x1', 20, 'x1_to_h_or'), wi('bias', -10, 'bias_to_h_or')], { 'nn.layer' => 'hidden' })
        ->activation('h_or', 'h_or_sum', 'sigmoid', { 'nn.layer' => 'hidden' }, 'h_or_sum_to_h_or')
        ->weighted_sum('h_nand_sum', [wi('x0', -20, 'x0_to_h_nand'), wi('x1', -20, 'x1_to_h_nand'), wi('bias', 30, 'bias_to_h_nand')], { 'nn.layer' => 'hidden' })
        ->activation('h_nand', 'h_nand_sum', 'sigmoid', { 'nn.layer' => 'hidden' }, 'h_nand_sum_to_h_nand')
        ->weighted_sum('out_sum', [wi('h_or', 20, 'h_or_to_out'), wi('h_nand', 20, 'h_nand_to_out'), wi('bias', -30, 'bias_to_out')], { 'nn.layer' => 'output' })
        ->activation('out_activation', 'out_sum', 'sigmoid', { 'nn.layer' => 'output' }, 'out_sum_to_activation')
        ->output('out', 'out_activation', 'prediction', { 'nn.layer' => 'output' }, 'activation_to_out');
    return $network;
}

package CodingAdventures::NeuralNetwork::Graph;
use strict;
use warnings;
sub new {
    my ($class, $name) = @_;
    my $self = bless { graph_properties => { 'nn.version' => '0' }, nodes => [], node_properties => {}, edges => [], next_edge_id => 0 }, $class;
    $self->{graph_properties}{'nn.name'} = $name if defined $name;
    return $self;
}
sub add_node {
    my ($self, $node, $properties) = @_;
    $properties //= {};
    if (!exists $self->{node_properties}{$node}) { push @{$self->{nodes}}, $node; $self->{node_properties}{$node} = {}; }
    $self->{node_properties}{$node} = { %{$self->{node_properties}{$node}}, %$properties };
}
sub nodes { return [@{$_[0]->{nodes}}]; }
sub edges { return [@{$_[0]->{edges}}]; }
sub node_properties { my ($self, $node) = @_; return { %{$self->{node_properties}{$node} // {}} }; }
sub add_edge {
    my ($self, $from, $to, $weight, $properties, $edge_id) = @_;
    $properties //= {}; $weight //= 1.0;
    $self->add_node($from, {}); $self->add_node($to, {});
    if (!defined $edge_id) { $edge_id = 'e' . $self->{next_edge_id}; $self->{next_edge_id}++; }
    push @{$self->{edges}}, { id => $edge_id, from => $from, to => $to, weight => 0.0 + $weight, properties => { %$properties, weight => 0.0 + $weight } };
    return $edge_id;
}
sub incoming_edges { my ($self, $node) = @_; return [grep { $_->{to} eq $node } @{$self->{edges}}]; }
sub topological_sort {
    my ($self) = @_;
    my %indegree = map { $_ => 0 } @{$self->{nodes}};
    for my $edge (@{$self->{edges}}) { $indegree{$edge->{from}} //= 0; $indegree{$edge->{to}} //= 0; $indegree{$edge->{to}}++; }
    my @ready = sort grep { $indegree{$_} == 0 } keys %indegree;
    my @order;
    while (@ready) {
        my $node = shift @ready;
        push @order, $node;
        my @released;
        for my $edge (grep { $_->{from} eq $node } @{$self->{edges}}) {
            $indegree{$edge->{to}}--;
            push @released, $edge->{to} if $indegree{$edge->{to}} == 0;
        }
        push @ready, sort @released;
    }
    die 'neural graph contains a cycle' unless @order == keys %indegree;
    return \@order;
}

package CodingAdventures::NeuralNetwork::Network;
use strict;
use warnings;
sub new { my ($class, $name) = @_; return bless { graph => CodingAdventures::NeuralNetwork::create_neural_graph($name) }, $class; }
sub graph { return $_[0]->{graph}; }
sub input { CodingAdventures::NeuralNetwork::add_input($_[0]->{graph}, $_[1], $_[2] // $_[1], $_[3] // {}); return $_[0]; }
sub constant { CodingAdventures::NeuralNetwork::add_constant($_[0]->{graph}, $_[1], $_[2], $_[3] // {}); return $_[0]; }
sub weighted_sum { CodingAdventures::NeuralNetwork::add_weighted_sum($_[0]->{graph}, $_[1], $_[2], $_[3] // {}); return $_[0]; }
sub activation { CodingAdventures::NeuralNetwork::add_activation($_[0]->{graph}, $_[1], $_[2], $_[3], $_[4] // {}, $_[5]); return $_[0]; }
sub output { CodingAdventures::NeuralNetwork::add_output($_[0]->{graph}, $_[1], $_[2], $_[3] // $_[1], $_[4] // {}, $_[5]); return $_[0]; }

1;
