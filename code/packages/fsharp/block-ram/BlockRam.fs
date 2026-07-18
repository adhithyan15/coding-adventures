namespace CodingAdventures.BlockRam.FSharp

open System

module internal Validation =
    let bit name value =
        if value <> 0 && value <> 1 then
            raise (ArgumentOutOfRangeException(name, value, $"{name} must be 0 or 1"))

    let data name width (values: int array) =
        if isNull values then
            nullArg name

        if values.Length <> width then
            raise (ArgumentException($"{name} length {values.Length} does not match width {width}", name))

        values
        |> Array.iteri (fun index value -> bit $"{name}[{index}]" value)

        Array.copy values

/// A single-bit static RAM cell.
type SRAMCell() =
    let mutable value = 0

    member _.Value = value

    member _.Read(wordLine: int) =
        Validation.bit "wordLine" wordLine
        if wordLine = 1 then Some value else None

    member _.Write(wordLine: int, bitLine: int) =
        Validation.bit "wordLine" wordLine
        Validation.bit "bitLine" bitLine

        if wordLine = 1 then
            value <- bitLine

/// A rectangular zero-initialized array of SRAM cells.
type SRAMArray(rows: int, cols: int) =
    do
        if rows < 1 then
            raise (ArgumentOutOfRangeException("rows", rows, "rows must be >= 1"))

        if cols < 1 then
            raise (ArgumentOutOfRangeException("cols", cols, "cols must be >= 1"))

    let cells = Array.init rows (fun _ -> Array.init cols (fun _ -> SRAMCell()))

    let validateRow row =
        if row < 0 || row >= rows then
            raise (ArgumentOutOfRangeException("row", row, $"row {row} out of range [0, {rows - 1}]"))

    member _.Rows = rows
    member _.Cols = cols
    member _.Shape = rows, cols

    member _.Read(row: int) =
        validateRow row

        cells[row]
        |> Array.map (fun cell -> cell.Read(1) |> Option.get)

    member _.Write(row: int, data: int array) =
        validateRow row
        let bits = Validation.data "data" cols data
        Array.iter2 (fun (cell: SRAMCell) bit -> cell.Write(1, bit)) cells[row] bits

/// Controls the value exposed by a RAM port during writes.
type ReadMode =
    | ReadFirst
    | WriteFirst
    | NoChange

/// Raised when both ports write the same address on one rising edge.
type WriteCollisionException(address: int) =
    inherit InvalidOperationException($"Write collision: both ports writing to address {address}")
    member _.Address = address

/// A synchronous single-port RAM.
type SinglePortRAM(depth: int, width: int, ?readMode: ReadMode) =
    do
        if depth < 1 then
            raise (ArgumentOutOfRangeException("depth", depth, "depth must be >= 1"))

        if width < 1 then
            raise (ArgumentOutOfRangeException("width", width, "width must be >= 1"))

    let readMode = defaultArg readMode ReadFirst
    let memory = SRAMArray(depth, width)
    let mutable previousClock = 0
    let mutable lastRead = Array.zeroCreate width

    let validateAddress address =
        if address < 0 || address >= depth then
            raise (ArgumentOutOfRangeException("address", address, $"address {address} out of range [0, {depth - 1}]"))

    member _.Depth = depth
    member _.Width = width

    member _.Tick(clock: int, address: int, dataIn: int array, writeEnable: int) =
        Validation.bit "clock" clock
        Validation.bit "writeEnable" writeEnable
        validateAddress address
        let data = Validation.data "dataIn" width dataIn
        let risingEdge = previousClock = 0 && clock = 1
        previousClock <- clock

        if risingEdge then
            if writeEnable = 0 then
                lastRead <- memory.Read address
            else
                match readMode with
                | ReadFirst ->
                    lastRead <- memory.Read address
                    memory.Write(address, data)
                | WriteFirst ->
                    memory.Write(address, data)
                    lastRead <- data
                | NoChange -> memory.Write(address, data)

        Array.copy lastRead

    member _.Dump() =
        Array.init depth memory.Read

/// A synchronous true dual-port RAM with collision detection.
type DualPortRAM(depth: int, width: int, ?readModeA: ReadMode, ?readModeB: ReadMode) =
    do
        if depth < 1 then
            raise (ArgumentOutOfRangeException("depth", depth, "depth must be >= 1"))

        if width < 1 then
            raise (ArgumentOutOfRangeException("width", width, "width must be >= 1"))

    let readModeA = defaultArg readModeA ReadFirst
    let readModeB = defaultArg readModeB ReadFirst
    let memory = SRAMArray(depth, width)
    let mutable previousClock = 0
    let mutable lastReadA = Array.zeroCreate width
    let mutable lastReadB = Array.zeroCreate width

    let validateAddress name address =
        if address < 0 || address >= depth then
            raise (ArgumentOutOfRangeException(name, address, $"address {address} out of range [0, {depth - 1}]"))

    let processPort address data writeEnable mode lastRead =
        if writeEnable = 0 then
            memory.Read address
        else
            match mode with
            | ReadFirst ->
                let oldData = memory.Read address
                memory.Write(address, data)
                oldData
            | WriteFirst ->
                memory.Write(address, data)
                data
            | NoChange ->
                memory.Write(address, data)
                Array.copy lastRead

    member _.Depth = depth
    member _.Width = width

    member _.Tick(
        clock: int,
        addressA: int,
        dataInA: int array,
        writeEnableA: int,
        addressB: int,
        dataInB: int array,
        writeEnableB: int
    ) =
        Validation.bit "clock" clock
        Validation.bit "writeEnableA" writeEnableA
        Validation.bit "writeEnableB" writeEnableB
        validateAddress "addressA" addressA
        validateAddress "addressB" addressB
        let dataA = Validation.data "dataInA" width dataInA
        let dataB = Validation.data "dataInB" width dataInB
        let risingEdge = previousClock = 0 && clock = 1
        previousClock <- clock

        if risingEdge then
            if writeEnableA = 1 && writeEnableB = 1 && addressA = addressB then
                raise (WriteCollisionException(addressA))

            lastReadA <- processPort addressA dataA writeEnableA readModeA lastReadA
            lastReadB <- processPort addressB dataB writeEnableB readModeB lastReadB

        Array.copy lastReadA, Array.copy lastReadB

/// An FPGA-style dual-port block RAM with configurable aspect ratio.
type ConfigurableBRAM(?totalBits: int, ?width: int) =
    let totalBits = defaultArg totalBits 18_432
    let mutable width = defaultArg width 8

    let validateConfiguration width =
        if totalBits < 1 then
            raise (ArgumentOutOfRangeException("totalBits", totalBits, "totalBits must be >= 1"))

        if width < 1 then
            raise (ArgumentOutOfRangeException("width", width, "width must be >= 1"))

        if totalBits % width <> 0 then
            raise (ArgumentException($"width {width} does not evenly divide totalBits {totalBits}", "width"))

    do validateConfiguration width

    let mutable depth = totalBits / width
    let mutable ram = DualPortRAM(depth, width)

    member _.TotalBits = totalBits
    member _.Width = width
    member _.Depth = depth

    member _.Reconfigure(newWidth: int) =
        validateConfiguration newWidth
        width <- newWidth
        depth <- totalBits / width
        ram <- DualPortRAM(depth, width)

    member _.TickA(clock: int, address: int, dataIn: int array, writeEnable: int) =
        let dataOutA, _ = ram.Tick(clock, address, dataIn, writeEnable, 0, Array.zeroCreate width, 0)
        dataOutA

    member _.TickB(clock: int, address: int, dataIn: int array, writeEnable: int) =
        let _, dataOutB = ram.Tick(clock, 0, Array.zeroCreate width, 0, address, dataIn, writeEnable)
        dataOutB
