namespace CodingAdventures.CompilerSourceMap;

/// <summary>
/// A span of characters in a source file.
/// </summary>
public sealed record SourcePosition(string File, int Line, int Column, int Length)
{
    /// <summary>Return a debugger-friendly source span label.</summary>
    public override string ToString() => $"{File}:{Line}:{Column} (len={Length})";
}

/// <summary>
/// One mapping from a source position to an AST node ID.
/// </summary>
public sealed record SourceToAstEntry(SourcePosition Position, int AstNodeId);

/// <summary>
/// Segment 1: source positions to AST node IDs.
/// </summary>
public sealed class SourceToAst
{
    private readonly List<SourceToAstEntry> _entries = [];

    /// <summary>All source-position to AST-node mappings.</summary>
    public IReadOnlyList<SourceToAstEntry> Entries => _entries.AsReadOnly();

    /// <summary>Add a source-position to AST-node mapping.</summary>
    public void Add(SourcePosition position, int astNodeId)
    {
        ArgumentNullException.ThrowIfNull(position);
        _entries.Add(new SourceToAstEntry(position, astNodeId));
    }

    /// <summary>Return the source position for an AST node ID, if known.</summary>
    public SourcePosition? LookupByNodeId(int astNodeId) =>
        _entries.FirstOrDefault(entry => entry.AstNodeId == astNodeId)?.Position;
}

/// <summary>
/// One mapping from an AST node to the IR instruction IDs it produced.
/// </summary>
public sealed record AstToIrEntry(int AstNodeId, IReadOnlyList<long> IrIds);

/// <summary>
/// Segment 2: AST node IDs to IR instruction IDs.
/// </summary>
public sealed class AstToIr
{
    private readonly List<AstToIrEntry> _entries = [];

    /// <summary>All AST-node to IR-ID mappings.</summary>
    public IReadOnlyList<AstToIrEntry> Entries => _entries.AsReadOnly();

    /// <summary>Add the IR instruction IDs produced by an AST node.</summary>
    public void Add(int astNodeId, IEnumerable<long> irIds)
    {
        ArgumentNullException.ThrowIfNull(irIds);
        _entries.Add(new AstToIrEntry(astNodeId, irIds.ToArray()));
    }

    /// <summary>Return the IR IDs produced by an AST node, if known.</summary>
    public IReadOnlyList<long>? LookupByAstNodeId(int astNodeId) =>
        _entries.FirstOrDefault(entry => entry.AstNodeId == astNodeId)?.IrIds;

    /// <summary>Return the AST node that produced an IR ID, or -1 if unknown.</summary>
    public int LookupByIrId(long irId)
    {
        foreach (var entry in _entries)
        {
            if (entry.IrIds.Contains(irId))
            {
                return entry.AstNodeId;
            }
        }

        return -1;
    }
}

/// <summary>
/// One optimizer mapping from an original IR ID to replacement IR IDs.
/// </summary>
public sealed record IrToIrEntry(long OriginalId, IReadOnlyList<long> NewIds);

/// <summary>
/// Segment 3: one optimizer pass from original IR IDs to optimized IR IDs.
/// </summary>
public sealed class IrToIr
{
    private readonly HashSet<long> _deleted = [];
    private readonly List<IrToIrEntry> _entries = [];

    /// <summary>Create an optimizer-pass segment.</summary>
    public IrToIr(string passName = "")
    {
        PassName = passName;
    }

    /// <summary>All original-IR to new-IR mappings.</summary>
    public IReadOnlyList<IrToIrEntry> Entries => _entries.AsReadOnly();

    /// <summary>Original IR IDs deleted by this pass.</summary>
    public IReadOnlySet<long> Deleted => _deleted;

    /// <summary>The optimizer pass name.</summary>
    public string PassName { get; }

    /// <summary>Add a mapping from one original IR ID to replacement IR IDs.</summary>
    public void AddMapping(long originalId, IEnumerable<long> newIds)
    {
        ArgumentNullException.ThrowIfNull(newIds);
        _entries.Add(new IrToIrEntry(originalId, newIds.ToArray()));
    }

    /// <summary>Record that an original IR ID was deleted.</summary>
    public void AddDeletion(long originalId)
    {
        _deleted.Add(originalId);
        _entries.Add(new IrToIrEntry(originalId, Array.Empty<long>()));
    }

    /// <summary>Return replacement IR IDs for an original ID, or null when deleted or unknown.</summary>
    public IReadOnlyList<long>? LookupByOriginalId(long originalId)
    {
        if (_deleted.Contains(originalId))
        {
            return null;
        }

        return _entries.FirstOrDefault(entry => entry.OriginalId == originalId)?.NewIds;
    }

    /// <summary>Return the original IR ID that produced a new ID, or -1 if unknown.</summary>
    public long LookupByNewId(long newId)
    {
        foreach (var entry in _entries)
        {
            if (entry.NewIds.Contains(newId))
            {
                return entry.OriginalId;
            }
        }

        return -1;
    }
}

/// <summary>
/// One mapping from an IR instruction to a machine-code byte range.
/// </summary>
public sealed record IrToMachineCodeEntry(long IrId, long MachineCodeOffset, long MachineCodeLength);

/// <summary>
/// Segment 4: IR instruction IDs to machine-code byte offsets.
/// </summary>
public sealed class IrToMachineCode
{
    private readonly List<IrToMachineCodeEntry> _entries = [];

    /// <summary>All IR to machine-code range mappings.</summary>
    public IReadOnlyList<IrToMachineCodeEntry> Entries => _entries.AsReadOnly();

    /// <summary>Add a machine-code byte range for an IR instruction.</summary>
    public void Add(long irId, long machineCodeOffset, long machineCodeLength)
    {
        _entries.Add(new IrToMachineCodeEntry(irId, machineCodeOffset, machineCodeLength));
    }

    /// <summary>Return the machine-code range for an IR ID, or (-1, 0) if unknown.</summary>
    public (long Offset, long Length) LookupByIrId(long irId)
    {
        var entry = _entries.FirstOrDefault(item => item.IrId == irId);
        return entry is null ? (-1, 0) : (entry.MachineCodeOffset, entry.MachineCodeLength);
    }

    /// <summary>Return the IR ID whose machine-code range contains the offset, or -1.</summary>
    public long LookupByMachineCodeOffset(long offset)
    {
        foreach (var entry in _entries)
        {
            if (entry.MachineCodeOffset <= offset && offset < entry.MachineCodeOffset + entry.MachineCodeLength)
            {
                return entry.IrId;
            }
        }

        return -1;
    }
}

/// <summary>
/// Full source-map sidecar composed from the compiler pipeline segments.
/// </summary>
public sealed class SourceMapChain
{
    /// <summary>Create an empty source-map chain.</summary>
    public SourceMapChain()
    {
        SourceToAst = new SourceToAst();
        AstToIr = new AstToIr();
    }

    /// <summary>Source-position to AST-node segment.</summary>
    public SourceToAst SourceToAst { get; }

    /// <summary>AST-node to IR-ID segment.</summary>
    public AstToIr AstToIr { get; }

    /// <summary>Optimizer pass segments in pipeline order.</summary>
    public List<IrToIr> IrToIr { get; } = [];

    /// <summary>IR to machine-code segment, filled by the backend.</summary>
    public IrToMachineCode? IrToMachineCode { get; set; }

    /// <summary>Create an empty source-map chain.</summary>
    public static SourceMapChain New() => new();

    /// <summary>Append an optimizer-pass segment.</summary>
    public void AddOptimizerPass(IrToIr segment)
    {
        ArgumentNullException.ThrowIfNull(segment);
        IrToIr.Add(segment);
    }

    /// <summary>Compose the chain from source position to machine-code ranges.</summary>
    public IReadOnlyList<IrToMachineCodeEntry>? SourceToMc(SourcePosition position)
    {
        ArgumentNullException.ThrowIfNull(position);
        if (IrToMachineCode is null)
        {
            return null;
        }

        var astNodeId = SourceToAst.Entries
            .FirstOrDefault(entry =>
                entry.Position.File == position.File
                && entry.Position.Line == position.Line
                && entry.Position.Column == position.Column)
            ?.AstNodeId;
        if (astNodeId is null)
        {
            return null;
        }

        var irIds = AstToIr.LookupByAstNodeId(astNodeId.Value);
        if (irIds is null)
        {
            return null;
        }

        var currentIds = irIds.ToList();
        foreach (var pass in IrToIr)
        {
            var nextIds = new List<long>();
            foreach (var irId in currentIds)
            {
                if (pass.Deleted.Contains(irId))
                {
                    continue;
                }

                var newIds = pass.LookupByOriginalId(irId);
                if (newIds is not null)
                {
                    nextIds.AddRange(newIds);
                }
            }

            currentIds = nextIds;
        }

        if (currentIds.Count == 0)
        {
            return null;
        }

        var results = new List<IrToMachineCodeEntry>();
        foreach (var irId in currentIds)
        {
            var (offset, length) = IrToMachineCode.LookupByIrId(irId);
            if (offset >= 0)
            {
                results.Add(new IrToMachineCodeEntry(irId, offset, length));
            }
        }

        return results.Count == 0 ? null : results;
    }

    /// <summary>Compose the chain from machine-code offset back to source position.</summary>
    public SourcePosition? McToSource(long machineCodeOffset)
    {
        if (IrToMachineCode is null)
        {
            return null;
        }

        var currentId = IrToMachineCode.LookupByMachineCodeOffset(machineCodeOffset);
        if (currentId == -1)
        {
            return null;
        }

        for (var i = IrToIr.Count - 1; i >= 0; i--)
        {
            var originalId = IrToIr[i].LookupByNewId(currentId);
            if (originalId == -1)
            {
                return null;
            }

            currentId = originalId;
        }

        var astNodeId = AstToIr.LookupByIrId(currentId);
        return astNodeId == -1 ? null : SourceToAst.LookupByNodeId(astNodeId);
    }
}

/// <summary>
/// Package metadata.
/// </summary>
public static class CompilerSourceMapPackage
{
    /// <summary>The package version.</summary>
    public const string Version = "0.1.0";
}
