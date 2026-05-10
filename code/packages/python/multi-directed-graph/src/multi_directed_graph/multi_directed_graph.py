from __future__ import annotations

from dataclasses import dataclass
from math import isnan
from typing import Generic, TypeAlias, TypeVar, Union

T = TypeVar("T")
GraphPropertyValue: TypeAlias = Union[str, int, float, bool, None]
GraphPropertyBag: TypeAlias = dict[str, GraphPropertyValue]


@dataclass(frozen=True)
class MultiDirectedEdge(Generic[T]):
    id: str
    from_node: T
    to_node: T
    weight: float = 1.0


class NodeNotFoundError(KeyError):
    def __init__(self, node: object) -> None:
        super().__init__(node)
        self.node = node


class EdgeNotFoundError(KeyError):
    def __init__(self, edge_id: str) -> None:
        super().__init__(edge_id)
        self.edge_id = edge_id


class DuplicateEdgeIdError(ValueError):
    def __init__(self, edge_id: str) -> None:
        super().__init__(f"edge ID already exists: {edge_id}")
        self.edge_id = edge_id


class MultiDirectedGraphCycleError(ValueError):
    pass


def _node_sort_key(node: object) -> str:
    return f"{type(node).__name__}:{node!r}"


class MultiDirectedGraph(Generic[T]):
    def __init__(self, allow_self_loops: bool = False) -> None:
        self.allow_self_loops = allow_self_loops
        self._nodes: list[T] = []
        self._node_set: set[T] = set()
        self._edges: dict[str, MultiDirectedEdge[T]] = {}
        self._outgoing: dict[T, list[str]] = {}
        self._incoming: dict[T, list[str]] = {}
        self._graph_properties: GraphPropertyBag = {}
        self._node_properties: dict[T, GraphPropertyBag] = {}
        self._edge_properties: dict[str, GraphPropertyBag] = {}
        self._next_edge_id = 0

    def __len__(self) -> int:
        return len(self._nodes)

    @property
    def size(self) -> int:
        return len(self._nodes)

    def add_node(self, node: T, properties: GraphPropertyBag | None = None) -> None:
        if node not in self._node_set:
            self._node_set.add(node)
            self._nodes.append(node)
            self._outgoing[node] = []
            self._incoming[node] = []
            self._node_properties[node] = {}
        if properties:
            self._node_properties[node].update(properties)

    def remove_node(self, node: T) -> None:
        self._assert_node(node)
        edge_ids = set(self._outgoing[node]) | set(self._incoming[node])
        for edge_id in list(edge_ids):
            self.remove_edge(edge_id)
        self._node_set.remove(node)
        self._nodes.remove(node)
        del self._outgoing[node]
        del self._incoming[node]
        del self._node_properties[node]

    def has_node(self, node: T) -> bool:
        return node in self._node_set

    def nodes(self) -> list[T]:
        return list(self._nodes)

    def add_edge(
        self,
        from_node: T,
        to_node: T,
        weight: float = 1.0,
        properties: GraphPropertyBag | None = None,
        edge_id: str | None = None,
    ) -> str:
        if from_node == to_node and not self.allow_self_loops:
            raise ValueError(f"self-loops are not allowed: {from_node!r} -> {to_node!r}")
        weight = self._validate_weight(weight)
        edge_id = edge_id if edge_id is not None else self._allocate_edge_id()
        if edge_id in self._edges:
            raise DuplicateEdgeIdError(edge_id)

        self.add_node(from_node)
        self.add_node(to_node)

        self._edges[edge_id] = MultiDirectedEdge(edge_id, from_node, to_node, weight)
        self._outgoing[from_node].append(edge_id)
        self._incoming[to_node].append(edge_id)
        merged = dict(properties or {})
        merged["weight"] = weight
        self._edge_properties[edge_id] = merged
        return edge_id

    def remove_edge(self, edge_id: str) -> None:
        edge = self.edge(edge_id)
        self._outgoing[edge.from_node].remove(edge_id)
        self._incoming[edge.to_node].remove(edge_id)
        del self._edges[edge_id]
        del self._edge_properties[edge_id]

    def has_edge(self, edge_id: str) -> bool:
        return edge_id in self._edges

    def edge(self, edge_id: str) -> MultiDirectedEdge[T]:
        try:
            return self._edges[edge_id]
        except KeyError as error:
            raise EdgeNotFoundError(edge_id) from error

    def edges(self) -> list[MultiDirectedEdge[T]]:
        return list(self._edges.values())

    def edges_between(self, from_node: T, to_node: T) -> list[MultiDirectedEdge[T]]:
        self._assert_node(from_node)
        self._assert_node(to_node)
        return [edge for edge in self.outgoing_edges(from_node) if edge.to_node == to_node]

    def outgoing_edges(self, node: T) -> list[MultiDirectedEdge[T]]:
        self._assert_node(node)
        return [self.edge(edge_id) for edge_id in self._outgoing[node]]

    def incoming_edges(self, node: T) -> list[MultiDirectedEdge[T]]:
        self._assert_node(node)
        return [self.edge(edge_id) for edge_id in self._incoming[node]]

    def successors(self, node: T) -> list[T]:
        seen: set[T] = set()
        result: list[T] = []
        for edge in self.outgoing_edges(node):
            if edge.to_node not in seen:
                seen.add(edge.to_node)
                result.append(edge.to_node)
        return result

    def predecessors(self, node: T) -> list[T]:
        seen: set[T] = set()
        result: list[T] = []
        for edge in self.incoming_edges(node):
            if edge.from_node not in seen:
                seen.add(edge.from_node)
                result.append(edge.from_node)
        return result

    def edge_weight(self, edge_id: str) -> float:
        return self.edge(edge_id).weight

    def graph_properties(self) -> GraphPropertyBag:
        return dict(self._graph_properties)

    def set_graph_property(self, key: str, value: GraphPropertyValue) -> None:
        self._graph_properties[key] = value

    def remove_graph_property(self, key: str) -> None:
        self._graph_properties.pop(key, None)

    def node_properties(self, node: T) -> GraphPropertyBag:
        self._assert_node(node)
        return dict(self._node_properties[node])

    def set_node_property(self, node: T, key: str, value: GraphPropertyValue) -> None:
        self._assert_node(node)
        self._node_properties[node][key] = value

    def remove_node_property(self, node: T, key: str) -> None:
        self._assert_node(node)
        self._node_properties[node].pop(key, None)

    def edge_properties(self, edge_id: str) -> GraphPropertyBag:
        self._assert_edge(edge_id)
        properties = dict(self._edge_properties[edge_id])
        properties["weight"] = self.edge_weight(edge_id)
        return properties

    def set_edge_property(
        self,
        edge_id: str,
        key: str,
        value: GraphPropertyValue,
    ) -> None:
        self._assert_edge(edge_id)
        if key == "weight":
            if not isinstance(value, (int, float)) or isinstance(value, bool):
                raise ValueError("edge property 'weight' must be numeric")
            self._set_edge_weight(edge_id, float(value))
        self._edge_properties[edge_id][key] = value

    def remove_edge_property(self, edge_id: str, key: str) -> None:
        self._assert_edge(edge_id)
        if key == "weight":
            self._set_edge_weight(edge_id, 1.0)
            self._edge_properties[edge_id]["weight"] = 1.0
            return
        self._edge_properties[edge_id].pop(key, None)

    def topological_sort(self) -> list[T]:
        indegree = {node: len(self._incoming[node]) for node in self._nodes}
        ready = sorted((node for node in self._nodes if indegree[node] == 0), key=_node_sort_key)
        order: list[T] = []

        while ready:
            node = ready.pop(0)
            order.append(node)
            for edge in self.outgoing_edges(node):
                indegree[edge.to_node] -= 1
                if indegree[edge.to_node] == 0:
                    ready.append(edge.to_node)
                    ready.sort(key=_node_sort_key)

        if len(order) != len(self._nodes):
            raise MultiDirectedGraphCycleError(
                f"graph contains a cycle: processed {len(order)}/{len(self._nodes)} nodes"
            )
        return order

    def has_cycle(self) -> bool:
        try:
            self.topological_sort()
            return False
        except MultiDirectedGraphCycleError:
            return True

    def independent_groups(self) -> list[list[T]]:
        indegree = {node: len(self._incoming[node]) for node in self._nodes}
        current = sorted((node for node in self._nodes if indegree[node] == 0), key=_node_sort_key)
        groups: list[list[T]] = []
        processed = 0

        while current:
            groups.append(current)
            processed += len(current)
            next_nodes: set[T] = set()
            for node in current:
                for edge in self.outgoing_edges(node):
                    indegree[edge.to_node] -= 1
                    if indegree[edge.to_node] == 0:
                        next_nodes.add(edge.to_node)
            current = sorted(next_nodes, key=_node_sort_key)

        if processed != len(self._nodes):
            raise MultiDirectedGraphCycleError(
                f"graph contains a cycle: processed {processed}/{len(self._nodes)} nodes"
            )
        return groups

    def __repr__(self) -> str:
        return f"MultiDirectedGraph(nodes={len(self._nodes)}, edges={len(self._edges)})"

    def _allocate_edge_id(self) -> str:
        edge_id = f"e{self._next_edge_id}"
        while edge_id in self._edges:
            self._next_edge_id += 1
            edge_id = f"e{self._next_edge_id}"
        self._next_edge_id += 1
        return edge_id

    def _assert_node(self, node: T) -> None:
        if node not in self._node_set:
            raise NodeNotFoundError(node)

    def _assert_edge(self, edge_id: str) -> None:
        if edge_id not in self._edges:
            raise EdgeNotFoundError(edge_id)

    def _validate_weight(self, weight: float) -> float:
        if not isinstance(weight, (int, float)) or isinstance(weight, bool):
            raise ValueError("edge weight must be numeric")
        numeric_weight = float(weight)
        if isnan(numeric_weight):
            raise ValueError("edge weight must not be NaN")
        return numeric_weight

    def _set_edge_weight(self, edge_id: str, weight: float) -> None:
        weight = self._validate_weight(weight)
        edge = self.edge(edge_id)
        self._edges[edge_id] = MultiDirectedEdge(
            edge.id,
            edge.from_node,
            edge.to_node,
            weight,
        )
