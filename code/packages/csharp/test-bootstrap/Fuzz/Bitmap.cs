using System.Numerics;

namespace CodingAdventures.TestBootstrap.Fuzz;

/// <summary>
/// A 65,536-bit execution map and the admission and scheduling rules around it
/// (VER06 section 4, VER06h section 4).
/// </summary>
/// <remarks>
/// Bit <c>e</c> set means edge <c>e</c> was executed. Edge ids are reduced modulo 65,536 as
/// <em>unsigned</em> 64-bit numbers. Collisions are the known price of a fixed-size map; AFL pays
/// the same one.
/// </remarks>
public sealed class Bitmap
{
    /// <summary>Number of edges the map distinguishes.</summary>
    public const int CoverageMapSize = 65_536;

    /// <summary>Size of the map in bytes.</summary>
    public const int CoverageBytes = CoverageMapSize / 8;

    private readonly byte[] _map = new byte[CoverageBytes];

    // The masked ids in the order first seen, beside the map, so listing the edges of one
    // execution costs O(edges hit), not O(8192).
    private int[] _edges = new int[16];
    private int _size;

    private static int SlotOf(ulong edge) => (int)(edge & (CoverageMapSize - 1)); // low 16 bits == mod 65536

    /// <summary>Marks an edge as executed.</summary>
    public void Record(ulong edge)
    {
        int e = SlotOf(edge);
        int idx = e >> 3;
        int mask = 1 << (e & 7);
        if ((_map[idx] & mask) == 0)
        {
            _map[idx] |= (byte)mask;
            if (_size == _edges.Length)
            {
                Array.Resize(ref _edges, _size * 2);
            }

            _edges[_size++] = e;
        }
    }

    /// <summary>Whether an edge is set.</summary>
    public bool Has(ulong edge)
    {
        int e = SlotOf(edge);
        return (_map[e >> 3] & (1 << (e & 7))) != 0;
    }

    /// <summary>The result of <see cref="Admit"/>: whether any edge was new and how many were.</summary>
    public readonly record struct Admission(bool HasNew, int CountNew);

    /// <summary>ORs the trace into this (cumulative) map; reports whether any edge was new and how many.</summary>
    public Admission Admit(int[] trace)
    {
        int fresh = 0;
        foreach (int e in trace)
        {
            if (!Has((ulong)e))
            {
                Record((ulong)e);
                fresh++;
            }
        }

        return new Admission(fresh > 0, fresh);
    }

    /// <summary>Every covered edge in ascending order (a fresh array).</summary>
    public int[] Edges()
    {
        int[] output = new int[_size];
        Array.Copy(_edges, output, _size);
        Array.Sort(output);
        return output;
    }

    /// <summary>The edges in first-seen order (a fresh array); the campaign's per-execution trace.</summary>
    internal int[] Trace()
    {
        int[] output = new int[_size];
        Array.Copy(_edges, output, _size);
        return output;
    }

    /// <summary>The number of distinct covered edges (a population count).</summary>
    public int Count
    {
        get
        {
            int n = 0;
            foreach (byte b in _map)
            {
                n += BitOperations.PopCount(b);
            }

            return n;
        }
    }

    /// <summary>ORs another map into this one.</summary>
    public void Merge(Bitmap other)
    {
        for (int i = 0; i < other._size; i++)
        {
            Record((ulong)other._edges[i]);
        }
    }

    /// <summary>Clears the map in O(edges recorded).</summary>
    public void Reset()
    {
        for (int i = 0; i < _size; i++)
        {
            int e = _edges[i];
            _map[e >> 3] &= (byte)~(1 << (e & 7));
        }

        _size = 0;
    }

    /// <summary>A copy of the 8,192 bytes of the map (LSB-first within a byte).</summary>
    public byte[] Raw() => (byte[])_map.Clone();

    /// <summary>The result of <see cref="CorpusAdmit"/>.</summary>
    public readonly record struct Novelty(bool Admitted, int NewEdges);

    /// <summary>
    /// The pure novelty check: does the trace hold an edge absent from <paramref name="existing"/>?
    /// It counts UNIQUE novel edges (a trace that repeats one novel edge three times adds one). Ids
    /// are compared as full unsigned 64-bit words, not reduced.
    /// <code>
    ///   CorpusAdmit([1 2 3], [2 4]) -> (true, 1)    (edge 4 is new)
    ///   CorpusAdmit([1 2 3], [1 2]) -> (false, 0)
    /// </code>
    /// </summary>
    public static Novelty CorpusAdmit(ulong[] existing, ulong[] trace)
    {
        HashSet<ulong> have = new(existing);
        int novel = 0;
        foreach (ulong e in trace)
        {
            if (have.Add(e)) // count each novel edge once
            {
                novel++;
            }
        }

        return new Novelty(novel > 0, novel);
    }

    /// <summary>
    /// The mutation budget: <c>max(1, round(base * (1 + new / total) / sqrt(len)))</c>.
    /// </summary>
    /// <remarks>
    /// Short seeds that found a large share of the coverage get the most energy. The edge bonus is
    /// 1.0 while nothing is covered (total == 0). Rounding is half AWAY from zero
    /// (<c>floor(x + 0.5)</c>), as in the other lanes: <see cref="Math.Round(double)"/> rounds half
    /// to even and would turn 2.5 into 2.
    /// <para>Worked example: base 10, len 4, 3 new of 6 total edges: 10 * 1.5 / 2 = 7.5 rounds to 8.</para>
    /// </remarks>
    public static int CalculateEnergy(int baseEnergy, int seedLen, int newEdges, int totalEdges)
    {
        double b = Math.Max(1, baseEnergy);
        double bonus = totalEdges > 0 ? 1.0 + ((double)newEdges / totalEdges) : 1.0;
        double factor = Math.Sqrt(Math.Max(1, seedLen));

        // Clamp BEFORE the int conversion: no campaign runs more than MaxIterations slots.
        double energy = Math.Floor((b * bonus / factor) + 0.5);
        return Math.Max(1, (int)Math.Min(energy, Mutate.MaxIterations));
    }
}
