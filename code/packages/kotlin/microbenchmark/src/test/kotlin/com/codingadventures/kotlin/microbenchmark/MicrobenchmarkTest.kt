package com.codingadventures.kotlin.microbenchmark

import com.fasterxml.jackson.databind.ObjectMapper
import com.fasterxml.jackson.databind.node.ObjectNode
import java.io.ByteArrayOutputStream
import java.io.OutputStream
import java.nio.file.Files
import java.nio.file.Path
import java.util.concurrent.atomic.AtomicLong
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class MicrobenchmarkTest {
    private val mapper = ObjectMapper()
    private val quick = Config(5, 95, 1.05, 91, 1, 1)
    private fun ticks(): Clock { val value = AtomicLong(); return { value.getAndAdd(100) } }
    private fun report(config: Config = quick) = Microbenchmark.collect(config, 7, { n -> 7 * n }, { n -> 7 * n }, ticks())

    @Test fun configAndAnalysisValidateBoundaries() {
        Config().validate()
        assertFailsWith<IllegalArgumentException> { Config(pairs = 0).validate() }
        assertFailsWith<IllegalArgumentException> { Config(pairs = 10_001).validate() }
        assertFailsWith<IllegalArgumentException> { Config(confidencePercent = 0).validate() }
        assertFailsWith<IllegalArgumentException> { Config(confidencePercent = 100).validate() }
        assertFailsWith<IllegalArgumentException> { Config(minimumFactor = 1.0).validate() }
        assertFailsWith<IllegalArgumentException> { Config(minimumFactor = Double.NaN).validate() }
        assertFailsWith<IllegalArgumentException> { Config(targetNs = 0).validate() }
        assertFailsWith<IllegalArgumentException> { Config(maxBatch = 0).validate() }
        val faster = List(6) { Observation(100, 80, if (it < 3) Microbenchmark.BASELINE_FIRST else Microbenchmark.CANDIDATE_FIRST) }
        assertEquals("faster", Microbenchmark.analyze(faster, 95, 1.05).verdict)
        assertFailsWith<IllegalArgumentException> { Microbenchmark.analyze(emptyList(), 95, 1.05) }
        assertFailsWith<IllegalArgumentException> { Microbenchmark.analyze(faster, 0, 1.05) }
        assertFailsWith<IllegalArgumentException> { Microbenchmark.analyze(listOf(Observation(0, 1, Microbenchmark.BASELINE_FIRST)), 95, 1.05) }
        assertFailsWith<IllegalArgumentException> { Microbenchmark.analyze(listOf(Observation(1, 1, "bad")), 95, 1.05) }
        assertFailsWith<IllegalArgumentException> { Microbenchmark.analyze(faster, 100, 1.05) }
        assertFailsWith<IllegalArgumentException> { Microbenchmark.analyze(faster, 95, Double.POSITIVE_INFINITY) }
        assertFailsWith<IllegalArgumentException> {
            Microbenchmark.analyze(List(10_001) { Observation(1, 1, Microbenchmark.BASELINE_FIRST) }, 95, 1.05)
        }
    }

    @Test fun replayEnforcesTextTreeAndFileBudgetsBeforePairConversion() {
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(" ".repeat(4 * 1024 * 1024 + 1)) }
        val tooDeep = "[".repeat(129) + "0" + "]".repeat(129)
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(tooDeep) }
        val tooManyValues = "{\"x\":[" + "0,".repeat(65_535) + "0]}"
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(tooManyValues) }

        val oversized = Files.createTempFile("kotlin-microbenchmark-replay", ".json")
        try {
            Files.write(oversized, ByteArray(4 * 1024 * 1024 + 1))
            assertFailsWith<IllegalArgumentException> { ReportCodec.readFile(oversized) }
        } finally {
            Files.deleteIfExists(oversized)
        }
    }

    @Test fun collectorScheduleAndFailures() {
        val first = report()
        val second = report()
        assertEquals(1L, first.batchSize)
        assertEquals(first.pairs.map { it.order }, second.pairs.map { it.order })
        assertEquals(3, first.pairs.count { it.order == Microbenchmark.BASELINE_FIRST })
        assertFailsWith<IllegalArgumentException> { Microbenchmark.collect(quick, 7, { n -> 8 * n }, { n -> 7 * n }, ticks()) }
        val changesAfterWarmup = AtomicLong()
        assertFailsWith<IllegalArgumentException> { Microbenchmark.collect(quick, 7,
            { n -> if (changesAfterWarmup.getAndIncrement() == 0L) 7 * n else 8 * n }, { n -> 7 * n }, ticks()) }
        assertFailsWith<IllegalStateException> { Microbenchmark.collect(quick, 7, { error("callback") }, { n -> 7 * n }, ticks()) }
        val back = AtomicLong(3)
        assertFailsWith<IllegalArgumentException> { Microbenchmark.collect(quick, 7, { n -> 7 * n }, { n -> 7 * n }, { back.getAndDecrement() }) }
        assertFailsWith<IllegalArgumentException> { Microbenchmark.collect(quick, 7, { n -> 7 * n }, { n -> 7 * n }, { -1 }) }
        assertFailsWith<IllegalArgumentException> { Microbenchmark.collect(Config(pairs = 2, targetNs = 100, maxBatch = 1), 7, { n -> 7 * n }, { n -> 7 * n }, { 5 }) }
        assertFailsWith<IllegalArgumentException> { Microbenchmark.collect(Config(pairs = 1, targetNs = 200, maxBatch = 2), 7, { n -> 7 * n }, { n -> 7 * n }, ticks()) }
        assertFailsWith<IllegalArgumentException> { Microbenchmark.collect(Config(pairs = 1, targetNs = 300, maxBatch = 3), 7, { n -> 7 * n }, { n -> 7 * n }, ticks()) }
        val edgeReads = AtomicLong()
        val zeroMeasured = { when (edgeReads.getAndIncrement()) { 0L -> 10L; 1L -> 20L; 2L -> 30L; 3L -> 40L; else -> 50L } }
        assertFailsWith<IllegalArgumentException> { Microbenchmark.collect(Config(pairs = 1, targetNs = 1, maxBatch = 1), 7, { n -> 7 * n }, { n -> 7 * n }, zeroMeasured) }
        assertTrue(Microbenchmark.monotonicClock()() >= 0)
        assertTrue(Microbenchmark.monotonicClock()() >= 0)
        val overflow = Config(pairs = 1, targetNs = 200, maxBatch = 2)
        assertFailsWith<ArithmeticException> { Microbenchmark.collect(overflow, Long.MAX_VALUE, { Long.MAX_VALUE }, { Long.MAX_VALUE }, ticks()) }
        assertEquals(Long.MIN_VALUE, Config(seed = Long.MIN_VALUE, maxBatch = Long.MAX_VALUE).seed)
    }

    @Test fun reportCodecKeepsInt64AndRejectsTampering() {
        val config = Config(1, 95, 1.05, Long.MIN_VALUE, 1, Long.MAX_VALUE)
        val pair = Observation(Long.MAX_VALUE, Long.MAX_VALUE, Microbenchmark.BASELINE_FIRST)
        val raw = Report(config, Long.MAX_VALUE, listOf(pair), Microbenchmark.analyze(listOf(pair), 95, 1.05))
        val json = ReportCodec.toJson(raw, listOf("bench"), "r", "Kotlin 21", "Windows", "x86_64")
        assertTrue("-9223372036854775808" in json)
        assertTrue("9223372036854775807" in json)
        val decoded = ReportCodec.replay(json)
        assertEquals(Long.MIN_VALUE, decoded.config.seed)
        assertEquals(Long.MAX_VALUE, decoded.pairs.single().baselineNs)
        assertEquals(Long.MAX_VALUE, decoded.batchSize)
        val beyondSafeInteger = 9_007_199_254_740_993L
        val precisePair = Observation(beyondSafeInteger, beyondSafeInteger, Microbenchmark.BASELINE_FIRST)
        val preciseReport = Report(Config(pairs = 1, targetNs = 1, maxBatch = 1), 1, listOf(precisePair), Microbenchmark.analyze(listOf(precisePair), 95, 1.05))
        val preciseJson = ReportCodec.toJson(preciseReport, listOf("bench"), "r", "Kotlin", "Windows", "x64")
        assertTrue("9007199254740993" in preciseJson)
        assertEquals(beyondSafeInteger, ReportCodec.replay(preciseJson).pairs.single().baselineNs)
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(json.replace("9223372036854775807", "9223372036854775808")) }
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay("{\"x\":1,\"x\":2}") }
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(json + "{}") }
        val genericWorkload = json.replace("indexed-array-sum-4096", "dependent-link-order")
        val genericNode = mapper.readTree(genericWorkload) as ObjectNode
        genericNode.put("profile", "native-go")
        assertEquals(decoded.pairs, ReportCodec.replay(genericNode.toString()).pairs)
        genericNode.put("workload", "\u2003")
        assertEquals(decoded.pairs, ReportCodec.replay(genericNode.toString()).pairs)
        genericNode.put("workload", String(Character.toChars(0x1F680)))
        assertEquals(decoded.pairs, ReportCodec.replay(genericNode.toString()).pairs)
        genericNode.put("workload", "\uFFFD")
        assertEquals(decoded.pairs, ReportCodec.replay(genericNode.toString()).pairs)
        genericNode.put("workload", "literal" + 92.toChar() + "uD800")
        assertEquals(decoded.pairs, ReportCodec.replay(genericNode.toString()).pairs)
        listOf("uD800", "uDFFF").forEach { suffix ->
            val escapedSurrogate = json.replace("indexed-array-sum-4096", "\\$suffix")
            assertFailsWith<IllegalArgumentException> { ReportCodec.replay(escapedSurrogate) }
        }
        genericNode.put("workload", "")
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(genericNode.toString()) }
        genericNode.put("workload", " \t\n\r\u000c" + 11.toChar())
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(genericNode.toString()) }
        genericNode.put("workload", "\u0000")
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(genericNode.toString()) }
        genericNode.put("workload", "dependent\u0000link")
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(genericNode.toString()) }
        genericNode.putNull("workload")
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(genericNode.toString()) }
        genericNode.put("workload", 1)
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(genericNode.toString()) }
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay("[]") }
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay("not-json") }
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(json.replace("\"invocation\" : [", "\"invocation\" : 3,")) }
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(json.replace("\"minimum_factor\" : 1.05", "\"minimum_factor\" : 1e999")) }

        val valid = report()
        val validJson = ReportCodec.toJson(valid, listOf("bench"), "r", "Kotlin 21", "Windows", "x86_64")
        val tampered = ObjectMapper().readTree(validJson) as ObjectNode
        val rows = tampered.path("pairs")
        rows.forEach { (it as ObjectNode).put("order", Microbenchmark.BASELINE_FIRST) }
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(tampered.toString()) }
        val withProfile = ObjectMapper().readTree(validJson) as ObjectNode
        withProfile.put("profile", "portable")
        assertTrue(ReportCodec.replay(withProfile.toString()).analysis.verdict.isNotEmpty())
        val withEmptyArgument = mapper.readTree(validJson) as ObjectNode
        (withEmptyArgument.path("invocation") as com.fasterxml.jackson.databind.node.ArrayNode).insert(1, "")
        assertEquals(ReportCodec.replay(validJson).pairs, ReportCodec.replay(withEmptyArgument.toString()).pairs)
        withEmptyArgument.remove("invocation")
        withEmptyArgument.putArray("invocation")
        assertEquals(ReportCodec.replay(validJson).pairs, ReportCodec.replay(withEmptyArgument.toString()).pairs)
        val metadataBase = mapper.readTree(validJson) as ObjectNode
        metadataBase.put("profile", "portable")
        listOf("workload", "revision", "runtime", "os", "arch", "profile", "invocation").forEach { field ->
            val withNul = metadataBase.deepCopy()
            if (field == "invocation") {
                (withNul.path(field) as com.fasterxml.jackson.databind.node.ArrayNode).set(0, "argument\u0000with-nul")
            } else {
                withNul.put(field, "metadata\u0000with-nul")
            }
            assertFailsWith<IllegalArgumentException>(field) { ReportCodec.replay(withNul.toString()) }
        }
        listOf("workload", "revision", "runtime", "os", "arch", "profile").forEach { field ->
            val withEmptyMetadata = metadataBase.deepCopy()
            withEmptyMetadata.put(field, "")
            assertFailsWith<IllegalArgumentException>(field) { ReportCodec.replay(withEmptyMetadata.toString()) }
        }
        assertRejected(validJson) { it.put("extra", true) }
        assertRejected(validJson) { it.putNull("invocation") }
        assertRejected(validJson) { it.putNull("revision") }
        assertRejected(validJson) { it.set<ObjectNode>("config", mapper.createObjectNode()) }
        assertRejected(validJson) { it.path("config").let { cfg -> (cfg as ObjectNode).put("extra", 1) } }
        assertRejected(validJson) { it.put("batch_size", 0) }
        assertRejected(validJson) { it.set<com.fasterxml.jackson.databind.JsonNode>("pairs", mapper.createArrayNode()) }
        assertRejected(validJson) { it.path("pairs").let { rows -> (rows[0] as ObjectNode).put("extra", 1) } }
        assertRejected(validJson) { it.set<ObjectNode>("analysis", mapper.createObjectNode()) }
        assertRejected(validJson) { it.path("analysis").let { a -> (a as ObjectNode).put("ratio", 2.0) } }
        assertRejected(validJson) { it.path("analysis").let { a -> (a as ObjectNode).put("verdict", "faster") } }
        val withInterval = List(8) { Observation(100, 80, if (it < 4) Microbenchmark.BASELINE_FIRST else Microbenchmark.CANDIDATE_FIRST) }
        val intervalReport = Report(Config(pairs = 8), 1, withInterval, Microbenchmark.analyze(withInterval, 95, 1.05))
        val intervalJson = ReportCodec.toJson(intervalReport, listOf("bench"), "r", "Kotlin", "Windows", "x64")
        assertTrue(ReportCodec.replay(intervalJson).analysis.hasInterval)
        assertRejected(intervalJson) { it.path("analysis").let { a -> (a as ObjectNode).set<com.fasterxml.jackson.databind.JsonNode>("interval", mapper.createArrayNode().add(0.0).add(0.9)) } }
    }

    private fun assertRejected(text: String, change: (ObjectNode) -> Unit) {
        val node = mapper.readTree(text) as ObjectNode
        change(node)
        assertFailsWith<IllegalArgumentException> { ReportCodec.replay(node.toString()) }
    }

    @Test fun canonicalFixtureAndNativeCli() {
        val local = Path.of("testdata", "microbenchmark-pairs-v1.json")
        val canonical = Path.of("../../../specs/fixtures/microbenchmark-pairs-v1.json")
        assertContentEquals(Files.readAllBytes(canonical), Files.readAllBytes(local))
        val root = ObjectMapper().readTree(local.toFile())
        root.path("cases").forEach { sample ->
            val observations = sample.path("candidate_ns").map { Observation(sample.path("baseline_ns").longValue(), it.longValue(), Microbenchmark.BASELINE_FIRST) }
            val result = Microbenchmark.analyze(observations, root.path("confidence_percent").intValue(), root.path("minimum_factor").doubleValue())
            assertEquals(sample.path("ratio").doubleValue(), result.ratio, 1e-12)
            assertEquals(sample.path("verdict").asText(), result.verdict)
            if (sample.path("interval").isNull) assertFalse(result.hasInterval) else {
                assertEquals(sample.path("interval")[0].doubleValue(), result.low!!, 1e-12)
                assertEquals(sample.path("interval")[1].doubleValue(), result.high!!, 1e-12)
            }
        }
        val output = ByteArrayOutputStream()
        assertEquals(0, ArrayMicrobenchmark.run(arrayOf("--pairs", "40", "--target-ns", "1", "--max-batch", "1", "--revision", "test"), output, System.err, ticks()))
        val result = ReportCodec.replay(output.toString(Charsets.UTF_8))
        assertEquals(40, result.pairs.size)
        assertEquals(20, result.pairs.count { it.order == Microbenchmark.BASELINE_FIRST })
        val allOptions = ByteArrayOutputStream()
        assertEquals(0, ArrayMicrobenchmark.run(arrayOf("--pairs", "2", "--confidence", "90", "--factor", "1.1", "--seed", "-9223372036854775808", "--target-ns", "1", "--max-batch", "1", "--revision", "test"), allOptions, System.err, ticks()))
        assertEquals(2, ReportCodec.replay(allOptions.toString(Charsets.UTF_8)).pairs.size)
        assertEquals(0, ArrayMicrobenchmark.run(arrayOf("--help"), ByteArrayOutputStream(), System.err, ticks()))
        assertEquals(1, ArrayMicrobenchmark.run(arrayOf("--unknown", "x"), ByteArrayOutputStream(), System.err, ticks()))
        assertEquals(1, ArrayMicrobenchmark.run(arrayOf("--seed"), ByteArrayOutputStream(), System.err, ticks()))
        val replayFile = Files.createTempFile("kotlin-microbenchmark", ".json")
        try {
            Files.writeString(replayFile, allOptions.toString(Charsets.UTF_8))
            val replayOut = ByteArrayOutputStream()
            assertEquals(0, ArrayMicrobenchmark.run(arrayOf("--replay", replayFile.toString()), replayOut, System.err, ticks()))
            assertTrue("replay valid" in replayOut.toString(Charsets.UTF_8))
        } finally { Files.deleteIfExists(replayFile) }
        val broken = object : OutputStream() { override fun write(value: Int) { throw java.io.IOException("broken") } }
        assertEquals(1, ArrayMicrobenchmark.run(arrayOf("--pairs", "1", "--target-ns", "1", "--max-batch", "1"), broken, System.err, ticks()))
        assertEquals(1, ArrayMicrobenchmark.run(arrayOf("--pairs", "1", "--target-ns", "1", "--max-batch", "1"), java.io.PrintStream(broken), System.err, ticks()))
    }
}
