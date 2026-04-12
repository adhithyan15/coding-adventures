import Foundation

public enum Resolver {
    public static func resolveDependencies(packages: [BuildPackage]) -> DirectedGraph {
        let graph = DirectedGraph()
        for package in packages {
            graph.addNode(package.name)
        }

        var knownNamesByScope: [String: [String: String]] = [:]
        for package in packages {
            let scope = dependencyScope(for: package.language)
            if knownNamesByScope[scope] == nil {
                knownNamesByScope[scope] = buildKnownNames(packages: packages, language: scope)
            }
        }

        for package in packages {
            let knownNames = knownNamesByScope[dependencyScope(for: package.language)] ?? [:]
            let deps: [String]
            switch package.language {
            case "python":
                deps = parsePythonDeps(package: package, knownNames: knownNames)
            case "ruby":
                deps = parseRubyDeps(package: package, knownNames: knownNames)
            case "go":
                deps = parseGoDeps(package: package, knownNames: knownNames)
            case "typescript":
                deps = parseTypeScriptDeps(package: package, knownNames: knownNames)
            case "rust", "wasm":
                deps = parseRustDeps(package: package, knownNames: knownNames)
            case "elixir":
                deps = parseElixirDeps(package: package, knownNames: knownNames)
            case "lua":
                deps = parseLuaDeps(package: package, knownNames: knownNames)
            case "perl":
                deps = parsePerlDeps(package: package, knownNames: knownNames)
            case "swift":
                deps = parseSwiftDeps(package: package, knownNames: knownNames)
            case "haskell":
                deps = parseHaskellDeps(package: package, knownNames: knownNames)
            case "csharp", "fsharp", "dotnet":
                deps = parseDotnetDeps(package: package, knownNames: knownNames)
            default:
                deps = []
            }

            for dependency in deps {
                graph.addEdge(from: dependency, to: package.name)
            }
        }

        return graph
    }

    public static func buildKnownNames(packages: [BuildPackage], language: String? = nil) -> [String: String] {
        var known: [String: String] = [:]

        func setKnown(_ key: String, _ value: String, path: String) {
            let normalizedPath = normalizePath(path)
            if known[key] == nil || !normalizedPath.contains("/programs/") {
                known[key] = value
            }
        }

        for package in packages {
            if let language, !isPackageLanguage(package.language, in: language) {
                continue
            }
            let packageDirName = (package.path as NSString).lastPathComponent.lowercased()
            switch package.language {
            case "python":
                setKnown("coding-adventures-\(packageDirName)", package.name, path: package.path)
            case "ruby":
                setKnown("coding_adventures_\(packageDirName)", package.name, path: package.path)
            case "go":
                let goModPath = (package.path as NSString).appendingPathComponent("go.mod")
                if let content = try? String(contentsOfFile: goModPath, encoding: .utf8) {
                    for line in content.split(whereSeparator: \.isNewline) {
                        let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
                        if trimmed.hasPrefix("module ") {
                            let modulePath = trimmed.replacingOccurrences(of: "module ", with: "").trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
                            known[modulePath] = package.name
                            break
                        }
                    }
                }
            case "typescript":
                setKnown("@coding-adventures/\(packageDirName)", package.name, path: package.path)
                setKnown(packageDirName, package.name, path: package.path)
                let packageJSONPath = (package.path as NSString).appendingPathComponent("package.json")
                if let content = try? String(contentsOfFile: packageJSONPath, encoding: .utf8),
                   let match = content.firstMatch(for: #""name"\s*:\s*"([^"]+)""#) {
                    setKnown(match.lowercased(), package.name, path: package.path)
                }
            case "rust", "wasm":
                setKnown(packageDirName, package.name, path: package.path)
                if let cargoName = readCargoPackageName(in: package.path) {
                    setKnown(cargoName, package.name, path: package.path)
                }
            case "elixir":
                let baseName = packageDirName.replacingOccurrences(of: "-", with: "_")
                setKnown("coding_adventures_\(baseName)", package.name, path: package.path)
                setKnown(baseName, package.name, path: package.path)
                let mixPath = (package.path as NSString).appendingPathComponent("mix.exs")
                if let content = try? String(contentsOfFile: mixPath, encoding: .utf8),
                   let match = content.firstMatch(for: #"app:\s*:([a-z0-9_]+)"#) {
                    setKnown(match.lowercased(), package.name, path: package.path)
                }
            case "lua":
                setKnown("coding-adventures-\(packageDirName.replacingOccurrences(of: "_", with: "-"))", package.name, path: package.path)
            case "perl":
                setKnown("coding-adventures-\(packageDirName)", package.name, path: package.path)
            case "swift":
                setKnown(packageDirName, package.name, path: package.path)
            case "haskell":
                setKnown("coding-adventures-\(packageDirName.replacingOccurrences(of: "_", with: "-"))", package.name, path: package.path)
            case "csharp", "fsharp", "dotnet":
                setKnown(packageDirName, package.name, path: package.path)
            default:
                break
            }
        }

        return known
    }

    public static func parseSwiftDeps(package: BuildPackage, knownNames: [String: String]) -> [String] {
        let manifestPath = (package.path as NSString).appendingPathComponent("Package.swift")
        guard let content = try? String(contentsOfFile: manifestPath, encoding: .utf8) else {
            return []
        }

        var dependencies: [String] = []
        for line in content.split(whereSeparator: \.isNewline) {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty,
                  !trimmed.hasPrefix("//"),
                  let match = trimmed.firstMatch(for: #"\.package\s*\(\s*path\s*:\s*"\.\./([^"]+)""#) else {
                continue
            }
            let depDirectory = match.lowercased()
            if depDirectory.contains("/") || depDirectory.contains("\\") || depDirectory == ".." {
                continue
            }
            if let packageName = knownNames[depDirectory] {
                dependencies.append(packageName)
            }
        }

        return dependencies
    }

    private static func parseDotnetDeps(package: BuildPackage, knownNames: [String: String]) -> [String] {
        let projectFiles = filePaths(in: package.path, suffixes: [".csproj", ".fsproj"])
        guard !projectFiles.isEmpty else {
            return []
        }

        var dependencies: [String] = []
        for projectFile in projectFiles {
            guard let content = try? String(contentsOfFile: projectFile, encoding: .utf8) else {
                continue
            }
            for match in content.matches(for: #"<ProjectReference\s+Include\s*=\s*"\.\.[\\/]+([^/\\"]+)[\\/][^"]*""#) {
                let depDir = match.lowercased()
                if depDir.contains("/") || depDir.contains("\\") || depDir == ".." {
                    continue
                }
                if let packageName = knownNames[depDir] {
                    dependencies.append(packageName)
                }
            }
        }

        return dependencies
    }

    private static func parsePythonDeps(package: BuildPackage, knownNames: [String: String]) -> [String] {
        let filePath = (package.path as NSString).appendingPathComponent("pyproject.toml")
        guard let content = try? String(contentsOfFile: filePath, encoding: .utf8) else {
            return []
        }

        var dependencies: [String] = []
        var inDependencies = false
        for line in content.split(whereSeparator: \.isNewline) {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            if !inDependencies {
                if trimmed.hasPrefix("dependencies"), trimmed.contains("="), trimmed.contains("[") {
                    inDependencies = true
                    extractQuotedDeps(from: trimmed, knownNames: knownNames, into: &dependencies)
                    if trimmed.contains("]") {
                        inDependencies = false
                    }
                }
                continue
            }

            extractQuotedDeps(from: trimmed, knownNames: knownNames, into: &dependencies)
            if trimmed.contains("]") {
                inDependencies = false
            }
        }

        return dependencies
    }

    private static func parseRubyDeps(package: BuildPackage, knownNames: [String: String]) -> [String] {
        guard let gemspecPath = firstFile(in: package.path, suffix: ".gemspec"),
              let content = try? String(contentsOfFile: gemspecPath, encoding: .utf8) else {
            return []
        }

        let pattern = #"spec\.add_dependency\s+"([^"]+)""#
        return content.matches(for: pattern).compactMap { knownNames[$0.lowercased()] }
    }

    private static func parseGoDeps(package: BuildPackage, knownNames: [String: String]) -> [String] {
        let filePath = (package.path as NSString).appendingPathComponent("go.mod")
        guard let content = try? String(contentsOfFile: filePath, encoding: .utf8) else {
            return []
        }

        var dependencies: [String] = []
        var inRequireBlock = false

        for line in content.split(whereSeparator: \.isNewline) {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            if trimmed == "require (" {
                inRequireBlock = true
                continue
            }
            if trimmed == ")" {
                inRequireBlock = false
                continue
            }

            if inRequireBlock || trimmed.hasPrefix("require ") {
                let stripped = trimmed.replacingOccurrences(of: "require ", with: "")
                let parts = stripped.split(whereSeparator: \.isWhitespace)
                if let modulePath = parts.first?.lowercased(),
                   let packageName = knownNames[modulePath] {
                    dependencies.append(packageName)
                }
            }
        }

        return dependencies
    }

    private static func parseTypeScriptDeps(package: BuildPackage, knownNames: [String: String]) -> [String] {
        let filePath = (package.path as NSString).appendingPathComponent("package.json")
        guard let content = try? String(contentsOfFile: filePath, encoding: .utf8) else {
            return []
        }

        var dependencies: [String] = []
        var inDependencies = false
        for line in content.split(whereSeparator: \.isNewline) {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            if !inDependencies {
                if (trimmed.contains(#""dependencies""#) || trimmed.contains(#""devDependencies""#)), trimmed.contains("{") {
                    inDependencies = true
                }
                continue
            }

            if trimmed.contains("}") {
                inDependencies = false
                continue
            }

            for match in trimmed.matches(for: #""([^"]+)"\s*:"#) {
                let depName = match.lowercased()
                if let packageName = knownNames[depName] {
                    dependencies.append(packageName)
                }
            }
        }

        return dependencies
    }

    private static func parseRustDeps(package: BuildPackage, knownNames: [String: String]) -> [String] {
        let filePath = (package.path as NSString).appendingPathComponent("Cargo.toml")
        guard let content = try? String(contentsOfFile: filePath, encoding: .utf8) else {
            return []
        }

        var dependencies: [String] = []
        var inDependencies = false

        for line in content.split(whereSeparator: \.isNewline) {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            if trimmed.hasPrefix("[") {
                inDependencies = trimmed == "[dependencies]"
                continue
            }
            guard inDependencies else {
                continue
            }
            if trimmed.contains("path"), trimmed.contains("=") {
                let parts = trimmed.split(separator: "=", maxSplits: 1)
                let crateName = parts[0].trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
                if let packageName = knownNames[crateName] {
                    dependencies.append(packageName)
                }
            }
        }

        return dependencies
    }

    private static func parseElixirDeps(package: BuildPackage, knownNames: [String: String]) -> [String] {
        let filePath = (package.path as NSString).appendingPathComponent("mix.exs")
        guard let content = try? String(contentsOfFile: filePath, encoding: .utf8) else {
            return []
        }
        return content
            .matches(for: #"\{:(coding_adventures_[a-z0-9_]+)"#)
            .compactMap { knownNames[$0.lowercased()] }
    }

    private static func parseLuaDeps(package: BuildPackage, knownNames: [String: String]) -> [String] {
        guard let rockspecPath = firstFile(in: package.path, suffix: ".rockspec"),
              let content = try? String(contentsOfFile: rockspecPath, encoding: .utf8) else {
            return []
        }

        var dependencies: [String] = []
        var inBlock = false
        for line in content.split(whereSeparator: \.isNewline) {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            if !inBlock {
                if trimmed.contains("dependencies"), trimmed.contains("="), trimmed.contains("{") {
                    inBlock = true
                    extractLuaDeps(from: trimmed, knownNames: knownNames, into: &dependencies)
                    if trimmed.contains("}") {
                        inBlock = false
                    }
                }
                continue
            }

            extractLuaDeps(from: trimmed, knownNames: knownNames, into: &dependencies)
            if trimmed.contains("}") {
                inBlock = false
            }
        }

        return dependencies
    }

    private static func parsePerlDeps(package: BuildPackage, knownNames: [String: String]) -> [String] {
        let filePath = (package.path as NSString).appendingPathComponent("cpanfile")
        guard let content = try? String(contentsOfFile: filePath, encoding: .utf8) else {
            return []
        }
        return content
            .matches(for: #"requires\s+['"](coding-adventures-[^'"]+)['"]"#)
            .compactMap { knownNames[$0.lowercased()] }
    }

    private static func parseHaskellDeps(package: BuildPackage, knownNames: [String: String]) -> [String] {
        guard let cabalPath = firstFile(in: package.path, suffix: ".cabal"),
              let content = try? String(contentsOfFile: cabalPath, encoding: .utf8) else {
            return []
        }
        return content
            .matches(for: #"(coding-adventures-[a-zA-Z0-9-]+)"#)
            .compactMap { depName in
                guard let pkgName = knownNames[depName.lowercased()], pkgName != package.name else {
                    return nil
                }
                return pkgName
            }
    }

    private static func dependencyScope(for language: String) -> String {
        switch language {
        case "csharp", "fsharp", "dotnet":
            return "dotnet"
        case "wasm":
            return "wasm"
        default:
            return language
        }
    }

    private static func isPackageLanguage(_ packageLanguage: String, in scope: String) -> Bool {
        switch scope {
        case "dotnet":
            return ["csharp", "fsharp", "dotnet"].contains(packageLanguage)
        case "wasm":
            return ["wasm", "rust"].contains(packageLanguage)
        default:
            return packageLanguage == scope
        }
    }

    private static func readCargoPackageName(in directory: String) -> String? {
        let cargoPath = (directory as NSString).appendingPathComponent("Cargo.toml")
        guard let content = try? String(contentsOfFile: cargoPath, encoding: .utf8) else {
            return nil
        }
        return content.firstMatch(for: #"(?m)^\s*name\s*=\s*"([^"]+)""#)?.lowercased()
    }

    private static func filePaths(in directory: String, suffixes: [String]) -> [String] {
        guard let entries = try? FileManager.default.contentsOfDirectory(atPath: directory) else {
            return []
        }
        return entries
            .sorted()
            .filter { entry in suffixes.contains { entry.hasSuffix($0) } }
            .map { (directory as NSString).appendingPathComponent($0) }
    }

    private static func extractQuotedDeps(from line: String, knownNames: [String: String], into dependencies: inout [String]) {
        for match in line.matches(for: #"["']([^"']+)["']"#) {
            let depName = splitVersionSpecifier(match)
            if let packageName = knownNames[depName] {
                dependencies.append(packageName)
            }
        }
    }

    private static func extractLuaDeps(from line: String, knownNames: [String: String], into dependencies: inout [String]) {
        for match in line.matches(for: #""([^"]+)""#) {
            let depName = splitVersionSpecifier(match)
            if let packageName = knownNames[depName] {
                dependencies.append(packageName)
            }
        }
    }

    private static func splitVersionSpecifier(_ value: String) -> String {
        let separators = CharacterSet(charactersIn: "><= !~;")
        let parts = value.lowercased().components(separatedBy: separators)
        return parts.first?.trimmingCharacters(in: .whitespacesAndNewlines) ?? value.lowercased()
    }

    private static func normalizePath(_ path: String) -> String {
        path.replacingOccurrences(of: "\\", with: "/").lowercased()
    }

    private static func firstFile(in directory: String, suffix: String) -> String? {
        guard let entries = try? FileManager.default.contentsOfDirectory(atPath: directory) else {
            return nil
        }
        return entries
            .sorted()
            .first(where: { $0.hasSuffix(suffix) })
            .map { (directory as NSString).appendingPathComponent($0) }
    }
}

private extension String {
    func firstMatch(for pattern: String) -> String? {
        matches(for: pattern).first
    }

    func matches(for pattern: String) -> [String] {
        guard let regex = try? NSRegularExpression(pattern: pattern) else {
            return []
        }
        let range = NSRange(location: 0, length: utf16.count)
        return regex.matches(in: self, options: [], range: range).compactMap { match in
            guard match.numberOfRanges > 1,
                  let range = Range(match.range(at: 1), in: self) else {
                return nil
            }
            return String(self[range])
        }
    }
}
