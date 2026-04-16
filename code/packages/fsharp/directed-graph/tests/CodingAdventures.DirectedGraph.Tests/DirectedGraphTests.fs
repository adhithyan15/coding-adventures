namespace CodingAdventures.DirectedGraph.FSharp.Tests

open CodingAdventures.DirectedGraph.FSharp
open Xunit

module DirectedGraphTests =
    [<Fact>]
    let ``topological sort works`` () =
        let graph =
            DirectedGraph.createDefault()
            |> DirectedGraph.addEdge "A" "B"
            |> DirectedGraph.addEdge "B" "C"

        Assert.Equal<string list>(["A"; "B"; "C"], DirectedGraph.topologicalSort graph)

    [<Fact>]
    let ``graph exposes successors predecessors and affected nodes`` () =
        let graph = Graph()
        graph.AddEdge("logic-gates", "arithmetic")
        graph.AddEdge("arithmetic", "cpu")
        graph.AddEdge("logic-gates", "alu")

        Assert.Equal<string list>(["alu"; "arithmetic"], graph.Successors("logic-gates"))
        Assert.Equal<string list>(["logic-gates"], graph.Predecessors("arithmetic"))
        Assert.Equal<string list>(["logic-gates"; "alu"; "arithmetic"; "cpu"], graph.AffectedNodes([ "logic-gates" ]))

    [<Fact>]
    let ``cycles surface a helpful error`` () =
        let graph = Graph()
        graph.AddEdge("A", "B")
        graph.AddEdge("B", "C")
        graph.AddEdge("C", "A")

        let error = Assert.Throws<CycleError>(fun () -> graph.TopologicalSort() |> ignore)
        Assert.Contains("A", error.Cycle)
