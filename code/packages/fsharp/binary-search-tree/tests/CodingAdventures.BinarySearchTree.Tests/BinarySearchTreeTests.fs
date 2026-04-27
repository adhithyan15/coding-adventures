namespace CodingAdventures.BinarySearchTree.Tests

open Xunit
open CodingAdventures.BinarySearchTree

type BinarySearchTreeTests() =
    let populated() =
        [ 5; 1; 8; 3; 7 ]
        |> List.fold (fun (tree: BinarySearchTree<int>) value -> tree.Insert(value)) (BinarySearchTree<int>.Empty())

    [<Fact>]
    member _.``insert search and delete work immutably``() =
        let tree = populated()

        Assert.Equal<int list>([ 1; 3; 5; 7; 8 ], tree.ToSortedArray())
        Assert.Equal(5, tree.Size())
        Assert.True(tree.Contains(7))
        Assert.Equal(Some 7, tree.Search(7) |> Option.map _.Value)
        Assert.Equal(Some 1, tree.MinValue())
        Assert.Equal(Some 8, tree.MaxValue())
        Assert.Equal(Some 3, tree.Predecessor(5))
        Assert.Equal(Some 7, tree.Successor(5))
        Assert.Equal(2, tree.Rank(4))
        Assert.Equal(Some 7, tree.KthSmallest(4))

        let deleted = tree.Delete(5)
        Assert.False(deleted.Contains(5))
        Assert.True(deleted.IsValid())
        Assert.True(tree.Contains(5))

    [<Fact>]
    member _.``from sorted array builds a balanced tree``() =
        let tree = BinarySearchTree<int>.FromSortedArray([ 1; 2; 3; 4; 5; 6; 7 ])

        Assert.Equal<int list>([ 1; 2; 3; 4; 5; 6; 7 ], tree.ToSortedArray())
        Assert.Equal(2, tree.Height())
        Assert.Equal(7, tree.Size())
        Assert.True(tree.IsValid())
        Assert.Equal(Some 4, tree.Root |> Option.map _.Value)

    [<Fact>]
    member _.``empty tree returns options and neutral metrics``() =
        let tree = BinarySearchTree<int>.Empty()

        Assert.Equal<BstNode<int> option>(None, tree.Search(1))
        Assert.Equal<int option>(None, tree.MinValue())
        Assert.Equal<int option>(None, tree.MaxValue())
        Assert.Equal<int option>(None, tree.Predecessor(1))
        Assert.Equal<int option>(None, tree.Successor(1))
        Assert.Equal<int option>(None, tree.KthSmallest(0))
        Assert.Equal<int option>(None, tree.KthSmallest(1))
        Assert.Equal(0, tree.Rank(1))
        Assert.Equal(-1, tree.Height())
        Assert.Equal(0, tree.Size())
        Assert.Equal("BinarySearchTree(root=null, size=0)", tree.ToString())

    [<Fact>]
    member _.``duplicates and single child delete keep persistent shape``() =
        let tree = BinarySearchTree<int>.FromSortedArray([ 2; 4; 6; 8 ])

        Assert.Equal(Some 6, tree.Root |> Option.map _.Value)

        let duplicate = tree.Insert(4)
        Assert.Equal<int list>(tree.ToSortedArray(), duplicate.ToSortedArray())
        Assert.Equal<int list>([ 4; 6; 8 ], tree.Delete(2).ToSortedArray())

    [<Fact>]
    member _.``validation catches bad ordering and stale sizes``() =
        let badOrder =
            BinarySearchTree<int>(
                Some
                    { Value = 5
                      Left = Some(BstNode.Leaf 6)
                      Right = None
                      Size = 2 }
            )

        let badSize =
            BinarySearchTree<int>(
                Some
                    { Value = 5
                      Left = Some(BstNode.Leaf 3)
                      Right = None
                      Size = 99 }
            )

        Assert.False(badOrder.IsValid())
        Assert.False(badSize.IsValid())

    [<Fact>]
    member _.``module functions support raw root composition``() =
        let root =
            None
            |> BinarySearchTreeAlgorithms.insert 5
            |> BinarySearchTreeAlgorithms.insert 2
            |> BinarySearchTreeAlgorithms.insert 9

        Assert.Equal(Some 5, BinarySearchTreeAlgorithms.search 5 root |> Option.map _.Value)
        Assert.Equal(Some 2, BinarySearchTreeAlgorithms.minValue root)
        Assert.Equal(Some 9, BinarySearchTreeAlgorithms.maxValue root)
        Assert.Equal(Some 5, BinarySearchTreeAlgorithms.kthSmallest 2 root)
        Assert.Equal(1, BinarySearchTreeAlgorithms.rank 5 root)
        Assert.Equal(1, BinarySearchTreeAlgorithms.height root)
        Assert.Equal(3, BinarySearchTreeAlgorithms.size root)
        Assert.True(BinarySearchTreeAlgorithms.isValid root)
        Assert.Equal<int list>([ 2; 5; 9 ], BinarySearchTreeAlgorithms.toSortedArray root)

        let deleted = BinarySearchTreeAlgorithms.delete 2 root
        Assert.Equal<int list>([ 5; 9 ], BinarySearchTreeAlgorithms.toSortedArray deleted)

    [<Fact>]
    member _.``reference comparable values retain sorted order``() =
        let tree =
            BinarySearchTree<string>.Empty()
                .Insert("delta")
                .Insert("alpha")
                .Insert("gamma")

        Assert.Equal<string list>([ "alpha"; "delta"; "gamma" ], tree.ToSortedArray())
        Assert.Equal("BinarySearchTree(root=\"delta\", size=3)", tree.ToString())
        Assert.Equal(Some "gamma", tree.Successor("delta"))
        Assert.Equal<string option>(None, tree.Predecessor("alpha"))
