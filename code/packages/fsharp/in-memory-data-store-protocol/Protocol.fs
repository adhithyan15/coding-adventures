namespace CodingAdventures.InMemoryDataStoreProtocol.FSharp

open CodingAdventures.RespProtocol.FSharp

type DataStoreCommand =
    {
        Name: string
        Args: string list
    }

module DataStoreProtocol =
    let commandFromParts (parts: string list) =
        match parts with
        | [] -> invalidOp "command frame cannot be empty"
        | name :: args -> { Name = name.Trim().ToUpperInvariant(); Args = args }

    let commandName parts = (commandFromParts parts).Name

    let commandToParts command = command.Name :: command.Args

    let respValueToString value =
        match value with
        | RespSimpleString text -> Some text
        | RespErrorValue text -> Some text
        | RespInteger value -> Some (string value)
        | RespBulkString value -> value
        | RespArray _ -> None

    let commandFromResp value =
        match value with
        | RespArray(Some values) when values.Length > 0 ->
            values
            |> List.map respValueToString
            |> List.fold
                (fun state item ->
                    match state, item with
                    | Some parts, Some part -> Some (parts @ [ part ])
                    | _ -> None)
                (Some [])
            |> Option.map commandFromParts
        | _ -> None

    let commandToResp command =
        commandToParts command |> List.map (Some >> RespBulkString) |> Some |> RespArray
