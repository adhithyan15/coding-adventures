"""Graph-native neural network authoring primitives."""

from __future__ import annotations

from dataclasses import dataclass, field
from math import isfinite
from typing import Union

GraphPropertyValue = Union[str, int, float, bool, None]
GraphPropertyBag = dict[str, GraphPropertyValue]
ActivationKind = str


@dataclass(frozen=True)
class NeuralEdge:
    id: str
    from_node: str
    to_node: str
    weight: float = 1.0
    properties: GraphPropertyBag = field(default_factory=dict)


@dataclass(frozen=True)
class WeightedInput:
    from_node: str
    weight: float = 1.0
    edge_id: str | None = None
    properties: GraphPropertyBag = field(default_factory=dict)


class NeuralGraph:
    def __init__(self, name: str | None = None) -> None:
        self._graph_properties: GraphPropertyBag = {"nn.version": "0"}
        if name is not None:
            self._graph_properties["nn.name"] = name
        self._nodes: list[str] = []
        self._node_properties: dict[str, GraphPropertyBag] = {}
        self._edges: list[NeuralEdge] = []
        self._next_edge_id = 0

    def set_graph_property(self, key: str, value: GraphPropertyValue) -> None:
        self._graph_properties[key] = value

    def graph_properties(self) -> GraphPropertyBag:
        return dict(self._graph_properties)

    def add_node(self, node: str, properties: GraphPropertyBag | None = None) -> None:
        if node not in self._node_properties:
            self._nodes.append(node)
            self._node_properties[node] = {}
        if properties:
            self._node_properties[node].update(properties)

    def nodes(self) -> list[str]:
        return list(self._nodes)

    def node_properties(self, node: str) -> GraphPropertyBag:
        return dict(self._node_properties[node])

    def add_edge(
        self,
        from_node: str,
        to_node: str,
        weight: float = 1.0,
        properties: GraphPropertyBag | None = None,
        edge_id: str | None = None,
    ) -> str:
        self.add_node(from_node)
        self.add_node(to_node)
        if edge_id is None:
            edge_id = f"e{self._next_edge_id}"
            self._next_edge_id += 1
        merged = dict(properties or {})
        merged["weight"] = weight
        self._edges.append(NeuralEdge(edge_id, from_node, to_node, weight, merged))
        return edge_id

    def edges(self) -> list[NeuralEdge]:
        return list(self._edges)

    def incoming_edges(self, node: str) -> list[NeuralEdge]:
        return [edge for edge in self._edges if edge.to_node == node]

    def edge_properties(self, edge_id: str) -> GraphPropertyBag:
        for edge in self._edges:
            if edge.id == edge_id:
                return dict(edge.properties)
        raise KeyError(edge_id)

    def topological_sort(self) -> list[str]:
        indegree = {node: 0 for node in self._nodes}
        outgoing: dict[str, list[str]] = {node: [] for node in self._nodes}
        for edge in self._edges:
            indegree[edge.to_node] += 1
            outgoing[edge.from_node].append(edge.to_node)

        ready = sorted(node for node in self._nodes if indegree[node] == 0)
        order: list[str] = []
        while ready:
            node = ready.pop(0)
            order.append(node)
            for successor in outgoing[node]:
                indegree[successor] -= 1
                if indegree[successor] == 0:
                    ready.append(successor)
                    ready.sort()
        if len(order) != len(self._nodes):
            raise ValueError("neural graph contains a cycle")
        return order


class NeuralNetwork:
    def __init__(self, name: str | None = None, graph: NeuralGraph | None = None) -> None:
        self.graph = graph if graph is not None else create_neural_graph(name)

    def input(self, node: str, input_name: str | None = None, properties: GraphPropertyBag | None = None) -> "NeuralNetwork":
        add_input(self.graph, node, input_name or node, properties or {})
        return self

    def constant(self, node: str, value: float, properties: GraphPropertyBag | None = None) -> "NeuralNetwork":
        add_constant(self.graph, node, value, properties or {})
        return self

    def weighted_sum(self, node: str, inputs: list[WeightedInput], properties: GraphPropertyBag | None = None) -> "NeuralNetwork":
        add_weighted_sum(self.graph, node, inputs, properties or {})
        return self

    def activation(
        self,
        node: str,
        input_node: str,
        activation: ActivationKind,
        properties: GraphPropertyBag | None = None,
        edge_id: str | None = None,
    ) -> "NeuralNetwork":
        add_activation(self.graph, node, input_node, activation, properties or {}, edge_id)
        return self

    def output(
        self,
        node: str,
        input_node: str,
        output_name: str | None = None,
        properties: GraphPropertyBag | None = None,
        edge_id: str | None = None,
    ) -> "NeuralNetwork":
        add_output(self.graph, node, input_node, output_name or node, properties or {}, edge_id)
        return self


def create_neural_graph(name: str | None = None) -> NeuralGraph:
    return NeuralGraph(name)


def create_neural_network(name: str | None = None) -> NeuralNetwork:
    return NeuralNetwork(name)


def add_input(graph: NeuralGraph, node: str, input_name: str | None = None, properties: GraphPropertyBag | None = None) -> None:
    graph.add_node(node, {**(properties or {}), "nn.op": "input", "nn.input": input_name or node})


def add_constant(graph: NeuralGraph, node: str, value: float, properties: GraphPropertyBag | None = None) -> None:
    if not isfinite(value):
        raise ValueError("constant value must be finite")
    graph.add_node(node, {**(properties or {}), "nn.op": "constant", "nn.value": value})


def add_weighted_sum(
    graph: NeuralGraph,
    node: str,
    inputs: list[WeightedInput],
    properties: GraphPropertyBag | None = None,
) -> None:
    graph.add_node(node, {**(properties or {}), "nn.op": "weighted_sum"})
    for input_item in inputs:
        graph.add_edge(
            input_item.from_node,
            node,
            input_item.weight,
            input_item.properties,
            input_item.edge_id,
        )


def add_activation(
    graph: NeuralGraph,
    node: str,
    input_node: str,
    activation: ActivationKind,
    properties: GraphPropertyBag | None = None,
    edge_id: str | None = None,
) -> str:
    graph.add_node(node, {**(properties or {}), "nn.op": "activation", "nn.activation": activation})
    return graph.add_edge(input_node, node, 1.0, {}, edge_id)


def add_output(
    graph: NeuralGraph,
    node: str,
    input_node: str,
    output_name: str | None = None,
    properties: GraphPropertyBag | None = None,
    edge_id: str | None = None,
) -> str:
    graph.add_node(node, {**(properties or {}), "nn.op": "output", "nn.output": output_name or node})
    return graph.add_edge(input_node, node, 1.0, {}, edge_id)


def create_xor_network(name: str = "xor") -> NeuralNetwork:
    return (
        create_neural_network(name)
        .input("x0")
        .input("x1")
        .constant("bias", 1.0, {"nn.role": "bias"})
        .weighted_sum("h_or_sum", [
            WeightedInput("x0", 20.0, "x0_to_h_or"),
            WeightedInput("x1", 20.0, "x1_to_h_or"),
            WeightedInput("bias", -10.0, "bias_to_h_or"),
        ], {"nn.layer": "hidden", "nn.role": "weighted_sum"})
        .activation("h_or", "h_or_sum", "sigmoid", {"nn.layer": "hidden", "nn.role": "activation"}, "h_or_sum_to_h_or")
        .weighted_sum("h_nand_sum", [
            WeightedInput("x0", -20.0, "x0_to_h_nand"),
            WeightedInput("x1", -20.0, "x1_to_h_nand"),
            WeightedInput("bias", 30.0, "bias_to_h_nand"),
        ], {"nn.layer": "hidden", "nn.role": "weighted_sum"})
        .activation("h_nand", "h_nand_sum", "sigmoid", {"nn.layer": "hidden", "nn.role": "activation"}, "h_nand_sum_to_h_nand")
        .weighted_sum("out_sum", [
            WeightedInput("h_or", 20.0, "h_or_to_out"),
            WeightedInput("h_nand", 20.0, "h_nand_to_out"),
            WeightedInput("bias", -30.0, "bias_to_out"),
        ], {"nn.layer": "output", "nn.role": "weighted_sum"})
        .activation("out_activation", "out_sum", "sigmoid", {"nn.layer": "output", "nn.role": "activation"}, "out_sum_to_activation")
        .output("out", "out_activation", "prediction", {"nn.layer": "output"}, "activation_to_out")
    )
