namespace CodingAdventures.InMemoryDataStore.FSharp.Tests

open CodingAdventures.InMemoryDataStore.FSharp
open CodingAdventures.InMemoryDataStoreEngine.FSharp
open CodingAdventures.InMemoryDataStoreProtocol.FSharp
open CodingAdventures.RespProtocol.FSharp
open System.Text
open Xunit

type InMemoryDataStoreTests() =
    [<Fact>]
    member _.``executes RESP frames end to end``() =
        let store = InMemoryDataStore()
        let response = store.ExecuteFrame(RespArray(Some [ RespBulkString(Some "PING") ]))
        Assert.Equal(Some (RespSimpleString "PONG"), response)

    [<Fact>]
    member _.``supports execution helpers process and handle``() =
        let store = InMemoryDataStore()
        Assert.Equal(RespBulkString(Some "hello"), store.Execute([ "ECHO"; "hello" ]))
        Assert.Equal(RespBulkString(Some "again"), store.Execute({ Name = "ECHO"; Args = [ "again" ] }))

        let input =
            [ RespArray(Some [ RespBulkString(Some "SET"); RespBulkString(Some "counter"); RespBulkString(Some "1") ])
              RespArray(Some [ RespBulkString(Some "GET"); RespBulkString(Some "counter") ]) ]
            |> List.map RespProtocol.encode
            |> RespProtocol.concatBytes

        Assert.True(store.Process input = [ RespSimpleString "OK"; RespBulkString(Some "1") ])
        Assert.Equal("+PONG\r\n", store.Handle("*1\r\n$4\r\nPING\r\n") |> Encoding.UTF8.GetString)

    [<Fact>]
    member _.``supports reset and frame validation``() =
        let store = InMemoryDataStore()
        store.Reset(Store.Empty().Set("name", DataStoreTypes.stringEntry "redis" None))

        Assert.True(store.Store.Exists "name")
        Assert.Equal(Some (RespErrorValue "ERR expected RESP array command"), store.ExecuteFrame(RespSimpleString "PING"))
        Assert.Equal(Some (RespErrorValue "ERR expected RESP command array"), store.ExecuteFrame(RespArray(Some [ RespArray None ])))
        Assert.Equal(None, store.ExecuteFrame(RespArray(Some [])))
