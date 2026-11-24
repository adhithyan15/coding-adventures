namespace CodingAdventures.LogicGates

open System

/// Raised when a gate receives an argument that is not the bit 0 or 1.
///
/// It derives from ArgumentOutOfRangeException, so existing callers that catch
/// the BCL type (for example fsharp/arithmetic and fsharp/fpga) keep working,
/// while new callers can catch the rejection-specific type and read which
/// parameter was bad and what value it had.
///
/// F# has no implicit numeric conversions, so `andGate 1L 0`, `andGate 'a' 1`,
/// `andGate true 0` and `andGate 1.0 0` do not compile: the type-confusion
/// matrix is rejected at compile time. Only `int` values can reach a gate, and
/// the only runtime rejection is an int outside {0, 1}.
type InvalidBitException(parameter: string, value: int) =
    inherit ArgumentOutOfRangeException(parameter, value, "Bit values must be 0 or 1.")

    /// Name of the offending parameter ("a" or "b").
    member _.Parameter = parameter

    /// The rejected value.
    member _.Value = value

/// Core digital logic gates over integer bits.
[<RequireQualifiedAccess>]
module LogicGates =
    let private bitError (name: string) (value: int) : unit =
        raise (InvalidBitException(name, value))

    let private validateBit name value =
        if value <> 0 && value <> 1 then
            bitError name value

    // Every gate validates every argument itself, up front and in parameter
    // order, so the first invalid parameter is reported under the gate's own
    // public name ("a"/"b") rather than an inner helper's.
    let private check1 a = validateBit "a" a

    let private check2 a b =
        validateBit "a" a
        validateBit "b" b

    let private notRaw a = if a = 0 then 1 else 0
    let private andRaw a b = if a = 1 && b = 1 then 1 else 0
    let private orRaw a b = if a = 1 || b = 1 then 1 else 0
    let private xorRaw a b = if a <> b then 1 else 0
    let private nandRaw a b = notRaw (andRaw a b)

    let notGate (a: int) =
        check1 a
        notRaw a

    let andGate (a: int) (b: int) =
        check2 a b
        andRaw a b

    let orGate (a: int) (b: int) =
        check2 a b
        orRaw a b

    let xorGate (a: int) (b: int) =
        check2 a b
        xorRaw a b

    let nand (a: int) (b: int) =
        check2 a b
        nandRaw a b

    let nor (a: int) (b: int) =
        check2 a b
        notRaw (orRaw a b)

    let xnor (a: int) (b: int) =
        check2 a b
        notRaw (xorRaw a b)

    let nandNot (a: int) =
        check1 a
        nandRaw a a

    let nandAnd (a: int) (b: int) =
        check2 a b
        notRaw (nandRaw a b)

    let nandOr (a: int) (b: int) =
        check2 a b
        nandRaw (nandRaw a a) (nandRaw b b)

    let nandXor (a: int) (b: int) =
        check2 a b
        let n = nandRaw a b
        nandRaw (nandRaw a n) (nandRaw b n)

    /// Maximum number of inputs accepted by andN / orN / xorN.
    let maxInputs = 65536

    /// Validate an N-ary input list.  Ordering contract:
    ///   1. null                         -> ArgumentNullException("inputs")
    ///   2. length > maxInputs           -> ArgumentException("inputs")
    ///   3. length < minLength           -> ArgumentException("inputs")
    ///   4. first element not 0 or 1     -> InvalidBitException("inputs[i]")
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
        |> List.iteri (fun i v ->
            if v <> 0 && v <> 1 then
                bitError (sprintf "inputs[%d]" i) v)

    let andN (inputs: int list) =
        validateInputs "andN" 2 inputs
        if List.forall (fun v -> v = 1) inputs then 1 else 0

    let orN (inputs: int list) =
        validateInputs "orN" 2 inputs
        if List.exists (fun v -> v = 1) inputs then 1 else 0

    let xorN (inputs: int list) =
        validateInputs "xorN" 0 inputs
        inputs |> List.fold (fun acc v -> acc ^^^ v) 0
