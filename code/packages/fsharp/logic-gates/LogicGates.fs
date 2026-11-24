namespace CodingAdventures.LogicGates

open System

/// Core digital logic gates over integer bits.
[<RequireQualifiedAccess>]
module LogicGates =
    let private validateBit name value =
        if value <> 0 && value <> 1 then
            raise (ArgumentOutOfRangeException(name, value, "Bit values must be 0 or 1."))

    let notGate a =
        validateBit (nameof a) a
        if a = 0 then 1 else 0

    let andGate a b =
        validateBit (nameof a) a
        validateBit (nameof b) b
        if a = 1 && b = 1 then 1 else 0

    let orGate a b =
        validateBit (nameof a) a
        validateBit (nameof b) b
        if a = 1 || b = 1 then 1 else 0

    let xorGate a b =
        validateBit (nameof a) a
        validateBit (nameof b) b
        if a <> b then 1 else 0

    let nand a b = notGate (andGate a b)

    let nor a b = notGate (orGate a b)

    let xnor a b = notGate (xorGate a b)

    let nandNot a = nand a a

    let nandAnd a b = nandNot (nand a b)

    let nandOr a b = nand (nandNot a) (nandNot b)

    let nandXor a b =
        let nandValue = nand a b
        nand (nand a nandValue) (nand b nandValue)

    /// Maximum number of inputs accepted by andN / orN / xorN.
    let maxInputs = 65536

    /// Validate an N-ary input list.  Ordering contract:
    ///   1. null                         -> ArgumentNullException("inputs")
    ///   2. length > maxInputs           -> ArgumentException("inputs")
    ///   3. length < minLength           -> ArgumentException("inputs")
    ///   4. first element not 0 or 1     -> ArgumentOutOfRangeException("inputs[i]")
    /// Shape errors always win over bit errors.  The walk stops after
    /// maxInputs + 1 cells, so an oversized list is never traversed fully.
    let private validateInputs (name: string) (minLength: int) (inputs: int list) =
        if isNull (box inputs) then
            nullArg "inputs"

        let mutable count = 0
        let mutable rest = inputs

        while count <= maxInputs && not rest.IsEmpty do
            count <- count + 1
            rest <- rest.Tail

        if count > maxInputs then
            invalidArg "inputs" (sprintf "%s accepts at most %d inputs." name maxInputs)

        if count < minLength then
            invalidArg "inputs" (sprintf "%s requires at least %d inputs." name minLength)

        inputs
        |> List.iteri (fun i v -> validateBit (sprintf "inputs[%d]" i) v)

    let andN (inputs: int list) =
        validateInputs "andN" 2 inputs
        if List.forall (fun v -> v = 1) inputs then 1 else 0

    let orN (inputs: int list) =
        validateInputs "orN" 2 inputs
        if List.exists (fun v -> v = 1) inputs then 1 else 0

    let xorN (inputs: int list) =
        validateInputs "xorN" 0 inputs
        inputs |> List.fold (fun acc v -> acc ^^^ v) 0
