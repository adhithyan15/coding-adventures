namespace CodingAdventures.BlockRam;

internal static class BitValidation
{
    internal static void Validate(int value, string name)
    {
        if (value is not (0 or 1))
        {
            throw new ArgumentOutOfRangeException(name, value, $"{name} must be 0 or 1");
        }
    }

    internal static int[] CopyAndValidate(IReadOnlyList<int> data, int width, string name)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (data.Count != width)
        {
            throw new ArgumentException($"{name} length {data.Count} does not match width {width}", name);
        }

        var result = new int[width];
        for (var index = 0; index < width; index++)
        {
            Validate(data[index], $"{name}[{index}]");
            result[index] = data[index];
        }

        return result;
    }
}

/// <summary>A single-bit static RAM cell.</summary>
public sealed class SRAMCell
{
    /// <summary>The bit currently stored in the cell.</summary>
    public int Value { get; private set; }

    /// <summary>Reads the cell when its word line is asserted.</summary>
    public int? Read(int wordLine)
    {
        BitValidation.Validate(wordLine, nameof(wordLine));
        return wordLine == 1 ? Value : null;
    }

    /// <summary>Writes the cell when its word line is asserted.</summary>
    public void Write(int wordLine, int bitLine)
    {
        BitValidation.Validate(wordLine, nameof(wordLine));
        BitValidation.Validate(bitLine, nameof(bitLine));
        if (wordLine == 1)
        {
            Value = bitLine;
        }
    }
}

/// <summary>A rectangular zero-initialized array of SRAM cells.</summary>
public sealed class SRAMArray
{
    private readonly SRAMCell[][] _cells;

    public SRAMArray(int rows, int cols)
    {
        if (rows < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(rows), rows, "rows must be >= 1");
        }

        if (cols < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(cols), cols, "cols must be >= 1");
        }

        Rows = rows;
        Cols = cols;
        _cells = Enumerable.Range(0, rows)
            .Select(_ => Enumerable.Range(0, cols).Select(_ => new SRAMCell()).ToArray())
            .ToArray();
    }

    public int Rows { get; }
    public int Cols { get; }
    public (int Rows, int Cols) Shape => (Rows, Cols);

    public int[] Read(int row)
    {
        ValidateRow(row);
        return _cells[row].Select(cell => cell.Read(1)!.Value).ToArray();
    }

    public void Write(int row, IReadOnlyList<int> data)
    {
        ValidateRow(row);
        var bits = BitValidation.CopyAndValidate(data, Cols, nameof(data));
        for (var column = 0; column < Cols; column++)
        {
            _cells[row][column].Write(1, bits[column]);
        }
    }

    private void ValidateRow(int row)
    {
        if (row < 0 || row >= Rows)
        {
            throw new ArgumentOutOfRangeException(nameof(row), row, $"row {row} out of range [0, {Rows - 1}]");
        }
    }
}

/// <summary>Controls the value exposed by a RAM port during writes.</summary>
public enum ReadMode
{
    ReadFirst,
    WriteFirst,
    NoChange,
}

/// <summary>Raised when both ports write the same address on one rising edge.</summary>
public sealed class WriteCollisionException : InvalidOperationException
{
    public WriteCollisionException(int address)
        : base($"Write collision: both ports writing to address {address}")
    {
        Address = address;
    }

    public int Address { get; }
}

/// <summary>A synchronous single-port RAM.</summary>
public sealed class SinglePortRAM
{
    private readonly SRAMArray _array;
    private readonly ReadMode _readMode;
    private int _previousClock;
    private int[] _lastRead;

    public SinglePortRAM(int depth, int width, ReadMode readMode = ReadMode.ReadFirst)
    {
        if (depth < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(depth), depth, "depth must be >= 1");
        }

        if (width < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(width), width, "width must be >= 1");
        }

        Depth = depth;
        Width = width;
        _readMode = readMode;
        _array = new SRAMArray(depth, width);
        _lastRead = new int[width];
    }

    public int Depth { get; }
    public int Width { get; }

    public int[] Tick(int clock, int address, IReadOnlyList<int> dataIn, int writeEnable)
    {
        BitValidation.Validate(clock, nameof(clock));
        BitValidation.Validate(writeEnable, nameof(writeEnable));
        ValidateAddress(address, nameof(address));
        var data = BitValidation.CopyAndValidate(dataIn, Width, nameof(dataIn));

        var risingEdge = _previousClock == 0 && clock == 1;
        _previousClock = clock;
        if (!risingEdge)
        {
            return [.. _lastRead];
        }

        if (writeEnable == 0)
        {
            _lastRead = _array.Read(address);
            return [.. _lastRead];
        }

        switch (_readMode)
        {
            case ReadMode.ReadFirst:
                _lastRead = _array.Read(address);
                _array.Write(address, data);
                break;
            case ReadMode.WriteFirst:
                _array.Write(address, data);
                _lastRead = data;
                break;
            case ReadMode.NoChange:
                _array.Write(address, data);
                break;
            default:
                throw new InvalidOperationException($"Unsupported read mode: {_readMode}");
        }

        return [.. _lastRead];
    }

    public int[][] Dump() => Enumerable.Range(0, Depth).Select(_array.Read).ToArray();

    private void ValidateAddress(int address, string name)
    {
        if (address < 0 || address >= Depth)
        {
            throw new ArgumentOutOfRangeException(name, address, $"address {address} out of range [0, {Depth - 1}]");
        }
    }
}

/// <summary>A synchronous true dual-port RAM with collision detection.</summary>
public sealed class DualPortRAM
{
    private readonly SRAMArray _array;
    private readonly ReadMode _readModeA;
    private readonly ReadMode _readModeB;
    private int _previousClock;
    private int[] _lastReadA;
    private int[] _lastReadB;

    public DualPortRAM(
        int depth,
        int width,
        ReadMode readModeA = ReadMode.ReadFirst,
        ReadMode readModeB = ReadMode.ReadFirst)
    {
        if (depth < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(depth), depth, "depth must be >= 1");
        }

        if (width < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(width), width, "width must be >= 1");
        }

        Depth = depth;
        Width = width;
        _readModeA = readModeA;
        _readModeB = readModeB;
        _array = new SRAMArray(depth, width);
        _lastReadA = new int[width];
        _lastReadB = new int[width];
    }

    public int Depth { get; }
    public int Width { get; }

    public (int[] DataOutA, int[] DataOutB) Tick(
        int clock,
        int addressA,
        IReadOnlyList<int> dataInA,
        int writeEnableA,
        int addressB,
        IReadOnlyList<int> dataInB,
        int writeEnableB)
    {
        BitValidation.Validate(clock, nameof(clock));
        BitValidation.Validate(writeEnableA, nameof(writeEnableA));
        BitValidation.Validate(writeEnableB, nameof(writeEnableB));
        ValidateAddress(addressA, nameof(addressA));
        ValidateAddress(addressB, nameof(addressB));
        var dataA = BitValidation.CopyAndValidate(dataInA, Width, nameof(dataInA));
        var dataB = BitValidation.CopyAndValidate(dataInB, Width, nameof(dataInB));

        var risingEdge = _previousClock == 0 && clock == 1;
        _previousClock = clock;
        if (!risingEdge)
        {
            return ([.. _lastReadA], [.. _lastReadB]);
        }

        if (writeEnableA == 1 && writeEnableB == 1 && addressA == addressB)
        {
            throw new WriteCollisionException(addressA);
        }

        _lastReadA = ProcessPort(addressA, dataA, writeEnableA, _readModeA, _lastReadA);
        _lastReadB = ProcessPort(addressB, dataB, writeEnableB, _readModeB, _lastReadB);
        return ([.. _lastReadA], [.. _lastReadB]);
    }

    private int[] ProcessPort(int address, int[] data, int writeEnable, ReadMode mode, int[] lastRead)
    {
        if (writeEnable == 0)
        {
            return _array.Read(address);
        }

        switch (mode)
        {
            case ReadMode.ReadFirst:
                var oldData = _array.Read(address);
                _array.Write(address, data);
                return oldData;
            case ReadMode.WriteFirst:
                _array.Write(address, data);
                return data;
            case ReadMode.NoChange:
                _array.Write(address, data);
                return [.. lastRead];
            default:
                throw new InvalidOperationException($"Unsupported read mode: {mode}");
        }
    }

    private void ValidateAddress(int address, string name)
    {
        if (address < 0 || address >= Depth)
        {
            throw new ArgumentOutOfRangeException(name, address, $"address {address} out of range [0, {Depth - 1}]");
        }
    }
}

/// <summary>An FPGA-style dual-port block RAM with configurable aspect ratio.</summary>
public sealed class ConfigurableBRAM
{
    private DualPortRAM _ram;

    public ConfigurableBRAM(int totalBits = 18_432, int width = 8)
    {
        ValidateConfiguration(totalBits, width);
        TotalBits = totalBits;
        Width = width;
        Depth = totalBits / width;
        _ram = NewRam();
    }

    public int TotalBits { get; }
    public int Width { get; private set; }
    public int Depth { get; private set; }

    public void Reconfigure(int width)
    {
        ValidateConfiguration(TotalBits, width);
        Width = width;
        Depth = TotalBits / width;
        _ram = NewRam();
    }

    public int[] TickA(int clock, int address, IReadOnlyList<int> dataIn, int writeEnable)
    {
        var result = _ram.Tick(clock, address, dataIn, writeEnable, 0, new int[Width], 0);
        return result.DataOutA;
    }

    public int[] TickB(int clock, int address, IReadOnlyList<int> dataIn, int writeEnable)
    {
        var result = _ram.Tick(clock, 0, new int[Width], 0, address, dataIn, writeEnable);
        return result.DataOutB;
    }

    private DualPortRAM NewRam() => new(Depth, Width, ReadMode.ReadFirst, ReadMode.ReadFirst);

    private static void ValidateConfiguration(int totalBits, int width)
    {
        if (totalBits < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(totalBits), totalBits, "totalBits must be >= 1");
        }

        if (width < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(width), width, "width must be >= 1");
        }

        if (totalBits % width != 0)
        {
            throw new ArgumentException($"width {width} does not evenly divide totalBits {totalBits}", nameof(width));
        }
    }
}
