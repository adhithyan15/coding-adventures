package com.codingadventures.bezier2d;

import static org.junit.jupiter.api.Assertions.*;

import com.codingadventures.point2d.Point;
import com.codingadventures.point2d.Rect;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.file.Path;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import org.junit.jupiter.api.Test;

/** G2D02 polynomial and adversarial flattening evidence, read from the shared corpora. */
class Bezier2DTest {
    private static final double EPS = 1e-12;

    private static Point point(JsonNode node) {
        return new Point(node.get(0).asDouble(), node.get(1).asDouble());
    }

    private static void near(Point expected, Point actual) {
        assertEquals(expected.x(), actual.x(), EPS);
        assertEquals(expected.y(), actual.y(), EPS);
    }

    private static void near(Rect expected, Rect actual) {
        assertEquals(expected.x(), actual.x(), EPS);
        assertEquals(expected.y(), actual.y(), EPS);
        assertEquals(expected.width(), actual.width(), EPS);
        assertEquals(expected.height(), actual.height(), EPS);
    }

    private static Rect rect(JsonNode node) {
        return new Rect(node.get(0).asDouble(), node.get(1).asDouble(),
                node.get(2).asDouble(), node.get(3).asDouble());
    }

    private static QuadraticBezier quadratic(JsonNode nodes) {
        return new QuadraticBezier(point(nodes.get(0)), point(nodes.get(1)), point(nodes.get(2)));
    }

    private static CubicBezier cubic(JsonNode nodes) {
        return new CubicBezier(point(nodes.get(0)), point(nodes.get(1)),
                point(nodes.get(2)), point(nodes.get(3)));
    }

    private static double segmentDistance(Point p, Point a, Point b) {
        Point chord = b.subtract(a);
        double length2 = chord.magnitudeSquared();
        double t = length2 == 0 ? 0 : Math.max(0, Math.min(1, p.subtract(a).dot(chord) / length2));
        return p.distance(a.add(chord.scale(t)));
    }

    private static double polylineDistance(Point p, List<Point> polyline) {
        double best = Double.POSITIVE_INFINITY;
        for (int i = 1; i < polyline.size(); i++) {
            best = Math.min(best, segmentDistance(p, polyline.get(i - 1), polyline.get(i)));
        }
        return best;
    }

    @Test
    void consumesAllPolynomialCases() throws Exception {
        JsonNode corpus = new ObjectMapper().readTree(
                Path.of("../../../specs/fixtures/geometry2d-v1/cases.json").toFile());
        assertEquals(1, corpus.path("version").asInt());
        assertEquals(EPS, corpus.path("absolute_tolerance").asDouble());
        Set<String> seen = new HashSet<>();
        for (JsonNode fixture : corpus.path("cases")) {
            String operation = fixture.path("operation").asText();
            if (!operation.startsWith("bezier-")) continue;
            String id = fixture.path("id").asText();
            assertTrue(seen.add(id), "duplicate fixture: " + id);
            JsonNode split = fixture.path("expected_split");
            double t = fixture.path("t").asDouble();
            switch (operation) {
                case "bezier-quadratic" -> {
                    QuadraticBezier q = quadratic(fixture.path("control_points"));
                    near(point(fixture.path("expected_point")), q.evaluate(t));
                    near(point(fixture.path("expected_derivative")), q.derivative(t));
                    Split<QuadraticBezier> halves = q.split(t);
                    near(quadratic(split.get(0)).p0(), halves.left().p0());
                    near(quadratic(split.get(0)).p1(), halves.left().p1());
                    near(quadratic(split.get(0)).p2(), halves.left().p2());
                    near(quadratic(split.get(1)).p0(), halves.right().p0());
                    near(quadratic(split.get(1)).p1(), halves.right().p1());
                    near(quadratic(split.get(1)).p2(), halves.right().p2());
                    near(rect(fixture.path("expected_bounds")), q.boundingBox());
                    near(q.evaluate(t), q.elevate().evaluate(t));
                }
                case "bezier-cubic" -> {
                    CubicBezier c = cubic(fixture.path("control_points"));
                    near(point(fixture.path("expected_point")), c.evaluate(t));
                    near(point(fixture.path("expected_derivative")), c.derivative(t));
                    Split<CubicBezier> halves = c.split(t);
                    CubicBezier left = cubic(split.get(0));
                    CubicBezier right = cubic(split.get(1));
                    near(left.p0(), halves.left().p0());
                    near(left.p1(), halves.left().p1());
                    near(left.p2(), halves.left().p2());
                    near(left.p3(), halves.left().p3());
                    near(right.p0(), halves.right().p0());
                    near(right.p1(), halves.right().p1());
                    near(right.p2(), halves.right().p2());
                    near(right.p3(), halves.right().p3());
                    near(rect(fixture.path("expected_bounds")), c.boundingBox());
                }
                default -> fail("unknown Bezier operation: " + operation);
            }
        }
        assertEquals(Set.of("bezier-quadratic-quarter", "bezier-cubic-quarter",
                "bezier-cubic-x-overshoot"), seen);
    }

    @Test
    void consumesAllSafeFlatteningCases() throws Exception {
        JsonNode corpus = new ObjectMapper().readTree(
                Path.of("../../../specs/fixtures/bezier2d-flattening-v1/cases.json").toFile());
        assertEquals(1, corpus.path("version").asInt());
        assertEquals(EPS, corpus.path("absolute_tolerance").asDouble());
        assertEquals(32, corpus.path("max_depth").asInt());
        assertEquals(65535, corpus.path("max_subdivisions").asInt());
        Set<String> seen = new HashSet<>();
        for (JsonNode fixture : corpus.path("cases")) {
            String id = fixture.path("id").asText();
            assertTrue(seen.add(id), "duplicate fixture: " + id);
            double tolerance = fixture.path("tolerance").asDouble();
            boolean isQuadratic = fixture.path("degree").asInt() == 2;
            JsonNode controls = fixture.path("control_points");
            String disposition = fixture.path("expected_disposition").asText();
            if ("invalid-tolerance".equals(disposition)) {
                assertThrows(IllegalArgumentException.class,
                        () -> { if (isQuadratic) quadratic(controls).toPolyline(tolerance);
                                else cubic(controls).toPolyline(tolerance); }, id);
                continue;
            }
            if ("budget-error".equals(fixture.path("expected_termination").asText())) {
                assertThrows(IllegalStateException.class,
                        () -> { if (isQuadratic) quadratic(controls).toPolyline(tolerance);
                                else cubic(controls).toPolyline(tolerance); }, id);
                continue;
            }
            List<Point> polyline = isQuadratic
                    ? quadratic(controls).toPolyline(tolerance) : cubic(controls).toPolyline(tolerance);
            near(point(controls.get(0)), polyline.get(0));
            near(point(controls.get(controls.size() - 1)), polyline.get(polyline.size() - 1));
            if ("chord".equals(disposition)) assertEquals(2, polyline.size(), id);
            else if ("subdivide".equals(disposition)) assertTrue(polyline.size() > 2, id);
            else fail("unknown flattening disposition: " + disposition);
            // Probe the entire parameter domain, not only the named adversarial witness.
            for (int sample = 0; sample <= 1024; sample++) {
                double t = sample / 1024.0;
                Point onCurve = isQuadratic ? quadratic(controls).evaluate(t) : cubic(controls).evaluate(t);
                assertTrue(polylineDistance(onCurve, polyline) <= tolerance + 1e-14,
                        id + " at t=" + t);
            }
            if (fixture.has("witness_t")) {
                Point witness = isQuadratic ? quadratic(controls).evaluate(fixture.path("witness_t").asDouble())
                        : cubic(controls).evaluate(fixture.path("witness_t").asDouble());
                near(point(fixture.path("expected_witness")), witness);
                assertTrue(polylineDistance(witness, polyline) <= tolerance + 1e-14, id);
            }
        }
        assertEquals(Set.of("quadratic-straight", "quadratic-arch", "cubic-collinear-inside",
                "cubic-symmetric-s", "cubic-collinear-overshoot", "cubic-coincident-loop",
                "zero-tolerance", "negative-tolerance", "quadratic-budget-exhaustion"), seen);
    }

    @Test
    void rejectsNonfiniteInputsAndTolerance() {
        QuadraticBezier q = new QuadraticBezier(new Point(Double.NaN, 0), new Point(1, 1), new Point(2, 0));
        assertThrows(IllegalArgumentException.class, () -> q.toPolyline(0.1));
        assertThrows(IllegalArgumentException.class, () -> q.toPolyline(Double.NaN));
        assertThrows(IllegalArgumentException.class, () -> q.toPolyline(Double.NEGATIVE_INFINITY));
        CubicBezier c = new CubicBezier(new Point(0, 0), new Point(1, 1),
                new Point(2, 1), new Point(3, 0));
        assertThrows(IllegalArgumentException.class, () -> c.toPolyline(Double.POSITIVE_INFINITY));
        assertThrows(IllegalArgumentException.class, () -> new CubicBezier(
                new Point(0, 0), new Point(1, Double.POSITIVE_INFINITY),
                new Point(2, 1), new Point(3, 0)).toPolyline(0.1));
    }
}
