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
