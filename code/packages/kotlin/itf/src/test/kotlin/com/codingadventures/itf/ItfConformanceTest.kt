package com.codingadventures.itf

import com.fasterxml.jackson.core.JsonParser
import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import org.junit.jupiter.api.DynamicTest
import org.junit.jupiter.api.TestFactory
import java.io.IOException
import java.io.InputStream
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.security.MessageDigest
import java.util.HexFormat
import java.util.stream.Stream
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

/**
 * Standard conformance test suite reading the canonical barcode-symbologies-v1 cases.json fixture in Kotlin.
 */
class ItfConformanceTest {
    companion object {
        private const val MAX_FIXTURE_BYTES: Int = 131_072
        private val JSON: ObjectMapper = ObjectMapper().apply {
            factory.enable(JsonParser.Feature.STRICT_DUPLICATE_DETECTION)
        }

        private fun sha256(value: String): String {
            val digest = MessageDigest.getInstance("SHA-256")
            val hash = digest.digest(value.toByteArray(StandardCharsets.UTF_8))
            return HexFormat.of().formatHex(hash)
        }

        private fun locateCasesFixture(): Path {
            val relativePath = Path.of("code", "specs", "fixtures", "barcode-symbologies-v1", "cases.json")
            var current: Path? = Path.of("").toAbsolutePath()
            while (current != null) {
                var candidate = current.resolve(relativePath)
                if (Files.isRegularFile(candidate)) {
                    return candidate
                }
                candidate = current.resolve("specs/fixtures/barcode-symbologies-v1/cases.json")
                if (Files.isRegularFile(candidate)) {
                    return candidate
                }
                current = current.parent
            }
            error("Could not find barcode-symbologies-v1 cases.json")
        }

        private fun loadCasesJson(): JsonNode {
            val fixturePath = locateCasesFixture()
            val bytes: ByteArray
            Files.newInputStream(fixturePath).use { input: InputStream ->
                bytes = input.readNBytes(MAX_FIXTURE_BYTES + 1)
            }
            if (bytes.size > MAX_FIXTURE_BYTES) {
                error("Fixture exceeds max bounded byte limit: ${bytes.size}")
            }
            return JSON.readTree(bytes)
        }

        private fun materializeInput(inputNode: JsonNode): String {
            if (inputNode.has("text")) {
                return inputNode.get("text").asText()
            }
            val repeat = inputNode.get("repeat")
            val text = repeat.get("text").asText()
            val count = repeat.get("count").asInt()
            return text.repeat(count)
        }

        private fun computeRunLengths(modules: String): List<Int> {
            val runs = ArrayList<Int>()
            if (modules.isEmpty()) {
                return runs
            }
            var current = modules[0]
            var count = 1
            for (i in 1 until modules.length) {
                val ch = modules[i]
                if (ch == current) {
                    count++
                } else {
                    runs.add(count)
                    current = ch
                    count = 1
                }
            }
            runs.add(count)
            return runs
        }
    }

    @TestFactory
    fun testCanonicalItfCases(): Stream<DynamicTest> {
        val root = loadCasesJson()
        val casesNode = root.get("cases")
        val tests = ArrayList<DynamicTest>()

        for (testCase in casesNode) {
            if (testCase.path("symbology").asText() != "itf") {
                continue
            }

            val id = testCase.get("id").asText()
            tests.add(
                DynamicTest.dynamicTest(id) {
                    val input = materializeInput(testCase.get("input"))
                    val expected = testCase.get("expected")

                    if (expected.has("error")) {
                        val expectedError = expected.get("error").asText()
                        val ex = assertFailsWith<InvalidItfInputException> {
                            Itf.normalizeItf(input)
                        }
                        assertEquals(expectedError, ex.errorId)
                        return@dynamicTest
                    }

                    val normalized = Itf.normalizeItf(input)
                    val encodedPairs = Itf.encodeItf(input)

                    val modulesBuilder = StringBuilder(ItfConstants.START_PATTERN)
                    for (pair in encodedPairs) {
                        modulesBuilder.append(pair.binaryPattern)
                    }
                    modulesBuilder.append(ItfConstants.STOP_PATTERN)
                    val modules = modulesBuilder.toString()

                    val runs = computeRunLengths(modules)

                    if (expected.has("normalized")) {
                        assertEquals(expected.get("normalized").asText(), normalized)
                        assertEquals(expected.get("modules").asText(), modules)

                        val expectedRuns = ArrayList<Int>()
                        for (runNode in expected.get("run_lengths")) {
                            expectedRuns.add(runNode.asInt())
                        }
                        assertEquals(expectedRuns, runs)
                    } else {
                        assertEquals(expected.get("normalized_sha256").asText(), sha256(normalized))
                        assertEquals(expected.get("run_count").asInt(), runs.size)

                        val compactRuns = "[" + runs.joinToString(",") + "]"
                        assertEquals(expected.get("run_lengths_sha256").asText(), sha256(compactRuns))
                    }

                    assertEquals(expected.get("module_count").asInt(), modules.length)
                    assertEquals(expected.get("module_sha256").asText(), sha256(modules))
                }
            )
        }

        assertTrue(tests.size >= 10, "Expected at least 10 ITF conformance cases")
        return tests.stream()
    }
}
