using System.Collections.ObjectModel;
using System.Text.Json;
using CodingAdventures.BlockRam;
using Gates = CodingAdventures.LogicGates.LogicGates;

namespace CodingAdventures.Fpga;

internal static class Validation
{
    internal static void Bit(int value, string name)
    {
        if (value is not (0 or 1))
        {
            throw new ArgumentOutOfRangeException(name, value, $"{name} must be 0 or 1");
        }
    }

    internal static int[] Bits(IReadOnlyList<int> values, int length, string name)
    {
        ArgumentNullException.ThrowIfNull(values);
        if (values.Count != length)
        {
            throw new ArgumentException($"{name} length {values.Count} does not match {length}", name);
        }

        var result = new int[length];
        for (var index = 0; index < length; index++)
        {
            Bit(values[index], $"{name}[{index}]");
            result[index] = values[index];
        }

        return result;
    }
}

/// <summary>A K-input lookup table backed by SRAM cells.</summary>
public sealed class LUT
{
    private readonly SRAMCell[] _cells;

    public LUT(int k = 4, IReadOnlyList<int>? truthTable = null)
    {
        if (k is < 2 or > 6)
        {
            throw new ArgumentOutOfRangeException(nameof(k), k, "k must be between 2 and 6");
        }

        K = k;
        _cells = Enumerable.Range(0, 1 << k).Select(_ => new SRAMCell()).ToArray();
        if (truthTable is not null)
        {
            Configure(truthTable);
        }
    }

    public int K { get; }

    public IReadOnlyList<int> TruthTable => Array.AsReadOnly(_cells.Select(cell => cell.Read(1)!.Value).ToArray());

    public void Configure(IReadOnlyList<int> truthTable)
    {
        var bits = Validation.Bits(truthTable, _cells.Length, nameof(truthTable));
        for (var index = 0; index < bits.Length; index++)
        {
            _cells[index].Write(1, bits[index]);
        }
    }

    public int Evaluate(IReadOnlyList<int> inputs)
    {
        var bits = Validation.Bits(inputs, K, nameof(inputs));
        var tableIndex = 0;
        for (var index = 0; index < bits.Length; index++)
        {
            tableIndex |= bits[index] << index;
        }

        return _cells[tableIndex].Read(1)!.Value;
    }
}

public readonly record struct SliceOutput(int OutputA, int OutputB, int CarryOut);

internal sealed class FlipFlop
{
    private int _master;
    private int _output;

    internal int Evaluate(int data, int clock)
    {
        Validation.Bit(data, nameof(data));
        Validation.Bit(clock, nameof(clock));
        if (clock == 1)
        {
            _master = data;
        }
        else
        {
            _output = _master;
        }

        return _output;
    }
}

/// <summary>Two LUTs, optional registers, and a carry chain.</summary>
public sealed class Slice
{
    private FlipFlop _flipFlopA = new();
    private FlipFlop _flipFlopB = new();
    private bool _flipFlopAEnabled;
    private bool _flipFlopBEnabled;
    private bool _carryEnabled;

    public Slice(int lutInputs = 4)
    {
        LutA = new LUT(lutInputs);
        LutB = new LUT(lutInputs);
        K = lutInputs;
    }

    public LUT LutA { get; }
    public LUT LutB { get; }
    public int K { get; }

    public void Configure(
        IReadOnlyList<int> lutATable,
        IReadOnlyList<int> lutBTable,
        bool flipFlopAEnabled = false,
        bool flipFlopBEnabled = false,
        bool carryEnabled = false)
    {
        LutA.Configure(lutATable);
        LutB.Configure(lutBTable);
        _flipFlopAEnabled = flipFlopAEnabled;
        _flipFlopBEnabled = flipFlopBEnabled;
        _carryEnabled = carryEnabled;
        _flipFlopA = new FlipFlop();
        _flipFlopB = new FlipFlop();
    }

    public SliceOutput Evaluate(
        IReadOnlyList<int> inputsA,
        IReadOnlyList<int> inputsB,
        int clock,
        int carryIn = 0)
    {
        Validation.Bit(clock, nameof(clock));
        Validation.Bit(carryIn, nameof(carryIn));
        var lutA = LutA.Evaluate(inputsA);
        var lutB = LutB.Evaluate(inputsB);
        var outputA = _flipFlopAEnabled ? _flipFlopA.Evaluate(lutA, clock) : lutA;
        var outputB = _flipFlopBEnabled ? _flipFlopB.Evaluate(lutB, clock) : lutB;
        var carry = _carryEnabled
            ? Gates.Or(Gates.And(lutA, lutB), Gates.And(carryIn, Gates.Xor(lutA, lutB)))
            : 0;
        return new SliceOutput(outputA, outputB, carry);
    }
}

public readonly record struct CLBOutput(SliceOutput Slice0, SliceOutput Slice1);

/// <summary>A configurable logic block containing two slices.</summary>
public sealed class CLB
{
    public CLB(int lutInputs = 4)
    {
        Slice0 = new Slice(lutInputs);
        Slice1 = new Slice(lutInputs);
        K = lutInputs;
    }

    public Slice Slice0 { get; }
    public Slice Slice1 { get; }
    public int K { get; }

    public CLBOutput Evaluate(
        IReadOnlyList<int> slice0InputsA,
        IReadOnlyList<int> slice0InputsB,
        IReadOnlyList<int> slice1InputsA,
        IReadOnlyList<int> slice1InputsB,
        int clock,
        int carryIn = 0)
    {
        var first = Slice0.Evaluate(slice0InputsA, slice0InputsB, clock, carryIn);
        var second = Slice1.Evaluate(slice1InputsA, slice1InputsB, clock, first.CarryOut);
        return new CLBOutput(first, second);
    }
}

/// <summary>A programmable crossbar with one driver per destination.</summary>
public sealed class SwitchMatrix
{
    private readonly HashSet<string> _ports;
    private readonly Dictionary<string, string> _connections = new(StringComparer.Ordinal);

    public SwitchMatrix(IEnumerable<string> ports)
    {
        ArgumentNullException.ThrowIfNull(ports);
        _ports = new HashSet<string>(ports, StringComparer.Ordinal);
        if (_ports.Count == 0 || _ports.Any(string.IsNullOrWhiteSpace))
        {
            throw new ArgumentException("ports must contain non-empty unique names", nameof(ports));
        }
    }

    public IReadOnlySet<string> Ports => _ports;
    public IReadOnlyDictionary<string, string> Connections =>
        new ReadOnlyDictionary<string, string>(new Dictionary<string, string>(_connections, StringComparer.Ordinal));
    public int ConnectionCount => _connections.Count;

    public void Connect(string source, string destination)
    {
        if (!_ports.Contains(source))
        {
            throw new ArgumentException($"unknown source port: {source}", nameof(source));
        }

        if (!_ports.Contains(destination))
        {
            throw new ArgumentException($"unknown destination port: {destination}", nameof(destination));
        }

        if (source == destination)
        {
            throw new ArgumentException("a port cannot connect to itself", nameof(destination));
        }

        if (!_connections.TryAdd(destination, source))
        {
            throw new InvalidOperationException($"destination {destination} is already connected");
        }
    }

    public void Disconnect(string destination)
    {
        if (!_ports.Contains(destination))
        {
            throw new ArgumentException($"unknown port: {destination}", nameof(destination));
        }

        if (!_connections.Remove(destination))
        {
            throw new InvalidOperationException($"port {destination} is not connected");
        }
    }

    public void Clear() => _connections.Clear();

    public IReadOnlyDictionary<string, int> Route(IReadOnlyDictionary<string, int> inputs)
    {
        ArgumentNullException.ThrowIfNull(inputs);
        var outputs = new Dictionary<string, int>(StringComparer.Ordinal);
        foreach (var (destination, source) in _connections)
        {
            if (inputs.TryGetValue(source, out var value))
            {
                Validation.Bit(value, $"inputs[{source}]");
                outputs[destination] = value;
            }
        }

        return new ReadOnlyDictionary<string, int>(outputs);
    }
}

public enum IOMode
{
    Input,
    Output,
    Tristate,
}

/// <summary>A configurable external I/O pad.</summary>
public sealed class IOBlock
{
    private int _padValue;
    private int _internalValue;

    public IOBlock(string name, IOMode mode = IOMode.Input)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            throw new ArgumentException("name must be non-empty", nameof(name));
        }

        Name = name;
        Mode = mode;
    }

    public string Name { get; }
    public IOMode Mode { get; private set; }

    public void Configure(IOMode mode) => Mode = mode;

    public void DrivePad(int value)
    {
        Validation.Bit(value, nameof(value));
        _padValue = value;
    }

    public void DriveInternal(int value)
    {
        Validation.Bit(value, nameof(value));
        _internalValue = value;
    }

    public int ReadInternal() => Mode == IOMode.Input ? _padValue : _internalValue;

    public int? ReadPad() => Mode switch
    {
        IOMode.Input => _padValue,
        IOMode.Output => _internalValue,
        _ => null,
    };
}

public sealed class SliceConfig
{
    public SliceConfig(
        IReadOnlyList<int>? lutA = null,
        IReadOnlyList<int>? lutB = null,
        bool flipFlopAEnabled = false,
        bool flipFlopBEnabled = false,
        bool carryEnabled = false)
    {
        LutA = (lutA ?? Array.Empty<int>()).ToArray();
        LutB = (lutB ?? Array.Empty<int>()).ToArray();
        FlipFlopAEnabled = flipFlopAEnabled;
        FlipFlopBEnabled = flipFlopBEnabled;
        CarryEnabled = carryEnabled;
    }

    public IReadOnlyList<int> LutA { get; }
    public IReadOnlyList<int> LutB { get; }
    public bool FlipFlopAEnabled { get; }
    public bool FlipFlopBEnabled { get; }
    public bool CarryEnabled { get; }
}

public sealed record CLBConfig(SliceConfig Slice0, SliceConfig Slice1);
public sealed record RouteConfig(string Source, string Destination);
public sealed record IOConfig(string Mode);

/// <summary>Immutable FPGA configuration data.</summary>
public sealed class Bitstream
{
    public Bitstream(
        IReadOnlyDictionary<string, CLBConfig>? clbs = null,
        IReadOnlyDictionary<string, IReadOnlyList<RouteConfig>>? routing = null,
        IReadOnlyDictionary<string, IOConfig>? io = null,
        int lutK = 4)
    {
        if (lutK is < 2 or > 6)
        {
            throw new ArgumentOutOfRangeException(nameof(lutK), lutK, "lutK must be between 2 and 6");
        }

        LutK = lutK;
        Clbs = new ReadOnlyDictionary<string, CLBConfig>(
            (clbs ?? new Dictionary<string, CLBConfig>())
                .ToDictionary(pair => pair.Key, pair => pair.Value, StringComparer.Ordinal));
        Routing = new ReadOnlyDictionary<string, IReadOnlyList<RouteConfig>>(
            (routing ?? new Dictionary<string, IReadOnlyList<RouteConfig>>())
                .ToDictionary(pair => pair.Key, pair => (IReadOnlyList<RouteConfig>)pair.Value.ToArray(), StringComparer.Ordinal));
        IO = new ReadOnlyDictionary<string, IOConfig>(
            (io ?? new Dictionary<string, IOConfig>())
                .ToDictionary(pair => pair.Key, pair => pair.Value, StringComparer.Ordinal));
    }

    public IReadOnlyDictionary<string, CLBConfig> Clbs { get; }
    public IReadOnlyDictionary<string, IReadOnlyList<RouteConfig>> Routing { get; }
    public IReadOnlyDictionary<string, IOConfig> IO { get; }
    public int LutK { get; }

    public static Bitstream Empty(int lutK = 4) => new(lutK: lutK);

    public static Bitstream ParseJson(string json)
    {
        ArgumentNullException.ThrowIfNull(json);
        using var document = JsonDocument.Parse(json);
        var root = document.RootElement;
        if (root.ValueKind != JsonValueKind.Object)
        {
            throw new JsonException("bitstream JSON root must be an object");
        }

        var lutK = root.TryGetProperty("lut_k", out var lutElement) ? lutElement.GetInt32() : 4;
        if (lutK is < 2 or > 6)
        {
            throw new JsonException("lut_k must be between 2 and 6");
        }

        var tableLength = 1 << lutK;
        var clbs = new Dictionary<string, CLBConfig>(StringComparer.Ordinal);
        if (root.TryGetProperty("clbs", out var clbElement))
        {
            foreach (var property in clbElement.EnumerateObject())
            {
                clbs[property.Name] = new CLBConfig(
                    ParseSlice(property.Value, "slice0", tableLength),
                    ParseSlice(property.Value, "slice1", tableLength));
            }
        }

        var routing = new Dictionary<string, IReadOnlyList<RouteConfig>>(StringComparer.Ordinal);
        if (root.TryGetProperty("routing", out var routingElement))
        {
            foreach (var property in routingElement.EnumerateObject())
            {
                routing[property.Name] = property.Value.EnumerateArray()
                    .Select(route => new RouteConfig(route.GetProperty("src").GetString()!, route.GetProperty("dst").GetString()!))
                    .ToArray();
            }
        }

        var io = new Dictionary<string, IOConfig>(StringComparer.Ordinal);
        if (root.TryGetProperty("io", out var ioElement))
        {
            foreach (var property in ioElement.EnumerateObject())
            {
                var mode = property.Value.TryGetProperty("mode", out var modeElement) ? modeElement.GetString() ?? "input" : "input";
                io[property.Name] = new IOConfig(mode);
            }
        }

        return new Bitstream(clbs, routing, io, lutK);
    }

    private static SliceConfig ParseSlice(JsonElement clb, string name, int tableLength)
    {
        if (!clb.TryGetProperty(name, out var slice))
        {
            return new SliceConfig(new int[tableLength], new int[tableLength]);
        }

        return new SliceConfig(
            ParseTable(slice, "lut_a", tableLength),
            ParseTable(slice, "lut_b", tableLength),
            slice.TryGetProperty("ff_a", out var ffA) && ffA.GetBoolean(),
            slice.TryGetProperty("ff_b", out var ffB) && ffB.GetBoolean(),
            slice.TryGetProperty("carry", out var carry) && carry.GetBoolean());
    }

    private static int[] ParseTable(JsonElement slice, string name, int tableLength) =>
        slice.TryGetProperty(name, out var table)
            ? table.EnumerateArray().Select(value => value.GetInt32()).ToArray()
            : new int[tableLength];
}

/// <summary>A configured FPGA fabric with CLBs, routing, and I/O blocks.</summary>
public sealed class FPGA
{
    private readonly Dictionary<string, CLB> _clbs = new(StringComparer.Ordinal);
    private readonly Dictionary<string, SwitchMatrix> _switches = new(StringComparer.Ordinal);
    private readonly Dictionary<string, IOBlock> _ioBlocks = new(StringComparer.Ordinal);

    public FPGA(Bitstream bitstream)
    {
        ArgumentNullException.ThrowIfNull(bitstream);
        Bitstream = bitstream;
        Configure(bitstream);
    }

    public Bitstream Bitstream { get; }
    public IReadOnlyDictionary<string, CLB> Clbs => new ReadOnlyDictionary<string, CLB>(_clbs);
    public IReadOnlyDictionary<string, SwitchMatrix> Switches => new ReadOnlyDictionary<string, SwitchMatrix>(_switches);
    public IReadOnlyDictionary<string, IOBlock> IOBlocks => new ReadOnlyDictionary<string, IOBlock>(_ioBlocks);

    public CLBOutput EvaluateCLB(
        string name,
        IReadOnlyList<int> slice0InputsA,
        IReadOnlyList<int> slice0InputsB,
        IReadOnlyList<int> slice1InputsA,
        IReadOnlyList<int> slice1InputsB,
        int clock,
        int carryIn = 0)
    {
        if (!_clbs.TryGetValue(name, out var clb))
        {
            throw new KeyNotFoundException($"CLB {name} was not found");
        }

        return clb.Evaluate(slice0InputsA, slice0InputsB, slice1InputsA, slice1InputsB, clock, carryIn);
    }

    public IReadOnlyDictionary<string, int> Route(string name, IReadOnlyDictionary<string, int> signals)
    {
        if (!_switches.TryGetValue(name, out var matrix))
        {
            throw new KeyNotFoundException($"switch matrix {name} was not found");
        }

        return matrix.Route(signals);
    }

    public void SetInput(string name, int value) => GetIO(name).DrivePad(value);
    public void DriveOutput(string name, int value) => GetIO(name).DriveInternal(value);
    public int? ReadOutput(string name) => GetIO(name).ReadPad();

    private IOBlock GetIO(string name) =>
        _ioBlocks.TryGetValue(name, out var io) ? io : throw new KeyNotFoundException($"I/O pin {name} was not found");

    private void Configure(Bitstream bitstream)
    {
        foreach (var (name, config) in bitstream.Clbs)
        {
            var clb = new CLB(bitstream.LutK);
            ConfigureSlice(clb.Slice0, config.Slice0);
            ConfigureSlice(clb.Slice1, config.Slice1);
            _clbs[name] = clb;
        }

        foreach (var (name, routes) in bitstream.Routing)
        {
            var ports = routes.SelectMany(route => new[] { route.Source, route.Destination }).ToHashSet(StringComparer.Ordinal);
            if (ports.Count == 0)
            {
                continue;
            }

            var matrix = new SwitchMatrix(ports);
            foreach (var route in routes)
            {
                matrix.Connect(route.Source, route.Destination);
            }

            _switches[name] = matrix;
        }

        foreach (var (name, config) in bitstream.IO)
        {
            _ioBlocks[name] = new IOBlock(name, ParseMode(config.Mode));
        }
    }

    private static void ConfigureSlice(Slice slice, SliceConfig config) =>
        slice.Configure(config.LutA, config.LutB, config.FlipFlopAEnabled, config.FlipFlopBEnabled, config.CarryEnabled);

    private static IOMode ParseMode(string mode) => mode.ToLowerInvariant() switch
    {
        "output" => IOMode.Output,
        "tristate" => IOMode.Tristate,
        _ => IOMode.Input,
    };
}
