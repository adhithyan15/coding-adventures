module CodingAdventures.WasmRuntime.Tests

open System.Collections.Generic
open System.Text
open CodingAdventures.WasmLeb128.FSharp
open CodingAdventures.WasmRuntime.FSharp
open Xunit

let private buildSquareWasm () =
    let parts = ResizeArray<byte>()
    parts.AddRange([| 0x00uy; 0x61uy; 0x73uy; 0x6Duy; 0x01uy; 0x00uy; 0x00uy; 0x00uy |])

    let typePayload = ResizeArray<byte>([| 0x01uy; 0x60uy; 0x01uy; 0x7Fuy; 0x01uy; 0x7Fuy |])
    parts.Add(0x01uy)
    parts.AddRange(WasmLeb128.encodeUnsignedInt typePayload.Count)
    parts.AddRange(typePayload)

    let functionPayload = ResizeArray<byte>([| 0x01uy; 0x00uy |])
    parts.Add(0x03uy)
    parts.AddRange(WasmLeb128.encodeUnsignedInt functionPayload.Count)
    parts.AddRange(functionPayload)

    let nameBytes = Encoding.UTF8.GetBytes("square")
    let exportPayload = ResizeArray<byte>([| 0x01uy |])
    exportPayload.AddRange(WasmLeb128.encodeUnsignedInt nameBytes.Length)
    exportPayload.AddRange(nameBytes)
    exportPayload.Add(0x00uy)
    exportPayload.Add(0x00uy)
    parts.Add(0x07uy)
    parts.AddRange(WasmLeb128.encodeUnsignedInt exportPayload.Count)
    parts.AddRange(exportPayload)

    let body = ResizeArray<byte>([| 0x00uy; 0x20uy; 0x00uy; 0x20uy; 0x00uy; 0x6Cuy; 0x0Buy |])
    let codePayload = ResizeArray<byte>([| 0x01uy |])
    codePayload.AddRange(WasmLeb128.encodeUnsignedInt body.Count)
    codePayload.AddRange(body)
    parts.Add(0x0Auy)
    parts.AddRange(WasmLeb128.encodeUnsignedInt codePayload.Count)
    parts.AddRange(codePayload)

    parts.ToArray()

type WasmRuntimeTests() =
    [<Fact>]
    member _.``Has version``() =
        Assert.Equal("0.1.0", Version.VERSION)

    [<Theory>]
    [<InlineData(5, 25)>]
    [<InlineData(0, 0)>]
    [<InlineData(-3, 9)>]
    member _.``LoadAndRun executes square module``(input: int, expected: int) =
        let runtime = WasmRuntime()
        let result = runtime.LoadAndRun(buildSquareWasm (), "square", input)
        Assert.Single(result) |> ignore
        Assert.Equal(expected, result[0] :?> int)

    [<Fact>]
    member _.``Instantiate exposes function export``() =
        let runtime = WasmRuntime()
        let instance = runtime.Instantiate(buildSquareWasm ())
        Assert.True(instance.Exports.ContainsKey "square")
