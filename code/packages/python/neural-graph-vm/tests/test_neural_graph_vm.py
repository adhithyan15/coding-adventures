from neural_graph_vm import (
    NeuralGraphCompileError,
    compile_neural_graph_to_bytecode,
    compile_neural_network_to_bytecode,
    run_neural_bytecode_forward,
)
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


def tiny_weighted_sum_graph():
    graph = create_neural_graph("tiny-weighted-sum")
    add_input(graph, "x0")
    add_input(graph, "x1")
    add_constant(graph, "bias", 1.0)
    add_weighted_sum(graph, "sum", [
        WeightedInput("bias", -1.0, "bias_to_sum"),
        WeightedInput("x0", 0.25, "w0", {"nn.trainable": True}),
        WeightedInput("x1", 0.75, "w1", {"nn.trainable": True}),
    ])
    add_activation(graph, "relu", "sum", "relu", edge_id="sum_to_relu")
    add_output(graph, "out", "relu", "prediction", edge_id="relu_to_out")
    return graph


def test_compiles_forward_bytecode():
    bytecode = compile_neural_graph_to_bytecode(tiny_weighted_sum_graph())

    assert bytecode.magic == "CANN"
    assert bytecode.version == 0
    assert [instruction.op for instruction in bytecode.functions[0].instructions] == [
        "LOAD_CONST",
        "LOAD_INPUT",
        "LOAD_INPUT",
        "LOAD_EDGE_WEIGHT",
        "MUL",
        "LOAD_EDGE_WEIGHT",
        "MUL",
        "LOAD_EDGE_WEIGHT",
        "MUL",
        "ADD",
        "ACTIVATE",
        "STORE_OUTPUT",
    ]


def test_runs_scalar_forward_interpreter():
    bytecode = compile_neural_graph_to_bytecode(tiny_weighted_sum_graph())
    assert run_neural_bytecode_forward(bytecode, {"x0": 4.0, "x1": 8.0}) == {"prediction": 6.0}


def test_compiles_chainable_network():
    network = (
        create_neural_network("tiny-network")
        .input("x0")
        .input("x1")
        .weighted_sum("sum", [WeightedInput("x0", 0.25, "w0"), WeightedInput("x1", 0.75, "w1")])
        .output("out", "sum", "prediction")
    )
    bytecode = compile_neural_network_to_bytecode(network)

    assert run_neural_bytecode_forward(bytecode, {"x0": 4.0, "x1": 8.0}) == {"prediction": 7.0}


def test_runs_xor_network():
    bytecode = compile_neural_network_to_bytecode(create_xor_network())
    predictions = [
        run_neural_bytecode_forward(bytecode, {"x0": 0.0, "x1": 0.0})["prediction"],
        run_neural_bytecode_forward(bytecode, {"x0": 0.0, "x1": 1.0})["prediction"],
        run_neural_bytecode_forward(bytecode, {"x0": 1.0, "x1": 0.0})["prediction"],
        run_neural_bytecode_forward(bytecode, {"x0": 1.0, "x1": 1.0})["prediction"],
    ]

    assert predictions[0] < 0.01
    assert predictions[1] > 0.99
    assert predictions[2] > 0.99
    assert predictions[3] < 0.01


def test_rejects_unsupported_ops():
    graph = create_neural_graph()
    graph.add_node("custom", {"nn.op": "custom_kernel"})

    try:
        compile_neural_graph_to_bytecode(graph)
    except NeuralGraphCompileError:
        return
    raise AssertionError("expected unsupported op to fail")
