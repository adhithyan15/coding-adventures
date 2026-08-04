namespace CodingAdventures.Fpga.FSharp.Tests

open System
open System.Collections.Generic
open System.Text.Json
open Xunit
open CodingAdventures.Fpga.FSharp

module Helpers =
    let zeros k = Array.zeroCreate (1 <<< k)

    let andTable k =
        Array.init (1 <<< k) (fun index -> if index &&& 3 = 3 then 1 else 0)

    let xorTable k =
        Array.init (1 <<< k) (fun index -> (index &&& 1) ^^^ ((index >>> 1) &&& 1))

    let sliceConfig lutA lutB =
        { LutA = lutA
          LutB = lutB
          FlipFlopAEnabled = false
          FlipFlopBEnabled = false
          CarryEnabled = false }

    let bitstream () =
        let slice0 = sliceConfig (andTable 4) (zeros 4)
        let slice1 = sliceConfig (zeros 4) (zeros 4)

        Bitstream(
            Map.ofList [ "clb0", { Slice0 = slice0; Slice1 = slice1 } ],
            Map.ofList
                [ "switch0",
                  [ { Source = "clb_out"; Destination = "east" }
                    { Source = "north"; Destination = "south" } ] ],
            Map.ofList
                [ "in", { Mode = "input" }
                  "out", { Mode = "output" }
                  "tri", { Mode = "tristate" } ],
            4
        )

type LUTTests() =
    [<Fact>]
    member _.``implements AND truth table``() =
        let lut = LUT(4, Helpers.andTable 4)
        Assert.Equal(0, lut.Evaluate [| 0; 0; 0; 0 |])
        Assert.Equal(0, lut.Evaluate [| 1; 0; 0; 0 |])
        Assert.Equal(0, lut.Evaluate [| 0; 1; 0; 0 |])
        Assert.Equal(1, lut.Evaluate [| 1; 1; 0; 0 |])

    [<Fact>]
    member _.``implements XOR truth table``() =
        let lut = LUT(4, Helpers.xorTable 4)
        Assert.Equal(1, lut.Evaluate [| 1; 0; 0; 0 |])
        Assert.Equal(1, lut.Evaluate [| 0; 1; 0; 0 |])
        Assert.Equal(0, lut.Evaluate [| 1; 1; 0; 0 |])

    [<Fact>]
    member _.``defaults to zeros``() =
        let lut = LUT(2)
        Assert.Equal(2, lut.K)

        [| [| 0; 0 |]; [| 1; 0 |]; [| 0; 1 |]; [| 1; 1 |] |]
        |> Array.iter (fun inputs -> Assert.Equal(0, lut.Evaluate inputs))

    [<Fact>]
    member _.``can be reconfigured``() =
        let lut = LUT(4, Helpers.andTable 4)
        lut.Configure(Helpers.xorTable 4)
        Assert.Equal(0, lut.Evaluate [| 1; 1; 0; 0 |])
        Assert.Equal(1, lut.Evaluate [| 1; 0; 0; 0 |])

    [<Fact>]
    member _.``truth tables are defensively copied``() =
        let source = Helpers.andTable 4
        let lut = LUT(4, source)
        source[3] <- 0
        let snapshot = lut.TruthTable
        snapshot[3] <- 0
        Assert.Equal(1, lut.Evaluate [| 1; 1; 0; 0 |])

    [<Fact>]
    member _.``rejects invalid widths``() =
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> LUT(1) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> LUT(7) |> ignore) |> ignore

    [<Fact>]
    member _.``rejects invalid tables``() =
        Assert.Throws<ArgumentException>(fun () -> LUT(4).Configure [| 0; 1 |]) |> ignore
        let table = Helpers.zeros 4
        table[5] <- 2
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> LUT(4, table) |> ignore) |> ignore

    [<Fact>]
    member _.``rejects invalid inputs``() =
        let lut = LUT(4)
        Assert.Throws<ArgumentException>(fun () -> lut.Evaluate [| 0; 1 |] |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> lut.Evaluate [| 0; 0; -1; 0 |] |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> lut.Evaluate null |> ignore) |> ignore

type SliceAndCLBTests() =
    [<Fact>]
    member _.``slice evaluates independent combinational LUTs``() =
        let slice = Slice()
        slice.Configure(Helpers.andTable 4, Helpers.xorTable 4)
        let result = slice.Evaluate([| 1; 1; 0; 0 |], [| 1; 0; 0; 0 |], 0)
        Assert.Equal({ OutputA = 1; OutputB = 1; CarryOut = 0 }, result)

    [<Fact>]
    member _.``slice registers outputs across high then low clock``() =
        let slice = Slice()
        slice.Configure(Helpers.andTable 4, Helpers.andTable 4, enableFlipFlopA = true, enableFlipFlopB = true)
        let ones = [| 1; 1; 0; 0 |]
        Assert.Equal({ OutputA = 0; OutputB = 0; CarryOut = 0 }, slice.Evaluate(ones, ones, 1))
        Assert.Equal({ OutputA = 1; OutputB = 1; CarryOut = 0 }, slice.Evaluate(ones, ones, 0))

    [<Fact>]
    member _.``slice reconfiguration resets registers``() =
        let slice = Slice()
        let ones = [| 1; 1; 0; 0 |]
        slice.Configure(Helpers.andTable 4, Helpers.andTable 4, enableFlipFlopA = true, enableFlipFlopB = true)
        slice.Evaluate(ones, ones, 1) |> ignore
        slice.Evaluate(ones, ones, 0) |> ignore
        slice.Configure(Helpers.andTable 4, Helpers.andTable 4, enableFlipFlopA = true, enableFlipFlopB = true)
        Assert.Equal(0, slice.Evaluate(ones, ones, 1).OutputA)

    [<Fact>]
    member _.``slice carry chain generates propagates and blocks``() =
        let slice = Slice()
        slice.Configure(Helpers.andTable 4, Helpers.andTable 4, enableCarry = true)
        let ones = [| 1; 1; 0; 0 |]
        let zeros = [| 0; 0; 0; 0 |]
        Assert.Equal(1, slice.Evaluate(ones, ones, 0).CarryOut)
        Assert.Equal(1, slice.Evaluate(ones, zeros, 0, carryIn = 1).CarryOut)
        Assert.Equal(0, slice.Evaluate(zeros, zeros, 0, carryIn = 1).CarryOut)

    [<Fact>]
    member _.``slice validates clock and carry``() =
        let slice = Slice()
        let zeros = [| 0; 0; 0; 0 |]
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> slice.Evaluate(zeros, zeros, 2) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> slice.Evaluate(zeros, zeros, 0, carryIn = -1) |> ignore) |> ignore

    [<Fact>]
    member _.``CLB evaluates slices independently``() =
        let clb = CLB()
        clb.Slice0.Configure(Helpers.andTable 4, Helpers.andTable 4)
        clb.Slice1.Configure(Helpers.andTable 4, Helpers.andTable 4)
        let result = clb.Evaluate([| 1; 1; 0; 0 |], [| 1; 1; 0; 0 |], [| 0; 1; 0; 0 |], [| 1; 0; 0; 0 |], 0)
        Assert.Equal(1, result.Slice0.OutputA)
        Assert.Equal(0, result.Slice1.OutputA)
        Assert.Equal(4, clb.K)

    [<Fact>]
    member _.``CLB chains carry between slices``() =
        let clb = CLB()
        clb.Slice0.Configure(Helpers.andTable 4, Helpers.andTable 4, enableCarry = true)
        clb.Slice1.Configure(Helpers.andTable 4, Helpers.andTable 4, enableCarry = true)
        let ones = [| 1; 1; 0; 0 |]
        let result = clb.Evaluate(ones, ones, ones, ones, 0)
        Assert.Equal(1, result.Slice0.CarryOut)
        Assert.Equal(1, result.Slice1.CarryOut)

type SwitchMatrixTests() =
    [<Fact>]
    member _.``routes connected signals``() =
        let matrix = SwitchMatrix [ "north"; "south"; "east"; "out" ]
        matrix.Connect("out", "east")
        matrix.Connect("north", "south")
        let routed = matrix.Route(Map.ofList [ "out", 1; "north", 0 ])
        Assert.Equal(1, routed["east"])
        Assert.Equal(0, routed["south"])

    [<Fact>]
    member _.``supports fan out``() =
        let matrix = SwitchMatrix [ "source"; "a"; "b" ]
        matrix.Connect("source", "a")
        matrix.Connect("source", "b")
        let routed = matrix.Route(Map.ofList [ "source", 1 ])
        Assert.Equal(1, routed["a"])
        Assert.Equal(1, routed["b"])

    [<Fact>]
    member _.``omits destinations without source values``() =
        let matrix = SwitchMatrix [ "a"; "b" ]
        matrix.Connect("a", "b")
        Assert.Empty(matrix.Route(Map.ofList [ "b", 1 ]))

    [<Fact>]
    member _.``disconnects and clears``() =
        let matrix = SwitchMatrix [ "a"; "b"; "c" ]
        matrix.Connect("a", "b")
        matrix.Disconnect("b")
        Assert.Equal(0, matrix.ConnectionCount)
        matrix.Connect("a", "b")
        matrix.Connect("a", "c")
        matrix.Clear()
        Assert.Empty(matrix.Connections)

    [<Fact>]
    member _.``exposes port and connection snapshots``() =
        let matrix = SwitchMatrix [ "a"; "b" ]
        matrix.Connect("a", "b")
        Assert.Equal(2, matrix.Ports.Count)
        Assert.Equal("a", matrix.Connections["b"])

    [<Fact>]
    member _.``rejects invalid port sets``() =
        Assert.Throws<ArgumentNullException>(fun () -> SwitchMatrix null |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> SwitchMatrix [] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> SwitchMatrix [ "" ] |> ignore) |> ignore

    [<Fact>]
    member _.``rejects invalid connections``() =
        let matrix = SwitchMatrix [ "a"; "b"; "c" ]
        Assert.Throws<ArgumentException>(fun () -> matrix.Connect("x", "a")) |> ignore
        Assert.Throws<ArgumentException>(fun () -> matrix.Connect("a", "x")) |> ignore
        Assert.Throws<ArgumentException>(fun () -> matrix.Connect("a", "a")) |> ignore
        matrix.Connect("a", "b")
        Assert.Throws<InvalidOperationException>(fun () -> matrix.Connect("c", "b")) |> ignore

    [<Fact>]
    member _.``rejects invalid disconnects and bits``() =
        let matrix = SwitchMatrix [ "a"; "b"; "c" ]
        Assert.Throws<ArgumentException>(fun () -> matrix.Disconnect("x")) |> ignore
        Assert.Throws<InvalidOperationException>(fun () -> matrix.Disconnect("c")) |> ignore
        matrix.Connect("a", "b")
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> matrix.Route(Map.ofList [ "a", 2 ]) |> ignore) |> ignore

type IOBlockTests() =
    [<Fact>]
    member _.``input mode reads pad``() =
        let io = IOBlock("sensor")
        io.DrivePad 1
        Assert.Equal(1, io.ReadInternal())
        Assert.Equal(Some 1, io.ReadPad())

    [<Fact>]
    member _.``output mode drives pad``() =
        let io = IOBlock("led", Output)
        io.DriveInternal 1
        Assert.Equal(1, io.ReadInternal())
        Assert.Equal(Some 1, io.ReadPad())

    [<Fact>]
    member _.``can be reconfigured to tristate``() =
        let io = IOBlock("bus", Output)
        io.DriveInternal 1
        io.Configure Tristate
        Assert.Equal(None, io.ReadPad())
        Assert.Equal(Tristate, io.Mode)

    [<Fact>]
    member _.``validates name and bits``() =
        Assert.Throws<ArgumentException>(fun () -> IOBlock(" ") |> ignore) |> ignore
        let io = IOBlock("pin")
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> io.DrivePad 2) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> io.DriveInternal -1) |> ignore
        Assert.Equal("pin", io.Name)

type BitstreamAndFabricTests() =
    [<Fact>]
    member _.``empty bitstream uses requested LUT width``() =
        let bitstream = Bitstream.Empty(lutK = 3)
        Assert.Equal(3, bitstream.LutK)
        Assert.Empty(bitstream.Clbs)
        Assert.Empty(bitstream.Routing)
        Assert.Empty(bitstream.IO)

    [<Fact>]
    member _.``bitstream defensively copies tables``() =
        let table = Helpers.andTable 4
        let slice = Helpers.sliceConfig table (Helpers.zeros 4)
        let bitstream = Bitstream(Map.ofList [ "c", { Slice0 = slice; Slice1 = slice } ], Map.empty, Map.empty, 4)
        table[3] <- 0
        Assert.Equal(1, bitstream.Clbs["c"].Slice0.LutA[3])

    [<Fact>]
    member _.``bitstream rejects invalid LUT widths``() =
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> Bitstream.Empty(lutK = 8) |> ignore) |> ignore

    [<Fact>]
    member _.``JSON parses all configuration sections``() =
        let bitstream =
            Bitstream.ParseJson(
                """{"lut_k":2,"clbs":{"c":{"slice0":{"lut_a":[0,0,0,1],"ff_a":true}}},"routing":{"s":[{"src":"a","dst":"b"}]},"io":{"in":{"mode":"input"},"out":{"mode":"output"}}}"""
            )

        Assert.Equal(2, bitstream.LutK)
        Assert.True(bitstream.Clbs["c"].Slice0.FlipFlopAEnabled)
        Assert.Equal(1, bitstream.Clbs["c"].Slice0.LutA[3])
        Assert.Equal("b", bitstream.Routing["s"].Head.Destination)
        Assert.Equal("output", bitstream.IO["out"].Mode)

    [<Fact>]
    member _.``JSON supplies missing defaults``() =
        let bitstream = Bitstream.ParseJson("""{"clbs":{"c":{"slice0":{"ff_b":true}}}}""")
        Assert.Equal(4, bitstream.LutK)
        Assert.Equal(16, bitstream.Clbs["c"].Slice0.LutA.Length)
        Assert.Equal(16, bitstream.Clbs["c"].Slice1.LutB.Length)
        Assert.True(bitstream.Clbs["c"].Slice0.FlipFlopBEnabled)

    [<Fact>]
    member _.``JSON rejects malformed documents``() =
        Assert.Throws<ArgumentNullException>(fun () -> Bitstream.ParseJson null |> ignore) |> ignore
        Assert.Throws<JsonException>(fun () -> Bitstream.ParseJson "[]" |> ignore) |> ignore
        Assert.Throws<JsonException>(fun () -> Bitstream.ParseJson "{\"lut_k\":9}" |> ignore) |> ignore
        Assert.ThrowsAny<JsonException>(fun () -> Bitstream.ParseJson "{invalid" |> ignore) |> ignore

    [<Fact>]
    member _.``fabric evaluates configured CLBs``() =
        let fpga = FPGA(Helpers.bitstream ())
        let zeros = [| 0; 0; 0; 0 |]
        let result = fpga.EvaluateCLB("clb0", [| 1; 1; 0; 0 |], zeros, zeros, zeros, 0)
        Assert.Equal(1, result.Slice0.OutputA)
        Assert.Single(fpga.Clbs) |> ignore

    [<Fact>]
    member _.``fabric routes configured signals``() =
        let fpga = FPGA(Helpers.bitstream ())
        let routed = fpga.Route("switch0", Map.ofList [ "clb_out", 1; "north", 0 ])
        Assert.Equal(1, routed["east"])
        Assert.Equal(0, routed["south"])
        Assert.Single(fpga.Switches) |> ignore

    [<Fact>]
    member _.``fabric drives input output and tristate pins``() =
        let fpga = FPGA(Helpers.bitstream ())
        fpga.SetInput("in", 1)
        Assert.Equal(Some 1, fpga.ReadOutput "in")
        fpga.DriveOutput("out", 1)
        Assert.Equal(Some 1, fpga.ReadOutput "out")
        Assert.Equal(None, fpga.ReadOutput "tri")
        Assert.Equal(3, fpga.IOBlocks.Count)

    [<Fact>]
    member _.``fabric rejects unknown resources``() =
        let fpga = FPGA(Helpers.bitstream ())
        let zeros = [| 0; 0; 0; 0 |]
        Assert.Throws<KeyNotFoundException>(fun () -> fpga.EvaluateCLB("missing", zeros, zeros, zeros, zeros, 0) |> ignore) |> ignore
        Assert.Throws<KeyNotFoundException>(fun () -> fpga.Route("missing", Map.empty) |> ignore) |> ignore
        Assert.Throws<KeyNotFoundException>(fun () -> fpga.SetInput("missing", 0)) |> ignore
        Assert.Throws<KeyNotFoundException>(fun () -> fpga.DriveOutput("missing", 0)) |> ignore
        Assert.Throws<KeyNotFoundException>(fun () -> fpga.ReadOutput "missing" |> ignore) |> ignore

    [<Fact>]
    member _.``fabric accepts an empty bitstream``() =
        let bitstream = Bitstream.Empty()
        let fpga = FPGA(bitstream)
        Assert.Same(bitstream, fpga.Bitstream)
        Assert.Empty(fpga.Clbs)
        Assert.Empty(fpga.Switches)
        Assert.Empty(fpga.IOBlocks)
        Assert.Throws<ArgumentNullException>(fun () -> FPGA(null) |> ignore) |> ignore
