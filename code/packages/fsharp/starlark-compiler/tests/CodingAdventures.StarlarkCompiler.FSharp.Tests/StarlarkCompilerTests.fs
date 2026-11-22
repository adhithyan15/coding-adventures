namespace CodingAdventures.StarlarkCompiler.FSharp.Tests

open System
open System.IO
open System.Text
open System.Text.Json
open Xunit
open CodingAdventures.StarlarkCompiler.FSharp
open CodingAdventures.StarlarkCompiler.FSharp.Opcodes

module StarlarkCompilerTests =

    let private independentOpcodes : (int * string * string) array =
        [|
            0x01, "LoadConst", "stack"
            0x02, "Pop", "stack"
            0x03, "Dup", "stack"
            0x04, "LoadNone", "stack"
            0x05, "LoadTrue", "stack"
            0x06, "LoadFalse", "stack"

            0x10, "StoreName", "variable"
            0x11, "LoadName", "variable"
            0x12, "StoreLocal", "variable"
            0x13, "LoadLocal", "variable"
            0x14, "StoreClosure", "variable"
            0x15, "LoadClosure", "variable"

            0x20, "Add", "arithmetic"
            0x21, "Sub", "arithmetic"
            0x22, "Mul", "arithmetic"
            0x23, "Div", "arithmetic"
            0x24, "FloorDiv", "arithmetic"
            0x25, "Mod", "arithmetic"
            0x26, "Power", "arithmetic"
            0x27, "Negate", "arithmetic"
            0x28, "BitAnd", "arithmetic"
            0x29, "BitOr", "arithmetic"
            0x2A, "BitXor", "arithmetic"
            0x2B, "BitNot", "arithmetic"
            0x2C, "LShift", "arithmetic"
            0x2D, "RShift", "arithmetic"

            0x30, "CmpEq", "comparison"
            0x31, "CmpNe", "comparison"
            0x32, "CmpLt", "comparison"
            0x33, "CmpGt", "comparison"
            0x34, "CmpLe", "comparison"
            0x35, "CmpGe", "comparison"
            0x36, "CmpIn", "comparison"
            0x37, "CmpNotIn", "comparison"
            0x38, "Not", "comparison"

            0x40, "Jump", "control_flow"
            0x41, "JumpIfFalse", "control_flow"
            0x42, "JumpIfTrue", "control_flow"
            0x43, "JumpIfFalseOrPop", "control_flow"
            0x44, "JumpIfTrueOrPop", "control_flow"

            0x50, "MakeFunction", "function"
            0x51, "CallFunction", "function"
            0x52, "CallFunctionKw", "function"
            0x53, "Return", "function"

            0x60, "BuildList", "collection"
            0x61, "BuildDict", "collection"
            0x62, "BuildTuple", "collection"
            0x63, "ListAppend", "collection"
            0x64, "DictSet", "collection"

            0x70, "LoadSubscript", "subscript_attribute"
            0x71, "StoreSubscript", "subscript_attribute"
            0x72, "LoadAttr", "subscript_attribute"
            0x73, "StoreAttr", "subscript_attribute"
            0x74, "LoadSlice", "subscript_attribute"

            0x80, "GetIter", "iteration"
            0x81, "ForIter", "iteration"
            0x82, "UnpackSequence", "iteration"

            0x90, "LoadModule", "module"
            0x91, "ImportFrom", "module"

            0xA0, "Print", "io"

            0xFF, "Halt", "vm_control"
        |]

    let private independentOperators : (string * string * string) array =
        [|
            // Binary (12)
            "binary", "+", "Add"
            "binary", "-", "Sub"
            "binary", "*", "Mul"
            "binary", "/", "Div"
            "binary", "//", "FloorDiv"
            "binary", "%", "Mod"
            "binary", "**", "Power"
            "binary", "&", "BitAnd"
            "binary", "|", "BitOr"
            "binary", "^", "BitXor"
            "binary", "<<", "LShift"
            "binary", ">>", "RShift"

            // Comparison (8)
            "compare", "==", "CmpEq"
            "compare", "!=", "CmpNe"
            "compare", "<", "CmpLt"
            "compare", ">", "CmpGt"
            "compare", "<=", "CmpLe"
            "compare", ">=", "CmpGe"
            "compare", "in", "CmpIn"
            "compare", "not in", "CmpNotIn"

            // Augmented Assignment (12)
            "augmented_assign", "+=", "Add"
            "augmented_assign", "-=", "Sub"
            "augmented_assign", "*=", "Mul"
            "augmented_assign", "/=", "Div"
            "augmented_assign", "//=", "FloorDiv"
            "augmented_assign", "%=", "Mod"
            "augmented_assign", "**=", "Power"
            "augmented_assign", "&=", "BitAnd"
            "augmented_assign", "|=", "BitOr"
            "augmented_assign", "^=", "BitXor"
            "augmented_assign", "<<=", "LShift"
            "augmented_assign", ">>=", "RShift"

            // Unary (2)
            "unary", "-", "Negate"
            "unary", "~", "BitNot"
        |]

    let private locateFixture (relativePath: string) : string =
        let rec search (dir: DirectoryInfo option) (depth: int) =
            if depth > 10 then
                raise (FileNotFoundException(sprintf "Cannot locate fixture: %s" relativePath))
            else
                match dir with
                | None -> raise (FileNotFoundException(sprintf "Cannot locate fixture: %s" relativePath))
                | Some current ->
                    let candidate = Path.Combine(current.FullName, relativePath)
                    if File.Exists candidate then candidate
                    else search (Option.ofObj current.Parent) (depth + 1)
        search (Some (DirectoryInfo AppContext.BaseDirectory)) 0

    let private requiredString (element: JsonElement) (property: string) : string =
        match element.GetProperty(property).GetString() |> Option.ofObj with
        | None -> raise (InvalidDataException(sprintf "fixture property %s is null" property))
        | Some value -> value

    let private optionalString (element: JsonElement) (property: string) : string option =
        element.GetProperty(property).GetString() |> Option.ofObj

    [<Fact>]
    let ``STP01 CasesJson Conformance Verification`` () =
        let fixturePath = locateFixture "code/specs/fixtures/starlark-opcode-provider-v1/cases.json"
        let rawBytes = File.ReadAllBytes fixturePath
        Assert.True(rawBytes.Length <= Constants.MaxCorpusBytes)

        validateCorpusBytes rawBytes

        use doc = JsonDocument.Parse rawBytes
        let root = doc.RootElement

        // Verify opcodes
        let opcodes = root.GetProperty "opcodes"
        Assert.Equal(61, opcodes.GetArrayLength())
        for item in opcodes.EnumerateArray() do
            let symbol = requiredString item "symbol"
            let b = item.GetProperty("byte").GetInt32()
            let category = requiredString item "category"

            match opFromSymbol symbol with
            | Some op ->
                Assert.Equal(b, int (opByte op))
                Assert.Equal(symbol, opSymbol op)
                Assert.Equal(category, categoryId (opCategory op))
                Assert.Equal(Some op, opFromByte b)
            | None -> Assert.Fail(sprintf "Expected opcode %s to exist" symbol)

        // Verify decode table
        let decode = root.GetProperty "decode"
        Assert.Equal(256, decode.GetArrayLength())
        let mutable decodeIdx = 0
        let mutable nonNullCount = 0
        for item in decode.EnumerateArray() do
            let opFromByteVal = opFromByte decodeIdx
            match optionalString item "symbol" with
            | None ->
                Assert.Equal(None, opFromByteVal)
            | Some symbol ->
                nonNullCount <- nonNullCount + 1
                match opFromByteVal with
                | Some op -> Assert.Equal(symbol, opSymbol op)
                | None -> Assert.Fail(sprintf "Expected non-null opcode at %d" decodeIdx)
            decodeIdx <- decodeIdx + 1

        Assert.Equal(61, nonNullCount)
        Assert.Equal(195, 256 - nonNullCount)

        // Verify operators
        let operators = root.GetProperty "operators"
        let binMap = binaryOpMap ()
        let cmpMap = compareOpMap ()
        let augMap = augmentedAssignMap ()
        let unMap = unaryOpMap ()

        for item in operators.GetProperty("binary").EnumerateArray() do
            let opStr = requiredString item "operator"
            let sym = requiredString item "symbol"
            Assert.True(binMap.ContainsKey opStr)
            Assert.Equal(sym, opSymbol binMap.[opStr])
            Assert.Equal(Some sym, binaryOpcode opStr |> Option.map opSymbol)

        for item in operators.GetProperty("compare").EnumerateArray() do
            let opStr = requiredString item "operator"
            let sym = requiredString item "symbol"
            Assert.True(cmpMap.ContainsKey opStr)
            Assert.Equal(sym, opSymbol cmpMap.[opStr])
            Assert.Equal(Some sym, compareOpcode opStr |> Option.map opSymbol)

        for item in operators.GetProperty("augmented_assign").EnumerateArray() do
            let opStr = requiredString item "operator"
            let sym = requiredString item "symbol"
            Assert.True(augMap.ContainsKey opStr)
            Assert.Equal(sym, opSymbol augMap.[opStr])
            Assert.Equal(Some sym, augmentedAssignOpcode opStr |> Option.map opSymbol)

        for item in operators.GetProperty("unary").EnumerateArray() do
            let opStr = requiredString item "operator"
            let sym = requiredString item "symbol"
            Assert.True(unMap.ContainsKey opStr)
            Assert.Equal(sym, opSymbol unMap.[opStr])
            Assert.Equal(Some sym, unaryOpcode opStr |> Option.map opSymbol)

        // Verify unknown lookups
        let unknownLookups = root.GetProperty "unknown_lookups"
        for item in unknownLookups.EnumerateArray() do
            let group = requiredString item "group"
            let opStr = requiredString item "operator"
            let result =
                match group with
                | "binary" -> binaryOpcode opStr
                | "compare" -> compareOpcode opStr
                | "augmented_assign" -> augmentedAssignOpcode opStr
                | "unary" -> unaryOpcode opStr
                | _ -> failwith "unknown group"
            Assert.Equal(None, result)

    [<Fact>]
    let ``Independent Literal Verification Opcodes And Operators`` () =
        Assert.Equal(61, independentOpcodes.Length)
        for b, sym, cat in independentOpcodes do
            match opFromByte b with
            | Some op ->
                Assert.Equal(sym, opSymbol op)
                Assert.Equal(cat, categoryId (opCategory op))
                Assert.Equal(Some op, opFromSymbol sym)
            | None -> Assert.Fail(sprintf "Missing opcode for byte %d" b)

        Assert.Equal(34, independentOperators.Length)
        for group, opStr, expectedSym in independentOperators do
            let opOpt =
                match group with
                | "binary" -> binaryOpcode opStr
                | "compare" -> compareOpcode opStr
                | "augmented_assign" -> augmentedAssignOpcode opStr
                | "unary" -> unaryOpcode opStr
                | _ -> None
            match opOpt with
            | Some op -> Assert.Equal(expectedSym, opSymbol op)
            | None -> Assert.Fail(sprintf "Missing operator %s in %s" opStr group)

    [<Fact>]
    let ``OpCategory Metadata Verification`` () =
        let allCats = Enum.GetValues<OpCategory>()
        Assert.Equal(12, allCats.Length)
        for cat in allCats do
            Assert.False(String.IsNullOrEmpty(categoryId cat))
            Assert.False(String.IsNullOrEmpty(categoryCamelName cat))
            Assert.True(categoryNibble cat <= 0x0Fuy)

        Assert.Equal("stack", categoryId OpCategory.Stack)
        Assert.Equal("control_flow", categoryId OpCategory.ControlFlow)
        Assert.Equal("controlFlow", categoryCamelName OpCategory.ControlFlow)
        Assert.Equal("subscriptAttribute", categoryCamelName OpCategory.SubscriptAttribute)
        Assert.Equal("vmControl", categoryCamelName OpCategory.VmControl)
        Assert.Equal(0uy, categoryNibble OpCategory.Stack)
        Assert.Equal(0x0Fuy, categoryNibble OpCategory.VmControl)

        let invalidCat = enum<OpCategory> 999
        Assert.Throws<Errors.StarlarkCompilerException>(fun () -> categoryId invalidCat |> ignore) |> ignore
        Assert.Throws<Errors.StarlarkCompilerException>(fun () -> categoryCamelName invalidCat |> ignore) |> ignore

    [<Fact>]
    let ``Boundary And Lookup Failures`` () =
        Assert.Equal(None, opFromByte -1)
        Assert.Equal(None, opFromByte 256)
        Assert.Equal(None, opFromByte 1000)
        Assert.Equal(None, opFromByte 0x00)
        Assert.Equal(None, opFromByte 0x07)
        Assert.Equal(None, opFromByte 0xFE)

        Assert.Equal(None, opCategoryFromByte -1)
        Assert.Equal(None, opCategoryFromByte 256)
        Assert.Equal(None, opCategoryFromByte 0x00)
        Assert.Equal(Some OpCategory.Stack, opCategoryFromByte 0x01)

        Assert.Equal(None, opFromSymbol "")
        Assert.Equal(None, opFromSymbol (Unchecked.defaultof<string>))
        Assert.Equal(None, opFromSymbol "UnknownOpcode")
        Assert.Equal(None, opFromSymbol "loadconst")

        Assert.Equal(None, binaryOpcode (Unchecked.defaultof<string>))
        Assert.Equal(None, binaryOpcode "invalid")
        Assert.Equal(None, compareOpcode (Unchecked.defaultof<string>))
        Assert.Equal(None, compareOpcode "invalid")
        Assert.Equal(None, augmentedAssignOpcode (Unchecked.defaultof<string>))
        Assert.Equal(None, augmentedAssignOpcode "invalid")
        Assert.Equal(None, unaryOpcode (Unchecked.defaultof<string>))
        Assert.Equal(None, unaryOpcode "invalid")

    [<Fact>]
    let ``Fresh Map Ownership Guarantee`` () =
        let map1 = binaryOpMap ()
        let map2 = binaryOpMap ()
        Assert.NotSame(map1, map2)
        map1.["+"] <- Op.Halt
        Assert.Equal(Op.Add, map2.["+"])
        Assert.Equal(Some Op.Add, binaryOpcode "+")

        let cmp1 = compareOpMap ()
        let cmp2 = compareOpMap ()
        Assert.NotSame(cmp1, cmp2)
        cmp1.["=="] <- Op.Halt
        Assert.Equal(Op.CmpEq, cmp2.["=="])

        let aug1 = augmentedAssignMap ()
        let aug2 = augmentedAssignMap ()
        Assert.NotSame(aug1, aug2)
        aug1.["+="] <- Op.Halt
        Assert.Equal(Op.Add, aug2.["+="])

        let un1 = unaryOpMap ()
        let un2 = unaryOpMap ()
        Assert.NotSame(un1, un2)
        un1.["-"] <- Op.Halt
        Assert.Equal(Op.Negate, un2.["-"])

        let ops1 = allOps ()
        let ops2 = allOps ()
        Assert.NotSame(ops1, ops2)
        Assert.Equal(61, ops1.Length)

    [<Fact>]
    let ``Standing Rule 20 Diagnostic Catalog And Exceptions`` () =
        let exInvalidByte = Errors.invalidByte ()
        Assert.Equal(Constants.ErrorIdInvalidByte, exInvalidByte.ErrorId)
        Assert.Equal(Errors.StarlarkCompilerErrorCode.InvalidByte, exInvalidByte.Code)
        Assert.DoesNotContain("0x", exInvalidByte.Message)

        let exUnknownOpcode = Errors.unknownOpcode ()
        Assert.Equal(Constants.ErrorIdUnknownOpcode, exUnknownOpcode.ErrorId)
        Assert.Equal(Errors.StarlarkCompilerErrorCode.UnknownOpcode, exUnknownOpcode.Code)

        let exUnknownOperator = Errors.unknownOperator ()
        Assert.Equal(Constants.ErrorIdUnknownOperator, exUnknownOperator.ErrorId)
        Assert.Equal(Errors.StarlarkCompilerErrorCode.UnknownOperator, exUnknownOperator.Code)

        let exByteLimit = Errors.corpusByteLimitExceeded ()
        Assert.Equal(Constants.ErrorIdInvalidCorpus, exByteLimit.ErrorId)
        Assert.Equal(Errors.StarlarkCompilerErrorCode.CorpusByteLimitExceeded, exByteLimit.Code)

        let exTransport = Errors.corpusMalformedTransport ()
        Assert.Equal(Constants.ErrorIdInvalidCorpus, exTransport.ErrorId)
        Assert.Equal(Errors.StarlarkCompilerErrorCode.CorpusMalformedTransport, exTransport.Code)

        let exSchema = Errors.corpusSchemaViolation ()
        Assert.Equal(Constants.ErrorIdInvalidCorpus, exSchema.ErrorId)
        Assert.Equal(Errors.StarlarkCompilerErrorCode.CorpusSchemaViolation, exSchema.Code)

    [<Fact>]
    let ``Transport And Schema Corruption Rejections`` () =
        // Null payload rejection
        let exNull = Assert.Throws<Errors.StarlarkCompilerException>(fun () -> validateCorpusBytes (Unchecked.defaultof<byte array>))
        Assert.Equal(Errors.StarlarkCompilerErrorCode.CorpusMalformedTransport, exNull.Code)

        // Byte limit exceeded
        let oversized = Array.zeroCreate (Constants.MaxCorpusBytes + 1)
        let ex1 = Assert.Throws<Errors.StarlarkCompilerException>(fun () -> validateCorpusBytes oversized)
        Assert.Equal(Errors.StarlarkCompilerErrorCode.CorpusByteLimitExceeded, ex1.Code)

        // BOM rejection
        let bomBytes = [| 0xEFuy; 0xBBuy; 0xBFuy; byte '{'; byte '}' |]
        let ex2 = Assert.Throws<Errors.StarlarkCompilerException>(fun () -> validateCorpusBytes bomBytes)
        Assert.Equal(Errors.StarlarkCompilerErrorCode.CorpusMalformedTransport, ex2.Code)

        // Invalid UTF-8
        let invalidUtf8 = [| byte '{'; 0xFFuy; byte '}' |]
        let ex3 = Assert.Throws<Errors.StarlarkCompilerException>(fun () -> validateCorpusBytes invalidUtf8)
        Assert.Equal(Errors.StarlarkCompilerErrorCode.CorpusMalformedTransport, ex3.Code)

        // Lone high surrogate byte sequence
        let highSurrogateBytes = [| byte '{'; 0xEDuy; 0xA0uy; 0x80uy; byte '}' |]
        let exSurrogate = Assert.Throws<Errors.StarlarkCompilerException>(fun () -> validateCorpusBytes highSurrogateBytes)
        Assert.Equal(Errors.StarlarkCompilerErrorCode.CorpusMalformedTransport, exSurrogate.Code)

        // Malformed JSON syntax
        let malformedJson = Encoding.UTF8.GetBytes "{broken"
        let exMalformed = Assert.Throws<Errors.StarlarkCompilerException>(fun () -> validateCorpusBytes malformedJson)
        Assert.Equal(Errors.StarlarkCompilerErrorCode.CorpusMalformedTransport, exMalformed.Code)

        // Schema violations
        let notAnObject = Encoding.UTF8.GetBytes "[1, 2, 3]"
        Assert.Throws<Errors.StarlarkCompilerException>(fun () -> validateCorpusBytes notAnObject) |> ignore

        let wrongVersion = Encoding.UTF8.GetBytes "{\"schema_version\": 99, \"profile\": \"standalone-provider-v1\", \"opcodes\": [], \"decode\": []}"
        Assert.Throws<Errors.StarlarkCompilerException>(fun () -> validateCorpusBytes wrongVersion) |> ignore

        let wrongVersionType = Encoding.UTF8.GetBytes "{\"schema_version\": \"1\", \"profile\": \"standalone-provider-v1\", \"opcodes\": [], \"decode\": []}"
        let exWrongVersionType = Assert.Throws<Errors.StarlarkCompilerException>(fun () -> validateCorpusBytes wrongVersionType)
        Assert.Equal(Errors.StarlarkCompilerErrorCode.CorpusSchemaViolation, exWrongVersionType.Code)

        let wrongProfile = Encoding.UTF8.GetBytes "{\"schema_version\": 1, \"profile\": \"wrong\", \"opcodes\": [], \"decode\": []}"
        Assert.Throws<Errors.StarlarkCompilerException>(fun () -> validateCorpusBytes wrongProfile) |> ignore

        let wrongProfileType = Encoding.UTF8.GetBytes "{\"schema_version\": 1, \"profile\": 123, \"opcodes\": [], \"decode\": []}"
        let exWrongProfileType = Assert.Throws<Errors.StarlarkCompilerException>(fun () -> validateCorpusBytes wrongProfileType)
        Assert.Equal(Errors.StarlarkCompilerErrorCode.CorpusSchemaViolation, exWrongProfileType.Code)

        let dummyOps = Array.create 61 "{}" |> String.concat ","
        let wrongDecodeCount = Encoding.UTF8.GetBytes(sprintf "{\"schema_version\": 1, \"profile\": \"standalone-provider-v1\", \"opcodes\": [%s], \"decode\": []}" dummyOps)
        Assert.Throws<Errors.StarlarkCompilerException>(fun () -> validateCorpusBytes wrongDecodeCount) |> ignore

    [<Fact>]
    let ``Security Showcase 5 Class Resilience`` () =
        // Class 1: Memory Exhaustion / Payload Size Defense
        let bomb = Array.zeroCreate (Constants.MaxCorpusBytes + 10)
        Assert.Throws<Errors.StarlarkCompilerException>(fun () -> validateCorpusBytes bomb) |> ignore

        // Class 2: Stack Exhaustion Defense - 100k lookups without recursion
        for i in 0 .. 99_999 do
            match opFromByte (i &&& 0xFF) with
            | Some op -> Assert.NotNull(opSymbol op)
            | None -> ()

        // Class 3: Opcode Integrity & Byte Validation
        for b in -50 .. 300 do
            let op = opFromByte b
            if b >= 0 && b <= 255 then
                match op with
                | Some o -> Assert.Equal(byte b, opByte o)
                | None -> ()
            else
                Assert.Equal(None, op)

        // Class 4: Injection & Parsing Boundary Defense
        let ex = Errors.invalidByte ()
        Assert.Equal("starlark-compiler: invalid opcode byte value", ex.Message)
        Assert.Equal("invalid-byte", ex.ErrorId)

        // Class 5: State Mutation Defense
        let bin = binaryOpMap ()
        bin.Clear()
        Assert.Equal(12, (binaryOpMap ()).Count)

    [<Fact>]
    let ``Deterministic 200 Case PRNG Property Test`` () =
        let rng = Random 1337
        for _ in 1 .. 200 do
            let v = rng.Next(-500, 500)
            let opOpt = opFromByte v
            if v >= 0 && v <= 255 then
                match opOpt with
                | Some op ->
                    Assert.Equal(byte v, opByte op)
                    Assert.Equal(Some op, opFromSymbol (opSymbol op))
                | None -> ()
            else
                Assert.Equal(None, opOpt)

    [<Fact>]
    let ``Adversarial 10000 Iteration Mutation Fuzz Loop`` () =
        let rng = Random 42
        let baseCorpus = Encoding.UTF8.GetBytes "{\"schema_version\": 1, \"profile\": \"standalone-provider-v1\"}"

        for _ in 1 .. 10_000 do
            let mutable mutated = Array.copy baseCorpus
            let mutationType = rng.Next(0, 4)
            match mutationType with
            | 0 ->
                mutated.[rng.Next(0, mutated.Length)] <- byte (rng.Next(0, 256))
            | 1 ->
                mutated <- Array.append [| 0xEFuy; 0xBBuy; 0xBFuy |] mutated
            | 2 ->
                let newLen = rng.Next(0, mutated.Length)
                mutated <- Array.sub mutated 0 newLen
            | 3 ->
                let extra = Array.zeroCreate (rng.Next(1, 100))
                rng.NextBytes extra
                mutated <- Array.append mutated extra
            | _ -> ()

            // Must either pass or throw typed StarlarkCompilerException (never unhandled crash or external JsonException)
            try
                validateCorpusBytes mutated
            with
            | :? Errors.StarlarkCompilerException -> ()
