import Foundation

public enum StarlarkEvaluator {
    public static let knownRules = [
        "py_library",
        "py_binary",
        "go_library",
        "go_binary",
        "ruby_library",
        "ruby_binary",
        "ts_library",
        "ts_binary",
        "rust_library",
        "rust_binary",
        "elixir_library",
        "elixir_binary",
        "lua_library",
        "lua_binary",
        "perl_library",
        "perl_binary",
        "swift_library",
        "swift_binary",
    ]

    public static func isStarlarkBuild(_ content: String) -> Bool {
        for rawLine in content.split(whereSeparator: \.isNewline) {
            let line = rawLine.trimmingCharacters(in: .whitespacesAndNewlines)
            if line.isEmpty || line.hasPrefix("#") {
                continue
            }
            if line.hasPrefix("load(") || line.hasPrefix("def ") {
                return true
            }
            for rule in knownRules where line.hasPrefix("\(rule)(") {
                return true
            }
            break
        }
        return false
    }

    public static func evaluateBuildFile(
        at buildFilePath: String,
        packageDirectory: String,
        repoRoot: String
    ) throws -> BuildFileEvaluation {
        _ = packageDirectory
        _ = repoRoot
        let content = try String(contentsOfFile: buildFilePath, encoding: .utf8)
        return try evaluateBuildContent(content)
    }

    public static func evaluateBuildContent(_ content: String) throws -> BuildFileEvaluation {
        let lines = content.split(whereSeparator: \.isNewline).map(String.init)
        var targets: [BuildTarget] = []
        var activeRule: String?
        var buffer: [String] = []
        var parenBalance = 0

        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            if activeRule == nil {
                guard let rule = knownRules.first(where: { trimmed.hasPrefix("\($0)(") }) else {
                    continue
                }
                activeRule = rule
                buffer = [line]
                parenBalance = countParentheses(in: line)
                if parenBalance <= 0 {
                    let call = buffer.joined(separator: "\n")
                    targets.append(try parseTarget(rule: rule, call: call))
                    activeRule = nil
                    buffer = []
                }
                continue
            }

            buffer.append(line)
            parenBalance += countParentheses(in: line)
            if parenBalance <= 0, let rule = activeRule {
                let call = buffer.joined(separator: "\n")
                targets.append(try parseTarget(rule: rule, call: call))
                activeRule = nil
                buffer = []
            }
        }

        return BuildFileEvaluation(targets: targets)
    }

    public static func generateCommands(for target: BuildTarget) -> [String] {
        switch target.rule {
        case "py_library", "py_binary":
            let testRunner = target.testRunner.isEmpty ? "pytest" : target.testRunner
            let testCommand = testRunner == "unittest"
                ? "python -m unittest discover tests/"
                : "python -m pytest --cov --cov-report=term-missing"
            return [
                "uv pip install --system -e .[dev]",
                testCommand,
            ]
        case "go_library", "go_binary":
            return [
                "go build ./...",
                "go test ./... -v -cover",
                "go vet ./...",
            ]
        case "ruby_library", "ruby_binary":
            return [
                "bundle install --quiet",
                "bundle exec rake test",
            ]
        case "ts_library", "ts_binary":
            return [
                "npm install --silent",
                "npx vitest run --coverage",
            ]
        case "rust_library", "rust_binary":
            return [
                "cargo build",
                "cargo test",
            ]
        case "elixir_library", "elixir_binary":
            return [
                "mix deps.get",
                "mix test --cover",
            ]
        case "lua_library", "lua_binary":
            return [
                "luarocks make --local",
                "busted",
            ]
        case "perl_library", "perl_binary":
            return [
                "cpanm --installdeps .",
                "prove -lr t",
            ]
        case "swift_library", "swift_binary":
            return [
                "swift build",
                "swift test",
            ]
        default:
            return []
        }
    }

    private static func countParentheses(in line: String) -> Int {
        var balance = 0
        var quote: Character?
        var escaped = false

        for character in line {
            if escaped {
                escaped = false
                continue
            }
            if character == "\\" {
                escaped = true
                continue
            }
            if let currentQuote = quote {
                if character == currentQuote {
                    quote = nil
                }
                continue
            }
            if character == "\"" || character == "'" {
                quote = character
                continue
            }
            if character == "(" {
                balance += 1
            } else if character == ")" {
                balance -= 1
            }
        }

        return balance
    }

    private static func parseTarget(rule: String, call: String) throws -> BuildTarget {
        guard let start = call.firstIndex(of: "("),
              let end = call.lastIndex(of: ")"),
              start < end else {
            throw BuildToolError.invalidPlan("invalid Starlark target call for \(rule)")
        }

        let body = String(call[call.index(after: start)..<end])
        let parts = splitTopLevel(body, separator: ",")

        var values: [String: String] = [:]
        for part in parts {
            guard let equalIndex = topLevelEqualsIndex(in: part) else {
                continue
            }
            let key = part[..<equalIndex].trimmingCharacters(in: .whitespacesAndNewlines)
            let value = part[part.index(after: equalIndex)...].trimmingCharacters(in: .whitespacesAndNewlines)
            values[key] = value
        }

        let name = parseString(values["name"]) ?? ""
        let srcs = parseStringArray(values["srcs"])
        let deps = parseStringArray(values["deps"])
        let testRunner = parseString(values["test_runner"]) ?? defaultTestRunner(for: rule)
        let entryPoint = parseString(values["entry_point"]) ?? defaultEntryPoint(for: rule)

        return BuildTarget(
            rule: rule,
            name: name,
            srcs: srcs,
            deps: deps,
            testRunner: testRunner,
            entryPoint: entryPoint
        )
    }

    private static func splitTopLevel(_ source: String, separator: Character) -> [String] {
        var parts: [String] = []
        var current = ""
        var bracketDepth = 0
        var parenDepth = 0
        var braceDepth = 0
        var quote: Character?
        var escaped = false

        for character in source {
            if escaped {
                current.append(character)
                escaped = false
                continue
            }
            if character == "\\" {
                current.append(character)
                escaped = true
                continue
            }
            if let currentQuote = quote {
                current.append(character)
                if character == currentQuote {
                    quote = nil
                }
                continue
            }
            if character == "\"" || character == "'" {
                current.append(character)
                quote = character
                continue
            }

            switch character {
            case "[":
                bracketDepth += 1
            case "]":
                bracketDepth -= 1
            case "(":
                parenDepth += 1
            case ")":
                parenDepth -= 1
            case "{":
                braceDepth += 1
            case "}":
                braceDepth -= 1
            default:
                break
            }

            if character == separator,
               bracketDepth == 0,
               parenDepth == 0,
               braceDepth == 0 {
                let trimmed = current.trimmingCharacters(in: .whitespacesAndNewlines)
                if !trimmed.isEmpty {
                    parts.append(trimmed)
                }
                current = ""
            } else {
                current.append(character)
            }
        }

        let trimmed = current.trimmingCharacters(in: .whitespacesAndNewlines)
        if !trimmed.isEmpty {
            parts.append(trimmed)
        }
        return parts
    }

    private static func topLevelEqualsIndex(in value: String) -> String.Index? {
        var bracketDepth = 0
        var parenDepth = 0
        var braceDepth = 0
        var quote: Character?
        var escaped = false
        var index = value.startIndex

        while index < value.endIndex {
            let character = value[index]
            if escaped {
                escaped = false
                index = value.index(after: index)
                continue
            }
            if character == "\\" {
                escaped = true
                index = value.index(after: index)
                continue
            }
            if let currentQuote = quote {
                if character == currentQuote {
                    quote = nil
                }
                index = value.index(after: index)
                continue
            }
            if character == "\"" || character == "'" {
                quote = character
                index = value.index(after: index)
                continue
            }

            switch character {
            case "[":
                bracketDepth += 1
            case "]":
                bracketDepth -= 1
            case "(":
                parenDepth += 1
            case ")":
                parenDepth -= 1
            case "{":
                braceDepth += 1
            case "}":
                braceDepth -= 1
            case "=" where bracketDepth == 0 && parenDepth == 0 && braceDepth == 0:
                return index
            default:
                break
            }
            index = value.index(after: index)
        }

        return nil
    }

    private static func parseString(_ rawValue: String?) -> String? {
        guard var value = rawValue?.trimmingCharacters(in: .whitespacesAndNewlines), !value.isEmpty else {
            return nil
        }
        if (value.hasPrefix("\"") && value.hasSuffix("\"")) || (value.hasPrefix("'") && value.hasSuffix("'")) {
            value.removeFirst()
            value.removeLast()
            value = value.replacingOccurrences(of: "\\\"", with: "\"")
            value = value.replacingOccurrences(of: "\\'", with: "'")
            value = value.replacingOccurrences(of: "\\\\", with: "\\")
        }
        return value
    }

    private static func parseStringArray(_ rawValue: String?) -> [String] {
        guard let rawValue else {
            return []
        }
        let trimmed = rawValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.hasPrefix("["),
              trimmed.hasSuffix("]") else {
            return []
        }

        let start = trimmed.index(after: trimmed.startIndex)
        let end = trimmed.index(before: trimmed.endIndex)
        let body = String(trimmed[start..<end])
        return splitTopLevel(body, separator: ",")
            .compactMap(parseString)
    }

    private static func defaultTestRunner(for rule: String) -> String {
        switch rule {
        case "py_library", "py_binary":
            return "pytest"
        case "ruby_library":
            return "minitest"
        case "ts_library":
            return "vitest"
        default:
            return ""
        }
    }

    private static func defaultEntryPoint(for rule: String) -> String {
        switch rule {
        case "py_binary":
            return "main.py"
        case "ruby_binary":
            return "main.rb"
        case "ts_binary":
            return "src/index.ts"
        case "elixir_binary":
            return "lib/main.ex"
        default:
            return ""
        }
    }
}
