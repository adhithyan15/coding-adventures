package com.codingadventures.kotlin.microbenchmark

import com.fasterxml.jackson.core.JsonFactory
import com.fasterxml.jackson.core.StreamReadFeature
import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.DeserializationFeature
import com.fasterxml.jackson.databind.ObjectMapper
import java.io.IOException

/** JSON report codec that accepts only exact signed 64-bit integral tokens. */
object ReportCodec {
    private val json = ObjectMapper(JsonFactory.builder().enable(StreamReadFeature.STRICT_DUPLICATE_DETECTION).build())
        .enable(DeserializationFeature.FAIL_ON_TRAILING_TOKENS)
    private val topKeys = setOf("workload", "invocation", "revision", "runtime", "os", "arch", "config", "batch_size", "pairs", "analysis")
    private val configKeys = setOf("pairs", "confidence_percent", "minimum_factor", "seed", "target_ns", "max_batch")

    fun toJson(report: Report, invocation: List<String>, revision: String, runtime: String, os: String, arch: String): String {
        val root = linkedMapOf<String, Any?>(
            "workload" to "indexed-array-sum-4096", "invocation" to invocation,
            "revision" to revision, "runtime" to runtime, "os" to os, "arch" to arch,
            "config" to linkedMapOf("pairs" to report.config.pairs, "confidence_percent" to report.config.confidencePercent,
                "minimum_factor" to report.config.minimumFactor, "seed" to report.config.seed,
                "target_ns" to report.config.targetNs, "max_batch" to report.config.maxBatch),
            "batch_size" to report.batchSize,
            "pairs" to report.pairs.map { linkedMapOf("baseline_ns" to it.baselineNs, "candidate_ns" to it.candidateNs, "order" to it.order) },
            "analysis" to linkedMapOf("ratio" to report.analysis.ratio,
                "interval" to if (report.analysis.hasInterval) listOf(report.analysis.low, report.analysis.high) else null,
                "verdict" to report.analysis.verdict),
        )
        return json.writerWithDefaultPrettyPrinter().writeValueAsString(root) + "\n"
    }

    fun replay(text: String): Report {
        val root = try { json.readTree(text) } catch (error: IOException) {
            throw IllegalArgumentException("invalid JSON report", error)
        }
        require(root != null && root.isObject) { "report must be an object" }
        val fields = names(root)
        require(fields == topKeys || fields == topKeys + "profile") { "report fields do not match portable schema" }
        val workload = string(root, "workload")
        require(isValidWorkload(workload)) { "workload must contain a non-ASCII-whitespace character and no NUL" }
        val invocation = root.path("invocation")
        require(invocation.isArray && invocation.all { it.isTextual }) { "invocation must contain strings" }
        listOf("revision", "runtime", "os", "arch").forEach { string(root, it) }
        if ("profile" in fields) string(root, "profile")
        val cfg = root.path("config")
        require(cfg.isObject && names(cfg) == configKeys) { "invalid config fields" }
        val config = Config(
            int(cfg, "pairs", 1, 10_000), int(cfg, "confidence_percent", 1, 99), number(cfg, "minimum_factor"),
            long(cfg, "seed", Long.MIN_VALUE, Long.MAX_VALUE), long(cfg, "target_ns", 1, Long.MAX_VALUE),
            long(cfg, "max_batch", 1, Long.MAX_VALUE),
        )
        config.validate()
        val batchSize = long(root, "batch_size", 1, config.maxBatch)
        val rows = root.path("pairs")
        require(rows.isArray && rows.size() == config.pairs) { "pair count does not match config" }
        val pairs = rows.map { row ->
            require(row.isObject && names(row) == setOf("baseline_ns", "candidate_ns", "order")) { "invalid pair fields" }
            Observation(long(row, "baseline_ns", 1, Long.MAX_VALUE), long(row, "candidate_ns", 1, Long.MAX_VALUE), string(row, "order"))
        }
        require(pairs.count { it.order == Microbenchmark.BASELINE_FIRST } == (config.pairs + 1) / 2) { "pair order schedule is unbalanced" }
        val analysis = Microbenchmark.analyze(pairs, config.confidencePercent, config.minimumFactor)
        checkAnalysis(root.path("analysis"), analysis)
        return Report(config, batchSize, pairs, analysis)
    }

    private fun checkAnalysis(node: JsonNode, want: Analysis) {
        require(node.isObject && names(node) == setOf("ratio", "interval", "verdict")) { "invalid analysis fields" }
        require(number(node, "ratio").toBits() == want.ratio.toBits()) { "analysis ratio does not match raw pairs" }
        require(string(node, "verdict") == want.verdict) { "analysis verdict does not match raw pairs" }
        val interval = node.path("interval")
        if (!want.hasInterval) require(interval.isNull) { "analysis interval should be null" }
        else require(interval.isArray && interval.size() == 2 && interval[0].isNumber && interval[1].isNumber &&
            interval[0].doubleValue().toBits() == want.low!!.toBits() && interval[1].doubleValue().toBits() == want.high!!.toBits()) {
            "analysis interval does not match raw pairs"
        }
    }
    private fun names(node: JsonNode) = node.fieldNames().asSequence().toSet()
    private fun isValidWorkload(workload: String): Boolean {
        if ('\u0000' in workload) return false
        var hasNonWhitespace = false
        var index = 0
        while (index < workload.length) {
            val character = workload[index]
            when {
                Character.isHighSurrogate(character) -> {
                    if (index + 1 >= workload.length || !Character.isLowSurrogate(workload[index + 1])) return false
                    index++
                    hasNonWhitespace = true
                }
                Character.isLowSurrogate(character) -> return false
                character != ' ' && character !in '\t'..'\r' -> hasNonWhitespace = true
            }
            index++
        }
        return hasNonWhitespace
    }
    private fun string(node: JsonNode, name: String): String {
        val value = node.path(name)
        require(value.isTextual) { "$name must be a string" }
        return value.textValue()
    }
    private fun long(node: JsonNode, name: String, min: Long, max: Long): Long {
        val value = node.path(name)
        require(value.isIntegralNumber && value.canConvertToLong()) { "$name must be an exact signed 64-bit integer" }
        val result = value.longValue()
        require(result in min..max) { "$name is outside its allowed range" }
        return result
    }
    private fun int(node: JsonNode, name: String, min: Int, max: Int): Int = long(node, name, min.toLong(), max.toLong()).toInt()
    private fun number(node: JsonNode, name: String): Double {
        val value = node.path(name)
        require(value.isNumber && value.doubleValue().isFinite()) { "$name must be finite number" }
        return value.doubleValue()
    }
}
