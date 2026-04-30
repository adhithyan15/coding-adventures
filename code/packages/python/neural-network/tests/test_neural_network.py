from neural_network import (
    WeightedInput,
    add_activation,
    add_constant,
    add_input,
    add_output,
    add_weighted_sum,
    create_neural_graph,
    create_neural_network,
    create_xor_network,
)


def test_creates_neural_graph_metadata():
    graph = create_neural_graph("tiny-model")

    assert graph.graph_properties() == {"nn.version": "0", "nn.name": "tiny-model"}


def test_authors_primitive_metadata():
    graph = create_neural_graph()

    add_input(graph, "x0")
    add_input(graph, "x1", "feature")
    add_constant(graph, "bias", 1.0)
    add_weighted_sum(
        graph,
        "sum",
        [
            WeightedInput("x0", 0.25, "w0", {"nn.trainable": True}),
            WeightedInput("x1", 0.75, "w1"),
            WeightedInput("bias", -0.1, "bias_to_sum"),
        ],
    )
    add_activation(graph, "relu", "sum", "relu", edge_id="sum_to_relu")
    add_output(graph, "out", "relu", "prediction", edge_id="relu_to_out")

    assert graph.node_properties("x0") == {"nn.op": "input", "nn.input": "x0"}
    assert graph.node_properties("x1") == {"nn.op": "input", "nn.input": "feature"}
    assert graph.node_properties("sum") == {"nn.op": "weighted_sum"}
    assert graph.node_properties("bias") == {"nn.op": "constant", "nn.value": 1.0}
    assert graph.node_properties("relu") == {"nn.op": "activation", "nn.activation": "relu"}
    assert graph.node_properties("out") == {"nn.op": "output", "nn.output": "prediction"}
    assert graph.edge_properties("w0") == {"nn.trainable": True, "weight": 0.25}


def test_chainable_authoring_api():
    network = (
        create_neural_network("chain")
        .input("x0")
        .input("x1")
        .weighted_sum("sum", [WeightedInput("x0", 0.5, "w0"), WeightedInput("x1", 0.5, "w1")])
        .activation("relu", "sum", "relu", edge_id="sum_to_relu")
        .output("out", "relu", "prediction", edge_id="relu_to_out")
    )

    assert network.graph.nodes() == ["x0", "x1", "sum", "relu", "out"]
    assert network.graph.topological_sort() == ["x0", "x1", "sum", "relu", "out"]


def test_authors_xor_network():
    network = create_xor_network()

    assert network.graph.graph_properties()["nn.name"] == "xor"
    assert network.graph.node_properties("bias")["nn.value"] == 1.0
    assert network.graph.node_properties("h_or")["nn.activation"] == "sigmoid"
    assert network.graph.edge_properties("bias_to_out")["weight"] == -30.0
    assert "out" in network.graph.topological_sort()
