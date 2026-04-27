namespace CodingAdventures.InMemoryDataStore.FSharp

open System.Text
open CodingAdventures.InMemoryDataStoreEngine.FSharp
open CodingAdventures.InMemoryDataStoreProtocol.FSharp
open CodingAdventures.RespProtocol.FSharp

type InMemoryDataStore(?engine: DataStoreEngine) =
    let engine = defaultArg engine (DataStoreEngine())
    let mutable decoder = RespDecoder()

    member _.Store = engine.Store

    member _.Execute(command: DataStoreCommand) = engine.Execute(command.Name :: command.Args)
    member _.Execute(parts: string list) = engine.Execute(parts)
    member _.Reset(store: Store) = engine.Reset(store); decoder <- RespDecoder()

    member _.ExecuteFrame(frame: RespValue) =
        match frame with
        | RespArray(Some values) when values.Length > 0 ->
            match DataStoreProtocol.commandFromResp frame with
            | Some command -> Some (engine.Execute(command.Name :: command.Args))
            | None -> Some (RespErrorValue "ERR expected RESP command array")
        | RespArray(Some []) -> None
        | _ -> Some (RespErrorValue "ERR expected RESP array command")

    member this.Process(input: byte[]) =
        decoder.Feed input
        let mutable output = []
        while decoder.HasMessage() do
            match this.ExecuteFrame(decoder.GetMessage()) with
            | Some response -> output <- output @ [ response ]
            | None -> ()
        output

    member this.Process(text: string) = this.Process(Encoding.UTF8.GetBytes text)
    member this.Handle(input: byte[]) = this.Process(input) |> List.map RespProtocol.encode |> RespProtocol.concatBytes
    member this.Handle(text: string) = this.Process(text) |> List.map RespProtocol.encode |> RespProtocol.concatBytes
