package com.codingadventures.buildtool;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.text.Normalizer;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.PriorityQueue;
import java.util.Set;
import java.util.regex.Pattern;

/** Pure graph and diff-selection behavior from the build-tool conformance contract. */
public final class BuildToolCore {
    private static final int MAX_PACKAGES = 4096;
    private static final int MAX_EDGES = 16384;
    private static final long MAX_MATCH_WORK = 50_000_000L;
    private static final byte[] BOUNDARY_DOMAIN =
            "coding-adventures/build-tool-repository-source-input-boundary/v1\0"
                    .getBytes(StandardCharsets.UTF_8);
    private static final Set<String> BUILD_NAMES = Set.of(
            "BUILD", "BUILD_windows", "BUILD_mac", "BUILD_linux", "BUILD_mac_and_linux");
    private static final Comparator<String> UNICODE_ORDER = BuildToolCore::compareUnicode;
    private static final Comparator<Edge> EDGE_ORDER = Comparator
            .comparing(Edge::prerequisite, UNICODE_ORDER)
            .thenComparing(Edge::dependent, UNICODE_ORDER);
    private static final Pattern PACKAGE_NAME =
            Pattern.compile("^[a-z0-9][a-z0-9._-]*(/[a-z0-9][a-z0-9._-]*)+$");
    private static final Pattern DIGEST = Pattern.compile("^[0-9a-f]{64}$");
    private static final Set<String> WINDOWS_RESERVED = Set.of(
            "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$",
            "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
            "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
            "COM¹", "COM²", "COM³", "LPT¹", "LPT²", "LPT³");

    private BuildToolCore() {}

    public enum SourceMode { PACKAGE_PREFIX, STRICT_GLOBS }

    public enum UnknownPathPolicy { ALL, ERROR }

    public record Edge(String prerequisite, String dependent) {}

    public record GraphInput(List<String> packages, List<Edge> edges) {
        public GraphInput {
            packages = List.copyOf(packages);
            edges = List.copyOf(edges);
        }
    }

    public record GraphResult(List<Edge> edges, List<List<String>> levels, String errorCode) {
        public GraphResult {
            edges = List.copyOf(edges);
            levels = levels.stream().map(List::copyOf).toList();
        }
    }

    public record PackageSpec(String name, String relPath, SourceMode sourceMode, List<String> sourceGlobs) {
        public PackageSpec {
            sourceGlobs = List.copyOf(sourceGlobs);
        }
    }

    public record AppliesTo(
            List<String> exactRoots, List<String> descendantRoots, List<String> excludedRoots) {
        public AppliesTo {
            exactRoots = List.copyOf(exactRoots);
            descendantRoots = List.copyOf(descendantRoots);
            excludedRoots = List.copyOf(excludedRoots);
        }
    }

    public record BoundaryInput(String path, String role, String generatedComponent) {}

    public record BoundaryRule(
            String id,
            String inputOrigin,
            AppliesTo appliesTo,
            List<BoundaryInput> inputs,
            String reason,
            String owner) {
        public BoundaryRule {
            inputs = List.copyOf(inputs);
        }
    }

    public record RepositoryBoundary(
            int schemaVersion, String languageSourceInputRegistrySha256, List<BoundaryRule> boundaries) {
        public RepositoryBoundary {
            boundaries = List.copyOf(boundaries);
        }

        public String digest() {
            byte[] encoded = canonicalBoundary(this).getBytes(StandardCharsets.UTF_8);
            try {
                MessageDigest digest = MessageDigest.getInstance("SHA-256");
                digest.update(BOUNDARY_DOMAIN);
                digest.update(ByteBuffer.allocate(Long.BYTES).putLong(encoded.length).array());
                return toHex(digest.digest(encoded));
            } catch (NoSuchAlgorithmException exception) {
                throw new IllegalStateException("SHA-256 is unavailable", exception);
            }
        }
    }

    public record DiffSelectionInput(
            List<PackageSpec> packages,
            List<Edge> edges,
            List<String> forcedPackages,
            UnknownPathPolicy unknownPathPolicy,
            List<String> changedPaths,
            String boundarySha256,
            RepositoryBoundary boundary) {
        public DiffSelectionInput {
            packages = List.copyOf(packages);
            edges = List.copyOf(edges);
            forcedPackages = List.copyOf(forcedPackages);
            changedPaths = List.copyOf(changedPaths);
        }
    }

    public record DiffSelectionResult(
            List<String> changedPackages,
            List<String> affectedPackages,
            List<String> prerequisitePackages,
            String errorCode) {
        public DiffSelectionResult {
            changedPackages = List.copyOf(changedPackages);
            affectedPackages = List.copyOf(affectedPackages);
            prerequisitePackages = List.copyOf(prerequisitePackages);
        }
    }

    /** Evaluate the canonical dependency graph without reading host state. */
    public static GraphResult evaluateGraph(GraphInput input) {
        ValidatedGraph graph = validateGraph(input.packages(), input.edges());
        Map<String, Integer> indegree = new HashMap<>();
        for (String name : graph.names()) {
            indegree.put(name, 0);
        }
        for (Edge edge : graph.edges()) {
            indegree.put(edge.dependent(), indegree.get(edge.dependent()) + 1);
        }

        PriorityQueue<String> ready = new PriorityQueue<>(UNICODE_ORDER);
        for (Map.Entry<String, Integer> entry : indegree.entrySet()) {
            if (entry.getValue() == 0) ready.add(entry.getKey());
        }
        List<List<String>> levels = new ArrayList<>();
        int visited = 0;
        while (!ready.isEmpty()) {
            List<String> level = new ArrayList<>();
            while (!ready.isEmpty()) level.add(ready.remove());
            levels.add(level);
            PriorityQueue<String> next = new PriorityQueue<>(UNICODE_ORDER);
            for (String name : level) {
                visited++;
                for (String dependent : graph.dependents().get(name)) {
                    int degree = indegree.get(dependent) - 1;
                    indegree.put(dependent, degree);
                    if (degree == 0) next.add(dependent);
                }
            }
            ready = next;
        }
        if (visited != graph.names().size()) {
            return new GraphResult(List.of(), List.of(), "GRAPH_CYCLE");
        }
        return new GraphResult(graph.edges(), levels, "");
    }

    /** Evaluate changed, affected, and prerequisite sets without host authority. */
    public static DiffSelectionResult evaluateDiffSelection(DiffSelectionInput input) {
        List<String> names = input.packages().stream().map(PackageSpec::name).toList();
        ValidatedGraph graph = validateGraph(names, input.edges());
        require(!isCyclic(graph), "DIFF_EDGE_CYCLE");
        Map<String, PackageSpec> packages = new LinkedHashMap<>();
        Map<String, String> normalizedRoots = new HashMap<>();
        for (PackageSpec spec : input.packages()) {
            require(isPackageName(spec.name()), "DIFF_PACKAGE_INVALID");
            require(isPortablePath(spec.relPath()), "DIFF_PATH_INVALID");
            String rootIdentity = caseFoldIdentity(spec.relPath());
            for (Map.Entry<String, String> entry : normalizedRoots.entrySet()) {
                require(!rootIdentity.equals(entry.getKey())
                        && !rootIdentity.startsWith(entry.getKey() + "/")
                        && !entry.getKey().startsWith(rootIdentity + "/"), "DIFF_PATH_INVALID");
            }
            normalizedRoots.put(rootIdentity, spec.relPath());
            require(spec.sourceGlobs().size() <= 256
                    && new HashSet<>(spec.sourceGlobs()).size() == spec.sourceGlobs().size(),
                    "DIFF_GLOB_INVALID");
            for (String glob : spec.sourceGlobs()) {
                require(isPortableGlob(glob), "DIFF_GLOB_INVALID");
            }
            require(spec.sourceMode() == SourceMode.STRICT_GLOBS || spec.sourceGlobs().isEmpty(),
                    "DIFF_GLOB_INVALID");
            require(packages.put(spec.name(), spec) == null, "DIFF_PACKAGE_DUPLICATE");
        }
        require(input.forcedPackages().size() <= MAX_PACKAGES
                && new HashSet<>(input.forcedPackages()).size() == input.forcedPackages().size(),
                "DIFF_FORCED_PACKAGE_INVALID");
        require(input.changedPaths().size() <= MAX_PACKAGES
                && new HashSet<>(input.changedPaths()).size() == input.changedPaths().size(),
                "DIFF_PATH_INVALID");
        for (String path : input.changedPaths()) {
            require(isPortablePath(path), "DIFF_PATH_INVALID");
        }
        for (String forced : input.forcedPackages()) {
            require(packages.containsKey(forced), "DIFF_FORCED_PACKAGE_UNKNOWN");
        }

        Map<String, Set<String>> boundaryConsumers = new HashMap<>();
        if (!input.boundarySha256().isEmpty()) {
            require(DIGEST.matcher(input.boundarySha256()).matches(), "DIFF_BOUNDARY_DIGEST_MISMATCH");
            require(input.boundary() != null
                    && input.boundarySha256().equals(input.boundary().digest()),
                    "DIFF_BOUNDARY_DIGEST_MISMATCH");
            for (PackageSpec spec : input.packages()) {
                for (BoundaryRule rule : input.boundary().boundaries()) {
                    if (!applies(rule.appliesTo(), spec.relPath())) continue;
                    for (BoundaryInput boundaryInput : rule.inputs()) {
                        boundaryConsumers.computeIfAbsent(
                                boundaryInput.path(), ignored -> new HashSet<>()).add(spec.name());
                    }
                }
            }
        } else {
            require(input.boundary() == null, "DIFF_BOUNDARY_DIGEST_MISMATCH");
        }

        long remaining = MAX_MATCH_WORK;
        for (PackageSpec spec : input.packages()) {
            if (spec.sourceMode() != SourceMode.STRICT_GLOBS) continue;
            long patternFactor = spec.sourceGlobs().stream()
                    .mapToLong(pattern -> scalarLength(pattern) + 1L).sum();
            for (String path : input.changedPaths()) {
                if (!inside(path, spec.relPath())) continue;
                String relative = relative(path, spec.relPath());
                if (BUILD_NAMES.contains(basename(relative))) continue;
                long pathFactor = scalarLength(relative) + 1L;
                if (patternFactor != 0 && pathFactor > remaining / patternFactor) {
                    return diffError("DIFF_MATCH_LIMIT_EXCEEDED");
                }
                remaining -= patternFactor * pathFactor;
            }
        }

        Set<String> changed = new HashSet<>(input.forcedPackages());
        boolean unknown = false;
        for (String path : input.changedPaths()) {
            Set<String> consumers = boundaryConsumers.getOrDefault(path, Set.of());
            changed.addAll(consumers);
            boolean known = !consumers.isEmpty();
            for (PackageSpec spec : input.packages()) {
                if (!inside(path, spec.relPath())) continue;
                known = true;
                String relative = relative(path, spec.relPath());
                if (spec.sourceMode() == SourceMode.PACKAGE_PREFIX
                        || BUILD_NAMES.contains(basename(relative))
                        || spec.sourceGlobs().stream().anyMatch(pattern -> globMatches(pattern, relative))) {
                    changed.add(spec.name());
                }
            }
            if (!known) unknown = true;
        }
        if (unknown) {
            if (input.unknownPathPolicy() == UnknownPathPolicy.ERROR) {
                return diffError("DIFF_UNKNOWN_PATH");
            }
            changed.addAll(packages.keySet());
        }

        Set<String> affected = closure(changed, graph.dependents());
        Set<String> closed = closure(affected, graph.prerequisites());
        closed.removeAll(affected);
        return new DiffSelectionResult(sorted(changed), sorted(affected), sorted(closed), "");
    }

    private static ValidatedGraph validateGraph(List<String> packages, List<Edge> edges) {
        require(packages.size() <= MAX_PACKAGES, "GRAPH_PACKAGE_LIMIT_EXCEEDED");
        require(edges.size() <= MAX_EDGES, "GRAPH_EDGE_LIMIT_EXCEEDED");
        Set<String> names = new HashSet<>();
        for (String name : packages) {
            require(isPackageName(name), "GRAPH_PACKAGE_INVALID");
            require(names.add(name), "GRAPH_PACKAGE_DUPLICATE");
        }
        Set<Edge> uniqueEdges = new HashSet<>();
        Map<String, List<String>> dependents = new HashMap<>();
        Map<String, List<String>> prerequisites = new HashMap<>();
        for (String name : names) {
            dependents.put(name, new ArrayList<>());
            prerequisites.put(name, new ArrayList<>());
        }
        for (Edge edge : edges) {
            require(names.contains(edge.prerequisite()) && names.contains(edge.dependent()),
                    "GRAPH_EDGE_UNKNOWN");
            require(!edge.prerequisite().equals(edge.dependent()), "GRAPH_EDGE_SELF");
            require(uniqueEdges.add(edge), "GRAPH_EDGE_DUPLICATE");
            dependents.get(edge.prerequisite()).add(edge.dependent());
            prerequisites.get(edge.dependent()).add(edge.prerequisite());
        }
        dependents.values().forEach(values -> values.sort(UNICODE_ORDER));
        prerequisites.values().forEach(values -> values.sort(UNICODE_ORDER));
        List<Edge> canonicalEdges = new ArrayList<>(edges);
        canonicalEdges.sort(EDGE_ORDER);
        return new ValidatedGraph(Set.copyOf(names), List.copyOf(canonicalEdges), dependents, prerequisites);
    }

    private static Set<String> closure(Set<String> seeds, Map<String, List<String>> adjacency) {
        Set<String> result = new HashSet<>(seeds);
        ArrayDeque<String> pending = new ArrayDeque<>(seeds);
        while (!pending.isEmpty()) {
            for (String next : adjacency.get(pending.removeFirst())) {
                if (result.add(next)) pending.addLast(next);
            }
        }
        return result;
    }

    private static boolean isCyclic(ValidatedGraph graph) {
        Map<String, Integer> indegree = new HashMap<>();
        for (String name : graph.names()) indegree.put(name, 0);
        for (Edge edge : graph.edges()) {
            indegree.put(edge.dependent(), indegree.get(edge.dependent()) + 1);
        }
        ArrayDeque<String> ready = new ArrayDeque<>();
        indegree.forEach((name, degree) -> {
            if (degree == 0) ready.addLast(name);
        });
        int visited = 0;
        while (!ready.isEmpty()) {
            String name = ready.removeFirst();
            visited++;
            for (String dependent : graph.dependents().get(name)) {
                int degree = indegree.get(dependent) - 1;
                indegree.put(dependent, degree);
                if (degree == 0) ready.addLast(dependent);
            }
        }
        return visited != graph.names().size();
    }

    private static boolean applies(AppliesTo appliesTo, String root) {
        if (appliesTo.exactRoots().contains(root)) return true;
        if (appliesTo.excludedRoots().contains(root)) return false;
        return appliesTo.descendantRoots().stream().anyMatch(parent -> root.startsWith(parent + "/"));
    }

    private static boolean inside(String path, String root) {
        return path.equals(root) || path.startsWith(root + "/");
    }

    private static String relative(String path, String root) {
        if (path.equals(root)) return "";
        return path.substring(root.length() + 1);
    }

    private static String basename(String path) {
        int slash = path.lastIndexOf('/');
        return path.substring(slash + 1);
    }

    private static boolean globMatches(String pattern, String path) {
        String[] patterns = pattern.split("/", -1);
        String[] paths = path.split("/", -1);
        return globSegments(patterns, paths, 0, 0, new HashMap<>());
    }

    private static boolean globSegments(
            String[] patterns, String[] paths, int patternIndex, int pathIndex, Map<Long, Boolean> memo) {
        long key = ((long) patternIndex << 32) | (pathIndex & 0xffffffffL);
        Boolean known = memo.get(key);
        if (known != null) return known;
        boolean result;
        if (patternIndex == patterns.length) {
            result = pathIndex == paths.length;
        } else if (patterns[patternIndex].equals("**")) {
            result = globSegments(patterns, paths, patternIndex + 1, pathIndex, memo)
                    || (pathIndex < paths.length
                            && globSegments(patterns, paths, patternIndex, pathIndex + 1, memo));
        } else {
            result = pathIndex < paths.length
                    && segmentMatches(patterns[patternIndex], paths[pathIndex])
                    && globSegments(patterns, paths, patternIndex + 1, pathIndex + 1, memo);
        }
        memo.put(key, result);
        return result;
    }

    private static boolean segmentMatches(String pattern, String value) {
        int[] p = pattern.codePoints().toArray();
        int[] v = value.codePoints().toArray();
        return segmentMatches(p, v, 0, 0, new HashMap<>());
    }

    private static boolean segmentMatches(
            int[] pattern, int[] value, int patternIndex, int valueIndex, Map<Long, Boolean> memo) {
        long key = ((long) patternIndex << 32) | (valueIndex & 0xffffffffL);
        Boolean known = memo.get(key);
        if (known != null) return known;
        boolean result;
        if (patternIndex == pattern.length) {
            result = valueIndex == value.length;
        } else if (pattern[patternIndex] == '*') {
            result = segmentMatches(pattern, value, patternIndex + 1, valueIndex, memo)
                    || (valueIndex < value.length
                            && segmentMatches(pattern, value, patternIndex, valueIndex + 1, memo));
        } else if (pattern[patternIndex] == '[') {
            int closing = closingBracket(pattern, patternIndex + 1);
            if (closing < 0) {
                result = valueIndex < value.length && pattern[patternIndex] == value[valueIndex]
                        && segmentMatches(pattern, value, patternIndex + 1, valueIndex + 1, memo);
            } else {
                result = valueIndex < value.length
                        && classMatches(pattern, patternIndex + 1, closing, value[valueIndex])
                        && segmentMatches(pattern, value, closing + 1, valueIndex + 1, memo);
            }
        } else {
            result = valueIndex < value.length && pattern[patternIndex] == value[valueIndex]
                    && segmentMatches(pattern, value, patternIndex + 1, valueIndex + 1, memo);
        }
        memo.put(key, result);
        return result;
    }

    private static int closingBracket(int[] pattern, int start) {
        int search = start;
        if (search < pattern.length && pattern[search] == '!') search++;
        if (search < pattern.length && pattern[search] == ']') search++;
        for (int index = search; index < pattern.length; index++) {
            if (pattern[index] == ']') return index;
        }
        return -1;
    }

    private static boolean classMatches(int[] pattern, int start, int end, int value) {
        boolean negated = start < end && pattern[start] == '!';
        int index = negated ? start + 1 : start;
        boolean matched = false;
        while (index < end) {
            if (index + 2 < end && pattern[index + 1] == '-') {
                matched |= value >= pattern[index] && value <= pattern[index + 2];
                index += 3;
            } else {
                matched |= value == pattern[index++];
            }
        }
        return negated ? !matched : matched;
    }

    private static int scalarLength(String value) {
        return value.codePointCount(0, value.length());
    }

    private static boolean isPackageName(String value) {
        return value != null && scalarLength(value) <= 240 && PACKAGE_NAME.matcher(value).matches();
    }

    private static boolean isPortablePath(String value) {
        if (value == null || scalarLength(value) < 1 || scalarLength(value) > 512
                || !Normalizer.isNormalized(value, Normalizer.Form.NFC)
                || value.startsWith("/") || value.matches("^[A-Za-z]:.*")
                || value.contains("\\") || value.contains("//") || hasForbidden(value, true)) {
            return false;
        }
        for (String segment : value.split("/", -1)) {
            if (segment.isEmpty() || segment.equals(".") || segment.equals("..")
                    || segment.endsWith(" ") || segment.endsWith(".") || reserved(segment)) return false;
        }
        return true;
    }

    private static boolean isPortableGlob(String value) {
        if (value == null || scalarLength(value) < 1 || scalarLength(value) > 512
                || !Normalizer.isNormalized(value, Normalizer.Form.NFC)
                || value.startsWith("/") || value.matches("^[A-Za-z]:.*")
                || value.contains("\\") || value.contains("//") || hasForbidden(value, false)) {
            return false;
        }
        for (String segment : value.split("/", -1)) {
            if (segment.isEmpty() || segment.equals(".") || segment.equals("..")
                    || segment.endsWith(" ") || segment.endsWith(".")) return false;
            if (segment.chars().noneMatch(character -> "*[]{}".indexOf(character) >= 0)
                    && reserved(segment)) return false;
            if (hasAmbiguousCharacterClass(segment)) return false;
        }
        return true;
    }

    private static boolean hasAmbiguousCharacterClass(String segment) {
        int[] characters = segment.codePoints().toArray();
        int index = 0;
        while (index < characters.length) {
            if (characters[index] != '[') {
                index++;
                continue;
            }
            int cursor = index + 1;
            if (cursor < characters.length && characters[cursor] == '!') cursor++;
            int closing = cursor;
            if (closing < characters.length && characters[closing] == ']') closing++;
            while (closing < characters.length && characters[closing] != ']') closing++;
            if (closing == characters.length) {
                index++;
                continue;
            }
            for (int member = cursor; member + 1 < closing; member++) {
                int left = characters[member];
                int right = characters[member + 1];
                if ((left == '-' && right == '-') || (left == '&' && right == '&')
                        || (left == '~' && right == '~') || (left == '|' && right == '|')) {
                    return true;
                }
            }
            int member = cursor;
            while (member + 2 < closing) {
                if (characters[member + 1] == '-') {
                    if (characters[member] > characters[member + 2]) return true;
                    member += 3;
                } else {
                    member++;
                }
            }
            index = closing + 1;
        }
        return false;
    }

    private static boolean hasForbidden(String value, boolean path) {
        String forbidden = path ? "<>:\"|?*" : "<>:\"|?";
        return value.codePoints().anyMatch(codePoint -> codePoint < 32 || forbidden.indexOf(codePoint) >= 0);
    }

    private static boolean reserved(String segment) {
        String base = segment.split("\\.", 2)[0].toUpperCase(Locale.ROOT);
        return WINDOWS_RESERVED.contains(base);
    }

    private static String caseFoldIdentity(String value) {
        String normalized = Normalizer.normalize(value, Normalizer.Form.NFC);
        String lower = normalized.toLowerCase(Locale.ROOT);
        String upperLower = normalized.toUpperCase(Locale.ROOT).toLowerCase(Locale.ROOT);
        String folded = upperLower.codePointCount(0, upperLower.length())
                        > normalized.codePointCount(0, normalized.length())
                ? upperLower
                : lower;
        StringBuilder result = new StringBuilder(folded.length());
        folded.codePoints().forEach(codePoint -> {
            if (codePoint == 0x03C2) {
                result.appendCodePoint(0x03C3);
            } else if (codePoint >= 0xAB70 && codePoint <= 0xABBF) {
                result.appendCodePoint(Character.toUpperCase(codePoint));
            } else if (codePoint == 0x0131) {
                result.appendCodePoint(codePoint);
            } else {
                String one = new String(Character.toChars(codePoint));
                String canonical = one.toUpperCase(Locale.ROOT).toLowerCase(Locale.ROOT);
                result.append(canonical);
            }
        });
        return result.toString();
    }

    private static List<String> sorted(Set<String> values) {
        return values.stream().sorted(UNICODE_ORDER).toList();
    }

    private static DiffSelectionResult diffError(String code) {
        return new DiffSelectionResult(List.of(), List.of(), List.of(), code);
    }

    private static void require(boolean condition, String code) {
        if (!condition) throw new IllegalArgumentException(code);
    }

    private static int compareUnicode(String left, String right) {
        int[] a = left.codePoints().toArray();
        int[] b = right.codePoints().toArray();
        int length = Math.min(a.length, b.length);
        for (int index = 0; index < length; index++) {
            int compared = Integer.compare(a[index], b[index]);
            if (compared != 0) return compared;
        }
        return Integer.compare(a.length, b.length);
    }

    private static String canonicalBoundary(RepositoryBoundary boundary) {
        StringBuilder json = new StringBuilder("{\"boundaries\":[");
        appendJoined(json, boundary.boundaries(), BuildToolCore::appendBoundaryRule);
        json.append("],\"language_source_input_registry_sha256\":");
        appendString(json, boundary.languageSourceInputRegistrySha256());
        json.append(",\"schema_version\":").append(boundary.schemaVersion()).append('}');
        return json.toString();
    }

    private static void appendBoundaryRule(StringBuilder json, BoundaryRule rule) {
        json.append("{\"applies_to\":{\"descendant_roots\":[");
        appendStrings(json, rule.appliesTo().descendantRoots());
        json.append("],\"exact_roots\":[");
        appendStrings(json, rule.appliesTo().exactRoots());
        json.append("],\"excluded_roots\":[");
        appendStrings(json, rule.appliesTo().excludedRoots());
        json.append("]},\"id\":");
        appendString(json, rule.id());
        json.append(",\"input_origin\":");
        appendString(json, rule.inputOrigin());
        json.append(",\"inputs\":[");
        appendJoined(json, rule.inputs(), BuildToolCore::appendBoundaryInput);
        json.append("],\"owner\":");
        appendString(json, rule.owner());
        json.append(",\"reason\":");
        appendString(json, rule.reason());
        json.append('}');
    }

    private static void appendBoundaryInput(StringBuilder json, BoundaryInput input) {
        json.append('{');
        if (!input.generatedComponent().isEmpty()) {
            json.append("\"generated_component\":");
            appendString(json, input.generatedComponent());
            json.append(',');
        }
        json.append("\"path\":");
        appendString(json, input.path());
        json.append(",\"role\":");
        appendString(json, input.role());
        json.append('}');
    }

    private static void appendStrings(StringBuilder json, List<String> values) {
        appendJoined(json, values, BuildToolCore::appendString);
    }

    private static <T> void appendJoined(
            StringBuilder json, List<T> values, Appender<T> appender) {
        for (int index = 0; index < values.size(); index++) {
            if (index > 0) json.append(',');
            appender.append(json, values.get(index));
        }
    }

    private static void appendString(StringBuilder json, String value) {
        json.append('"');
        value.codePoints().forEach(codePoint -> {
            switch (codePoint) {
                case '"' -> json.append("\\\"");
                case '\\' -> json.append("\\\\");
                case '\b' -> json.append("\\b");
                case '\f' -> json.append("\\f");
                case '\n' -> json.append("\\n");
                case '\r' -> json.append("\\r");
                case '\t' -> json.append("\\t");
                default -> {
                    if (codePoint < 0x20) json.append(String.format("\\u%04x", codePoint));
                    else json.appendCodePoint(codePoint);
                }
            }
        });
        json.append('"');
    }

    private static String toHex(byte[] bytes) {
        StringBuilder result = new StringBuilder(bytes.length * 2);
        for (byte value : bytes) result.append(String.format("%02x", value & 0xff));
        return result.toString();
    }

    private record ValidatedGraph(
            Set<String> names,
            List<Edge> edges,
            Map<String, List<String>> dependents,
            Map<String, List<String>> prerequisites) {}

    @FunctionalInterface
    private interface Appender<T> {
        void append(StringBuilder builder, T value);
    }
}
