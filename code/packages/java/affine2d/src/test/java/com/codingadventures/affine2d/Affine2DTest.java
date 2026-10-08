package com.codingadventures.affine2d;

import static org.junit.jupiter.api.Assertions.*;

import com.codingadventures.point2d.Point;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.file.Path;
import java.util.HashSet;
import java.util.Set;
import org.junit.jupiter.api.Test;

/** G2D01 laws and every checked neutral affine case. */
class Affine2DTest {
    private static final double EPS = 1e-12;

    private static Point point(JsonNode pair) {
        return new Point(pair.get(0).asDouble(), pair.get(1).asDouble());
    }

    private static Affine2D matrix(JsonNode array) {
        return new Affine2D(array.get(0).asDouble(), array.get(1).asDouble(),
                array.get(2).asDouble(), array.get(3).asDouble(),
                array.get(4).asDouble(), array.get(5).asDouble());
    }

    private static void near(Point expected, Point actual) {
        assertEquals(expected.x(), actual.x(), EPS);
        assertEquals(expected.y(), actual.y(), EPS);
    }

    private static void near(double[] expected, double[] actual) {
        assertEquals(6, actual.length);
        for (int i = 0; i < 6; i++) assertEquals(expected[i], actual[i], EPS, "matrix component " + i);
    }

    @Test
    void factoriesAndPredicates() {
        Point p = new Point(1, 2);
        near(p, Affine2D.identity().applyToPoint(p));
        assertTrue(Affine2D.identity().isIdentity());
        assertTrue(Affine2D.translate(5, 7).isTranslationOnly());
        assertFalse(Affine2D.translate(0.001, 0).isIdentity());
        assertFalse(Affine2D.rotate(0.1).isTranslationOnly());
        near(new Point(6, 9), Affine2D.translate(5, 7).applyToPoint(p));
        near(new Point(2, 6), Affine2D.scale(2, 3).applyToPoint(p));
        near(new Point(2, 4), Affine2D.scaleUniform(2).applyToPoint(p));
        near(new Point(0, 1), Affine2D.rotate(Math.PI / 2).applyToPoint(new Point(1, 0)));
        near(new Point(5, 6), Affine2D.rotateAround(new Point(5, 5), Math.PI / 2)
                .applyToPoint(new Point(6, 5)));
        near(new Point(3, 2), Affine2D.skewX(Math.PI / 4).applyToPoint(p));
        near(new Point(1, 3), Affine2D.skewY(Math.PI / 4).applyToPoint(p));
        assertEquals(6, Affine2D.scale(2, 3).determinant(), EPS);
        assertEquals(1, Affine2D.rotate(0.4).determinant(), 1e-10);
        assertNull(Affine2D.scale(0, 1).invert());
        near(Affine2D.identity().toArray(), Affine2D.translate(3, 5)
                .multiply(Affine2D.translate(3, 5).invert()).toArray());
    }

    @Test
    void consumesEveryNeutralAffineCase() throws Exception {
        JsonNode corpus = new ObjectMapper().readTree(
                Path.of("../../../specs/fixtures/geometry2d-v1/cases.json").toFile());
        assertEquals(1, corpus.path("version").asInt());
        assertEquals(EPS, corpus.path("absolute_tolerance").asDouble());
        Set<String> seen = new HashSet<>();
        for (JsonNode fixture : corpus.path("cases")) {
            String operation = fixture.path("operation").asText();
            if (!operation.startsWith("affine-")) continue;
            String id = fixture.path("id").asText();
            assertTrue(seen.add(id), "duplicate fixture: " + id);
            switch (operation) {
                case "affine-compose" -> {
                    Affine2D actual = matrix(fixture.path("first"))
                            .multiply(matrix(fixture.path("second")));
                    near(matrix(fixture.path("expected_matrix")).toArray(), actual.toArray());
                    near(point(fixture.path("expected_point")),
                            actual.applyToPoint(point(fixture.path("point"))));
                }
                case "affine-invert" -> {
                    Affine2D original = matrix(fixture.path("matrix"));
                    Affine2D inverse = original.invert();
                    JsonNode expected = fixture.path("expected_inverse");
                    if (expected.isNull()) assertNull(inverse);
                    else {
                        assertNotNull(inverse);
                        near(matrix(expected).toArray(), inverse.toArray());
                        near(Affine2D.identity().toArray(), original.multiply(inverse).toArray());
                        near(Affine2D.identity().toArray(), inverse.multiply(original).toArray());
                    }
                }
                case "affine-vector" -> near(point(fixture.path("expected")),
                        matrix(fixture.path("matrix")).applyToVector(point(fixture.path("vector"))));
                default -> fail("unknown affine operation: " + operation);
            }
        }
        assertEquals(Set.of("affine-compose-order-a", "affine-compose-order-b",
                "affine-invert-nonsingular", "affine-invert-singular",
                "affine-vector-translation"), seen);
    }
}
