namespace CodingAdventures.EventLoop

open System
open System.Collections.Generic
open System.Threading

module EventLoopPackage =
    [<Literal>]
    let Version = "0.1.0"

[<RequireQualifiedAccess>]
type ControlFlow =
    | Continue
    | Exit

type IEventSource<'T> =
    abstract Poll: unit -> 'T list

type internal StopState() =
    let mutable stopped = 0

    member _.IsStopped =
        Volatile.Read(&stopped) <> 0

    member _.Stop() =
        Volatile.Write(&stopped, 1)

    member _.Reset() =
        Volatile.Write(&stopped, 0)

type StopHandle internal (state: StopState) =
    member _.Stop() = state.Stop()

type EventLoop<'T>() =
    let sources = ResizeArray<IEventSource<'T>>()
    let handlers = ResizeArray<'T -> ControlFlow>()
    let state = StopState()

    member _.AddSource(source: IEventSource<'T>) =
        if isNull (box source) then
            nullArg (nameof source)

        sources.Add(source)

    member _.OnEvent(handler: 'T -> ControlFlow) =
        if isNull (box handler) then
            nullArg (nameof handler)

        handlers.Add(handler)

    member _.GetStopHandle() =
        StopHandle(state)

    member _.Stop() =
        state.Stop()

    member _.Run() =
        state.Reset()

        while not state.IsStopped do
            let queue = ResizeArray<'T>()

            for source in sources do
                queue.AddRange(source.Poll())

            let mutable shouldExit = false
            let mutable eventIndex = 0

            while eventIndex < queue.Count && not shouldExit do
                let eventItem = queue[eventIndex]
                let mutable handlerIndex = 0

                while handlerIndex < handlers.Count && not shouldExit do
                    let handler = handlers[handlerIndex]

                    if handler eventItem = ControlFlow.Exit then
                        shouldExit <- true

                    handlerIndex <- handlerIndex + 1

                eventIndex <- eventIndex + 1

            if shouldExit then
                state.Stop()
            elif queue.Count = 0 then
                Thread.Yield() |> ignore
