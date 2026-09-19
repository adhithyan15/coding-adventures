package com.codingadventures.buildtool;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;

final class BuildToolCoreTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();
    private static final Set<String> EXPECTED_CASES = Set.of(
            "diff-selection/forced-package",
            "diff-selection/match-work-at-limit",
            "diff-selection/match-work-over-limit",
            "diff-selection/package-prefix",
            "diff-selection/repository-boundary-reverse-index",
            "diff-selection/transitive-package-change",
            "diff-selection/unknown-path-all",
            "diff-selection/unknown-path-error",
            "graph/canonical-edge-order",
            "graph/chain",
            "graph/cycle",
            "graph/diamond",
            "graph/isolated",
            "graph/multiple-components");

    @Test
    void consumesEverySharedGraphAndDiffFixture() throws IOException {
        Path fixtureRoot = repositoryRoot().resolve("code/specs/fixtures/build-tool-v1");
        Set<String> seen = new HashSet<>();
        try (Stream<Path> paths = Files.list(fixtureRoot.resolve("cases"))) {
            for (Path path : paths.filter(file -> file.toString().endsWith(".json")).sorted().toList()) {
                JsonNode fixture = MAPPER.readTree(path.toFile());
                String domain = fixture.path("domain").asText();
                if (!domain.equals("graph") && !domain.equals("diff_selection")) {
                    continue;
                }
                seen.add(fixture.path("id").asText());
                if (domain.equals("graph")) {
                    assertGraphFixture(fixture);
                } else {
                    assertDiffFixture(fixture, fixtureRoot);
                }
            }
        }
        assertEquals(EXPECTED_CASES, seen);
    }

    @Test
    void rejectsMalformedInputsBeforeEvaluation() {
        assertThrows(IllegalArgumentException.class, () -> BuildToolCore.evaluateGraph(
                new BuildToolCore.GraphInput(
                        List.of("fixture/a"),
                        List.of(new BuildToolCore.Edge("fixture/a", "fixture/missing")))));
        assertThrows(IllegalArgumentException.class, () -> BuildToolCore.evaluateGraph(
                new BuildToolCore.GraphInput(List.of("fixture/a", "fixture/a"), List.of())));
        assertThrows(IllegalArgumentException.class, () -> BuildToolCore.evaluateGraph(
                new BuildToolCore.GraphInput(
                        List.of("fixture/a"), List.of(new BuildToolCore.Edge("fixture/a", "fixture/a")))));
        assertThrows(IllegalArgumentException.class, () -> BuildToolCore.evaluateGraph(
                new BuildToolCore.GraphInput(
                        List.of("fixture/a", "fixture/b"),
                        List.of(
                                new BuildToolCore.Edge("fixture/a", "fixture/b"),
                                new BuildToolCore.Edge("fixture/a", "fixture/b")))));

        BuildToolCore.RepositoryBoundary boundary = new BuildToolCore.RepositoryBoundary(
                1, "registry", List.of());
        BuildToolCore.DiffSelectionInput mismatched = new BuildToolCore.DiffSelectionInput(
                List.of(new BuildToolCore.PackageSpec(
                        "fixture/a", "a", BuildToolCore.SourceMode.PACKAGE_PREFIX, List.of())),
                List.of(), List.of(), BuildToolCore.UnknownPathPolicy.ERROR,
                List.of("a/file"), "0".repeat(64), boundary);
        IllegalArgumentException error = assertThrows(
                IllegalArgumentException.class,
                () -> BuildToolCore.evaluateDiffSelection(mismatched));
        assertEquals("DIFF_BOUNDARY_DIGEST_MISMATCH", error.getMessage());
        BuildToolCore.DiffSelectionInput oversized = new BuildToolCore.DiffSelectionInput(
                List.of(new BuildToolCore.PackageSpec(
                        "fixture/a", "a", BuildToolCore.SourceMode.STRICT_GLOBS, List.of("*"))),
                List.of(), List.of(), BuildToolCore.UnknownPathPolicy.ERROR,
                List.of("a/" + "x".repeat(100_000)), "", null);
        assertEquals(
                "DIFF_PATH_INVALID",
                assertThrows(
                        IllegalArgumentException.class,
                        () -> BuildToolCore.evaluateDiffSelection(oversized)).getMessage());
        BuildToolCore.DiffSelectionInput cycle = new BuildToolCore.DiffSelectionInput(
                List.of(
                        new BuildToolCore.PackageSpec(
                                "fixture/a", "a", BuildToolCore.SourceMode.PACKAGE_PREFIX, List.of()),
                        new BuildToolCore.PackageSpec(
                                "fixture/b", "b", BuildToolCore.SourceMode.PACKAGE_PREFIX, List.of())),
                List.of(
                        new BuildToolCore.Edge("fixture/a", "fixture/b"),
                        new BuildToolCore.Edge("fixture/b", "fixture/a")),
                List.of(), BuildToolCore.UnknownPathPolicy.ERROR, List.of("a/file"), "", null);
        assertEquals(
                "DIFF_EDGE_CYCLE",
                assertThrows(
                        IllegalArgumentException.class,
                        () -> BuildToolCore.evaluateDiffSelection(cycle)).getMessage());
        BuildToolCore.DiffSelectionInput collidingRoots = new BuildToolCore.DiffSelectionInput(
                List.of(
                        new BuildToolCore.PackageSpec(
                                "fixture/a", "straße", BuildToolCore.SourceMode.PACKAGE_PREFIX, List.of()),
                        new BuildToolCore.PackageSpec(
                                "fixture/b", "strasse", BuildToolCore.SourceMode.PACKAGE_PREFIX, List.of())),
                List.of(), List.of(), BuildToolCore.UnknownPathPolicy.ERROR, List.of("strasse/file"), "", null);
        assertEquals(
                "DIFF_PATH_INVALID",
                assertThrows(
                        IllegalArgumentException.class,
                        () -> BuildToolCore.evaluateDiffSelection(collidingRoots)).getMessage());
        BuildToolCore.DiffSelectionInput descendingGlob = new BuildToolCore.DiffSelectionInput(
                List.of(new BuildToolCore.PackageSpec(
                        "fixture/a", "a", BuildToolCore.SourceMode.STRICT_GLOBS, List.of("[z-a]"))),
                List.of(), List.of(), BuildToolCore.UnknownPathPolicy.ERROR, List.of("a/z"), "", null);
        assertEquals(
                "DIFF_GLOB_INVALID",
                assertThrows(
                        IllegalArgumentException.class,
                        () -> BuildToolCore.evaluateDiffSelection(descendingGlob)).getMessage());
    }

    @Test
    void coversEmptyPartialCycleGlobAndBoundaryEdges() {
        assertEquals(
                new BuildToolCore.GraphResult(List.of(), List.of(), ""),
                BuildToolCore.evaluateGraph(new BuildToolCore.GraphInput(List.of(), List.of())));
        BuildToolCore.GraphResult cycle = BuildToolCore.evaluateGraph(new BuildToolCore.GraphInput(
                List.of("fixture/free", "fixture/a", "fixture/b"),
                List.of(
                        new BuildToolCore.Edge("fixture/a", "fixture/b"),
                        new BuildToolCore.Edge("fixture/b", "fixture/a"))));
        assertEquals("GRAPH_CYCLE", cycle.errorCode());
        assertEquals(List.of(), cycle.levels());

        BuildToolCore.PackageSpec spec = new BuildToolCore.PackageSpec(
                "fixture/p", "p", BuildToolCore.SourceMode.STRICT_GLOBS,
                List.of("src/**/[a-c]*.txt", "literal["));
        BuildToolCore.DiffSelectionResult selected = BuildToolCore.evaluateDiffSelection(
                diffInput(spec, List.of("p/src/deep/bee.txt", "p/BUILD_debug", "p/BUILD")));
        assertEquals(List.of("fixture/p"), selected.changedPackages());
        BuildToolCore.DiffSelectionResult knownButUnmatched = BuildToolCore.evaluateDiffSelection(
                diffInput(spec, List.of("p/src/deep/x.bin")));
        assertEquals(List.of(), knownButUnmatched.changedPackages());
        BuildToolCore.DiffSelectionResult literalBracket = BuildToolCore.evaluateDiffSelection(
                diffInput(spec, List.of("p/literal[")));
        assertEquals(List.of("fixture/p"), literalBracket.changedPackages());
        BuildToolCore.PackageSpec bracketSpec = new BuildToolCore.PackageSpec(
                "fixture/p", "p", BuildToolCore.SourceMode.STRICT_GLOBS, List.of("[]]", "[!]]"));
        assertEquals(
                List.of("fixture/p"),
                BuildToolCore.evaluateDiffSelection(diffInput(bracketSpec, List.of("p/]"))).changedPackages());
        assertEquals(
                List.of("fixture/p"),
                BuildToolCore.evaluateDiffSelection(diffInput(bracketSpec, List.of("p/x"))).changedPackages());

        BuildToolCore.RepositoryBoundary escaped = new BuildToolCore.RepositoryBoundary(
                1,
                "line\nquote\"slash\\tab\t",
                List.of(new BuildToolCore.BoundaryRule(
                        "id", "origin",
                        new BuildToolCore.AppliesTo(List.of("p"), List.of(), List.of()),
                        List.of(new BuildToolCore.BoundaryInput("path", "role", "generated")),
                        "back\bform\freturn\r", "owner")));
        assertEquals(64, escaped.digest().length());
    }

    private static BuildToolCore.DiffSelectionInput diffInput(
            BuildToolCore.PackageSpec spec, List<String> changedPaths) {
        return new BuildToolCore.DiffSelectionInput(
                List.of(spec), List.of(), List.of(), BuildToolCore.UnknownPathPolicy.ERROR,
                changedPaths, "", null);
    }

    private static void assertGraphFixture(JsonNode fixture) {
        JsonNode options = fixture.path("input").path("options");
        BuildToolCore.GraphResult actual = BuildToolCore.evaluateGraph(new BuildToolCore.GraphInput(
                strings(options.path("packages")), edges(options.path("edges"))));
        JsonNode expected = fixture.path("expected");
        if (expected.path("outcome").asText().equals("error")) {
            assertEquals(expected.path("diagnostics").get(0).path("code").asText(), actual.errorCode());
            assertEquals(List.of(), actual.edges());
            assertEquals(List.of(), actual.levels());
        } else {
            assertEquals("", actual.errorCode());
            assertEquals(edges(expected.path("result").path("edges")), actual.edges());
            assertEquals(nestedStrings(expected.path("result").path("levels")), actual.levels());
        }
    }

    private static void assertDiffFixture(JsonNode fixture, Path fixtureRoot) throws IOException {
        JsonNode options = fixture.path("input").path("options");
        String digest = options.path("boundary_sha256").asText("");
        BuildToolCore.RepositoryBoundary boundary = digest.isEmpty()
                ? null
                : boundary(MAPPER.readTree(fixtureRoot.resolve("repository-source-input-boundary.json").toFile()));
        BuildToolCore.DiffSelectionInput input = new BuildToolCore.DiffSelectionInput(
                packages(options.path("packages")),
                edges(options.path("edges")),
                strings(options.path("forced_packages")),
                BuildToolCore.UnknownPathPolicy.valueOf(options.path("unknown_path_policy").asText().toUpperCase()),
                strings(fixture.path("input").path("changed_paths")),
                digest,
                boundary);
        BuildToolCore.DiffSelectionResult actual = BuildToolCore.evaluateDiffSelection(input);
        JsonNode expected = fixture.path("expected");
        if (expected.path("outcome").asText().equals("error")) {
            assertEquals(expected.path("diagnostics").get(0).path("code").asText(), actual.errorCode());
            assertEquals(List.of(), actual.changedPackages());
            assertEquals(List.of(), actual.affectedPackages());
            assertEquals(List.of(), actual.prerequisitePackages());
        } else {
            JsonNode result = expected.path("result");
            assertEquals("", actual.errorCode());
            assertEquals(strings(result.path("changed_packages")), actual.changedPackages());
            assertEquals(strings(result.path("affected_packages")), actual.affectedPackages());
            assertEquals(strings(result.path("prerequisite_packages")), actual.prerequisitePackages());
        }
    }

    private static List<BuildToolCore.PackageSpec> packages(JsonNode nodes) {
        List<BuildToolCore.PackageSpec> result = new ArrayList<>();
        for (JsonNode node : nodes) {
            result.add(new BuildToolCore.PackageSpec(
                    node.path("name").asText(),
                    node.path("rel_path").asText(),
                    BuildToolCore.SourceMode.valueOf(node.path("source_mode").asText().toUpperCase()),
                    strings(node.path("source_globs"))));
        }
        return result;
    }

    private static BuildToolCore.RepositoryBoundary boundary(JsonNode node) {
        List<BuildToolCore.BoundaryRule> rules = new ArrayList<>();
        for (JsonNode rule : node.path("boundaries")) {
            JsonNode applies = rule.path("applies_to");
            List<BuildToolCore.BoundaryInput> inputs = new ArrayList<>();
            for (JsonNode input : rule.path("inputs")) {
                inputs.add(new BuildToolCore.BoundaryInput(
                        input.path("path").asText(), input.path("role").asText(),
                        input.path("generated_component").asText("")));
            }
            rules.add(new BuildToolCore.BoundaryRule(
                    rule.path("id").asText(), rule.path("input_origin").asText(),
                    new BuildToolCore.AppliesTo(
                            strings(applies.path("exact_roots")),
                            strings(applies.path("descendant_roots")),
                            strings(applies.path("excluded_roots"))),
                    inputs, rule.path("reason").asText(), rule.path("owner").asText()));
        }
        return new BuildToolCore.RepositoryBoundary(
                node.path("schema_version").asInt(),
                node.path("language_source_input_registry_sha256").asText(), rules);
    }

    private static List<String> strings(JsonNode nodes) {
        List<String> result = new ArrayList<>();
        for (JsonNode node : nodes) {
            result.add(node.asText());
        }
        return result;
    }

    private static List<List<String>> nestedStrings(JsonNode nodes) {
        List<List<String>> result = new ArrayList<>();
        for (JsonNode node : nodes) {
            result.add(strings(node));
        }
        return result;
    }

    private static List<BuildToolCore.Edge> edges(JsonNode nodes) {
        List<BuildToolCore.Edge> result = new ArrayList<>();
        for (JsonNode node : nodes) {
            result.add(new BuildToolCore.Edge(node.get(0).asText(), node.get(1).asText()));
        }
        return result;
    }

    private static Path repositoryRoot() {
        Path current = Path.of("").toAbsolutePath();
        while (current != null) {
            if (Files.isDirectory(current.resolve("code/specs/fixtures/build-tool-v1"))) {
                return current;
            }
            current = current.getParent();
        }
        throw new IllegalStateException("repository root not found");
    }
}
