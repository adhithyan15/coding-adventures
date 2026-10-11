package com.codingadventures.arc2d

import com.codingadventures.point2d.Point
import com.codingadventures.point2d.Rect
import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import java.io.File
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.tan
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class Arc2DTest {
    private fun point(node: JsonNode) = Point(node[0].asDouble(), node[1].asDouble())
    private fun close(actual: Double, expected: Double) = assertEquals(expected, actual, 1e-12)
    private fun close(actual: Point, expected: Point) {
        close(actual.x, expected.x)
        close(actual.y, expected.y)
    }
    private fun close(actual: Rect, expected: JsonNode) {
        close(actual.x, expected[0].asDouble())
        close(actual.y, expected[1].asDouble())
        close(actual.width, expected[2].asDouble())
        close(actual.height, expected[3].asDouble())
    }

    @Test fun neutralArcCases() {
        val cases = ObjectMapper().readTree(File("../../../specs/fixtures/geometry2d-v1/cases.json"))["cases"]
        var consumed = 0
        for (fixture in cases) {
            when (fixture["operation"].asText()) {
                "svg-arc-degenerate" -> {
                    val arc = SvgArc(point(fixture["from"]), point(fixture["to"]),
                        fixture["rx"].asDouble(), fixture["ry"].asDouble(), 0.0, false, true)
                    assertNull(arc.toCenterArc())
                    assertTrue(arc.toCubicBeziers().isEmpty())
                    close(arc.evaluate(fixture["sample_t"].asDouble()), point(fixture["expected_point"]))
                    close(arc.boundingBox(), fixture["expected_bounds"])
                    consumed++
                }
                "center-arc-bounds", "center-arc-cubics" -> {
                    val arc = CenterArc(point(fixture["center"]), fixture["rx"].asDouble(),
                        fixture["ry"].asDouble(), fixture["start_angle"].asDouble(),
                        fixture["sweep_angle"].asDouble(), fixture["x_rotation"].asDouble())
                    if (fixture["operation"].asText() == "center-arc-bounds") {
                        close(arc.boundingBox(), fixture["expected_bounds"])
                    } else if (fixture.has("expected_error")) {
                        assertThrows(IllegalArgumentException::class.java) { arc.toCubicBeziers() }
                        assertThrows(IllegalArgumentException::class.java) { arc.boundingBox() }
                        assertThrows(IllegalArgumentException::class.java) { arc.evaluate(0.0) }
                    } else {
                        val cubics = arc.toCubicBeziers()
                        assertEquals(fixture["expected_count"].asInt(), cubics.size)
                        close(cubics.first().p0, arc.evaluate(0.0))
                        close(cubics.last().p3, arc.evaluate(1.0))
                        for (i in 1 until cubics.size) close(cubics[i - 1].p3, cubics[i].p0)
                        if (arc.sweepAngle == 0.0) {
                            close(cubics[0].p0, cubics[0].p1)
                            close(cubics[0].p0, cubics[0].p2)
                            close(cubics[0].p0, cubics[0].p3)
                        }
                    }
                    consumed++
                }
            }
        }
        assertEquals(11, consumed, "consume every current Arc2D fixture")
    }

    @Test fun svgConversionAndEvaluation() {
        val quarter = SvgArc(Point(1.0, 0.0), Point(0.0, 1.0), 1.0, 1.0, 0.0, false, true)
        val center = quarter.toCenterArc()!!
        close(center.center, Point.origin())
        close(center.startAngle, 0.0)
        close(center.sweepAngle, PI / 2)
        close(center.evaluate(0.0), quarter.from)
        close(center.evaluate(1.0), quarter.to)
        close(center.tangent(0.0), Point(0.0, PI / 2))
        close(quarter.evaluate(0.5), center.evaluate(0.5))
        close(quarter.boundingBox().x, 0.0)
        assertEquals(1, quarter.toCubicBeziers().size)
        close(quarter.toCubicBeziers()[0].p1.y, 4.0 / 3.0 * tan(PI / 8))

        val half = SvgArc(Point(1.0, 0.0), Point(-1.0, 0.0), 1.0, 1.0, 0.0, false, true)
        close(half.toCenterArc()!!.sweepAngle, PI)
        close(half.evaluate(0.5), Point(0.0, 1.0))
        assertEquals(2, half.toCubicBeziers().size)
        val scaled = SvgArc(Point.origin(), Point(10.0, 0.0), .1, .1, 0.0, false, true)
        assertTrue(scaled.toCenterArc()!!.rx >= 5.0)
        close(scaled.evaluate(1.0), scaled.to)
    }

    @Test fun flagsRotationAndBoundaries() {
        val from = Point(1.0, 0.0)
        val to = Point(0.0, 1.0)
        for (large in listOf(false, true)) for (sweep in listOf(false, true)) {
            val arc = SvgArc(from, to, 2.0, 2.0, PI / 4, large, sweep).toCenterArc()!!
            assertEquals(large, abs(arc.sweepAngle) > PI)
            assertEquals(sweep, arc.sweepAngle > 0)
            close(arc.evaluate(0.0), from)
            close(arc.evaluate(1.0), to)
        }
        for (arc in listOf(
            SvgArc(Point.origin(), Point(1.0, 0.0), 1e-10, 1.0, 0.0, false, true),
            SvgArc(Point.origin(), Point(1e-10, 0.0), 1.0, 1.0, 0.0, false, true),
            SvgArc(Point.origin(), Point(8e-11, 8e-11), 1.0, 1.0, 0.0, false, true),
        )) {
            assertNotNull(arc.toCenterArc())
            assertTrue(arc.toCenterArc()!!.center.x.isFinite())
        }
        val positive = SvgArc(from, to, 2.0, 2.0, 0.0, false, true)
        val negative = SvgArc(from, to, -2.0, 2.0, 0.0, false, true)
        close(positive.toCenterArc()!!.center, negative.toCenterArc()!!.center)
    }

    @Test fun invalidCenterInputFails() {
        for (value in listOf(Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY)) {
            val arcs = listOf(
                CenterArc(Point(value, 0.0), 1.0, 1.0, 0.0, 1.0, 0.0),
                CenterArc(Point.origin(), value, 1.0, 0.0, 1.0, 0.0),
                CenterArc(Point.origin(), 1.0, 1.0, value, 1.0, 0.0),
                CenterArc(Point.origin(), 1.0, 1.0, 0.0, value, 0.0),
                CenterArc(Point.origin(), 1.0, 1.0, 0.0, 1.0, value),
            )
            for (arc in arcs) assertThrows(IllegalArgumentException::class.java) { arc.toCubicBeziers() }
        }
        for (radius in listOf(0.0, -1.0)) {
            val arc = CenterArc(Point.origin(), radius, 1.0, 0.0, 1.0, 0.0)
            assertThrows(IllegalArgumentException::class.java) { arc.boundingBox() }
        }
        val overflow = CenterArc(Point(Double.MAX_VALUE, 0.0), Double.MAX_VALUE, 1.0, 0.0, 1.0, 0.0)
        assertThrows(IllegalArgumentException::class.java) { overflow.toCubicBeziers() }
    }

    @Test fun degenerateLineRejectsNonfiniteResults() {
        val line = SvgArc(Point(-1e308, 0.0), Point(1e308, 0.0), 0.0, 1.0, 0.0, false, true)
        assertThrows(IllegalArgumentException::class.java) { line.boundingBox() }
        assertThrows(IllegalArgumentException::class.java) { line.evaluate(0.5) }
        val ordinary = SvgArc(Point.origin(), Point(1.0, 1.0), 0.0, 1.0, 0.0, false, true)
        assertThrows(IllegalArgumentException::class.java) { ordinary.evaluate(Double.NaN) }
        assertThrows(IllegalArgumentException::class.java) { ordinary.evaluate(Double.POSITIVE_INFINITY) }
    }
}
