namespace CodingAdventures.Tree.Tests

open System
open CodingAdventures.Tree
open Xunit

type TreeTests() =
    let sample () =
        Tree("root")
            .AddChild("root", "child1")
            .AddChild("root", "child2")
            .AddChild("child1", "grandchild")

    [<Fact>]
    member _.ConstructionAndQueriesWork() =
        let tree = sample ()

        Assert.Equal("root", tree.Root)
        Assert.Equal(4, tree.Size)
        Assert.Equal(2, tree.Height())
        Assert.Equal(Some "child1", tree.Parent "grandchild")
        Assert.Equal(None, tree.Parent "root")
        Assert.Equal<string>([ "child1"; "child2" ], tree.Children "root")
        Assert.Equal<string>([ "child2" ], tree.Siblings "child1")
        Assert.True(tree.IsLeaf "grandchild")
        Assert.True(tree.IsRoot "root")
        Assert.True(tree.HasNode "child2")
        Assert.Equal("Tree(root=root, size=4, height=2)", tree.ToString())

    [<Fact>]
    member _.TraversalsAndPathsAreDeterministic() =
        let tree = sample ()

        Assert.Equal<string>([ "child1"; "child2"; "grandchild"; "root" ], tree.Nodes())
        Assert.Equal<string>([ "child2"; "grandchild" ], tree.Leaves())
        Assert.Equal<string>([ "root"; "child1"; "grandchild"; "child2" ], tree.Preorder())
        Assert.Equal<string>([ "grandchild"; "child1"; "child2"; "root" ], tree.Postorder())
        Assert.Equal<string>([ "root"; "child1"; "child2"; "grandchild" ], tree.LevelOrder())
        Assert.Equal<string>([ "root"; "child1"; "grandchild" ], tree.PathTo "grandchild")
        Assert.Equal("root", tree.LowestCommonAncestor("grandchild", "child2"))
        Assert.Equal("child1", tree.Lca("child1", "grandchild"))

    [<Fact>]
    member _.SubtreeAndRemoveSubtreeWork() =
        let tree = sample ()
        let subtree = tree.Subtree "child1"

        Assert.Equal("child1", subtree.Root)
        Assert.Equal<string>([ "child1"; "grandchild" ], subtree.Preorder())

        tree.RemoveSubtree "child1" |> ignore

        Assert.Equal<string>([ "root"; "child2" ], tree.Preorder())
        Assert.False(tree.HasNode "grandchild")

    [<Fact>]
    member _.ErrorsAreSpecific() =
        let tree = sample ()

        let missing = Assert.Throws<TreeException>(fun () -> tree.AddChild("missing", "x") |> ignore)
        Assert.Equal(TreeErrorKind.NodeNotFound, missing.Kind)
        let duplicate = Assert.Throws<TreeException>(fun () -> tree.AddChild("root", "child1") |> ignore)
        Assert.Equal(TreeErrorKind.DuplicateNode, duplicate.Kind)
        let rootRemoval = Assert.Throws<TreeException>(fun () -> tree.RemoveSubtree "root" |> ignore)
        Assert.Equal(TreeErrorKind.RootRemoval, rootRemoval.Kind)
        Assert.Throws<TreeException>(fun () -> tree.Depth "missing" |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Tree("") |> ignore) |> ignore

    [<Fact>]
    member _.AsciiRenderingIncludesShape() =
        let ascii = (sample ()).ToAscii()

        Assert.Contains("root", ascii)
        Assert.Contains("|-- child1", ascii)
        Assert.Contains("`-- child2", ascii)
