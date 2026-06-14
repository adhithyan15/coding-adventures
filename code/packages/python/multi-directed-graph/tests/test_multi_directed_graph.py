import pytest

from multi_directed_graph import (
    DuplicateEdgeIdError,
    EdgeNotFoundError,
    MultiDirectedGraph,
    MultiDirectedGraphCycleError,
    NodeNotFoundError,
)


def test_stores_parallel_directed_edges_with_stable_ids():
    graph = MultiDirectedGraph[str]()

    first = graph.add_edge("A", "B", 0.25, {"kind": "fast"}, "w0")
    second = graph.add_edge("A", "B", 0.75, {"kind": "slow"}, "w1")

    assert first == "w0"
    assert second == "w1"
    assert graph.nodes() == ["A", "B"]
    assert [edge.id for edge in graph.edges_between("A", "B")] == ["w0", "w1"]
    assert graph.edge_properties("w0") == {"kind": "fast", "weight": 0.25}


def test_auto_allocates_edge_ids_without_colliding():
    graph = MultiDirectedGraph[str]()

    graph.add_edge("A", "B", edge_id="e0")
    allocated = graph.add_edge("B", "C")

    assert allocated == "e1"


def test_rejects_duplicate_edge_ids():
    graph = MultiDirectedGraph[str]()
    graph.add_edge("A", "B", edge_id="w")

    with pytest.raises(DuplicateEdgeIdError):
        graph.add_edge("B", "C", edge_id="w")


def test_graph_node_and_edge_property_bags_are_copied():
    graph = MultiDirectedGraph[str]()

    graph.set_graph_property("name", "generic")
    graph.add_node("A", {"role": "input"})
    edge_id = graph.add_edge("A", "B", 2.0, {"trainable": True})

    assert graph.graph_properties() == {"name": "generic"}
    assert graph.node_properties("A") == {"role": "input"}
    assert graph.edge_properties(edge_id) == {"trainable": True, "weight": 2.0}

    graph.node_properties("A")["role"] = "mutated"
    graph.edge_properties(edge_id)["trainable"] = False

    assert graph.node_properties("A") == {"role": "input"}
    assert graph.edge_properties(edge_id) == {"trainable": True, "weight": 2.0}


def test_setting_weight_property_updates_edge_weight():
    graph = MultiDirectedGraph[str]()
    edge_id = graph.add_edge("A", "B", 1.0)

    graph.set_edge_property(edge_id, "weight", 3.5)

    assert graph.edge_weight(edge_id) == 3.5
    assert graph.edge_properties(edge_id)["weight"] == 3.5


def test_removing_weight_property_resets_weight():
    graph = MultiDirectedGraph[str]()
    edge_id = graph.add_edge("A", "B", 2.0)

    graph.remove_edge_property(edge_id, "weight")

    assert graph.edge_weight(edge_id) == 1.0
    assert graph.edge_properties(edge_id)["weight"] == 1.0


def test_successors_and_predecessors_deduplicate_parallel_edges():
    graph = MultiDirectedGraph[str]()
    graph.add_edge("A", "B", edge_id="w0")
    graph.add_edge("A", "B", edge_id="w1")
    graph.add_edge("C", "B", edge_id="w2")

    assert graph.successors("A") == ["B"]
    assert graph.predecessors("B") == ["A", "C"]


def test_topological_sort_and_independent_groups_are_stable():
    graph = MultiDirectedGraph[str]()
    graph.add_edge("A", "C")
    graph.add_edge("B", "C")
    graph.add_edge("C", "D")

    assert graph.topological_sort() == ["A", "B", "C", "D"]
    assert graph.independent_groups() == [["A", "B"], ["C"], ["D"]]


def test_cycle_detection_and_cycle_errors():
    graph = MultiDirectedGraph[str](allow_self_loops=True)
    graph.add_edge("A", "B")
    graph.add_edge("B", "A")

    assert graph.has_cycle()
    with pytest.raises(MultiDirectedGraphCycleError):
        graph.topological_sort()


def test_removes_nodes_and_incident_edges():
    graph = MultiDirectedGraph[str]()
    incoming = graph.add_edge("A", "B")
    outgoing = graph.add_edge("B", "C")

    graph.remove_node("B")

    assert not graph.has_node("B")
    assert not graph.has_edge(incoming)
    assert not graph.has_edge(outgoing)
    assert graph.nodes() == ["A", "C"]


def test_missing_node_and_edge_raise_specific_errors():
    graph = MultiDirectedGraph[str]()

    with pytest.raises(NodeNotFoundError):
        graph.incoming_edges("missing")
    with pytest.raises(EdgeNotFoundError):
        graph.edge("missing")
