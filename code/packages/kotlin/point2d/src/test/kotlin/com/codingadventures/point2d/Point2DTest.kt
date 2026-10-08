package com.codingadventures.point2d

import com.fasterxml.jackson.databind.ObjectMapper
import java.io.File
import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.Test

/** Native G2D00 laws, plus the checked mixed-corpus Point2D projection. */
class Point2DTest {
    @Test fun pointArithmeticIsImmutable() {
        val a = Point(1.0, 2.0)
        val b = Point(3.0, 4.0)
        assertEquals(Point(0.0, 0.0), Point.origin())
        assertEquals(Point(4.0, 6.0), a.add(b))
        assertEquals(Point(-2.0, -2.0), a.subtract(b))
        assertEquals(Point(2.0, 4.0), a.scale(2.0))
        assertEquals(Point(-1.0, -2.0), a.negate())
        assertEquals(Point(1.0, 2.0), a)
        assertEquals(Point(3.0, 4.0), b)
    }

    @Test fun vectorGeometryAndAngles() {
        val right = Point(1.0, 0.0)
        val up = Point(0.0, 1.0)
        val threeFour = Point(3.0, 4.0)
        assertEquals(0.0, right.dot(up))
        assertEquals(1.0, right.cross(up))
        assertEquals(-1.0, up.cross(right))
        assertEquals(25.0, threeFour.magnitudeSquared())
        assertEquals(5.0, threeFour.magnitude(), 1e-12)
        assertEquals(25.0, Point.origin().distanceSquared(threeFour))
        assertEquals(5.0, Point.origin().distance(threeFour), 1e-12)
        assertEquals(Point(5.0, 5.0), Point.origin().lerp(Point(10.0, 10.0), 0.5))
        assertEquals(Point(-5.0, -5.0), Point.origin().lerp(Point(10.0, 10.0), -0.5))
        assertEquals(up.x, right.perpendicular().x, 0.0)
        assertEquals(up.y, right.perpendicular().y, 0.0)
        assertEquals(right.negate().x, right.perpendicular().perpendicular().x, 0.0)
        assertEquals(right.negate().y, right.perpendicular().perpendicular().y, 0.0)
        assertEquals(0.0, right.angle(), 1e-10)
        assertEquals(Math.PI / 2, up.angle(), 1e-9)
        assertEquals(Math.PI, Point(-1.0, 0.0).angle(), 1e-9)
        assertEquals(-Math.PI / 2, Point(0.0, -1.0).angle(), 1e-9)
    }

    @Test fun rectanglesHaveHalfOpenBoundsAndEmptyIdentities() {
        val a = Rect(0.0, 0.0, 10.0, 10.0)
        val b = Rect(5.0, 5.0, 10.0, 10.0)
        assertEquals(Rect(0.0, 0.0, 0.0, 0.0), Rect.zero())
        assertEquals(a, Rect.fromPoints(Point(0.0, 0.0), Point(10.0, 10.0)))
        assertEquals(Point(0.0, 0.0), a.min())
        assertEquals(Point(10.0, 10.0), a.max())
        assertEquals(Point(5.0, 5.0), a.center())
        assertFalse(a.isEmpty())
        assertTrue(Rect.zero().isEmpty())
        assertTrue(Rect(0.0, 0.0, -1.0, 2.0).isEmpty())
        assertTrue(Rect(0.0, 0.0, 2.0, -1.0).isEmpty())
        assertTrue(a.containsPoint(Point(0.0, 0.0)))
        assertTrue(a.containsPoint(Point(5.0, 5.0)))
        assertFalse(a.containsPoint(Point(10.0, 10.0)))
        assertFalse(a.containsPoint(Point(-1.0, 0.0)))
        assertFalse(Rect.zero().containsPoint(Point.origin()))
        assertEquals(Rect(0.0, 0.0, 15.0, 15.0), a.union(b))
        assertEquals(a, a.union(Rect.zero()))
        assertEquals(a, Rect.zero().union(a))
        assertEquals(Rect(5.0, 5.0, 5.0, 5.0), a.intersection(b))
        assertNull(a.intersection(Rect(10.0, 0.0, 2.0, 2.0)))
        assertNull(Rect.zero().intersection(a))
        assertEquals(Rect(0.0, 0.0, 10.0, 10.0), Rect(1.0, 1.0, 8.0, 8.0).expandBy(1.0))
    }

    @Test fun consumesAllAndOnlyCheckedPointFixtures() {
        val corpus = ObjectMapper().readTree(File("../../../specs/fixtures/geometry2d-v1/cases.json"))
        assertEquals(1, corpus.path("version").asInt())
        assertEquals(1e-12, corpus.path("absolute_tolerance").asDouble())
        val seen = mutableSetOf<String>()
        for (fixture in corpus.path("cases")) {
            if (fixture.path("operation").asText() != "point-normalize") continue
            val id = fixture.path("id").asText()
            assertTrue(seen.add(id), "duplicate fixture: $id")
            val actual = Point(fixture.path("point")[0].asDouble(), fixture.path("point")[1].asDouble()).normalize()
            val expectedX = fixture.path("expected")[0].asDouble()
            val expectedY = fixture.path("expected")[1].asDouble()
            when (fixture.path("comparison").asText()) {
                "exact" -> { assertEquals(expectedX, actual.x, 0.0); assertEquals(expectedY, actual.y, 0.0) }
                "absolute" -> { assertEquals(expectedX, actual.x, 1e-12); assertEquals(expectedY, actual.y, 1e-12) }
                else -> fail<String>("unknown comparison for $id")
            }
        }
        assertEquals(setOf("normalize-zero", "normalize-below-epsilon", "normalize-at-epsilon", "normalize-three-four"), seen)
    }
}
