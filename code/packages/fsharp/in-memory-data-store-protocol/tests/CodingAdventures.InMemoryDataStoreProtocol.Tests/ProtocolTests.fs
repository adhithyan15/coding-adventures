namespace CodingAdventures.InMemoryDataStoreProtocol.FSharp.Tests

open CodingAdventures.InMemoryDataStoreProtocol.FSharp
open CodingAdventures.RespProtocol.FSharp
open Xunit

type ProtocolTests() =
    [<Fact>]
    member _.``parses RESP command arrays``() =
        let command =
            RespArray(Some [ RespBulkString(Some "SET"); RespBulkString(Some "counter"); RespBulkString(Some "1") ])
            |> DataStoreProtocol.commandFromResp

        Assert.Equal(Some { Name = "SET"; Args = [ "counter"; "1" ] }, command)
