namespace CodingAdventures.StarlarkCompiler.FSharp

open System
open System.Collections.Generic
open System.Text
open System.Text.Json

module Opcodes =
    /// Opcode categories representing functional bytecode domains.
    type OpCategory =
        | Stack = 0x0
        | Variable = 0x1
        | Arithmetic = 0x2
        | Comparison = 0x3
        | ControlFlow = 0x4
        | Function = 0x5
        | Collection = 0x6
        | SubscriptAttribute = 0x7
        | Iteration = 0x8
        | Module = 0x9
        | Io = 0xA
        | VmControl = 0xF

    /// Category metadata helper functions.
    let categoryId (category: OpCategory) : string =
        match category with
        | OpCategory.Stack -> "stack"
        | OpCategory.Variable -> "variable"
        | OpCategory.Arithmetic -> "arithmetic"
        | OpCategory.Comparison -> "comparison"
        | OpCategory.ControlFlow -> "control_flow"
        | OpCategory.Function -> "function"
        | OpCategory.Collection -> "collection"
        | OpCategory.SubscriptAttribute -> "subscript_attribute"
        | OpCategory.Iteration -> "iteration"
        | OpCategory.Module -> "module"
        | OpCategory.Io -> "io"
        | OpCategory.VmControl -> "vm_control"
        | _ -> raise (Errors.corpusSchemaViolation ())

    let categoryCamelName (category: OpCategory) : string =
        match category with
        | OpCategory.Stack -> "stack"
        | OpCategory.Variable -> "variable"
        | OpCategory.Arithmetic -> "arithmetic"
        | OpCategory.Comparison -> "comparison"
        | OpCategory.ControlFlow -> "controlFlow"
        | OpCategory.Function -> "function"
        | OpCategory.Collection -> "collection"
        | OpCategory.SubscriptAttribute -> "subscriptAttribute"
        | OpCategory.Iteration -> "iteration"
        | OpCategory.Module -> "module"
        | OpCategory.Io -> "io"
        | OpCategory.VmControl -> "vmControl"
        | _ -> raise (Errors.corpusSchemaViolation ())

    let categoryNibble (category: OpCategory) : byte =
        byte category

    /// The 61 distinct Starlark bytecode opcode enumerations.
    type Op =
        // Stack (0x01..0x06)
        | LoadConst = 0x01uy
        | Pop = 0x02uy
        | Dup = 0x03uy
        | LoadNone = 0x04uy
        | LoadTrue = 0x05uy
        | LoadFalse = 0x06uy

        // Variable (0x10..0x15)
        | StoreName = 0x10uy
        | LoadName = 0x11uy
        | StoreLocal = 0x12uy
        | LoadLocal = 0x13uy
        | StoreClosure = 0x14uy
        | LoadClosure = 0x15uy

        // Arithmetic (0x20..0x2D)
        | Add = 0x20uy
        | Sub = 0x21uy
        | Mul = 0x22uy
        | Div = 0x23uy
        | FloorDiv = 0x24uy
        | Mod = 0x25uy
        | Power = 0x26uy
        | Negate = 0x27uy
        | BitAnd = 0x28uy
        | BitOr = 0x29uy
        | BitXor = 0x2Auy
        | BitNot = 0x2Buy
        | LShift = 0x2Cuy
        | RShift = 0x2Duy

        // Comparison (0x30..0x38)
        | CmpEq = 0x30uy
        | CmpNe = 0x31uy
        | CmpLt = 0x32uy
        | CmpGt = 0x33uy
        | CmpLe = 0x34uy
        | CmpGe = 0x35uy
        | CmpIn = 0x36uy
        | CmpNotIn = 0x37uy
        | Not = 0x38uy

        // Control Flow (0x40..0x44)
        | Jump = 0x40uy
        | JumpIfFalse = 0x41uy
        | JumpIfTrue = 0x42uy
        | JumpIfFalseOrPop = 0x43uy
        | JumpIfTrueOrPop = 0x44uy

        // Function (0x50..0x53)
        | MakeFunction = 0x50uy
        | CallFunction = 0x51uy
        | CallFunctionKw = 0x52uy
        | Return = 0x53uy

        // Collection (0x60..0x64)
        | BuildList = 0x60uy
        | BuildDict = 0x61uy
        | BuildTuple = 0x62uy
        | ListAppend = 0x63uy
        | DictSet = 0x64uy

        // Subscript / Attribute (0x70..0x74)
        | LoadSubscript = 0x70uy
        | StoreSubscript = 0x71uy
        | LoadAttr = 0x72uy
        | StoreAttr = 0x73uy
        | LoadSlice = 0x74uy

        // Iteration (0x80..0x82)
        | GetIter = 0x80uy
        | ForIter = 0x81uy
        | UnpackSequence = 0x82uy

        // Module (0x90..0x91)
        | LoadModule = 0x90uy
        | ImportFrom = 0x91uy

        // IO (0xA0)
        | Print = 0xA0uy

        // VM Control (0xFF)
        | Halt = 0xFFuy

    let private canonicalOps : Op array =
        [|
            Op.LoadConst; Op.Pop; Op.Dup; Op.LoadNone; Op.LoadTrue; Op.LoadFalse
            Op.StoreName; Op.LoadName; Op.StoreLocal; Op.LoadLocal; Op.StoreClosure; Op.LoadClosure
            Op.Add; Op.Sub; Op.Mul; Op.Div; Op.FloorDiv; Op.Mod; Op.Power; Op.Negate
            Op.BitAnd; Op.BitOr; Op.BitXor; Op.BitNot; Op.LShift; Op.RShift
            Op.CmpEq; Op.CmpNe; Op.CmpLt; Op.CmpGt; Op.CmpLe; Op.CmpGe; Op.CmpIn; Op.CmpNotIn; Op.Not
            Op.Jump; Op.JumpIfFalse; Op.JumpIfTrue; Op.JumpIfFalseOrPop; Op.JumpIfTrueOrPop
            Op.MakeFunction; Op.CallFunction; Op.CallFunctionKw; Op.Return
            Op.BuildList; Op.BuildDict; Op.BuildTuple; Op.ListAppend; Op.DictSet
            Op.LoadSubscript; Op.StoreSubscript; Op.LoadAttr; Op.StoreAttr; Op.LoadSlice
            Op.GetIter; Op.ForIter; Op.UnpackSequence
            Op.LoadModule; Op.ImportFrom
            Op.Print
            Op.Halt
        |]

    let private decodeTable : ValueOption<Op> array =
        Array.create Constants.TotalDecodeEntries ValueNone

    let private symbolToOp = Dictionary<string, Op>(StringComparer.Ordinal)
    let private opToSymbol = Dictionary<Op, string>()
    let private opToCategory = Dictionary<Op, OpCategory>()

    let private rawBinaryOps =
        dict [
            "+", Op.Add
            "-", Op.Sub
            "*", Op.Mul
            "/", Op.Div
            "//", Op.FloorDiv
            "%", Op.Mod
            "**", Op.Power
            "&", Op.BitAnd
            "|", Op.BitOr
            "^", Op.BitXor
            "<<", Op.LShift
            ">>", Op.RShift
        ]

    let private rawCompareOps =
        dict [
            "==", Op.CmpEq
            "!=", Op.CmpNe
            "<", Op.CmpLt
            ">", Op.CmpGt
            "<=", Op.CmpLe
            ">=", Op.CmpGe
            "in", Op.CmpIn
            "not in", Op.CmpNotIn
        ]

    let private rawAugmentedAssignOps =
        dict [
            "+=", Op.Add
            "-=", Op.Sub
            "*=", Op.Mul
            "/=", Op.Div
            "//=", Op.FloorDiv
            "%=", Op.Mod
            "**=", Op.Power
            "&=", Op.BitAnd
            "|=", Op.BitOr
            "^=", Op.BitXor
            "<<=", Op.LShift
            ">>=", Op.RShift
        ]

    let private rawUnaryOps =
        dict [
            "-", Op.Negate
            "~", Op.BitNot
        ]

    do
        for op in canonicalOps do
            let b = byte op
            let sym = op.ToString()
            decodeTable.[int b] <- ValueSome op
            symbolToOp.[sym] <- op
            opToSymbol.[op] <- sym

            let cat =
                match int b >>> Constants.CategoryNibbleShift with
                | 0x0 -> OpCategory.Stack
                | 0x1 -> OpCategory.Variable
                | 0x2 -> OpCategory.Arithmetic
                | 0x3 -> OpCategory.Comparison
                | 0x4 -> OpCategory.ControlFlow
                | 0x5 -> OpCategory.Function
                | 0x6 -> OpCategory.Collection
                | 0x7 -> OpCategory.SubscriptAttribute
                | 0x8 -> OpCategory.Iteration
                | 0x9 -> OpCategory.Module
                | 0xA -> OpCategory.Io
                | 0xF -> OpCategory.VmControl
                | _ -> raise (Errors.corpusSchemaViolation ())
            opToCategory.[op] <- cat

    /// Returns a fresh copy of all 61 canonical opcodes.
    let allOps () : Op array =
        Array.copy canonicalOps

    /// Decodes an opcode from an integer byte value (0..255).
    let opFromByte (value: int) : Op option =
        if value < 0 || value >= Constants.TotalDecodeEntries then
            None
        else
            match decodeTable.[value] with
            | ValueSome op -> Some op
            | ValueNone -> None

    /// Looks up an opcode by its exact PascalCase symbol name.
    let opFromSymbol (symbol: string) : Op option =
        if String.IsNullOrEmpty symbol then
            None
        else
            match symbolToOp.TryGetValue(symbol) with
            | true, op -> Some op
            | false, _ -> None

    /// Gets the byte value of an opcode.
    let opByte (op: Op) : byte =
        byte op

    /// Gets the PascalCase symbol of an opcode.
    let opSymbol (op: Op) : string =
        opToSymbol.[op]

    /// Gets the category of an opcode.
    let opCategory (op: Op) : OpCategory =
        opToCategory.[op]

    /// Gets the category corresponding to an opcode byte value, or None if unassigned.
    let opCategoryFromByte (value: int) : OpCategory option =
        match opFromByte value with
        | Some op -> Some opToCategory.[op]
        | None -> None

    /// Returns a fresh mutable dictionary of the 12 binary operators.
    let binaryOpMap () : Dictionary<string, Op> =
        Dictionary<string, Op>(rawBinaryOps, StringComparer.Ordinal)

    /// Returns a fresh mutable dictionary of the 8 comparison operators.
    let compareOpMap () : Dictionary<string, Op> =
        Dictionary<string, Op>(rawCompareOps, StringComparer.Ordinal)

    /// Returns a fresh mutable dictionary of the 12 augmented assignment operators.
    let augmentedAssignMap () : Dictionary<string, Op> =
        Dictionary<string, Op>(rawAugmentedAssignOps, StringComparer.Ordinal)

    /// Returns a fresh mutable dictionary of the 2 unary operators.
    let unaryOpMap () : Dictionary<string, Op> =
        Dictionary<string, Op>(rawUnaryOps, StringComparer.Ordinal)

    /// Finds the opcode corresponding to a binary operator string.
    let binaryOpcode (opStr: string) : Op option =
        if String.IsNullOrEmpty opStr then None
        else
            match rawBinaryOps.TryGetValue opStr with
            | true, v -> Some v
            | false, _ -> None

    /// Finds the opcode corresponding to a comparison operator string.
    let compareOpcode (opStr: string) : Op option =
        if String.IsNullOrEmpty opStr then None
        else
            match rawCompareOps.TryGetValue opStr with
            | true, v -> Some v
            | false, _ -> None

    /// Finds the opcode corresponding to an augmented assignment operator string.
    let augmentedAssignOpcode (opStr: string) : Op option =
        if String.IsNullOrEmpty opStr then None
        else
            match rawAugmentedAssignOps.TryGetValue opStr with
            | true, v -> Some v
            | false, _ -> None

    /// Finds the opcode corresponding to a unary operator string.
    let unaryOpcode (opStr: string) : Op option =
        if String.IsNullOrEmpty opStr then None
        else
            match rawUnaryOps.TryGetValue opStr with
            | true, v -> Some v
            | false, _ -> None

    /// Validates raw STP01 corpus payload against bounds, transport, and schema.
    let validateCorpusBytes (rawBytes: byte array) : unit =
        if obj.ReferenceEquals(rawBytes, null) then
            raise (Errors.corpusMalformedTransport ())

        if rawBytes.Length > Constants.MaxCorpusBytes then
            raise (Errors.corpusByteLimitExceeded ())

        // BOM rejection
        if rawBytes.Length >= Constants.Utf8BomLength &&
           rawBytes.[0] = Constants.Utf8BomByte0 &&
           rawBytes.[1] = Constants.Utf8BomByte1 &&
           rawBytes.[2] = Constants.Utf8BomByte2 then
            raise (Errors.corpusMalformedTransport ())

        let text =
            try
                let utf8Strict = UTF8Encoding(false, true)
                utf8Strict.GetString(rawBytes)
            with :? DecoderFallbackException ->
                raise (Errors.corpusMalformedTransport ())

        use doc =
            try
                JsonDocument.Parse(text)
            with :? JsonException ->
                raise (Errors.corpusMalformedTransport ())

        let root = doc.RootElement
        if root.ValueKind <> JsonValueKind.Object then
            raise (Errors.corpusSchemaViolation ())

        match root.TryGetProperty(Constants.PropSchemaVersion) with
        | true, prop when prop.ValueKind = JsonValueKind.Number ->
            let mutable sv = 0
            if not (prop.TryGetInt32(&sv)) || sv <> Constants.ExpectedSchemaVersion then
                raise (Errors.corpusSchemaViolation ())
        | _ -> raise (Errors.corpusSchemaViolation ())

        match root.TryGetProperty(Constants.PropProfile) with
        | true, prop when prop.ValueKind = JsonValueKind.String && prop.GetString() = Constants.ProfileStandaloneProviderV1 -> ()
        | _ -> raise (Errors.corpusSchemaViolation ())

        match root.TryGetProperty(Constants.PropOpcodes) with
        | true, prop when prop.ValueKind = JsonValueKind.Array && prop.GetArrayLength() = Constants.TotalOpcodeCount -> ()
        | _ -> raise (Errors.corpusSchemaViolation ())

        match root.TryGetProperty(Constants.PropDecode) with
        | true, prop when prop.ValueKind = JsonValueKind.Array && prop.GetArrayLength() = Constants.TotalDecodeEntries -> ()
        | _ -> raise (Errors.corpusSchemaViolation ())
