package com.codingadventures.point2d;

import static org.junit.jupiter.api.Assertions.*;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.file.Path;
import java.util.HashSet;
import java.util.Set;
import org.junit.jupiter.api.Test;

/** Native G2D00 laws and the four checked language-neutral normalization seams. */
class Point2DTest {
    @Test
    void pointArithmeticIsImmutable() {
        Point a = new Point(1, 2);
        Point b = new Point(3, 4);
        assertEquals(Point.origin(), new Point(0, 0));
        assertEquals(new Point(4, 6), a.add(b));
        assertEquals(new Point(-2, -2), a.subtract(b));
        assertEquals(new Point(2, 4), a.scale(2));
        assertEquals(new Point(-1, -2), a.negate());
        assertEquals(new Point(1, 2), a);
        assertEquals(new Point(3, 4), b);
    }

    @Test
    void vectorGeometryAndAngles() {
        Point right = new Point(1, 0);
        Point up = new Point(0, 1);
        Point threeFour = new Point(3, 4);
        assertEquals(0, right.dot(up));
        assertEquals(1, right.cross(up));
        assertEquals(-1, up.cross(right));
        assertEquals(25, threeFour.magnitudeSquared());
        assertEquals(5, threeFour.magnitude(), 1e-12);
        assertEquals(25, Point.origin().distanceSquared(threeFour));
        assertEquals(5, Point.origin().distance(threeFour), 1e-12);
        assertEquals(new Point(5, 5), Point.origin().lerp(new Point(10, 10), 0.5));
        assertEquals(new Point(-5, -5), Point.origin().lerp(new Point(10, 10), -0.5));
        assertEquals(up, right.perpendicular());
        assertEquals(right.negate(), right.perpendicular().perpendicular());
        assertEquals(0, right.angle(), 1e-10);
        assertEquals(Math.PI / 2, up.angle(), 1e-9);
        assertEquals(Math.PI, new Point(-1, 0).angle(), 1e-9);
        assertEquals(-Math.PI / 2, new Point(0, -1).angle(), 1e-9);
    }

    @Test
    void rectanglesHaveHalfOpenBoundsAndEmptyIdentities() {
        Rect a = new Rect(0, 0, 10, 10);
        Rect b = new Rect(5, 5, 10, 10);
        assertEquals(Rect.zero(), new Rect(0, 0, 0, 0));
        assertEquals(a, Rect.fromPoints(new Point(0, 0), new Point(10, 10)));
        assertEquals(new Point(0, 0), a.min());
        assertEquals(new Point(10, 10), a.max());
        assertEquals(new Point(5, 5), a.center());
        assertFalse(a.isEmpty());
        assertTrue(Rect.zero().isEmpty());
        assertTrue(new Rect(0, 0, -1, 2).isEmpty());
        assertTrue(new Rect(0, 0, 2, -1).isEmpty());
        assertTrue(a.containsPoint(new Point(0, 0)));
        assertTrue(a.containsPoint(new Point(5, 5)));
        assertFalse(a.containsPoint(new Point(10, 10)));
        assertFalse(a.containsPoint(new Point(-1, 0)));
        assertFalse(Rect.zero().containsPoint(Point.origin()));
        assertEquals(new Rect(0, 0, 15, 15), a.union(b));
        assertEquals(a, a.union(Rect.zero()));
        assertEquals(a, Rect.zero().union(a));
        assertEquals(new Rect(5, 5, 5, 5), a.intersection(b));
        assertNull(a.intersection(new Rect(10, 0, 2, 2)));
        assertNull(Rect.zero().intersection(a));
        assertEquals(new Rect(0, 0, 10, 10), new Rect(1, 1, 8, 8).expandBy(1));
        assertEquals(a, new Rect(1, 1, 8, 8).expandBy(1));
    }

    @Test
    void consumesAllAndOnlyCheckedPointFixtures() throws Exception {
        JsonNode corpus = new ObjectMapper().readTree(Path.of("../../../specs/fixtures/geometry2d-v1/cases.json").toFile());
        assertEquals(1, corpus.path("version").asInt());
        assertEquals(1e-12, corpus.path("absolute_tolerance").asDouble());
        Set<String> seen = new HashSet<>();
        for (JsonNode fixture : corpus.path("cases")) {
            if (!"point-normalize".equals(fixture.path("operation").asText())) continue;
            String id = fixture.path("id").asText();
            assertTrue(seen.add(id), "duplicate fixture: " + id);
            Point actual = new Point(fixture.path("point").get(0).asDouble(), fixture.path("point").get(1).asDouble()).normalize();
            double expectedX = fixture.path("expected").get(0).asDouble();
            double expectedY = fixture.path("expected").get(1).asDouble();
            switch (fixture.path("comparison").asText()) {
                case "exact" -> { assertEquals(expectedX, actual.x(), 0); assertEquals(expectedY, actual.y(), 0); }
                case "absolute" -> { assertEquals(expectedX, actual.x(), 1e-12); assertEquals(expectedY, actual.y(), 1e-12); }
                default -> fail("unknown comparison for " + id);
            }
        }
        assertEquals(Set.of("normalize-zero", "normalize-below-epsilon", "normalize-at-epsilon", "normalize-three-four"), seen);
    }
}
