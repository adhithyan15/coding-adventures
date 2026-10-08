package com.codingadventures.arc2d;

import com.codingadventures.bezier2d.CubicBezier;
import com.codingadventures.point2d.Point;
import com.codingadventures.point2d.Rect;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class Arc2DTest {
    private static final double EPS = 1e-12;

    private static Point point(JsonNode value) {
        return new Point(value.get(0).asDouble(), value.get(1).asDouble());
    }

    private static void close(double actual, double expected) {
        assertEquals(expected, actual, EPS);
    }

    private static void close(Point actual, Point expected) {
        close(actual.x(), expected.x());
        close(actual.y(), expected.y());
    }

    private static void close(Rect actual, JsonNode expected) {
        close(actual.x(), expected.get(0).asDouble());
        close(actual.y(), expected.get(1).asDouble());
        close(actual.width(), expected.get(2).asDouble());
        close(actual.height(), expected.get(3).asDouble());
    }

    @Test void neutralArcCases() throws Exception {
        JsonNode cases = new ObjectMapper().readTree(
                Path.of("../../../specs/fixtures/geometry2d-v1/cases.json").toFile()).path("cases");
        int consumed = 0;
        for (JsonNode fixture : cases) {
            String operation = fixture.path("operation").asText();
            if (operation.equals("svg-arc-degenerate")) {
                SvgArc arc = new SvgArc(point(fixture.path("from")), point(fixture.path("to")),
                        fixture.path("rx").asDouble(), fixture.path("ry").asDouble(), 0, false, true);
                assertNull(arc.toCenterArc(), fixture.path("id").asText());
                assertTrue(arc.toCubicBeziers().isEmpty());
                close(arc.evaluate(fixture.path("sample_t").asDouble()), point(fixture.path("expected_point")));
                close(arc.boundingBox(), fixture.path("expected_bounds"));
                consumed++;
            } else if (operation.equals("center-arc-bounds") || operation.equals("center-arc-cubics")) {
                CenterArc arc = new CenterArc(point(fixture.path("center")), fixture.path("rx").asDouble(),
                        fixture.path("ry").asDouble(), fixture.path("start_angle").asDouble(),
                        fixture.path("sweep_angle").asDouble(), fixture.path("x_rotation").asDouble());
                if (operation.equals("center-arc-bounds")) {
                    close(arc.boundingBox(), fixture.path("expected_bounds"));
                } else if (fixture.has("expected_error")) {
                    assertThrows(IllegalArgumentException.class, arc::toCubicBeziers);
                    assertThrows(IllegalArgumentException.class, arc::boundingBox);
                    assertThrows(IllegalArgumentException.class, () -> arc.evaluate(0));
                } else {
                    List<CubicBezier> cubics = arc.toCubicBeziers();
                    assertEquals(fixture.path("expected_count").asInt(), cubics.size());
                    close(cubics.get(0).p0(), arc.evaluate(0));
                    close(cubics.get(cubics.size() - 1).p3(), arc.evaluate(1));
                    for (int i = 1; i < cubics.size(); i++) close(cubics.get(i - 1).p3(), cubics.get(i).p0());
                    if (arc.sweepAngle() == 0) {
                        close(cubics.get(0).p0(), cubics.get(0).p1());
                        close(cubics.get(0).p0(), cubics.get(0).p2());
                        close(cubics.get(0).p0(), cubics.get(0).p3());
                    }
                }
                consumed++;
            }
        }
        assertEquals(11, consumed, "consume every current Arc2D fixture");
    }

    @Test void svgConversionAndEvaluation() {
        SvgArc quarter = new SvgArc(new Point(1, 0), new Point(0, 1), 1, 1, 0, false, true);
        CenterArc center = quarter.toCenterArc();
        assertNotNull(center);
        close(center.center(), Point.origin());
        close(center.startAngle(), 0);
        close(center.sweepAngle(), Math.PI / 2);
        close(center.evaluate(0), quarter.from());
        close(center.evaluate(1), quarter.to());
        close(center.tangent(0), new Point(0, Math.PI / 2));
        close(quarter.evaluate(0.5), center.evaluate(0.5));
        close(quarter.boundingBox().x(), 0);
        close(quarter.boundingBox().y(), 0);
        assertEquals(1, quarter.toCubicBeziers().size());
        close(quarter.toCubicBeziers().get(0).p1().y(), 4.0 / 3.0 * Math.tan(Math.PI / 8));

        SvgArc half = new SvgArc(new Point(1, 0), new Point(-1, 0), 1, 1, 0, false, true);
        close(half.toCenterArc().sweepAngle(), Math.PI);
        close(half.evaluate(0.5), new Point(0, 1));
        assertEquals(2, half.toCubicBeziers().size());
        SvgArc scaled = new SvgArc(new Point(0, 0), new Point(10, 0), .1, .1, 0, false, true);
        assertTrue(scaled.toCenterArc().rx() >= 5);
        close(scaled.evaluate(1), scaled.to());
    }

    @Test void flagsRotationAndStrictBoundaries() {
        Point from = new Point(1, 0), to = new Point(0, 1);
        for (boolean large : new boolean[] {false, true}) {
            for (boolean sweep : new boolean[] {false, true}) {
                CenterArc arc = new SvgArc(from, to, 2, 2, Math.PI / 4, large, sweep).toCenterArc();
                assertNotNull(arc);
                assertEquals(large, Math.abs(arc.sweepAngle()) > Math.PI);
                assertEquals(sweep, arc.sweepAngle() > 0);
                close(arc.evaluate(0), from);
                close(arc.evaluate(1), to);
            }
        }
        for (SvgArc arc : new SvgArc[] {
                new SvgArc(new Point(0, 0), new Point(1, 0), 1e-10, 1, 0, false, true),
                new SvgArc(new Point(0, 0), new Point(1e-10, 0), 1, 1, 0, false, true),
                new SvgArc(new Point(0, 0), new Point(8e-11, 8e-11), 1, 1, 0, false, true) }) {
            assertNotNull(arc.toCenterArc());
            assertTrue(Double.isFinite(arc.toCenterArc().center().x()));
        }
        SvgArc positive = new SvgArc(from, to, 2, 2, 0, false, true);
        SvgArc negative = new SvgArc(from, to, -2, 2, 0, false, true);
        close(positive.toCenterArc().center(), negative.toCenterArc().center());
    }

    @Test void invalidCenterInputFailsBeforeWork() {
        double[] invalid = {Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY};
        for (double value : invalid) {
            CenterArc[] arcs = {
                    new CenterArc(new Point(value, 0), 1, 1, 0, 1, 0),
                    new CenterArc(Point.origin(), value, 1, 0, 1, 0),
                    new CenterArc(Point.origin(), 1, 1, value, 1, 0),
                    new CenterArc(Point.origin(), 1, 1, 0, value, 0),
                    new CenterArc(Point.origin(), 1, 1, 0, 1, value) };
            for (CenterArc arc : arcs) assertThrows(IllegalArgumentException.class, arc::toCubicBeziers);
        }
        for (double radius : new double[] {0, -1}) {
            CenterArc arc = new CenterArc(Point.origin(), radius, 1, 0, 1, 0);
            assertThrows(IllegalArgumentException.class, arc::boundingBox);
        }
        CenterArc overflow = new CenterArc(new Point(Double.MAX_VALUE, 0), Double.MAX_VALUE, 1, 0, 1, 0);
        assertThrows(IllegalArgumentException.class, overflow::toCubicBeziers);
    }
}
