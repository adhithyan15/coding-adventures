namespace CodingAdventures.InMemoryDataStore.FSharp.Tests

open CodingAdventures.InMemoryDataStore.FSharp
open CodingAdventures.RespProtocol.FSharp
open Xunit

type InMemoryDataStoreTests() =
    [<Fact>]
    member _.``executes RESP frames end to end``() =
        let store = InMemoryDataStore()
        let response = store.ExecuteFrame(RespArray(Some [ RespBulkString(Some "PING") ]))
        Assert.Equal(Some (RespSimpleString "PONG"), response)
