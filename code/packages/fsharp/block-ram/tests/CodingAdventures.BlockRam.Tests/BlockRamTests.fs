namespace CodingAdventures.BlockRam.FSharp.Tests

open System
open Xunit
open CodingAdventures.BlockRam.FSharp

module Helpers =
    let write (ram: SinglePortRAM) address data =
        ram.Tick(0, address, data, 1) |> ignore
        ram.Tick(1, address, data, 1)

    let read (ram: SinglePortRAM) address =
        let zeros = Array.zeroCreate ram.Width
        ram.Tick(0, address, zeros, 0) |> ignore
        ram.Tick(1, address, zeros, 0)

type SRAMTests() =
    [<Fact>]
    member _.``cell holds reads and writes one bit``() =
        let cell = SRAMCell()
        Assert.Equal(0, cell.Value)
        Assert.Equal(None, cell.Read 0)
        cell.Write(0, 1)
        Assert.Equal(0, cell.Value)
        cell.Write(1, 1)
        Assert.Equal(Some 1, cell.Read 1)
        cell.Write(1, 0)
        Assert.Equal(0, cell.Value)

    [<Fact>]
    member _.``cell rejects non bits``() =
        let cell = SRAMCell()
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> cell.Read 2 |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> cell.Write(1, -1)) |> ignore

    [<Fact>]
    member _.``array stores independent rows``() =
        let memory = SRAMArray(3, 4)
        Assert.Equal((3, 4), memory.Shape)
        memory.Write(0, [| 1; 0; 1; 0 |])
        memory.Write(2, [| 0; 1; 0; 1 |])
        Assert.Equal<int>([| 1; 0; 1; 0 |], memory.Read 0)
        Assert.Equal<int>([| 0; 0; 0; 0 |], memory.Read 1)
        Assert.Equal<int>([| 0; 1; 0; 1 |], memory.Read 2)

    [<Fact>]
    member _.``array validates shape address and data``() =
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> SRAMArray(0, 1) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> SRAMArray(1, 0) |> ignore) |> ignore
        let memory = SRAMArray(2, 2)
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> memory.Read 2 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> memory.Write(0, [| 1 |])) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> memory.Write(0, null)) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> memory.Write(0, [| 0; 2 |])) |> ignore

type SinglePortRAMTests() =
    [<Fact>]
    member _.``read first returns old value and writes new value``() =
        let ram = SinglePortRAM(4, 4)
        Helpers.write ram 0 [| 1; 0; 1; 0 |] |> ignore
        Assert.Equal<int>([| 1; 0; 1; 0 |], Helpers.write ram 0 [| 0; 1; 0; 1 |])
        Assert.Equal<int>([| 0; 1; 0; 1 |], Helpers.read ram 0)
        Assert.Equal(4, ram.Depth)
        Assert.Equal(4, ram.Width)

    [<Fact>]
    member _.``write first returns new value``() =
        let ram = SinglePortRAM(2, 2, readMode = WriteFirst)
        Assert.Equal<int>([| 1; 1 |], Helpers.write ram 0 [| 1; 1 |])

    [<Fact>]
    member _.``no change retains output during write``() =
        let ram = SinglePortRAM(2, 2, readMode = NoChange)
        Assert.Equal<int>([| 0; 0 |], Helpers.read ram 0)
        Assert.Equal<int>([| 0; 0 |], Helpers.write ram 0 [| 1; 1 |])
        Assert.Equal<int>([| 1; 1 |], Helpers.read ram 0)

    [<Fact>]
    member _.``only rising edge operates and results are copies``() =
        let ram = SinglePortRAM(2, 2, readMode = WriteFirst)
        Assert.Equal<int>([| 0; 0 |], ram.Tick(0, 0, [| 1; 0 |], 1))
        let high = ram.Tick(1, 0, [| 1; 0 |], 1)
        high[0] <- 0
        Assert.Equal<int>([| 1; 0 |], ram.Tick(1, 0, [| 0; 0 |], 0))
        Assert.Equal<int>([| 1; 0 |], Helpers.read ram 0)

    [<Fact>]
    member _.``dump returns all rows``() =
        let ram = SinglePortRAM(3, 2)
        Helpers.write ram 1 [| 1; 0 |] |> ignore
        let dump = ram.Dump()
        Assert.Equal(3, dump.Length)
        Assert.Equal<int>([| 0; 0 |], dump[0])
        Assert.Equal<int>([| 1; 0 |], dump[1])

    [<Fact>]
    member _.``validates constructor and signals``() =
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> SinglePortRAM(0, 1) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> SinglePortRAM(1, 0) |> ignore) |> ignore
        let ram = SinglePortRAM(2, 2)
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> ram.Tick(2, 0, [| 0; 0 |], 0) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> ram.Tick(1, 0, [| 0; 0 |], 2) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> ram.Tick(1, 2, [| 0; 0 |], 0) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> ram.Tick(1, 0, [| 0 |], 0) |> ignore) |> ignore

type DualPortRAMTests() =
    [<Fact>]
    member _.``ports read and write different addresses together``() =
        let ram = DualPortRAM(4, 4)
        ram.Tick(0, 0, [| 1; 0; 0; 0 |], 1, 1, [| 0; 1; 0; 0 |], 1) |> ignore
        ram.Tick(1, 0, [| 1; 0; 0; 0 |], 1, 1, [| 0; 1; 0; 0 |], 1) |> ignore
        ram.Tick(0, 0, Array.zeroCreate 4, 0, 1, Array.zeroCreate 4, 0) |> ignore
        let dataA, dataB = ram.Tick(1, 0, Array.zeroCreate 4, 0, 1, Array.zeroCreate 4, 0)
        Assert.Equal<int>([| 1; 0; 0; 0 |], dataA)
        Assert.Equal<int>([| 0; 1; 0; 0 |], dataB)
        Assert.Equal(4, ram.Depth)
        Assert.Equal(4, ram.Width)

    [<Fact>]
    member _.``collision reports address``() =
        let ram = DualPortRAM(2, 2)
        ram.Tick(0, 0, [| 1; 0 |], 1, 0, [| 0; 1 |], 1) |> ignore
        let error =
            Assert.Throws<WriteCollisionException>(fun () ->
                ram.Tick(1, 0, [| 1; 0 |], 1, 0, [| 0; 1 |], 1) |> ignore)
        Assert.Equal(0, error.Address)
        Assert.Contains("address 0", error.Message)

    [<Fact>]
    member _.``per port read modes are independent``() =
        let ram = DualPortRAM(4, 2, readModeA = NoChange, readModeB = WriteFirst)
        ram.Tick(0, 0, [| 1; 1 |], 1, 1, [| 1; 0 |], 1) |> ignore
        let dataA, dataB = ram.Tick(1, 0, [| 1; 1 |], 1, 1, [| 1; 0 |], 1)
        Assert.Equal<int>([| 0; 0 |], dataA)
        Assert.Equal<int>([| 1; 0 |], dataB)
        ram.Tick(0, 0, [| 0; 0 |], 0, 1, [| 0; 1 |], 1) |> ignore
        let dataA, dataB = ram.Tick(1, 0, [| 0; 0 |], 0, 1, [| 0; 1 |], 1)
        Assert.Equal<int>([| 1; 1 |], dataA)
        Assert.Equal<int>([| 0; 1 |], dataB)

    [<Fact>]
    member _.``read first port returns old data``() =
        let ram = DualPortRAM(2, 2)
        ram.Tick(0, 0, [| 1; 1 |], 1, 1, Array.zeroCreate 2, 0) |> ignore
        ram.Tick(1, 0, [| 1; 1 |], 1, 1, Array.zeroCreate 2, 0) |> ignore
        ram.Tick(0, 0, [| 0; 0 |], 1, 1, Array.zeroCreate 2, 0) |> ignore
        let dataA, _ = ram.Tick(1, 0, [| 0; 0 |], 1, 1, Array.zeroCreate 2, 0)
        Assert.Equal<int>([| 1; 1 |], dataA)

    [<Fact>]
    member _.``validates constructor and port inputs``() =
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> DualPortRAM(0, 1) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> DualPortRAM(1, 0) |> ignore) |> ignore
        let ram = DualPortRAM(2, 2)
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> ram.Tick(2, 0, [| 0; 0 |], 0, 0, [| 0; 0 |], 0) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> ram.Tick(1, 0, [| 0; 0 |], 2, 0, [| 0; 0 |], 0) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> ram.Tick(1, 0, [| 0; 0 |], 0, 0, [| 0; 0 |], -1) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> ram.Tick(1, -1, [| 0; 0 |], 0, 0, [| 0; 0 |], 0) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> ram.Tick(1, 0, [| 0; 0 |], 0, 2, [| 0; 0 |], 0) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> ram.Tick(1, 0, [| 0 |], 0, 0, [| 0; 0 |], 0) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> ram.Tick(1, 0, [| 0; 0 |], 0, 0, [| 0 |], 0) |> ignore) |> ignore

type ConfigurableBRAMTests() =
    [<Fact>]
    member _.``defaults to an eighteen kilobit eight wide block``() =
        let bram = ConfigurableBRAM()
        Assert.Equal(18_432, bram.TotalBits)
        Assert.Equal(8, bram.Width)
        Assert.Equal(2_304, bram.Depth)

    [<Fact>]
    member _.``ports share storage``() =
        let bram = ConfigurableBRAM(totalBits = 64, width = 4)
        bram.TickA(0, 3, [| 1; 0; 1; 1 |], 1) |> ignore
        bram.TickA(1, 3, [| 1; 0; 1; 1 |], 1) |> ignore
        bram.TickB(0, 3, Array.zeroCreate 4, 0) |> ignore
        Assert.Equal<int>([| 1; 0; 1; 1 |], bram.TickB(1, 3, Array.zeroCreate 4, 0))

    [<Fact>]
    member _.``reconfigure changes shape and clears storage``() =
        let bram = ConfigurableBRAM(totalBits = 64, width = 4)
        bram.TickB(0, 0, [| 1; 1; 1; 1 |], 1) |> ignore
        bram.TickB(1, 0, [| 1; 1; 1; 1 |], 1) |> ignore
        bram.Reconfigure 8
        Assert.Equal(8, bram.Width)
        Assert.Equal(8, bram.Depth)
        bram.TickA(0, 0, Array.zeroCreate 8, 0) |> ignore
        Assert.Equal<int>(Array.zeroCreate 8, bram.TickA(1, 0, Array.zeroCreate 8, 0))

    [<Fact>]
    member _.``validates configurations``() =
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> ConfigurableBRAM(totalBits = 0, width = 1) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> ConfigurableBRAM(totalBits = 8, width = 0) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> ConfigurableBRAM(totalBits = 8, width = 3) |> ignore) |> ignore
        let bram = ConfigurableBRAM(totalBits = 8, width = 2)
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> bram.Reconfigure 0) |> ignore
        Assert.Throws<ArgumentException>(fun () -> bram.Reconfigure 3) |> ignore
