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
}
