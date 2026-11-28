package com.codingadventures.kotlin.microbenchmark

import java.io.OutputStream
import java.io.PrintStream
import java.nio.file.Path
import java.nio.charset.StandardCharsets
import kotlin.system.exitProcess

/** Native Kotlin driver for the VER07 indexed-array workload. */
object ArrayMicrobenchmark {
    private val values = IntArray(4096) { (17 * it + 3) % 251 }
    private val onePass = values.sumOf(Int::toLong)

    @JvmStatic fun main(args: Array<String>) {
        val status = run(args, System.out, System.err, Microbenchmark.monotonicClock())
        if (status != 0) exitProcess(status)
    }

    @JvmSynthetic
    internal fun run(args: Array<String>, stdout: OutputStream, stderr: PrintStream, clock: Clock): Int {
      try {
        var config = Config()
        var revision = "unknown"
        var replayPath: String? = null
        var i = 0
        while (i < args.size) {
            val option = args[i]
            if (option == "--help") {
                stderr.println("Usage: array-microbenchmark [--pairs N] [--confidence N] [--factor F] [--seed I64] [--target-ns N] [--max-batch N] [--revision REV] | --replay FILE")
                return 0
            }
            require(i + 1 < args.size) { "missing value for $option" }
            val value = args[++i]
            config = when (option) {
                "--pairs" -> config.copy(pairs = value.toInt())
                "--confidence" -> config.copy(confidencePercent = value.toInt())
                "--factor" -> config.copy(minimumFactor = value.toDouble())
                "--seed" -> config.copy(seed = value.toLong())
                "--target-ns" -> config.copy(targetNs = value.toLong())
                "--max-batch" -> config.copy(maxBatch = value.toLong())
                "--revision" -> { revision = value; config }
                "--replay" -> { replayPath = value; config }
                else -> error("unknown option: $option")
            }
            i++
        }
        if (replayPath != null) {
            val report = ReportCodec.replay(ReportCodec.readFile(Path.of(replayPath!!)))
            write(stdout, "replay valid: ${report.pairs.size} pairs, ${report.analysis.verdict}\n".toByteArray(StandardCharsets.UTF_8))
            return 0
        }
        config.validate()
        val indexed: Batch = { batch -> var sum = 0L; var pass = 0L; while (pass < batch) { for (i in values.indices) sum += values[i]; pass++ }; sum }
        val iterated: Batch = { batch -> var sum = 0L; var pass = 0L; while (pass < batch) { for (value in values) sum += value; pass++ }; sum }
        val report = Microbenchmark.collect(config, onePass, indexed, iterated, clock)
        val json = ReportCodec.toJson(report, listOf("array-microbenchmark", *args), revision,
            Runtime.version().toString(), System.getProperty("os.name", "unknown"), System.getProperty("os.arch", "unknown"))
        ReportCodec.replay(json)
        write(stdout, json.toByteArray(StandardCharsets.UTF_8))
        return 0
      } catch (error: Exception) {
          stderr.println("benchmark failed: ${error.message}")
          return 1
      }
    }

    private fun write(out: OutputStream, bytes: ByteArray) {
        out.write(bytes)
        out.flush()
        if (out is PrintStream && out.checkError()) throw java.io.IOException("stdout write failed")
    }
}
