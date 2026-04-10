from __future__ import annotations

import pytest

from graph_native import (
    EdgeNotFoundError,
    Graph,
    GraphRepr,
    NodeNotFoundError,
    bfs,
    connected_components,
    dfs,
    has_cycle,
    is_connected,
    minimum_spanning_tree,
    shortest_path,
)


def make_graph(repr: GraphRepr) -> Graph:
    graph = Graph(repr=repr)
    graph.add_edge("London", "Paris", 300.0)
    graph.add_edge("London", "Amsterdam", 520.0)
    graph.add_edge("Paris", "Berlin", 878.0)
    graph.add_edge("Amsterdam", "Berlin", 655.0)
    graph.add_edge("Amsterdam", "Brussels", 180.0)
    return graph


@pytest.fixture(params=[GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX])
def repr_kind(request: pytest.FixtureRequest) -> GraphRepr:
    return request.param


def test_node_and_edge_operations(repr_kind: GraphRepr) -> None:
    graph = Graph(repr=repr_kind)
    graph.add_node("A")
    graph.add_edge("A", "B", 2.5)

    assert graph.has_node("A")
    assert graph.has_node("B")
    assert graph.has_edge("A", "B")
    assert graph.has_edge("B", "A")
    assert graph.edge_weight("A", "B") == pytest.approx(2.5)
    assert graph.nodes() == frozenset({"A", "B"})
    assert graph.edges() == frozenset({("A", "B", 2.5)})
    assert graph.degree("A") == 1


def test_remove_edge_and_node_errors(repr_kind: GraphRepr) -> None:
    graph = Graph(repr=repr_kind)
    graph.add_edge("A", "B")
    graph.remove_edge("A", "B")
    graph.remove_node("B")

    with pytest.raises(EdgeNotFoundError):
        graph.remove_edge("A", "B")
    with pytest.raises(NodeNotFoundError):
        graph.neighbors("B")


def test_neighbors_weighted_and_repr(repr_kind: GraphRepr) -> None:
    graph = make_graph(repr_kind)
    assert graph.neighbors("Amsterdam") == frozenset({"London", "Berlin", "Brussels"})
    assert graph.neighbors_weighted("Amsterdam") == {
        "Berlin": 655.0,
        "Brussels": 180.0,
        "London": 520.0,
    }
    assert "Graph(" in repr(graph)


def test_traversals_and_connectivity(repr_kind: GraphRepr) -> None:
    graph = make_graph(repr_kind)
    assert bfs(graph, "London") == [
        "London",
        "Amsterdam",
        "Paris",
        "Berlin",
        "Brussels",
    ]
    assert dfs(graph, "London")[0] == "London"
    assert is_connected(graph)


def test_components_cycle_shortest_path_and_mst(repr_kind: GraphRepr) -> None:
    graph = make_graph(repr_kind)
    other = Graph(repr=repr_kind)
    other.add_edge("A", "B")
    other.add_edge("B", "C")
    other.add_edge("C", "A")
    other.add_edge("D", "E")

    components = connected_components(other)
    assert frozenset({"A", "B", "C"}) in components
    assert frozenset({"D", "E"}) in components
    assert has_cycle(other)

    assert shortest_path(graph, "London", "Berlin") == [
        "London",
        "Amsterdam",
        "Berlin",
    ]

    mst = minimum_spanning_tree(graph)
    assert len(mst) == len(graph) - 1
    assert sum(weight for _, _, weight in mst) == pytest.approx(1655.0)


def test_disconnected_mst_raises(repr_kind: GraphRepr) -> None:
    graph = Graph(repr=repr_kind)
    graph.add_edge("A", "B")
    graph.add_node("C")

    with pytest.raises(ValueError, match="not connected"):
        minimum_spanning_tree(graph)


def test_instance_algorithm_methods_and_dunders() -> None:
    graph = Graph()
    graph.add_edge("A", "B")
    graph.add_edge("B", "C")

    assert len(graph) == 3
    assert "A" in graph
    assert 7 not in graph
    assert graph.bfs("A") == ["A", "B", "C"]
    assert graph.dfs("A") == ["A", "B", "C"]
    assert graph.is_connected()
    assert graph.connected_components() == [frozenset({"A", "B", "C"})]
    assert not graph.has_cycle()
    assert graph.shortest_path("A", "C") == ["A", "B", "C"]
    assert graph.minimum_spanning_tree() == frozenset(
        {("A", "B", 1.0), ("B", "C", 1.0)}
    )
