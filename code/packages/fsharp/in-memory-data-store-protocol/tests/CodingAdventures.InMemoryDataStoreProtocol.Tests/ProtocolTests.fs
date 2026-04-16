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

    [<Fact>]
    member _.``supports command helpers and validation``() =
        let command = DataStoreProtocol.commandFromParts [ " ping "; "hello" ]
        let frame = DataStoreProtocol.commandToResp command

        Assert.Equal("PING", DataStoreProtocol.commandName [ " ping " ])
        Assert.True(DataStoreProtocol.commandToParts command = [ "PING"; "hello" ])
        Assert.Equal(Some "1", DataStoreProtocol.respValueToString (RespInteger 1L))
        Assert.Equal(None, DataStoreProtocol.respValueToString (RespArray None))
        Assert.Equal(RespArray(Some [ RespBulkString(Some "PING"); RespBulkString(Some "hello") ]), frame)
        Assert.ThrowsAny<System.InvalidOperationException>(fun () -> DataStoreProtocol.commandFromParts [] |> ignore) |> ignore
        Assert.Equal(None, DataStoreProtocol.commandFromResp (RespSimpleString "OK"))
