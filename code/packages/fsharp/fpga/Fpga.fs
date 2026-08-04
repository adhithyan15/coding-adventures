namespace CodingAdventures.Fpga.FSharp

open System
open System.Collections.Generic
open System.Text.Json
open CodingAdventures.BlockRam.FSharp
open CodingAdventures.LogicGates

module internal Validation =
    let bit name value =
        if value <> 0 && value <> 1 then
            raise (ArgumentOutOfRangeException(name, value, $"{name} must be 0 or 1"))

    let bits name length (values: int array) =
        if isNull values then
            nullArg name

        if values.Length <> length then
            raise (ArgumentException($"{name} length {values.Length} does not match {length}", name))

        values |> Array.iteri (fun index value -> bit $"{name}[{index}]" value)
        Array.copy values

/// A K-input lookup table backed by SRAM cells.
type LUT(k: int, ?truthTable: int array) =
    do
        if k < 2 || k > 6 then
            raise (ArgumentOutOfRangeException("k", k, "k must be between 2 and 6"))

    let cells = Array.init (1 <<< k) (fun _ -> SRAMCell())

    do
        match truthTable with
        | Some values ->
            let validated = Validation.bits "truthTable" cells.Length values
            Array.iter2 (fun (cell: SRAMCell) value -> cell.Write(1, value)) cells validated
        | None -> ()

    member _.K = k
    member _.TruthTable = cells |> Array.map (fun cell -> cell.Read(1) |> Option.get)

    member _.Configure(truthTable: int array) =
        let values = Validation.bits "truthTable" cells.Length truthTable
        Array.iter2 (fun (cell: SRAMCell) value -> cell.Write(1, value)) cells values

    member _.Evaluate(inputs: int array) =
        let values = Validation.bits "inputs" k inputs
        let tableIndex = values |> Array.mapi (fun index value -> value <<< index) |> Array.sum
        cells[tableIndex].Read(1) |> Option.get

type SliceOutput =
    { OutputA: int
      OutputB: int
      CarryOut: int }

type private FlipFlop() =
    let mutable master = 0
    let mutable output = 0

    member _.Evaluate(data: int, clock: int) =
        Validation.bit "data" data
        Validation.bit "clock" clock

        if clock = 1 then
            master <- data
        else
            output <- master

        output

/// Two LUTs, optional registers, and a carry chain.
type Slice(lutInputs: int) =
    let lutA = LUT(lutInputs)
    let lutB = LUT(lutInputs)
    let mutable flipFlopA = FlipFlop()
    let mutable flipFlopB = FlipFlop()
    let mutable flipFlopAEnabled = false
    let mutable flipFlopBEnabled = false
    let mutable carryEnabled = false

    new() = Slice(4)

    member _.LutA = lutA
    member _.LutB = lutB
    member _.K = lutInputs

    member _.Configure(
        lutATable: int array,
        lutBTable: int array,
        ?enableFlipFlopA: bool,
        ?enableFlipFlopB: bool,
        ?enableCarry: bool
    ) =
        lutA.Configure lutATable
        lutB.Configure lutBTable
        flipFlopAEnabled <- defaultArg enableFlipFlopA false
        flipFlopBEnabled <- defaultArg enableFlipFlopB false
        carryEnabled <- defaultArg enableCarry false
        flipFlopA <- FlipFlop()
        flipFlopB <- FlipFlop()

    member _.Evaluate(inputsA: int array, inputsB: int array, clock: int, ?carryIn: int) =
        let carryIn = defaultArg carryIn 0
        Validation.bit "clock" clock
        Validation.bit "carryIn" carryIn
        let valueA = lutA.Evaluate inputsA
        let valueB = lutB.Evaluate inputsB
        let outputA = if flipFlopAEnabled then flipFlopA.Evaluate(valueA, clock) else valueA
        let outputB = if flipFlopBEnabled then flipFlopB.Evaluate(valueB, clock) else valueB

        let carryOut =
            if carryEnabled then
                LogicGates.orGate
                    (LogicGates.andGate valueA valueB)
                    (LogicGates.andGate carryIn (LogicGates.xorGate valueA valueB))
            else
                0

        { OutputA = outputA
          OutputB = outputB
          CarryOut = carryOut }

type CLBOutput =
    { Slice0: SliceOutput
      Slice1: SliceOutput }

/// A configurable logic block containing two slices.
type CLB(lutInputs: int) =
    let slice0 = Slice(lutInputs)
    let slice1 = Slice(lutInputs)

    new() = CLB(4)

    member _.Slice0 = slice0
    member _.Slice1 = slice1
    member _.K = lutInputs

    member _.Evaluate(
        slice0InputsA: int array,
        slice0InputsB: int array,
        slice1InputsA: int array,
        slice1InputsB: int array,
        clock: int,
        ?carryIn: int
    ) =
        let first = slice0.Evaluate(slice0InputsA, slice0InputsB, clock, defaultArg carryIn 0)
        let second = slice1.Evaluate(slice1InputsA, slice1InputsB, clock, first.CarryOut)
        { Slice0 = first; Slice1 = second }

/// A programmable crossbar with one driver per destination.
type SwitchMatrix(ports: seq<string>) =
    do
        if isNull (box ports) then
            nullArg "ports"

    let ports = HashSet<string>(ports, StringComparer.Ordinal)
    let connections = Dictionary<string, string>(StringComparer.Ordinal)

    do
        if ports.Count = 0 || ports |> Seq.exists String.IsNullOrWhiteSpace then
            invalidArg "ports" "ports must contain non-empty unique names"

    member _.Ports = ports |> Set.ofSeq
    member _.Connections = connections |> Seq.map (fun pair -> pair.Key, pair.Value) |> Map.ofSeq
    member _.ConnectionCount = connections.Count

    member _.Connect(source: string, destination: string) =
        if not (ports.Contains source) then
            invalidArg "source" $"unknown source port: {source}"

        if not (ports.Contains destination) then
            invalidArg "destination" $"unknown destination port: {destination}"

        if source = destination then
            invalidArg "destination" "a port cannot connect to itself"

        if not (connections.TryAdd(destination, source)) then
            raise (InvalidOperationException($"destination {destination} is already connected"))

    member _.Disconnect(destination: string) =
        if not (ports.Contains destination) then
            invalidArg "destination" $"unknown port: {destination}"

        if not (connections.Remove destination) then
            raise (InvalidOperationException($"port {destination} is not connected"))

    member _.Clear() = connections.Clear()

    member _.Route(inputs: Map<string, int>) =
        if isNull (box inputs) then
            nullArg "inputs"

        connections
        |> Seq.choose (fun pair ->
            inputs
            |> Map.tryFind pair.Value
            |> Option.map (fun value ->
                Validation.bit $"inputs[{pair.Value}]" value
                pair.Key, value))
        |> Map.ofSeq

type IOMode =
    | Input
    | Output
    | Tristate

/// A configurable external I/O pad.
type IOBlock(name: string, ?mode: IOMode) =
    do
        if String.IsNullOrWhiteSpace name then
            invalidArg "name" "name must be non-empty"

    let mutable mode = defaultArg mode Input
    let mutable padValue = 0
    let mutable internalValue = 0

    member _.Name = name
    member _.Mode = mode
    member _.Configure(newMode: IOMode) = mode <- newMode

    member _.DrivePad(value: int) =
        Validation.bit "value" value
        padValue <- value

    member _.DriveInternal(value: int) =
        Validation.bit "value" value
        internalValue <- value

    member _.ReadInternal() = if mode = Input then padValue else internalValue

    member _.ReadPad() =
        match mode with
        | Input -> Some padValue
        | Output -> Some internalValue
        | Tristate -> None

type SliceConfig =
    { LutA: int array
      LutB: int array
      FlipFlopAEnabled: bool
      FlipFlopBEnabled: bool
      CarryEnabled: bool }

type CLBConfig = { Slice0: SliceConfig; Slice1: SliceConfig }
type RouteConfig = { Source: string; Destination: string }
type IOConfig = { Mode: string }

/// Immutable FPGA configuration data.
type Bitstream(clbs: Map<string, CLBConfig>, routing: Map<string, RouteConfig list>, io: Map<string, IOConfig>, lutK: int) =
    do
        if lutK < 2 || lutK > 6 then
            raise (ArgumentOutOfRangeException("lutK", lutK, "lutK must be between 2 and 6"))

    let copySlice config =
        { config with
            LutA = Array.copy config.LutA
            LutB = Array.copy config.LutB }

    let clbs = clbs |> Map.map (fun _ config -> { Slice0 = copySlice config.Slice0; Slice1 = copySlice config.Slice1 })
    let routing = routing |> Map.map (fun _ routes -> List.ofSeq routes)

    member _.Clbs = clbs
    member _.Routing = routing
    member _.IO = io
    member _.LutK = lutK

    static member Empty(?lutK: int) = Bitstream(Map.empty, Map.empty, Map.empty, defaultArg lutK 4)

    static member Create(
        ?clbs: Map<string, CLBConfig>,
        ?routing: Map<string, RouteConfig list>,
        ?io: Map<string, IOConfig>,
        ?lutK: int
    ) =
        Bitstream(defaultArg clbs Map.empty, defaultArg routing Map.empty, defaultArg io Map.empty, defaultArg lutK 4)

    static member ParseJson(json: string) =
        if isNull json then
            nullArg "json"

        use document = JsonDocument.Parse json
        let root = document.RootElement

        if root.ValueKind <> JsonValueKind.Object then
            raise (JsonException("bitstream JSON root must be an object"))

        let tryProperty (name: string) (element: JsonElement) =
            let mutable value = Unchecked.defaultof<JsonElement>
            if element.TryGetProperty(name, &value) then Some value else None

        let lutK = tryProperty "lut_k" root |> Option.map _.GetInt32() |> Option.defaultValue 4

        if lutK < 2 || lutK > 6 then
            raise (JsonException("lut_k must be between 2 and 6"))

        let tableLength = 1 <<< lutK

        let parseTable name (slice: JsonElement) =
            match tryProperty name slice with
            | Some table -> table.EnumerateArray() |> Seq.map _.GetInt32() |> Array.ofSeq
            | None -> Array.zeroCreate tableLength

        let parseSlice name (clb: JsonElement) =
            match tryProperty name clb with
            | None ->
                { LutA = Array.zeroCreate tableLength
                  LutB = Array.zeroCreate tableLength
                  FlipFlopAEnabled = false
                  FlipFlopBEnabled = false
                  CarryEnabled = false }
            | Some slice ->
                let flag property =
                    tryProperty property slice |> Option.exists _.GetBoolean()

                { LutA = parseTable "lut_a" slice
                  LutB = parseTable "lut_b" slice
                  FlipFlopAEnabled = flag "ff_a"
                  FlipFlopBEnabled = flag "ff_b"
                  CarryEnabled = flag "carry" }

        let clbs =
            match tryProperty "clbs" root with
            | None -> Map.empty
            | Some values ->
                values.EnumerateObject()
                |> Seq.map (fun property ->
                    property.Name,
                    { Slice0 = parseSlice "slice0" property.Value
                      Slice1 = parseSlice "slice1" property.Value })
                |> Map.ofSeq

        let routing =
            match tryProperty "routing" root with
            | None -> Map.empty
            | Some values ->
                values.EnumerateObject()
                |> Seq.map (fun property ->
                    property.Name,
                    (property.Value.EnumerateArray()
                     |> Seq.map (fun route ->
                         { Source = route.GetProperty("src").GetString()
                           Destination = route.GetProperty("dst").GetString() })
                     |> List.ofSeq))
                |> Map.ofSeq

        let io =
            match tryProperty "io" root with
            | None -> Map.empty
            | Some values ->
                values.EnumerateObject()
                |> Seq.map (fun property ->
                    let mode =
                        tryProperty "mode" property.Value
                        |> Option.bind (fun value -> Option.ofObj (value.GetString()))
                        |> Option.defaultValue "input"

                    property.Name, { Mode = mode })
                |> Map.ofSeq

        Bitstream(clbs, routing, io, lutK)

/// A configured FPGA fabric with CLBs, routing, and I/O blocks.
type FPGA(bitstream: Bitstream) =
    do
        if isNull (box bitstream) then
            nullArg "bitstream"

    let clbs = Dictionary<string, CLB>(StringComparer.Ordinal)
    let switches = Dictionary<string, SwitchMatrix>(StringComparer.Ordinal)
    let ioBlocks = Dictionary<string, IOBlock>(StringComparer.Ordinal)

    let configureSlice (slice: Slice) config =
        slice.Configure(
            config.LutA,
            config.LutB,
            config.FlipFlopAEnabled,
            config.FlipFlopBEnabled,
            config.CarryEnabled
        )

    let parseMode (mode: string) =
        match mode.ToLowerInvariant() with
        | "output" -> Output
        | "tristate" -> Tristate
        | _ -> Input

    do
        for KeyValue(name, config) in bitstream.Clbs do
            let clb = CLB(bitstream.LutK)
            configureSlice clb.Slice0 config.Slice0
            configureSlice clb.Slice1 config.Slice1
            clbs[name] <- clb

        for KeyValue(name, routes) in bitstream.Routing do
            let ports = routes |> Seq.collect (fun route -> [ route.Source; route.Destination ]) |> Set.ofSeq
            if not ports.IsEmpty then
                let matrix = SwitchMatrix ports
                routes |> List.iter (fun route -> matrix.Connect(route.Source, route.Destination))
                switches[name] <- matrix

        for KeyValue(name, config) in bitstream.IO do
            ioBlocks[name] <- IOBlock(name, parseMode config.Mode)

    let getIO name =
        match ioBlocks.TryGetValue name with
        | true, value -> value
        | _ -> raise (KeyNotFoundException($"I/O pin {name} was not found"))

    member _.Bitstream = bitstream
    member _.Clbs = clbs |> Seq.map (fun pair -> pair.Key, pair.Value) |> Map.ofSeq
    member _.Switches = switches |> Seq.map (fun pair -> pair.Key, pair.Value) |> Map.ofSeq
    member _.IOBlocks = ioBlocks |> Seq.map (fun pair -> pair.Key, pair.Value) |> Map.ofSeq

    member _.EvaluateCLB(
        name: string,
        slice0InputsA: int array,
        slice0InputsB: int array,
        slice1InputsA: int array,
        slice1InputsB: int array,
        clock: int,
        ?carryIn: int
    ) =
        match clbs.TryGetValue name with
        | true, clb ->
            clb.Evaluate(
                slice0InputsA,
                slice0InputsB,
                slice1InputsA,
                slice1InputsB,
                clock,
                defaultArg carryIn 0
            )
        | _ -> raise (KeyNotFoundException($"CLB {name} was not found"))

    member _.Route(name: string, signals: Map<string, int>) =
        match switches.TryGetValue name with
        | true, matrix -> matrix.Route signals
        | _ -> raise (KeyNotFoundException($"switch matrix {name} was not found"))

    member _.SetInput(name: string, value: int) = getIO name |> fun io -> io.DrivePad value
    member _.DriveOutput(name: string, value: int) = getIO name |> fun io -> io.DriveInternal value
    member _.ReadOutput(name: string) = getIO name |> fun io -> io.ReadPad()
