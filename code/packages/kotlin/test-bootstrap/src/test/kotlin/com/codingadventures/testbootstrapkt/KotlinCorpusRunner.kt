package com.codingadventures.testbootstrapkt

import com.codingadventures.testbootstrap.corpusjson.Canon
import com.codingadventures.testbootstrap.corpusjson.Clip
import com.codingadventures.testbootstrap.corpusjson.Values
import com.codingadventures.testbootstrap.pbt.Corpus
import com.codingadventures.testbootstrap.tfv.Tfv
import java.math.BigInteger

/**
 * The pbt-v1 corpus runner for the KOTLIN surface (VER05f section 3).
 *
 * It shares only decoding with the Java package (`Corpus.parse`,
 * `Values.decode`, `Values.same`).  It never calls `Corpus.evaluate`: every op
 * is answered by calling the Kotlin API (`genU64`, `shrinkInt`, `checkProperty`,
 * ...), so a broken wrapper changes this runner's answers and the corpus fails.
 * Every problem about a case starts with that case's id.
 */
object KotlinCorpusRunner {
    const val CASE_COUNT = 25

    data class Report(val evaluated: Int, val problems: List<String>)

    private fun big(n: Int) = BigInteger.valueOf(n.toLong())

    // Properties named by the corpus.  true means "holds".
    private val intProps: Map<String, (BigInteger) -> Boolean> =
        mapOf(
            "int_le_100" to { x -> x <= big(100) },
            "int_in_range_neg10_50" to { x -> x >= big(-10) && x <= big(50) },
            "int_is_even" to { x -> !x.testBit(0) },
            "int_abs_le_10" to { x -> x.abs() <= big(10) },
            "refinement_x_le_50" to { x -> x <= big(50) },
        )
    private val listProps: Map<String, (List<BigInteger>) -> Boolean> =
        mapOf(
            "list_no_consecutive_duplicates" to { l -> l.zipWithNext().none { (a, b) -> a == b } },
            "list_sum_le_20" to { l -> l.fold(BigInteger.ZERO) { a, b -> a + b } <= big(20) },
        )
    private val bytesProps: Map<String, (ByteArray) -> Boolean> =
        mapOf(
            "bytes_len_lt_5" to { b -> b.size < 5 },
            "bytes_no_zero_byte" to { b -> b.none { it == 0.toByte() } },
        )

    /** The closed field set of each op, besides id and op. */
    private val shapes: Map<String, List<String>> =
        mapOf(
            "gen_primitive" to listOf("type", "seed", "count", "outputs"),
            "gen_bounded" to listOf("min", "max", "seed", "count", "outputs"),
            "gen_bytes" to listOf("min_len", "max_len", "seed", "count", "outputs"),
            "shrink_int" to listOf("initial", "target", "min", "max", "property", "expected_minimal", "expected_steps"),
            "shrink_refinement" to listOf("initial", "target", "min", "max", "property", "expected_minimal", "expected_steps"),
            "shrink_list" to listOf("initial", "elem_target", "property", "expected_minimal", "expected_steps"),
            "shrink_bytes" to listOf("initial", "property", "expected_minimal", "expected_steps"),
            "replay_consistency" to listOf("seed", "property", "initial_fail", "expected_minimal"),
            "filter_exhausted" to listOf("max_retries", "verdict", "failure_id"),
            "failure_id_valid" to listOf("input", "valid"),
            "corpus_reject" to listOf("text"),
        )

    private class CaseProblem(message: String) : RuntimeException(message)

    private fun same(a: Any?, b: Any?) = Values.same(a, b)

    private fun show(v: Any?) = Values.show(v)

    private fun intOf(v: Any?, what: String): BigInteger = v as? BigInteger ?: throw CaseProblem("$what must be an int")

    private fun sizeOf(v: Any?, what: String): Int {
        val b = intOf(v, what)
        if (b.signum() < 0 || b > big(Values.MAX_SIZE)) throw CaseProblem("$what ${show(b)} outside [0, ${Values.MAX_SIZE}]")
        return b.toInt()
    }

    /** A corpus seed reduced modulo 2^64, as the reference masks it. */
    private fun seedOf(v: Any?): ULong = intOf(v, "seed").toLong().toULong()

    /** An unsigned value as the decimal-int the corpus holds, built from its text, not a library conversion. */
    private fun asInt(text: String) = BigInteger(text)

    fun evaluate(casesIn: List<Any?>, expected: Int = CASE_COUNT): Report {
        val problems = mutableListOf<String>()
        var evaluated = 0
        var cases = casesIn
        if (expected >= 0 && cases.size != expected) problems += "pbt-v1: expected $expected cases, got ${cases.size}"

        val seenIds = HashSet<String>()
        for (raw in cases) {
            val id = ((raw as? Map<*, *>)?.get("id") as? String)
            if (id.isNullOrEmpty() || !seenIds.add(id)) {
                problems += "duplicate or missing case id"
                break
            }
        }

        val opsSeen = HashSet<String>()
        val registeredSeen = HashSet<String>()
        var rejectedIdSeen = false
        for (raw in cases) {
            val c = raw as? Map<*, *>
            if (c == null) {
                problems += "case is not an object: ${show(raw)}"
                continue
            }
            val rawId = c["id"] as? String
            if (rawId.isNullOrEmpty()) {
                problems += "case missing a string id: ${show(c)}"
                continue
            }
            val cid = Clip.clip(rawId, 100)
            val op = c["op"] as? String ?: ""
            val fields = shapes[op]
            if (fields == null) {
                problems += "$cid: unknown op ${show(c["op"])}"
                continue
            }
            if (c.size != fields.size + 2 || !c.containsKey("id") || !c.containsKey("op") || fields.any { !c.containsKey(it) }) {
                problems += "$cid: fields ${c.keys.map { it.toString() }.sorted()} != ${listOf("id", "op") + fields}"
                continue
            }
            try {
                val v = HashMap<String, Any?>()
                for ((k, value) in c) if (k != "id" && k != "op") v[k as String] = Values.decode(value)
                val found = check(cid, op, v)
                problems += found.problems
                if (op == "failure_id_valid") {
                    if (found.registered) registeredSeen += found.candidate else rejectedIdSeen = true
                }
                opsSeen += op
                evaluated++
            } catch (e: CaseProblem) {
                problems += "$cid: $op could not be evaluated: ${Clip.clip(e.message ?: "", 200)}"
            } catch (e: RuntimeException) {
                problems += "$cid: $op could not be evaluated: ${Clip.clip(e.toString(), 200)}"
            }
        }

        val missingOps = shapes.keys.toSortedSet().apply { removeAll(opsSeen) }
        if (missingOps.isNotEmpty()) problems += "pbt-v1: operations never exercised: $missingOps"
        val missingIds = FailureId.ALL.filter { it !in registeredSeen }
        if (missingIds.isNotEmpty()) problems += "pbt-v1: registered failure ids missing: $missingIds"
        if (!rejectedIdSeen) problems += "pbt-v1: failure_id_valid needs an unregistered-id rejection"
        return Report(evaluated, problems.map { Clip.clip(it, Values.MAX_PROBLEM_LEN) })
    }

    private class Found(val problems: List<String>, val candidate: String = "", val registered: Boolean = false)

    private fun check(cid: String, op: String, v: Map<String, Any?>): Found =
        when (op) {
            "gen_primitive", "gen_bounded", "gen_bytes" -> Found(checkGen(cid, op, v))
            "shrink_int", "shrink_refinement" -> Found(checkShrinkInt(cid, op, v))
            "shrink_list" -> Found(checkShrinkList(cid, v))
            "shrink_bytes" -> Found(checkShrinkBytes(cid, v))
            "replay_consistency" -> Found(checkReplay(cid, v))
            "filter_exhausted" -> Found(checkFilterExhausted(cid, v))
            "failure_id_valid" -> {
                val input = v["input"] as? String
                val got = FailureId.isValid(input)
                val found = if (same(got, v["valid"])) emptyList() else listOf("$cid: failure id membership disagrees for ${show(v["input"])}")
                Found(found, input ?: "", got)
            }
            else -> { // corpus_reject
                val text = v["text"] as? String ?: throw CaseProblem("text must be a string")
                Found(if (Tfv.isValid(text)) listOf("$cid: corpus_reject text was accepted by the strict parser") else emptyList())
            }
        }

    private fun checkGen(cid: String, op: String, v: Map<String, Any?>): List<String> {
        val seed = seedOf(v["seed"])
        val count = sizeOf(v["count"], "count")
        val want = v["outputs"] as? List<*>
        if (want == null || want.size != count) throw CaseProblem("outputs must be a list of exactly count ($count) values")
        val got: List<Any> =
            when (op) {
                "gen_primitive" ->
                    when (val type = v["type"] as? String ?: throw CaseProblem("type must be a string")) {
                        "bool" -> genBool(seed, count)
                        "u8" -> genU8(seed, count).map { asInt(it.toString()) }
                        "u64" -> genU64(seed, count).map { asInt(it.toString()) }
                        "i128" -> genI128(seed, count)
                        else -> throw CaseProblem("unknown primitive type \"$type\"")
                    }
                "gen_bounded" -> genBounded(seed, intOf(v["min"], "min"), intOf(v["max"], "max"), count)
                else -> genBytes(seed, sizeOf(v["min_len"], "min_len"), sizeOf(v["max_len"], "max_len"), count)
            }
        return if (same(got, v["outputs"])) emptyList() else listOf("$cid: $op outputs mismatch")
    }

    private fun lookup(v: Map<String, Any?>, what: String) = v["property"] as? String ?: throw CaseProblem("unknown property ${show(v["property"])} for $what")

    private fun checkShrinkInt(cid: String, op: String, v: Map<String, Any?>): List<String> {
        val name = lookup(v, op)
        val holds = intProps[name] ?: throw CaseProblem("unknown property ${show(name)} for $op")
        val lo = intOf(v["min"], "min")
        val hi = intOf(v["max"], "max")
        val r = shrinkInt(intOf(v["initial"], "initial"), intOf(v["target"], "target"), lo, hi, 1000, holds)
        val problems = mutableListOf<String>()
        if (!same(r.value, v["expected_minimal"])) problems += "$cid: $op minimal mismatch: got ${show(r.value)}, corpus ${show(v["expected_minimal"])}"
        if (!same(big(r.steps), v["expected_steps"])) problems += "$cid: $op step count mismatch: got ${r.steps}, corpus ${show(v["expected_steps"])}"
        if (holds(r.value)) problems += "$cid: $op minimal ${show(r.value)} does not falsify the property"
        if (r.value < lo || r.value > hi) problems += "$cid: $op minimal ${show(r.value)} left [${show(lo)}, ${show(hi)}]"
        return problems
    }

    private fun checkShrinkList(cid: String, v: Map<String, Any?>): List<String> {
        val name = lookup(v, "shrink_list")
        val holds = listProps[name] ?: throw CaseProblem("unknown property ${show(name)} for shrink_list")
        val items = v["initial"] as? List<*> ?: throw CaseProblem("initial must be a list of ints")
        val initial = items.map { intOf(it, "element") }
        val r = shrinkList(initial, intOf(v["elem_target"], "elem_target"), 1000, holds)
        val problems = mutableListOf<String>()
        if (!same(r.value, v["expected_minimal"])) problems += "$cid: shrink_list minimal mismatch: got ${Canon.render(r.value)}, corpus ${Clip.clip(Canon.render(v["expected_minimal"]), 200)}"
        if (!same(big(r.steps), v["expected_steps"])) problems += "$cid: shrink_list steps mismatch: got ${r.steps}, corpus ${show(v["expected_steps"])}"
        if (holds(r.value)) problems += "$cid: shrink_list minimal ${show(r.value)} does not falsify the property"
        return problems
    }

    private fun checkShrinkBytes(cid: String, v: Map<String, Any?>): List<String> {
        val name = lookup(v, "shrink_bytes")
        val holds = bytesProps[name] ?: throw CaseProblem("unknown property ${show(name)} for shrink_bytes")
        val initial = v["initial"] as? ByteArray ?: throw CaseProblem("initial must be bytes")
        val r = shrinkBytes(initial, 1000, holds)
        val problems = mutableListOf<String>()
        if (!same(r.value, v["expected_minimal"])) problems += "$cid: shrink_bytes minimal mismatch: got ${show(r.value)}, corpus ${show(v["expected_minimal"])}"
        if (!same(big(r.steps), v["expected_steps"])) problems += "$cid: shrink_bytes steps mismatch: got ${r.steps}, corpus ${show(v["expected_steps"])}"
        if (holds(r.value)) problems += "$cid: shrink_bytes minimal ${show(r.value)} does not falsify the property"
        return problems
    }

    private fun checkReplay(cid: String, v: Map<String, Any?>): List<String> {
        val seed = seedOf(v["seed"])
        val name = lookup(v, "replay_consistency")
        val holds = intProps[name] ?: throw CaseProblem("unknown property ${show(name)} for replay_consistency")
        val r = simulateReplay(seed, holds)
        val problems = mutableListOf<String>()
        if (!same(r.initialFail, v["initial_fail"])) problems += "$cid: replay initial_fail mismatch: got ${show(r.initialFail)}, corpus ${show(v["initial_fail"])}"
        if (!same(r.minimal, v["expected_minimal"])) problems += "$cid: replay minimal mismatch: got ${show(r.minimal)}, corpus ${show(v["expected_minimal"])}"
        return problems
    }

    private fun checkFilterExhausted(cid: String, v: Map<String, Any?>): List<String> {
        val maxRetries = sizeOf(v["max_retries"], "max_retries")
        if (maxRetries < 1) throw CaseProblem("max_retries must be at least 1")
        var calls = 0
        val gen = integer(BigInteger.ZERO, BigInteger.TEN).filter(maxRetries) { calls++; false }
        val verdict = checkProperty(0uL, gen, Options(1, 1000, CiMode.ON)) { true }
        val problems = mutableListOf<String>()
        val failure = verdict as? Verdict.Failed
        if (!same("fail", v["verdict"]) || failure == null) problems += "$cid: an unsatisfiable filter must be a fail verdict"
        if (!same(failure?.failureId, v["failure_id"])) problems += "$cid: failure id \"${failure?.failureId}\", corpus ${show(v["failure_id"])}"
        if (calls != maxRetries) problems += "$cid: filter sampled $calls times, expected $maxRetries"
        return problems
    }

    /** Decodes the committed document envelope; shared with the Java package by design. */
    fun parse(data: ByteArray): List<Any?> = Corpus.parse(data)
}
