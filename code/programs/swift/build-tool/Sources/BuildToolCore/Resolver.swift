import Foundation

public enum Resolver {
    public static func resolveDependencies(packages: [BuildPackage]) throws -> DirectedGraph {
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
                deps = try parseLuaDeps(package: package, knownNames: knownNames)
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
            case "rust":
                // A Rust crate is known by its directory name ("logic-gates")
                // and by its declared `[package]` `name`.
                setKnown(packageDirName, package.name, path: package.path)
                if let cargoName = readCargoPackageName(in: package.path) {
                    setKnown(cargoName, package.name, path: package.path)
                }
            case "wasm":
                // A WASM wrapper is known only by its declared Cargo name
                // ("graph-wasm"). Its bare directory name ("graph") belongs to
                // the Rust crate it wraps, and claiming it would turn the
                // wrapper's own `graph = { path = "../../rust/graph" }` into a
                // self-edge (as the Go reference explains).
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

    // MARK: - Rust (Cargo.toml)
    //
    // # Which tables are build-graph edges
    //
    // A graph edge means "rebuild and re-test the dependent when this
    // changes", so it must cover everything Cargo compiles INTO the crate:
    //
    //     table                                  edge?  why
    //     -------------------------------------  -----  ---------------------------
    //     [dependencies]                         yes    linked into the library
    //     [build-dependencies]                   yes    compiled into build.rs,
    //                                                   which generates or
    //                                                   configures it
    //     [target.<cfg>.dependencies]            yes    linked on that target; CI
    //     [target.<cfg>.build-dependencies]      yes    builds on several hosts,
    //                                                   so the union over every cfg
    //     [dev-dependencies] (every form)        no     tests only; Cargo allows
    //                                                   dev cycles, a graph must not
    //     [patch], [replace], [workspace], ...   no     not a dependency at all
    //
    // `build_dependencies` is Cargo's legacy spelling of `build-dependencies`.
    //
    // # Which entry shapes
    //
    // Both TOML spellings of a path dependency are read:
    //
    //     [dependencies]
    //     logic-gates = { path = "../logic-gates" }          <- inline table
    //     gates = { package = "logic-gates", path = "..." }  <- renamed: "logic-gates"
    //
    //     [dependencies.logic-gates]                         <- sub-table
    //     path = "../logic-gates"
    //
    // The crate name (the `package` rename if present, else the key) is then
    // looked up in the known names, and each internal package is returned
    // once. The line procedure is the cross-implementation contract in the
    // Rust paragraph of code/specs/build-tool-conformance.md; every build-tool
    // front implements the same one (ported here from the Go reference,
    // cargoLinkDependencyNames), and the shared fixture
    // resolution-rust-field-aware pins it.
    //
    // # Why bytes, not Characters
    //
    // A Swift `Character` is a grapheme cluster, and "\r\n" is ONE grapheme,
    // so splitting a String at "\n" would never split a CRLF file at all. The
    // contract is defined on LF and on ASCII delimiters, so the reader works
    // on the UTF-8 bytes. Every byte it compares is ASCII, and in UTF-8 an
    // ASCII byte is never part of a multi-byte character, so byte offsets are
    // exact and every slice it decodes back to a String is whole characters.

    private static func parseRustDeps(package: BuildPackage, knownNames: [String: String]) -> [String] {
        let filePath = (package.path as NSString).appendingPathComponent("Cargo.toml")
        guard let content = try? String(contentsOfFile: filePath, encoding: .utf8) else {
            return []
        }

        var dependencies: [String] = []
        for crateName in cargoLinkDependencyNames(content) {
            // A crate may sit in several link tables at once (say in
            // `[dependencies]` and `[build-dependencies]`): one edge.
            if let packageName = knownNames[crateName], !dependencies.contains(packageName) {
                dependencies.append(packageName)
            }
        }
        return dependencies
    }

    /// The table names whose path entries link into, or build, the crate.
    /// They are compared exactly: TOML keys are case-sensitive, so
    /// `[Dependencies]` is some other table.
    private static let cargoLinkTableNames: Set<String> = [
        "dependencies", "build-dependencies", "build_dependencies",
    ]

    /// How the lines under the current header are read.
    private enum CargoLinkTable {
        /// Not a link table: its lines are skipped.
        case ignored
        /// `[D]` or `[target.C.D]`: one `name = { ... }` entry per line.
        case inline
        /// `[D.N]` or `[target.C.D.N]`: the keys of the one entry `N`.
        case sub(String)
    }

    private typealias Bytes = ArraySlice<UInt8>

    private static let quote = UInt8(ascii: "\"")
    private static let apostrophe = UInt8(ascii: "'")
    private static let backslash = UInt8(ascii: "\\")
    private static let space = UInt8(ascii: " ")
    private static let tab = UInt8(ascii: "\t")

    /// Returns the ASCII-lowercased crate names of every path dependency in
    /// a manifest's link tables, in manifest order, duplicates included (the
    /// caller collapses them).
    ///
    /// It is one pass over the lines with a three-state machine
    /// (`CargoLinkTable`). Every header line — even a malformed one, or an
    /// array of tables — ends the current table, and so does the end of the
    /// file; a sub-table's entry is complete only then, because its `path`
    /// and `package` keys may come in any order.
    ///
    /// One inline entry may cover several lines. TOML 1.0 lets an ARRAY
    /// inside an inline table span lines, and the repository writes exactly
    /// that:
    ///
    ///     coding_adventures_vault_pm_cli = { path = "../vault-pm-cli", features = [
    ///         "crash-injection",
    ///     ] }
    ///
    /// So while the entry's value still has an unclosed `{` or `[`, the next
    /// lines are joined onto it with one space (and are not looked at as
    /// headers or entries). If the file ends first, the entry contributes
    /// nothing.
    public static func cargoLinkDependencyNames(_ text: String) -> [String] {
        var bytes = Array(text.utf8)
        if bytes.starts(with: [0xEF, 0xBB, 0xBF]) {
            bytes.removeFirst(3) // a leading UTF-8 byte-order mark
        }
        // Step 1 of the contract: split at LF, drop ONE trailing CR, strip
        // the comment outside strings, trim spaces and tabs.
        let lines = bytes.split(separator: UInt8(ascii: "\n"), omittingEmptySubsequences: false)
        func cleanLine(_ raw: Bytes) -> Bytes {
            var line = raw
            if line.last == UInt8(ascii: "\r") {
                line = line.dropLast()
            }
            return trimBlank(stripLineComment(line))
        }

        var names: [String] = []
        var table = CargoLinkTable.ignored
        var subPath: Bytes?
        var subPackage: Bytes?

        // A sub-table is a candidate only once it ends, when `path` was set;
        // it is named by its `package` rename, else by its key `N`.
        func finishTable() {
            if case .sub(let key) = table, subPath != nil {
                names.append(asciiLowercased(subPackage.map(decode) ?? key))
            }
        }

        var index = 0
        while index < lines.count {
            let line = cleanLine(lines[index])
            index += 1

            if line.first == UInt8(ascii: "["), line.last == UInt8(ascii: "]") {
                finishTable()
                table = classifyCargoLinkHeader(line)
                subPath = nil
                subPackage = nil
                continue
            }

            guard let equals = indexOutsideStrings(line, UInt8(ascii: "=")) else {
                continue
            }
            switch table {
            case .ignored:
                break
            case .inline:
                var value = Array(trimBlank(line[(equals + 1)...]))
                var complete = true
                while value.first == UInt8(ascii: "{"), openBracketDepth(value[...]) > 0 {
                    guard index < lines.count else {
                        complete = false // the file ended inside the value
                        break
                    }
                    value.append(space)
                    value.append(contentsOf: cleanLine(lines[index]))
                    index += 1
                }
                if complete, let name = cargoPathDependencyName(key: line[..<equals], value: value[...]) {
                    names.append(name)
                }
            case .sub:
                // Only the two bare, lowercase keys matter; the first
                // occurrence of each wins. `version`, `features`,
                // `optional` ... are ignored.
                guard let value = singleString(line[(equals + 1)...]) else {
                    continue
                }
                switch decode(trimBlank(line[..<equals])) {
                case "path" where subPath == nil:
                    subPath = value
                case "package" where subPackage == nil:
                    subPackage = value
                default:
                    break
                }
            }
        }
        finishTable()
        return names
    }

    /// Decides how to read the lines under a header line (already
    /// comment-stripped and trimmed, starting with `[` and ending with `]`).
    ///
    /// The header is split into dotted-key segments at every `.` OUTSIDE a
    /// string, and each segment is decoded to its value. That is what makes
    /// a cfg header one segment however much punctuation it quotes:
    ///
    ///     [target.'cfg(any(target_os = "linux", windows))'.dependencies]
    ///      target  C (one quoted segment)                    D
    ///
    ///     [target."thumbv8m.main-none-eabihf".dependencies.pi]  -> sub-table "pi"
    ///     [target.x86_64-unknown-linux-gnu.build-dependencies]  -> bare triple
    ///     ["dependencies"]                                      -> same as [dependencies]
    ///
    /// Segment lists that are not exactly one of `[D]`, `[D.N]`,
    /// `[target.C.D]` or `[target.C.D.N]` — and malformed headers — are
    /// ignored tables.
    private static func classifyCargoLinkHeader(_ line: Bytes) -> CargoLinkTable {
        if line.starts(with: [UInt8(ascii: "["), UInt8(ascii: "[")]) {
            return .ignored // array of tables, e.g. [[bin]]
        }
        let inner = line[(line.startIndex + 1)..<(line.endIndex - 1)]
        var segments: [String] = []
        for raw in splitOutsideStrings(inner, separator: UInt8(ascii: "."), nested: false) {
            guard let segment = keySegment(raw) else {
                return .ignored
            }
            segments.append(segment)
        }
        if segments.count >= 3, segments[0] == "target" {
            segments.removeFirst(2) // drop `target` and the cfg, whatever it is
        }
        if segments.count == 1, cargoLinkTableNames.contains(segments[0]) {
            return .inline
        }
        if segments.count == 2, cargoLinkTableNames.contains(segments[0]) {
            return .sub(segments[1])
        }
        return .ignored
    }

    /// Returns the crate a completed inline-table entry `key = value` names
    /// when it is a path dependency — the only kind that can point at a
    /// repository package. The key is the crate name unless the inline table
    /// renames it with `package = "..."`:
    ///
    ///     logic-gates = { path = "../logic-gates" }                    -> logic-gates
    ///     gates = { package = "logic-gates", path = "../logic-gates" } -> logic-gates
    ///     serde = "1"                                                  -> not a path dep
    ///     x = { version = "1", features = ["path"] }                   -> not a path dep
    ///     foo.path = "../foo"                                          -> dotted key: no
    ///
    /// Fields are split at commas outside strings AND outside nested `[...]`
    /// or `{...}`, so a `features` array cannot leak a fake field and a
    /// nested table's `path` is not the entry's. Field keys must be exactly
    /// `path` and `package` (Cargo ignores `Package`, so it must not count as
    /// a rename).
    private static func cargoPathDependencyName(key: Bytes, value: Bytes) -> String? {
        guard let crateName = keySegment(key) else {
            return nil
        }
        let value = trimBlank(value)
        guard value.count >= 2, value.first == UInt8(ascii: "{"), value.last == UInt8(ascii: "}") else {
            return nil
        }
        var path: Bytes?
        var packageName: Bytes?
        let fields = value[(value.startIndex + 1)..<(value.endIndex - 1)]
        for field in splitOutsideStrings(fields, separator: UInt8(ascii: ","), nested: true) {
            guard let equals = indexOutsideStrings(field, UInt8(ascii: "=")),
                  let fieldValue = singleString(field[(equals + 1)...]) else {
                continue
            }
            switch decode(trimBlank(field[..<equals])) {
            case "path" where path == nil:
                path = fieldValue
            case "package" where packageName == nil:
                packageName = fieldValue
            default:
                break
            }
        }
        guard path != nil else {
            return nil
        }
        return asciiLowercased(packageName.map(decode) ?? crateName)
    }

    /// Trims spaces and tabs only — the contract's notion of blank.
    private static func trimBlank(_ bytes: Bytes) -> Bytes {
        var slice = bytes
        while let first = slice.first, first == space || first == tab {
            slice = slice.dropFirst()
        }
        while let last = slice.last, last == space || last == tab {
            slice = slice.dropLast()
        }
        return slice
    }

    /// Returns the index just past the string that opens at `bytes[start]`
    /// (a `"` or `'`), or nil if it is not closed on this line. In a basic
    /// `"..."` string a backslash makes the next byte part of the string, so
    /// `\"` does not end it; a literal `'...'` string has no escapes.
    private static func stringEnd(_ bytes: Bytes, _ start: Int) -> Int? {
        let delimiter = bytes[start]
        var index = start + 1
        while index < bytes.endIndex {
            if delimiter == quote, bytes[index] == backslash {
                index += 1
            } else if bytes[index] == delimiter {
                return index + 1
            }
            index += 1
        }
        return nil
    }

    /// Removes a TOML comment: everything from the first `#` outside a
    /// string. An unclosed string runs to the end of the line.
    private static func stripLineComment(_ line: Bytes) -> Bytes {
        var index = line.startIndex
        while index < line.endIndex {
            switch line[index] {
            case quote, apostrophe:
                guard let end = stringEnd(line, index) else {
                    return line
                }
                index = end
            case UInt8(ascii: "#"):
                return line[..<index]
            default:
                index += 1
            }
        }
        return line
    }

    /// Returns the index of the first `target` byte that is outside every
    /// string, or nil (also when a string is left unclosed before it).
    private static func indexOutsideStrings(_ bytes: Bytes, _ target: UInt8) -> Int? {
        var index = bytes.startIndex
        while index < bytes.endIndex {
            switch bytes[index] {
            case quote, apostrophe:
                guard let end = stringEnd(bytes, index) else {
                    return nil
                }
                index = end
            case target:
                return index
            default:
                index += 1
            }
        }
        return nil
    }

    /// Counts the `{` and `[` outside strings that are not closed again
    /// within `bytes`. Zero (or less) means the value is complete; an
    /// unclosed string stops the count where it opens.
    private static func openBracketDepth(_ bytes: Bytes) -> Int {
        var depth = 0
        var index = bytes.startIndex
        while index < bytes.endIndex {
            switch bytes[index] {
            case quote, apostrophe:
                guard let end = stringEnd(bytes, index) else {
                    return depth
                }
                index = end
                continue
            case UInt8(ascii: "{"), UInt8(ascii: "["):
                depth += 1
            case UInt8(ascii: "}"), UInt8(ascii: "]"):
                depth -= 1
            default:
                break
            }
            index += 1
        }
        return depth
    }

    /// Splits at every `separator` outside strings. With `nested` set, a
    /// separator inside `[...]` or `{...}` does not split either. An
    /// unclosed string makes the rest the last part.
    private static func splitOutsideStrings(_ bytes: Bytes, separator: UInt8, nested: Bool) -> [Bytes] {
        var parts: [Bytes] = []
        var start = bytes.startIndex
        var depth = 0
        var index = bytes.startIndex
        while index < bytes.endIndex {
            let byte = bytes[index]
            if byte == quote || byte == apostrophe {
                guard let end = stringEnd(bytes, index) else {
                    parts.append(bytes[start...])
                    return parts
                }
                index = end
                continue
            }
            if nested, byte == UInt8(ascii: "[") || byte == UInt8(ascii: "{") {
                depth += 1
            } else if nested, byte == UInt8(ascii: "]") || byte == UInt8(ascii: "}") {
                depth -= 1
            } else if byte == separator, depth == 0 {
                parts.append(bytes[start..<index])
                start = index + 1
            }
            index += 1
        }
        parts.append(bytes[start...])
        return parts
    }

    /// Decodes one key or header segment: a bare key (ASCII letters,
    /// digits, `_`, `-`), or exactly one complete string, whose value is the
    /// text between its quotes taken verbatim (escapes are not decoded).
    /// Anything else — empty, dotted, or text around a string — is not a key.
    private static func keySegment(_ raw: Bytes) -> String? {
        let segment = trimBlank(raw)
        guard let first = segment.first else {
            return nil
        }
        if first == quote || first == apostrophe {
            guard stringEnd(segment, segment.startIndex) == segment.endIndex else {
                return nil
            }
            return decode(segment[(segment.startIndex + 1)..<(segment.endIndex - 1)])
        }
        let isBare = segment.allSatisfy { byte in
            (byte >= UInt8(ascii: "a") && byte <= UInt8(ascii: "z"))
                || (byte >= UInt8(ascii: "A") && byte <= UInt8(ascii: "Z"))
                || (byte >= UInt8(ascii: "0") && byte <= UInt8(ascii: "9"))
                || byte == UInt8(ascii: "_") || byte == UInt8(ascii: "-")
        }
        return isBare ? decode(segment) : nil
    }

    /// Returns the content of a value that is exactly one complete,
    /// non-empty string (after trimming), as `path` and `package` require.
    /// `"1"` is one; `"a" "b"`, `1`, `""` and `["x"]` are not.
    private static func singleString(_ raw: Bytes) -> Bytes? {
        let value = trimBlank(raw)
        guard let first = value.first, first == quote || first == apostrophe,
              stringEnd(value, value.startIndex) == value.endIndex else {
            return nil
        }
        let content = value[(value.startIndex + 1)..<(value.endIndex - 1)]
        return content.isEmpty ? nil : content
    }

    private static func decode(_ bytes: Bytes) -> String {
        String(decoding: bytes, as: UTF8.self)
    }

    /// Lowercases A-Z only, as the contract says: `String.lowercased()`
    /// would also fold non-ASCII letters.
    private static func asciiLowercased(_ name: String) -> String {
        String(decoding: name.utf8.map { byte in
            byte >= UInt8(ascii: "A") && byte <= UInt8(ascii: "Z") ? byte + 32 : byte
        }, as: UTF8.self)
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

    private static func parseLuaDeps(package: BuildPackage, knownNames: [String: String]) throws -> [String] {
        guard let rockspecPath = firstFile(in: package.path, suffix: ".rockspec"),
              let data = try? Data(contentsOf: URL(fileURLWithPath: rockspecPath)) else {
            return []
        }
        guard let content = String(data: data, encoding: .utf8) else {
            throw MetadataEncodingError(
                code: "METADATA_INVALID_UTF8",
                package: package.name,
                manifest: repositoryManifestPath(rockspecPath),
                encoding: "UTF-8"
            )
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

    private static func repositoryManifestPath(_ path: String) -> String {
        let normalized = path.replacingOccurrences(of: "\\", with: "/")
        let parts = normalized.split(separator: "/")
        var canonicalStart: Int?
        if parts.count >= 2 {
            for index in 0 ..< (parts.count - 1) where
                parts[index] == "code" && (parts[index + 1] == "packages" || parts[index + 1] == "programs") {
                canonicalStart = index
            }
        }
        if let canonicalStart {
            return parts[canonicalStart...].joined(separator: "/")
        }
        return URL(fileURLWithPath: path).lastPathComponent
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
