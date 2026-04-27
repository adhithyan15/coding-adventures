namespace CodingAdventures.ContentAddressableStorage.FSharp

open System
open System.Collections.Generic
open System.IO
open CodingAdventures.Sha1.FSharp

module ContentAddressableStoragePackage =
    [<Literal>]
    let Version = "0.1.0"

[<RequireQualifiedAccess>]
module Cas =
    [<Literal>]
    let KeyLength = 20

    let validateKey (key: byte array) =
        if isNull key then
            nullArg "key"

        if key.Length <> KeyLength then
            invalidArg "key" $"key must be exactly 20 bytes, got {key.Length}"

    let keyToHex (key: byte array) =
        validateKey key
        Convert.ToHexString(key).ToLowerInvariant()

    let hexToKey (hex: string) =
        if isNull hex then
            nullArg "hex"

        if hex.Length <> KeyLength * 2 then
            invalidArg "hex" $"expected 40 hex chars, got {hex.Length}"

        try
            Convert.FromHexString hex
        with :? FormatException as ex ->
            raise (ArgumentException($"invalid hex string: {hex}", "hex", ex))

    let decodeHexPrefix (prefix: string) =
        if isNull prefix then
            nullArg "prefix"

        if prefix.Length = 0 then
            invalidArg "prefix" "prefix cannot be empty"

        if prefix.Length > KeyLength * 2 then
            invalidArg "prefix" $"prefix cannot be longer than 40 hex chars, got {prefix.Length}"

        for ch in prefix do
            if not (Uri.IsHexDigit ch) then
                invalidArg "prefix" $"invalid hex character: {ch}"

        let padded =
            if prefix.Length % 2 = 0 then
                prefix
            else
                prefix + "0"

        Convert.FromHexString padded

type CasError =
    inherit Exception

    new(message: string) = { inherit Exception(message) }

    new(message: string, innerException: Exception) =
        { inherit Exception(message, innerException) }

type CasStoreError(message: string, innerException: Exception) =
    inherit CasError(message, innerException)

type CasNotFoundError(key: byte array) =
    inherit CasError($"object not found: {Cas.keyToHex key}")

    member _.Key = Array.copy key

type CasCorruptedError(key: byte array) =
    inherit CasError($"object corrupted: {Cas.keyToHex key}")

    member _.Key = Array.copy key

type CasAmbiguousPrefixError(prefix: string) =
    inherit CasError($"ambiguous prefix: {prefix}")

    member _.Prefix = prefix

type CasPrefixNotFoundError(prefix: string) =
    inherit CasError($"object not found for prefix: {prefix}")

    member _.Prefix = prefix

type CasInvalidPrefixError(prefix: string) =
    inherit CasError($"invalid hex prefix: {prefix}")

    member _.Prefix = prefix

type IBlobStore =
    abstract Put: key: byte array * data: byte array -> unit
    abstract Get: key: byte array -> byte array
    abstract Exists: key: byte array -> bool
    abstract KeysWithPrefix: prefix: byte array -> byte array list

type ContentAddressableStore(store: IBlobStore) =
    do
        if isNull (box store) then
            nullArg "store"

    member _.Store = store

    member _.Put(data: byte array) =
        if isNull data then
            nullArg "data"

        let key = Sha1.hash data

        try
            store.Put(key, data)
        with ex ->
            raise (CasStoreError(ex.Message, ex))

        key

    member _.Get(key: byte array) =
        Cas.validateKey key

        let data =
            try
                store.Get key
            with
            | :? FileNotFoundException as ex -> raise (CasNotFoundError key)
            | :? KeyNotFoundException as ex -> raise (CasNotFoundError key)
            | ex -> raise (CasStoreError(ex.Message, ex))

        if Sha1.hash data <> key then
            raise (CasCorruptedError key)

        data

    member _.Exists(key: byte array) =
        Cas.validateKey key

        try
            store.Exists key
        with ex ->
            raise (CasStoreError(ex.Message, ex))

    member _.FindByPrefix(hexPrefix: string) =
        let prefix =
            try
                Cas.decodeHexPrefix hexPrefix
            with :? ArgumentException ->
                raise (CasInvalidPrefixError hexPrefix)

        let matches =
            try
                store.KeysWithPrefix prefix
            with ex ->
                raise (CasStoreError(ex.Message, ex))

        match matches |> List.map Array.copy |> List.sortBy Cas.keyToHex with
        | [] -> raise (CasPrefixNotFoundError hexPrefix)
        | [ key ] -> key
        | _ -> raise (CasAmbiguousPrefixError hexPrefix)

type LocalDiskStore(root: string) =
    do
        if String.IsNullOrWhiteSpace root then
            invalidArg "root" "root cannot be empty"

        Directory.CreateDirectory root |> ignore

    member _.ObjectPath(key: byte array) =
        let hex = Cas.keyToHex key
        Path.Combine(root, hex.Substring(0, 2), hex.Substring(2))

    member this.Put(key: byte array, data: byte array) =
        Cas.validateKey key

        if isNull data then
            nullArg "data"

        let finalPath = this.ObjectPath key

        if not (File.Exists finalPath) then
            let directory = Path.GetDirectoryName finalPath
            Directory.CreateDirectory directory |> ignore

            let tempPath =
                Path.Combine(
                    directory,
                    $"{Path.GetFileName finalPath}.{Environment.ProcessId}.{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()}.{Guid.NewGuid():N}.tmp"
                )

            try
                File.WriteAllBytes(tempPath, data)

                try
                    File.Move(tempPath, finalPath)
                with :? IOException when File.Exists finalPath ->
                    File.Delete tempPath
            with _ ->
                if File.Exists tempPath then
                    File.Delete tempPath

                reraise()

    member this.Get(key: byte array) =
        Cas.validateKey key
        File.ReadAllBytes(this.ObjectPath key)

    member this.Exists(key: byte array) =
        Cas.validateKey key
        File.Exists(this.ObjectPath key)

    member _.KeysWithPrefix(prefix: byte array) =
        if isNull prefix then
            nullArg "prefix"

        if prefix.Length = 0 then
            []
        else
            let firstByteHex = prefix[0].ToString "x2"
            let bucket = Path.Combine(root, firstByteHex)

            if not (Directory.Exists bucket) then
                []
            else
                Directory.EnumerateFiles bucket
                |> Seq.choose (fun path ->
                    let name = Path.GetFileName path

                    if name.Length <> 38 then
                        None
                    else
                        try
                            let key = Cas.hexToKey (firstByteHex + name)
                            let startsWithPrefix = key |> Seq.take prefix.Length |> Seq.toArray = prefix

                            if startsWithPrefix then
                                Some key
                            else
                                None
                        with :? ArgumentException ->
                            None)
                |> Seq.toList

    interface IBlobStore with
        member this.Put(key, data) = this.Put(key, data)
        member this.Get key = this.Get key
        member this.Exists key = this.Exists key
        member this.KeysWithPrefix prefix = this.KeysWithPrefix prefix
