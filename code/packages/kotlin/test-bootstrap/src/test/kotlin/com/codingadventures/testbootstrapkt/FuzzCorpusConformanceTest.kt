package com.codingadventures.testbootstrapkt

import com.codingadventures.testbootstrap.corpusjson.StrictJson
import java.math.BigInteger
import java.nio.file.Files
import java.nio.file.Path
import kotlin.test.AfterTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * Replays the whole committed fuzz-v1 corpus through the KOTLIN API, with its
 * negative controls and the Kotlin-layer sabotage controls (VER06g sections 3, 4).
 * The fixture is read from its committed path, never a copy.
 */
class FuzzCorpusConformanceTest {
    private fun fixturePath(): Path {
        var dir: Path? = Path.of("").toAbsolutePath()
        while (dir != null) {
            val candidate = dir.resolve("code/specs/fixtures/fuzz-v1/cases.json")
            if (Files.isRegularFile(candidate)) return candidate
            dir = dir.parent
        }
        throw AssertionError("could not find code/specs/fixtures/fuzz-v1/cases.json above the working directory")
    }

    private fun loadCases(): List<Any?> = KotlinFuzzCorpusRunner.parse(Files.readAllBytes(fixturePath()))

    @AfterTest
    fun resetSeams() = Seams.reset()

    // ---- helpers ---------------------------------------------------------------

    @Suppress("UNCHECKED_CAST")
    private fun deepCopy(node: Any?): Any? =
        when (node) {
            is Map<*, *> -> LinkedHashMap<String, Any?>().also { out -> (node as Map<String, Any?>).forEach { (k, v) -> out[k] = deepCopy(v) } }
            is List<*> -> node.map { deepCopy(it) }.toMutableList<Any?>()
            else -> node
        }

    private fun copyAll(cases: List<Any?>): MutableList<Any?> = cases.map { deepCopy(it) }.toMutableList()

    private fun intNode(v: String) = linkedMapOf<String, Any?>("kind" to "int", "value" to v)

    private fun bytesNode(hex: String) = linkedMapOf<String, Any?>("kind" to "bytes", "hex" to hex)

    @Suppress("UNCHECKED_CAST")
    private fun firstOf(cases: List<Any?>, op: String): MutableMap<String, Any?> =
        deepCopy(cases.first { (it as Map<*, *>)["op"] == op }) as MutableMap<String, Any?>

    private fun swapped(cases: List<Any?>, replacement: Map<String, Any?>): List<Any?> =
        cases.map { if ((it as Map<*, *>)["id"] == replacement["id"]) replacement else deepCopy(it) }

    private fun problems(cases: List<Any?>) = KotlinFuzzCorpusRunner.evaluate(cases, -1).problems

    private fun anyContains(ps: List<String>, fragment: String) = ps.any { fragment in it }

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

    private fun corruptions(op: String): List<(MutableMap<String, Any?>) -> Unit> =
        when (op) {
            "mutate_bit_flip", "mutate_byte_flip", "mutate_arithmetic", "mutate_boundary", "mutate_block_delete",
            "mutate_block_insert", "mutate_block_duplicate", "mutate_splice", "mutate_sequence",
            -> listOf({ c -> c["output"] = corruptValue(c["output"]) })
            "corpus_admit" -> listOf({ c -> c["admitted"] = corruptValue(c["admitted"]) }, { c -> c["new_edges"] = corruptValue(c["new_edges"]) })
            "crash_dedup" -> listOf({ c -> c["is_duplicate"] = corruptValue(c["is_duplicate"]) })
            "failure_id_valid" -> listOf({ c -> c["valid"] = corruptValue(c["valid"]) })
            "private_reporting" ->
                listOf({ c -> c["tap_line"] = "not ok 1 - ct_compare [fuzz-crash:3a8f19b3]" }, { c -> c["contains_raw_payload"] = corruptValue(c["contains_raw_payload"]) })
            "corpus_reject" -> listOf({ c -> c["text"] = "tfv 1\n" })
            else -> throw AssertionError("no corruption defined for op $op")
        }

    // ---- the positive run --------------------------------------------------------

    @Test
    fun corpusConformsToAllTwentyFiveCasesThroughKotlin() {
        val report = KotlinFuzzCorpusRunner.evaluate(loadCases())
        assertEquals(emptyList(), report.problems)
        assertEquals(25, report.evaluated, "exactly 25 cases must be fully evaluated")
        assertEquals(25, KotlinFuzzCorpusRunner.CASE_COUNT)
    }

    @Test
    fun everyOpIsPresentInTheFixtureAndHasACorruption() {
        val ops = loadCases().map { (it as Map<*, *>)["op"] as String }.toSortedSet()
        assertEquals(14, ops.size, "ops: $ops")
        for (op in ops) assertTrue(corruptions(op).isNotEmpty())
    }

    // ---- negative controls ----------------------------------------------------------

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
                val report = KotlinFuzzCorpusRunner.evaluate(cases)
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
            val report = KotlinFuzzCorpusRunner.evaluate(pristine.toMutableList().also { it.removeAt(i) })
            assertTrue(report.problems.isNotEmpty(), "dropping case $i went unnoticed")
            assertTrue(report.problems[0].contains("expected 25 cases, got 24"))
        }
        assertTrue(KotlinFuzzCorpusRunner.evaluate(emptyList()).problems.isNotEmpty())
        assertTrue(KotlinFuzzCorpusRunner.evaluate(pristine.subList(0, 8)).problems.isNotEmpty())
    }

    @Test
    fun droppingEveryCaseOfAnOpIsAProblemEvenWithTheCountDisabled() {
        val pristine = loadCases()
        for (op in pristine.map { (it as Map<*, *>)["op"] as String }.toSet()) {
            val cases = pristine.filter { (it as Map<*, *>)["op"] != op }
            assertTrue(anyContains(problems(cases), "operations never exercised"), op)
        }
    }

    @Test
    fun aTruncatedDocumentFailsToParse() {
        val data = Files.readAllBytes(fixturePath())
        assertFailsWith<StrictJson.JsonException> { KotlinFuzzCorpusRunner.parse(data.copyOf(data.size / 2)) }
    }

    @Test
    fun duplicatedCaseIdsAreAProblem() {
        val cases = loadCases().toMutableList()
        cases[1] = deepCopy(cases[0])
        assertTrue("duplicate or missing case id" in KotlinFuzzCorpusRunner.evaluate(cases).problems)
    }

    private fun problemFor(cases: List<Any?>, id: String, fragment: String) =
        KotlinFuzzCorpusRunner.evaluate(cases).problems.any { it.startsWith("$id:") && fragment in it }

    @Test
    fun anUnknownOpAMissingAndAnExtraFieldAreProblems() {
        val cases = copyAll(loadCases())
        @Suppress("UNCHECKED_CAST")
        val first = cases[0] as MutableMap<String, Any?>
        val id = first["id"] as String
        first["op"] = "gen_everything"
        assertTrue(problemFor(cases, id, "unknown op"))
        first["op"] = "mutate_bit_flip"
        first["surprise"] = "field"
        assertTrue(problemFor(cases, id, "fields"))
        first.remove("surprise")
        first.remove("count")
        assertTrue(problemFor(cases, id, "fields"))
    }

    @Test
    fun aNonObjectCaseAndANonStringIdAreProblemsNotCrashes() {
        val cases = loadCases().toMutableList()
        cases[0] = "not an object"
        assertTrue(anyContains(KotlinFuzzCorpusRunner.evaluate(cases).problems, "case is not an object"))
        cases[0] = linkedMapOf<String, Any?>("id" to 7, "op" to "x")
        assertTrue(anyContains(KotlinFuzzCorpusRunner.evaluate(cases).problems, "missing a string id"))
    }

    @Test
    fun nonCanonicalAndWronglyTypedFieldsAreProblemsNamingTheCase() {
        val pristine = loadCases()
        val bad =
            listOf(
                Triple("mutate_bit_flip", "bit_offset", intNode("007")),
                Triple("mutate_bit_flip", "bit_offset", intNode("+1")),
                Triple("mutate_bit_flip", "input", bytesNode("0G")),
                Triple("mutate_bit_flip", "input", bytesNode("ABCD")),
                Triple("mutate_bit_flip", "input", bytesNode("abc")),
                Triple("mutate_bit_flip", "count", bytesNode("00")),
                Triple("mutate_bit_flip", "input", intNode("1")),
                Triple("mutate_arithmetic", "endian", "middle"),
                Triple("mutate_arithmetic", "endian", true),
                Triple("mutate_arithmetic", "width", "4"),
                Triple("mutate_sequence", "steps", intNode("10001")),
                Triple("mutate_sequence", "steps", intNode("-1")),
                Triple("mutate_sequence", "seed", "42"),
                Triple("mutate_splice", "donor", intNode("1")),
                Triple("corpus_admit", "trace_edges", intNode("1")),
                Triple("corpus_admit", "trace_edges", mutableListOf<Any?>("x")),
                Triple("corpus_admit", "trace_edges", mutableListOf<Any?>(intNode("-1"))),
                Triple("corpus_admit", "trace_edges", mutableListOf<Any?>(intNode("18446744073709551616"))),
                Triple("corpus_admit", "admitted", intNode("1")),
                Triple("crash_dedup", "crash_a_frames", "x"),
                Triple("crash_dedup", "crash_a_frames", mutableListOf<Any?>(intNode("1"))),
                Triple("crash_dedup", "crash_a_kind", intNode("1")),
                Triple("crash_dedup", "is_duplicate", "true"),
                Triple("private_reporting", "raw_payload", "x"),
                Triple("private_reporting", "contains_raw_payload", "x"),
                Triple("private_reporting", "sig_hash", "NOTHEX00"),
                Triple("private_reporting", "failure_id", "fuzz-bogus"),
                Triple("private_reporting", "tap_line", intNode("1")),
                Triple("corpus_reject", "text", intNode("1")),
            )
        for ((op, field, replacement) in bad) {
            val victim = firstOf(pristine, op)
            val id = victim["id"] as String
            victim[field] = replacement
            val ps = problems(swapped(pristine, victim))
            assertTrue(ps.any { it.startsWith("$id:") }, "$op.$field=$replacement: $ps")
        }
    }

    @Test
    fun anUnsupportedWidthLeavesTheInputUnchangedNotAProblem() {
        val cases = copyAll(loadCases())
        val c = firstOf(cases, "mutate_arithmetic")
        for (w in listOf("3", "99999")) {
            c["width"] = intNode(w)
            c["output"] = c["input"]
            assertEquals(emptyList(), problems(swapped(cases, c)))
        }
    }

    @Test
    fun coverageGatesNeedAnAcceptedAndARejectedFailureId() {
        fun only(input: String) = loadCases().filter { (it as Map<*, *>)["input"] == input }
        assertTrue(anyContains(problems(only("fuzz-crash")), "unregistered-id rejection"))
        assertTrue(anyContains(problems(only("fuzz-arbitrary-error")), "registered-id acceptance"))
    }

    @Test
    fun theSequenceWorkBudgetIsAggregateAndFixtureSizesAreProblemsNotHangs() {
        val all = loadCases()
        val c = firstOf(all, "mutate_sequence")
        c["steps"] = intNode("1000000000")
        assertTrue(anyContains(problems(swapped(all, c)), "outside [0, 10000]"))
        // Each sequence case is charged input + steps * maxLen up front, so a corpus of them
        // exhausts the 2^24 aggregate budget quickly and every later case fails without running.
        val many = (0 until 50).map { i ->
            firstOf(all, "mutate_sequence").also {
                it["id"] = "seq/$i"
                it["steps"] = intNode("1000")
            }
        }
        val ps = problems(many)
        val charge = 8L + 1000L * (8 + KotlinFuzzCorpusRunner.SEQUENCE_GROWTH)
        val ran = (KotlinFuzzCorpusRunner.MAX_SEQUENCE_WORK / charge).toInt()
        assertEquals(50 - ran, ps.count { "work budget" in it }, ps.toString())
        assertTrue(ps.none { it.startsWith("seq/0:") && "work budget" in it }, "the first case fits the budget")
        val tooMany = (0 until 10_001).map { i -> firstOf(all, "failure_id_valid").also { it["id"] = "f/$i" } }
        assertTrue(anyContains(problems(tooMany), "exceeds the cap"))
    }

    @Test
    fun problemTextIsClippedAndAnOutputThatIsNotBytesIsAProblem() {
        val all = loadCases()
        val c = firstOf(all, "mutate_bit_flip")
        c["id"] = "i".repeat(5000)
        c["output"] = bytesNode("ab".repeat(5000))
        for (p in problems(swapped(all, c))) assertTrue(p.length <= 403, "${p.length} chars")
        val d = firstOf(all, "mutate_bit_flip")
        d["output"] = intNode("1")
        assertTrue(anyContains(problems(swapped(all, d)), "output must be bytes"))
    }

    @Test
    fun aHandBuiltTooDeepValueIsAProblemNotAStackOverflow() {
        val all = loadCases()
        val c = firstOf(all, "mutate_bit_flip")
        var nested: Any = ArrayList<Any?>()
        repeat(100_000) { nested = arrayListOf<Any?>(nested) }
        c["count"] = nested
        assertTrue(anyContains(problems(swapped(all, c)), "could not be evaluated"))
    }

    @Test
    fun theEnvelopeIsParsedStrictly() {
        val bad = listOf("nope", "", "{\"schema_version\":2,\"cases\":[]}", "{\"schema_version\":1}", "{\"schema_version\":1,\"cases\":{}}", "[1]")
        for (text in bad) assertFailsWith<StrictJson.JsonException>(text) { KotlinFuzzCorpusRunner.parse(text.toByteArray()) }
        assertEquals(2, KotlinFuzzCorpusRunner.parse("{\"schema_version\":1,\"cases\":[1,\"a\"]}".toByteArray()).size)
    }

    // ---- Kotlin-layer sabotage ----------------------------------------------------------

    private fun assertCaught(name: String, vararg idFragments: String) {
        val ps = KotlinFuzzCorpusRunner.evaluate(loadCases()).problems
        assertTrue(ps.isNotEmpty(), "sabotage was not caught: $name")
        for (f in idFragments) assertTrue(ps.any { it.startsWith("fuzz/$f") }, "$name: no problem names fuzz/$f: $ps")
        Seams.reset()
        assertEquals(emptyList(), KotlinFuzzCorpusRunner.evaluate(loadCases()).problems, "seams must reset after $name")
    }

    @Test
    fun aWrapperThatShiftsMutatorOffsetsIsCaught() {
        Seams.fuzzOffsetDelta = 1
        assertCaught("offset+1", "mutate_bit_flip/", "mutate_byte_flip/", "mutate_block_delete/", "mutate_splice/")
    }

    @Test
    fun aWrapperThatSwapsByteOrderIsCaught() {
        Seams.swapEndian = true
        assertCaught("endian swapped", "mutate_arithmetic/u16_le_add35", "mutate_boundary/u32_signed_min_le")
    }

    @Test
    fun aWrapperThatShiftsTheSequenceSeedIsCaught() {
        Seams.seedOffset = 1
        assertCaught("sequence seed+1", "mutate_sequence/seed42_steps4")
    }

    @Test
    fun aWrapperThatInvertsDedupIsCaught() {
        Seams.invertDedup = true
        assertCaught("dedup inverted", "crash_dedup/same_signature", "crash_dedup/divergent_signature")
    }

    @Test
    fun aWrapperThatLeaksTheTargetIsCaught() {
        Seams.leakTapTarget = true
        assertCaught("tap leaks target", "private_reporting/opaque_token")
    }

    @Test
    fun aWrapperThatAdmitsEverythingIsCaught() {
        Seams.admitEverything = true
        assertCaught("admit everything", "corpus_admit/redundant_edge")
    }

    @Test
    fun aWrapperThatAcceptsAnyFailureIdIsCaught() {
        Seams.acceptAnyFailureId = true
        assertCaught("any failure id", "failure_id/unregistered")
    }

    @Test
    fun seamsResetToProduction() {
        Seams.fuzzOffsetDelta = 3
        Seams.swapEndian = true
        Seams.invertDedup = true
        Seams.leakTapTarget = true
        Seams.admitEverything = true
        Seams.acceptAnyFailureId = true
        Seams.reset()
        assertEquals(emptyList(), KotlinFuzzCorpusRunner.evaluate(loadCases()).problems)
    }

    /** Every registered fuzz id is accepted and an unregistered one is not (the engine owns the list). */
    @Test
    fun theKotlinFailureVocabularyMatchesTheEngine() {
        assertEquals(listOf("fuzz-crash", "fuzz-timeout", "fuzz-oom", "fuzz-leak", "fuzz-panic"), FuzzFailureId.ALL)
        assertTrue(FuzzFailureId.ALL.all { FuzzFailureId.isValid(it) })
        assertFalse(FuzzFailureId.isValid("fuzz-arbitrary-error"))
        assertFalse(FuzzFailureId.isValid(null))
    }
}
