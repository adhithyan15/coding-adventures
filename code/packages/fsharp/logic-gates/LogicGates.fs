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
    let private validateBit name value =
        if value <> 0 && value <> 1 then
            raise (InvalidBitException(name, value))

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

    let andN (inputs: int list) =
        if isNull (box inputs) then
            nullArg (nameof inputs)

        match inputs with
        | first :: second :: rest -> rest |> List.fold andGate (andGate first second)
        | _ -> invalidArg (nameof inputs) "andN requires at least two inputs."

    let orN (inputs: int list) =
        if isNull (box inputs) then
            nullArg (nameof inputs)

        match inputs with
        | first :: second :: rest -> rest |> List.fold orGate (orGate first second)
        | _ -> invalidArg (nameof inputs) "orN requires at least two inputs."

    let xorN (inputs: int list) =
        if isNull (box inputs) then
            nullArg (nameof inputs)

        inputs |> List.fold xorGate 0
