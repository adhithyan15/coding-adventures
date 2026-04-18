using CodingAdventures.PaintInstructions;
using CodingAdventures.PaintVm;
using static CodingAdventures.PaintInstructions.PaintInstructions;

namespace CodingAdventures.PaintVm.Tests;

public sealed class PaintVmTests
{
    [Fact]
    public void Version_IsSemver()
    {
        Assert.Equal("0.1.0", PaintVmPackage.VERSION);
    }

    [Fact]
    public void Register_RejectsDuplicateKinds()
    {
        var vm = new PaintVM<List<string>>((_, _, _, _) => { });
        vm.Register("rect", (_, _, _) => { });

        var error = Assert.Throws<DuplicateHandlerError>(() => vm.Register("rect", (_, _, _) => { }));
        Assert.Equal("rect", error.Kind);
    }

    [Fact]
    public void Dispatch_UsesSpecificHandlerBeforeWildcard()
    {
        var log = new List<string>();
        var vm = new PaintVM<List<string>>((_, _, _, _) => { });
        vm.Register("*", (instruction, context, _) => context.Add($"wildcard:{instruction.Kind}"));
        vm.Register("rect", (_, context, _) => context.Add("specific:rect"));

        vm.Dispatch(PaintRect(0, 0, 10, 10), log);

        Assert.Equal(["specific:rect"], log);
    }

    [Fact]
    public void Dispatch_ThrowsForUnknownInstructionKind()
    {
        var vm = new PaintVM<List<string>>((_, _, _, _) => { });

        var error = Assert.Throws<UnknownInstructionError>(() => vm.Dispatch(PaintRect(0, 0, 10, 10), []));
        Assert.Equal("rect", error.Kind);
    }

    [Fact]
    public void Execute_ClearsThenDispatchesInOrder()
    {
        var log = new List<string>();
        var vm = CreateVm();
        var scene = PaintScene(100, 50, "#f0f0f0", [PaintRect(0, 0, 10, 10), PaintEllipse(20, 20, 5, 5)]);

        vm.Execute(scene, log);

        Assert.Equal(["clear:#f0f0f0", "rect:0,0", "ellipse:20,20"], log);
    }

    [Fact]
    public void Execute_ThrowsWhenContextIsNull()
    {
        var vm = CreateVm();
        var scene = PaintScene(1, 1, "#fff", []);

        Assert.Throws<NullContextError>(() => vm.Execute(scene, null!));
    }

    [Fact]
    public void Patch_WithoutCallbacksFallsBackToExecute()
    {
        var log = new List<string>();
        var vm = CreateVm();
        var next = PaintScene(100, 50, "#fff", [PaintRect(0, 0, 20, 20)]);

        vm.Patch(PaintScene(100, 50, "#fff", [PaintRect(0, 0, 10, 10)]), next, log);

        Assert.Equal(["clear:#fff", "rect:0,0"], log);
    }

    [Fact]
    public void Patch_ReportsDeleteUpdateAndInsertOperations()
    {
        var deletions = new List<string>();
        var updates = new List<string>();
        var insertions = new List<string>();
        var vm = CreateVm();

        var oldScene = PaintScene(100, 50, "#fff",
        [
            PaintRect(0, 0, 10, 10, new PaintRectOptions { Id = "keep" }),
            PaintRect(20, 0, 10, 10, new PaintRectOptions { Id = "delete-me" }),
        ]);

        var newScene = PaintScene(100, 50, "#fff",
        [
            PaintRect(0, 0, 12, 12, new PaintRectOptions { Id = "keep" }),
            PaintEllipse(50, 50, 5, 5),
        ]);

        vm.Patch(oldScene, newScene, new List<string>(), new PatchCallbacks
        {
            OnDelete = instruction => deletions.Add(instruction.Id ?? instruction.Kind),
            OnUpdate = (oldInstruction, newInstruction) => updates.Add($"{oldInstruction.Kind}->{newInstruction.Kind}"),
            OnInsert = (instruction, index) => insertions.Add($"{index}:{instruction.Kind}"),
        });

        Assert.Equal(["delete-me"], deletions);
        Assert.Equal(["rect->rect", "rect->ellipse"], updates);
        Assert.Empty(insertions);
    }

    [Fact]
    public void Export_ThrowsWhenBackendDoesNotProvideReadback()
    {
        var vm = CreateVm();

        Assert.Throws<ExportNotSupportedError>(() => vm.Export(PaintScene(10, 10, "#fff", [])));
    }

    [Fact]
    public void Export_UsesProvidedOffscreenRenderer()
    {
        var vm = new PaintVM<List<string>>(
            (_, _, _, _) => { },
            (scene, _, options) =>
            {
                var pixels = new CodingAdventures.PixelContainer.PixelContainer((int)(scene.Width * options.Scale), (int)(scene.Height * options.Scale));
                pixels.Fill(255, 0, 0, 255);
                return pixels;
            });

        var pixels = vm.Export(PaintScene(10, 5, "#fff", []), new ExportOptions { Scale = 2.0 });

        Assert.Equal(20, pixels.Width);
        Assert.Equal(10, pixels.Height);
        Assert.Equal(new CodingAdventures.PixelContainer.Rgba(255, 0, 0, 255), pixels.GetPixel(0, 0));
    }

    [Fact]
    public void DeepEqual_ComparesNestedObjectsByStructure()
    {
        var left = PaintGroup([PaintRect(0, 0, 10, 10)], new PaintGroupOptions
        {
            Id = "group",
            Metadata = new Dictionary<string, object?> { ["layer"] = "foreground" },
        });

        var right = PaintGroup([PaintRect(0, 0, 10, 10)], new PaintGroupOptions
        {
            Id = "group",
            Metadata = new Dictionary<string, object?> { ["layer"] = "foreground" },
        });

        Assert.True(PaintVM<object>.DeepEqual(left, right));
        Assert.False(PaintVM<object>.DeepEqual(left, PaintGroup([PaintRect(0, 0, 20, 10)])));
    }

    [Fact]
    public void DeepEqual_HandlesCyclicGraphsWithoutStackOverflow()
    {
        var left = new CyclicNode { Name = "root" };
        left.Next = left;

        var right = new CyclicNode { Name = "root" };
        right.Next = right;

        var different = new CyclicNode { Name = "other" };
        different.Next = different;

        Assert.True(PaintVM<object>.DeepEqual(left, right));
        Assert.False(PaintVM<object>.DeepEqual(left, different));
    }

    private static PaintVM<List<string>> CreateVm()
    {
        var vm = new PaintVM<List<string>>((context, background, _, _) => context.Add($"clear:{background}"));
        vm.Register("rect", (instruction, context, _) =>
        {
            if (instruction is PaintRect rect)
            {
                context.Add($"rect:{rect.X},{rect.Y}");
            }
        });
        vm.Register("ellipse", (instruction, context, _) =>
        {
            if (instruction is PaintEllipse ellipse)
            {
                context.Add($"ellipse:{ellipse.Cx},{ellipse.Cy}");
            }
        });
        vm.Register("group", (instruction, context, runtime) =>
        {
            if (instruction is PaintGroup group)
            {
                context.Add("group:start");
                foreach (var child in group.Children)
                {
                    runtime.Dispatch(child, context);
                }
                context.Add("group:end");
            }
        });
        return vm;
    }

    private sealed class CyclicNode
    {
        public string Name { get; init; } = string.Empty;

        public CyclicNode? Next { get; set; }
    }
}
