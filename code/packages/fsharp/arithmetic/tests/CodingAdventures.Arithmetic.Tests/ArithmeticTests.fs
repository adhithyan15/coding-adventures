namespace CodingAdventures.Arithmetic.Tests

open Xunit
open CodingAdventures.Arithmetic.FSharp

module ArithmeticTests =
    let private intToBits value width =
        [ for i in 0 .. width - 1 -> (value >>> i) &&& 1 ]

    let private bitsToInt bits =
        bits |> List.mapi (fun i bit -> bit <<< i) |> List.sum

    [<Theory>]
    [<InlineData(0, 0, 0, 0)>]
    [<InlineData(0, 1, 1, 0)>]
    [<InlineData(1, 0, 1, 0)>]
    [<InlineData(1, 1, 0, 1)>]
    let ``half adder matches truth table`` a b expectedSum expectedCarry =
        Assert.Equal((expectedSum, expectedCarry), Adders.halfAdder a b)

    [<Theory>]
    [<InlineData(0, 0, 0, 0, 0)>]
    [<InlineData(0, 0, 1, 1, 0)>]
    [<InlineData(0, 1, 1, 0, 1)>]
    [<InlineData(1, 0, 1, 0, 1)>]
    [<InlineData(1, 1, 0, 0, 1)>]
    [<InlineData(1, 1, 1, 1, 1)>]
    let ``full adder matches truth table`` a b carryIn expectedSum expectedCarry =
        Assert.Equal((expectedSum, expectedCarry), Adders.fullAdder a b carryIn)

    [<Fact>]
    let ``ripple carry adder adds and carries`` () =
        let result = Adders.rippleCarryAdder (intToBits 15 4) (intToBits 1 4) 0

        Assert.Equal(0, bitsToInt result.Sum)
        Assert.Equal(1, result.CarryOut)

    [<Fact>]
    let ``ripple carry adder supports carry in`` () =
        let result = Adders.rippleCarryAdder (intToBits 1 4) (intToBits 1 4) 1

        Assert.Equal(3, bitsToInt result.Sum)
        Assert.Equal(0, result.CarryOut)

    [<Fact>]
    let ``ripple carry adder validates inputs`` () =
        Assert.Throws<System.ArgumentException>(fun () -> Adders.rippleCarryAdder [ 0; 1 ] [ 0; 1; 0 ] 0 |> ignore) |> ignore
        Assert.Throws<System.ArgumentException>(fun () -> Adders.rippleCarryAdder [] [] 0 |> ignore) |> ignore
        Assert.Throws<System.ArgumentOutOfRangeException>(fun () -> Adders.rippleCarryAdder [ 2 ] [ 0 ] 0 |> ignore) |> ignore

    [<Fact>]
    let ``alu adds subtracts and sets flags`` () =
        let alu = Alu 8

        let add = alu.Execute(AluOp.Add, intToBits 255 8, intToBits 1 8)
        let sub = alu.Execute(AluOp.Sub, intToBits 5 8, intToBits 3 8)

        Assert.Equal(0, bitsToInt add.Value)
        Assert.True add.Carry
        Assert.True add.Zero
        Assert.Equal(2, bitsToInt sub.Value)
        Assert.False sub.Zero

    [<Fact>]
    let ``alu bitwise operations`` () =
        let cases =
            [ AluOp.And, 0xCC, 0xAA, 0x88
              AluOp.Or, 0xCC, 0xAA, 0xEE
              AluOp.Xor, 0xCC, 0xAA, 0x66 ]

        for op, a, b, expected in cases do
            let result = (Alu 8).Execute(op, intToBits a 8, intToBits b 8)

            Assert.Equal(expected, bitsToInt result.Value)
            Assert.False result.Carry
            Assert.False result.Overflow

    [<Fact>]
    let ``alu not ignores b`` () =
        let result = (Alu 8).Execute(AluOp.Not, intToBits 0 8, [])

        Assert.Equal(255, bitsToInt result.Value)
        Assert.True result.Negative

    [<Fact>]
    let ``alu detects signed overflow`` () =
        let result = (Alu 8).Execute(AluOp.Add, intToBits 127 8, intToBits 1 8)

        Assert.True result.Overflow
        Assert.True result.Negative
        Assert.Equal<int list>(result.Value, result.Result)

    [<Fact>]
    let ``alu validates width`` () =
        let alu = Alu 8

        Assert.Throws<System.ArgumentException>(fun () -> alu.Execute(AluOp.Add, [ 0; 1 ], [ 0; 1 ]) |> ignore) |> ignore
        Assert.Throws<System.ArgumentException>(fun () -> alu.Execute(AluOp.And, intToBits 1 8, [ 1 ]) |> ignore) |> ignore
        Assert.Throws<System.ArgumentException>(fun () -> Alu 0 |> ignore) |> ignore
