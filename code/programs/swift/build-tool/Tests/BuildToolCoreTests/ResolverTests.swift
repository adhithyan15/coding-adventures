import BuildToolCore
import Foundation
import Testing

struct ResolverTests {
    @Test
    func parseSwiftDepsReadsPackageManifestPaths() throws {
        let root = try makeTempDirectory(label: "resolver")
        defer { try? FileManager.default.removeItem(atPath: root) }

        let packagePath = (root as NSString).appendingPathComponent("code/packages/swift/arc2d")
        try writeFile(
            (packagePath as NSString).appendingPathComponent("Package.swift"),
            """
            import PackageDescription

            let package = Package(
                name: "arc2d",
                dependencies: [
                    .package(path: "../trig"),
                    .package(path: "../point2d"),
                ]
            )
            """
        )

        let package = BuildPackage(name: "swift/arc2d", path: packagePath, language: "swift")
        let deps = Resolver.parseSwiftDeps(
            package: package,
            knownNames: [
                "trig": "swift/trig",
                "point2d": "swift/point2d",
            ]
        )

        #expect(deps == ["swift/trig", "swift/point2d"])
    }

    @Test
    func resolveDependenciesCreatesSwiftEdge() throws {
        let root = try makeTempDirectory(label: "resolver_graph")
        defer { try? FileManager.default.removeItem(atPath: root) }
        let trigPath = (root as NSString).appendingPathComponent("code/packages/swift/trig")
        let arcPath = (root as NSString).appendingPathComponent("code/packages/swift/arc2d")
        try writeFile((trigPath as NSString).appendingPathComponent("Package.swift"), "import PackageDescription\nlet package = Package(name: \"trig\")\n")
        try writeFile(
            (arcPath as NSString).appendingPathComponent("Package.swift"),
            """
            import PackageDescription
            let package = Package(
                name: "arc2d",
                dependencies: [.package(path: "../trig")]
            )
            """
        )

        let graph = Resolver.resolveDependencies(
            packages: [
                BuildPackage(name: "swift/trig", path: trigPath, language: "swift"),
                BuildPackage(name: "swift/arc2d", path: arcPath, language: "swift"),
            ]
        )

        #expect(graph.successors(of: "swift/trig") == ["swift/arc2d"])
        #expect(graph.predecessors(of: "swift/arc2d") == ["swift/trig"])
    }
}
