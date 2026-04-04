// ============================================================================
// GraphTests.swift — Tests for the DirectedGraph package
// ============================================================================

import XCTest
@testable import DirectedGraph

final class GraphTests: XCTestCase {

    // MARK: - Node Operations

    func testAddNode() {
        var g = Graph()
        g.addNode("A")
        XCTAssertTrue(g.hasNode("A"))
        XCTAssertEqual(g.size, 1)
    }

    func testAddDuplicateNode() {
        var g = Graph()
        g.addNode("A")
        g.addNode("A")
        XCTAssertEqual(g.size, 1)
    }

    func testRemoveNode() throws {
        var g = Graph()
        g.addNode("A")
        g.addNode("B")
        try g.addEdge(from: "A", to: "B")
        try g.removeNode("B")
        XCTAssertFalse(g.hasNode("B"))
        XCTAssertEqual(g.size, 1)
        XCTAssertEqual(try g.successors(of: "A"), [])
    }

    func testRemoveNonexistentNodeThrows() {
        var g = Graph()
        XCTAssertThrowsError(try g.removeNode("Z")) { error in
            XCTAssertTrue(error is NodeNotFoundError)
        }
    }

    func testNodes() {
        var g = Graph()
        g.addNode("C")
        g.addNode("A")
        g.addNode("B")
        XCTAssertEqual(g.nodes(), ["A", "B", "C"])
    }

    // MARK: - Edge Operations

    func testAddEdge() throws {
        var g = Graph()
        try g.addEdge(from: "A", to: "B")
        XCTAssertTrue(g.hasEdge(from: "A", to: "B"))
        XCTAssertFalse(g.hasEdge(from: "B", to: "A"))
        XCTAssertTrue(g.hasNode("A"))
        XCTAssertTrue(g.hasNode("B"))
    }

    func testSelfLoopDisallowed() {
        var g = Graph(allowSelfLoops: false)
        XCTAssertThrowsError(try g.addEdge(from: "A", to: "A"))
    }

    func testSelfLoopAllowed() throws {
        var g = Graph(allowSelfLoops: true)
        try g.addEdge(from: "A", to: "A")
        XCTAssertTrue(g.hasEdge(from: "A", to: "A"))
    }

    func testRemoveEdge() throws {
        var g = Graph()
        try g.addEdge(from: "A", to: "B")
        try g.removeEdge(from: "A", to: "B")
        XCTAssertFalse(g.hasEdge(from: "A", to: "B"))
    }

    func testRemoveNonexistentEdgeThrows() {
        var g = Graph()
        g.addNode("A")
        g.addNode("B")
        XCTAssertThrowsError(try g.removeEdge(from: "A", to: "B")) { error in
            XCTAssertTrue(error is EdgeNotFoundError)
        }
    }

    func testEdges() throws {
        var g = Graph()
        try g.addEdge(from: "A", to: "B")
        try g.addEdge(from: "A", to: "C")
        let edges = g.edges()
        XCTAssertEqual(edges.count, 2)
        XCTAssertEqual(edges[0].0, "A")
        XCTAssertEqual(edges[0].1, "B")
        XCTAssertEqual(edges[1].0, "A")
        XCTAssertEqual(edges[1].1, "C")
    }

    // MARK: - Neighbors

    func testSuccessors() throws {
        var g = Graph()
        try g.addEdge(from: "A", to: "B")
        try g.addEdge(from: "A", to: "C")
        XCTAssertEqual(try g.successors(of: "A"), ["B", "C"])
    }

    func testPredecessors() throws {
        var g = Graph()
        try g.addEdge(from: "A", to: "C")
        try g.addEdge(from: "B", to: "C")
        XCTAssertEqual(try g.predecessors(of: "C"), ["A", "B"])
    }

    func testSuccessorsOfUnknownNodeThrows() {
        let g = Graph()
        XCTAssertThrowsError(try g.successors(of: "X")) { error in
            XCTAssertTrue(error is NodeNotFoundError)
        }
    }

    // MARK: - Topological Sort

    func testTopologicalSort() throws {
        var g = Graph()
        try g.addEdge(from: "A", to: "B")
        try g.addEdge(from: "B", to: "C")
        let sorted = try g.topologicalSort()
        XCTAssertEqual(sorted, ["A", "B", "C"])
    }

    func testTopologicalSortDiamond() throws {
        var g = Graph()
        try g.addEdge(from: "A", to: "B")
        try g.addEdge(from: "A", to: "C")
        try g.addEdge(from: "B", to: "D")
        try g.addEdge(from: "C", to: "D")
        let sorted = try g.topologicalSort()
        // A must come first, D must come last
        XCTAssertEqual(sorted.first, "A")
        XCTAssertEqual(sorted.last, "D")
    }

    func testTopologicalSortCycleThrows() throws {
        var g = Graph()
        try g.addEdge(from: "A", to: "B")
        try g.addEdge(from: "B", to: "C")
        try g.addEdge(from: "C", to: "A")
        XCTAssertThrowsError(try g.topologicalSort()) { error in
            XCTAssertTrue(error is CycleError)
        }
    }

    // MARK: - Cycle Detection

    func testHasCycleTrue() throws {
        var g = Graph()
        try g.addEdge(from: "A", to: "B")
        try g.addEdge(from: "B", to: "A")
        XCTAssertTrue(g.hasCycle())
    }

    func testHasCycleFalse() throws {
        var g = Graph()
        try g.addEdge(from: "A", to: "B")
        try g.addEdge(from: "B", to: "C")
        XCTAssertFalse(g.hasCycle())
    }

    // MARK: - Transitive Closure

    func testTransitiveClosure() throws {
        var g = Graph()
        try g.addEdge(from: "A", to: "B")
        try g.addEdge(from: "B", to: "C")
        try g.addEdge(from: "C", to: "D")
        let closure = try g.transitiveClosure(of: "A")
        XCTAssertEqual(closure, Set(["B", "C", "D"]))
    }

    func testTransitiveClosureNoSuccessors() throws {
        var g = Graph()
        g.addNode("A")
        let closure = try g.transitiveClosure(of: "A")
        XCTAssertEqual(closure, Set())
    }

    // MARK: - Transitive Dependents

    func testTransitiveDependents() throws {
        var g = Graph()
        try g.addEdge(from: "A", to: "B")
        try g.addEdge(from: "B", to: "C")
        let deps = try g.transitiveDependents(of: "C")
        XCTAssertEqual(deps, Set(["A", "B"]))
    }

    // MARK: - Independent Groups

    func testIndependentGroups() throws {
        var g = Graph()
        try g.addEdge(from: "A", to: "C")
        try g.addEdge(from: "B", to: "C")
        try g.addEdge(from: "C", to: "D")
        let groups = try g.independentGroups()
        XCTAssertEqual(groups.count, 3)
        XCTAssertEqual(groups[0], ["A", "B"])  // Independent, can run in parallel
        XCTAssertEqual(groups[1], ["C"])
        XCTAssertEqual(groups[2], ["D"])
    }

    // MARK: - Affected Nodes

    func testAffectedNodes() throws {
        var g = Graph()
        try g.addEdge(from: "A", to: "B")
        try g.addEdge(from: "B", to: "C")
        try g.addEdge(from: "D", to: "C")
        let affected = g.affectedNodes(changed: Set(["B"]))
        // B changed, A depends on B (transitively via reverse edges)
        XCTAssertTrue(affected.contains("B"))
        XCTAssertTrue(affected.contains("A"))
    }

    // MARK: - Empty Graph

    func testEmptyGraph() throws {
        let g = Graph()
        XCTAssertEqual(g.size, 0)
        XCTAssertEqual(g.nodes(), [])
        XCTAssertTrue(g.edges().isEmpty)
        XCTAssertEqual(try g.topologicalSort(), [])
        XCTAssertFalse(g.hasCycle())
    }
}
