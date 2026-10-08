package com.codingadventures.affine2d

import com.codingadventures.point2d.Point
import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import java.io.File
import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.Test

/** G2D01 transform laws and the closed five-case neutral projection. */
class Affine2DTest {
    private val epsilon = 1e-12

    private fun point(node: JsonNode) = Point(node[0].asDouble(), node[1].asDouble())

    private fun matrix(node: JsonNode) = Affine2D(
        node[0].asDouble(), node[1].asDouble(), node[2].asDouble(),
        node[3].asDouble(), node[4].asDouble(), node[5].asDouble(),
    )

    private fun near(expected: Point, actual: Point) {
        assertEquals(expected.x, actual.x, epsilon)
        assertEquals(expected.y, actual.y, epsilon)
    }

    private fun near(expected: DoubleArray, actual: DoubleArray) {
        assertEquals(6, actual.size)
        for (index in expected.indices) assertEquals(expected[index], actual[index], epsilon, "matrix component $index")
    }

    @Test fun factoriesAndPredicates() {
        val p = Point(1.0, 2.0)
        near(p, Affine2D.identity().applyToPoint(p))
        assertTrue(Affine2D.identity().isIdentity())
        assertTrue(Affine2D.translate(5.0, 7.0).isTranslationOnly())
        assertFalse(Affine2D.translate(0.001, 0.0).isIdentity())
        assertFalse(Affine2D.rotate(0.1).isTranslationOnly())
        near(Point(6.0, 9.0), Affine2D.translate(5.0, 7.0).applyToPoint(p))
        near(Point(2.0, 6.0), Affine2D.scale(2.0, 3.0).applyToPoint(p))
        near(Point(2.0, 4.0), Affine2D.scaleUniform(2.0).applyToPoint(p))
        near(Point(0.0, 1.0), Affine2D.rotate(Math.PI / 2).applyToPoint(Point(1.0, 0.0)))
        near(Point(5.0, 6.0), Affine2D.rotateAround(Point(5.0, 5.0), Math.PI / 2)
            .applyToPoint(Point(6.0, 5.0)))
        near(Point(3.0, 2.0), Affine2D.skewX(Math.PI / 4).applyToPoint(p))
        near(Point(1.0, 3.0), Affine2D.skewY(Math.PI / 4).applyToPoint(p))
        assertEquals(6.0, Affine2D.scale(2.0, 3.0).determinant(), epsilon)
        assertEquals(1.0, Affine2D.rotate(0.4).determinant(), 1e-10)
        assertNull(Affine2D.scale(0.0, 1.0).invert())
        val translated = Affine2D.translate(3.0, 5.0)
        near(Affine2D.identity().toArray(), translated.multiply(translated.invert()!!).toArray())
    }

    @Test fun consumesEveryNeutralAffineCase() {
        val corpus = ObjectMapper().readTree(File("../../../specs/fixtures/geometry2d-v1/cases.json"))
        assertEquals(1, corpus.path("version").asInt())
        assertEquals(epsilon, corpus.path("absolute_tolerance").asDouble())
        val seen = mutableSetOf<String>()
        for (fixture in corpus.path("cases")) {
            val operation = fixture.path("operation").asText()
            if (!operation.startsWith("affine-")) continue
            val id = fixture.path("id").asText()
            assertTrue(seen.add(id), "duplicate fixture: $id")
            when (operation) {
                "affine-compose" -> {
                    val actual = matrix(fixture.path("first")).multiply(matrix(fixture.path("second")))
                    near(matrix(fixture.path("expected_matrix")).toArray(), actual.toArray())
                    near(point(fixture.path("expected_point")), actual.applyToPoint(point(fixture.path("point"))))
                }
                "affine-invert" -> {
                    val actual = matrix(fixture.path("matrix")).invert()
                    val expected = fixture.path("expected_inverse")
                    if (expected.isNull) assertNull(actual)
                    else near(matrix(expected).toArray(), requireNotNull(actual).toArray())
                }
                "affine-vector" -> near(point(fixture.path("expected")),
                    matrix(fixture.path("matrix")).applyToVector(point(fixture.path("vector"))))
                else -> fail<String>("unknown affine operation: $operation")
            }
        }
        assertEquals(setOf("affine-compose-order-a", "affine-compose-order-b",
            "affine-invert-nonsingular", "affine-invert-singular", "affine-vector-translation"), seen)
    }
}
