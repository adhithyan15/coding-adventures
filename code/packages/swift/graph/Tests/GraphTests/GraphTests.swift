import XCTest
@testable import Graph

private func makeGraph(_ repr: GraphRepr) -> Graph {
    var graph = Graph(repr: repr)
    graph.addEdge("London", "Paris", weight: 300.0)
    graph.addEdge("London", "Amsterdam", weight: 520.0)
    graph.addEdge("Paris", "Berlin", weight: 878.0)
    graph.addEdge("Amsterdam", "Berlin", weight: 655.0)
    graph.addEdge("Amsterdam", "Brussels", weight: 180.0)
    return graph
}

final class GraphTests: XCTestCase {
    func testNodeAndEdgeOperations() throws {
        for repr in GraphRepr.allCases {
            var graph = Graph(repr: repr)
            graph.addNode("A")
            graph.addEdge("A", "B", weight: 2.5)

            XCTAssertTrue(graph.hasNode("A"))
            XCTAssertTrue(graph.hasEdge("A", "B"))
            XCTAssertTrue(graph.hasEdge("B", "A"))
            XCTAssertEqual(graph.nodes(), ["A", "B"])
            XCTAssertEqual(graph.edges().count, 1)
            XCTAssertEqual(try graph.edgeWeight("A", "B"), 2.5)
            XCTAssertEqual(try graph.degree(of: "A"), 1)
        }
    }

    func testWeightedNeighborsTraversalAndConnectivity() throws {
        for repr in GraphRepr.allCases {
            let graph = makeGraph(repr)
            XCTAssertEqual(try graph.neighbors(of: "Amsterdam"), ["Berlin", "Brussels", "London"])
            XCTAssertEqual(try graph.neighborsWeighted(of: "Amsterdam"), ["Berlin": 655.0, "Brussels": 180.0, "London": 520.0])
            XCTAssertEqual(try bfs(graph, start: "London"), ["London", "Amsterdam", "Paris", "Berlin", "Brussels"])
            XCTAssertEqual(try dfs(graph, start: "London"), ["London", "Amsterdam", "Berlin", "Paris", "Brussels"])
            XCTAssertTrue(isConnected(graph))
            XCTAssertTrue(hasCycle(graph))
        }
    }

    func testShortestPathComponentsAndMst() throws {
        for repr in GraphRepr.allCases {
            let graph = makeGraph(repr)
            XCTAssertEqual(shortestPath(graph, start: "London", finish: "Berlin"), ["London", "Amsterdam", "Berlin"])
            let mst = try minimumSpanningTree(graph)
            XCTAssertEqual(mst.count, graph.size - 1)
            XCTAssertEqual(mst.reduce(0.0) { $0 + $1.2 }, 1655.0, accuracy: 1e-9)

            var other = Graph(repr: repr)
            other.addEdge("A", "B")
            other.addEdge("B", "C")
            other.addEdge("D", "E")
            let components = connectedComponents(other)
            XCTAssertTrue(components.contains { $0 == ["A", "B", "C"] })
            XCTAssertTrue(components.contains { $0 == ["D", "E"] })
        }
    }

    func testDisconnectedGraphThrowsForMst() {
        for repr in GraphRepr.allCases {
            var graph = Graph(repr: repr)
            graph.addEdge("A", "B")
            graph.addNode("C")

            XCTAssertThrowsError(try minimumSpanningTree(graph)) { error in
                XCTAssertTrue(error is NotConnectedError)
            }
        }
    }

    func testPropertyBags() throws {
        for repr in GraphRepr.allCases {
            var graph = Graph(repr: repr)

            graph.setGraphProperty("name", value: .string("city-map"))
            graph.setGraphProperty("version", value: .number(1.0))
            XCTAssertEqual(graph.graphProperties(), ["name": .string("city-map"), "version": .number(1.0)])
            graph.removeGraphProperty("version")
            XCTAssertEqual(graph.graphProperties(), ["name": .string("city-map")])

            graph.addNode("A", properties: ["kind": .string("input")])
            graph.addNode("A", properties: ["trainable": .bool(false)])
            try graph.setNodeProperty("A", "slot", value: .number(0.0))
            XCTAssertEqual(
                try graph.nodeProperties("A"),
                ["kind": .string("input"), "trainable": .bool(false), "slot": .number(0.0)]
            )
            try graph.removeNodeProperty("A", "slot")
            XCTAssertEqual(try graph.nodeProperties("A"), ["kind": .string("input"), "trainable": .bool(false)])

            graph.addEdge("A", "B", weight: 2.5, properties: ["role": .string("distance")])
            XCTAssertEqual(try graph.edgeProperties("B", "A"), ["role": .string("distance"), "weight": .number(2.5)])
            try graph.setEdgeProperty("B", "A", "weight", value: .number(7.0))
            XCTAssertEqual(try graph.edgeWeight("A", "B"), 7.0)
            try graph.setEdgeProperty("A", "B", "trainable", value: .bool(true))
            try graph.removeEdgeProperty("A", "B", "role")
            XCTAssertEqual(try graph.edgeProperties("A", "B"), ["weight": .number(7.0), "trainable": .bool(true)])

            try graph.removeEdge("A", "B")
            XCTAssertThrowsError(try graph.edgeProperties("A", "B")) { error in
                XCTAssertTrue(error is EdgeNotFoundError)
            }
        }
    }
}
