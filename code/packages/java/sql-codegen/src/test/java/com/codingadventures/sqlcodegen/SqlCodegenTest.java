package com.codingadventures.sqlcodegen;

import com.codingadventures.sqlplanner.SqlPlanner;
import com.codingadventures.sqloptimizer.SqlOptimizer;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;

import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

// SqlCodegenTest.java — unit tests for the SQL bytecode code generator.
//
// Each test constructs an OptimizedPlan (or LogicalPlan for the convenience
// compile() entry point) and verifies that the emitted instruction list contains
// the expected instructions in the expected structural relationship.
//
// Tests are grouped by the plan node type they primarily exercise:
//   1. Expression compilation tests (compileExpr / compileExprCtx)
//   2. Basic scan / project / filter tests
//   3. JOIN tests
//   4. Aggregate tests
//   5. Post-processing (Sort / Limit / Distinct) tests
//   6. DML tests (INSERT / UPDATE / DELETE)
//   7. DDL tests (CREATE TABLE / DROP TABLE)
//   8. Program-level properties (labels, resultSchema, Halt position)
//   9. EmptyResult handling
//  10. compile(LogicalPlan) convenience method

class SqlCodegenTest {

    // ── Helpers ───────────────────────────────────────────────────────────────

    /** Count occurrences of an instruction class in the program. */
    private static long count(SqlCodegen.Program prog,
                               Class<? extends SqlCodegen.Instruction> cls) {
        return prog.instructions().stream().filter(cls::isInstance).count();
    }

    /** Find the first instruction of the given class, or throw. */
    @SuppressWarnings("unchecked")
    private static <T extends SqlCodegen.Instruction> T first(
            SqlCodegen.Program prog, Class<T> cls) {
        return prog.instructions().stream()
            .filter(cls::isInstance)
            .map(i -> (T) i)
            .findFirst()
            .orElseThrow(() -> new AssertionError("No " + cls.getSimpleName() + " found"));
    }

    /** Convenience: build a minimal Project(Scan) plan. */
    private static SqlOptimizer.OptimizedPlan projectScan(
            String table, String alias, String colName, String colAlias) {
        return new SqlOptimizer.OptimizedPlan.Project(
            new SqlOptimizer.OptimizedPlan.Scan(table, alias),
            List.of(new SqlPlanner.OutputColumn.Expr(
                new SqlPlanner.SqlExpr.Column(alias, colName), colAlias)));
    }

    // ── 1. Expression compilation tests ──────────────────────────────────────

    @Test
    @DisplayName("compileExpr: Literal produces LoadConst")
    void compileExpr_literal() {
        var instrs = SqlCodegen.compileExpr(new SqlPlanner.SqlExpr.Literal(42L));
        assertEquals(1, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.LoadConst.class, instrs.get(0));
        assertEquals(42L, ((SqlCodegen.Instruction.LoadConst) instrs.get(0)).value());
    }

    @Test
    @DisplayName("compileExpr: null Literal produces LoadConst(null)")
    void compileExpr_nullLiteral() {
        var instrs = SqlCodegen.compileExpr(new SqlPlanner.SqlExpr.Literal(null));
        assertEquals(1, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.LoadConst.class, instrs.get(0));
        assertNull(((SqlCodegen.Instruction.LoadConst) instrs.get(0)).value());
    }

    @Test
    @DisplayName("compileExpr: Column produces LoadColumn")
    void compileExpr_column() {
        var instrs = SqlCodegen.compileExpr(
            new SqlPlanner.SqlExpr.Column("u", "name"));
        assertEquals(1, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.LoadColumn.class, instrs.get(0));
        assertEquals("name", ((SqlCodegen.Instruction.LoadColumn) instrs.get(0)).column());
    }

    @Test
    @DisplayName("compileExpr: BinaryOp(ADD) produces [left, right, BinaryOp(ADD)]")
    void compileExpr_binaryAdd() {
        var expr = new SqlPlanner.SqlExpr.BinaryOp(
            SqlPlanner.BinaryOperator.ADD,
            new SqlPlanner.SqlExpr.Literal(3L),
            new SqlPlanner.SqlExpr.Literal(4L));
        var instrs = SqlCodegen.compileExpr(expr);
        assertEquals(3, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.LoadConst.class, instrs.get(0));
        assertInstanceOf(SqlCodegen.Instruction.LoadConst.class, instrs.get(1));
        assertInstanceOf(SqlCodegen.Instruction.BinaryOp.class, instrs.get(2));
        assertEquals(SqlCodegen.BinaryOpCode.ADD,
            ((SqlCodegen.Instruction.BinaryOp) instrs.get(2)).op());
    }

    @Test
    @DisplayName("compileExpr: UnaryOp(NEG) produces [expr, UnaryOp(NEG)]")
    void compileExpr_unaryNeg() {
        var expr = new SqlPlanner.SqlExpr.UnaryOp(
            SqlPlanner.UnaryOperator.NEG,
            new SqlPlanner.SqlExpr.Literal(5L));
        var instrs = SqlCodegen.compileExpr(expr);
        assertEquals(2, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.LoadConst.class, instrs.get(0));
        assertInstanceOf(SqlCodegen.Instruction.UnaryOp.class, instrs.get(1));
        assertEquals(SqlCodegen.UnaryOpCode.NEG,
            ((SqlCodegen.Instruction.UnaryOp) instrs.get(1)).op());
    }

    @Test
    @DisplayName("compileExpr: UnaryOp(NOT) produces [expr, UnaryOp(NOT)]")
    void compileExpr_unaryNot() {
        var expr = new SqlPlanner.SqlExpr.UnaryOp(
            SqlPlanner.UnaryOperator.NOT,
            new SqlPlanner.SqlExpr.Literal(true));
        var instrs = SqlCodegen.compileExpr(expr);
        assertEquals(2, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.UnaryOp.class, instrs.get(1));
        assertEquals(SqlCodegen.UnaryOpCode.NOT,
            ((SqlCodegen.Instruction.UnaryOp) instrs.get(1)).op());
    }

    @Test
    @DisplayName("compileExpr: IsNull produces [expr, IsNull]")
    void compileExpr_isNull() {
        var expr = new SqlPlanner.SqlExpr.IsNull(
            new SqlPlanner.SqlExpr.Column(null, "age"));
        var instrs = SqlCodegen.compileExpr(expr);
        assertEquals(2, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.LoadColumn.class, instrs.get(0));
        assertInstanceOf(SqlCodegen.Instruction.IsNull.class, instrs.get(1));
    }

    @Test
    @DisplayName("compileExpr: IsNotNull produces [expr, IsNotNull]")
    void compileExpr_isNotNull() {
        var expr = new SqlPlanner.SqlExpr.IsNotNull(
            new SqlPlanner.SqlExpr.Column(null, "age"));
        var instrs = SqlCodegen.compileExpr(expr);
        assertEquals(2, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.IsNotNull.class, instrs.get(1));
    }

    @Test
    @DisplayName("compileExpr: Between produces [value, low, high, Between]")
    void compileExpr_between() {
        var expr = new SqlPlanner.SqlExpr.Between(
            new SqlPlanner.SqlExpr.Column(null, "age"),
            new SqlPlanner.SqlExpr.Literal(18L),
            new SqlPlanner.SqlExpr.Literal(65L));
        var instrs = SqlCodegen.compileExpr(expr);
        assertEquals(4, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.LoadColumn.class, instrs.get(0));
        assertInstanceOf(SqlCodegen.Instruction.LoadConst.class, instrs.get(1));
        assertInstanceOf(SqlCodegen.Instruction.LoadConst.class, instrs.get(2));
        assertInstanceOf(SqlCodegen.Instruction.Between.class, instrs.get(3));
    }

    @Test
    @DisplayName("compileExpr: In(value, [a,b]) produces [value, a, b, InList(2)]")
    void compileExpr_inList() {
        var expr = new SqlPlanner.SqlExpr.In(
            new SqlPlanner.SqlExpr.Column(null, "id"),
            List.of(new SqlPlanner.SqlExpr.Literal(1L),
                    new SqlPlanner.SqlExpr.Literal(2L)));
        var instrs = SqlCodegen.compileExpr(expr);
        assertEquals(4, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.InList.class, instrs.get(3));
        assertEquals(2, ((SqlCodegen.Instruction.InList) instrs.get(3)).n());
    }

    @Test
    @DisplayName("compileExpr: Like produces [value, LoadConst(pattern), Like(false)]")
    void compileExpr_like() {
        var expr = new SqlPlanner.SqlExpr.Like(
            new SqlPlanner.SqlExpr.Column(null, "name"),
            "%Alice%");
        var instrs = SqlCodegen.compileExpr(expr);
        assertEquals(3, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.LoadColumn.class, instrs.get(0));
        assertInstanceOf(SqlCodegen.Instruction.LoadConst.class, instrs.get(1));
        assertEquals("%Alice%", ((SqlCodegen.Instruction.LoadConst) instrs.get(1)).value());
        assertInstanceOf(SqlCodegen.Instruction.Like.class, instrs.get(2));
        assertFalse(((SqlCodegen.Instruction.Like) instrs.get(2)).negated());
    }

    @Test
    @DisplayName("compileExpr: NotLike produces Like(negated=true)")
    void compileExpr_notLike() {
        var expr = new SqlPlanner.SqlExpr.NotLike(
            new SqlPlanner.SqlExpr.Column(null, "name"),
            "%Bob%");
        var instrs = SqlCodegen.compileExpr(expr);
        assertEquals(3, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.Like.class, instrs.get(2));
        assertTrue(((SqlCodegen.Instruction.Like) instrs.get(2)).negated());
    }

    @Test
    @DisplayName("compileExpr: FuncCall produces [args..., CallScalar]")
    void compileExpr_funcCall() {
        var expr = new SqlPlanner.SqlExpr.FuncCall(
            "UPPER",
            List.of(new SqlPlanner.SqlExpr.Column(null, "name")));
        var instrs = SqlCodegen.compileExpr(expr);
        assertEquals(2, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.LoadColumn.class, instrs.get(0));
        assertInstanceOf(SqlCodegen.Instruction.CallScalar.class, instrs.get(1));
        var cs = (SqlCodegen.Instruction.CallScalar) instrs.get(1);
        assertEquals("upper", cs.func()); // lowercased
        assertEquals(1, cs.nArgs());
    }

    @Test
    @DisplayName("compileExpr: all BinaryOperators map correctly")
    void compileExpr_allBinaryOps() {
        var opMap = Map.of(
            SqlPlanner.BinaryOperator.SUB,    SqlCodegen.BinaryOpCode.SUB,
            SqlPlanner.BinaryOperator.MUL,    SqlCodegen.BinaryOpCode.MUL,
            SqlPlanner.BinaryOperator.DIV,    SqlCodegen.BinaryOpCode.DIV,
            SqlPlanner.BinaryOperator.MOD,    SqlCodegen.BinaryOpCode.MOD,
            SqlPlanner.BinaryOperator.EQ,     SqlCodegen.BinaryOpCode.EQ,
            SqlPlanner.BinaryOperator.NOT_EQ, SqlCodegen.BinaryOpCode.NEQ,
            SqlPlanner.BinaryOperator.LT,     SqlCodegen.BinaryOpCode.LT,
            SqlPlanner.BinaryOperator.LTE,    SqlCodegen.BinaryOpCode.LTE,
            SqlPlanner.BinaryOperator.GT,     SqlCodegen.BinaryOpCode.GT,
            SqlPlanner.BinaryOperator.GTE,    SqlCodegen.BinaryOpCode.GTE
        );
        for (var entry : opMap.entrySet()) {
            var expr = new SqlPlanner.SqlExpr.BinaryOp(
                entry.getKey(),
                new SqlPlanner.SqlExpr.Literal(1L),
                new SqlPlanner.SqlExpr.Literal(2L));
            var instrs = SqlCodegen.compileExpr(expr);
            var bop = (SqlCodegen.Instruction.BinaryOp) instrs.get(2);
            assertEquals(entry.getValue(), bop.op(),
                "Operator " + entry.getKey() + " should map to " + entry.getValue());
        }
    }

    // ── 2. Scan / Project / Filter tests ─────────────────────────────────────

    @Test
    @DisplayName("Bare scan: program contains OpenScan, AdvanceCursor, CloseScan")
    void compileScan_basic() {
        var plan = projectScan("users", "u", "id", "id");
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(1, count(prog, SqlCodegen.Instruction.OpenScan.class));
        assertEquals(1, count(prog, SqlCodegen.Instruction.AdvanceCursor.class));
        assertEquals(1, count(prog, SqlCodegen.Instruction.CloseScan.class));
        assertEquals("users", first(prog, SqlCodegen.Instruction.OpenScan.class).table());
    }

    @Test
    @DisplayName("Project(Scan) emits SetResultSchema + BeginRow + EmitColumn + EmitRow")
    void compileProject_scan() {
        var plan = projectScan("products", "p", "price", "price");
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(1, count(prog, SqlCodegen.Instruction.SetResultSchema.class));
        assertTrue(count(prog, SqlCodegen.Instruction.BeginRow.class) >= 1);
        assertEquals(1, count(prog, SqlCodegen.Instruction.EmitColumn.class));
        assertTrue(count(prog, SqlCodegen.Instruction.EmitRow.class) >= 1);
    }

    @Test
    @DisplayName("Project(Scan) resultSchema contains output column name")
    void compileProject_schema() {
        var plan = projectScan("orders", "o", "amount", "total");
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(List.of("total"), prog.resultSchema());
    }

    @Test
    @DisplayName("Filter(Scan) emits JumpIfFalse instruction")
    void compileFilter_scan() {
        var scan   = new SqlOptimizer.OptimizedPlan.Scan("users", "u");
        var filter = new SqlOptimizer.OptimizedPlan.Filter(
            scan,
            new SqlPlanner.SqlExpr.BinaryOp(
                SqlPlanner.BinaryOperator.GT,
                new SqlPlanner.SqlExpr.Column("u", "age"),
                new SqlPlanner.SqlExpr.Literal(18L)));
        var proj = new SqlOptimizer.OptimizedPlan.Project(
            filter,
            List.of(new SqlPlanner.OutputColumn.Expr(
                new SqlPlanner.SqlExpr.Column("u", "name"), "name")));
        var prog = SqlCodegen.compileOptimized(proj);
        assertTrue(count(prog, SqlCodegen.Instruction.JumpIfFalse.class) >= 1);
    }

    @Test
    @DisplayName("EmptyResult at top level: only Halt in program")
    void compileEmptyResult_topLevel() {
        var plan = new SqlOptimizer.OptimizedPlan.EmptyResult();
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(1, count(prog, SqlCodegen.Instruction.Halt.class));
        assertEquals(0, count(prog, SqlCodegen.Instruction.OpenScan.class));
    }

    @Test
    @DisplayName("EmptyResult inside Filter: no scan loop emitted")
    void compileEmptyResult_insideFilter() {
        var plan = new SqlOptimizer.OptimizedPlan.Project(
            new SqlOptimizer.OptimizedPlan.EmptyResult(),
            List.of(new SqlPlanner.OutputColumn.Expr(
                new SqlPlanner.SqlExpr.Literal(1L), "x")));
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(0, count(prog, SqlCodegen.Instruction.OpenScan.class));
        assertEquals(1, count(prog, SqlCodegen.Instruction.Halt.class));
    }

    // ── 3. JOIN tests ─────────────────────────────────────────────────────────

    @Test
    @DisplayName("INNER JOIN: two nested AdvanceCursor instructions")
    void compileInnerJoin() {
        var left  = new SqlOptimizer.OptimizedPlan.Scan("users", "u");
        var right = new SqlOptimizer.OptimizedPlan.Scan("orders", "o");
        var join  = new SqlOptimizer.OptimizedPlan.Join(
            left, right,
            SqlPlanner.JoinKind.INNER,
            new SqlPlanner.SqlExpr.BinaryOp(
                SqlPlanner.BinaryOperator.EQ,
                new SqlPlanner.SqlExpr.Column("u", "id"),
                new SqlPlanner.SqlExpr.Column("o", "user_id")));
        var proj = new SqlOptimizer.OptimizedPlan.Project(
            join,
            List.of(new SqlPlanner.OutputColumn.Expr(
                new SqlPlanner.SqlExpr.Column("u", "name"), "name")));
        var prog = SqlCodegen.compileOptimized(proj);
        assertEquals(2, count(prog, SqlCodegen.Instruction.OpenScan.class));
        assertEquals(2, count(prog, SqlCodegen.Instruction.AdvanceCursor.class));
        assertEquals(2, count(prog, SqlCodegen.Instruction.CloseScan.class));
    }

    @Test
    @DisplayName("CROSS JOIN: no condition, two nested scans")
    void compileCrossJoin() {
        var left  = new SqlOptimizer.OptimizedPlan.Scan("a", "a");
        var right = new SqlOptimizer.OptimizedPlan.Scan("b", "b");
        var join  = new SqlOptimizer.OptimizedPlan.Join(
            left, right, SqlPlanner.JoinKind.CROSS, null);
        var proj = new SqlOptimizer.OptimizedPlan.Project(
            join,
            List.of(new SqlPlanner.OutputColumn.Expr(
                new SqlPlanner.SqlExpr.Column("a", "x"), "x")));
        var prog = SqlCodegen.compileOptimized(proj);
        assertEquals(2, count(prog, SqlCodegen.Instruction.OpenScan.class));
        assertEquals(0, count(prog, SqlCodegen.Instruction.JoinBeginRow.class));
    }

    @Test
    @DisplayName("LEFT JOIN: JoinBeginRow / JoinSetMatched / JoinIfMatched present")
    void compileLeftJoin() {
        var left  = new SqlOptimizer.OptimizedPlan.Scan("users", "u");
        var right = new SqlOptimizer.OptimizedPlan.Scan("orders", "o");
        var join  = new SqlOptimizer.OptimizedPlan.Join(
            left, right,
            SqlPlanner.JoinKind.LEFT,
            new SqlPlanner.SqlExpr.BinaryOp(
                SqlPlanner.BinaryOperator.EQ,
                new SqlPlanner.SqlExpr.Column("u", "id"),
                new SqlPlanner.SqlExpr.Column("o", "user_id")));
        var proj = new SqlOptimizer.OptimizedPlan.Project(
            join,
            List.of(new SqlPlanner.OutputColumn.Expr(
                new SqlPlanner.SqlExpr.Column("u", "name"), "name")));
        var prog = SqlCodegen.compileOptimized(proj);
        assertTrue(count(prog, SqlCodegen.Instruction.JoinBeginRow.class) >= 1);
        assertTrue(count(prog, SqlCodegen.Instruction.JoinSetMatched.class) >= 1);
        assertTrue(count(prog, SqlCodegen.Instruction.JoinIfMatched.class) >= 1);
    }

    // ── 4. Aggregate tests ────────────────────────────────────────────────────

    @Test
    @DisplayName("COUNT(*) aggregate: InitAgg(COUNT_STAR) + UpdateAgg + AdvanceGroupKey + FinalizeAgg")
    void compileAggregate_countStar() {
        var scan = new SqlOptimizer.OptimizedPlan.Scan("users", "u");
        var agg  = new SqlOptimizer.OptimizedPlan.Aggregate(
            scan,
            List.of(),
            List.of(new SqlPlanner.AggregateItem(
                SqlPlanner.AggFunction.COUNT,
                new SqlPlanner.AggArg.Star(),
                "_agg0",
                false)));
        var proj = new SqlOptimizer.OptimizedPlan.Project(
            agg,
            List.of(new SqlPlanner.OutputColumn.Expr(
                new SqlPlanner.SqlExpr.AggExpr(
                    SqlPlanner.AggFunction.COUNT,
                    new SqlPlanner.AggArg.Star(), false),
                "cnt")));
        var prog = SqlCodegen.compileOptimized(proj);
        assertEquals(1, count(prog, SqlCodegen.Instruction.InitAgg.class));
        assertEquals(1, count(prog, SqlCodegen.Instruction.UpdateAgg.class));
        assertEquals(1, count(prog, SqlCodegen.Instruction.AdvanceGroupKey.class));
        assertEquals(1, count(prog, SqlCodegen.Instruction.FinalizeAgg.class));
        var init = first(prog, SqlCodegen.Instruction.InitAgg.class);
        assertEquals(SqlCodegen.AggFunc.COUNT_STAR, init.func());
    }

    @Test
    @DisplayName("SUM aggregate maps to AggFunc.SUM")
    void compileAggregate_sum() {
        var scan = new SqlOptimizer.OptimizedPlan.Scan("orders", "o");
        var agg  = new SqlOptimizer.OptimizedPlan.Aggregate(
            scan,
            List.of(),
            List.of(new SqlPlanner.AggregateItem(
                SqlPlanner.AggFunction.SUM,
                new SqlPlanner.AggArg.Expr(new SqlPlanner.SqlExpr.Column("o", "amount")),
                "_agg0",
                false)));
        var proj = new SqlOptimizer.OptimizedPlan.Project(
            agg,
            List.of(new SqlPlanner.OutputColumn.Expr(
                new SqlPlanner.SqlExpr.AggExpr(
                    SqlPlanner.AggFunction.SUM,
                    new SqlPlanner.AggArg.Expr(new SqlPlanner.SqlExpr.Column("o", "amount")), false),
                "total")));
        var prog = SqlCodegen.compileOptimized(proj);
        var init = first(prog, SqlCodegen.Instruction.InitAgg.class);
        assertEquals(SqlCodegen.AggFunc.SUM, init.func());
    }

    @Test
    @DisplayName("GROUP BY aggregate: SaveGroupKey emitted in scan loop")
    void compileAggregate_groupBy() {
        var scan = new SqlOptimizer.OptimizedPlan.Scan("orders", "o");
        var agg  = new SqlOptimizer.OptimizedPlan.Aggregate(
            scan,
            List.of(new SqlPlanner.SqlExpr.Column("o", "status")),
            List.of(new SqlPlanner.AggregateItem(
                SqlPlanner.AggFunction.COUNT,
                new SqlPlanner.AggArg.Star(),
                "_agg0",
                false)));
        var proj = new SqlOptimizer.OptimizedPlan.Project(
            agg,
            List.of(
                new SqlPlanner.OutputColumn.Expr(
                    new SqlPlanner.SqlExpr.Column("o", "status"), "status"),
                new SqlPlanner.OutputColumn.Expr(
                    new SqlPlanner.SqlExpr.AggExpr(SqlPlanner.AggFunction.COUNT,
                        new SqlPlanner.AggArg.Star(), false), "cnt")));
        var prog = SqlCodegen.compileOptimized(proj);
        assertTrue(count(prog, SqlCodegen.Instruction.SaveGroupKey.class) >= 1);
        assertTrue(count(prog, SqlCodegen.Instruction.AdvanceGroupKey.class) >= 1);
    }

    // ── 5. Post-processing tests ──────────────────────────────────────────────

    @Test
    @DisplayName("Sort: SortResult emitted after scan loop")
    void compileSort() {
        var inner = projectScan("users", "u", "name", "name");
        var sort  = new SqlOptimizer.OptimizedPlan.Sort(
            inner,
            List.of(new SqlPlanner.SortKey(
                new SqlPlanner.SqlExpr.Column("u", "name"),
                SqlPlanner.SortDir.ASC,
                SqlPlanner.NullOrder.NULLS_LAST)));
        var prog = SqlCodegen.compileOptimized(sort);
        assertEquals(1, count(prog, SqlCodegen.Instruction.SortResult.class));
        // SortResult must appear after CloseScan
        int sortIdx = -1, closeIdx = -1;
        var instrs = prog.instructions();
        for (int i = 0; i < instrs.size(); i++) {
            if (instrs.get(i) instanceof SqlCodegen.Instruction.SortResult) sortIdx = i;
            if (instrs.get(i) instanceof SqlCodegen.Instruction.CloseScan) closeIdx = i;
        }
        assertTrue(sortIdx > closeIdx, "SortResult must come after CloseScan");
    }

    @Test
    @DisplayName("Sort key direction and nulls order are preserved")
    void compileSort_keyDetails() {
        var inner = projectScan("users", "u", "age", "age");
        var sort  = new SqlOptimizer.OptimizedPlan.Sort(
            inner,
            List.of(new SqlPlanner.SortKey(
                new SqlPlanner.SqlExpr.Column("u", "age"),
                SqlPlanner.SortDir.DESC,
                SqlPlanner.NullOrder.NULLS_FIRST)));
        var prog = SqlCodegen.compileOptimized(sort);
        var sr = first(prog, SqlCodegen.Instruction.SortResult.class);
        assertEquals(1, sr.keys().size());
        assertEquals(SqlCodegen.Direction.DESC, sr.keys().get(0).direction());
        assertEquals(SqlCodegen.NullsOrder.FIRST, sr.keys().get(0).nullsOrder());
        assertEquals("age", sr.keys().get(0).column());
    }

    @Test
    @DisplayName("Limit: LimitResult emitted with correct count and offset")
    void compileLimit() {
        var inner = projectScan("users", "u", "id", "id");
        var lim   = new SqlOptimizer.OptimizedPlan.Limit(inner, 10L, 5L);
        var prog  = SqlCodegen.compileOptimized(lim);
        assertEquals(1, count(prog, SqlCodegen.Instruction.LimitResult.class));
        var lr = first(prog, SqlCodegen.Instruction.LimitResult.class);
        assertEquals(10L, lr.count());
        assertEquals(5L, lr.offset());
    }

    @Test
    @DisplayName("Distinct: DistinctResult emitted after scan")
    void compileDistinct() {
        var inner = projectScan("users", "u", "name", "name");
        var dist  = new SqlOptimizer.OptimizedPlan.Distinct(inner);
        var prog  = SqlCodegen.compileOptimized(dist);
        assertEquals(1, count(prog, SqlCodegen.Instruction.DistinctResult.class));
    }

    @Test
    @DisplayName("Sort(Limit(Project(Scan))): SortResult appears before LimitResult")
    void compileSortLimit_ordering() {
        var inner = projectScan("users", "u", "name", "name");
        var lim   = new SqlOptimizer.OptimizedPlan.Limit(inner, 10L, null);
        var sort  = new SqlOptimizer.OptimizedPlan.Sort(
            lim,
            List.of(new SqlPlanner.SortKey(
                new SqlPlanner.SqlExpr.Column("u", "name"),
                SqlPlanner.SortDir.ASC,
                SqlPlanner.NullOrder.NULLS_LAST)));
        var prog  = SqlCodegen.compileOptimized(sort);
        int sortIdx  = -1, limitIdx = -1;
        var instrs = prog.instructions();
        for (int i = 0; i < instrs.size(); i++) {
            if (instrs.get(i) instanceof SqlCodegen.Instruction.SortResult)  sortIdx  = i;
            if (instrs.get(i) instanceof SqlCodegen.Instruction.LimitResult) limitIdx = i;
        }
        assertTrue(sortIdx >= 0, "SortResult not found");
        assertTrue(limitIdx >= 0, "LimitResult not found");
        assertTrue(sortIdx < limitIdx, "SortResult must precede LimitResult");
    }

    // ── 6. DML tests ──────────────────────────────────────────────────────────

    @Test
    @DisplayName("INSERT: InsertRow emitted for each value row")
    void compileInsert() {
        var plan = new SqlOptimizer.OptimizedPlan.Insert(
            "users",
            List.of("name", "age"),
            List.of(
                List.of(new SqlPlanner.SqlExpr.Literal("Alice"),
                        new SqlPlanner.SqlExpr.Literal(30L)),
                List.of(new SqlPlanner.SqlExpr.Literal("Bob"),
                        new SqlPlanner.SqlExpr.Literal(25L))));
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(2, count(prog, SqlCodegen.Instruction.InsertRow.class));
        var ir = first(prog, SqlCodegen.Instruction.InsertRow.class);
        assertEquals("users", ir.table());
        assertEquals(List.of("name", "age"), ir.columns());
    }

    @Test
    @DisplayName("UPDATE: scan loop + UpdateRows emitted")
    void compileUpdate() {
        var plan = new SqlOptimizer.OptimizedPlan.Update(
            "users",
            List.of(new SqlPlanner.Assignment("name",
                new SqlPlanner.SqlExpr.Literal("Charlie"))),
            new SqlPlanner.SqlExpr.BinaryOp(
                SqlPlanner.BinaryOperator.EQ,
                new SqlPlanner.SqlExpr.Column("users", "id"),
                new SqlPlanner.SqlExpr.Literal(1L)));
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(1, count(prog, SqlCodegen.Instruction.OpenScan.class));
        assertEquals(1, count(prog, SqlCodegen.Instruction.UpdateRows.class));
        assertTrue(count(prog, SqlCodegen.Instruction.JumpIfFalse.class) >= 1);
    }

    @Test
    @DisplayName("UPDATE without predicate: no JumpIfFalse emitted")
    void compileUpdate_noPredicate() {
        var plan = new SqlOptimizer.OptimizedPlan.Update(
            "users",
            List.of(new SqlPlanner.Assignment("active",
                new SqlPlanner.SqlExpr.Literal(true))),
            null); // no WHERE clause
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(0, count(prog, SqlCodegen.Instruction.JumpIfFalse.class));
        assertEquals(1, count(prog, SqlCodegen.Instruction.UpdateRows.class));
    }

    @Test
    @DisplayName("DELETE: scan loop + DeleteRows emitted")
    void compileDelete() {
        var plan = new SqlOptimizer.OptimizedPlan.Delete(
            "users",
            new SqlPlanner.SqlExpr.BinaryOp(
                SqlPlanner.BinaryOperator.LT,
                new SqlPlanner.SqlExpr.Column("users", "age"),
                new SqlPlanner.SqlExpr.Literal(18L)));
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(1, count(prog, SqlCodegen.Instruction.OpenScan.class));
        assertEquals(1, count(prog, SqlCodegen.Instruction.DeleteRows.class));
    }

    @Test
    @DisplayName("DELETE without predicate: no JumpIfFalse")
    void compileDelete_noPredicate() {
        var plan = new SqlOptimizer.OptimizedPlan.Delete("users", null);
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(0, count(prog, SqlCodegen.Instruction.JumpIfFalse.class));
        assertEquals(1, count(prog, SqlCodegen.Instruction.DeleteRows.class));
    }

    // ── 7. DDL tests ──────────────────────────────────────────────────────────

    @Test
    @DisplayName("CREATE TABLE: CreateTable instruction emitted + Halt")
    void compileCreateTable() {
        var cols = List.of(
            new SqlPlanner.ColumnDef("id",   "INTEGER", true, true,  false, null),
            new SqlPlanner.ColumnDef("name", "TEXT",    true, false, false, null));
        var plan = new SqlOptimizer.OptimizedPlan.CreateTable("employees", false, cols);
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(1, count(prog, SqlCodegen.Instruction.CreateTable.class));
        assertEquals(1, count(prog, SqlCodegen.Instruction.Halt.class));
        var ct = first(prog, SqlCodegen.Instruction.CreateTable.class);
        assertEquals("employees", ct.table());
        assertFalse(ct.ifNotExists());
        assertEquals(2, ct.columns().size());
    }

    @Test
    @DisplayName("CREATE TABLE IF NOT EXISTS: ifNotExists=true")
    void compileCreateTable_ifNotExists() {
        var plan = new SqlOptimizer.OptimizedPlan.CreateTable("t", true, List.of());
        var prog = SqlCodegen.compileOptimized(plan);
        assertTrue(first(prog, SqlCodegen.Instruction.CreateTable.class).ifNotExists());
    }

    @Test
    @DisplayName("DROP TABLE: DropTable instruction emitted + Halt")
    void compileDropTable() {
        var plan = new SqlOptimizer.OptimizedPlan.DropTable("old_table", false);
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(1, count(prog, SqlCodegen.Instruction.DropTable.class));
        assertEquals(1, count(prog, SqlCodegen.Instruction.Halt.class));
        var dt = first(prog, SqlCodegen.Instruction.DropTable.class);
        assertEquals("old_table", dt.table());
        assertFalse(dt.ifExists());
    }

    @Test
    @DisplayName("DROP TABLE IF EXISTS: ifExists=true")
    void compileDropTable_ifExists() {
        var plan = new SqlOptimizer.OptimizedPlan.DropTable("maybe", true);
        var prog = SqlCodegen.compileOptimized(plan);
        assertTrue(first(prog, SqlCodegen.Instruction.DropTable.class).ifExists());
    }

    // ── 8. Program-level property tests ──────────────────────────────────────

    @Test
    @DisplayName("Labels map contains entries for all Label instructions")
    void labelsMap_complete() {
        var plan = projectScan("users", "u", "id", "id");
        var prog = SqlCodegen.compileOptimized(plan);
        // Every Label instruction must have a corresponding entry in the labels map.
        for (var instr : prog.instructions()) {
            if (instr instanceof SqlCodegen.Instruction.Label lb) {
                assertTrue(prog.labels().containsKey(lb.name()),
                    "Label '" + lb.name() + "' not in labels map");
                assertEquals(prog.instructions().indexOf(lb),
                    prog.labels().get(lb.name()),
                    "Label '" + lb.name() + "' has wrong index");
            }
        }
    }

    @Test
    @DisplayName("Halt is always present in the program")
    void halt_alwaysPresent() {
        var plan = projectScan("x", "x", "y", "y");
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(1, count(prog, SqlCodegen.Instruction.Halt.class));
        // Halt is the last instruction.
        var instrs = prog.instructions();
        assertInstanceOf(SqlCodegen.Instruction.Halt.class, instrs.get(instrs.size() - 1));
    }

    @Test
    @DisplayName("Program.resultSchema matches SetResultSchema columns")
    void resultSchema_matchesSetResultSchema() {
        var plan = new SqlOptimizer.OptimizedPlan.Project(
            new SqlOptimizer.OptimizedPlan.Scan("users", "u"),
            List.of(
                new SqlPlanner.OutputColumn.Expr(
                    new SqlPlanner.SqlExpr.Column("u", "id"), "user_id"),
                new SqlPlanner.OutputColumn.Expr(
                    new SqlPlanner.SqlExpr.Column("u", "name"), "full_name")));
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(List.of("user_id", "full_name"), prog.resultSchema());
        var srs = first(prog, SqlCodegen.Instruction.SetResultSchema.class);
        assertEquals(prog.resultSchema(), srs.columns());
    }

    @Test
    @DisplayName("compile(LogicalPlan) convenience method works end-to-end")
    void compile_logicalPlan() {
        // Build a LogicalPlan directly and verify compile() produces a valid Program.
        var logical = new SqlPlanner.LogicalPlan.Project(
            new SqlPlanner.LogicalPlan.Scan("items", "i"),
            List.of(new SqlPlanner.OutputColumn.Expr(
                new SqlPlanner.SqlExpr.Column("i", "sku"), "sku")));
        var prog = SqlCodegen.compile(logical);
        assertNotNull(prog);
        assertTrue(prog.instructions().size() > 0);
        assertEquals(1, count(prog, SqlCodegen.Instruction.Halt.class));
    }

    @Test
    @DisplayName("Scan with alias registers alias in cursor map (LoadColumn uses alias's cursor)")
    void compileScan_aliasedCursor() {
        // The column reference uses alias "u" — the codegen must resolve it to cursor 0.
        var plan = new SqlOptimizer.OptimizedPlan.Project(
            new SqlOptimizer.OptimizedPlan.Scan("users", "u"),
            List.of(new SqlPlanner.OutputColumn.Expr(
                new SqlPlanner.SqlExpr.Column("u", "id"), "id")));
        var prog = SqlCodegen.compileOptimized(plan);
        // LoadColumn should reference cursor 0.
        var lc = first(prog, SqlCodegen.Instruction.LoadColumn.class);
        assertEquals(0, lc.cursorId());
        assertEquals("id", lc.column());
    }

    @Test
    @DisplayName("NotIn compiles to InList + UnaryOp(NOT)")
    void compileExpr_notIn() {
        var expr = new SqlPlanner.SqlExpr.NotIn(
            new SqlPlanner.SqlExpr.Column(null, "status"),
            List.of(new SqlPlanner.SqlExpr.Literal("deleted"),
                    new SqlPlanner.SqlExpr.Literal("archived")));
        var instrs = SqlCodegen.compileExpr(expr);
        // value + 2 items + InList + NOT = 5 instructions
        assertEquals(5, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.InList.class, instrs.get(3));
        assertInstanceOf(SqlCodegen.Instruction.UnaryOp.class, instrs.get(4));
        assertEquals(SqlCodegen.UnaryOpCode.NOT,
            ((SqlCodegen.Instruction.UnaryOp) instrs.get(4)).op());
    }

    @Test
    @DisplayName("FuncCall with multiple args: all args emitted before CallScalar")
    void compileExpr_funcCall_multipleArgs() {
        var expr = new SqlPlanner.SqlExpr.FuncCall(
            "SUBSTR",
            List.of(
                new SqlPlanner.SqlExpr.Column(null, "name"),
                new SqlPlanner.SqlExpr.Literal(1L),
                new SqlPlanner.SqlExpr.Literal(3L)));
        var instrs = SqlCodegen.compileExpr(expr);
        assertEquals(4, instrs.size());
        var cs = (SqlCodegen.Instruction.CallScalar) instrs.get(3);
        assertEquals("substr", cs.func());
        assertEquals(3, cs.nArgs());
    }

    @Test
    @DisplayName("compileExpr: AND and OR operators map correctly")
    void compileExpr_andOrOps() {
        var and = new SqlPlanner.SqlExpr.BinaryOp(
            SqlPlanner.BinaryOperator.AND,
            new SqlPlanner.SqlExpr.Literal(true),
            new SqlPlanner.SqlExpr.Literal(false));
        var andInstrs = SqlCodegen.compileExpr(and);
        assertEquals(SqlCodegen.BinaryOpCode.AND,
            ((SqlCodegen.Instruction.BinaryOp) andInstrs.get(2)).op());

        var or = new SqlPlanner.SqlExpr.BinaryOp(
            SqlPlanner.BinaryOperator.OR,
            new SqlPlanner.SqlExpr.Literal(true),
            new SqlPlanner.SqlExpr.Literal(false));
        var orInstrs = SqlCodegen.compileExpr(or);
        assertEquals(SqlCodegen.BinaryOpCode.OR,
            ((SqlCodegen.Instruction.BinaryOp) orInstrs.get(2)).op());
    }

    @Test
    @DisplayName("RIGHT JOIN: JoinBeginRow / JoinSetMatched / JoinIfMatched present")
    void compileRightJoin() {
        var left  = new SqlOptimizer.OptimizedPlan.Scan("users", "u");
        var right = new SqlOptimizer.OptimizedPlan.Scan("orders", "o");
        var join  = new SqlOptimizer.OptimizedPlan.Join(
            left, right,
            SqlPlanner.JoinKind.RIGHT,
            new SqlPlanner.SqlExpr.BinaryOp(
                SqlPlanner.BinaryOperator.EQ,
                new SqlPlanner.SqlExpr.Column("u", "id"),
                new SqlPlanner.SqlExpr.Column("o", "user_id")));
        var proj = new SqlOptimizer.OptimizedPlan.Project(
            join,
            List.of(new SqlPlanner.OutputColumn.Expr(
                new SqlPlanner.SqlExpr.Column("o", "id"), "oid")));
        var prog = SqlCodegen.compileOptimized(proj);
        assertTrue(count(prog, SqlCodegen.Instruction.JoinBeginRow.class) >= 1);
        assertTrue(count(prog, SqlCodegen.Instruction.JoinSetMatched.class) >= 1);
        assertTrue(count(prog, SqlCodegen.Instruction.JoinIfMatched.class) >= 1);
    }

    @Test
    @DisplayName("LEFT JOIN without condition: JoinSetMatched emitted unconditionally")
    void compileLeftJoin_noCondition() {
        var left  = new SqlOptimizer.OptimizedPlan.Scan("a", "a");
        var right = new SqlOptimizer.OptimizedPlan.Scan("b", "b");
        var join  = new SqlOptimizer.OptimizedPlan.Join(
            left, right, SqlPlanner.JoinKind.LEFT, null);
        var proj  = new SqlOptimizer.OptimizedPlan.Project(
            join,
            List.of(new SqlPlanner.OutputColumn.Expr(
                new SqlPlanner.SqlExpr.Column("a", "x"), "x")));
        var prog  = SqlCodegen.compileOptimized(proj);
        assertTrue(count(prog, SqlCodegen.Instruction.JoinSetMatched.class) >= 1);
    }

    @Test
    @DisplayName("Bare Aggregate (no outer Project): bare aggregate path runs")
    void compileBareAggregate() {
        // An Aggregate not wrapped by Project falls to the bare aggregate branch.
        var scan = new SqlOptimizer.OptimizedPlan.Scan("users", "u");
        var agg  = new SqlOptimizer.OptimizedPlan.Aggregate(
            scan,
            List.of(),
            List.of(new SqlPlanner.AggregateItem(
                SqlPlanner.AggFunction.MAX,
                new SqlPlanner.AggArg.Expr(new SqlPlanner.SqlExpr.Column("u", "age")),
                "_agg0",
                false)));
        var prog = SqlCodegen.compileOptimized(agg);
        assertTrue(count(prog, SqlCodegen.Instruction.InitAgg.class) >= 1);
        var init = first(prog, SqlCodegen.Instruction.InitAgg.class);
        assertEquals(SqlCodegen.AggFunc.MAX, init.func());
    }

    @Test
    @DisplayName("AVG and MIN aggregate functions map correctly")
    void compileAggregate_avgMin() {
        // AVG
        var scanAvg = new SqlOptimizer.OptimizedPlan.Scan("t", "t");
        var aggAvg  = new SqlOptimizer.OptimizedPlan.Aggregate(
            scanAvg, List.of(),
            List.of(new SqlPlanner.AggregateItem(
                SqlPlanner.AggFunction.AVG,
                new SqlPlanner.AggArg.Expr(new SqlPlanner.SqlExpr.Column("t", "v")),
                "_agg0", false)));
        var progAvg = SqlCodegen.compileOptimized(aggAvg);
        assertEquals(SqlCodegen.AggFunc.AVG, first(progAvg, SqlCodegen.Instruction.InitAgg.class).func());

        // MIN
        var scanMin = new SqlOptimizer.OptimizedPlan.Scan("t", "t");
        var aggMin  = new SqlOptimizer.OptimizedPlan.Aggregate(
            scanMin, List.of(),
            List.of(new SqlPlanner.AggregateItem(
                SqlPlanner.AggFunction.MIN,
                new SqlPlanner.AggArg.Expr(new SqlPlanner.SqlExpr.Column("t", "v")),
                "_agg0", false)));
        var progMin = SqlCodegen.compileOptimized(aggMin);
        assertEquals(SqlCodegen.AggFunc.MIN, first(progMin, SqlCodegen.Instruction.InitAgg.class).func());
    }

    @Test
    @DisplayName("COUNT(distinct col) maps to COUNT (not COUNT_STAR)")
    void compileAggregate_countDistinct() {
        var scan = new SqlOptimizer.OptimizedPlan.Scan("t", "t");
        var agg  = new SqlOptimizer.OptimizedPlan.Aggregate(
            scan, List.of(),
            List.of(new SqlPlanner.AggregateItem(
                SqlPlanner.AggFunction.COUNT,
                new SqlPlanner.AggArg.Expr(new SqlPlanner.SqlExpr.Column("t", "id")),
                "_agg0", true)));
        var prog = SqlCodegen.compileOptimized(agg);
        var init = first(prog, SqlCodegen.Instruction.InitAgg.class);
        assertEquals(SqlCodegen.AggFunc.COUNT, init.func());
        assertTrue(init.distinct());
    }

    @Test
    @DisplayName("Having node in scan body: strips Having and recurses to inner")
    void compileScanBody_havingStripped() {
        // Having appears in the scan-body path when an Aggregate is not wrapped by Project.
        // We build: Aggregate(Having(Scan)) — the Having wraps the raw scan.
        var scan    = new SqlOptimizer.OptimizedPlan.Scan("orders", "o");
        var having  = new SqlOptimizer.OptimizedPlan.Having(
            scan,
            new SqlPlanner.SqlExpr.BinaryOp(
                SqlPlanner.BinaryOperator.GT,
                new SqlPlanner.SqlExpr.Column("o", "amount"),
                new SqlPlanner.SqlExpr.Literal(100L)));
        var agg = new SqlOptimizer.OptimizedPlan.Aggregate(
            having, List.of(),
            List.of(new SqlPlanner.AggregateItem(
                SqlPlanner.AggFunction.COUNT, new SqlPlanner.AggArg.Star(), "_agg0", false)));
        var prog = SqlCodegen.compileOptimized(agg);
        // Should still produce a scan loop and aggregate instructions
        assertTrue(count(prog, SqlCodegen.Instruction.OpenScan.class) >= 1);
        assertTrue(count(prog, SqlCodegen.Instruction.InitAgg.class) >= 1);
    }

    @Test
    @DisplayName("compileExpr: AggExpr with Expr arg compiles arg expression")
    void compileExpr_aggExprWithArg() {
        // AggExpr is typically not compiled standalone, but when encountered
        // in a non-aggregate context the code should handle it gracefully.
        var expr = new SqlPlanner.SqlExpr.AggExpr(
            SqlPlanner.AggFunction.SUM,
            new SqlPlanner.AggArg.Expr(new SqlPlanner.SqlExpr.Column(null, "price")),
            false);
        var instrs = SqlCodegen.compileExpr(expr);
        // Should compile the inner expression
        assertEquals(1, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.LoadColumn.class, instrs.get(0));
    }

    @Test
    @DisplayName("compileExpr: AggExpr with Star arg returns LoadConst(null)")
    void compileExpr_aggExprStar() {
        var expr = new SqlPlanner.SqlExpr.AggExpr(
            SqlPlanner.AggFunction.COUNT,
            new SqlPlanner.AggArg.Star(),
            false);
        var instrs = SqlCodegen.compileExpr(expr);
        assertEquals(1, instrs.size());
        assertInstanceOf(SqlCodegen.Instruction.LoadConst.class, instrs.get(0));
        assertNull(((SqlCodegen.Instruction.LoadConst) instrs.get(0)).value());
    }

    @Test
    @DisplayName("compileExpr: Wildcard returns empty instruction list")
    void compileExpr_wildcard() {
        var instrs = SqlCodegen.compileExpr(new SqlPlanner.SqlExpr.Wildcard());
        assertEquals(0, instrs.size());
    }

    @Test
    @DisplayName("OutputColumn.Star in Project: schema contains *")
    void compileProject_starColumn() {
        var plan = new SqlOptimizer.OptimizedPlan.Project(
            new SqlOptimizer.OptimizedPlan.Scan("t", "t"),
            List.of(new SqlPlanner.OutputColumn.Star()));
        var prog = SqlCodegen.compileOptimized(plan);
        var srs = first(prog, SqlCodegen.Instruction.SetResultSchema.class);
        assertEquals(List.of("*"), srs.columns());
    }

    @Test
    @DisplayName("Project output column with no alias uses column name as schema name")
    void compileProject_columnNameInferredFromColumn() {
        // When alias is null and expression is a Column, use the column name.
        var plan = new SqlOptimizer.OptimizedPlan.Project(
            new SqlOptimizer.OptimizedPlan.Scan("t", "t"),
            List.of(new SqlPlanner.OutputColumn.Expr(
                new SqlPlanner.SqlExpr.Column("t", "price"), null)));
        var prog = SqlCodegen.compileOptimized(plan);
        assertEquals(List.of("price"), prog.resultSchema());
    }
}
