"""Reference Neural Graph VM compiler and scalar interpreter."""

from __future__ import annotations

from dataclasses import dataclass
from math import exp, tanh

from neural_network import NeuralGraph, NeuralNetwork

NeuralBytecodeOpcode = str


@dataclass(frozen=True)
class NeuralBytecodeInstruction:
    op: NeuralBytecodeOpcode
    dst: str | None = None
    input_name: str | None = None
    output_name: str | None = None
    edge_id: str | None = None
    value: float | None = None
    left: str | None = None
    right: str | None = None
    inputs: list[str] | None = None
    input: str | None = None
    activation: str | None = None
    source_node: str | None = None
    source_edge: str | None = None


@dataclass(frozen=True)
class NeuralBytecodeFunction:
    id: str
    kind: str
    instructions: list[NeuralBytecodeInstruction]


@dataclass(frozen=True)
class NeuralBytecodeGraphEdge:
    id: str
    from_node: str
    to_node: str
    weight: float


@dataclass(frozen=True)
class NeuralBytecodeModule:
    magic: str
    version: int
    nodes: list[str]
    edges: list[NeuralBytecodeGraphEdge]
    functions: list[NeuralBytecodeFunction]


class NeuralGraphCompileError(ValueError):
    pass


def compile_neural_network_to_bytecode(network: NeuralNetwork) -> NeuralBytecodeModule:
    return compile_neural_graph_to_bytecode(network.graph)


def compile_neural_graph_to_bytecode(graph: NeuralGraph) -> NeuralBytecodeModule:
    order = graph.topological_sort()
    instructions: list[NeuralBytecodeInstruction] = []
    values: dict[str, str] = {}
    next_value_id = 0

    def allocate_value() -> str:
        nonlocal next_value_id
        value_id = f"v{next_value_id}"
        next_value_id += 1
        return value_id

    for node in order:
        properties = graph.node_properties(node)
        op = str(properties.get("nn.op", "weighted_sum"))

        if op == "input":
            dst = allocate_value()
            values[node] = dst
            instructions.append(NeuralBytecodeInstruction(
                "LOAD_INPUT",
                dst=dst,
                input_name=str(properties.get("nn.input", node)),
                source_node=node,
            ))
            continue

        if op == "constant":
            dst = allocate_value()
            values[node] = dst
            instructions.append(NeuralBytecodeInstruction(
                "LOAD_CONST",
                dst=dst,
                value=float(properties["nn.value"]),
                source_node=node,
            ))
            continue

        if op == "weighted_sum":
            terms: list[str] = []
            for edge in sorted(graph.incoming_edges(node), key=lambda item: item.id):
                source_value = values.get(edge.from_node)
                if source_value is None:
                    raise NeuralGraphCompileError(f"source node has no value: {edge.from_node}")
                weight_value = allocate_value()
                term_value = allocate_value()
                instructions.append(NeuralBytecodeInstruction(
                    "LOAD_EDGE_WEIGHT",
                    dst=weight_value,
                    edge_id=edge.id,
                    source_edge=edge.id,
                ))
                instructions.append(NeuralBytecodeInstruction(
                    "MUL",
                    dst=term_value,
                    left=source_value,
                    right=weight_value,
                    source_edge=edge.id,
                ))
                terms.append(term_value)
            dst = allocate_value()
            values[node] = dst
            instructions.append(NeuralBytecodeInstruction(
                "LOAD_CONST" if not terms else "ADD",
                dst=dst,
                value=0.0 if not terms else None,
                inputs=terms or None,
                source_node=node,
            ))
            continue

        if op == "activation":
            input_value = _single_input_value(graph, values, node)
            dst = allocate_value()
            values[node] = dst
            instructions.append(NeuralBytecodeInstruction(
                "ACTIVATE",
                dst=dst,
                input=input_value,
                activation=str(properties.get("nn.activation", "relu")),
                source_node=node,
            ))
            continue

        if op == "output":
            input_value = _single_input_value(graph, values, node)
            values[node] = input_value
            instructions.append(NeuralBytecodeInstruction(
                "STORE_OUTPUT",
                output_name=str(properties.get("nn.output", node)),
                input=input_value,
                source_node=node,
            ))
            continue

        raise NeuralGraphCompileError(f"unsupported neural graph op: {op}")

    return NeuralBytecodeModule(
        "CANN",
        0,
        graph.nodes(),
        [
            NeuralBytecodeGraphEdge(edge.id, edge.from_node, edge.to_node, edge.weight)
            for edge in graph.edges()
        ],
        [NeuralBytecodeFunction("forward", "forward", instructions)],
    )


def run_neural_bytecode_forward(module: NeuralBytecodeModule, inputs: dict[str, float]) -> dict[str, float]:
    forward = next((fn for fn in module.functions if fn.kind == "forward"), None)
    if forward is None:
        raise ValueError("neural bytecode module has no forward function")

    values: dict[str, float] = {}
    edge_weights = {edge.id: edge.weight for edge in module.edges}
    outputs: dict[str, float] = {}

    def read(value_id: str | None) -> float:
        if value_id is None or value_id not in values:
            raise ValueError(f"missing value: {value_id or '<undefined>'}")
        return values[value_id]

    for instruction in forward.instructions:
        if instruction.op == "LOAD_INPUT":
            input_name = instruction.input_name
            if input_name is None or input_name not in inputs:
                raise ValueError(f"missing input: {input_name or '<undefined>'}")
            values[_require_dst(instruction)] = inputs[input_name]
        elif instruction.op == "LOAD_CONST":
            values[_require_dst(instruction)] = instruction.value or 0.0
        elif instruction.op == "LOAD_EDGE_WEIGHT":
            values[_require_dst(instruction)] = edge_weights.get(instruction.edge_id or "", 1.0)
        elif instruction.op == "MUL":
            values[_require_dst(instruction)] = read(instruction.left) * read(instruction.right)
        elif instruction.op == "ADD":
            values[_require_dst(instruction)] = sum(read(value_id) for value_id in instruction.inputs or [])
        elif instruction.op == "ACTIVATE":
            values[_require_dst(instruction)] = apply_neural_activation(
                read(instruction.input),
                instruction.activation or "relu",
            )
        elif instruction.op == "STORE_OUTPUT":
            outputs[instruction.output_name or "output"] = read(instruction.input)
    return outputs


def apply_neural_activation(value: float, activation: str) -> float:
    if activation == "relu":
        return max(0.0, value)
    if activation == "sigmoid":
        clamped = max(-500.0, min(500.0, value))
        return 1.0 / (1.0 + exp(-clamped))
    if activation == "tanh":
        return tanh(value)
    if activation == "none":
        return value
    return value


def _single_input_value(graph: NeuralGraph, values: dict[str, str], node: str) -> str:
    incoming = sorted(graph.incoming_edges(node), key=lambda item: item.id)
    if len(incoming) != 1:
        raise NeuralGraphCompileError(f"expected exactly one input edge for {node}")
    value = values.get(incoming[0].from_node)
    if value is None:
        raise NeuralGraphCompileError(f"source node has no value: {incoming[0].from_node}")
    return value


def _require_dst(instruction: NeuralBytecodeInstruction) -> str:
    if instruction.dst is None:
        raise ValueError(f"instruction {instruction.op} is missing dst")
    return instruction.dst
