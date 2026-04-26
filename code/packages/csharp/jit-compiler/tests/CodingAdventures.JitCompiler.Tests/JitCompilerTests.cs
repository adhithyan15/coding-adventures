namespace CodingAdventures.JitCompiler.Tests;

public sealed class JitCompilerTests
{
    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", JitCompiler.Version);
    }

    [Fact]
    public void ConfigValidatesHotThreshold()
    {
        var config = new JitCompilerConfig(TargetIsa.RiscV, 3);

        Assert.Equal(TargetIsa.RiscV, config.Target);
        Assert.Equal(3UL, config.HotThreshold);
        Assert.Throws<ArgumentOutOfRangeException>(() => new JitCompilerConfig(TargetIsa.Arm, 0));
    }

    [Fact]
    public void PathBecomesHotExactlyAtThreshold()
    {
        var jit = new JitCompiler(new JitCompilerConfig(TargetIsa.RiscV, 3));

        Assert.False(jit.ObserveExecution(24));
        Assert.False(jit.ObserveExecution(24));
        Assert.True(jit.ObserveExecution(24));
        Assert.False(jit.ObserveExecution(24));
    }

    [Fact]
    public void ProfileReportsExecutionCountAndHotness()
    {
        var jit = new JitCompiler(new JitCompilerConfig(TargetIsa.Arm, 2));

        Assert.Null(jit.Profile(8));
        jit.ObserveExecution(8);
        var profile = jit.Profile(8);
        Assert.NotNull(profile);
        Assert.Equal(1UL, profile.ExecutionCount);
        Assert.False(profile.IsHot);

        jit.ObserveExecution(8);
        var hotProfile = jit.Profile(8);
        Assert.NotNull(hotProfile);
        Assert.Equal(2UL, hotProfile.ExecutionCount);
        Assert.True(hotProfile.IsHot);
    }

    [Fact]
    public void ShellBlockInstallationUsesConfiguredTarget()
    {
        var jit = new JitCompiler(new JitCompilerConfig(TargetIsa.X86, 5));

        var block = jit.InstallShellBlock(32, ["locals stay integers"]);

        Assert.Equal(32, block.BytecodeOffset);
        Assert.Equal(TargetIsa.X86, block.Target);
        Assert.Empty(block.MachineCode);
        Assert.Equal(["locals stay integers"], block.Assumptions);
        Assert.True(jit.HasNativeBlock(32));
        Assert.Same(block, jit.GetNativeBlock(32));
    }

    [Fact]
    public void DeoptimizeRemovesNativeBlock()
    {
        var jit = new JitCompiler(new JitCompilerConfig(TargetIsa.RiscV, 10));
        jit.InstallShellBlock(99, ["shape stays stable"]);

        var block = jit.Deoptimize(99);

        Assert.NotNull(block);
        Assert.Equal(99, block.BytecodeOffset);
        Assert.False(jit.HasNativeBlock(99));
        Assert.Null(jit.GetNativeBlock(99));
        Assert.Null(jit.Deoptimize(99));
    }

    [Fact]
    public void InvalidArgumentsAreRejected()
    {
        Assert.Throws<ArgumentNullException>(() => new JitCompiler(null!));
        var jit = new JitCompiler(new JitCompilerConfig(TargetIsa.Arm, 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => jit.ObserveExecution(-1));
        Assert.Throws<ArgumentOutOfRangeException>(() => jit.Profile(-1));
        Assert.Throws<ArgumentOutOfRangeException>(() => jit.HasNativeBlock(-1));
        Assert.Throws<ArgumentOutOfRangeException>(() => jit.GetNativeBlock(-1));
        Assert.Throws<ArgumentOutOfRangeException>(() => jit.Deoptimize(-1));
        Assert.Throws<ArgumentNullException>(() => jit.InstallShellBlock(1, null!));
    }
}
