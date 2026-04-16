namespace CodingAdventures.RespProtocol.FSharp.Tests

open CodingAdventures.RespProtocol.FSharp
open Xunit

type RespProtocolTests() =
    [<Fact>]
    member _.``round trips bulk strings``() =
        let encoded = RespProtocol.bulkString (Some "hello") |> RespProtocol.encode
        match RespProtocol.decode encoded with
        | Some result ->
            match result.Value with
            | RespBulkString(Some value) -> Assert.Equal("hello", value)
            | _ -> failwith "unexpected value"
        | None -> failwith "expected decode result"

    [<Fact>]
    member _.``supports incremental decoding``() =
        let decoder = RespDecoder()
        decoder.Feed("*1\r\n$4\r\nPING\r\n")
        Assert.True(decoder.HasMessage())
