package com.codingadventures.bezier2d

import com.codingadventures.point2d.Point
import com.codingadventures.point2d.Rect
import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import java.io.File
import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.Test

/** Dynamic G2D02 polynomial and bounded-flattening conformance. */
class Bezier2DTest {
    private val epsilon = 1e-12

    private fun point(node: JsonNode) = Point(node[0].asDouble(), node[1].asDouble())
    private fun quadratic(nodes: JsonNode) = QuadraticBezier(point(nodes[0]), point(nodes[1]), point(nodes[2]))
    private fun cubic(nodes: JsonNode) = CubicBezier(point(nodes[0]), point(nodes[1]), point(nodes[2]), point(nodes[3]))
    private fun rect(node: JsonNode) = Rect(node[0].asDouble(), node[1].asDouble(), node[2].asDouble(), node[3].asDouble())

    private fun near(expected: Point, actual: Point) {
        assertEquals(expected.x, actual.x, epsilon)
        assertEquals(expected.y, actual.y, epsilon)
    }

    private fun near(expected: Rect, actual: Rect) {
        assertEquals(expected.x, actual.x, epsilon)
        assertEquals(expected.y, actual.y, epsilon)
        assertEquals(expected.width, actual.width, epsilon)
        assertEquals(expected.height, actual.height, epsilon)
    }

    private fun segmentDistance(p: Point, a: Point, b: Point): Double {
        val chord = b.subtract(a)
        val length2 = chord.magnitudeSquared()
        val t = if (length2 == 0.0) 0.0 else (p.subtract(a).dot(chord) / length2).coerceIn(0.0, 1.0)
        return p.distance(a.add(chord.scale(t)))
    }

    private fun polylineDistance(p: Point, line: List<Point>): Double =
        (1 until line.size).minOf { segmentDistance(p, line[it - 1], line[it]) }

    @Test fun consumesAllPolynomialCases() {
        val corpus = ObjectMapper().readTree(File("../../../specs/fixtures/geometry2d-v1/cases.json"))
        assertEquals(1, corpus.path("version").asInt())
        assertEquals(epsilon, corpus.path("absolute_tolerance").asDouble())
        val seen = mutableSetOf<String>()
        for (fixture in corpus.path("cases")) {
            val operation = fixture.path("operation").asText()
            if (!operation.startsWith("bezier-")) continue
            val id = fixture.path("id").asText()
            assertTrue(seen.add(id), "duplicate fixture: $id")
            val split = fixture.path("expected_split")
            val t = fixture.path("t").asDouble()
            when (operation) {
                "bezier-quadratic" -> {
                    val q = quadratic(fixture.path("control_points"))
                    near(point(fixture.path("expected_point")), q.evaluate(t))
                    near(point(fixture.path("expected_derivative")), q.derivative(t))
                    val (left, right) = q.split(t)
                    val expectedLeft = quadratic(split[0]); val expectedRight = quadratic(split[1])
                    near(expectedLeft.p0, left.p0); near(expectedLeft.p1, left.p1); near(expectedLeft.p2, left.p2)
                    near(expectedRight.p0, right.p0); near(expectedRight.p1, right.p1); near(expectedRight.p2, right.p2)
                    near(rect(fixture.path("expected_bounds")), q.boundingBox())
                    near(q.evaluate(t), q.elevate().evaluate(t))
                }
                "bezier-cubic" -> {
                    val c = cubic(fixture.path("control_points"))
                    near(point(fixture.path("expected_point")), c.evaluate(t))
                    near(point(fixture.path("expected_derivative")), c.derivative(t))
                    val (left, right) = c.split(t)
                    val expectedLeft = cubic(split[0]); val expectedRight = cubic(split[1])
                    near(expectedLeft.p0, left.p0); near(expectedLeft.p1, left.p1)
                    near(expectedLeft.p2, left.p2); near(expectedLeft.p3, left.p3)
                    near(expectedRight.p0, right.p0); near(expectedRight.p1, right.p1)
                    near(expectedRight.p2, right.p2); near(expectedRight.p3, right.p3)
                    near(rect(fixture.path("expected_bounds")), c.boundingBox())
                }
                else -> fail<String>("unknown Bezier operation: $operation")
            }
        }
        assertEquals(setOf("bezier-quadratic-quarter", "bezier-cubic-quarter", "bezier-cubic-x-overshoot"), seen)
    }

    @Test fun consumesAllSafeFlatteningCases() {
        val corpus = ObjectMapper().readTree(File("../../../specs/fixtures/bezier2d-flattening-v1/cases.json"))
        assertEquals(1, corpus.path("version").asInt())
        assertEquals(epsilon, corpus.path("absolute_tolerance").asDouble())
        assertEquals(32, corpus.path("max_depth").asInt())
        assertEquals(65535, corpus.path("max_subdivisions").asInt())
        val seen = mutableSetOf<String>()
        for (fixture in corpus.path("cases")) {
            val id = fixture.path("id").asText()
            assertTrue(seen.add(id), "duplicate fixture: $id")
            val tolerance = fixture.path("tolerance").asDouble()
            val controls = fixture.path("control_points")
            val isQuadratic = fixture.path("degree").asInt() == 2
            val disposition = fixture.path("expected_disposition").asText()
            if (disposition == "invalid-tolerance") {
                assertThrows(IllegalArgumentException::class.java) {
                    if (isQuadratic) quadratic(controls).toPolyline(tolerance) else cubic(controls).toPolyline(tolerance)
                }
                continue
            }
            if (fixture.path("expected_termination").asText() == "budget-error") {
                assertThrows(IllegalStateException::class.java) {
                    if (isQuadratic) quadratic(controls).toPolyline(tolerance) else cubic(controls).toPolyline(tolerance)
                }
                continue
            }
            val line = if (isQuadratic) quadratic(controls).toPolyline(tolerance) else cubic(controls).toPolyline(tolerance)
            near(point(controls[0]), line.first())
            near(point(controls[controls.size() - 1]), line.last())
            when (disposition) {
                "chord" -> assertEquals(2, line.size, id)
                "subdivide" -> assertTrue(line.size > 2, id)
                else -> fail<String>("unknown flattening disposition: $disposition")
            }
            if (fixture.has("witness_t")) {
                val witness = if (isQuadratic) quadratic(controls).evaluate(fixture.path("witness_t").asDouble())
                    else cubic(controls).evaluate(fixture.path("witness_t").asDouble())
                near(point(fixture.path("expected_witness")), witness)
                assertTrue(polylineDistance(witness, line) <= tolerance + epsilon, id)
            }
        }
        assertEquals(setOf("quadratic-straight", "quadratic-arch", "cubic-collinear-inside",
            "cubic-symmetric-s", "cubic-collinear-overshoot", "cubic-coincident-loop",
            "zero-tolerance", "negative-tolerance", "quadratic-budget-exhaustion"), seen)
    }

    @Test fun rejectsNonfiniteInputsAndTolerance() {
        val q = QuadraticBezier(Point(Double.NaN, 0.0), Point(1.0, 1.0), Point(2.0, 0.0))
        assertThrows(IllegalArgumentException::class.java) { q.toPolyline(0.1) }
        val c = CubicBezier(Point(0.0, 0.0), Point(1.0, 1.0), Point(2.0, 1.0), Point(3.0, 0.0))
        assertThrows(IllegalArgumentException::class.java) { c.toPolyline(Double.POSITIVE_INFINITY) }
    }
}
