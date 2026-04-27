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

    [<Fact>]
    member _.``supports peek maxheap and heap sort``() =
        let minHeap = MinHeap<int>.FromEnumerable([ 4; 1; 3 ])
        Assert.Equal(1, minHeap.Peek())
        Assert.False(minHeap.IsEmpty())
        Assert.True(minHeap.ToArray() = [ 1; 4; 3 ])

        let maxHeap = MaxHeap<int>()
        maxHeap.Push 2
        maxHeap.Push 5
        maxHeap.Push 1
        Assert.Equal(5, maxHeap.Pop())
        Assert.True(HeapFunctions.heapSort [ 3; 1; 2 ] = [ 1; 2; 3 ])

    [<Fact>]
    member _.``throws on empty heap access``() =
        let heap = MinHeap<int>()
        Assert.ThrowsAny<System.InvalidOperationException>(fun () -> heap.Peek() |> ignore) |> ignore
        Assert.ThrowsAny<System.InvalidOperationException>(fun () -> heap.Pop() |> ignore) |> ignore
