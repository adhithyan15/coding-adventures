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
