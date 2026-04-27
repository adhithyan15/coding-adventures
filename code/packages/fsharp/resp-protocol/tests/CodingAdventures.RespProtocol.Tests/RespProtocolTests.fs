namespace CodingAdventures.RespProtocol.FSharp.Tests

open CodingAdventures.RespProtocol.FSharp
open System.Text
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

    [<Fact>]
    member _.``round trips multiple RESP types``() =
        let simple = RespProtocol.simpleString "OK" |> RespProtocol.encode |> RespProtocol.decode
        let err = RespProtocol.errorValue "ERR boom" |> RespProtocol.encode |> RespProtocol.decode
        let integer = RespProtocol.integer 42L |> RespProtocol.encode |> RespProtocol.decode
        let array =
            RespProtocol.array (Some [ RespBulkString(Some "SET"); RespBulkString(Some "k"); RespBulkString(Some "1") ])
            |> RespProtocol.encode
            |> RespProtocol.decode

        Assert.Equal(Some (RespSimpleString "OK"), simple |> Option.map _.Value)
        Assert.Equal(Some (RespErrorValue "ERR boom"), err |> Option.map _.Value)
        Assert.Equal(Some (RespInteger 42L), integer |> Option.map _.Value)
        Assert.Equal(Some (RespArray(Some [ RespBulkString(Some "SET"); RespBulkString(Some "k"); RespBulkString(Some "1") ])), array |> Option.map _.Value)

    [<Fact>]
    member _.``supports null arrays inline commands and decode all``() =
        let nilBulk = RespProtocol.decodeText "$-1\r\n"
        let nilArray = RespProtocol.decodeText "*-1\r\n"
        let inlineCommand = RespProtocol.decodeText "PING hello\r\n"
        let combined =
            RespProtocol.concatBytes [ RespProtocol.encode (RespSimpleString "OK"); RespProtocol.encode (RespInteger 2L) ]
            |> RespProtocol.decodeAll

        Assert.Equal(Some (RespBulkString None), nilBulk |> Option.map _.Value)
        Assert.Equal(Some (RespArray None), nilArray |> Option.map _.Value)
        Assert.Equal(Some (RespArray(Some [ RespBulkString(Some "PING"); RespBulkString(Some "hello") ])), inlineCommand |> Option.map _.Value)
        Assert.Equal(2, combined.Values.Length)

    [<Fact>]
    member _.``handles decoder boundaries and empty queue``() =
        let decoder = RespDecoder()
        decoder.Feed(Encoding.UTF8.GetBytes("$5\r\nhel"))
        Assert.False(decoder.HasMessage())
        decoder.Feed("lo\r\n")
        Assert.Equal(RespBulkString(Some "hello"), decoder.GetMessage())
        Assert.Throws<RespDecodeError>(fun () -> decoder.GetMessage() |> ignore) |> ignore
