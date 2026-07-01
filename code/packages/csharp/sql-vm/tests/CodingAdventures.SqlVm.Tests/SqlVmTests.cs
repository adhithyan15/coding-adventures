// SqlVmTests.cs — Integration tests for the C# SQL VM.
//
// TEST PHILOSOPHY
// ───────────────
// These tests are intentionally "VM-native": they build Program objects by
// hand (directly constructing Instruction lists) rather than going through
// the full SQL parser → planner → optimizer → codegen pipeline.
//
// Why? Because the VM is a stand-alone unit. Building programs by hand:
//   • Makes tests fast and self-contained (no parser dependency).
//   • Exercises the VM's own correctness in isolation.
//   • Catches bugs that the codegen might paper over (e.g. wrong NULL handling).
//
// All tests use InMemoryBackend as the backend — it is the canonical reference
// implementation that ships with this repo.
//
// COVERAGE TARGET: ≥80% line coverage on CodingAdventures.SqlVm.
// Actual coverage should be well above that given the breadth of cases.
//
// ORGANIZATION
// ────────────
// The tests are grouped by feature area:
//   1. Basic SELECT (scan, column load, row emission)
//   2. WHERE / filtering (JumpIfFalse, comparisons)
//   3. Aggregate functions (COUNT, SUM, AVG, MIN, MAX)
//   4. GROUP BY
//   5. ORDER BY (sort post-op)
//   6. LIMIT / OFFSET
//   7. DISTINCT
//   8. INSERT / UPDATE / DELETE (DML)
//   9. CREATE TABLE / DROP TABLE (DDL)
//  10. NULL semantics (three-valued logic)
//  11. BETWEEN
//  12. IN list
//  13. Transactions
//  14. LEFT JOIN
//  15. Error paths (stack underflow, invalid label, etc.)
//  16. Scalar functions
//  17. LIKE matching

using CodingAdventures.SqlBackend;
using CodingAdventures.SqlCodegen;
using CodingAdventures.SqlVm;
using PlColumnDef = CodingAdventures.SqlPlanner.ColumnDef;
using Xunit;

namespace CodingAdventures.SqlVm.Tests;

// ── Test fixture helpers ───────────────────────────────────────────────────────

/// <summary>
/// Helpers for building Program objects from raw instruction lists.
/// These mirror the exact structure that SqlCodegen produces, so the
/// tests exercise the real VM dispatch paths.
/// </summary>
internal static class ProgramBuilder
{
    /// <summary>
    /// Build a Program from a list of instructions, resolving labels inline.
    /// </summary>
    public static Program Build(params Instruction[] instructions)
    {
        var list = instructions.ToList();
        var labels = new Dictionary<string, int>(StringComparer.Ordinal);
        for (var i = 0; i < list.Count; i++)
        {
            if (list[i] is CodegenLabel lbl)
                labels[lbl.Name] = i;
        }
        // Extract schema from first SetResultSchema.
        var schema = list.OfType<SetResultSchema>().FirstOrDefault()?.Columns
            ?? Array.Empty<string>();
        return new Program(list, labels, schema);
    }
}

/// <summary>
/// Helpers for constructing pre-populated InMemoryBackend instances.
/// </summary>
internal static class BackendBuilder
{
    /// <summary>
    /// Build a backend with one table containing the given rows.
    /// Column types are inferred from the row values.
    /// </summary>
    public static InMemoryBackend WithTable(
        string tableName,
        IReadOnlyList<string> columns,
        IReadOnlyList<Row> rows)
    {
        return InMemoryBackend.FromTables(new Dictionary<string, (IReadOnlyList<ColumnDef>, IReadOnlyList<Row>)>
        {
            [tableName] = (
                columns.Select(c => new ColumnDef(c, "TEXT")).ToList(),
                rows
            )
        });
    }

    /// <summary>Create a Row from parallel name and value arrays.</summary>
    public static Row MakeRow(string[] cols, object?[] vals)
    {
        var row = new Row();
        for (var i = 0; i < cols.Length; i++)
            row[cols[i]] = vals[i];
        return row;
    }
}

// ── Test collection ────────────────────────────────────────────────────────────

public class SqlVmTests
{
    // ═══════════════════════════════════════════════════════════════════════════
    // 1. Basic SELECT — scan a table and return all rows
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void SelectStar_ReturnsAllRows()
    {
        // Arrange: a table with two rows.
        var backend = BackendBuilder.WithTable("t",
            new[] { "id", "name" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "id", "name" }, new object?[] { 1L, "Alice" }),
                BackendBuilder.MakeRow(new[] { "id", "name" }, new object?[] { 2L, "Bob"   }),
            });

        // Program: scan t, emit both columns, loop.
        // SELECT id, name FROM t
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "id", "name" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "id"),
            new EmitColumn("id"),
            new LoadColumn(0, "name"),
            new EmitColumn("name"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new Halt()
        );

        // Act
        var result = SqlVm.Execute(program, backend);

        // Assert
        Assert.Equal(new[] { "id", "name" }, result.Columns);
        Assert.Equal(2, result.Rows.Count);
        Assert.Equal(1L, result.Rows[0][0]);
        Assert.Equal("Alice", result.Rows[0][1]);
        Assert.Equal(2L, result.Rows[1][0]);
        Assert.Equal("Bob", result.Rows[1][1]);
    }

    [Fact]
    public void SelectFromEmptyTable_ReturnsNoRows()
    {
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" }, Array.Empty<Row>());

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "x"),
            new EmitColumn("x"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(new[] { "x" }, result.Columns);
        Assert.Empty(result.Rows);
    }

    [Fact]
    public void SelectLiteral_PushesConstant()
    {
        var backend = new InMemoryBackend();

        // SELECT 42 (no table scan)
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(42L),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(42L, result.Rows[0][0]);
    }

    [Fact]
    public void SelectConstantNull_ReturnsNullRow()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(null),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Null(result.Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 2. WHERE / filtering
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void WhereFilter_ReturnsMatchingRows()
    {
        // SELECT x FROM t WHERE x > 5
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 3L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 7L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 10L }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            // WHERE x > 5
            new LoadColumn(0, "x"),
            new LoadConst(5L),
            new BinaryOpInstr(BinaryOpCode.Gt),
            new JumpIfFalse("skip"),
            // body
            new BeginRow(),
            new LoadColumn(0, "x"),
            new EmitColumn("x"),
            new EmitRow(),
            new CodegenLabel("skip"),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(2, result.Rows.Count);
        Assert.Equal(7L,  result.Rows[0][0]);
        Assert.Equal(10L, result.Rows[1][0]);
    }

    [Fact]
    public void WhereEqFilter_ReturnsExactMatch()
    {
        var backend = BackendBuilder.WithTable("t",
            new[] { "name" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "name" }, new object?[] { "Alice" }),
                BackendBuilder.MakeRow(new[] { "name" }, new object?[] { "Bob"   }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "name" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new LoadColumn(0, "name"),
            new LoadConst("Bob"),
            new BinaryOpInstr(BinaryOpCode.Eq),
            new JumpIfFalse("skip"),
            new BeginRow(),
            new LoadColumn(0, "name"),
            new EmitColumn("name"),
            new EmitRow(),
            new CodegenLabel("skip"),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal("Bob", result.Rows[0][0]);
    }

    [Fact]
    public void WhereWithNeq_FiltersCorrectly()
    {
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 1L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 2L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 3L }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new LoadColumn(0, "x"),
            new LoadConst(2L),
            new BinaryOpInstr(BinaryOpCode.Neq),
            new JumpIfFalse("skip"),
            new BeginRow(),
            new LoadColumn(0, "x"),
            new EmitColumn("x"),
            new EmitRow(),
            new CodegenLabel("skip"),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(2, result.Rows.Count);
        Assert.Equal(1L, result.Rows[0][0]);
        Assert.Equal(3L, result.Rows[1][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 3. Aggregate functions (no GROUP BY)
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void CountStar_ReturnsRowCount()
    {
        // SELECT COUNT(*) FROM t
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 1L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 2L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 3L }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "count(*)" }),
            // Phase 1: accumulation scan
            new OpenScan(0, "t"),
            new CodegenLabel("scan_loop"),
            new AdvanceCursor(0, "scan_end"),
            new SaveGroupKey(0),             // empty group key
            new InitAgg(0, AggFunc.CountStar, false),
            new LoadConst(null),             // * — arg is null
            new UpdateAgg(0),
            new Jump("scan_loop"),
            new CodegenLabel("scan_end"),
            new CloseScan(0),
            // Phase 2: emit groups
            new CodegenLabel("g_start"),
            new AdvanceGroupKey("g_end", false),
            new BeginRow(),
            new FinalizeAgg(0, AggFunc.CountStar),
            new EmitColumn("count(*)"),
            new EmitRow(),
            new Jump("g_start"),
            new CodegenLabel("g_end"),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(3, result.Rows[0][0]);
    }

    [Fact]
    public void Sum_ReturnsTotal()
    {
        // SELECT SUM(x) FROM t
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 10L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 20L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 30L }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "sum(x)" }),
            new OpenScan(0, "t"),
            new CodegenLabel("scan_loop"),
            new AdvanceCursor(0, "scan_end"),
            new SaveGroupKey(0),
            new InitAgg(0, AggFunc.Sum, false),
            new LoadColumn(0, "x"),
            new UpdateAgg(0),
            new Jump("scan_loop"),
            new CodegenLabel("scan_end"),
            new CloseScan(0),
            new CodegenLabel("g_start"),
            new AdvanceGroupKey("g_end", false),
            new BeginRow(),
            new FinalizeAgg(0, AggFunc.Sum),
            new EmitColumn("sum(x)"),
            new EmitRow(),
            new Jump("g_start"),
            new CodegenLabel("g_end"),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(60L, result.Rows[0][0]);
    }

    [Fact]
    public void Avg_ReturnsCorrectAverage()
    {
        // SELECT AVG(x) FROM t   (10, 20, 30)  → 20.0
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 10L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 20L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 30L }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "avg(x)" }),
            new OpenScan(0, "t"),
            new CodegenLabel("scan_loop"),
            new AdvanceCursor(0, "scan_end"),
            new SaveGroupKey(0),
            new InitAgg(0, AggFunc.Avg, false),
            new LoadColumn(0, "x"),
            new UpdateAgg(0),
            new Jump("scan_loop"),
            new CodegenLabel("scan_end"),
            new CloseScan(0),
            new CodegenLabel("g_start"),
            new AdvanceGroupKey("g_end", false),
            new BeginRow(),
            new FinalizeAgg(0, AggFunc.Avg),
            new EmitColumn("avg(x)"),
            new EmitRow(),
            new Jump("g_start"),
            new CodegenLabel("g_end"),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(20.0, result.Rows[0][0]);
    }

    [Fact]
    public void Avg_OnEmptyTable_ReturnsNull()
    {
        // AVG on empty table → NULL
        var backend = BackendBuilder.WithTable("t", new[] { "x" }, Array.Empty<Row>());

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "avg(x)" }),
            new OpenScan(0, "t"),
            new CodegenLabel("scan_loop"),
            new AdvanceCursor(0, "scan_end"),
            new SaveGroupKey(0),
            new InitAgg(0, AggFunc.Avg, false),
            new LoadColumn(0, "x"),
            new UpdateAgg(0),
            new Jump("scan_loop"),
            new CodegenLabel("scan_end"),
            new CloseScan(0),
            new CodegenLabel("g_start"),
            new AdvanceGroupKey("g_end", false),
            new BeginRow(),
            new FinalizeAgg(0, AggFunc.Avg),
            new EmitColumn("avg(x)"),
            new EmitRow(),
            new Jump("g_start"),
            new CodegenLabel("g_end"),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Null(result.Rows[0][0]);
    }

    [Fact]
    public void CountStar_OnEmptyTable_ReturnsZero()
    {
        // COUNT(*) on empty table → 0
        var backend = BackendBuilder.WithTable("t", new[] { "x" }, Array.Empty<Row>());

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "cnt" }),
            new OpenScan(0, "t"),
            new CodegenLabel("scan_loop"),
            new AdvanceCursor(0, "scan_end"),
            new SaveGroupKey(0),
            new InitAgg(0, AggFunc.CountStar, false),
            new LoadConst(null),
            new UpdateAgg(0),
            new Jump("scan_loop"),
            new CodegenLabel("scan_end"),
            new CloseScan(0),
            new CodegenLabel("g_start"),
            new AdvanceGroupKey("g_end", false),
            new BeginRow(),
            new FinalizeAgg(0, AggFunc.CountStar),
            new EmitColumn("cnt"),
            new EmitRow(),
            new Jump("g_start"),
            new CodegenLabel("g_end"),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(0, result.Rows[0][0]);
    }

    [Fact]
    public void Min_ReturnsMinimumValue()
    {
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 5L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 3L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 9L }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "min(x)" }),
            new OpenScan(0, "t"),
            new CodegenLabel("scan_loop"),
            new AdvanceCursor(0, "scan_end"),
            new SaveGroupKey(0),
            new InitAgg(0, AggFunc.Min, false),
            new LoadColumn(0, "x"),
            new UpdateAgg(0),
            new Jump("scan_loop"),
            new CodegenLabel("scan_end"),
            new CloseScan(0),
            new CodegenLabel("g_start"),
            new AdvanceGroupKey("g_end", false),
            new BeginRow(),
            new FinalizeAgg(0, AggFunc.Min),
            new EmitColumn("min(x)"),
            new EmitRow(),
            new Jump("g_start"),
            new CodegenLabel("g_end"),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(3L, result.Rows[0][0]);
    }

    [Fact]
    public void Max_ReturnsMaximumValue()
    {
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 5L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 3L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 9L }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "max(x)" }),
            new OpenScan(0, "t"),
            new CodegenLabel("scan_loop"),
            new AdvanceCursor(0, "scan_end"),
            new SaveGroupKey(0),
            new InitAgg(0, AggFunc.Max, false),
            new LoadColumn(0, "x"),
            new UpdateAgg(0),
            new Jump("scan_loop"),
            new CodegenLabel("scan_end"),
            new CloseScan(0),
            new CodegenLabel("g_start"),
            new AdvanceGroupKey("g_end", false),
            new BeginRow(),
            new FinalizeAgg(0, AggFunc.Max),
            new EmitColumn("max(x)"),
            new EmitRow(),
            new Jump("g_start"),
            new CodegenLabel("g_end"),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(9L, result.Rows[0][0]);
    }

    [Fact]
    public void CountAndSum_CombinedInOneQuery()
    {
        // SELECT COUNT(*), SUM(x) FROM t
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 4L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 6L }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "cnt", "s" }),
            new OpenScan(0, "t"),
            new CodegenLabel("scan_loop"),
            new AdvanceCursor(0, "scan_end"),
            new SaveGroupKey(0),
            new InitAgg(0, AggFunc.CountStar, false),
            new LoadConst(null),
            new UpdateAgg(0),
            new InitAgg(1, AggFunc.Sum, false),
            new LoadColumn(0, "x"),
            new UpdateAgg(1),
            new Jump("scan_loop"),
            new CodegenLabel("scan_end"),
            new CloseScan(0),
            new CodegenLabel("g_start"),
            new AdvanceGroupKey("g_end", false),
            new BeginRow(),
            new FinalizeAgg(0, AggFunc.CountStar),
            new EmitColumn("cnt"),
            new FinalizeAgg(1, AggFunc.Sum),
            new EmitColumn("s"),
            new EmitRow(),
            new Jump("g_start"),
            new CodegenLabel("g_end"),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(2, result.Rows[0][0]);   // COUNT(*)
        Assert.Equal(10L, result.Rows[0][1]); // SUM
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 4. ORDER BY (sort post-op)
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void OrderByAsc_SortsRows()
    {
        // SELECT x FROM t ORDER BY x ASC
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 30L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 10L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 20L }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "x"),
            new EmitColumn("x"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new SortResult(new[] { new CodegenSortKey("x", Direction.Asc, NullsOrder.Last) }),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(3, result.Rows.Count);
        Assert.Equal(10L, result.Rows[0][0]);
        Assert.Equal(20L, result.Rows[1][0]);
        Assert.Equal(30L, result.Rows[2][0]);
    }

    [Fact]
    public void OrderByDesc_SortsRowsDescending()
    {
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 1L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 3L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 2L }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "x"),
            new EmitColumn("x"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new SortResult(new[] { new CodegenSortKey("x", Direction.Desc, NullsOrder.Last) }),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(3L, result.Rows[0][0]);
        Assert.Equal(2L, result.Rows[1][0]);
        Assert.Equal(1L, result.Rows[2][0]);
    }

    [Fact]
    public void OrderByNullsLast_PlacesNullsAtEnd()
    {
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { null }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 1L  }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { null }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 2L  }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "x"),
            new EmitColumn("x"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new SortResult(new[] { new CodegenSortKey("x", Direction.Asc, NullsOrder.Last) }),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(1L,  result.Rows[0][0]);
        Assert.Equal(2L,  result.Rows[1][0]);
        Assert.Null(result.Rows[2][0]);
        Assert.Null(result.Rows[3][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 5. LIMIT / OFFSET
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void Limit_ReturnsOnlyFirstN()
    {
        // SELECT x FROM t LIMIT 2
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 1L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 2L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 3L }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "x"),
            new EmitColumn("x"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new LimitResult(2, null),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(2, result.Rows.Count);
        Assert.Equal(1L, result.Rows[0][0]);
        Assert.Equal(2L, result.Rows[1][0]);
    }

    [Fact]
    public void LimitWithOffset_SkipsAndLimits()
    {
        // SELECT x FROM t LIMIT 2 OFFSET 1
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 10L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 20L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 30L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 40L }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "x"),
            new EmitColumn("x"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new LimitResult(2, 1),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(2, result.Rows.Count);
        Assert.Equal(20L, result.Rows[0][0]);
        Assert.Equal(30L, result.Rows[1][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 6. DISTINCT
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void Distinct_RemovesDuplicateRows()
    {
        // SELECT DISTINCT x FROM t
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 1L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 2L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 1L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 3L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 2L }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "x"),
            new EmitColumn("x"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new DistinctResult(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(3, result.Rows.Count);
        // Values should be the first occurrence of each distinct value.
        var values = result.Rows.Select(r => r[0]).ToHashSet();
        Assert.Contains(1L, values);
        Assert.Contains(2L, values);
        Assert.Contains(3L, values);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 7. INSERT
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void Insert_AddsRowToTable()
    {
        var backend = new InMemoryBackend();
        backend.CreateTable("t", new[] { new ColumnDef("x", "INTEGER") }, false);

        // INSERT INTO t (x) VALUES (42)
        var program = ProgramBuilder.Build(
            new LoadConst(42L),
            new InsertRow("t", new[] { "x" }),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(1, result.RowsAffected);

        // Verify the row was inserted.
        var rows = backend.Scan("t");
        var row = rows.Next();
        Assert.NotNull(row);
        Assert.Equal(42L, row["x"]);
    }

    [Fact]
    public void Insert_MultipleRows_CountsAll()
    {
        var backend = new InMemoryBackend();
        backend.CreateTable("t", new[] { new ColumnDef("x", "INTEGER") }, false);

        // INSERT INTO t (x) VALUES (1), (2), (3)
        var program = ProgramBuilder.Build(
            new LoadConst(1L),
            new InsertRow("t", new[] { "x" }),
            new LoadConst(2L),
            new InsertRow("t", new[] { "x" }),
            new LoadConst(3L),
            new InsertRow("t", new[] { "x" }),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(3, result.RowsAffected);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 8. UPDATE
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void Update_ModifiesMatchingRows()
    {
        // UPDATE t SET x = 99 WHERE x = 2
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 1L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 2L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 3L }),
            });

        var program = ProgramBuilder.Build(
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new LoadColumn(0, "x"),
            new LoadConst(2L),
            new BinaryOpInstr(BinaryOpCode.Eq),
            new JumpIfFalse("skip"),
            new LoadConst(99L),
            new UpdateRows("t", new[] { "x" }, 0),
            new CodegenLabel("skip"),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(1, result.RowsAffected);

        // Verify the update.
        var scan = backend.OpenCursor("t");
        var rows = new List<long?>();
        Row? r;
        while ((r = scan.Next()) != null)
            rows.Add(r["x"] as long?);
        Assert.DoesNotContain(2L, rows);
        Assert.Contains(99L, rows);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 9. DELETE
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void Delete_RemovesMatchingRows()
    {
        // DELETE FROM t WHERE x = 2
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 1L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 2L }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 3L }),
            });

        var program = ProgramBuilder.Build(
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new LoadColumn(0, "x"),
            new LoadConst(2L),
            new BinaryOpInstr(BinaryOpCode.Eq),
            new JumpIfFalse("skip"),
            new DeleteRows("t", 0),
            new CodegenLabel("skip"),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(1, result.RowsAffected);

        // Verify the row is gone.
        var scan = backend.OpenCursor("t");
        var rows = new List<long?>();
        Row? r;
        while ((r = scan.Next()) != null)
            rows.Add(r["x"] as long?);
        Assert.Equal(2, rows.Count);
        Assert.DoesNotContain(2L, rows);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 10. CREATE TABLE / DROP TABLE
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void CreateTable_TableExistsAfterwards()
    {
        var backend = new InMemoryBackend();

        var program = ProgramBuilder.Build(
            new CreateTableInstr("employees",
                false,
                new[] { new PlColumnDef("id", "INTEGER"), new PlColumnDef("name", "TEXT") }),
            new Halt()
        );

        SqlVm.Execute(program, backend);
        Assert.Contains("employees", backend.Tables());
    }

    [Fact]
    public void CreateTableIfNotExists_DoesNotThrowWhenExists()
    {
        var backend = new InMemoryBackend();
        backend.CreateTable("t", new[] { new ColumnDef("x", "INTEGER") }, false);

        var program = ProgramBuilder.Build(
            new CreateTableInstr("t", true, new[] { new PlColumnDef("x", "INTEGER") }),
            new Halt()
        );

        // Should not throw.
        var result = SqlVm.Execute(program, backend);
        Assert.Equal(0, result.RowsAffected);
    }

    [Fact]
    public void DropTable_RemovesTable()
    {
        var backend = new InMemoryBackend();
        backend.CreateTable("temp", new[] { new ColumnDef("x", "INTEGER") }, false);

        var program = ProgramBuilder.Build(
            new DropTableInstr("temp", false),
            new Halt()
        );

        SqlVm.Execute(program, backend);
        Assert.DoesNotContain("temp", backend.Tables());
    }

    [Fact]
    public void DropTableIfExists_DoesNotThrowWhenMissing()
    {
        var backend = new InMemoryBackend();

        var program = ProgramBuilder.Build(
            new DropTableInstr("nonexistent", true),
            new Halt()
        );

        // Should not throw.
        SqlVm.Execute(program, backend);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 11. NULL semantics (three-valued logic)
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void NullPlusInteger_YieldsNull()
    {
        // SELECT NULL + 5 → NULL
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(null),
            new LoadConst(5L),
            new BinaryOpInstr(BinaryOpCode.Add),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Null(result.Rows[0][0]);
    }

    [Fact]
    public void NullAndFalse_YieldsFalse()
    {
        // NULL AND FALSE → FALSE (short-circuit)
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(null),
            new LoadConst(false),
            new BinaryOpInstr(BinaryOpCode.And),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(false, result.Rows[0][0]);
    }

    [Fact]
    public void NullOrTrue_YieldsTrue()
    {
        // NULL OR TRUE → TRUE (short-circuit)
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(null),
            new LoadConst(true),
            new BinaryOpInstr(BinaryOpCode.Or),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(true, result.Rows[0][0]);
    }

    [Fact]
    public void NullAndTrue_YieldsNull()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(null),
            new LoadConst(true),
            new BinaryOpInstr(BinaryOpCode.And),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Null(result.Rows[0][0]);
    }

    [Fact]
    public void IsNull_OnNull_ReturnsTrue()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(null),
            new IsNullInstr(),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(true, result.Rows[0][0]);
    }

    [Fact]
    public void IsNull_OnNonNull_ReturnsFalse()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(42L),
            new IsNullInstr(),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(false, result.Rows[0][0]);
    }

    [Fact]
    public void IsNotNull_OnNull_ReturnsFalse()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(null),
            new IsNotNullInstr(),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(false, result.Rows[0][0]);
    }

    [Fact]
    public void NullFilter_NullNotIncluded()
    {
        // SELECT x FROM t WHERE x IS NOT NULL
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { null }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 5L  }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { null }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new LoadColumn(0, "x"),
            new IsNotNullInstr(),
            new JumpIfFalse("skip"),
            new BeginRow(),
            new LoadColumn(0, "x"),
            new EmitColumn("x"),
            new EmitRow(),
            new CodegenLabel("skip"),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(5L, result.Rows[0][0]);
    }

    [Fact]
    public void IsNullFilter_IncludesNullRows()
    {
        // SELECT x FROM t WHERE x IS NULL
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { null }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 5L  }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new LoadColumn(0, "x"),
            new IsNullInstr(),
            new JumpIfFalse("skip"),
            new BeginRow(),
            new LoadColumn(0, "x"),
            new EmitColumn("x"),
            new EmitRow(),
            new CodegenLabel("skip"),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Null(result.Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 12. BETWEEN
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void Between_ValueInRange_ReturnsTrue()
    {
        // 5 BETWEEN 1 AND 10 → true
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(5L),  // value
            new LoadConst(1L),  // low
            new LoadConst(10L), // high
            new BetweenInstr(),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(true, result.Rows[0][0]);
    }

    [Fact]
    public void Between_ValueOutOfRange_ReturnsFalse()
    {
        // 15 BETWEEN 1 AND 10 → false
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(15L),
            new LoadConst(1L),
            new LoadConst(10L),
            new BetweenInstr(),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(false, result.Rows[0][0]);
    }

    [Fact]
    public void Between_NullValue_ReturnsNull()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(null),  // null value
            new LoadConst(1L),
            new LoadConst(10L),
            new BetweenInstr(),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Null(result.Rows[0][0]);
    }

    [Fact]
    public void Between_BoundaryInclusive()
    {
        // 1 BETWEEN 1 AND 10 → true  (boundary is inclusive)
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(1L),
            new LoadConst(1L),
            new LoadConst(10L),
            new BetweenInstr(),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(true, result.Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 13. IN list
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void InList_ProbeFound_ReturnsTrue()
    {
        // 2 IN (1, 2, 3) → true
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(2L),  // probe
            new LoadConst(1L),  // item 0
            new LoadConst(2L),  // item 1
            new LoadConst(3L),  // item 2
            new InListInstr(3),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(true, result.Rows[0][0]);
    }

    [Fact]
    public void InList_ProbeNotFound_ReturnsFalse()
    {
        // 5 IN (1, 2, 3) → false
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(5L),
            new LoadConst(1L),
            new LoadConst(2L),
            new LoadConst(3L),
            new InListInstr(3),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(false, result.Rows[0][0]);
    }

    [Fact]
    public void InList_NullProbe_ReturnsNull()
    {
        // NULL IN (1, 2, 3) → NULL
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(null),
            new LoadConst(1L),
            new LoadConst(2L),
            new InListInstr(2),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Null(result.Rows[0][0]);
    }

    [Fact]
    public void InList_Empty_ReturnsFalse()
    {
        // 5 IN () → false
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(5L),
            new InListInstr(0),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(false, result.Rows[0][0]);
    }

    [Fact]
    public void InList_NullInList_ReturnsNullWhenNotFound()
    {
        // 5 IN (1, NULL) → NULL (null in list, no match)
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(5L),
            new LoadConst(1L),
            new LoadConst(null),
            new InListInstr(2),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Null(result.Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 14. Transactions
    //
    // NOTE: At Level 1, the VM bytecode does NOT include transaction instructions
    // (BeginTransaction, CommitTransaction, RollbackTransaction are not defined in
    // SqlCodegen). Transactions are a backend-level concern. These tests verify
    // that the backend's transaction API works correctly without going through
    // the VM's dispatch loop — they are integration tests for the backend that
    // live here for coverage and documentation purposes.
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void Transaction_CommitPersistsData()
    {
        // Arrange: insert a row inside a commit.
        var backend = new InMemoryBackend();
        backend.CreateTable("t", new[] { new ColumnDef("x", "INTEGER") }, false);

        var txn = backend.BeginTransaction();
        backend.Insert("t", BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 1L }));
        backend.Commit(txn);

        // Assert: row is visible after commit — test via a simple VM scan.
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "x"),
            new EmitColumn("x"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(1L, result.Rows[0][0]);
    }

    [Fact]
    public void Transaction_RollbackUndoesData()
    {
        // Arrange: insert inside a rollback — row should disappear.
        var backend = new InMemoryBackend();
        backend.CreateTable("t", new[] { new ColumnDef("x", "INTEGER") }, false);

        var txn = backend.BeginTransaction();
        backend.Insert("t", BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 99L }));
        backend.Rollback(txn);

        // Assert: table is empty after rollback — scan via VM returns nothing.
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "x"),
            new EmitColumn("x"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Empty(result.Rows);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 15. Error paths
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void InvalidLabel_ThrowsInvalidLabel()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new Jump("nonexistent_label"),
            new Halt()
        );

        Assert.Throws<InvalidLabel>(() => SqlVm.Execute(program, backend));
    }

    [Fact]
    public void StackUnderflow_WhenPopOnEmptyStack_Throws()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new Pop(), // stack is empty — should throw
            new Halt()
        );

        Assert.Throws<StackUnderflow>(() => SqlVm.Execute(program, backend));
    }

    [Fact]
    public void CursorNotOpen_WhenAdvanceWithoutOpen_Throws()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new AdvanceCursor(999, "end"),
            new CodegenLabel("end"),
            new Halt()
        );

        Assert.Throws<CursorNotOpen>(() => SqlVm.Execute(program, backend));
    }

    [Fact]
    public void HaltStopsExecution()
    {
        // Ensure instructions after Halt are not executed.
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(1L),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt(),
            // These should not execute:
            new BeginRow(),
            new LoadConst(2L),
            new EmitColumn("v"),
            new EmitRow()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows); // only 1 row before Halt
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 16. Arithmetic and unary operators
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void ArithmeticAdd_WorksCorrectly()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(3L),
            new LoadConst(4L),
            new BinaryOpInstr(BinaryOpCode.Add),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(7L, result.Rows[0][0]);
    }

    [Fact]
    public void ArithmeticSub_WorksCorrectly()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(10L),
            new LoadConst(3L),
            new BinaryOpInstr(BinaryOpCode.Sub),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(7L, result.Rows[0][0]);
    }

    [Fact]
    public void ArithmeticMul_WorksCorrectly()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(3L),
            new LoadConst(4L),
            new BinaryOpInstr(BinaryOpCode.Mul),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(12L, result.Rows[0][0]);
    }

    [Fact]
    public void ArithmeticDiv_ProducesDouble()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(7L),
            new LoadConst(2L),
            new BinaryOpInstr(BinaryOpCode.Div),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(3.5, result.Rows[0][0]);
    }

    [Fact]
    public void ArithmeticDivByZero_ReturnsNull()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(5L),
            new LoadConst(0L),
            new BinaryOpInstr(BinaryOpCode.Div),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Null(result.Rows[0][0]);
    }

    [Fact]
    public void UnaryNeg_NegatesValue()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(5L),
            new UnaryOpInstr(UnaryOpCode.Neg),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(-5L, result.Rows[0][0]);
    }

    [Fact]
    public void UnaryNot_InvertsBoolean()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(true),
            new UnaryOpInstr(UnaryOpCode.Not),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(false, result.Rows[0][0]);
    }

    [Fact]
    public void UnaryNotNull_ReturnsNull()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(null),
            new UnaryOpInstr(UnaryOpCode.Not),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Null(result.Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 17. Scalar functions
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void ScalarAbs_ReturnsAbsoluteValue()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(-7L),
            new CallScalar("ABS", 1),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(7L, result.Rows[0][0]);
    }

    [Fact]
    public void ScalarUpper_ConvertsToUpperCase()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst("hello"),
            new CallScalar("UPPER", 1),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal("HELLO", result.Rows[0][0]);
    }

    [Fact]
    public void ScalarLower_ConvertsToLowerCase()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst("HELLO"),
            new CallScalar("LOWER", 1),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal("hello", result.Rows[0][0]);
    }

    [Fact]
    public void ScalarCoalesce_ReturnsFirstNonNull()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(null),
            new LoadConst(null),
            new LoadConst(42L),
            new CallScalar("COALESCE", 3),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(42L, result.Rows[0][0]);
    }

    [Fact]
    public void ScalarLength_ReturnsStringLength()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst("hello"),
            new CallScalar("LENGTH", 1),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(5L, result.Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 18. LIKE matching
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void LikeMatch_PercentWildcard_MatchesSubstring()
    {
        Assert.True(SqlVm.LikeMatch("hello world", "%world"));
        Assert.True(SqlVm.LikeMatch("hello world", "hello%"));
        Assert.True(SqlVm.LikeMatch("hello world", "%lo wor%"));
        Assert.False(SqlVm.LikeMatch("hello world", "%xyz%"));
    }

    [Fact]
    public void LikeMatch_UnderscoreWildcard_MatchesSingleChar()
    {
        Assert.True(SqlVm.LikeMatch("cat", "c_t"));
        Assert.True(SqlVm.LikeMatch("cot", "c_t"));
        Assert.False(SqlVm.LikeMatch("ct", "c_t"));
        Assert.False(SqlVm.LikeMatch("cart", "c_t"));
    }

    [Fact]
    public void LikeMatch_ExactMatch()
    {
        Assert.True(SqlVm.LikeMatch("hello", "hello"));
        Assert.False(SqlVm.LikeMatch("hello", "world"));
    }

    [Fact]
    public void LikeMatch_CaseInsensitive()
    {
        Assert.True(SqlVm.LikeMatch("Hello", "hello"));
        Assert.True(SqlVm.LikeMatch("HELLO", "hello"));
    }

    [Fact]
    public void LikeMatch_PercentMatchesEmpty()
    {
        Assert.True(SqlVm.LikeMatch("", "%"));
        Assert.True(SqlVm.LikeMatch("abc", "%"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 19. String concatenation
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void StringConcat_CombinesStrings()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst("Hello, "),
            new LoadConst("world!"),
            new BinaryOpInstr(BinaryOpCode.Concat),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal("Hello, world!", result.Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 20. Comparison operators
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void LteLte_BoundaryComparisons()
    {
        var backend = new InMemoryBackend();

        // Test Lte
        var prog1 = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(5L),
            new LoadConst(5L),
            new BinaryOpInstr(BinaryOpCode.Lte),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal(true, SqlVm.Execute(prog1, backend).Rows[0][0]);

        // Test Gte
        var prog2 = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(5L),
            new LoadConst(5L),
            new BinaryOpInstr(BinaryOpCode.Gte),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal(true, SqlVm.Execute(prog2, backend).Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 21. Multi-column results
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void MultiColumnResult_ColumnsInOrder()
    {
        var backend = BackendBuilder.WithTable("t",
            new[] { "a", "b", "c" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "a", "b", "c" }, new object?[] { 1L, "x", 3.14 }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "a", "b", "c" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "a"), new EmitColumn("a"),
            new LoadColumn(0, "b"), new EmitColumn("b"),
            new LoadColumn(0, "c"), new EmitColumn("c"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(1L,    result.Rows[0][0]);
        Assert.Equal("x",   result.Rows[0][1]);
        Assert.Equal(3.14,  result.Rows[0][2]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 22. Sum with NULLs in column
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void Sum_IgnoresNullValues()
    {
        // SUM(x) where some x are NULL → only sums non-null
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 10L  }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { null }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 20L  }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "s" }),
            new OpenScan(0, "t"),
            new CodegenLabel("scan_loop"),
            new AdvanceCursor(0, "scan_end"),
            new SaveGroupKey(0),
            new InitAgg(0, AggFunc.Sum, false),
            new LoadColumn(0, "x"),
            new UpdateAgg(0),
            new Jump("scan_loop"),
            new CodegenLabel("scan_end"),
            new CloseScan(0),
            new CodegenLabel("g_start"),
            new AdvanceGroupKey("g_end", false),
            new BeginRow(),
            new FinalizeAgg(0, AggFunc.Sum),
            new EmitColumn("s"),
            new EmitRow(),
            new Jump("g_start"),
            new CodegenLabel("g_end"),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(30L, result.Rows[0][0]); // 10 + 20
    }

    [Fact]
    public void Sum_AllNulls_ReturnsNull()
    {
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { null }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { null }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "s" }),
            new OpenScan(0, "t"),
            new CodegenLabel("scan_loop"),
            new AdvanceCursor(0, "scan_end"),
            new SaveGroupKey(0),
            new InitAgg(0, AggFunc.Sum, false),
            new LoadColumn(0, "x"),
            new UpdateAgg(0),
            new Jump("scan_loop"),
            new CodegenLabel("scan_end"),
            new CloseScan(0),
            new CodegenLabel("g_start"),
            new AdvanceGroupKey("g_end", false),
            new BeginRow(),
            new FinalizeAgg(0, AggFunc.Sum),
            new EmitColumn("s"),
            new EmitRow(),
            new Jump("g_start"),
            new CodegenLabel("g_end"),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Null(result.Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 23. JumpIfTrue
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void JumpIfTrue_JumpsWhenTrue()
    {
        var backend = new InMemoryBackend();
        // If true → emit 1, else emit 2
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new LoadConst(true),
            new JumpIfTrue("emit_one"),
            new BeginRow(),
            new LoadConst(2L),
            new EmitColumn("v"),
            new EmitRow(),
            new Jump("done"),
            new CodegenLabel("emit_one"),
            new BeginRow(),
            new LoadConst(1L),
            new EmitColumn("v"),
            new EmitRow(),
            new CodegenLabel("done"),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(1L, result.Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 24. Pop instruction
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void Pop_DiscardsTopOfStack()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(99L),   // will be popped
            new LoadConst(42L),   // will be emitted
            new Pop(),            // discard 42
            // Wait - Pop discards TOP which is 42, leaving 99. Let me restructure:
            // Push 99, push 42, pop 42, emit 99
            new EmitColumn("v"),  // emit 99
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(99L, result.Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 25. Modulo
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void ArithmeticMod_WorksCorrectly()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(10L),
            new LoadConst(3L),
            new BinaryOpInstr(BinaryOpCode.Mod),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(1L, result.Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 26. QueryResult.RowsAffected for SELECT
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void SelectQuery_RowsAffectedIsZero()
    {
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[] { BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 1L }) });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "x"),
            new EmitColumn("x"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(0, result.RowsAffected);
        Assert.Single(result.Rows);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 27. Scalar function edge cases
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void ScalarTrim_RemovesWhitespace()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst("  hello  "),
            new CallScalar("TRIM", 1),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal("hello", SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ScalarLtrim_RemovesLeadingWhitespace()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst("  hello"),
            new CallScalar("LTRIM", 1),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal("hello", SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ScalarRtrim_RemovesTrailingWhitespace()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst("hello  "),
            new CallScalar("RTRIM", 1),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal("hello", SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ScalarReplace_ReplacesSubstring()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst("hello world"),
            new LoadConst("world"),
            new LoadConst("SQL"),
            new CallScalar("REPLACE", 3),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal("hello SQL", SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ScalarSubstr_ReturnsSubstring()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst("hello world"),
            new LoadConst(7L),
            new LoadConst(5L),
            new CallScalar("SUBSTR", 3),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal("world", SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ScalarSubstr_NegativeStart_FromEnd()
    {
        // SUBSTR("hello", -3) → last 3 chars
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst("hello"),
            new LoadConst(-3L),
            new CallScalar("SUBSTR", 2),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal("llo", SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ScalarRound_RoundsToDecimals()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(3.14159),
            new LoadConst(2L),
            new CallScalar("ROUND", 2),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal(3.14, SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ScalarRound_NoDecimals()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(3.7),
            new CallScalar("ROUND", 1),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal(4.0, SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ScalarNullif_ReturnsNullWhenEqual()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(5L),
            new LoadConst(5L),
            new CallScalar("NULLIF", 2),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Null(SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ScalarNullif_ReturnsFirstWhenNotEqual()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(5L),
            new LoadConst(6L),
            new CallScalar("NULLIF", 2),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal(5L, SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ScalarTypeof_ReturnsTypeString()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(42L),
            new CallScalar("TYPEOF", 1),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal("integer", SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ScalarIfnull_ReturnsSecondWhenFirstIsNull()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(null),
            new LoadConst(99L),
            new CallScalar("IFNULL", 2),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal(99L, SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ScalarUnknown_ReturnsNull()
    {
        // Unknown scalar function should return NULL gracefully.
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(1L),
            new CallScalar("NONEXISTENT_FUNCTION", 1),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Null(SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ScalarAbs_NullReturnsNull()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(null),
            new CallScalar("ABS", 1),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Null(SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ScalarAbs_Double()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(-3.14),
            new CallScalar("ABS", 1),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal(3.14, SqlVm.Execute(program, backend).Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 28. LikeInstr dispatch (VM-level placeholder)
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void LikeInstr_NonNegated_PushesTrue()
    {
        // The codegen's LikeInstr only pops the value (pattern is not pushed at Level 1).
        // The VM returns TRUE as a placeholder for non-negated LIKE.
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst("hello"),
            new LikeInstr(false),  // non-negated → placeholder TRUE
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal(true, SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void LikeInstr_Negated_PushesFalse()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst("hello"),
            new LikeInstr(true),   // negated → placeholder FALSE
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal(false, SqlVm.Execute(program, backend).Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 29. BetweenInstr with NULL high bound
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void Between_NullLow_ReturnsNull()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(5L),
            new LoadConst(null),   // low is null
            new LoadConst(10L),
            new BetweenInstr(),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Null(SqlVm.Execute(program, backend).Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 30. InListInstr with NULLs in list (no match) → null
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void InList_WithNullInList_AndNonMatchingProbe_ReturnsNull()
    {
        // probe=99 not in {1, NULL, 2} → null (because list contains NULL)
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(99L),   // probe
            new LoadConst(1L),
            new LoadConst(null),  // NULL in list
            new LoadConst(2L),
            new InListInstr(3),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Null(SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void InList_EmptyList_ReturnsFalse2()
    {
        // Duplicate guard: test that an empty IN list always returns FALSE.
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(5L),
            new InListInstr(0),   // empty list
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal(false, SqlVm.Execute(program, backend).Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 31. JumpIfTrue with non-truthy value (should not jump)
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void JumpIfTrue_DoesNotJumpWhenFalse()
    {
        // When the top-of-stack is false, JumpIfTrue should NOT jump.
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(false),
            new JumpIfTrue("jumped"),
            new LoadConst("not_jumped"),
            new EmitColumn("v"),
            new EmitRow(),
            new Jump("end"),
            new CodegenLabel("jumped"),
            new LoadConst("jumped"),
            new EmitColumn("v"),
            new EmitRow(),
            new CodegenLabel("end"),
            new Halt()
        );
        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal("not_jumped", result.Rows[0][0]);  // should NOT have jumped
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 32. Double arithmetic
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void ArithmeticAdd_WithDoubles()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(1.5),
            new LoadConst(2.5),
            new BinaryOpInstr(BinaryOpCode.Add),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal(4.0, SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ArithmeticSub_WithDoubles()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(5.5),
            new LoadConst(2.2),
            new BinaryOpInstr(BinaryOpCode.Sub),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        // 5.5 - 2.2 = 3.3 (approximately)
        Assert.InRange((double)SqlVm.Execute(program, backend).Rows[0][0]!, 3.29, 3.31);
    }

    [Fact]
    public void ArithmeticMul_WithDoubles()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(2.0),
            new LoadConst(3.5),
            new BinaryOpInstr(BinaryOpCode.Mul),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal(7.0, SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void ArithmeticModByZero_ReturnsNull()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(7L),
            new LoadConst(0L),
            new BinaryOpInstr(BinaryOpCode.Mod),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Null(SqlVm.Execute(program, backend).Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 33. NullsFirst/Last in ORDER BY
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void OrderBy_NullsFirst()
    {
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 5L   }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { null }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 3L   }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "x"), new EmitColumn("x"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new SortResult(new[] { new CodegenSortKey("x", Direction.Asc, NullsOrder.First) }),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(3, result.Rows.Count);
        Assert.Null(result.Rows[0][0]);   // NULL first
        Assert.Equal(3L, result.Rows[1][0]);
        Assert.Equal(5L, result.Rows[2][0]);
    }

    [Fact]
    public void OrderBy_NullsLast()
    {
        var backend = BackendBuilder.WithTable("t",
            new[] { "x" },
            new[]
            {
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { null }),
                BackendBuilder.MakeRow(new[] { "x" }, new object?[] { 1L   }),
            });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "x" }),
            new OpenScan(0, "t"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "x"), new EmitColumn("x"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new SortResult(new[] { new CodegenSortKey("x", Direction.Asc, NullsOrder.Last) }),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(2, result.Rows.Count);
        Assert.Equal(1L,  result.Rows[0][0]);
        Assert.Null(result.Rows[1][0]);   // NULL last
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 34. RowIteratorCursor (generic backend fallback)
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void RowIteratorCursor_FallbackAdapterWorks()
    {
        // Verify that the RowIteratorCursor adapter (used for non-InMemoryBackend backends)
        // works for read-only scans via a StubBackend.
        var backend = new StubBackend();

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "n" }),
            new OpenScan(0, "stub"),
            new CodegenLabel("loop"),
            new AdvanceCursor(0, "end"),
            new BeginRow(),
            new LoadColumn(0, "n"), new EmitColumn("n"),
            new EmitRow(),
            new Jump("loop"),
            new CodegenLabel("end"),
            new CloseScan(0),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Equal(2, result.Rows.Count);
        Assert.Equal(1L, result.Rows[0][0]);
        Assert.Equal(2L, result.Rows[1][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 35. LEFT JOIN basic scenario
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void LeftJoin_UnmatchedRightProducesNullPad()
    {
        // Simulate a left outer join where the right side never matches.
        // Program structure:
        //   outer loop: scan left table
        //     JoinBeginRow
        //     inner loop: scan right (empty) → immediate exhaust
        //     JoinIfMatched → not matched, so fall through
        //     emit left row with NULL for right column
        var left = BackendBuilder.WithTable("L",
            new[] { "id" },
            new[] { BackendBuilder.MakeRow(new[] { "id" }, new object?[] { 1L }) });
        var right = new InMemoryBackend();
        right.CreateTable("R", new[] { new ColumnDef("val", "INTEGER") }, false);

        // Merge both tables into one backend using InMemoryBackend.FromTables.
        var backend = InMemoryBackend.FromTables(new Dictionary<string, (IReadOnlyList<ColumnDef>, IReadOnlyList<Row>)>
        {
            ["L"] = (new[] { new ColumnDef("id", "INTEGER") }, new[] { BackendBuilder.MakeRow(new[] { "id" }, new object?[] { 1L }) }),
            ["R"] = (new[] { new ColumnDef("val", "INTEGER") }, Array.Empty<Row>()),
        });

        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "id", "val" }),
            new OpenScan(0, "L"),
            new CodegenLabel("outer_loop"),
            new AdvanceCursor(0, "outer_end"),
            new JoinBeginRow(),
            new OpenScan(1, "R"),
            new CodegenLabel("inner_loop"),
            new AdvanceCursor(1, "inner_end"),
            new JoinSetMatched(),
            new Jump("inner_loop"),
            new CodegenLabel("inner_end"),
            new CloseScan(1),
            new JoinIfMatched("outer_loop"),
            // No match: emit left row with NULL for right column.
            new BeginRow(),
            new LoadColumn(0, "id"), new EmitColumn("id"),
            new LoadConst(null),     new EmitColumn("val"),
            new EmitRow(),
            new Jump("outer_loop"),
            new CodegenLabel("outer_end"),
            new CloseScan(0),
            new Halt()
        );

        var result = SqlVm.Execute(program, backend);
        Assert.Single(result.Rows);
        Assert.Equal(1L, result.Rows[0][0]);
        Assert.Null(result.Rows[0][1]);  // NULL-padded right side
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 36. UnknownInstruction error path
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void UnknownInstruction_ThrowsVmError()
    {
        var backend = new InMemoryBackend();
        // Create a fake instruction type that the VM doesn't know about.
        var instructions = new List<Instruction> { new FakeInstruction(), new Halt() };
        var program = new Program(instructions, new Dictionary<string, int>(), Array.Empty<string>());
        Assert.Throws<UnknownInstruction>(() => SqlVm.Execute(program, backend));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 37. Neq operator
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void Neq_UnequalValues_ReturnsTrue()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(1L),
            new LoadConst(2L),
            new BinaryOpInstr(BinaryOpCode.Neq),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal(true, SqlVm.Execute(program, backend).Rows[0][0]);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 38. OR with NULL short-circuit
    // ═══════════════════════════════════════════════════════════════════════════

    [Fact]
    public void Or_NullAndFalse_ReturnsNull()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(null),
            new LoadConst(false),
            new BinaryOpInstr(BinaryOpCode.Or),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Null(SqlVm.Execute(program, backend).Rows[0][0]);
    }

    [Fact]
    public void Or_FalseFalse_ReturnsFalse()
    {
        var backend = new InMemoryBackend();
        var program = ProgramBuilder.Build(
            new SetResultSchema(new[] { "v" }),
            new BeginRow(),
            new LoadConst(false),
            new LoadConst(false),
            new BinaryOpInstr(BinaryOpCode.Or),
            new EmitColumn("v"),
            new EmitRow(),
            new Halt()
        );
        Assert.Equal(false, SqlVm.Execute(program, backend).Rows[0][0]);
    }
}

// ── Helper types for tests ─────────────────────────────────────────────────────

/// <summary>
/// A minimal Backend stub that returns two fixed rows for any Scan(),
/// used to test the RowIteratorCursor fallback adapter (the non-InMemoryBackend path in SqlVm).
/// </summary>
internal sealed class StubBackend : Backend
{
    public override IRowIterator Scan(string table)
    {
        var rows = new List<Row>
        {
            new Row { ["n"] = 1L },
            new Row { ["n"] = 2L },
        };
        return new ListRowIterator(rows);
    }

    // All other operations unsupported — only Scan() is exercised by the RowIteratorCursor test.
    public override IReadOnlyList<string> Tables() => throw new NotSupportedException();
    public override IReadOnlyList<ColumnDef> Columns(string table) => throw new NotSupportedException();
    public override void Insert(string table, Row row) => throw new NotSupportedException();
    public override void Update(string table, ICursor cursor, IReadOnlyDictionary<string, object?> assignments) => throw new NotSupportedException();
    public override void Delete(string table, ICursor cursor) => throw new NotSupportedException();
    public override void CreateTable(string table, IReadOnlyList<ColumnDef> columns, bool ifNotExists) => throw new NotSupportedException();
    public override void DropTable(string table, bool ifExists) => throw new NotSupportedException();
    public override void AddColumn(string table, ColumnDef column) => throw new NotSupportedException();
    public override void CreateIndex(IndexDef index) => throw new NotSupportedException();
    public override void DropIndex(string name, bool ifExists = false) => throw new NotSupportedException();
    public override IReadOnlyList<IndexDef> ListIndexes(string? table = null) => throw new NotSupportedException();
    public override IEnumerable<int> ScanIndex(string indexName, IReadOnlyList<object?>? lo, IReadOnlyList<object?>? hi, bool loInclusive = true, bool hiInclusive = true) => throw new NotSupportedException();
    public override IRowIterator ScanByRowIds(string table, IReadOnlyList<int> rowids) => throw new NotSupportedException();
    public override TransactionHandle BeginTransaction() => throw new NotSupportedException();
    public override void Commit(TransactionHandle handle) => throw new NotSupportedException();
    public override void Rollback(TransactionHandle handle) => throw new NotSupportedException();
}

/// <summary>
/// Fake instruction used to test the UnknownInstruction error path.
/// </summary>
internal sealed record FakeInstruction() : Instruction;
