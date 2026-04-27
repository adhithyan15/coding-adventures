namespace CodingAdventures.BloomFilter.Tests

open System
open Xunit
open CodingAdventures.BloomFilter

type BloomFilterTests() =
    [<Fact>]
    member _.``Constructor sizes filter``() =
        let filter = BloomFilter<string>(1000, 0.01)
        Assert.True(filter.BitCount > 8000)
        Assert.True(filter.HashCount >= 5)
        Assert.Equal(0, filter.BitsSet)
        Assert.Equal(0, filter.Count)
        Assert.True(filter.SizeBytes * 8 >= filter.BitCount)

    [<Fact>]
    member _.``Explicit constructor sets parameters``() =
        let filter = BloomFilter<string>(1000, 5, true)
        Assert.Equal(1000, filter.BitCount)
        Assert.Equal(5, filter.HashCount)
        Assert.False(filter.IsOverCapacity)

    [<Fact>]
    member _.``Added elements are always found``() =
        let filter = BloomFilter<string>(1000, 0.01)
        let words = [| "apple"; "banana"; "cherry"; "date"; "elderberry" |]

        for word in words do
            filter.Add word

        for word in words do
            Assert.True(filter.Contains word)

        Assert.Equal(words.Length, filter.Size)
        Assert.True(filter.BitsSet > 0)
        Assert.InRange(filter.FillRatio, 0.0, 1.0)
        Assert.True(filter.EstimatedFalsePositiveRate > 0.0)

    [<Fact>]
    member _.``Empty filter rejects missing item``() =
        let filter = BloomFilter<string>(1000, 0.01)
        Assert.False(filter.Contains "not-added")
        Assert.Equal(0.0, filter.FillRatio)
        Assert.Equal(0.0, filter.EstimatedFalsePositiveRate)

    [<Fact>]
    member _.``Works with non string values``() =
        let filter = BloomFilter<int>(1000, 0.01)
        filter.Add 42
        filter.Add 100
        Assert.True(filter.Contains 42)
        Assert.True(filter.Contains 100)

    [<Fact>]
    member _.``Over capacity tracks auto sized filters only``() =
        let auto = BloomFilter<string>(2, 0.1)
        auto.Add "a"
        auto.Add "b"
        Assert.False(auto.IsOverCapacity)
        auto.Add "c"
        Assert.True(auto.IsOverCapacity)

        let explicitFilter = BloomFilter<string>(16, 2, true)
        for i in 0 .. 19 do
            explicitFilter.Add(string i)

        Assert.False(explicitFilter.IsOverCapacity)

    [<Fact>]
    member _.``Utility functions match expected shape``() =
        let m100 = BloomFilter<string>.OptimalM(100L, 0.01)
        let m1000 = BloomFilter<string>.OptimalM(1000L, 0.01)
        Assert.True(m1000 > m100)
        Assert.True(BloomFilter<string>.OptimalM(1000L, 0.001) > m1000)
        Assert.True(BloomFilter<string>.OptimalK(100L, 1000L) >= 1)
        Assert.True(BloomFilter<string>.CapacityForMemory(1_000_000L, 0.01) > 0L)

    [<Fact>]
    member _.``Invalid arguments are rejected``() =
        Assert.Throws<ArgumentException>(fun () -> BloomFilter<string>(0, 0.01) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> BloomFilter<string>(100, 0.0) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> BloomFilter<string>(0, 5, true) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> BloomFilter<string>(100, 0, true) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> BloomFilter<string>.OptimalM(0L, 0.01) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> BloomFilter<string>.OptimalM(100L, 1.0) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> BloomFilter<string>.OptimalK(100L, 0L) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> BloomFilter<string>.CapacityForMemory(0L, 0.01) |> ignore) |> ignore

    [<Fact>]
    member _.``To string includes key fields``() =
        let filter = BloomFilter<string>(100, 0.01)
        filter.Add "a"
        let text = filter.ToString()
        Assert.Contains("BloomFilter", text)
        Assert.Contains("m=", text)
        Assert.Contains("k=", text)
        Assert.Contains("~fp=", text)
