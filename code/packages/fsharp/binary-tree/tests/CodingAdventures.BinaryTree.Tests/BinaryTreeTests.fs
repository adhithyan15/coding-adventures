namespace CodingAdventures.BinaryTree.Tests

open System
open Xunit
open CodingAdventures.BinaryTree

type BinaryTreeTests() =
    let fullTree() =
        BinaryTree<int>.FromLevelOrder(
            [ Some 1
              Some 2
              Some 3
              Some 4
              Some 5
              Some 6
              Some 7 ]
        )

    [<Fact>]
    member _.``from level order projects back to an array``() =
        let tree = fullTree()

        Assert.Equal<int option list>(
            [ Some 1
              Some 2
              Some 3
              Some 4
              Some 5
              Some 6
              Some 7 ],
            tree.ToArray()
        )

        Assert.Equal<int list>([ 1; 2; 3; 4; 5; 6; 7 ], tree.LevelOrder())

    [<Fact>]
    member _.``shape queries distinguish full complete and perfect trees``() =
        let perfect = fullTree()
        Assert.True(perfect.IsFull())
        Assert.True(perfect.IsComplete())
        Assert.True(perfect.IsPerfect())
        Assert.Equal(2, perfect.Height())
        Assert.Equal(7, perfect.Size())

        let complete = BinaryTree<int>.FromLevelOrder([ Some 1; Some 2; Some 3; Some 4; None; None; None ])
        Assert.False(complete.IsFull())
        Assert.True(complete.IsComplete())
        Assert.False(complete.IsPerfect())

        let incomplete = BinaryTree<int>.FromLevelOrder([ Some 1; None; Some 3 ])
        Assert.False(incomplete.IsComplete())

    [<Fact>]
    member _.``traversals and child lookup match reference order``() =
        let tree = BinaryTree<int>.FromLevelOrder([ Some 1; Some 2; Some 3; Some 4; None; Some 5; None ])

        Assert.Equal<int list>([ 4; 2; 1; 5; 3 ], tree.InOrder())
        Assert.Equal<int list>([ 1; 2; 4; 3; 5 ], tree.PreOrder())
        Assert.Equal<int list>([ 4; 2; 5; 3; 1 ], tree.PostOrder())
        Assert.Equal<int list>([ 1; 2; 3; 4; 5 ], tree.LevelOrder())
        Assert.Equal<int option>(Some 2, tree.LeftChild(1) |> Option.map _.Value)
        Assert.Equal<int option>(Some 3, tree.RightChild(1) |> Option.map _.Value)
        Assert.Equal<BinaryTreeNode<int> option>(None, tree.Find(99))

    [<Fact>]
    member _.``empty and singleton trees expose edge case behavior``() =
        let empty = BinaryTree<string>.Empty()
        Assert.Equal(-1, empty.Height())
        Assert.Equal(0, empty.Size())
        Assert.True(empty.IsFull())
        Assert.True(empty.IsComplete())
        Assert.True(empty.IsPerfect())
        Assert.Equal<string list>([], empty.InOrder())
        Assert.Equal<string option list>([], empty.ToArray())
        Assert.Equal(String.Empty, empty.ToAscii())
        Assert.Equal("BinaryTree(root=null, size=0)", empty.ToString())

        let single = BinaryTree<string>.Singleton("root")
        Assert.Equal<string option>(Some "root", single.Root |> Option.map _.Value)
        Assert.Equal<string option list>([ Some "root" ], single.ToArray())
        Assert.Equal("BinaryTree(root=\"root\", size=1)", single.ToString())

    [<Fact>]
    member _.``ascii rendering contains debug values``() =
        let tree = BinaryTree<string>.FromLevelOrder([ Some "root"; Some "left"; Some "right" ])
        let ascii = tree.ToAscii()

        Assert.Contains("root", ascii)
        Assert.Contains("left", ascii)
        Assert.Contains("right", ascii)
        Assert.Contains("`--", ascii)

    [<Fact>]
    member _.``module functions support raw root composition``() =
        let left = Some(BinaryTreeNode.Leaf 2)
        let right = Some(BinaryTreeNode.Leaf 3)
        let root = Some(BinaryTreeNode.Create(1, left, right))

        Assert.Equal<int option>(Some 2, BinaryTreeAlgorithms.leftChild 1 root |> Option.map _.Value)
        Assert.Equal<int option>(Some 3, BinaryTreeAlgorithms.rightChild 1 root |> Option.map _.Value)
        Assert.Equal<int list>([ 2; 1; 3 ], BinaryTreeAlgorithms.inOrder root)
        Assert.Equal<int list>([ 1; 2; 3 ], BinaryTreeAlgorithms.preOrder root)
        Assert.Equal<int list>([ 2; 3; 1 ], BinaryTreeAlgorithms.postOrder root)
        Assert.Equal<int list>([ 1; 2; 3 ], BinaryTreeAlgorithms.levelOrder root)
        Assert.True(BinaryTreeAlgorithms.isFull root)
        Assert.True(BinaryTreeAlgorithms.isComplete root)
        Assert.True(BinaryTreeAlgorithms.isPerfect root)
        Assert.Equal(1, BinaryTreeAlgorithms.height root)
        Assert.Equal(3, BinaryTreeAlgorithms.size root)
        Assert.Contains("1", BinaryTreeAlgorithms.toAscii root)
