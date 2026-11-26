package com.codingadventures.testbootstrapkt

import com.codingadventures.testbootstrap.corpusjson.Clip
import com.codingadventures.testbootstrap.corpusjson.Values
import com.codingadventures.testbootstrap.fuzz.Corpus as JavaFuzzCorpus
import java.math.BigInteger

/**
 * The fuzz-v1 corpus runner for the KOTLIN surface (VER06g section 3).
 *
 * It shares only decoding with the Java package (`StrictJson`, `Values.decode`,
 * `Values.same`).  It never calls `fuzz.Corpus.evaluate`: every op is answered by
 * a Kotlin wrapper (`bitFlip`, `corpusAdmit`, `isDuplicateCrash`, ...), so a
 * broken wrapper changes this runner's answers and the corpus fails.  Every
 * problem about a case starts with that case's id.
 */
object KotlinFuzzCorpusRunner {
    const val CASE_COUNT = 25

    /**
     * Aggregate `mutate_sequence` work budget of one corpus, in byte copies (VER06g section 6,
     * item 5).  Each case is charged `input.size + steps * maxLen` BEFORE it runs, which
     * bounds the real work (the buffer never exceeds `maxLen`), so a failing or refused
     * case costs nothing extra and the total can never pass this figure.
     */
    const val MAX_SEQUENCE_WORK: Long = 1L shl 24

    /** Growth allowance above the input length: `maxLen = input.size + SEQUENCE_GROWTH`. */
    const val SEQUENCE_GROWTH = 4096

    private const val MAX_CASES = 10_000

    data class Report(val evaluated: Int, val problems: List<String>)

    private val shapes: Map<String, List<String>> =
        mapOf(
            "mutate_bit_flip" to listOf("input", "bit_offset", "count", "output"),
            "mutate_byte_flip" to listOf("input", "byte_offset", "count", "output"),
            "mutate_arithmetic" to listOf("input", "byte_offset", "delta", "width", "endian", "output"),
            "mutate_boundary" to listOf("input", "byte_offset", "width", "val", "endian", "output"),
            "mutate_block_delete" to listOf("input", "offset", "length", "output"),
            "mutate_block_insert" to listOf("input", "offset", "insert_bytes", "output"),
            "mutate_block_duplicate" to listOf("input", "src_offset", "length", "dst_offset", "output"),
            "mutate_splice" to listOf("target", "donor", "target_split", "donor_split", "output"),
            "mutate_sequence" to listOf("input", "seed", "steps", "output"),
            "corpus_admit" to listOf("existing_edges", "trace_edges", "admitted", "new_edges"),
            "crash_dedup" to listOf("crash_a_kind", "crash_a_frames", "crash_b_kind", "crash_b_frames", "is_duplicate"),
            "failure_id_valid" to listOf("input", "valid"),
            "private_reporting" to listOf("target", "failure_id", "raw_payload", "sig_hash", "tap_line", "contains_raw_payload"),
            "corpus_reject" to listOf("text"),
        )

    private class CaseProblem(message: String) : RuntimeException(message)

    private fun same(a: Any?, b: Any?) = Values.same(a, b)

    private fun show(v: Any?) = Values.show(v)

    private fun intOf(v: Any?, what: String): BigInteger = v as? BigInteger ?: throw CaseProblem("$what must be an int")

    private fun bytesOf(v: Any?, what: String): ByteArray = v as? ByteArray ?: throw CaseProblem("$what must be bytes")

    private fun textOf(v: Any?, what: String): String = v as? String ?: throw CaseProblem("$what must be a string")

    private fun boolOf(v: Any?, what: String): Boolean = v as? Boolean ?: throw CaseProblem("$what must be a bool")

    private fun sizeOf(v: Any?, what: String): Int {
        val b = intOf(v, what)
        if (b.signum() < 0 || b > BigInteger.valueOf(Values.MAX_SIZE.toLong())) throw CaseProblem("$what ${show(b)} outside [0, ${Values.MAX_SIZE}]")
        return b.toInt()
    }

    /** A width outside [0, 64] becomes -1, which every mutator treats as "not 1, 2, 4, 8". */
    private fun widthOf(v: Any?): Int {
        val b = intOf(v, "width")
        return if (b.signum() < 0 || b > BigInteger.valueOf(64)) -1 else b.toInt()
    }

    private fun endianOf(v: Any?): Endian =
        when (val s = textOf(v, "endian")) {
            "little" -> Endian.LITTLE
            "big" -> Endian.BIG
            else -> throw CaseProblem("endian must be 'little' or 'big', got ${show(s)}")
        }

    private fun textsOf(v: Any?, what: String): List<String> {
        val list = v as? List<*> ?: throw CaseProblem("$what must be a list of strings")
        return list.map { it as? String ?: throw CaseProblem("$what must be a list of strings") }
    }

    private val two64: BigInteger = BigInteger.ONE.shiftLeft(64)

    private fun edgesOf(v: Any?, what: String): List<ULong> {
        val list = v as? List<*> ?: throw CaseProblem("$what must be a list of ints")
        return list.map {
            val b = it as? BigInteger ?: throw CaseProblem("$what must be a list of ints")
            if (b.signum() < 0 || b >= two64) throw CaseProblem("$what element ${show(b)} outside [0, 2^64)")
            b.toLong().toULong()
        }
    }

    private fun hex(b: ByteArray): String = b.joinToString("") { "%02x".format(it) }

    private fun clampSplit(split: BigInteger, n: Int): Int = split.max(BigInteger.ZERO).min(BigInteger.valueOf(n.toLong())).toInt()

    fun evaluate(casesIn: List<Any?>, expected: Int = CASE_COUNT): Report {
        val problems = mutableListOf<String>()
        var evaluated = 0
        var cases = casesIn
        if (expected >= 0 && cases.size != expected) problems += "fuzz-v1: expected $expected cases, got ${cases.size}"
        if (cases.size > MAX_CASES) {
            problems += "fuzz-v1: ${cases.size} cases exceeds the cap of $MAX_CASES"
            cases = cases.subList(0, MAX_CASES)
        }

        val seenIds = HashSet<String>()
        for (raw in cases) {
            val id = (raw as? Map<*, *>)?.get("id") as? String
            if (id.isNullOrEmpty() || !seenIds.add(id)) {
                problems += "duplicate or missing case id"
                break
            }
        }

        val opsSeen = HashSet<String>()
        var acceptedIdSeen = false
        var rejectedIdSeen = false
        val workLeft = longArrayOf(MAX_SEQUENCE_WORK)
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
                val found = check(cid, op, v, workLeft)
                problems += found.problems
                if (op == "failure_id_valid") {
                    if (found.accepted) acceptedIdSeen = true else rejectedIdSeen = true
                }
                opsSeen += op
                evaluated++
            } catch (e: CaseProblem) {
                problems += "$cid: $op could not be evaluated: ${Clip.clip(e.message ?: "", 200)}"
            } catch (e: RuntimeException) {
                problems += "$cid: $op could not be evaluated: ${Clip.clip(e.message ?: e.javaClass.simpleName, 200)}"
            } catch (e: StackOverflowError) {
                problems += "$cid: $op could not be evaluated: StackOverflowError"
            }
        }

        val missingOps = shapes.keys.toSortedSet().apply { removeAll(opsSeen) }
        if (missingOps.isNotEmpty()) problems += "fuzz-v1: operations never exercised: $missingOps"
        if (!acceptedIdSeen) problems += "fuzz-v1: failure_id_valid needs a registered-id acceptance"
        if (!rejectedIdSeen) problems += "fuzz-v1: failure_id_valid needs an unregistered-id rejection"
        val unregistered = FuzzFailureId.ALL.filter { !FuzzFailureId.isValid(it) }
        if (unregistered.isNotEmpty()) problems += "fuzz-v1: engine rejects registered ids $unregistered"
        return Report(evaluated, problems.map { Clip.clip(it, Values.MAX_PROBLEM_LEN) })
    }

    private class Found(val problems: List<String>, val accepted: Boolean = false)

    private fun check(cid: String, op: String, v: Map<String, Any?>, work: LongArray): Found =
        when (op) {
            "corpus_admit" -> Found(checkAdmit(cid, v))
            "crash_dedup" -> Found(checkDedup(cid, v))
            "failure_id_valid" -> {
                val input = v["input"] as? String
                val got = input != null && FuzzFailureId.isValid(input)
                Found(if (same(got, v["valid"])) emptyList() else listOf("$cid: failure id membership disagrees for ${show(v["input"])}"), got)
            }
            "private_reporting" -> Found(checkPrivate(cid, v))
            "corpus_reject" ->
                Found(if (tfvIsValid(textOf(v["text"], "text"))) listOf("$cid: corpus_reject text was accepted by the strict parser") else emptyList())
            else -> Found(checkMutator(cid, op, v, work))
        }

    private fun compare(cid: String, op: String, got: ByteArray, v: Map<String, Any?>): MutableList<String> {
        val problems = mutableListOf<String>()
        val want = v["output"] as? ByteArray
        if (want == null) {
            problems += "$cid: $op output must be bytes"
        } else if (!got.contentEquals(want)) {
            problems += "$cid: $op expected ${Clip.clip(hex(want), 200)}, got ${Clip.clip(hex(got), 200)}"
        }
        return problems
    }

    private fun checkMutator(cid: String, op: String, v: Map<String, Any?>, work: LongArray): List<String> {
        val input = bytesOf(v[if (op == "mutate_splice") "target" else "input"], if (op == "mutate_splice") "target" else "input")
        val got: ByteArray =
            when (op) {
                "mutate_bit_flip" -> input.bitFlip(intOf(v["bit_offset"], "bit_offset"), intOf(v["count"], "count"))
                "mutate_byte_flip" -> input.byteFlip(intOf(v["byte_offset"], "byte_offset"), intOf(v["count"], "count"))
                "mutate_arithmetic" ->
                    input.byteArithmetic(intOf(v["byte_offset"], "byte_offset"), intOf(v["delta"], "delta"), widthOf(v["width"]), endianOf(v["endian"]))
                "mutate_boundary" ->
                    input.boundaryInsert(intOf(v["byte_offset"], "byte_offset"), widthOf(v["width"]), intOf(v["val"], "val"), endianOf(v["endian"]))
                "mutate_block_delete" -> input.blockDelete(intOf(v["offset"], "offset"), intOf(v["length"], "length"))
                "mutate_block_insert" -> input.blockInsert(intOf(v["offset"], "offset"), bytesOf(v["insert_bytes"], "insert_bytes"))
                "mutate_block_duplicate" ->
                    input.blockDuplicate(intOf(v["src_offset"], "src_offset"), intOf(v["length"], "length"), intOf(v["dst_offset"], "dst_offset"))
                "mutate_splice" -> {
                    val donor = bytesOf(v["donor"], "donor")
                    val ts = intOf(v["target_split"], "target_split")
                    val ds = intOf(v["donor_split"], "donor_split")
                    val out = input.splice(donor, ts, ds)
                    val problems = compare(cid, op, out, v)
                    val want = clampSplit(ts, input.size).toLong() + donor.size - clampSplit(ds, donor.size)
                    if (out.size.toLong() != want) problems += "$cid: splice length is not prefix + suffix"
                    return problems
                }
                else -> { // mutate_sequence
                    val seed = intOf(v["seed"], "seed").toLong().toULong()
                    val steps = sizeOf(v["steps"], "steps")
                    val maxLen = input.size + SEQUENCE_GROWTH
                    val charge = input.size.toLong() + steps.toLong() * maxLen
                    if (charge > work[0]) {
                        work[0] = 0 // the aggregate is breached: nothing is left for any later case
                        throw CaseProblem("mutate_sequence work budget of $MAX_SEQUENCE_WORK byte-copies exhausted")
                    }
                    work[0] -= charge
                    input.mutateSequence(seed, steps, maxLen)
                }
            }
        val problems = compare(cid, op, got, v)
        if ((op == "mutate_bit_flip" || op == "mutate_byte_flip" || op == "mutate_arithmetic" || op == "mutate_boundary") && got.size != input.size) {
            problems += "$cid: $op changed the length of its input"
        }
        return problems
    }

    private fun checkAdmit(cid: String, v: Map<String, Any?>): List<String> {
        val existing = edgesOf(v["existing_edges"], "existing_edges")
        val trace = edgesOf(v["trace_edges"], "trace_edges")
        val admitted = boolOf(v["admitted"], "admitted")
        val newEdges = intOf(v["new_edges"], "new_edges")
        val got = corpusAdmit(existing, trace)
        val problems = mutableListOf<String>()
        if (got.admitted != admitted) problems += "$cid: corpus_admit admitted ${got.admitted}, corpus $admitted"
        if (!same(BigInteger.valueOf(got.newEdges.toLong()), newEdges)) problems += "$cid: corpus_admit new_edges ${got.newEdges}, corpus ${show(newEdges)}"
        val have = existing.toSet()
        if (got.admitted != trace.any { it !in have }) problems += "$cid: corpus_admit disagrees with set difference"
        return problems
    }

    private fun checkDedup(cid: String, v: Map<String, Any?>): List<String> {
        val ka = textOf(v["crash_a_kind"], "crash_a_kind")
        val fa = textsOf(v["crash_a_frames"], "crash_a_frames")
        val kb = textOf(v["crash_b_kind"], "crash_b_kind")
        val fb = textsOf(v["crash_b_frames"], "crash_b_frames")
        val want = boolOf(v["is_duplicate"], "is_duplicate")
        val problems = mutableListOf<String>()
        val got = isDuplicateCrash(ka, fa, kb, fb)
        if (got != want) problems += "$cid: crash_dedup is_duplicate $got, corpus $want"
        // The dedup hashes: equal signatures exactly when the crashes are duplicates.
        val ha = crashSignature(ka, fa)
        val hb = crashSignature(kb, fb)
        val hexRe = Regex("[0-9a-f]{8}")
        if (!hexRe.matches(ha) || !hexRe.matches(hb)) problems += "$cid: crash_dedup signature is not 8 lowercase hex digits"
        if ((ha == hb) != want) problems += "$cid: crash_dedup signature hashes ${if (ha == hb) "agree" else "differ"}, corpus is_duplicate $want"
        return problems
    }

    private fun checkPrivate(cid: String, v: Map<String, Any?>): List<String> {
        val raw = bytesOf(v["raw_payload"], "raw_payload")
        val target = textOf(v["target"], "target")
        val fid = textOf(v["failure_id"], "failure_id")
        val sig = textOf(v["sig_hash"], "sig_hash")
        val tap = textOf(v["tap_line"], "tap_line")
        val wantLeak = boolOf(v["contains_raw_payload"], "contains_raw_payload")
        val line =
            try {
                formatTapFailure(1, target, fid, sig)
            } catch (e: IllegalArgumentException) {
                throw CaseProblem("format_tap_failure refused: ${e.message}")
            }
        val problems = mutableListOf<String>()
        if (line != tap) problems += "$cid: tap_line expected ${Clip.clip(tap, 200)}, got ${Clip.clip(line, 200)}"
        val leaked = raw.isNotEmpty() && line.contains(hex(raw))
        if (leaked != wantLeak) problems += "$cid: privacy verdict wrong, payload leaked: $leaked"
        if ('\r' in line || '\n' in line) problems += "$cid: failure line spans more than one line"
        return problems
    }

    /** Decodes the committed document envelope; shared with the Java package by design (VER06g section 3). */
    fun parse(data: ByteArray): List<Any?> = JavaFuzzCorpus.parse(data)
}
