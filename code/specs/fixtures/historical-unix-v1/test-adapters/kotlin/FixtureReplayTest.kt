package codingadventures.unixhistory

import com.fasterxml.jackson.core.type.TypeReference
import com.fasterxml.jackson.databind.ObjectMapper
import java.io.File
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals

private fun normalizeFixtureValue(value: Any?): Any? = when (value) {
    is Number -> value.toLong()
    is Map<*, *> -> value.entries.associate { (key, item) -> key.toString() to normalizeFixtureValue(item) }
    is Iterable<*> -> value.map(::normalizeFixtureValue)
    else -> value
}

/** Reads the canonical fixture outside the reusable Kotlin package. */
class FixtureReplayTest {
    @Test
    fun `replays all canonical file read cases through the package API`() {
        val fixture = File("../../../specs/fixtures/historical-unix-v1/file-read.json")
        require(fixture.isFile) { "canonical HOS00 fixture is missing: ${fixture.path}" }
        val mapper = ObjectMapper()
        val scenarios = mapper.readTree(fixture)
        require(scenarios.size() == 9) { "expected nine canonical traces, got ${scenarios.size()}" }

        scenarios.forEach { scenario ->
            val command = scenario["command"].asText()
            val actual = runCommand(command)
            assertEquals(scenario["output"].asText(), actual["output"], command)
            val expectedEvents = mapper.readValue<List<Map<String, Any?>>>(
                scenario["events"].toString(),
                object : TypeReference<List<Map<String, Any?>>>() {},
            )
            val actualEvents = mapper.convertValue<List<Map<String, Any?>>>(
                actual["events"],
                object : TypeReference<List<Map<String, Any?>>>() {},
            )
            assertEquals(normalizeFixtureValue(expectedEvents), normalizeFixtureValue(actualEvents), command)
        }
    }
}
