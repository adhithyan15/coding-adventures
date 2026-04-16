namespace CodingAdventures.DirectedGraph.FSharp

open CodingAdventures.DirectedGraph

module DirectedGraph =
    let create allowSelfLoops =
        Graph(allowSelfLoops)

    let createDefault () =
        Graph()

    let addEdge fromNode toNode (graph: Graph) =
        graph.AddEdge(fromNode, toNode)
        graph

    let topologicalSort (graph: Graph) =
        graph.TopologicalSort() |> Seq.toList
