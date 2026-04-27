namespace CodingAdventures.EventLoop.Tests

open System
open System.Collections.Generic
open System.Threading
open System.Threading.Tasks
open CodingAdventures.EventLoop
open Xunit

type FixedSource<'T>(batches: 'T list list) =
    let mutable remaining = batches

    interface IEventSource<'T> with
        member _.Poll() =
            match remaining with
            | [] -> []
            | batch :: tail ->
                remaining <- tail
                batch

type InfiniteSource() =
    let mutable next = 0

    interface IEventSource<int> with
        member _.Poll() =
            next <- next + 1
            [ next ]

type EventLoopTests() =
    [<Fact>]
    member _.VersionExists() =
        Assert.Equal("0.1.0", EventLoopPackage.Version)

    [<Fact>]
    member _.DeliversAllEvents() =
        let loop = EventLoop<int>()
        loop.AddSource(FixedSource([ [ 1; 2; 3 ]; [ -1 ] ]))
        let received = ResizeArray<int>()

        loop.OnEvent(fun eventItem ->
            if eventItem = -1 then
                ControlFlow.Exit
            else
                received.Add(eventItem)
                ControlFlow.Continue)

        loop.Run()

        Assert.Equal<int list>([ 1; 2; 3 ], received |> Seq.toList)

    [<Fact>]
    member _.ExitStopsLoopImmediately() =
        let loop = EventLoop<string>()
        loop.AddSource(FixedSource([ [ "a"; "b"; "stop"; "c"; "d" ] ]))
        let seen = ResizeArray<string>()

        loop.OnEvent(fun eventItem ->
            seen.Add(eventItem)

            if eventItem = "stop" then
                ControlFlow.Exit
            else
                ControlFlow.Continue)

        loop.Run()

        Assert.Equal<string list>([ "a"; "b"; "stop" ], seen |> Seq.toList)

    [<Fact>]
    member _.StopFromHandlerTerminatesLoop() =
        let loop = EventLoop<int>()
        loop.AddSource(InfiniteSource())
        let mutable count = 0

        loop.OnEvent(fun _ ->
            count <- count + 1

            if count >= 5 then
                loop.Stop()

            ControlFlow.Continue)

        loop.Run()

        Assert.Equal(5, count)

    [<Fact>]
    member _.MultipleHandlersAllSeeEvent() =
        let loop = EventLoop<int>()
        loop.AddSource(FixedSource([ [ 99 ]; [ -1 ] ]))
        let mutable first = Nullable<int>()
        let mutable second = Nullable<int>()

        loop.OnEvent(fun eventItem ->
            if eventItem = 99 then
                first <- Nullable(eventItem)

            if eventItem = -1 then
                ControlFlow.Exit
            else
                ControlFlow.Continue)

        loop.OnEvent(fun eventItem ->
            if eventItem = 99 then
                second <- Nullable(eventItem)

            ControlFlow.Continue)

        loop.Run()

        Assert.Equal(99, first.Value)
        Assert.Equal(99, second.Value)

    [<Fact>]
    member _.MultipleSourcesAreMerged() =
        let loop = EventLoop<string>()
        loop.AddSource(FixedSource([ [ "from-a" ] ]))
        loop.AddSource(FixedSource([ [ "from-b" ] ]))
        loop.AddSource(FixedSource([ []; [ "stop" ] ]))
        let seen = ResizeArray<string>()

        loop.OnEvent(fun eventItem ->
            if eventItem = "stop" then
                ControlFlow.Exit
            else
                seen.Add(eventItem)
                ControlFlow.Continue)

        loop.Run()

        Assert.Equal(2, seen.Count)
        Assert.Contains("from-a", seen)
        Assert.Contains("from-b", seen)

    [<Fact>]
    member _.StopWhileIdleTerminatesLoop() =
        let loop = EventLoop<int>()
        let mutable called = false

        loop.OnEvent(fun _ ->
            called <- true
            ControlFlow.Continue)

        let stopTask =
            Task.Run(Action(fun () ->
                Thread.Sleep(10)
                loop.Stop()))

        loop.Run()
        stopTask.Wait()

        Assert.False(called)

    [<Fact>]
    member _.StopHandleTerminatesIdleLoop() =
        let loop = EventLoop<int>()
        let handle = loop.GetStopHandle()

        let stopTask =
            Task.Run(Action(fun () ->
                Thread.Sleep(10)
                handle.Stop()))

        loop.Run()
        stopTask.Wait()

    [<Fact>]
    member _.ControlFlowValuesAreDistinct() =
        Assert.NotEqual(ControlFlow.Continue, ControlFlow.Exit)

    [<Fact>]
    member _.AddSourceAndOnEventRejectNull() =
        let loop = EventLoop<int>()

        Assert.Throws<ArgumentNullException>(fun () -> loop.AddSource(Unchecked.defaultof<IEventSource<int>>)) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> loop.OnEvent(Unchecked.defaultof<int -> ControlFlow>)) |> ignore

    [<Fact>]
    member _.NoEventsMeansHandlersAreNotCalled() =
        let loop = EventLoop<int>()
        loop.AddSource(FixedSource([ [] ]))
        let mutable count = 0

        loop.OnEvent(fun _ ->
            count <- count + 1
            ControlFlow.Continue)

        let stopTask =
            Task.Run(Action(fun () ->
                Thread.Sleep(10)
                loop.Stop()))

        loop.Run()
        stopTask.Wait()

        Assert.Equal(0, count)

    [<Fact>]
    member _.HandlerSeesEventsInOrder() =
        let loop = EventLoop<int>()
        loop.AddSource(FixedSource([ [ 3; 1; 4; 1; 5 ]; [ -1 ] ]))
        let received = ResizeArray<int>()

        loop.OnEvent(fun eventItem ->
            if eventItem = -1 then
                ControlFlow.Exit
            else
                received.Add(eventItem)
                ControlFlow.Continue)

        loop.Run()

        Assert.Equal<int list>([ 3; 1; 4; 1; 5 ], received |> Seq.toList)
