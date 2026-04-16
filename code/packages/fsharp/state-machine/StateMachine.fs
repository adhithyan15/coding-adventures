namespace CodingAdventures.StateMachine.FSharp

open System
open System.Collections.Generic

type TransitionAction = delegate of source: string * eventName: string * target: string -> unit

[<CLIMutable>]
type TransitionRecord =
    {
        Source: string
        Event: string
        Target: string
        ActionName: string
    }

[<CLIMutable>]
type ModeTransitionRecord =
    {
        FromMode: string
        Trigger: string
        ToMode: string
    }

[<CLIMutable>]
type PDATransition =
    {
        Source: string
        Event: string option
        StackRead: string
        Target: string
        StackPush: IReadOnlyList<string>
    }

[<CLIMutable>]
type PDATraceEntry =
    {
        Source: string
        Event: string option
        StackRead: string
        Target: string
        StackPush: IReadOnlyList<string>
        StackAfter: IReadOnlyList<string>
    }

type DFA
    (
        states: seq<string>,
        alphabet: seq<string>,
        transitions: IDictionary<struct (string * string), string>,
        initial: string,
        accepting: seq<string>,
        ?actions: IDictionary<struct (string * string), TransitionAction>
    ) =
    let states = HashSet<string>(states, StringComparer.Ordinal)
    let alphabet = HashSet<string>(alphabet, StringComparer.Ordinal)
    let accepting = HashSet<string>(accepting, StringComparer.Ordinal)
    let transitions = Dictionary<struct (string * string), string>(transitions)
    let actions =
        match actions with
        | Some value -> Dictionary<struct (string * string), TransitionAction>(value)
        | None -> Dictionary<struct (string * string), TransitionAction>()

    let trace = ResizeArray<TransitionRecord>()
    let mutable currentState = initial

    do
        if states.Count = 0 then
            invalidArg "states" "states set must be non-empty"

        if not (states.Contains(initial)) then
            invalidArg "initial" (sprintf "Initial state '%s' is not in the states set" initial)

        if not (accepting |> Seq.forall states.Contains) then
            invalidArg "accepting" "Accepting states must be a subset of states"

        for KeyValue(struct (state, eventName), target) in transitions do
            if not (states.Contains(state)) || not (states.Contains(target)) then
                invalidArg "transitions" "Transition source and target must be in states"

            if not (alphabet.Contains(eventName)) then
                invalidArg "transitions" "Transition events must be in alphabet"

    member _.States = states :> IReadOnlyCollection<string>
    member _.Alphabet = alphabet :> IReadOnlyCollection<string>
    member _.Accepting = accepting :> IReadOnlyCollection<string>
    member _.Initial = initial
    member _.CurrentState = currentState
    member _.Transitions = Dictionary<struct (string * string), string>(transitions) :> IReadOnlyDictionary<struct (string * string), string>
    member _.Trace = trace |> Seq.toList

    member _.Process(eventName: string) =
        let key = struct (currentState, eventName)
        match transitions.TryGetValue(key) with
        | true, target ->
            let action =
                match actions.TryGetValue(key) with
                | true, value -> Some value
                | _ -> None

            action |> Option.iter (fun callback -> callback.Invoke(currentState, eventName, target))
            trace.Add(
                {
                    Source = currentState
                    Event = eventName
                    Target = target
                    ActionName = action |> Option.map (fun callback -> callback.Method.Name) |> Option.defaultValue String.Empty
                })
            currentState <- target
            target
        | _ ->
            invalidOp (sprintf "No transition defined for (%s, %s)" currentState eventName)

    member this.Process(events: seq<string>) =
        let mutable state = currentState
        for eventName in events do
            state <- this.Process(eventName)

        state

    member _.Reset() =
        currentState <- initial
        trace.Clear()

    member _.IsAccepting() = accepting.Contains(currentState)

    member this.Accepts(events: seq<string>) =
        this.Reset()
        this.Process(events) |> ignore
        this.IsAccepting()

    member _.MissingTransitions() =
        let missing = Dictionary<struct (string * string), string>()
        for state in states do
            for eventName in alphabet do
                let key = struct (state, eventName)
                if not (transitions.ContainsKey(key)) then
                    missing.[key] <- String.Empty

        missing

    member this.IsComplete() = this.MissingTransitions().Count = 0

    member _.ReachableStates() =
        let reachable = HashSet<string>(StringComparer.Ordinal)
        reachable.Add(initial) |> ignore
        let queue = Queue<string>()
        queue.Enqueue(initial)

        while queue.Count > 0 do
            let state = queue.Dequeue()
            for eventName in alphabet do
                match transitions.TryGetValue(struct (state, eventName)) with
                | true, target when reachable.Add(target) ->
                    queue.Enqueue(target)
                | _ -> ()

        reachable :> ISet<string>

type NFA
    (
        states: seq<string>,
        alphabet: seq<string>,
        transitions: IDictionary<struct (string * string), seq<string>>,
        initial: string,
        accepting: seq<string>
    ) =
    let states = HashSet<string>(states, StringComparer.Ordinal)
    let alphabet = HashSet<string>(alphabet, StringComparer.Ordinal)
    let accepting = HashSet<string>(accepting, StringComparer.Ordinal)
    let transitions =
        Dictionary<struct (string * string), HashSet<string>>(
            transitions
            |> Seq.map (fun (KeyValue(key, value)) -> KeyValuePair<_, _>(key, HashSet<string>(value, StringComparer.Ordinal))))

    let mutable currentStates = HashSet<string>(StringComparer.Ordinal)

    do
        if not (states.Contains(initial)) then
            invalidArg "initial" (sprintf "Initial state '%s' is not in the states set" initial)

        currentStates <-
            let closure = HashSet<string>(StringComparer.Ordinal)
            closure.Add(initial) |> ignore
            closure

    static member EPSILON = ""

    member _.States = states :> IReadOnlyCollection<string>
    member _.Alphabet = alphabet :> IReadOnlyCollection<string>
    member _.Accepting = accepting :> IReadOnlyCollection<string>
    member _.Initial = initial
    member _.CurrentStates = currentStates :> IReadOnlySet<string>

    member this.Reset() =
        currentStates <- this.EpsilonClosure([ initial ])

    member _.EpsilonClosure(seedStates: seq<string>) =
        let closure = HashSet<string>(seedStates, StringComparer.Ordinal)
        let stack = Stack<string>(closure)
        while stack.Count > 0 do
            let state = stack.Pop()
            match transitions.TryGetValue(struct (state, NFA.EPSILON)) with
            | true, targets ->
                for target in targets do
                    if closure.Add(target) then
                        stack.Push(target)
            | _ -> ()

        closure

    member this.Process(eventName: string) =
        let next = HashSet<string>(StringComparer.Ordinal)
        for state in currentStates do
            match transitions.TryGetValue(struct (state, eventName)) with
            | true, targets -> next.UnionWith(targets)
            | _ -> ()

        currentStates <- this.EpsilonClosure(next)
        currentStates :> IReadOnlySet<string>

    member this.Accepts(events: seq<string>) =
        this.Reset()
        for eventName in events do
            this.Process(eventName) |> ignore

        currentStates.Overlaps(accepting)

[<AbstractClass; Sealed>]
type Minimize private () =
    static member Run(dfa: DFA) =
        let reachable = dfa.ReachableStates()
        let accepting = HashSet<string>(reachable |> Seq.filter (fun state -> (dfa.Accepting :> seq<string>) |> Seq.contains state), StringComparer.Ordinal)
        let nonAccepting = HashSet<string>(reachable |> Seq.filter (fun state -> not (accepting.Contains(state))), StringComparer.Ordinal)
        let mutable partitions =
            [
                if accepting.Count > 0 then accepting
                if nonAccepting.Count > 0 then nonAccepting
            ]

        let splitGroup (group: HashSet<string>) (allPartitions: HashSet<string> list) =
            if group.Count <= 1 then
                [ group ]
            else
                let stateToPartition =
                    allPartitions
                    |> List.mapi (fun index partition -> partition |> Seq.map (fun state -> state, index))
                    |> Seq.concat
                    |> dict

                group
                |> Seq.groupBy (fun state ->
                    dfa.Alphabet
                    |> Seq.cast<string>
                    |> Seq.sortWith (fun left right -> StringComparer.Ordinal.Compare(left, right))
                    |> Seq.map (fun eventName ->
                        match dfa.Transitions.TryGetValue(struct (state, eventName)) with
                        | true, target -> string stateToPartition.[target]
                        | _ -> "-1")
                    |> String.concat "|")
                |> Seq.map (fun (_, states) -> HashSet<string>(states, StringComparer.Ordinal))
                |> Seq.toList

        let mutable changed = true
        while changed do
            changed <- false
            let nextPartitions = ResizeArray<HashSet<string>>()
            for group in partitions do
                let splits = splitGroup group partitions
                nextPartitions.AddRange(splits)
                if splits.Length > 1 then
                    changed <- true

            partitions <- nextPartitions |> Seq.toList

        let partitionNames =
            partitions
            |> List.map (fun group ->
                if group.Count = 1 then
                    Seq.head group
                else
                    "{" + (group |> Seq.sortWith (fun left right -> StringComparer.Ordinal.Compare(left, right)) |> String.concat ",") + "}")

        let stateToPartition =
            partitions
            |> List.mapi (fun index partition -> partition |> Seq.map (fun state -> state, index))
            |> Seq.concat
            |> dict

        let minimizedTransitions = Dictionary<struct (string * string), string>()
        for index, group in partitions |> List.indexed do
            let representative = Seq.head group
            let fromName = partitionNames.[index]
            for eventName in dfa.Alphabet |> Seq.cast<string> do
                match dfa.Transitions.TryGetValue(struct (representative, eventName)) with
                | true, target ->
                    minimizedTransitions.[struct (fromName, eventName)] <- partitionNames.[stateToPartition.[target]]
                | _ -> ()

        let acceptingStates =
            partitions
            |> List.indexed
            |> List.choose (fun (index, group) ->
                if group |> Seq.exists (fun state -> (dfa.Accepting :> seq<string>) |> Seq.contains state) then
                    Some partitionNames.[index]
                else
                    None)

        DFA(partitionNames, dfa.Alphabet, minimizedTransitions, partitionNames.[stateToPartition.[dfa.Initial]], acceptingStates)

type ModalStateMachine(modes: IDictionary<string, DFA>, modeTransitions: IDictionary<struct (string * string), string>, initialMode: string) =
    let modes = Dictionary<string, DFA>(modes, StringComparer.Ordinal)
    let modeTransitions = Dictionary<struct (string * string), string>(modeTransitions)
    let modeTrace = ResizeArray<ModeTransitionRecord>()
    let mutable currentMode = initialMode

    do
        if not (modes.ContainsKey(initialMode)) then
            invalidArg "initialMode" (sprintf "Unknown mode '%s'" initialMode)

    member _.InitialMode = initialMode
    member _.CurrentMode = currentMode
    member _.ActiveMachine = modes.[currentMode]
    member _.ModeTrace = modeTrace |> Seq.toList

    member this.Process(eventName: string) =
        this.ActiveMachine.Process(eventName)

    member this.TriggerMode(trigger: string) =
        match modeTransitions.TryGetValue(struct (currentMode, trigger)) with
        | true, target ->
            let fromMode = currentMode
            currentMode <- target
            this.ActiveMachine.Reset()
            modeTrace.Add({ FromMode = fromMode; Trigger = trigger; ToMode = target })
            target
        | _ ->
            invalidOp (sprintf "No mode transition defined for (%s, %s)" currentMode trigger)

    member _.Reset() =
        currentMode <- initialMode
        modeTrace.Clear()
        for machine in modes.Values do
            machine.Reset()

type PushdownAutomaton
    (
        states: seq<string>,
        inputAlphabet: seq<string>,
        stackAlphabet: seq<string>,
        transitions: seq<PDATransition>,
        initial: string,
        initialStackSymbol: string,
        accepting: seq<string>
    ) =
    let index =
        Dictionary<struct (string * string option * string), PDATransition>(
            transitions
            |> Seq.map (fun transition -> KeyValuePair<_, _>(struct (transition.Source, transition.Event, transition.StackRead), transition)))

    let accepting = HashSet<string>(accepting, StringComparer.Ordinal)
    let stack = ResizeArray<string>()
    let trace = ResizeArray<PDATraceEntry>()
    let mutable currentState = initial

    do
        if not (states |> Seq.exists ((=) initial)) then
            invalidArg "initial" (sprintf "Initial state '%s' is not in the states set" initial)

        if not (accepting |> Seq.forall (fun state -> states |> Seq.exists ((=) state))) then
            invalidArg "accepting" "Accepting states must be a subset of states"

        ignore inputAlphabet
        ignore stackAlphabet
        stack.Add(initialStackSymbol)

    let applyTransition (transition: PDATransition) =
        if stack.Count = 0 || stack.[stack.Count - 1] <> transition.StackRead then
            invalidOp "PDA stack top does not match transition"

        stack.RemoveAt(stack.Count - 1)
        for symbol in transition.StackPush do
            stack.Add(symbol)

        currentState <- transition.Target
        trace.Add(
            {
                Source = transition.Source
                Event = transition.Event
                StackRead = transition.StackRead
                Target = transition.Target
                StackPush = transition.StackPush
                StackAfter = stack |> Seq.toList
            })

    let step eventName =
        let top = if stack.Count > 0 then stack.[stack.Count - 1] else String.Empty
        match index.TryGetValue(struct (currentState, eventName, top)) with
        | true, transition -> applyTransition transition
        | _ -> invalidOp (sprintf "No PDA transition defined for (%s, %A, %s)" currentState eventName top)

    let tryStepEpsilon () =
        let top = if stack.Count > 0 then stack.[stack.Count - 1] else String.Empty
        match index.TryGetValue(struct (currentState, None, top)) with
        | true, transition ->
            applyTransition transition
            true
        | _ -> false

    member _.Initial = initial
    member _.InitialStackSymbol = initialStackSymbol
    member _.CurrentState = currentState
    member _.Stack = stack |> Seq.toList
    member _.Trace = trace |> Seq.toList

    member _.Reset() =
        currentState <- initial
        stack.Clear()
        stack.Add(initialStackSymbol)
        trace.Clear()

    member _.Process(eventName: string) =
        step (Some eventName)
        while tryStepEpsilon () do
            ()

    member this.Process(events: seq<string>) =
        for eventName in events do
            this.Process(eventName)

        while tryStepEpsilon () do
            ()

    member this.Accepts(events: seq<string>) =
        this.Reset()
        this.Process(events)
        accepting.Contains(currentState)

module StateMachine =
    let createDfa
        (states: seq<string>)
        (alphabet: seq<string>)
        (transitions: IDictionary<struct (string * string), string>)
        (initial: string)
        (accepting: seq<string>) =
        DFA(states, alphabet, transitions, initial, accepting)
