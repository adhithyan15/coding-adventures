import Testing
@testable import BuildToolCore

struct GlobMatchTests {
    @Test
    func portableCharacterClassesMatchPythonSemantics() {
        let cases: [(String, String, Bool)] = [
            ("src/[!a].swift", "src/b.swift", true),
            ("src/[!a].swift", "src/a.swift", false),
            ("src/[]].swift", "src/].swift", true),
            ("src/[-a].swift", "src/-.swift", true),
            ("src/[a-].swift", "src/-.swift", true),
            ("src/[a-c].swift", "src/b.swift", true),
            ("src/file[.swift", "src/file[.swift", true),
        ]

        for (pattern, path, expected) in cases {
            #expect(GlobMatch.matchPath(pattern, path) == expected)
        }
    }

    @Test
    func manyGlobstarsUseBoundedDynamicProgramming() {
        let pattern =
            Array(repeating: "**", count: 256).joined(separator: "/")
            + "/missing.swift"
        let path =
            Array(repeating: "directory", count: 256).joined(separator: "/")
            + "/present.swift"

        #expect(!GlobMatch.matchPath(pattern, path))
    }
}
