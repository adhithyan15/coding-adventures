import Foundation

public enum BuildTool {
    public static func run(arguments: [String]) -> Int32 {
        do {
            let options = try parseArguments(arguments)
            if options.help {
                print(usage())
                return 0
            }

            guard let repoRoot = findRepoRoot(explicitRoot: options.root) else {
                fputs("Error: Could not find repo root (.git directory).\nUse --root to specify the repo root.\n", stderr)
                return 1
            }

            if let planFile = options.planFile,
               let result = runFromPlanIfPossible(options: options, repoRoot: repoRoot, planFile: planFile) {
                return result
            }

            return try runNormalFlow(options: options, repoRoot: repoRoot)
        } catch {
            fputs("Error: \(error.localizedDescription)\n", stderr)
            return 1
        }
    }

    public static func expandAffectedSetWithPrereqs(_ graph: DirectedGraph, affectedSet: Set<String>?) -> Set<String>? {
        guard let affectedSet else {
            return nil
        }

        var expanded = affectedSet
        var queue = Array(affectedSet)

        while !queue.isEmpty {
            let current = queue.removeFirst()
            for prerequisite in graph.predecessors(of: current) where !expanded.contains(prerequisite) {
                expanded.insert(prerequisite)
                queue.append(prerequisite)
            }
        }

        return expanded
    }

    public static func outputLanguageFlags(_ languagesNeeded: [String: Bool]) {
        let outputPath = ProcessInfo.processInfo.environment["GITHUB_OUTPUT"]
        let handle: FileHandle? = outputPath.flatMap { path in
            FileManager.default.createFile(atPath: path, contents: nil)
            return FileHandle(forWritingAtPath: path)
        }

        defer {
            handle?.closeFile()
        }

        for language in allToolchains {
            let value = languagesNeeded[language] ?? false
            let line = "needs_\(language)=\(value ? "true" : "false")"
            print(line)
            if let handle {
                handle.seekToEndOfFile()
                handle.write(Data((line + "\n").utf8))
            }
        }
    }

    private static func runNormalFlow(options: CLIOptions, repoRoot: String) throws -> Int32 {
        let codeRoot = (repoRoot as NSString).appendingPathComponent("code")
        guard FileManager.default.fileExists(atPath: codeRoot) else {
            throw BuildToolError.io("\(codeRoot) does not exist.")
        }

        var packages = Discovery.discoverPackages(root: codeRoot)
        packages = evaluateStarlarkPackages(packages: packages, repoRoot: repoRoot)

        if options.language != "all" {
            packages = packages.filter { $0.language == options.language }
            if packages.isEmpty {
                fputs("No \(options.language) packages found.\n", stderr)
                return 0
            }
        }

        if options.validateBuildFiles,
           let validationError = Validator.validateBuildContracts(repoRoot: repoRoot, packages: packages) {
            fputs("BUILD/CI validation failed:\n  - \(validationError)\nFix the BUILD file or CI workflow so isolated and full-build runs stay correct.\n", stderr)
            return 1
        }

        print("Discovered \(packages.count) packages")
        let graph = Resolver.resolveDependencies(packages: packages)

        var force = options.force
        var affectedSet: Set<String>? = nil
        var ciToolchains = Set<String>()

        if !force {
            let changedFiles = GitDiff.getChangedFiles(repoRoot: repoRoot, diffBase: options.diffBase)
            if !changedFiles.isEmpty {
                if changedFiles.contains(CIWorkflow.workflowPath) {
                    let ciChange = CIWorkflow.analyzeChanges(repoRoot: repoRoot, diffBase: options.diffBase)
                    if ciChange.requiresFullRebuild {
                        print("Git diff: ci.yml changed in shared ways -- rebuilding everything")
                        force = true
                    } else {
                        ciToolchains = ciChange.toolchains
                        if !ciToolchains.isEmpty {
                            print(
                                "Git diff: ci.yml changed only toolchain-scoped setup for \(CIWorkflow.sortedToolchains(ciToolchains).joined(separator: ", "))"
                            )
                        }
                    }
                }

                let sharedChanged = changedFiles.contains { file in
                    guard file != CIWorkflow.workflowPath else {
                        return false
                    }
                    return sharedPrefixes.contains { prefix in
                        file == prefix || file.hasPrefix(prefix + "/")
                    }
                }
                if sharedChanged {
                    print("Git diff: shared files changed -- rebuilding everything")
                    force = true
                } else {
                    let packagePaths = Dictionary(uniqueKeysWithValues: packages.map { ($0.name, $0.path) })
                    let changedPackages = GitDiff.mapFilesToPackages(
                        changedFiles: changedFiles,
                        packagePaths: packagePaths,
                        repoRoot: repoRoot,
                        packages: packages
                    )
                    if changedPackages.isEmpty {
                        print("Git diff: no package files changed -- nothing to build")
                        affectedSet = []
                    } else {
                        let affected = graph.affectedNodes(changed: changedPackages)
                        affectedSet = expandAffectedSetWithPrereqs(graph, affectedSet: affected)
                        print("Git diff: \(changedPackages.count) packages changed, \(affectedSet?.count ?? 0) affected (including dependents and prerequisites)")
                    }
                }
            } else {
                print("Git diff unavailable -- falling back to hash-based cache")
            }
        }

        if let emitPlan = options.emitPlan {
            return try emitPlanFile(
                path: emitPlan,
                options: options,
                packages: packages,
                graph: graph,
                affectedSet: affectedSet,
                force: force,
                ciToolchains: ciToolchains,
                repoRoot: repoRoot
            )
        }

        if options.detectLanguages {
            outputLanguageFlags(
                computeLanguagesNeeded(
                    packages: packages,
                    affectedSet: affectedSet,
                    force: force,
                    ciToolchains: ciToolchains
                )
            )
            return 0
        }

        let packageHashes = Dictionary(uniqueKeysWithValues: packages.map { ($0.name, Hasher.hashPackage($0)) })
        let depsHashes = Dictionary(uniqueKeysWithValues: packages.map { ($0.name, Hasher.hashDeps(packageName: $0.name, graph: graph, packageHashes: packageHashes)) })

        let cachePath = absolutePath(options.cacheFile, repoRoot: repoRoot)
        let cache = BuildCache()
        cache.load(from: cachePath)

        let results = try Executor.executeBuilds(
            packages: packages,
            graph: graph,
            cache: cache,
            packageHashes: packageHashes,
            depsHashes: depsHashes,
            force: force,
            dryRun: options.dryRun,
            maxJobs: options.jobs,
            affectedSet: affectedSet
        )

        if !options.dryRun {
            try cache.save(to: cachePath)
        }

        print(Reporter.formatReport(results: results), terminator: "")
        return results.values.contains(where: { $0.status == .failed }) ? 1 : 0
    }

    private static func runFromPlanIfPossible(options: CLIOptions, repoRoot: String, planFile: String) -> Int32? {
        let absolutePlanPath = absolutePath(planFile, repoRoot: repoRoot)
        guard let plan = try? PlanIO.readPlan(from: absolutePlanPath) else {
            fputs("Warning: could not load plan (\(absolutePlanPath)), falling back to normal flow\n", stderr)
            return nil
        }

        do {
            var packages = plan.packages.map {
                BuildPackage(
                    name: $0.name,
                    path: (repoRoot as NSString).appendingPathComponent($0.relPath),
                    buildCommands: $0.buildCommands,
                    language: $0.language,
                    isStarlark: $0.isStarlark,
                    declaredSrcs: $0.declaredSrcs,
                    declaredDeps: $0.declaredDeps
                )
            }

            if options.language != "all" {
                packages = packages.filter { $0.language == options.language }
                if packages.isEmpty {
                    fputs("No \(options.language) packages found in plan.\n", stderr)
                    return 0
                }
            }

            if options.validateBuildFiles,
               let validationError = Validator.validateBuildContracts(repoRoot: repoRoot, packages: packages) {
                fputs("BUILD/CI validation failed:\n  - \(validationError)\nFix the BUILD file or CI workflow so isolated and full-build runs stay correct.\n", stderr)
                return 1
            }

            let graph = DirectedGraph()
            for package in packages {
                graph.addNode(package.name)
            }
            for edge in plan.dependencyEdges where edge.count == 2 {
                let from = edge[0]
                let to = edge[1]
                if graph.hasNode(from), graph.hasNode(to) {
                    graph.addEdge(from: from, to: to)
                }
            }

            let affectedSet = plan.affectedPackages.map(Set.init)
            print("Loaded plan: \(packages.count) packages")

            let packageHashes = Dictionary(uniqueKeysWithValues: packages.map { ($0.name, Hasher.hashPackage($0)) })
            let depsHashes = Dictionary(uniqueKeysWithValues: packages.map { ($0.name, Hasher.hashDeps(packageName: $0.name, graph: graph, packageHashes: packageHashes)) })
            let cachePath = absolutePath(options.cacheFile, repoRoot: repoRoot)

            let cache = BuildCache()
            cache.load(from: cachePath)

            let results = try Executor.executeBuilds(
                packages: packages,
                graph: graph,
                cache: cache,
                packageHashes: packageHashes,
                depsHashes: depsHashes,
                force: plan.force || options.force,
                dryRun: options.dryRun,
                maxJobs: options.jobs,
                affectedSet: affectedSet
            )

            if !options.dryRun {
                try cache.save(to: cachePath)
            }

            print(Reporter.formatReport(results: results), terminator: "")
            return results.values.contains(where: { $0.status == .failed }) ? 1 : 0
        } catch {
            fputs("Error: \(error.localizedDescription)\n", stderr)
            return 1
        }
    }

    private static func emitPlanFile(
        path: String,
        options: CLIOptions,
        packages: [BuildPackage],
        graph: DirectedGraph,
        affectedSet: Set<String>?,
        force: Bool,
        ciToolchains: Set<String>,
        repoRoot: String
    ) throws -> Int32 {
        let affectedPackages = affectedSet.map { Array($0).sorted() }
        let packageEntries = packages.map { package in
            PackageEntry(
                name: package.name,
                relPath: makeRelative(path: package.path, root: repoRoot),
                language: package.language,
                buildCommands: package.buildCommands,
                isStarlark: package.isStarlark,
                declaredSrcs: package.declaredSrcs,
                declaredDeps: package.declaredDeps
            )
        }
        let dependencyEdges = graph.edges().map { [$0.0, $0.1] }
        let languagesNeeded = computeLanguagesNeeded(
            packages: packages,
            affectedSet: affectedSet,
            force: force,
            ciToolchains: ciToolchains
        )
        let plan = BuildPlan(
            schemaVersion: PlanIO.currentSchemaVersion,
            diffBase: options.diffBase,
            force: force,
            affectedPackages: affectedPackages,
            packages: packageEntries,
            dependencyEdges: dependencyEdges,
            languagesNeeded: languagesNeeded
        )

        try PlanIO.writePlan(plan, to: absolutePath(path, repoRoot: repoRoot))
        print("Build plan written to \(path) (\(packages.count) packages)")

        if options.detectLanguages {
            outputLanguageFlags(languagesNeeded)
        }
        return 0
    }

    private static func computeLanguagesNeeded(packages: [BuildPackage], affectedSet: Set<String>?, force: Bool) -> [String: Bool] {
        var languagesNeeded: [String: Bool] = Dictionary(uniqueKeysWithValues: allToolchains.map { ($0, false) })
        if force || affectedSet == nil {
            for toolchain in allToolchains {
                languagesNeeded[toolchain] = true
            }
            return languagesNeeded
        }

        for package in packages where affectedSet?.contains(package.name) == true {
            languagesNeeded[toolchain(for: package.language)] = true
        }
        return languagesNeeded
    }

    private static func evaluateStarlarkPackages(packages: [BuildPackage], repoRoot: String) -> [BuildPackage] {
        var evaluated = 0
        let updated = packages.map { package -> BuildPackage in
            guard StarlarkEvaluator.isStarlarkBuild(package.buildContent) else {
                return package
            }

            do {
                let buildFilePath = Discovery.getBuildFile(directory: package.path) ?? (package.path as NSString).appendingPathComponent("BUILD")
                let result = try StarlarkEvaluator.evaluateBuildFile(
                    at: buildFilePath,
                    packageDirectory: package.path,
                    repoRoot: repoRoot
                )
                guard let target = result.targets.first else {
                    var updated = package
                    updated.isStarlark = true
                    return updated
                }

                evaluated += 1
                return BuildPackage(
                    name: package.name,
                    path: package.path,
                    buildCommands: StarlarkEvaluator.generateCommands(for: target),
                    language: package.language,
                    buildContent: package.buildContent,
                    isStarlark: true,
                    declaredSrcs: target.srcs,
                    declaredDeps: target.deps
                )
            } catch {
                fputs("Warning: Starlark eval failed for \(package.name): \(error.localizedDescription)\n", stderr)
                return package
            }
        }

        if evaluated > 0 {
            print("Evaluated \(evaluated) Starlark BUILD files")
        }
        return updated
    }

    private static func parseArguments(_ arguments: [String]) throws -> CLIOptions {
        var options = CLIOptions()
        var index = 0
        let languageChoices = Set(allPackageLanguages + ["all"])

        func requireValue(_ flag: String) throws -> String {
            index += 1
            guard index < arguments.count else {
                throw BuildToolError.invalidArguments("missing value for \(flag)")
            }
            return arguments[index]
        }

        while index < arguments.count {
            let argument = arguments[index]
            switch argument {
            case "--root":
                options.root = try requireValue(argument)
            case "--force":
                options.force = true
            case "--dry-run":
                options.dryRun = true
            case "--jobs":
                guard let jobs = Int(try requireValue(argument)), jobs > 0 else {
                    throw BuildToolError.invalidArguments("invalid value for --jobs")
                }
                options.jobs = jobs
            case "--language":
                let language = try requireValue(argument)
                guard languageChoices.contains(language) else {
                    throw BuildToolError.invalidArguments("unsupported language: \(language)")
                }
                options.language = language
            case "--diff-base":
                options.diffBase = try requireValue(argument)
            case "--cache-file":
                options.cacheFile = try requireValue(argument)
            case "--emit-plan":
                options.emitPlan = try requireValue(argument)
            case "--plan-file":
                options.planFile = try requireValue(argument)
            case "--detect-languages":
                options.detectLanguages = true
            case "--validate-build-files":
                options.validateBuildFiles = true
            case "--help", "-h":
                options.help = true
            default:
                throw BuildToolError.invalidArguments("unknown argument: \(argument)")
            }
            index += 1
        }

        return options
    }

    private static func usage() -> String {
        """
        Usage: build-tool [options]

          --root PATH                 Repo root directory
          --force                     Rebuild everything regardless of cache
          --dry-run                   Show what would build without executing
          --jobs N                    Maximum number of parallel build jobs
          --language LANG             Filter to one language or all
          --diff-base REF             Git ref for change detection (default: origin/main)
          --cache-file PATH           Path to the build cache file
          --emit-plan PATH            Write a build plan JSON file and exit
          --plan-file PATH            Read a build plan and skip discovery
          --detect-languages          Output needed language toolchains and exit
          --validate-build-files      Validate BUILD/CI metadata contracts
          --help                      Show this help
        """
    }

    private static func toolchain(for language: String) -> String {
        switch language {
        case "wasm":
            return "rust"
        case "csharp", "fsharp", "dotnet":
            return "dotnet"
        default:
            return language
        }
    }

    private static func findRepoRoot(explicitRoot: String?) -> String? {
        let startingPath = explicitRoot ?? FileManager.default.currentDirectoryPath
        var currentURL = URL(fileURLWithPath: startingPath).standardizedFileURL

        while true {
            let gitPath = currentURL.appendingPathComponent(".git").path
            if FileManager.default.fileExists(atPath: gitPath) {
                return currentURL.path
            }
            let parent = currentURL.deletingLastPathComponent()
            if parent.path == currentURL.path {
                return nil
            }
            currentURL = parent
        }
    }

    private static func absolutePath(_ path: String, repoRoot: String) -> String {
        if path.hasPrefix("/") {
            return URL(fileURLWithPath: path).standardizedFileURL.path
        }
        return URL(fileURLWithPath: repoRoot).appendingPathComponent(path).standardizedFileURL.path
    }

    private static func makeRelative(path: String, root: String) -> String {
        let normalizedPath = path.replacingOccurrences(of: "\\", with: "/")
        let normalizedRoot = root.replacingOccurrences(of: "\\", with: "/")
        if normalizedPath.hasPrefix(normalizedRoot + "/") {
            return String(normalizedPath.dropFirst(normalizedRoot.count + 1))
        }
        return normalizedPath
    }
}
