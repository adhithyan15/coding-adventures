"""XOR demo: no hidden layer fails, one hidden layer succeeds."""

from single_layer_network import SingleLayerNetwork
from two_layer_network import (
    TwoLayerNetwork,
    create_xor_warm_start_parameters,
)
from neural_network import create_xor_network
from neural_graph_vm import compile_neural_network_to_bytecode, run_neural_bytecode_forward

XOR_INPUTS = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]
XOR_TARGETS = [[0.0], [1.0], [1.0], [0.0]]


def format_row(inputs, target, prediction, hidden=None):
    rounded = 1 if prediction >= 0.5 else 0
    hidden_text = "" if hidden is None else f" hidden={[round(v, 4) for v in hidden]}"
    return f"{inputs} target={target} prediction={prediction:.4f} rounded={rounded}{hidden_text}"


def run_linear_failure():
    model = SingleLayerNetwork.with_shape(2, 1, "sigmoid")
    history = model.fit(XOR_INPUTS, XOR_TARGETS, learning_rate=1.0, epochs=50000)
    print(f"No hidden layer after many runs: loss {history[-1].loss:.4f}")
    for inputs, target, prediction in zip(XOR_INPUTS, XOR_TARGETS, model.predict(XOR_INPUTS)):
        print(format_row(inputs, target[0], prediction[0]))


def run_hidden_success():
    model = TwoLayerNetwork(create_xor_warm_start_parameters(), learning_rate=0.5)
    passed = model.inspect(XOR_INPUTS)
    print("\nWith one hidden layer:")
    for inputs, target, prediction, hidden in zip(
        XOR_INPUTS,
        XOR_TARGETS,
        passed.predictions,
        passed.hidden_activations,
    ):
        print(format_row(inputs, target[0], prediction[0], hidden))


def run_graph_vm_success():
    bytecode = compile_neural_network_to_bytecode(create_xor_network("xor-graph-vm"))
    print("\nGraph API -> bytecode VM:")
    for inputs, target in zip(XOR_INPUTS, XOR_TARGETS):
        outputs = run_neural_bytecode_forward(bytecode, {"x0": inputs[0], "x1": inputs[1]})
        print(format_row(inputs, target[0], outputs["prediction"]))
    print(f"Compiled {len(bytecode.functions[0].instructions)} bytecode instructions from the graph")


if __name__ == "__main__":
    run_linear_failure()
    run_hidden_success()
    run_graph_vm_success()
