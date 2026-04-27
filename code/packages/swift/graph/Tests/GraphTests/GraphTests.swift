import XCTest
@testable import Graph

final class GraphTests: XCTestCase {

    // MARK: - Test Construction

    func testEmptyGraphAdjacencyList() {
        var g: Graph<String> = Graph(repr: .adjacencyList)
        XCTAssertEqual(g.count, 0)
        XCTAssertEqual(g.nodes.count, 0)
        XCTAssertEqual(g.edges.count, 0)
    }

    func testEmptyGraphAdjacencyMatrix() {
        var g: Graph<String> = Graph(repr: .adjacencyMatrix)
        XCTAssertEqual(g.count, 0)
        XCTAssertEqual(g.nodes.count, 0)
        XCTAssertEqual(g.edges.count, 0)
    }

    func testDefaultRepresentationIsAdjacencyList() {
        var g: Graph<String> = Graph()
        g.addNode("A")
        XCTAssertEqual(g.count, 1)
    }

    // MARK: - Test Node Operations

    func testAddNodeList() {
        var g: Graph<String> = Graph(repr: .adjacencyList)
        g.addNode("A")
        XCTAssertTrue(g.hasNode("A"))
        XCTAssertEqual(g.count, 1)
        XCTAssertTrue(g.nodes.contains("A"))
    }

    func testAddNodeMatrix() {
        var g: Graph<String> = Graph(repr: .adjacencyMatrix)
        g.addNode("X")
        XCTAssertTrue(g.hasNode("X"))
        XCTAssertEqual(g.count, 1)
    }

    func testAddMultipleNodes() {
        var g: Graph<String> = Graph(repr: .adjacencyList)
        g.addNode("A")
        g.addNode("B")
        g.addNode("C")
        XCTAssertEqual(g.count, 3)
        XCTAssertTrue(g.nodes.contains("A"))
        XCTAssertTrue(g.nodes.contains("B"))
        XCTAssertTrue(g.nodes.contains("C"))
    }

    func testAddDuplicateNode() {
        var g: Graph<String> = Graph()
        g.addNode("A")
        g.addNode("A")  // Should be no-op
        XCTAssertEqual(g.count, 1)
    }

    func testRemoveNodeList() {
        var g: Graph<String> = Graph(repr: .adjacencyList)
        g.addNode("A")
        g.addNode("B")
        g.addEdge("A", "B", weight: 1.0)

        try? g.removeNode("A")
        XCTAssertFalse(g.hasNode("A"))
        XCTAssertEqual(g.count, 1)
        XCTAssertFalse(g.hasEdge("A", "B"))
    }

    func testRemoveNodeNotFound() {
        var g: Graph<String> = Graph()
        XCTAssertThrowsError(try g.removeNode("NONEXISTENT")) { error in
            XCTAssertEqual(error as? GraphError, GraphError.nodeNotFound("NONEXISTENT"))
        }
    }

    // MARK: - Test Edge Operations

    func testAddEdgeList() {
        var g: Graph<String> = Graph(repr: .adjacencyList)
        g.addEdge("A", "B", weight: 1.5)
        XCTAssertTrue(g.hasNode("A"))
        XCTAssertTrue(g.hasNode("B"))
        XCTAssertTrue(g.hasEdge("A", "B"))
        XCTAssertTrue(g.hasEdge("B", "A"))  // Undirected
    }

    func testEdgeWeight() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 2.5)
        let w1 = try? g.edgeWeight("A", "B")
        let w2 = try? g.edgeWeight("B", "A")
        XCTAssertEqual(w1, 2.5)
        XCTAssertEqual(w2, 2.5)  // Symmetric
    }

    func testDefaultEdgeWeight() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B")  // Default weight 1.0
        let w = try? g.edgeWeight("A", "B")
        XCTAssertEqual(w, 1.0)
    }

    func testRemoveEdge() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addEdge("B", "C", weight: 1.0)

        try? g.removeEdge("A", "B")
        XCTAssertFalse(g.hasEdge("A", "B"))
        XCTAssertFalse(g.hasEdge("B", "A"))
        XCTAssertTrue(g.hasEdge("B", "C"))
    }

    func testRemoveEdgeNotFound() {
        var g: Graph<String> = Graph()
        g.addNode("A")
        XCTAssertThrowsError(try g.removeEdge("A", "Z"))
    }

    func testEdgesReturnsEachEdgeOnce() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addEdge("B", "C", weight: 2.0)

        let edges = g.edges
        XCTAssertEqual(edges.count, 2)
    }

    func testUpdateEdgeWeight() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addEdge("A", "B", weight: 3.0)  // Update
        let w = try? g.edgeWeight("A", "B")
        XCTAssertEqual(w, 3.0)
    }

    // MARK: - Test Neighbourhood Queries

    func testNeighbors() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addEdge("A", "C", weight: 2.0)

        let neighbors = (try? g.neighbors("A")) ?? []
        XCTAssertEqual(Set(neighbors), Set(["B", "C"]))
    }

    func testDegree() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addEdge("A", "C", weight: 2.0)
        g.addEdge("B", "C", weight: 3.0)

        let degA = (try? g.degree("A")) ?? -1
        let degB = (try? g.degree("B")) ?? -1
        let degC = (try? g.degree("C")) ?? -1

        XCTAssertEqual(degA, 2)
        XCTAssertEqual(degB, 2)
        XCTAssertEqual(degC, 2)
    }

    func testNeighborsNotFound() {
        var g: Graph<String> = Graph()
        XCTAssertThrowsError(try g.neighbors("NONEXISTENT"))
    }

    func testNeighborsWeighted() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.5)
        g.addEdge("A", "C", weight: 2.5)

        let neighbors = (try? g.neighborsWeighted("A")) ?? [:]
        XCTAssertEqual(neighbors["B"], 1.5)
        XCTAssertEqual(neighbors["C"], 2.5)
    }

    // MARK: - Test BFS

    func testBFS() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addEdge("B", "C", weight: 1.0)
        g.addEdge("A", "D", weight: 1.0)

        let result = g.bfs("A")
        XCTAssertEqual(result.count, 4)
        XCTAssertEqual(result[0], "A")
    }

    // MARK: - Test DFS

    func testDFS() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addEdge("B", "C", weight: 1.0)
        g.addEdge("A", "D", weight: 1.0)

        let result = g.dfs("A")
        XCTAssertEqual(result.count, 4)
        XCTAssertEqual(result[0], "A")
    }

    // MARK: - Test Connectivity

    func testIsConnectedEmpty() {
        let g: Graph<String> = Graph()
        XCTAssertTrue(g.isConnected)
    }

    func testIsConnectedSingleNode() {
        var g: Graph<String> = Graph()
        g.addNode("A")
        XCTAssertTrue(g.isConnected)
    }

    func testIsConnectedDisconnected() {
        var g: Graph<String> = Graph()
        g.addNode("A")
        g.addNode("B")
        XCTAssertFalse(g.isConnected)
    }

    func testIsConnectedConnected() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        XCTAssertTrue(g.isConnected)
    }

    // MARK: - Test Cycles

    func testHasCycleTree() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addEdge("B", "C", weight: 1.0)
        XCTAssertFalse(g.hasCycle())
    }

    func testHasCycleWithCycle() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addEdge("B", "C", weight: 1.0)
        g.addEdge("C", "A", weight: 1.0)  // Triangle
        XCTAssertTrue(g.hasCycle())
    }

    // MARK: - Test Shortest Path

    func testShortestPathUnweighted() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addEdge("B", "C", weight: 1.0)
        g.addEdge("A", "C", weight: 10.0)

        let path = g.shortestPath("A", "C")
        XCTAssertEqual(path, ["A", "B", "C"])
    }

    func testShortestPathNoPath() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addNode("C")

        let path = g.shortestPath("A", "C")
        XCTAssertEqual(path.count, 0)
    }

    func testShortestPathSameNode() {
        var g: Graph<String> = Graph()
        g.addNode("A")

        let path = g.shortestPath("A", "A")
        XCTAssertEqual(path, ["A"])
    }

    // MARK: - Test Bipartite

    func testIsBipartitePath() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addEdge("B", "C", weight: 1.0)
        XCTAssertTrue(g.isBipartite())
    }

    func testIsBipartiteTriangle() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addEdge("B", "C", weight: 1.0)
        g.addEdge("C", "A", weight: 1.0)
        XCTAssertFalse(g.isBipartite())
    }

    func testIsBipartiteEmpty() {
        let g: Graph<String> = Graph()
        XCTAssertTrue(g.isBipartite())
    }

    // MARK: - Test Minimum Spanning Tree

    func testMinimumSpanningTree() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addEdge("B", "C", weight: 2.0)
        g.addEdge("A", "C", weight: 3.0)

        let mst = (try? g.minimumSpanningTree()) ?? []
        XCTAssertEqual(mst.count, 2)  // V-1 edges

        let totalWeight = mst.reduce(0) { $0 + $1.2 }
        XCTAssertEqual(totalWeight, 3.0)  // 1.0 + 2.0
    }

    func testMinimumSpanningTreeDisconnected() {
        var g: Graph<String> = Graph()
        g.addEdge("A", "B", weight: 1.0)
        g.addNode("C")

        XCTAssertThrowsError(try g.minimumSpanningTree()) { error in
            XCTAssertEqual(error as? GraphError, GraphError.graphNotConnected)
        }
    }

    // MARK: - Test Both Representations

    func testBothRepresentations() {
        var gList: Graph<String> = Graph(repr: .adjacencyList)
        var gMatrix: Graph<String> = Graph(repr: .adjacencyMatrix)

        // Add same edges to both
        gList.addEdge("A", "B", weight: 1.0)
        gList.addEdge("B", "C", weight: 2.0)

        gMatrix.addEdge("A", "B", weight: 1.0)
        gMatrix.addEdge("B", "C", weight: 2.0)

        // Test both produce same results
        XCTAssertEqual(gList.edges.count, 2)
        XCTAssertEqual(gMatrix.edges.count, 2)

        XCTAssertTrue(gList.hasEdge("A", "B"))
        XCTAssertTrue(gMatrix.hasEdge("A", "B"))

        let neighborsL = (try? gList.neighbors("B")) ?? []
        let neighborsM = (try? gMatrix.neighbors("B")) ?? []
        XCTAssertEqual(Set(neighborsL), Set(neighborsM))

        XCTAssertEqual(gList.bfs("A").count, gMatrix.bfs("A").count)
    }

    // MARK: - Test with Integer Nodes

    func testGraphWithIntegerNodes() {
        var g: Graph<Int> = Graph()
        g.addEdge(1, 2, weight: 1.0)
        g.addEdge(2, 3, weight: 1.0)

        XCTAssertTrue(g.hasEdge(1, 2))
        XCTAssertEqual((try? g.neighbors(2))?.count, 2)
    }

    // MARK: - Edge Cases

    func testEmptyGraphBFS() {
        let g: Graph<String> = Graph()
        let result = g.bfs("A")
        XCTAssertEqual(result.count, 0)
    }

    func testEmptyGraphDFS() {
        let g: Graph<String> = Graph()
        let result = g.dfs("A")
        XCTAssertEqual(result.count, 0)
    }

    func testSingleNodeGraph() {
        var g: Graph<String> = Graph()
        g.addNode("A")

        XCTAssertTrue(g.isConnected)
        XCTAssertFalse(g.hasCycle())
        XCTAssertTrue(g.isBipartite())
    }
}
