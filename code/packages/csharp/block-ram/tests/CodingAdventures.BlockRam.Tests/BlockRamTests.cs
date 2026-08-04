using CodingAdventures.BlockRam;

public sealed class SRAMTests
{
    [Fact]
    public void CellHoldsReadsAndWritesOneBit()
    {
        var cell = new SRAMCell();
        Assert.Equal(0, cell.Value);
        Assert.Null(cell.Read(0));
        cell.Write(0, 1);
        Assert.Equal(0, cell.Value);
        cell.Write(1, 1);
        Assert.Equal(1, cell.Read(1));
        cell.Write(1, 0);
        Assert.Equal(0, cell.Value);
    }

    [Fact]
    public void CellRejectsNonBits()
    {
        var cell = new SRAMCell();
        Assert.Throws<ArgumentOutOfRangeException>(() => cell.Read(2));
        Assert.Throws<ArgumentOutOfRangeException>(() => cell.Write(1, -1));
    }

    [Fact]
    public void ArrayStoresIndependentRows()
    {
        var memory = new SRAMArray(3, 4);
        Assert.Equal((3, 4), memory.Shape);
        memory.Write(0, [1, 0, 1, 0]);
        memory.Write(2, [0, 1, 0, 1]);
        Assert.Equal([1, 0, 1, 0], memory.Read(0));
        Assert.Equal([0, 0, 0, 0], memory.Read(1));
        Assert.Equal([0, 1, 0, 1], memory.Read(2));
    }

    [Fact]
    public void ArrayValidatesShapeAddressAndData()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new SRAMArray(0, 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => new SRAMArray(1, 0));
        var memory = new SRAMArray(2, 2);
        Assert.Throws<ArgumentOutOfRangeException>(() => memory.Read(2));
        Assert.Throws<ArgumentException>(() => memory.Write(0, [1]));
        Assert.Throws<ArgumentNullException>(() => memory.Write(0, null!));
        Assert.Throws<ArgumentOutOfRangeException>(() => memory.Write(0, [0, 2]));
    }
}

public sealed class SinglePortRAMTests
{
    [Fact]
    public void ReadFirstReturnsOldValueAndWritesNewValue()
    {
        var ram = new SinglePortRAM(4, 4);
        Write(ram, 0, [1, 0, 1, 0]);
        Assert.Equal([1, 0, 1, 0], Write(ram, 0, [0, 1, 0, 1]));
        Assert.Equal([0, 1, 0, 1], Read(ram, 0));
        Assert.Equal(4, ram.Depth);
        Assert.Equal(4, ram.Width);
    }

    [Fact]
    public void WriteFirstReturnsNewValue()
    {
        var ram = new SinglePortRAM(2, 2, ReadMode.WriteFirst);
        Assert.Equal([1, 1], Write(ram, 0, [1, 1]));
    }

    [Fact]
    public void NoChangeRetainsOutputDuringWrite()
    {
        var ram = new SinglePortRAM(2, 2, ReadMode.NoChange);
        Assert.Equal([0, 0], Read(ram, 0));
        Assert.Equal([0, 0], Write(ram, 0, [1, 1]));
        Assert.Equal([1, 1], Read(ram, 0));
    }

    [Fact]
    public void OnlyRisingEdgePerformsOperationAndResultsAreCopies()
    {
        var ram = new SinglePortRAM(2, 2, ReadMode.WriteFirst);
        var low = ram.Tick(0, 0, new[] { 1, 0 }, 1);
        Assert.Equal([0, 0], low);
        var high = ram.Tick(1, 0, new[] { 1, 0 }, 1);
        high[0] = 0;
        Assert.Equal([1, 0], ram.Tick(1, 0, new[] { 0, 0 }, 0));
        Assert.Equal([1, 0], Read(ram, 0));
    }

    [Fact]
    public void DumpReturnsAllRows()
    {
        var ram = new SinglePortRAM(3, 2);
        Write(ram, 1, [1, 0]);
        var dump = ram.Dump();
        Assert.Equal(3, dump.Length);
        Assert.Equal([0, 0], dump[0]);
        Assert.Equal([1, 0], dump[1]);
    }

    [Fact]
    public void ValidatesConstructorAndSignals()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new SinglePortRAM(0, 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => new SinglePortRAM(1, 0));
        var ram = new SinglePortRAM(2, 2);
        Assert.Throws<ArgumentOutOfRangeException>(() => ram.Tick(2, 0, [0, 0], 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => ram.Tick(1, 0, [0, 0], 2));
        Assert.Throws<ArgumentOutOfRangeException>(() => ram.Tick(1, 2, [0, 0], 0));
        Assert.Throws<ArgumentException>(() => ram.Tick(1, 0, [0], 0));
    }

    private static int[] Write(SinglePortRAM ram, int address, int[] data)
    {
        ram.Tick(0, address, data, 1);
        return ram.Tick(1, address, data, 1);
    }

    private static int[] Read(SinglePortRAM ram, int address)
    {
        var zeros = new int[ram.Width];
        ram.Tick(0, address, zeros, 0);
        return ram.Tick(1, address, zeros, 0);
    }
}

public sealed class DualPortRAMTests
{
    [Fact]
    public void PortsReadAndWriteDifferentAddressesTogether()
    {
        var ram = new DualPortRAM(4, 4);
        ram.Tick(0, 0, new[] { 1, 0, 0, 0 }, 1, 1, new[] { 0, 1, 0, 0 }, 1);
        ram.Tick(1, 0, new[] { 1, 0, 0, 0 }, 1, 1, new[] { 0, 1, 0, 0 }, 1);
        ram.Tick(0, 0, new int[4], 0, 1, new int[4], 0);
        var result = ram.Tick(1, 0, new int[4], 0, 1, new int[4], 0);
        Assert.Equal([1, 0, 0, 0], result.DataOutA);
        Assert.Equal([0, 1, 0, 0], result.DataOutB);
        Assert.Equal(4, ram.Depth);
        Assert.Equal(4, ram.Width);
    }

    [Fact]
    public void CollisionReportsAddressWithoutWriting()
    {
        var ram = new DualPortRAM(2, 2);
        ram.Tick(0, 0, new[] { 1, 0 }, 1, 0, new[] { 0, 1 }, 1);
        var error = Assert.Throws<WriteCollisionException>(() =>
            ram.Tick(1, 0, new[] { 1, 0 }, 1, 0, new[] { 0, 1 }, 1));
        Assert.Equal(0, error.Address);
        Assert.Contains("address 0", error.Message);
    }

    [Fact]
    public void PerPortReadModesAreIndependent()
    {
        var ram = new DualPortRAM(4, 2, ReadMode.NoChange, ReadMode.WriteFirst);
        ram.Tick(0, 0, new[] { 1, 1 }, 1, 1, new[] { 1, 0 }, 1);
        var result = ram.Tick(1, 0, new[] { 1, 1 }, 1, 1, new[] { 1, 0 }, 1);
        Assert.Equal([0, 0], result.DataOutA);
        Assert.Equal([1, 0], result.DataOutB);

        ram.Tick(0, 0, new[] { 0, 0 }, 0, 1, new[] { 0, 1 }, 1);
        result = ram.Tick(1, 0, new[] { 0, 0 }, 0, 1, new[] { 0, 1 }, 1);
        Assert.Equal([1, 1], result.DataOutA);
        Assert.Equal([0, 1], result.DataOutB);
    }

    [Fact]
    public void ReadFirstPortReturnsOldData()
    {
        var ram = new DualPortRAM(2, 2);
        ram.Tick(0, 0, new[] { 1, 1 }, 1, 1, new int[2], 0);
        ram.Tick(1, 0, new[] { 1, 1 }, 1, 1, new int[2], 0);
        ram.Tick(0, 0, new[] { 0, 0 }, 1, 1, new int[2], 0);
        var result = ram.Tick(1, 0, new[] { 0, 0 }, 1, 1, new int[2], 0);
        Assert.Equal([1, 1], result.DataOutA);
    }

    [Fact]
    public void ValidatesConstructorAndPortInputs()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new DualPortRAM(0, 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => new DualPortRAM(1, 0));
        var ram = new DualPortRAM(2, 2);
        Assert.Throws<ArgumentOutOfRangeException>(() => ram.Tick(2, 0, [0, 0], 0, 0, [0, 0], 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => ram.Tick(1, 0, [0, 0], 2, 0, [0, 0], 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => ram.Tick(1, 0, [0, 0], 0, 0, [0, 0], -1));
        Assert.Throws<ArgumentOutOfRangeException>(() => ram.Tick(1, -1, [0, 0], 0, 0, [0, 0], 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => ram.Tick(1, 0, [0, 0], 0, 2, [0, 0], 0));
        Assert.Throws<ArgumentException>(() => ram.Tick(1, 0, [0], 0, 0, [0, 0], 0));
        Assert.Throws<ArgumentException>(() => ram.Tick(1, 0, [0, 0], 0, 0, [0], 0));
    }
}

public sealed class ConfigurableBRAMTests
{
    [Fact]
    public void DefaultsToAnEighteenKilobitEightWideBlock()
    {
        var bram = new ConfigurableBRAM();
        Assert.Equal(18_432, bram.TotalBits);
        Assert.Equal(8, bram.Width);
        Assert.Equal(2_304, bram.Depth);
    }

    [Fact]
    public void PortsShareStorage()
    {
        var bram = new ConfigurableBRAM(64, 4);
        bram.TickA(0, 3, new[] { 1, 0, 1, 1 }, 1);
        bram.TickA(1, 3, new[] { 1, 0, 1, 1 }, 1);
        bram.TickB(0, 3, new int[4], 0);
        Assert.Equal([1, 0, 1, 1], bram.TickB(1, 3, new int[4], 0));
    }

    [Fact]
    public void ReconfigureChangesShapeAndClearsStorage()
    {
        var bram = new ConfigurableBRAM(64, 4);
        bram.TickB(0, 0, new[] { 1, 1, 1, 1 }, 1);
        bram.TickB(1, 0, new[] { 1, 1, 1, 1 }, 1);
        bram.Reconfigure(8);
        Assert.Equal(8, bram.Width);
        Assert.Equal(8, bram.Depth);
        bram.TickA(0, 0, new int[8], 0);
        Assert.Equal(new int[8], bram.TickA(1, 0, new int[8], 0));
    }

    [Fact]
    public void ValidatesConfigurations()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new ConfigurableBRAM(0, 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => new ConfigurableBRAM(8, 0));
        Assert.Throws<ArgumentException>(() => new ConfigurableBRAM(8, 3));
        var bram = new ConfigurableBRAM(8, 2);
        Assert.Throws<ArgumentOutOfRangeException>(() => bram.Reconfigure(0));
        Assert.Throws<ArgumentException>(() => bram.Reconfigure(3));
    }
}
