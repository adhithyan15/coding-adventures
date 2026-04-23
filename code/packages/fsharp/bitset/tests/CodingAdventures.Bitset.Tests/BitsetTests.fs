namespace CodingAdventures.Bitset.Tests

open System
open Xunit
open CodingAdventures.Bitset

type BitsetTests() =
    let uint128 value = UInt128.Parse(value)

    [<Fact>]
    member _.``new tracks length capacity and empty queries``() =
        let empty = Bitset(0)
        Assert.Equal(0, empty.Length)
        Assert.Equal(0, empty.Capacity)
        Assert.Equal(0, empty.PopCount())
        Assert.True(empty.None())
        Assert.True(empty.All())
        Assert.True(empty.IsEmpty)
        Assert.Equal(String.Empty, empty.ToBinaryString())
        Assert.Equal(Some 0UL, empty.ToInteger())

        let sized = Bitset(65)
        Assert.Equal(65, sized.Length)
        Assert.Equal(128, sized.Capacity)
        Assert.False(sized.Any())
        Assert.False(sized.Test(64))

    [<Fact>]
    member _.``from integer supports small and cross-word values``() =
        let five = Bitset.FromInteger(uint128 "5")
        Assert.Equal(3, five.Length)
        Assert.True(five.Test(0))
        Assert.False(five.Test(1))
        Assert.True(five.Test(2))
        Assert.Equal("101", five.ToBinaryString())
        Assert.Equal(Some 5UL, five.ToInteger())

        let crossWord = Bitset.FromInteger(uint128 "18446744073709551617")
        Assert.Equal(65, crossWord.Length)
        Assert.True(crossWord.Test(0))
        Assert.True(crossWord.Test(64))
        Assert.Equal(None, crossWord.ToInteger())

    [<Fact>]
    member _.``from binary string understands conventional bit order``() =
        let bitset = Bitset.FromBinaryString("00101")
        Assert.Equal(5, bitset.Length)
        Assert.True(bitset.Test(0))
        Assert.False(bitset.Test(1))
        Assert.True(bitset.Test(2))
        Assert.False(bitset.Test(3))
        Assert.False(bitset.Test(4))
        Assert.Equal("00101", bitset.ToBinaryString())

    [<Fact>]
    member _.``from binary string rejects invalid characters``() =
        Assert.Throws<BitsetError>(fun () -> Bitset.FromBinaryString("10x1") |> ignore)
        |> ignore

    [<Fact>]
    member _.``set clear test and toggle handle single bits``() =
        let bitset = Bitset(10)

        bitset.Set(5)
        Assert.True(bitset.Test(5))
        Assert.Equal(1, bitset.PopCount())

        bitset.Set(5)
        Assert.Equal(1, bitset.PopCount())

        bitset.Clear(5)
        Assert.False(bitset.Test(5))
        Assert.Equal(0, bitset.PopCount())

        bitset.Toggle(2)
        Assert.True(bitset.Test(2))
        bitset.Toggle(2)
        Assert.False(bitset.Test(2))

    [<Fact>]
    member _.``clear and test beyond length are safe no-ops``() =
        let bitset = Bitset(8)
        bitset.Set(1)

        bitset.Clear(100)

        Assert.True(bitset.Test(1))
        Assert.False(bitset.Test(100))
        Assert.Equal(8, bitset.Length)

    [<Fact>]
    member _.``set and toggle auto-grow with doubling capacity``() =
        let bitset = Bitset(100)

        bitset.Set(200)

        Assert.Equal(201, bitset.Length)
        Assert.Equal(256, bitset.Capacity)
        Assert.True(bitset.Test(200))
        Assert.False(bitset.Test(199))

        bitset.Toggle(500)
        Assert.Equal(501, bitset.Length)
        Assert.Equal(512, bitset.Capacity)
        Assert.True(bitset.Test(500))

    [<Fact>]
    member _.``bulk operations match reference truth tables``() =
        let left = Bitset.FromInteger(uint128 "12")
        let right = Bitset.FromInteger(uint128 "10")

        Assert.Equal("1000", left.And(right).ToBinaryString())
        Assert.Equal("1110", left.Or(right).ToBinaryString())
        Assert.Equal("0110", left.Xor(right).ToBinaryString())
        Assert.Equal("0100", left.AndNot(right).ToBinaryString())

    [<Fact>]
    member _.``bulk operations zero-extend shorter inputs``() =
        let shortBitset = Bitset.FromBinaryString("101")
        let longBitset = Bitset.FromBinaryString("100001")

        Assert.Equal("100101", shortBitset.Or(longBitset).ToBinaryString())
        Assert.Equal("000001", shortBitset.And(longBitset).ToBinaryString())
        Assert.Equal("100100", shortBitset.Xor(longBitset).ToBinaryString())
        Assert.Equal(6, shortBitset.Or(longBitset).Length)

    [<Fact>]
    member _.``not only flips bits inside logical length``() =
        let bitset = Bitset(5)
        bitset.Set(0)
        bitset.Set(2)

        let inverted = bitset.Not()

        Assert.Equal("11010", inverted.ToBinaryString())
        Assert.Equal(3, inverted.PopCount())
        Assert.False(inverted.Test(5))
        Assert.True(bitset.Equals(inverted.Not()))

    [<Fact>]
    member _.``any all none and is-empty reflect current state``() =
        let empty = Bitset(0)
        Assert.False(empty.Any())
        Assert.True(empty.All())
        Assert.True(empty.None())
        Assert.True(empty.IsEmpty)

        let partial = Bitset(5)
        partial.Set(0)
        partial.Set(4)
        Assert.True(partial.Any())
        Assert.False(partial.All())
        Assert.False(partial.None())

        let full = Bitset.FromBinaryString("11111")
        Assert.True(full.All())
        Assert.False(full.None())

    [<Fact>]
    member _.``iter set bits yields ascending indices across words``() =
        let bitset = Bitset(130)
        bitset.Set(0)
        bitset.Set(2)
        bitset.Set(64)
        bitset.Set(129)

        Assert.Equal<int array>([| 0; 2; 64; 129 |], bitset.IterSetBits() |> Seq.toArray)
        Assert.True(bitset.Contains(64))

    [<Fact>]
    member _.``binary string round trip preserves leading zeros``() =
        let bitset = Bitset(8)
        bitset.Set(0)
        bitset.Set(7)

        Assert.Equal("10000001", bitset.ToBinaryString())
        Assert.True(bitset.Equals(Bitset.FromBinaryString(bitset.ToBinaryString())))

    [<Fact>]
    member _.``equality uses logical bits``() =
        let left = Bitset.FromBinaryString("101001")
        let right = Bitset(2)
        right.Set(5)
        right.Set(3)
        right.Set(0)

        Assert.True(left.Equals(right))
        Assert.Equal(left.GetHashCode(), right.GetHashCode())

    [<Fact>]
    member _.``negative indices throw``() =
        let bitset = Bitset(4)
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> bitset.Set(-1)) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> bitset.Clear(-1)) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> bitset.Test(-1) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> bitset.Toggle(-1)) |> ignore

    [<Fact>]
    member _.``to string uses binary form``() =
        let bitset = Bitset.FromBinaryString("101")
        Assert.Equal("Bitset(101)", bitset.ToString())
