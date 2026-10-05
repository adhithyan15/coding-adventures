import Foundation

// Graph and diff decisions live below the host-facing Git/build-plan layer.
// Every input here is already materialized: no operation opens a checkout,
// invokes Git, inspects the environment, or acquires execution authority.
public enum BuildToolGraphDiff {
    public struct GraphInput {
        public let packages: [String]
        public let edges: [[String]]

        public init(packages: [String], edges: [[String]]) {
            self.packages = packages
            self.edges = edges
        }
    }

    public struct GraphResult {
        public let edges: [[String]]
        public let levels: [[String]]
        public let diagnosticCodes: [String]
    }

    public struct DiffPackage {
        public let name: String
        public let relPath: String
        public let sourceMode: String
        public let sourceGlobs: [String]

        public init(name: String, relPath: String, sourceMode: String, sourceGlobs: [String]) {
            self.name = name
            self.relPath = relPath
            self.sourceMode = sourceMode
            self.sourceGlobs = sourceGlobs
        }
    }

    // This value is supplied by the caller after fixture or host decoding.
    // Keeping it explicit permits a future validated registry without silently
    // borrowing the process's checkout or the source-embedded hash registry.
    public struct Boundary: Codable {
        public struct Rule: Codable {
            public struct Applicability: Codable {
                public let exactRoots: [String]
                public let descendantRoots: [String]
                public let excludedRoots: [String]

                enum CodingKeys: String, CodingKey {
                    case exactRoots = "exact_roots"
                    case descendantRoots = "descendant_roots"
                    case excludedRoots = "excluded_roots"
                }

                public init(exactRoots: [String], descendantRoots: [String], excludedRoots: [String]) {
                    self.exactRoots = exactRoots
                    self.descendantRoots = descendantRoots
                    self.excludedRoots = excludedRoots
                }

                fileprivate func matches(_ root: String) -> Bool {
                    exactRoots.contains(root) || descendantRoots.contains {
                        root.hasPrefix($0 + "/") && !excludedRoots.contains(root)
                    }
                }
            }

            public struct Input: Codable {
                public let path: String
                public let role: String
                public let generatedComponent: String?

                enum CodingKeys: String, CodingKey {
                    case path, role
                    case generatedComponent = "generated_component"
                }

                public init(path: String, role: String, generatedComponent: String? = nil) {
                    self.path = path
                    self.role = role
                    self.generatedComponent = generatedComponent
                }
            }

            public let id: String
            public let inputOrigin: String
            public let appliesTo: Applicability
            public let inputs: [Input]
            public let reason: String
            public let owner: String

            enum CodingKeys: String, CodingKey {
                case id, inputs, reason, owner
                case inputOrigin = "input_origin"
                case appliesTo = "applies_to"
            }

            public init(
                id: String, inputOrigin: String, appliesTo: Applicability,
                inputs: [Input], reason: String, owner: String
            ) {
                self.id = id
                self.inputOrigin = inputOrigin
                self.appliesTo = appliesTo
                self.inputs = inputs
                self.reason = reason
                self.owner = owner
            }
        }

        public let schemaVersion: Int
        public let languageSourceInputRegistrySHA256: String
        public let boundaries: [Rule]

        enum CodingKeys: String, CodingKey {
            case schemaVersion = "schema_version"
            case languageSourceInputRegistrySHA256 = "language_source_input_registry_sha256"
            case boundaries
        }

        public init(
            schemaVersion: Int,
            languageSourceInputRegistrySHA256: String,
            boundaries: [Rule]
        ) {
            self.schemaVersion = schemaVersion
            self.languageSourceInputRegistrySHA256 = languageSourceInputRegistrySHA256
            self.boundaries = boundaries
        }

        public func digest() throws -> String {
            try Hasher.canonicalRepositorySourceInputBoundaryDigest(from: JSONEncoder().encode(self))
        }
    }

    public struct DiffInput {
        public let packages: [DiffPackage]
        public let edges: [[String]]
        public let forcedPackages: [String]
        public let unknownPathPolicy: String
        public let boundarySHA256: String?
        public let boundary: Boundary?
        public let changedPaths: [String]

        public init(
            packages: [DiffPackage],
            edges: [[String]],
            forcedPackages: [String],
            unknownPathPolicy: String,
            boundarySHA256: String?,
            boundary: Boundary?,
            changedPaths: [String]
        ) {
            self.packages = packages
            self.edges = edges
            self.forcedPackages = forcedPackages
            self.unknownPathPolicy = unknownPathPolicy
            self.boundarySHA256 = boundarySHA256
            self.boundary = boundary
            self.changedPaths = changedPaths
        }
    }

    public struct DiffResult {
        public let changedPackages: [String]
        public let affectedPackages: [String]
        public let prerequisitePackages: [String]
        public let diagnosticCodes: [String]
    }

    private static let maximumPackages = 4_096
    private static let maximumEdges = 16_384
    private static let maximumGlobs = 256
    private static let maximumMatchWork = 50_000_000
    private static let buildFronts: Set<String> = [
        "BUILD", "BUILD_windows", "BUILD_mac", "BUILD_linux", "BUILD_mac_and_linux",
    ]

    // Raw UTF-8 ordering is the neutral qualified-name order. Names are
    // ASCII by contract, but keeping the comparison explicit prevents locale
    // or Foundation collation from slipping into a future extension.
    private static func ordinalLess(_ left: String, _ right: String) -> Bool {
        left.utf8.lexicographicallyPrecedes(right.utf8)
    }

    private static func graphError(_ code: String) -> GraphResult {
        GraphResult(edges: [], levels: [], diagnosticCodes: [code])
    }

    private static func diffError(_ code: String) -> DiffResult {
        DiffResult(
            changedPackages: [], affectedPackages: [], prerequisitePackages: [],
            diagnosticCodes: [code]
        )
    }

    private static func validPackageName(_ value: String) -> Bool {
        guard !value.isEmpty, value.unicodeScalars.count <= 240 else { return false }
        let components = value.split(separator: "/", omittingEmptySubsequences: false)
        guard components.count >= 2 else { return false }
        for component in components {
            guard let first = component.utf8.first,
                  (first >= 0x61 && first <= 0x7A) || (first >= 0x30 && first <= 0x39)
            else { return false }
            for byte in component.utf8 {
                let valid = (byte >= 0x61 && byte <= 0x7A)
                    || (byte >= 0x30 && byte <= 0x39)
                    || byte == 0x2E || byte == 0x5F || byte == 0x2D
                if !valid { return false }
            }
        }
        return true
    }

    private static func validatedGraph(
        packages: [String], edges: [[String]]
    ) -> (graph: DirectedGraph?, diagnostic: String?) {
        guard packages.count <= maximumPackages else { return (nil, "GRAPH_PACKAGE_LIMIT_EXCEEDED") }
        guard edges.count <= maximumEdges else { return (nil, "GRAPH_EDGE_LIMIT_EXCEEDED") }
        let graph = DirectedGraph()
        var seenNames = Set<String>()
        for name in packages {
            guard validPackageName(name) else { return (nil, "GRAPH_PACKAGE_INVALID") }
            guard seenNames.insert(name).inserted else { return (nil, "GRAPH_PACKAGE_DUPLICATE") }
            graph.addNode(name)
        }
        var seenEdges = Set<String>()
        for edge in edges {
            guard edge.count == 2 else { return (nil, "GRAPH_EDGE_INVALID") }
            let prerequisite = edge[0], dependent = edge[1]
            guard seenNames.contains(prerequisite), seenNames.contains(dependent) else {
                return (nil, "GRAPH_EDGE_UNKNOWN")
            }
            guard prerequisite != dependent else { return (nil, "GRAPH_EDGE_SELF") }
            // Package names cannot contain NUL; a framed pair avoids the
            // ambiguity of a delimiter that a package name might contain.
            let identity = prerequisite + "\0" + dependent
            guard seenEdges.insert(identity).inserted else { return (nil, "GRAPH_EDGE_DUPLICATE") }
            graph.addEdge(from: prerequisite, to: dependent)
        }
        return (graph, nil)
    }

    public static func evaluateGraph(_ input: GraphInput) -> GraphResult {
        let validated = validatedGraph(packages: input.packages, edges: input.edges)
        if let code = validated.diagnostic { return graphError(code) }
        guard let graph = validated.graph else { return graphError("GRAPH_PACKAGE_INVALID") }
        guard let levels = try? graph.independentGroups() else { return graphError("GRAPH_CYCLE") }
        let canonicalEdges = graph.edges().map { [$0.0, $0.1] }
        return GraphResult(edges: canonicalEdges, levels: levels, diagnosticCodes: [])
    }

    private static func inside(_ path: String, root: String) -> Bool {
        path == root || path.hasPrefix(root + "/")
    }

    private static func relative(_ path: String, root: String) -> String {
        path == root ? "" : String(path.dropFirst(root.count + 1))
    }

    private static func isBuildFront(_ path: String) -> Bool {
        guard !path.isEmpty else { return false }
        return buildFronts.contains(String(path.split(separator: "/").last ?? ""))
    }

    private static func validPath(_ path: String) -> Bool {
        (try? Hasher.validatePortablePath(path)) != nil
    }

    private static func validGlob(_ pattern: String) -> Bool {
        (try? Hasher.validatePortableGlob(pattern)) != nil
    }

    private static func validateDiffInput(_ input: DiffInput, graph: DirectedGraph) -> String? {
        var roots: [String] = []
        var names = Set<String>()
        for package in input.packages {
            guard validPackageName(package.name) else { return "DIFF_PACKAGE_INVALID" }
            guard validPath(package.relPath) else { return "DIFF_PATH_INVALID" }
            let identity = TrackedArtifactUnicode17.casefold(package.relPath)
            if roots.contains(where: {
                identity == $0 || identity.hasPrefix($0 + "/") || $0.hasPrefix(identity + "/")
            }) { return "DIFF_PATH_INVALID" }
            roots.append(identity)
            guard package.sourceMode == "package_prefix" || package.sourceMode == "strict_globs" else {
                return "DIFF_SOURCE_MODE_INVALID"
            }
            guard package.sourceGlobs.count <= maximumGlobs,
                  Set(package.sourceGlobs).count == package.sourceGlobs.count,
                  package.sourceGlobs.allSatisfy(validGlob),
                  package.sourceMode == "strict_globs" || package.sourceGlobs.isEmpty
            else { return "DIFF_GLOB_INVALID" }
            guard names.insert(package.name).inserted else { return "DIFF_PACKAGE_DUPLICATE" }
        }
        guard names.count == graph.nodes().count else { return "DIFF_PACKAGE_INVALID" }
        guard input.unknownPathPolicy == "all" || input.unknownPathPolicy == "error" else {
            return "DIFF_POLICY_INVALID"
        }
        guard input.forcedPackages.count <= maximumPackages,
              Set(input.forcedPackages).count == input.forcedPackages.count
        else { return "DIFF_FORCED_PACKAGE_INVALID" }
        guard input.changedPaths.count <= maximumPackages,
              Set(input.changedPaths).count == input.changedPaths.count,
              input.changedPaths.allSatisfy(validPath)
        else { return "DIFF_PATH_INVALID" }
        guard input.forcedPackages.allSatisfy(names.contains) else {
            return "DIFF_FORCED_PACKAGE_UNKNOWN"
        }
        return nil
    }

    private static func boundaryConsumers(_ input: DiffInput) -> [String: Set<String>]? {
        guard let digest = input.boundarySHA256 else {
            return input.boundary == nil ? [:] : nil
        }
        guard let boundary = input.boundary,
              digest.count == 64,
              digest.utf8.allSatisfy({ (0x30...0x39).contains($0) || (0x61...0x66).contains($0) }),
              (try? boundary.digest()) == digest
        else { return nil }
        var consumers: [String: Set<String>] = [:]
        for package in input.packages {
            for rule in boundary.boundaries where rule.appliesTo.matches(package.relPath) {
                for boundaryInput in rule.inputs {
                    consumers[boundaryInput.path, default: []].insert(package.name)
                }
            }
        }
        return consumers
    }

    public static func evaluateDiffSelection(_ input: DiffInput) -> DiffResult {
        let validated = validatedGraph(packages: input.packages.map(\.name), edges: input.edges)
        if let code = validated.diagnostic { return diffError(code) }
        guard let graph = validated.graph else { return diffError("DIFF_PACKAGE_INVALID") }
        guard (try? graph.independentGroups()) != nil else { return diffError("DIFF_EDGE_CYCLE") }
        if let code = validateDiffInput(input, graph: graph) { return diffError(code) }
        guard let consumers = boundaryConsumers(input) else {
            return diffError("DIFF_BOUNDARY_DIGEST_MISMATCH")
        }

        // The ceiling is checked for the entire operation before calling the
        // matcher. All globs are charged even if an earlier one would match.
        var remaining = maximumMatchWork
        for package in input.packages where package.sourceMode == "strict_globs" {
            let patternFactor = package.sourceGlobs.reduce(0) { $0 + $1.unicodeScalars.count + 1 }
            for path in input.changedPaths where inside(path, root: package.relPath) {
                let local = relative(path, root: package.relPath)
                if isBuildFront(local) { continue }
                let pathFactor = local.unicodeScalars.count + 1
                let (cost, overflow) = patternFactor.multipliedReportingOverflow(by: pathFactor)
                if overflow || cost > remaining { return diffError("DIFF_MATCH_LIMIT_EXCEEDED") }
                remaining -= cost
            }
        }

        var changed = Set(input.forcedPackages)
        var sawUnknown = false
        for path in input.changedPaths {
            let boundaryNames = consumers[path] ?? []
            changed.formUnion(boundaryNames)
            var known = !boundaryNames.isEmpty
            for package in input.packages where inside(path, root: package.relPath) {
                known = true
                let local = relative(path, root: package.relPath)
                var selected = package.sourceMode == "package_prefix" || isBuildFront(local)
                if !selected {
                    selected = package.sourceGlobs.contains { GlobMatch.matchPath($0, local) }
                }
                if selected { changed.insert(package.name) }
            }
            if !known { sawUnknown = true }
        }
        if sawUnknown {
            if input.unknownPathPolicy == "error" { return diffError("DIFF_UNKNOWN_PATH") }
            changed = Set(input.packages.map(\.name))
        }

        let affected = graph.affectedNodes(changed: changed)
        var prerequisites = Set<String>()
        for name in affected { prerequisites.formUnion(graph.transitivePrerequisites(of: name)) }
        prerequisites.subtract(affected)
        return DiffResult(
            changedPackages: changed.sorted(by: ordinalLess),
            affectedPackages: affected.sorted(by: ordinalLess),
            prerequisitePackages: prerequisites.sorted(by: ordinalLess),
            diagnosticCodes: []
        )
    }
}
