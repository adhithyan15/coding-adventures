namespace CodingAdventures.StateMachine.FSharp.Tests

open System.Collections.Generic
open CodingAdventures.StateMachine.FSharp
open Xunit

module StateMachineTests =
    [<Fact>]
    let ``dfa processes events`` () =
        let transitions = Dictionary<struct (string * string), string>()
        transitions.Add(struct ("locked", "coin"), "unlocked")
        transitions.Add(struct ("locked", "push"), "locked")
        transitions.Add(struct ("unlocked", "coin"), "unlocked")
        transitions.Add(struct ("unlocked", "push"), "locked")

        let dfa = StateMachine.createDfa [ "locked"; "unlocked" ] [ "coin"; "push" ] transitions "locked" [ "unlocked" ]
        Assert.Equal("unlocked", dfa.Process("coin"))

    [<Fact>]
    let ``minimize collapses equivalent states`` () =
        let transitions = Dictionary<struct (string * string), string>()
        transitions.Add(struct ("A", "0"), "B")
        transitions.Add(struct ("A", "1"), "C")
        transitions.Add(struct ("B", "0"), "B")
        transitions.Add(struct ("B", "1"), "C")
        transitions.Add(struct ("C", "0"), "B")
        transitions.Add(struct ("C", "1"), "C")

        let dfa = DFA([ "A"; "B"; "C" ], [ "0"; "1" ], transitions, "A", [ "C" ])
        let minimized = Minimize.Run(dfa)
        Assert.True((minimized.States :> seq<string>) |> Seq.length <= 3)
        Assert.True(minimized.Accepts([ "1" ]))

    [<Fact>]
    let ``pushdown automaton traces stack updates`` () =
        let transitions =
            [
                {
                    Source = "start"
                    Event = Some "("
                    StackRead = "$"
                    Target = "start"
                    StackPush = [ "$"; "(" ]
                }
                {
                    Source = "start"
                    Event = Some "("
                    StackRead = "("
                    Target = "start"
                    StackPush = [ "("; "(" ]
                }
                {
                    Source = "start"
                    Event = Some ")"
                    StackRead = "("
                    Target = "start"
                    StackPush = []
                }
            ]

        let pda =
            PushdownAutomaton(
                [ "start" ],
                [ "("; ")" ],
                [ "$"; "(" ],
                transitions,
                "start",
                "$",
                [ "start" ])

        pda.Process([ "("; "("; ")" ])
        Assert.Equal("start", pda.CurrentState)
        Assert.True(pda.Stack |> Seq.contains "(")
