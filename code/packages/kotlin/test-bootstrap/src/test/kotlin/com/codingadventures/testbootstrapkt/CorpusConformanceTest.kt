package com.codingadventures.testbootstrapkt

import com.codingadventures.testbootstrap.corpusjson.StrictJson
import java.math.BigInteger
import java.nio.file.Files
import java.nio.file.Path
import kotlin.test.AfterTest
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * Replays the whole committed pbt-v1 corpus through the KOTLIN API, with its
 * negative controls and the Kotlin-layer sabotage controls (VER05f sections 3, 4).
 * The fixture is read from its committed path, never a copy.
 */
class CorpusConformanceTest {
    private fun fixturePath(): Path {
        var dir: Path? = Path.of("").toAbsolutePath()
        while (dir != null) {
            val candidate = dir.resolve("code/specs/fixtures/pbt-v1/cases.json")
            if (Files.isRegularFile(candidate)) return candidate
            dir = dir.parent
        }
        throw AssertionError("could not find code/specs/fixtures/pbt-v1/cases.json above the working directory")
    }

    private fun loadCases(): List<Any?> = KotlinCorpusRunner.parse(Files.readAllBytes(fixturePath()))

    @AfterTest
    fun resetSeams() = Seams.reset()

    // ---- deep copies and corruption ------------------------------------------

    @Suppress("UNCHECKED_CAST")
    private fun deepCopy(node: Any?): Any? =
        when (node) {
            is Map<*, *> -> LinkedHashMap<String, Any?>().also { out -> (node as Map<String, Any?>).forEach { (k, v) -> out[k] = deepCopy(v) } }
            is List<*> -> node.map { deepCopy(it) }.toMutableList<Any?>()
            else -> node
        }

    /** Same shape, different value. */
    @Suppress("UNCHECKED_CAST")
    private fun corruptValue(node: Any?): Any? {
        if (node is Boolean) return !node
        val m = node as MutableMap<String, Any?>
        if (m["kind"] == "int") {
            m["value"] = BigInteger(m["value"] as String).add(BigInteger.ONE).toString()
        } else {
            val hex = m["hex"] as String
            m["hex"] = if (hex.isEmpty()) "ff" else hex.substring(2)
        }
        return m
    }

    @Suppress("UNCHECKED_CAST")
    private fun corruptFirst(c: MutableMap<String, Any?>, field: String) {
        val list = c[field] as MutableList<Any?>
        list[0] = corruptValue(list[0])
    }

    /** Each variant is one corruption of the case; every one must be noticed. */
    private fun corruptions(op: String): List<(MutableMap<String, Any?>) -> Unit> =
        when (op) {
            "gen_primitive", "gen_bounded", "gen_bytes" -> listOf({ c -> corruptFirst(c, "outputs") })
            "shrink_int", "shrink_refinement", "shrink_bytes" ->
                listOf(
                    { c -> c["expected_minimal"] = corruptValue(c["expected_minimal"]) },
                    { c -> c["expected_steps"] = corruptValue(c["expected_steps"]) },
                )
            "shrink_list" -> listOf({ c -> corruptFirst(c, "expected_minimal") }, { c -> c["expected_steps"] = corruptValue(c["expected_steps"]) })
            "replay_consistency" ->
                listOf(
                    { c -> c["initial_fail"] = corruptValue(c["initial_fail"]) },
                    { c -> c["expected_minimal"] = corruptValue(c["expected_minimal"]) },
                )
            "filter_exhausted" -> listOf({ c -> c["failure_id"] = "property-panic" }, { c -> c["verdict"] = "pass" })
            "failure_id_valid" -> listOf({ c -> c["valid"] = corruptValue(c["valid"]) })
            "corpus_reject" -> listOf({ c -> c["text"] = "tfv 1\nr R1 Kop S706274\n" })
            else -> throw AssertionError("no corruption defined for op $op")
        }

    @Suppress("UNCHECKED_CAST")
    private fun copyAll(cases: List<Any?>): MutableList<Any?> = cases.map { deepCopy(it) }.toMutableList()

    // ---- the positive run ------------------------------------------------------

    @Test
    fun corpusConformsToAllTwentyFiveCasesThroughKotlin() {
        val report = KotlinCorpusRunner.evaluate(loadCases())
        assertEquals(emptyList(), report.problems)
        assertEquals(25, report.evaluated, "exactly 25 cases must be fully evaluated")
        assertEquals(25, KotlinCorpusRunner.CASE_COUNT)
    }

    @Test
    fun everyOpIsPresentInTheFixture() {
        val ops = loadCases().map { (it as Map<*, *>)["op"] as String }.toSortedSet()
        assertEquals(11, ops.size, "ops: $ops")
    }

    // ---- negative controls -----------------------------------------------------

    /**
     * Per case: corrupt the expected value in every way its op allows and require
     * the run to fail and to name exactly that case.
     */
    @Suppress("UNCHECKED_CAST")
    @Test
    fun corruptingAnyCaseMakesTheRunFailAndNamesThatCase() {
        val pristine = loadCases()
        var corrupted = 0
        for (i in pristine.indices) {
            val original = pristine[i] as Map<String, Any?>
            val id = original["id"] as String
            val op = original["op"] as String
            for (corruption in corruptions(op)) {
                val cases = copyAll(pristine)
                val victim = cases[i] as MutableMap<String, Any?>
                corruption(victim)
                assertFalse(deepCopy(original) == victim, "$id: the corruption must change the case")
                val report = KotlinCorpusRunner.evaluate(cases)
                assertTrue(report.problems.isNotEmpty(), "$id ($op): corruption went unnoticed")
                for (p in report.problems) assertTrue(p.startsWith("$id:"), "$id: problem names another case or none: $p")
                corrupted++
            }
        }
        assertTrue(corrupted >= 25, "every case must have been corrupted at least once, got $corrupted")
    }

    @Test
    fun droppingAnyCaseIsAProblem() {
        val pristine = loadCases()
        for (i in pristine.indices) {
            val cases = pristine.toMutableList().also { it.removeAt(i) }
            val report = KotlinCorpusRunner.evaluate(cases)
            assertTrue(report.problems.isNotEmpty(), "dropping case $i went unnoticed")
            assertContains(report.problems[0], "expected 25 cases, got 24")
        }
        assertTrue(KotlinCorpusRunner.evaluate(emptyList()).problems.isNotEmpty())
        assertTrue(KotlinCorpusRunner.evaluate(pristine.subList(0, 8)).problems.isNotEmpty())
    }

    @Test
    fun aTruncatedDocumentFailsToParse() {
        val data = Files.readAllBytes(fixturePath())
        assertFailsWith<StrictJson.JsonException> { KotlinCorpusRunner.parse(data.copyOf(data.size / 2)) }
    }

    @Test
    fun duplicatedIdsUnknownOpsAndBadFieldSetsAreProblems() {
        val dup = copyAll(loadCases())
        dup[1] = deepCopy(dup[0])
        assertContains(KotlinCorpusRunner.evaluate(dup).problems, "duplicate or missing case id")

        @Suppress("UNCHECKED_CAST")
        val first = copyAll(loadCases()).also { }
        @Suppress("UNCHECKED_CAST")
        val c0 = first[0] as MutableMap<String, Any?>
        val id = c0["id"] as String
        fun problemFor(fragment: String) = KotlinCorpusRunner.evaluate(first).problems.any { it.startsWith("$id:") && fragment in it }
        c0["op"] = "gen_everything"
        assertTrue(problemFor("unknown op"))
        c0["op"] = "gen_primitive"
        c0["surprise"] = "field"
        assertTrue(problemFor("fields"))
        c0.remove("surprise")
        c0.remove("count")
        assertTrue(problemFor("fields"))
    }

    @Test
    fun nonObjectCasesAndNonStringIdsAreProblemsNotCrashes() {
        val cases = copyAll(loadCases())
        cases[0] = "not an object"
        assertTrue(KotlinCorpusRunner.evaluate(cases).problems.any { "case is not an object" in it })
        cases[0] = linkedMapOf<String, Any?>("id" to 7, "op" to "x")
        assertTrue(KotlinCorpusRunner.evaluate(cases).problems.any { "missing a string id" in it })
    }

    @Test
    fun malformedCaseValuesAreProblemsNotCrashes() {
        fun run(mutate: (MutableMap<String, Any?>) -> Unit, idPart: String): List<String> {
            val cases = copyAll(loadCases())
            @Suppress("UNCHECKED_CAST")
            val victim = cases.first { ((it as Map<*, *>)["id"] as String).contains(idPart) } as MutableMap<String, Any?>
            mutate(victim)
            return KotlinCorpusRunner.evaluate(cases).problems
        }
        val intNode = { v: String -> linkedMapOf<String, Any?>("kind" to "int", "value" to v) }
        assertTrue(run({ it["type"] = "u7" }, "u8_seed0").any { "unknown primitive type" in it })
        assertTrue(run({ it["type"] = true }, "u8_seed0").any { "type must be a string" in it })
        assertTrue(run({ it["count"] = intNode("9") }, "u8_seed0").any { "exactly count" in it })
        assertTrue(run({ it["count"] = "x" }, "u8_seed0").any { "count must be an int" in it })
        assertTrue(run({ it["count"] = intNode("99999") }, "u8_seed0").any { "outside" in it })
        assertTrue(run({ it["property"] = "no_such" }, "54321_le_100").any { "unknown property" in it })
        assertTrue(run({ it["property"] = true }, "54321_le_100").any { "unknown property" in it })
        assertTrue(run({ it["property"] = "no_such" }, "consecutive_duplicates").any { "unknown property" in it })
        assertTrue(run({ it["initial"] = "x" }, "consecutive_duplicates").any { "initial must be a list" in it })
        assertTrue(run({ it["property"] = "no_such" }, "len_lt_5").any { "unknown property" in it })
        assertTrue(run({ it["initial"] = "x" }, "len_lt_5").any { "initial must be bytes" in it })
        assertTrue(run({ it["property"] = "no_such" }, "seed42_le_100").any { "unknown property" in it })
        assertTrue(run({ it["max_retries"] = intNode("0") }, "impossible").any { "at least 1" in it })
        assertTrue(run({ it["text"] = true }, "bad_header").any { "text must be a string" in it })
        // A non-falsifying initial value is an engine error reported as a problem.
        assertTrue(run({ it["initial"] = intNode("5") }, "54321_le_100").any { "could not be evaluated" in it })
    }

    @Test
    fun coverageGatesNeedEveryOpAndEveryFailureId() {
        val only =
            loadCases().filter {
                val m = it as Map<*, *>
                m["op"] == "failure_id_valid" && m["input"] != "unregistered-id" && m["input"] != "property-panic"
            }
        val problems = KotlinCorpusRunner.evaluate(only, -1).problems
        assertTrue(problems.any { "operations never exercised" in it })
        assertTrue(problems.any { "registered failure ids missing" in it })
        assertTrue(problems.any { "unregistered-id rejection" in it })
    }

    // ---- Kotlin-layer sabotage -------------------------------------------------

    /** The seeds shift: gen_* and replay cases must disagree, and each says which case. */
    @Test
    fun aWrapperThatShiftsTheSeedIsCaught() {
        Seams.seedOffset = 1
        val problems = KotlinCorpusRunner.evaluate(loadCases()).problems
        assertTrue(problems.any { it.startsWith("pbt/gen_") }, problems.toString())
        assertTrue(problems.any { it.startsWith("pbt/replay/") }, problems.toString())
    }

    /** One dropped shrink step in the wrapper: every shrink_* case must notice. */
    @Test
    fun aWrapperThatDropsAShrinkStepIsCaught() {
        Seams.shrinkStepDelta = -1
        val problems = KotlinCorpusRunner.evaluate(loadCases()).problems
        val shrinkCases = loadCases().map { (it as Map<*, *>)["id"] as String }.filter { it.startsWith("pbt/shrink_") }
        assertEquals(7, shrinkCases.size)
        for (id in shrinkCases) assertTrue(problems.any { it.startsWith("$id:") && "step" in it }, "$id not caught: $problems")
    }

    /** Swapped bounds in the wrapper: the gen_bounded cases fail. */
    @Test
    fun aWrapperThatSwapsBoundsIsCaught() {
        Seams.swapBounds = true
        val problems = KotlinCorpusRunner.evaluate(loadCases()).problems
        assertTrue(problems.any { it.startsWith("pbt/gen_bounded/small_seed7:") }, problems.toString())
        assertTrue(problems.any { it.startsWith("pbt/gen_bounded/large_seed99:") }, problems.toString())
    }

    @Test
    fun seamsResetToProduction() {
        Seams.seedOffset = 5
        Seams.shrinkStepDelta = 3
        Seams.swapBounds = true
        Seams.reset()
        assertEquals(emptyList(), KotlinCorpusRunner.evaluate(loadCases()).problems)
    }
}
