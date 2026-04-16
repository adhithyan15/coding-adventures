namespace CodingAdventures.StateMachine.FSharp.Tests

open System.Collections.Generic
open CodingAdventures.StateMachine.FSharp
open Xunit

module StateMachineTests =
    [<Fact>]
    let ``dfa wrapper works`` () =
        let transitions = Dictionary<struct (string * string), string>()
        transitions.Add(struct ("locked", "coin"), "unlocked")
        transitions.Add(struct ("locked", "push"), "locked")
        transitions.Add(struct ("unlocked", "coin"), "unlocked")
        transitions.Add(struct ("unlocked", "push"), "locked")

        let dfa = StateMachine.createDfa [ "locked"; "unlocked" ] [ "coin"; "push" ] transitions "locked" [ "unlocked" ]
        Assert.Equal("unlocked", dfa.Process("coin"))
