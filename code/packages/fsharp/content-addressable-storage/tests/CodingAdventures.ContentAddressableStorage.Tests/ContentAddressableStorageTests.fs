namespace CodingAdventures.ContentAddressableStorage.Tests

open System
open System.Collections.Generic
open System.IO
open System.Text
open Xunit
open CodingAdventures.ContentAddressableStorage.FSharp
open CodingAdventures.Sha1.FSharp

type MemoryStore() =
    let objects = Dictionary<string, byte array>()

    member _.Put(key: byte array, data: byte array) =
        objects[Cas.keyToHex key] <- Array.copy data

    member _.Get(key: byte array) =
        match objects.TryGetValue(Cas.keyToHex key) with
        | true, value -> Array.copy value
        | false, _ -> raise (FileNotFoundException(Cas.keyToHex key))

    member _.Exists(key: byte array) =
        objects.ContainsKey(Cas.keyToHex key)

    member _.KeysWithPrefix(prefix: byte array) =
        objects.Keys
        |> Seq.map Cas.hexToKey
        |> Seq.filter (fun key -> key |> Seq.take prefix.Length |> Seq.toArray = prefix)
        |> Seq.toList

    interface IBlobStore with
        member this.Put(key, data) = this.Put(key, data)
        member this.Get key = this.Get key
        member this.Exists key = this.Exists key
        member this.KeysWithPrefix prefix = this.KeysWithPrefix prefix

type FailingStore() =
    interface IBlobStore with
        member _.Put(_, _) = raise (IOException "put failed")
        member _.Get _ = raise (IOException "get failed")
        member _.Exists _ = raise (IOException "exists failed")
        member _.KeysWithPrefix _ = raise (IOException "prefix failed")

type TemporaryDirectory() =
    let path = Path.Combine(Path.GetTempPath(), $"ca-cas-{Guid.NewGuid():N}")

    do Directory.CreateDirectory path |> ignore

    member _.Path = path

    interface IDisposable with
        member _.Dispose() =
            if Directory.Exists path then
                Directory.Delete(path, true)

module ContentAddressableStorageTests =
    [<Fact>]
    let ``version is stable`` () =
        Assert.Equal("0.1.0", ContentAddressableStoragePackage.Version)

    [<Fact>]
    let ``hex helpers validate and round trip`` () =
        let hex = "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5"
        let key = Cas.hexToKey hex

        Assert.Equal(20, key.Length)
        Assert.Equal(hex, Cas.keyToHex key)
        Assert.Equal(String.replicate 40 "0", Cas.keyToHex (Array.zeroCreate 20))
        Assert.Throws<ArgumentException>(fun () -> Cas.hexToKey "abc" |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Cas.hexToKey (String.replicate 40 "z") |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Cas.keyToHex [| 1uy; 2uy; 3uy |] |> ignore) |> ignore

    [<Fact>]
    let ``decode hex prefix pads odd length on right`` () =
        Assert.Equal<byte>([| 0xa3uy; 0xf4uy |], Cas.decodeHexPrefix "a3f4")
        Assert.Equal<byte>([| 0xa3uy; 0xf0uy |], Cas.decodeHexPrefix "a3f")
        Assert.Equal<byte>([| 0xa0uy |], Cas.decodeHexPrefix "a")
        Assert.Equal<byte>(Cas.hexToKey "a3f4b2c1d0e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5", Cas.decodeHexPrefix "A3F4B2C1D0E9F8A7B6C5D4E3F2A1B0C9D8E7F6A5")
        Assert.Throws<ArgumentException>(fun () -> Cas.decodeHexPrefix "" |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Cas.decodeHexPrefix "xyz" |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Cas.decodeHexPrefix (String.replicate 41 "a") |> ignore) |> ignore

    [<Fact>]
    let ``memory store round trips and deduplicates`` () =
        let cas = ContentAddressableStore(MemoryStore())
        let data = Encoding.UTF8.GetBytes "hello, world"

        let firstKey = cas.Put data
        let secondKey = cas.Put data

        Assert.Equal<byte>(firstKey, secondKey)
        Assert.Equal<byte>(Sha1.hash data, firstKey)
        Assert.Equal<byte>(data, cas.Get firstKey)
        Assert.True(cas.Exists firstKey)
        Assert.False(cas.Exists(Sha1.hash "missing"B))

    [<Fact>]
    let ``supports empty and large blobs`` () =
        let cas = ContentAddressableStore(MemoryStore())
        let emptyKey = cas.Put [||]
        let large = Array.create (1024 * 1024) (byte 'x')
        let largeKey = cas.Put large

        Assert.Empty(cas.Get emptyKey)
        Assert.Equal<byte>(large, cas.Get largeKey)

    [<Fact>]
    let ``missing and corrupted objects raise typed errors`` () =
        let store = MemoryStore()
        let cas = ContentAddressableStore(store)
        let key = cas.Put "original"B
        store.Put(key, "tampered"B)

        let corrupted = Assert.Throws<CasCorruptedError>(fun () -> cas.Get key |> ignore)
        Assert.Equal<byte>(key, corrupted.Key)
        Assert.Contains(Cas.keyToHex key, corrupted.Message)

        let missing = Sha1.hash "missing"B
        let notFound = Assert.Throws<CasNotFoundError>(fun () -> cas.Get missing |> ignore)
        Assert.Equal<byte>(missing, notFound.Key)
        Assert.IsAssignableFrom<CasError>(notFound) |> ignore

    [<Fact>]
    let ``find by prefix resolves unique missing ambiguous and invalid prefixes`` () =
        let store = MemoryStore()
        let key1 = Cas.hexToKey "abcd123400000000000000000000000000000000"
        let key2 = Cas.hexToKey "abcd1234ffffffffffffffffffffffffffffffff"
        store.Put(key1, [||])
        store.Put(key2, [||])
        let cas = ContentAddressableStore(store)

        Assert.Equal<byte>(key1, cas.FindByPrefix(Cas.keyToHex key1))
        Assert.Throws<CasAmbiguousPrefixError>(fun () -> cas.FindByPrefix "abcd1234" |> ignore) |> ignore
        Assert.Throws<CasPrefixNotFoundError>(fun () -> cas.FindByPrefix "0000" |> ignore) |> ignore
        let invalid = Assert.Throws<CasInvalidPrefixError>(fun () -> cas.FindByPrefix "zzz" |> ignore)
        Assert.Equal("zzz", invalid.Prefix)

    [<Fact>]
    let ``backend errors are wrapped`` () =
        let cas = ContentAddressableStore(FailingStore())

        Assert.Throws<CasStoreError>(fun () -> cas.Put [| 1uy |] |> ignore) |> ignore
        Assert.Throws<CasStoreError>(fun () -> cas.Get(Sha1.hash [| 2uy |]) |> ignore) |> ignore
        Assert.Throws<CasStoreError>(fun () -> cas.Exists(Sha1.hash [| 3uy |]) |> ignore) |> ignore
        Assert.Throws<CasStoreError>(fun () -> cas.FindByPrefix "abcd" |> ignore) |> ignore

    [<Fact>]
    let ``store property returns wrapped store`` () =
        let inner = MemoryStore()
        let cas = ContentAddressableStore(inner)

        Assert.Same(inner, cas.Store)

    [<Fact>]
    let ``local disk store round trips with git style layout`` () =
        use temp = new TemporaryDirectory()
        let store = LocalDiskStore temp.Path
        let cas = ContentAddressableStore(store)
        let data = "persisted to disk"B

        let key = cas.Put data
        let hex = Cas.keyToHex key
        let expectedPath = Path.Combine(temp.Path, hex.Substring(0, 2), hex.Substring(2))

        Assert.Equal(expectedPath, store.ObjectPath key)
        Assert.True(File.Exists expectedPath)
        Assert.Equal<byte>(data, cas.Get key)
        Assert.True(cas.Exists key)
        Assert.Equal<byte>(key, cas.FindByPrefix(hex.Substring(0, 10)))
        Assert.Empty(Directory.EnumerateFiles(temp.Path, "*.tmp", SearchOption.AllDirectories))

    [<Fact>]
    let ``local disk store creates root skips artifacts and handles empty prefix`` () =
        use temp = new TemporaryDirectory()
        let root = Path.Combine(temp.Path, "deep", "objects")
        let store = LocalDiskStore root
        let key = Sha1.hash "real object"B
        store.Put(key, "real object"B)
        let hex = Cas.keyToHex key
        let bucket = Path.Combine(root, hex.Substring(0, 2))

        File.WriteAllBytes(Path.Combine(bucket, "some-temp.tmp"), [||])
        File.WriteAllBytes(Path.Combine(bucket, String.replicate 38 "z"), [||])
        Directory.CreateDirectory(Path.Combine(bucket, String.replicate 38 "a")) |> ignore

        Assert.True(Directory.Exists root)
        Assert.Empty(store.KeysWithPrefix [||])
        Assert.Empty(store.KeysWithPrefix [| 0x00uy |])
        Assert.Contains(store.KeysWithPrefix [| key[0] |], fun candidate -> candidate = key)

    [<Fact>]
    let ``local disk store second put skips existing file`` () =
        use temp = new TemporaryDirectory()
        let store = LocalDiskStore temp.Path
        let data = "idempotent"B
        let key = Sha1.hash data

        store.Put(key, data)
        let path = store.ObjectPath key
        let before = File.GetLastWriteTimeUtc path
        store.Put(key, data)

        Assert.Equal(before, File.GetLastWriteTimeUtc path)
