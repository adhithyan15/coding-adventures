namespace CodingAdventures.Heap.FSharp.Tests

open CodingAdventures.Heap.FSharp
open Xunit

type HeapTests() =
    [<Fact>]
    member _.``pushes and pops in order``() =
        let heap = MinHeap<int>()
        heap.Push 3
        heap.Push 1
        heap.Push 2

        Assert.Equal(1, heap.Pop())
        Assert.Equal(2, heap.Pop())
        Assert.Equal(3, heap.Pop())
