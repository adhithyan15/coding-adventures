namespace CodingAdventures.AvlTree.Tests

open Xunit
open CodingAdventures.AvlTree

type AvlTreeTests() =
    [<Fact>]
    member _.``rotations rebalance the tree``() =
        let tree = AvlTree<int>.FromValues([ 10; 20; 30 ])

        Assert.Equal<int list>([ 10; 20; 30 ], tree.ToSortedArray())
        Assert.Equal<int option>(Some 20, tree.Root |> Option.map _.Value)
        Assert.True(tree.IsValidBst())
        Assert.True(tree.IsValidAvl())
        Assert.Equal(1, tree.Height())
        Assert.Equal(3, tree.Size())
        Assert.Equal(0, tree.BalanceFactor(tree.Root))

        let descending = AvlTree<int>.FromValues([ 30; 20; 10 ])
        Assert.Equal<int option>(Some 20, descending.Root |> Option.map _.Value)
        Assert.True(descending.IsValidAvl())

    [<Fact>]
    member _.``search and order statistics work``() =
        let tree = AvlTree<int>.FromValues([ 40; 20; 60; 10; 30; 50; 70 ])

        Assert.Equal<int option>(Some 20, tree.Search(20) |> Option.map _.Value)
        Assert.True(tree.Contains(50))
        Assert.Equal<int option>(Some 10, tree.MinValue())
        Assert.Equal<int option>(Some 70, tree.MaxValue())
        Assert.Equal<int option>(Some 30, tree.Predecessor(40))
        Assert.Equal<int option>(Some 50, tree.Successor(40))
        Assert.Equal<int option>(Some 40, tree.KthSmallest(4))
        Assert.Equal(3, tree.Rank(35))

        let deleted = tree.Delete(20)
        Assert.False(deleted.Contains(20))
        Assert.True(deleted.IsValidAvl())
        Assert.True(tree.Contains(20))

    [<Fact>]
    member _.``edge cases and duplicates use options``() =
        let empty = AvlTree<int>.Empty()

        Assert.Equal<AvlNode<int> option>(None, empty.Search(1))
        Assert.Equal<int option>(None, empty.MinValue())
        Assert.Equal<int option>(None, empty.MaxValue())
        Assert.Equal<int option>(None, empty.Predecessor(1))
        Assert.Equal<int option>(None, empty.Successor(1))
        Assert.Equal<int option>(None, empty.KthSmallest(0))
        Assert.Equal(0, empty.Rank(1))
        Assert.Equal(0, empty.BalanceFactor(None))
        Assert.Equal(-1, empty.Height())
        Assert.Equal(0, empty.Size())
        Assert.Equal("AvlTree(root=null, size=0, height=-1)", empty.ToString())

        let tree = AvlTree<int>.FromValues([ 30; 20; 40; 10; 25; 35; 50 ])
        let duplicate = tree.Insert(25)
        Assert.Equal<int list>(tree.ToSortedArray(), duplicate.ToSortedArray())
        Assert.Equal<int list>(tree.ToSortedArray(), tree.Delete(999).ToSortedArray())

        let single = AvlTree<int>(Some(AvlNode.Leaf 5))
        Assert.Equal<int option>(Some 5, single.Root |> Option.map _.Value)
        Assert.Equal(0, single.Height())
        Assert.Equal<int list>([ 2 ], AvlTree<int>.FromValues([ 1; 2 ]).Delete(1).ToSortedArray())
        Assert.Equal<int list>([ 1 ], AvlTree<int>.FromValues([ 2; 1 ]).Delete(2).ToSortedArray())

    [<Fact>]
    member _.``double rotations and validation failures are covered``() =
        let leftRight = AvlTree<int>.FromValues([ 30; 10; 20 ])
        let rightLeft = AvlTree<int>.FromValues([ 10; 30; 20 ])

        Assert.Equal<int option>(Some 20, leftRight.Root |> Option.map _.Value)
        Assert.Equal<int option>(Some 20, rightLeft.Root |> Option.map _.Value)
        Assert.True(leftRight.IsValidAvl())
        Assert.True(rightLeft.IsValidAvl())

        let badOrder =
            AvlTree<int>(
                Some
                    { Value = 5
                      Left = Some(AvlNode.Leaf 6)
                      Right = None
                      Height = 1
                      Size = 2 }
            )

        let badRightOrder =
            AvlTree<int>(
                Some
                    { Value = 5
                      Left = None
                      Right = Some(AvlNode.Leaf 4)
                      Height = 1
                      Size = 2 }
            )

        let badHeight =
            AvlTree<int>(
                Some
                    { Value = 5
                      Left = Some(AvlNode.Leaf 3)
                      Right = None
                      Height = 99
                      Size = 2 }
            )

        Assert.False(badOrder.IsValidBst())
        Assert.False(badOrder.IsValidAvl())
        Assert.False(badRightOrder.IsValidBst())
        Assert.False(badRightOrder.IsValidAvl())
        Assert.False(badHeight.IsValidAvl())

    [<Fact>]
    member _.``delete with nested successor and module helpers work``() =
        let tree = AvlTree<int>.FromValues([ 5; 3; 8; 7; 9; 6 ])

        let deleted = tree.Delete(5)
        Assert.Equal<int list>([ 3; 6; 7; 8; 9 ], deleted.ToSortedArray())
        Assert.True(deleted.IsValidAvl())
        Assert.Equal<int option>(Some 3, tree.KthSmallest(1))
        Assert.Equal<int option>(Some 9, tree.KthSmallest(6))
        Assert.Equal(1, tree.Rank(5))

        let root =
            None
            |> AvlTreeAlgorithms.insert 2
            |> AvlTreeAlgorithms.insert 1
            |> AvlTreeAlgorithms.insert 3

        Assert.Equal<int option>(Some 2, AvlTreeAlgorithms.search 2 root |> Option.map _.Value)
        Assert.Equal<int option>(Some 1, AvlTreeAlgorithms.minValue root)
        Assert.Equal<int option>(Some 3, AvlTreeAlgorithms.maxValue root)
        Assert.Equal<int option>(Some 2, AvlTreeAlgorithms.kthSmallest 2 root)
        Assert.Equal(1, AvlTreeAlgorithms.rank 2 root)
        Assert.Equal(0, AvlTreeAlgorithms.balanceFactor root)
        Assert.True(AvlTreeAlgorithms.isValidBst root)
        Assert.True(AvlTreeAlgorithms.isValidAvl root)
        Assert.Equal<int list>([ 1; 2; 3 ], AvlTreeAlgorithms.toSortedArray root)
        Assert.Equal<int list>([ 2; 3 ], AvlTreeAlgorithms.delete 1 root |> AvlTreeAlgorithms.toSortedArray)

    [<Fact>]
    member _.``reference comparable values retain sorted order``() =
        let tree = AvlTree<string>.FromValues([ "delta"; "alpha"; "gamma" ])

        Assert.Equal<string list>([ "alpha"; "delta"; "gamma" ], tree.ToSortedArray())
        Assert.Equal("AvlTree(root=\"delta\", size=3, height=1)", tree.ToString())
        Assert.Equal<string option>(Some "gamma", tree.Successor("delta"))
        Assert.Equal<string option>(None, tree.Predecessor("alpha"))
