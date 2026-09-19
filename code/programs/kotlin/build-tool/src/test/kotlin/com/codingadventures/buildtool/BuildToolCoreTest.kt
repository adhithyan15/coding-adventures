package com.codingadventures.buildtool

import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import java.nio.file.Files
import java.nio.file.Path
import kotlin.io.path.extension
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class BuildToolCoreTest {
    private val mapper = ObjectMapper()

    @Test
    fun consumesEverySharedGraphAndDiffFixture() {
        val fixtureRoot = repositoryRoot().resolve("code/specs/fixtures/build-tool-v1")
        val seen = mutableSetOf<String>()
        Files.list(fixtureRoot.resolve("cases")).use { paths ->
            paths.filter { it.extension == "json" }.sorted().forEach { path ->
                val fixture = mapper.readTree(path.toFile())
                when (fixture.path("domain").asText()) {
                    "graph" -> {
                        seen += fixture.path("id").asText()
                        assertGraphFixture(fixture)
                    }
                    "diff_selection" -> {
                        seen += fixture.path("id").asText()
                        assertDiffFixture(fixture, fixtureRoot)
                    }
                }
            }
        }
        assertEquals(EXPECTED_CASES, seen)
    }

    @Test
    fun rejectsMalformedInputsBeforeEvaluation() {
        assertFailsWith<IllegalArgumentException> {
            BuildToolCore.evaluateGraph(
                GraphInput(listOf("fixture/a"), listOf(Edge("fixture/a", "fixture/missing"))),
            )
        }
        assertFailsWith<IllegalArgumentException> {
            BuildToolCore.evaluateGraph(GraphInput(listOf("fixture/a", "fixture/a"), emptyList()))
        }
        assertFailsWith<IllegalArgumentException> {
            BuildToolCore.evaluateGraph(GraphInput(listOf("fixture/a"), listOf(Edge("fixture/a", "fixture/a"))))
        }
        assertFailsWith<IllegalArgumentException> {
            BuildToolCore.evaluateGraph(
                GraphInput(
                    listOf("fixture/a", "fixture/b"),
                    listOf(Edge("fixture/a", "fixture/b"), Edge("fixture/a", "fixture/b")),
                ),
            )
        }
        val boundary = RepositoryBoundary(1, "registry", emptyList())
        val error = assertFailsWith<IllegalArgumentException> {
            BuildToolCore.evaluateDiffSelection(
                DiffSelectionInput(
                    listOf(PackageSpec("fixture/a", "a", SourceMode.PACKAGE_PREFIX, emptyList())),
                    emptyList(), emptyList(), UnknownPathPolicy.ERROR,
                    listOf("a/file"), "0".repeat(64), boundary,
                ),
            )
        }
        assertEquals("DIFF_BOUNDARY_DIGEST_MISMATCH", error.message)
        val oversized = DiffSelectionInput(
            listOf(PackageSpec("fixture/a", "a", SourceMode.STRICT_GLOBS, listOf("*"))),
            emptyList(), emptyList(), UnknownPathPolicy.ERROR,
            listOf("a/" + "x".repeat(100_000)), "", null,
        )
        assertEquals(
            "DIFF_PATH_INVALID",
            assertFailsWith<IllegalArgumentException> { BuildToolCore.evaluateDiffSelection(oversized) }.message,
        )
        val cycle = DiffSelectionInput(
            listOf(
                PackageSpec("fixture/a", "a", SourceMode.PACKAGE_PREFIX, emptyList()),
                PackageSpec("fixture/b", "b", SourceMode.PACKAGE_PREFIX, emptyList()),
            ),
            listOf(Edge("fixture/a", "fixture/b"), Edge("fixture/b", "fixture/a")),
            emptyList(), UnknownPathPolicy.ERROR, listOf("a/file"), "", null,
        )
        assertEquals(
            "DIFF_EDGE_CYCLE",
            assertFailsWith<IllegalArgumentException> { BuildToolCore.evaluateDiffSelection(cycle) }.message,
        )
        val collidingRoots = DiffSelectionInput(
            listOf(
                PackageSpec("fixture/a", "straße", SourceMode.PACKAGE_PREFIX, emptyList()),
                PackageSpec("fixture/b", "strasse", SourceMode.PACKAGE_PREFIX, emptyList()),
            ),
            emptyList(), emptyList(), UnknownPathPolicy.ERROR, listOf("strasse/file"), "", null,
        )
        assertEquals(
            "DIFF_PATH_INVALID",
            assertFailsWith<IllegalArgumentException> { BuildToolCore.evaluateDiffSelection(collidingRoots) }.message,
        )
        val descendingGlob = DiffSelectionInput(
            listOf(PackageSpec("fixture/a", "a", SourceMode.STRICT_GLOBS, listOf("[z-a]"))),
            emptyList(), emptyList(), UnknownPathPolicy.ERROR, listOf("a/z"), "", null,
        )
        assertEquals(
            "DIFF_GLOB_INVALID",
            assertFailsWith<IllegalArgumentException> { BuildToolCore.evaluateDiffSelection(descendingGlob) }.message,
        )
    }

    @Test
    fun coversEmptyPartialCycleGlobAndBoundaryEdges() {
        assertEquals(GraphResult(emptyList(), emptyList(), ""), BuildToolCore.evaluateGraph(GraphInput(emptyList(), emptyList())))
        val cycle = BuildToolCore.evaluateGraph(
            GraphInput(
                listOf("fixture/free", "fixture/a", "fixture/b"),
                listOf(Edge("fixture/a", "fixture/b"), Edge("fixture/b", "fixture/a")),
            ),
        )
        assertEquals("GRAPH_CYCLE", cycle.errorCode)
        assertEquals(emptyList(), cycle.levels)

        val spec = PackageSpec(
            "fixture/p", "p", SourceMode.STRICT_GLOBS,
            listOf("src/**/[a-c]*.txt", "literal["),
        )
        val selected = BuildToolCore.evaluateDiffSelection(
            diffInput(spec, listOf("p/src/deep/bee.txt", "p/BUILD_debug", "p/BUILD")),
        )
        assertEquals(listOf("fixture/p"), selected.changedPackages)
        val knownButUnmatched = BuildToolCore.evaluateDiffSelection(diffInput(spec, listOf("p/src/deep/x.bin")))
        assertEquals(emptyList(), knownButUnmatched.changedPackages)
        val literalBracket = BuildToolCore.evaluateDiffSelection(diffInput(spec, listOf("p/literal[")))
        assertEquals(listOf("fixture/p"), literalBracket.changedPackages)
        val bracketSpec = PackageSpec("fixture/p", "p", SourceMode.STRICT_GLOBS, listOf("[]]", "[!]]"))
        assertEquals(
            listOf("fixture/p"),
            BuildToolCore.evaluateDiffSelection(diffInput(bracketSpec, listOf("p/]"))).changedPackages,
        )
        assertEquals(
            listOf("fixture/p"),
            BuildToolCore.evaluateDiffSelection(diffInput(bracketSpec, listOf("p/x"))).changedPackages,
        )

        val escaped = RepositoryBoundary(
            1,
            "line\nquote\"slash\\tab\t",
            listOf(
                BoundaryRule(
                    "id", "origin", AppliesTo(listOf("p"), emptyList(), emptyList()),
                    listOf(BoundaryInput("path", "role", "generated")),
                    "back\bform\u000creturn\r", "owner",
                ),
            ),
        )
        assertEquals(64, escaped.digest().length)
    }

    private fun diffInput(spec: PackageSpec, changedPaths: List<String>) = DiffSelectionInput(
        listOf(spec), emptyList(), emptyList(), UnknownPathPolicy.ERROR,
        changedPaths, "", null,
    )

    private fun assertGraphFixture(fixture: JsonNode) {
        val options = fixture.path("input").path("options")
        val actual = BuildToolCore.evaluateGraph(
            GraphInput(strings(options.path("packages")), edges(options.path("edges"))),
        )
        val expected = fixture.path("expected")
        if (expected.path("outcome").asText() == "error") {
            assertEquals(expected.path("diagnostics")[0].path("code").asText(), actual.errorCode)
            assertEquals(emptyList(), actual.edges)
            assertEquals(emptyList(), actual.levels)
        } else {
            assertEquals("", actual.errorCode)
            assertEquals(edges(expected.path("result").path("edges")), actual.edges)
            assertEquals(expected.path("result").path("levels").map(::strings), actual.levels)
        }
    }

    private fun assertDiffFixture(fixture: JsonNode, fixtureRoot: Path) {
        val options = fixture.path("input").path("options")
        val digest = options.path("boundary_sha256").asText("")
        val boundary = if (digest.isEmpty()) {
            null
        } else {
            boundary(mapper.readTree(fixtureRoot.resolve("repository-source-input-boundary.json").toFile()))
        }
        val input = DiffSelectionInput(
            packages(options.path("packages")),
            edges(options.path("edges")),
            strings(options.path("forced_packages")),
            UnknownPathPolicy.valueOf(options.path("unknown_path_policy").asText().uppercase()),
            strings(fixture.path("input").path("changed_paths")),
            digest,
            boundary,
        )
        val actual = BuildToolCore.evaluateDiffSelection(input)
        val expected = fixture.path("expected")
        if (expected.path("outcome").asText() == "error") {
            assertEquals(expected.path("diagnostics")[0].path("code").asText(), actual.errorCode)
            assertEquals(emptyList(), actual.changedPackages)
            assertEquals(emptyList(), actual.affectedPackages)
            assertEquals(emptyList(), actual.prerequisitePackages)
        } else {
            val result = expected.path("result")
            assertEquals("", actual.errorCode)
            assertEquals(strings(result.path("changed_packages")), actual.changedPackages)
            assertEquals(strings(result.path("affected_packages")), actual.affectedPackages)
            assertEquals(strings(result.path("prerequisite_packages")), actual.prerequisitePackages)
        }
    }

    private fun packages(nodes: JsonNode): List<PackageSpec> = nodes.map { node ->
        PackageSpec(
            node.path("name").asText(),
            node.path("rel_path").asText(),
            SourceMode.valueOf(node.path("source_mode").asText().uppercase()),
            strings(node.path("source_globs")),
        )
    }

    private fun boundary(node: JsonNode): RepositoryBoundary = RepositoryBoundary(
        node.path("schema_version").asInt(),
        node.path("language_source_input_registry_sha256").asText(),
        node.path("boundaries").map { rule ->
            val applies = rule.path("applies_to")
            BoundaryRule(
                rule.path("id").asText(),
                rule.path("input_origin").asText(),
                AppliesTo(
                    strings(applies.path("exact_roots")),
                    strings(applies.path("descendant_roots")),
                    strings(applies.path("excluded_roots")),
                ),
                rule.path("inputs").map { input ->
                    BoundaryInput(
                        input.path("path").asText(),
                        input.path("role").asText(),
                        input.path("generated_component").asText(""),
                    )
                },
                rule.path("reason").asText(),
                rule.path("owner").asText(),
            )
        },
    )

    private fun strings(nodes: JsonNode): List<String> = nodes.map(JsonNode::asText)

    private fun edges(nodes: JsonNode): List<Edge> = nodes.map { Edge(it[0].asText(), it[1].asText()) }

    private fun repositoryRoot(): Path {
        var current: Path? = Path.of("").toAbsolutePath()
        while (current != null) {
            if (Files.isDirectory(current.resolve("code/specs/fixtures/build-tool-v1"))) return current
            current = current.parent
        }
        error("repository root not found")
    }

    companion object {
        private val EXPECTED_CASES = setOf(
            "diff-selection/forced-package",
            "diff-selection/exact-build-fronts",
            "diff-selection/known-unmatched-near-build",
            "diff-selection/match-work-at-limit",
            "diff-selection/match-work-over-limit",
            "diff-selection/package-prefix",
            "diff-selection/repository-boundary-reverse-index",
            "diff-selection/strict-glob-character-classes",
            "diff-selection/transitive-package-change",
            "diff-selection/unknown-path-all",
            "diff-selection/unknown-path-error",
            "graph/canonical-edge-order",
            "graph/chain",
            "graph/cycle",
            "graph/diamond",
            "graph/empty",
            "graph/isolated",
            "graph/multiple-components",
            "graph/partial-cycle-no-output",
        )
    }
}
