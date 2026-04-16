namespace CodingAdventures.StateMachine.FSharp

open System.Collections.Generic
open CodingAdventures.StateMachine

module StateMachine =
    let createDfa
        (states: seq<string>)
        (alphabet: seq<string>)
        (transitions: IDictionary<struct (string * string), string>)
        (initial: string)
        (accepting: seq<string>) =
        DFA(states, alphabet, Dictionary<struct (string * string), string>(transitions), initial, accepting)
