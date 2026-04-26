namespace CodingAdventures.JitCompiler;

public enum TargetIsa
{
    RiscV,
    Arm,
    X86,
}

public sealed record JitCompilerConfig
{
    public JitCompilerConfig(TargetIsa target, ulong hotThreshold)
    {
        if (hotThreshold == 0)
        {
            throw new ArgumentOutOfRangeException(nameof(hotThreshold), "hotThreshold must be greater than zero.");
        }

        Target = target;
        HotThreshold = hotThreshold;
    }

    public TargetIsa Target { get; }

    public ulong HotThreshold { get; }
}

public sealed record HotPathProfile(int BytecodeOffset, ulong ExecutionCount, bool IsHot);

public sealed record NativeBlock(
    int BytecodeOffset,
    TargetIsa Target,
    IReadOnlyList<byte> MachineCode,
    IReadOnlyList<string> Assumptions);

public sealed class JitCompiler
{
    public const string Version = "0.1.0";

    private readonly SortedDictionary<int, ulong> _executionCounts = [];
    private readonly SortedDictionary<int, NativeBlock> _nativeBlocks = [];

    public JitCompiler(JitCompilerConfig config)
    {
        ArgumentNullException.ThrowIfNull(config);
        Config = config;
    }

    public JitCompilerConfig Config { get; }

    public bool ObserveExecution(int bytecodeOffset)
    {
        ValidateOffset(bytecodeOffset);
        _executionCounts.TryGetValue(bytecodeOffset, out var count);
        count++;
        _executionCounts[bytecodeOffset] = count;
        return count == Config.HotThreshold;
    }

    public HotPathProfile? Profile(int bytecodeOffset)
    {
        ValidateOffset(bytecodeOffset);
        return _executionCounts.TryGetValue(bytecodeOffset, out var executionCount)
            ? new HotPathProfile(bytecodeOffset, executionCount, executionCount >= Config.HotThreshold)
            : null;
    }

    public NativeBlock InstallShellBlock(int bytecodeOffset, IEnumerable<string> assumptions)
    {
        ValidateOffset(bytecodeOffset);
        ArgumentNullException.ThrowIfNull(assumptions);
        var block = new NativeBlock(
            bytecodeOffset,
            Config.Target,
            Array.Empty<byte>(),
            assumptions.ToArray());

        _nativeBlocks[bytecodeOffset] = block;
        return block;
    }

    public bool HasNativeBlock(int bytecodeOffset)
    {
        ValidateOffset(bytecodeOffset);
        return _nativeBlocks.ContainsKey(bytecodeOffset);
    }

    public NativeBlock? GetNativeBlock(int bytecodeOffset)
    {
        ValidateOffset(bytecodeOffset);
        return _nativeBlocks.TryGetValue(bytecodeOffset, out var block) ? block : null;
    }

    public NativeBlock? Deoptimize(int bytecodeOffset)
    {
        ValidateOffset(bytecodeOffset);
        if (!_nativeBlocks.Remove(bytecodeOffset, out var block))
        {
            return null;
        }

        return block;
    }

    private static void ValidateOffset(int bytecodeOffset)
    {
        if (bytecodeOffset < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(bytecodeOffset), "bytecodeOffset must be zero or greater.");
        }
    }
}
