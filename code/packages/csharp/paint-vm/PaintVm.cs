using System.Collections;
using System.Reflection;
using System.Runtime.CompilerServices;
using CodingAdventures.PaintInstructions;
using PixelBuffer = CodingAdventures.PixelContainer.PixelContainer;

namespace CodingAdventures.PaintVm;

public static class PaintVmPackage
{
    public const string VERSION = "0.1.0";
}

public sealed class UnknownInstructionError(string kind) : Exception($"No handler registered for instruction kind: '{kind}'")
{
    public string Kind { get; } = kind;
}

public sealed class DuplicateHandlerError(string kind) : Exception($"Handler already registered for instruction kind: '{kind}'")
{
    public string Kind { get; } = kind;
}

public sealed class ExportNotSupportedError(string backendName)
    : Exception($"export() is not supported by the {backendName} backend. Use a backend that supports pixel readback.")
{
}

public sealed class NullContextError() : Exception("execute() and patch() require a non-null context")
{
}

public delegate void PaintHandler<TContext>(PaintInstructionBase instruction, TContext context, PaintVM<TContext> vm);

public sealed record ExportOptions
{
    public double Scale { get; init; } = 1.0;

    public int Channels { get; init; } = 4;

    public int BitDepth { get; init; } = 8;

    public string ColorSpace { get; init; } = "srgb";
}

public sealed record PatchCallbacks
{
    public Action<PaintInstructionBase>? OnDelete { get; init; }

    public Action<PaintInstructionBase, int>? OnInsert { get; init; }

    public Action<PaintInstructionBase, PaintInstructionBase>? OnUpdate { get; init; }
}

internal readonly struct ReferencePair(object left, object right)
{
    public object Left { get; } = left;

    public object Right { get; } = right;
}

internal sealed class ReferencePairComparer : IEqualityComparer<ReferencePair>
{
    public static ReferencePairComparer Instance { get; } = new();

    public bool Equals(ReferencePair x, ReferencePair y) =>
        ReferenceEquals(x.Left, y.Left) &&
        ReferenceEquals(x.Right, y.Right);

    public int GetHashCode(ReferencePair pair) =>
        HashCode.Combine(
            RuntimeHelpers.GetHashCode(pair.Left),
            RuntimeHelpers.GetHashCode(pair.Right));
}

/// <summary>
/// PaintVM is the dispatch-table execution engine for the paint IR. Backends
/// provide the clear function and the per-kind handlers; the VM supplies the
/// routing, diffing, and export contract.
/// </summary>
public sealed class PaintVM<TContext>
{
    private readonly Dictionary<string, PaintHandler<TContext>> _table = [];
    private readonly Action<TContext, string, double, double> _clear;
    private readonly Func<PaintScene, PaintVM<TContext>, ExportOptions, PixelBuffer>? _export;

    public PaintVM(
        Action<TContext, string, double, double> clear,
        Func<PaintScene, PaintVM<TContext>, ExportOptions, PixelBuffer>? export = null)
    {
        _clear = clear ?? throw new ArgumentNullException(nameof(clear));
        _export = export;
    }

    public void Register(string kind, PaintHandler<TContext> handler)
    {
        ArgumentException.ThrowIfNullOrEmpty(kind);
        ArgumentNullException.ThrowIfNull(handler);

        if (_table.ContainsKey(kind))
        {
            throw new DuplicateHandlerError(kind);
        }

        _table[kind] = handler;
    }

    public void Dispatch(PaintInstructionBase instruction, TContext context)
    {
        ArgumentNullException.ThrowIfNull(instruction);

        if (!_table.TryGetValue(instruction.Kind, out var handler) &&
            !_table.TryGetValue("*", out handler))
        {
            throw new UnknownInstructionError(instruction.Kind);
        }

        handler(instruction, context, this);
    }

    public void Execute(PaintScene scene, TContext context)
    {
        ArgumentNullException.ThrowIfNull(scene);
        EnsureContext(context);

        _clear(context, scene.Background, scene.Width, scene.Height);
        foreach (var instruction in scene.Instructions)
        {
            Dispatch(instruction, context);
        }
    }

    public void Patch(PaintScene oldScene, PaintScene newScene, TContext context, PatchCallbacks? callbacks = null)
    {
        ArgumentNullException.ThrowIfNull(oldScene);
        ArgumentNullException.ThrowIfNull(newScene);
        EnsureContext(context);

        if (callbacks is null)
        {
            Execute(newScene, context);
            return;
        }

        var oldById = oldScene.Instructions.Where(instruction => instruction.Id is not null)
            .ToDictionary(instruction => instruction.Id!, instruction => instruction);
        var newById = newScene.Instructions.Where(instruction => instruction.Id is not null)
            .ToDictionary(instruction => instruction.Id!, instruction => instruction);

        foreach (var (id, instruction) in oldById)
        {
            if (!newById.ContainsKey(id))
            {
                callbacks.OnDelete?.Invoke(instruction);
            }
        }

        for (var index = 0; index < newScene.Instructions.Count; index++)
        {
            var nextInstruction = newScene.Instructions[index];

            if (nextInstruction.Id is not null && oldById.TryGetValue(nextInstruction.Id, out var oldInstruction))
            {
                if (!DeepEqual(nextInstruction, oldInstruction))
                {
                    callbacks.OnUpdate?.Invoke(oldInstruction, nextInstruction);
                }

                continue;
            }

            if (index < oldScene.Instructions.Count)
            {
                var positionalOld = oldScene.Instructions[index];
                if (!DeepEqual(nextInstruction, positionalOld))
                {
                    callbacks.OnUpdate?.Invoke(positionalOld, nextInstruction);
                }
            }
            else
            {
                callbacks.OnInsert?.Invoke(nextInstruction, index);
            }
        }
    }

    public PixelBuffer Export(PaintScene scene, ExportOptions? options = null)
    {
        ArgumentNullException.ThrowIfNull(scene);

        if (_export is null)
        {
            throw new ExportNotSupportedError("this");
        }

        return _export(scene, this, options ?? new ExportOptions());
    }

    public IReadOnlyList<string> RegisteredKinds() => _table.Keys.Order().ToArray();

    public static bool DeepEqual(object? left, object? right) =>
        DeepEqual(left, right, new HashSet<ReferencePair>(ReferencePairComparer.Instance));

    private static bool DeepEqual(object? left, object? right, HashSet<ReferencePair> visited)
    {
        if (ReferenceEquals(left, right))
        {
            return true;
        }

        if (left is null || right is null)
        {
            return false;
        }

        if (left.GetType() != right.GetType())
        {
            return false;
        }

        var type = left.GetType();

        if (left is string || type.IsPrimitive || left is decimal || type.IsEnum)
        {
            return Equals(left, right);
        }

        if (!type.IsValueType && !visited.Add(new ReferencePair(left, right)))
        {
            return true;
        }

        if (left is IDictionary leftDictionary && right is IDictionary rightDictionary)
        {
            if (leftDictionary.Count != rightDictionary.Count)
            {
                return false;
            }

            foreach (var key in leftDictionary.Keys)
            {
                if (!rightDictionary.Contains(key) || !DeepEqual(leftDictionary[key], rightDictionary[key], visited))
                {
                    return false;
                }
            }

            return true;
        }

        if (left is IEnumerable leftEnumerable && right is IEnumerable rightEnumerable)
        {
            var leftItems = leftEnumerable.Cast<object?>().ToArray();
            var rightItems = rightEnumerable.Cast<object?>().ToArray();
            if (leftItems.Length != rightItems.Length)
            {
                return false;
            }

            for (var index = 0; index < leftItems.Length; index++)
            {
                if (!DeepEqual(leftItems[index], rightItems[index], visited))
                {
                    return false;
                }
            }

            return true;
        }

        foreach (var property in left.GetType().GetProperties(BindingFlags.Public | BindingFlags.Instance).Where(property => property.CanRead))
        {
            if (!DeepEqual(property.GetValue(left), property.GetValue(right), visited))
            {
                return false;
            }
        }

        return true;
    }

    private static void EnsureContext(TContext context)
    {
        if (context is null)
        {
            throw new NullContextError();
        }
    }
}
