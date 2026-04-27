namespace CodingAdventures.Arithmetic.FSharp

open System
open CodingAdventures.LogicGates

type RippleCarryResult =
    { Sum: int list
      CarryOut: int
      Overflow: bool }

[<RequireQualifiedAccess>]
module Adders =
    let halfAdder a b =
        LogicGates.xorGate a b, LogicGates.andGate a b

    let fullAdder a b carryIn =
        let partialSum, partialCarry = halfAdder a b
        let sum, carry2 = halfAdder partialSum carryIn
        sum, LogicGates.orGate partialCarry carry2

    let rippleCarryAdder (a: int list) (b: int list) (carryIn: int) =
        if isNull (box a) then
            nullArg "a"

        if isNull (box b) then
            nullArg "b"

        if a.Length <> b.Length then
            invalidArg "b" $"a and b must have the same length, got {a.Length} and {b.Length}."

        if List.isEmpty a then
            invalidArg "a" "bit lists must not be empty."

        let mutable carry = carryIn
        let sum = ResizeArray<int>()

        List.zip a b
        |> List.iter (fun (ai, bi) ->
            let sumBit, carryOut = fullAdder ai bi carry
            sum.Add sumBit
            carry <- carryOut)

        let aSign = List.last a
        let bSign = List.last b
        let resultSign = sum[sum.Count - 1]
        let overflow = aSign = bSign && resultSign <> aSign

        { Sum = sum |> Seq.toList
          CarryOut = carry
          Overflow = overflow }

type AluOp =
    | Add
    | Sub
    | And
    | Or
    | Xor
    | Not

type AluResult =
    { Value: int list
      Zero: bool
      Carry: bool
      Negative: bool
      Overflow: bool }

    member this.Result = this.Value

type Alu(?bitWidth: int) =
    let bitWidth = defaultArg bitWidth 8

    do
        if bitWidth < 1 then
            invalidArg "bitWidth" "bit_width must be at least 1."

    member _.BitWidth = bitWidth

    member _.Execute(op: AluOp, a: int list, b: int list) =
        if isNull (box a) then
            nullArg "a"

        if isNull (box b) then
            nullArg "b"

        if a.Length <> bitWidth then
            invalidArg "a" $"a must have {bitWidth} bits, got {a.Length}."

        if op <> AluOp.Not && b.Length <> bitWidth then
            invalidArg "b" $"b must have {bitWidth} bits, got {b.Length}."

        let value, carry =
            match op with
            | AluOp.Add ->
                let result = Adders.rippleCarryAdder a b 0
                result.Sum, result.CarryOut = 1
            | AluOp.Sub ->
                let notB = b |> List.map LogicGates.notGate
                let result = Adders.rippleCarryAdder a notB 1
                result.Sum, result.CarryOut = 1
            | AluOp.And ->
                List.map2 LogicGates.andGate a b, false
            | AluOp.Or ->
                List.map2 LogicGates.orGate a b, false
            | AluOp.Xor ->
                List.map2 LogicGates.xorGate a b, false
            | AluOp.Not ->
                a |> List.map LogicGates.notGate, false

        let zero = value |> List.forall ((=) 0)
        let negative = value |> List.last = 1

        let overflow =
            match op with
            | AluOp.Add ->
                let aSign = List.last a
                let bSign = List.last b
                let resultSign = List.last value
                aSign = bSign && resultSign <> aSign
            | AluOp.Sub ->
                let aSign = List.last a
                let bSign = b |> List.last |> LogicGates.notGate
                let resultSign = List.last value
                aSign = bSign && resultSign <> aSign
            | _ -> false

        { Value = value
          Zero = zero
          Carry = carry
          Negative = negative
          Overflow = overflow }

[<RequireQualifiedAccess>]
module ArithmeticPackage =
    [<Literal>]
    let Version = "0.1.0"
