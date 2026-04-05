import BuildToolCore
import Foundation
import Testing

struct DiscoveryTests {
    @Test
    func inferLanguageIncludesSwift() {
        #expect(Discovery.inferLanguage(path: "/repo/code/packages/swift/trig") == "swift")
    }

    @Test
    func discoverPackagesFindsSwiftPackage() throws {
        let root = try makeTempDirectory(label: "discovery")
        defer { try? FileManager.default.removeItem(atPath: root) }

        try writeFile(
            (root as NSString).appendingPathComponent("packages/swift/trig/BUILD"),
            "swift test\n"
        )
        try writeFile(
            (root as NSString).appendingPathComponent("packages/python/logic-gates/BUILD"),
            "pytest\n"
        )
        try writeFile(
            (root as NSString).appendingPathComponent("packages/swift/trig/Sources/Trig/main.swift"),
            "print(\"hi\")\n"
        )

        let packages = Discovery.discoverPackages(root: root)
        #expect(packages.map(\.name) == ["python/logic-gates", "swift/trig"])
        #expect(packages.last?.language == "swift")
        #expect(packages.last?.buildCommands == ["swift test"])
    }
}
