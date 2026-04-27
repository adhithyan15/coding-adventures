namespace CodingAdventures.BitsetNative.Tests

open System
open System.Collections
open System.Collections.Generic
open Xunit
open CodingAdventures.BitsetNative

module private Helpers =
    let uint128 (value: string) = UInt128.Parse(value)

type BitsetNativeTests() =
    [<Fact>]
    member _.``new tracks length capacity and empty queries``() =
        use empty = new Bitset(0)
        Assert.Equal(0, empty.Length)
        Assert.Equal(0, empty.Capacity)
        Assert.Equal(0, empty.PopCount())
        Assert.True(empty.None())
        Assert.True(empty.All())
        Assert.True(empty.IsEmpty)
        Assert.Equal(String.Empty, empty.ToBinaryString())
        Assert.Equal(Some 0UL, empty.ToInteger())

        use sized = new Bitset(65)
        Assert.Equal(65, sized.Length)
        Assert.Equal(128, sized.Capacity)
        Assert.False(sized.Any())
        Assert.False(sized.Test(64))

    [<Fact>]
    member _.``from integer supports small and cross-word values``() =
        use five = Bitset.FromInteger(Helpers.uint128 "5")
        Assert.Equal(3, five.Length)
        Assert.True(five.Test(0))
        Assert.False(five.Test(1))
        Assert.True(five.Test(2))
        Assert.Equal("101", five.ToBinaryString())
        Assert.Equal(Some 5UL, five.ToInteger())

        use crossWord = Bitset.FromInteger(Helpers.uint128 "18446744073709551617")
        Assert.Equal(65, crossWord.Length)
        Assert.True(crossWord.Test(0))
        Assert.True(crossWord.Test(64))
        Assert.Equal(None, crossWord.ToInteger())

    [<Fact>]
    member _.``from binary string understands conventional bit order``() =
        use bitset = Bitset.FromBinaryString("00101")
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
        use bitset = new Bitset(10)

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
        use bitset = new Bitset(8)
        bitset.Set(1)

        bitset.Clear(100)

        Assert.True(bitset.Test(1))
        Assert.False(bitset.Test(100))
        Assert.Equal(8, bitset.Length)

    [<Fact>]
    member _.``set and toggle auto-grow with doubling capacity``() =
        use bitset = new Bitset(100)

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
        use left = Bitset.FromInteger(Helpers.uint128 "12")
        use right = Bitset.FromInteger(Helpers.uint128 "10")
        use andResult = left.And(right)
        use orResult = left.Or(right)
        use xorResult = left.Xor(right)
        use andNotResult = left.AndNot(right)

        Assert.Equal("1000", andResult.ToBinaryString())
        Assert.Equal("1110", orResult.ToBinaryString())
        Assert.Equal("0110", xorResult.ToBinaryString())
        Assert.Equal("0100", andNotResult.ToBinaryString())

    [<Fact>]
    member _.``bulk operations zero-extend shorter inputs``() =
        use shortBitset = Bitset.FromBinaryString("101")
        use longBitset = Bitset.FromBinaryString("100001")
        use orResult = shortBitset.Or(longBitset)
        use andResult = shortBitset.And(longBitset)
        use xorResult = shortBitset.Xor(longBitset)

        Assert.Equal("100101", orResult.ToBinaryString())
        Assert.Equal("000001", andResult.ToBinaryString())
        Assert.Equal("100100", xorResult.ToBinaryString())
        Assert.Equal(6, orResult.Length)

    [<Fact>]
    member _.``not only flips bits inside logical length``() =
        use bitset = new Bitset(5)
        bitset.Set(0)
        bitset.Set(2)

        use inverted = bitset.Not()
        use roundTrip = inverted.Not()

        Assert.Equal("11010", inverted.ToBinaryString())
        Assert.Equal(3, inverted.PopCount())
        Assert.False(inverted.Test(5))
        Assert.True(bitset.Equals(roundTrip))

    [<Fact>]
    member _.``any all none and is-empty reflect current state``() =
        use empty = new Bitset(0)
        Assert.False(empty.Any())
        Assert.True(empty.All())
        Assert.True(empty.None())
        Assert.True(empty.IsEmpty)

        use partial = new Bitset(5)
        partial.Set(0)
        partial.Set(4)
        Assert.True(partial.Any())
        Assert.False(partial.All())
        Assert.False(partial.None())

        use full = Bitset.FromBinaryString("11111")
        Assert.True(full.All())
        Assert.False(full.None())

    [<Fact>]
    member _.``iter set bits yields ascending indices across words``() =
        use bitset = new Bitset(130)
        bitset.Set(0)
        bitset.Set(2)
        bitset.Set(64)
        bitset.Set(129)

        Assert.Equal<int array>([| 0; 2; 64; 129 |], bitset.IterSetBits() |> Seq.toArray)
        Assert.True(bitset.Contains(64))

    [<Fact>]
    member _.``binary string round trip preserves leading zeros``() =
        use bitset = new Bitset(8)
        bitset.Set(0)
        bitset.Set(7)

        Assert.Equal("10000001", bitset.ToBinaryString())

        use roundTrip = Bitset.FromBinaryString(bitset.ToBinaryString())
        Assert.True(bitset.Equals(roundTrip))

    [<Fact>]
    member _.``equality uses logical bits``() =
        use left = Bitset.FromBinaryString("101001")
        use right = new Bitset(2)
        right.Set(5)
        right.Set(3)
        right.Set(0)

        Assert.True(left.Equals(right))
        Assert.Equal(left.GetHashCode(), right.GetHashCode())

    [<Fact>]
    member _.``negative indices throw``() =
        use bitset = new Bitset(4)
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> bitset.Set(-1)) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> bitset.Clear(-1)) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> bitset.Test(-1) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> bitset.Toggle(-1)) |> ignore

    [<Fact>]
    member _.``constructors and bulk operations reject invalid managed inputs``() =
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> new Bitset(-1) |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Bitset.FromBinaryString(null) |> ignore) |> ignore

        use bitset = new Bitset(4)
        let nullBitset = null : Bitset

        Assert.Throws<ArgumentNullException>(fun () -> bitset.And(nullBitset) |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> bitset.Or(nullBitset) |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> bitset.Xor(nullBitset) |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> bitset.AndNot(nullBitset) |> ignore) |> ignore

    [<Fact>]
    member _.``equality and collection interfaces use managed conventions``() =
        use left = Bitset.FromBinaryString("101001")
        use right = Bitset.FromBinaryString("101001")

        Assert.True(left.Equals(left))
        Assert.False(left.Equals(null : Bitset))
        Assert.True(left.Equals(right :> obj))
        Assert.False(left.Equals("101001" :> obj))
        Assert.True((left :> IEquatable<Bitset>).Equals(right))

        let genericEnumerator = (left :> IEnumerable<int>).GetEnumerator()
        Assert.True(genericEnumerator.MoveNext())
        Assert.Equal(0, genericEnumerator.Current)
        Assert.True(genericEnumerator.MoveNext())
        Assert.Equal(3, genericEnumerator.Current)

        let nongenericEnumerator = (left :> IEnumerable).GetEnumerator()
        Assert.True(nongenericEnumerator.MoveNext())
        Assert.Equal(box 0, nongenericEnumerator.Current)

    [<Fact>]
    member _.``dispose prevents further use``() =
        let bitset = new Bitset(4)
        (bitset :> IDisposable).Dispose()
        (bitset :> IDisposable).Dispose()

        Assert.Throws<ObjectDisposedException>(fun () -> bitset.Set(0)) |> ignore

    [<Fact>]
    member _.``to string uses binary form``() =
        use bitset = Bitset.FromBinaryString("101")
        Assert.Equal("Bitset(101)", bitset.ToString())
