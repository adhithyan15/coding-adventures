package com.codingadventures.buildtool

import java.nio.ByteBuffer
import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import java.text.Normalizer
import java.util.ArrayDeque
import java.util.Locale
import java.util.PriorityQueue

enum class SourceMode { PACKAGE_PREFIX, STRICT_GLOBS }

enum class UnknownPathPolicy { ALL, ERROR }

data class Edge(val prerequisite: String, val dependent: String)

data class GraphInput(val packages: List<String>, val edges: List<Edge>)

data class GraphResult(val edges: List<Edge>, val levels: List<List<String>>, val errorCode: String)

data class PackageSpec(
    val name: String,
    val relPath: String,
    val sourceMode: SourceMode,
    val sourceGlobs: List<String>,
)

data class AppliesTo(
    val exactRoots: List<String>,
    val descendantRoots: List<String>,
    val excludedRoots: List<String>,
)

data class BoundaryInput(val path: String, val role: String, val generatedComponent: String)

data class BoundaryRule(
    val id: String,
    val inputOrigin: String,
    val appliesTo: AppliesTo,
    val inputs: List<BoundaryInput>,
    val reason: String,
    val owner: String,
)

data class RepositoryBoundary(
    val schemaVersion: Int,
    val languageSourceInputRegistrySha256: String,
    val boundaries: List<BoundaryRule>,
) {
    fun digest(): String {
        val encoded = canonicalBoundary(this).toByteArray(StandardCharsets.UTF_8)
        val digest = MessageDigest.getInstance("SHA-256")
        digest.update(BOUNDARY_DOMAIN)
        digest.update(ByteBuffer.allocate(Long.SIZE_BYTES).putLong(encoded.size.toLong()).array())
        return digest.digest(encoded).joinToString("") { "%02x".format(it.toInt() and 0xff) }
    }
}

data class DiffSelectionInput(
    val packages: List<PackageSpec>,
    val edges: List<Edge>,
    val forcedPackages: List<String>,
    val unknownPathPolicy: UnknownPathPolicy,
    val changedPaths: List<String>,
    val boundarySha256: String,
    val boundary: RepositoryBoundary?,
)

data class DiffSelectionResult(
    val changedPackages: List<String>,
    val affectedPackages: List<String>,
    val prerequisitePackages: List<String>,
    val errorCode: String,
)

/** Pure graph and diff-selection behavior from the build-tool conformance contract. */
object BuildToolCore {
    /** Evaluate the canonical dependency graph without reading host state. */
    fun evaluateGraph(input: GraphInput): GraphResult {
        val graph = validateGraph(input.packages, input.edges)
        val indegree = graph.names.associateWith { 0 }.toMutableMap()
        graph.edges.forEach { edge -> indegree[edge.dependent] = indegree.getValue(edge.dependent) + 1 }
        var ready = PriorityQueue(UNICODE_ORDER)
        indegree.filterValues { it == 0 }.keys.forEach(ready::add)
        val levels = mutableListOf<List<String>>()
        var visited = 0
        while (ready.isNotEmpty()) {
            val level = buildList { while (ready.isNotEmpty()) add(ready.remove()) }
            levels += level
            val next = PriorityQueue(UNICODE_ORDER)
            level.forEach { name ->
                visited++
                graph.dependents.getValue(name).forEach { dependent ->
                    val degree = indegree.getValue(dependent) - 1
                    indegree[dependent] = degree
                    if (degree == 0) next += dependent
                }
            }
            ready = next
        }
        return if (visited != graph.names.size) {
            GraphResult(emptyList(), emptyList(), "GRAPH_CYCLE")
        } else {
            GraphResult(graph.edges, levels, "")
        }
    }

    /** Evaluate changed, affected, and prerequisite sets without host authority. */
    fun evaluateDiffSelection(input: DiffSelectionInput): DiffSelectionResult {
        val graph = validateGraph(input.packages.map(PackageSpec::name), input.edges)
        requireCode(!isCyclic(graph), "DIFF_EDGE_CYCLE")
        val packages = linkedMapOf<String, PackageSpec>()
        val normalizedRoots = mutableSetOf<String>()
        input.packages.forEach { spec ->
            requireCode(isPackageName(spec.name), "DIFF_PACKAGE_INVALID")
            requireCode(isPortablePath(spec.relPath), "DIFF_PATH_INVALID")
            val rootIdentity = caseFoldIdentity(spec.relPath)
            requireCode(
                normalizedRoots.none {
                    rootIdentity == it || rootIdentity.startsWith("$it/") || it.startsWith("$rootIdentity/")
                },
                "DIFF_PATH_INVALID",
            )
            normalizedRoots += rootIdentity
            requireCode(spec.sourceGlobs.size <= 256 && spec.sourceGlobs.toSet().size == spec.sourceGlobs.size, "DIFF_GLOB_INVALID")
            spec.sourceGlobs.forEach { requireCode(isPortableGlob(it), "DIFF_GLOB_INVALID") }
            requireCode(spec.sourceMode == SourceMode.STRICT_GLOBS || spec.sourceGlobs.isEmpty(), "DIFF_GLOB_INVALID")
            requireCode(packages.put(spec.name, spec) == null, "DIFF_PACKAGE_DUPLICATE")
        }
        requireCode(
            input.forcedPackages.size <= MAX_PACKAGES && input.forcedPackages.toSet().size == input.forcedPackages.size,
            "DIFF_FORCED_PACKAGE_INVALID",
        )
        requireCode(
            input.changedPaths.size <= MAX_PACKAGES && input.changedPaths.toSet().size == input.changedPaths.size,
            "DIFF_PATH_INVALID",
        )
        input.changedPaths.forEach { requireCode(isPortablePath(it), "DIFF_PATH_INVALID") }
        input.forcedPackages.forEach {
            requireCode(packages.containsKey(it), "DIFF_FORCED_PACKAGE_UNKNOWN")
        }

        val boundaryConsumers = mutableMapOf<String, MutableSet<String>>()
        if (input.boundarySha256.isNotEmpty()) {
            requireCode(DIGEST.matches(input.boundarySha256), "DIFF_BOUNDARY_DIGEST_MISMATCH")
            val boundary = input.boundary
                ?: throw IllegalArgumentException("DIFF_BOUNDARY_DIGEST_MISMATCH")
            requireCode(input.boundarySha256 == boundary.digest(), "DIFF_BOUNDARY_DIGEST_MISMATCH")
            input.packages.forEach { spec ->
                boundary.boundaries.forEach { rule ->
                    if (applies(rule.appliesTo, spec.relPath)) {
                        rule.inputs.forEach { boundaryInput ->
                            boundaryConsumers.getOrPut(boundaryInput.path) { mutableSetOf() } += spec.name
                        }
                    }
                }
            }
        } else {
            requireCode(input.boundary == null, "DIFF_BOUNDARY_DIGEST_MISMATCH")
        }

        var remaining = MAX_MATCH_WORK
        input.packages.filter { it.sourceMode == SourceMode.STRICT_GLOBS }.forEach { spec ->
            val patternFactor = spec.sourceGlobs.sumOf { scalarLength(it) + 1L }
            input.changedPaths.filter { inside(it, spec.relPath) }.forEach { path ->
                val relative = relative(path, spec.relPath)
                if (basename(relative) !in BUILD_NAMES) {
                    val pathFactor = scalarLength(relative) + 1L
                    if (patternFactor != 0L && pathFactor > remaining / patternFactor) {
                        return diffError("DIFF_MATCH_LIMIT_EXCEEDED")
                    }
                    remaining -= patternFactor * pathFactor
                }
            }
        }

        val changed = input.forcedPackages.toMutableSet()
        var unknown = false
        input.changedPaths.forEach { path ->
            val consumers = boundaryConsumers[path].orEmpty()
            changed += consumers
            var known = consumers.isNotEmpty()
            input.packages.forEach { spec ->
                if (inside(path, spec.relPath)) {
                    known = true
                    val relative = relative(path, spec.relPath)
                    if (
                        spec.sourceMode == SourceMode.PACKAGE_PREFIX ||
                            basename(relative) in BUILD_NAMES ||
                            spec.sourceGlobs.any { globMatches(it, relative) }
                    ) {
                        changed += spec.name
                    }
                }
            }
            if (!known) unknown = true
        }
        if (unknown) {
            if (input.unknownPathPolicy == UnknownPathPolicy.ERROR) return diffError("DIFF_UNKNOWN_PATH")
            changed += packages.keys
        }

        val affected = closure(changed, graph.dependents)
        val prerequisites = closure(affected, graph.prerequisites) - affected
        return DiffSelectionResult(
            changed.sortedWith(UNICODE_ORDER),
            affected.sortedWith(UNICODE_ORDER),
            prerequisites.sortedWith(UNICODE_ORDER),
            "",
        )
    }

    private fun validateGraph(packages: List<String>, edges: List<Edge>): ValidatedGraph {
        requireCode(packages.size <= MAX_PACKAGES, "GRAPH_PACKAGE_LIMIT_EXCEEDED")
        requireCode(edges.size <= MAX_EDGES, "GRAPH_EDGE_LIMIT_EXCEEDED")
        val names = mutableSetOf<String>()
        packages.forEach { name ->
            requireCode(isPackageName(name), "GRAPH_PACKAGE_INVALID")
            requireCode(names.add(name), "GRAPH_PACKAGE_DUPLICATE")
        }
        val uniqueEdges = mutableSetOf<Edge>()
        val dependents = names.associateWith { mutableListOf<String>() }.toMutableMap()
        val prerequisites = names.associateWith { mutableListOf<String>() }.toMutableMap()
        edges.forEach { edge ->
            requireCode(edge.prerequisite in names && edge.dependent in names, "GRAPH_EDGE_UNKNOWN")
            requireCode(edge.prerequisite != edge.dependent, "GRAPH_EDGE_SELF")
            requireCode(uniqueEdges.add(edge), "GRAPH_EDGE_DUPLICATE")
            dependents.getValue(edge.prerequisite) += edge.dependent
            prerequisites.getValue(edge.dependent) += edge.prerequisite
        }
        dependents.values.forEach { it.sortWith(UNICODE_ORDER) }
        prerequisites.values.forEach { it.sortWith(UNICODE_ORDER) }
        return ValidatedGraph(
            names.toSet(),
            edges.sortedWith(compareBy(UNICODE_ORDER, Edge::prerequisite).thenBy(UNICODE_ORDER, Edge::dependent)),
            dependents,
            prerequisites,
        )
    }

    private fun closure(seeds: Set<String>, adjacency: Map<String, List<String>>): Set<String> {
        val result = seeds.toMutableSet()
        val pending = ArrayDeque(seeds)
        while (pending.isNotEmpty()) {
            adjacency.getValue(pending.removeFirst()).forEach { next ->
                if (result.add(next)) pending.addLast(next)
            }
        }
        return result
    }

    private fun isCyclic(graph: ValidatedGraph): Boolean {
        val indegree = graph.names.associateWith { 0 }.toMutableMap()
        graph.edges.forEach { edge -> indegree[edge.dependent] = indegree.getValue(edge.dependent) + 1 }
        val ready = ArrayDeque(indegree.filterValues { it == 0 }.keys)
        var visited = 0
        while (ready.isNotEmpty()) {
            val name = ready.removeFirst()
            visited++
            graph.dependents.getValue(name).forEach { dependent ->
                val degree = indegree.getValue(dependent) - 1
                indegree[dependent] = degree
                if (degree == 0) ready.addLast(dependent)
            }
        }
        return visited != graph.names.size
    }
}

private const val MAX_PACKAGES = 4096
private const val MAX_EDGES = 16384
private const val MAX_MATCH_WORK = 50_000_000L
private val BOUNDARY_DOMAIN =
    "coding-adventures/build-tool-repository-source-input-boundary/v1\u0000".toByteArray(StandardCharsets.UTF_8)
private val BUILD_NAMES = setOf("BUILD", "BUILD_windows", "BUILD_mac", "BUILD_linux", "BUILD_mac_and_linux")
private val UNICODE_ORDER = Comparator<String>(::compareUnicode)
private val PACKAGE_NAME = Regex("^[a-z0-9][a-z0-9._-]*(/[a-z0-9][a-z0-9._-]*)+$")
private val DIGEST = Regex("^[0-9a-f]{64}$")
private val WINDOWS_RESERVED = setOf(
    "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    "COM¹", "COM²", "COM³", "LPT¹", "LPT²", "LPT³",
)

private data class ValidatedGraph(
    val names: Set<String>,
    val edges: List<Edge>,
    val dependents: Map<String, List<String>>,
    val prerequisites: Map<String, List<String>>,
)

private fun diffError(code: String) = DiffSelectionResult(emptyList(), emptyList(), emptyList(), code)

private fun requireCode(condition: Boolean, code: String) {
    if (!condition) throw IllegalArgumentException(code)
}

private fun applies(appliesTo: AppliesTo, root: String): Boolean =
    root in appliesTo.exactRoots ||
        (root !in appliesTo.excludedRoots && appliesTo.descendantRoots.any { root.startsWith("$it/") })

private fun inside(path: String, root: String) = path == root || path.startsWith("$root/")

private fun relative(path: String, root: String) = if (path == root) "" else path.substring(root.length + 1)

private fun basename(path: String) = path.substringAfterLast('/')

private fun scalarLength(value: String) = value.codePointCount(0, value.length)

private fun isPackageName(value: String) = scalarLength(value) <= 240 && PACKAGE_NAME.matches(value)

private fun isPortablePath(value: String): Boolean {
    if (
        scalarLength(value) !in 1..512 || !Normalizer.isNormalized(value, Normalizer.Form.NFC) ||
        value.startsWith('/') || Regex("^[A-Za-z]:").containsMatchIn(value) || value.contains('\\') ||
        value.contains("//") || hasForbidden(value, path = true)
    ) return false
    return value.split('/').all { segment ->
        segment.isNotEmpty() && segment != "." && segment != ".." &&
            !segment.endsWith(' ') && !segment.endsWith('.') && !reserved(segment)
    }
}

private fun isPortableGlob(value: String): Boolean {
    if (
        scalarLength(value) !in 1..512 || !Normalizer.isNormalized(value, Normalizer.Form.NFC) ||
        value.startsWith('/') || Regex("^[A-Za-z]:").containsMatchIn(value) || value.contains('\\') ||
        value.contains("//") || hasForbidden(value, path = false)
    ) return false
    return value.split('/').all { segment ->
        segment.isNotEmpty() && segment != "." && segment != ".." &&
            !segment.endsWith(' ') && !segment.endsWith('.') &&
            (segment.any { it in "*[]{}" } || !reserved(segment)) &&
            !hasAmbiguousCharacterClass(segment)
    }
}

private fun hasAmbiguousCharacterClass(segment: String): Boolean {
    val characters = segment.codePoints().toArray()
    var index = 0
    while (index < characters.size) {
        if (characters[index] != '['.code) {
            index++
            continue
        }
        var cursor = index + 1
        if (cursor < characters.size && characters[cursor] == '!'.code) cursor++
        var closing = cursor
        if (closing < characters.size && characters[closing] == ']'.code) closing++
        while (closing < characters.size && characters[closing] != ']'.code) closing++
        if (closing == characters.size) {
            index++
            continue
        }
        for (member in cursor until closing - 1) {
            val pair = characters[member] to characters[member + 1]
            if (
                pair == '-'.code to '-'.code || pair == '&'.code to '&'.code ||
                pair == '~'.code to '~'.code || pair == '|'.code to '|'.code
            ) return true
        }
        var member = cursor
        while (member + 2 < closing) {
            if (characters[member + 1] == '-'.code) {
                if (characters[member] > characters[member + 2]) return true
                member += 3
            } else {
                member++
            }
        }
        index = closing + 1
    }
    return false
}

private fun hasForbidden(value: String, path: Boolean): Boolean {
    val forbidden = if (path) "<>:\"|?*" else "<>:\"|?"
    val forbiddenCodePoints = forbidden.codePoints().toArray().toSet()
    return value.codePoints().anyMatch { it < 32 || it in forbiddenCodePoints }
}

private fun reserved(segment: String): Boolean =
    segment.substringBefore('.').uppercase(Locale.ROOT) in WINDOWS_RESERVED

private fun caseFoldIdentity(value: String): String {
    val normalized = Normalizer.normalize(value, Normalizer.Form.NFC)
    val lower = normalized.lowercase(Locale.ROOT)
    val upperLower = normalized.uppercase(Locale.ROOT).lowercase(Locale.ROOT)
    val folded = if (scalarLength(upperLower) > scalarLength(normalized)) upperLower else lower
    return buildString {
        folded.codePoints().forEach { codePoint ->
            when {
                codePoint == 0x03C2 -> appendCodePoint(0x03C3)
                codePoint in 0xAB70..0xABBF -> appendCodePoint(Character.toUpperCase(codePoint))
                codePoint == 0x0131 -> appendCodePoint(codePoint)
                else -> append(String(Character.toChars(codePoint)).uppercase(Locale.ROOT).lowercase(Locale.ROOT))
            }
        }
    }
}

private fun compareUnicode(left: String, right: String): Int {
    val a = left.codePoints().toArray()
    val b = right.codePoints().toArray()
    for (index in 0 until minOf(a.size, b.size)) {
        val compared = a[index].compareTo(b[index])
        if (compared != 0) return compared
    }
    return a.size.compareTo(b.size)
}

private fun globMatches(pattern: String, path: String): Boolean {
    val patterns = pattern.split('/')
    val paths = path.split('/')
    val memo = mutableMapOf<Pair<Int, Int>, Boolean>()
    fun matches(patternIndex: Int, pathIndex: Int): Boolean = memo.getOrPut(patternIndex to pathIndex) {
        when {
            patternIndex == patterns.size -> pathIndex == paths.size
            patterns[patternIndex] == "**" ->
                matches(patternIndex + 1, pathIndex) ||
                    (pathIndex < paths.size && matches(patternIndex, pathIndex + 1))
            else ->
                pathIndex < paths.size && segmentMatches(patterns[patternIndex], paths[pathIndex]) &&
                    matches(patternIndex + 1, pathIndex + 1)
        }
    }
    return matches(0, 0)
}

private fun segmentMatches(pattern: String, value: String): Boolean {
    val p = pattern.codePoints().toArray()
    val v = value.codePoints().toArray()
    val memo = mutableMapOf<Pair<Int, Int>, Boolean>()
    fun matches(patternIndex: Int, valueIndex: Int): Boolean = memo.getOrPut(patternIndex to valueIndex) {
        when {
            patternIndex == p.size -> valueIndex == v.size
            p[patternIndex] == '*'.code ->
                matches(patternIndex + 1, valueIndex) ||
                    (valueIndex < v.size && matches(patternIndex, valueIndex + 1))
            p[patternIndex] == '['.code -> {
                var search = patternIndex + 1
                if (search < p.size && p[search] == '!'.code) search++
                if (search < p.size && p[search] == ']'.code) search++
                val closing = (search until p.size).firstOrNull { p[it] == ']'.code } ?: -1
                if (closing < 0) {
                    valueIndex < v.size && p[patternIndex] == v[valueIndex] &&
                        matches(patternIndex + 1, valueIndex + 1)
                } else {
                    valueIndex < v.size && classMatches(p, patternIndex + 1, closing, v[valueIndex]) &&
                        matches(closing + 1, valueIndex + 1)
                }
            }
            else -> valueIndex < v.size && p[patternIndex] == v[valueIndex] &&
                matches(patternIndex + 1, valueIndex + 1)
        }
    }
    return matches(0, 0)
}

private fun classMatches(pattern: IntArray, start: Int, end: Int, value: Int): Boolean {
    val negated = start < end && pattern[start] == '!'.code
    var index = if (negated) start + 1 else start
    var matched = false
    while (index < end) {
        if (index + 2 < end && pattern[index + 1] == '-'.code) {
            matched = matched || value in pattern[index]..pattern[index + 2]
            index += 3
        } else {
            matched = matched || value == pattern[index]
            index++
        }
    }
    return if (negated) !matched else matched
}

private fun canonicalBoundary(boundary: RepositoryBoundary): String = buildString {
    append("{\"boundaries\":[")
    boundary.boundaries.forEachIndexed { index, rule ->
        if (index > 0) append(',')
        append("{\"applies_to\":{\"descendant_roots\":[")
        appendStrings(rule.appliesTo.descendantRoots)
        append("],\"exact_roots\":[")
        appendStrings(rule.appliesTo.exactRoots)
        append("],\"excluded_roots\":[")
        appendStrings(rule.appliesTo.excludedRoots)
        append("]},\"id\":")
        appendJsonString(rule.id)
        append(",\"input_origin\":")
        appendJsonString(rule.inputOrigin)
        append(",\"inputs\":[")
        rule.inputs.forEachIndexed { inputIndex, input ->
            if (inputIndex > 0) append(',')
            append('{')
            if (input.generatedComponent.isNotEmpty()) {
                append("\"generated_component\":")
                appendJsonString(input.generatedComponent)
                append(',')
            }
            append("\"path\":")
            appendJsonString(input.path)
            append(",\"role\":")
            appendJsonString(input.role)
            append('}')
        }
        append("],\"owner\":")
        appendJsonString(rule.owner)
        append(",\"reason\":")
        appendJsonString(rule.reason)
        append('}')
    }
    append("],\"language_source_input_registry_sha256\":")
    appendJsonString(boundary.languageSourceInputRegistrySha256)
    append(",\"schema_version\":${boundary.schemaVersion}}")
}

private fun StringBuilder.appendStrings(values: List<String>) {
    values.forEachIndexed { index, value ->
        if (index > 0) append(',')
        appendJsonString(value)
    }
}

private fun StringBuilder.appendJsonString(value: String) {
    append('"')
    value.codePoints().forEach { codePoint ->
        when (codePoint) {
            '"'.code -> append("\\\"")
            '\\'.code -> append("\\\\")
            '\b'.code -> append("\\b")
            '\u000c'.code -> append("\\f")
            '\n'.code -> append("\\n")
            '\r'.code -> append("\\r")
            '\t'.code -> append("\\t")
            else -> if (codePoint < 0x20) append("\\u%04x".format(codePoint)) else appendCodePoint(codePoint)
        }
    }
    append('"')
}
