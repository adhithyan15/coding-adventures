import BuildToolCore
import Foundation
import Testing

struct PlanTests {
    @Test
    func writeAndReadPlanRoundTrips() throws {
        let root = try makeTempDirectory(label: "plan")
        defer { try? FileManager.default.removeItem(atPath: root) }

        let planPath = (root as NSString).appendingPathComponent("build-plan.json")
        let plan = BuildPlan(
            schemaVersion: 1,
            diffBase: "origin/main",
            force: false,
            affectedPackages: ["swift/trig"],
            packages: [
                PackageEntry(
                    name: "swift/trig",
                    relPath: "code/packages/swift/trig",
                    language: "swift",
                    buildCommands: ["swift test"],
                    isStarlark: false
                ),
            ],
            dependencyEdges: [["swift/trig", "swift/arc2d"]],
            languagesNeeded: ["swift": true]
        )

        try PlanIO.writePlan(plan, to: planPath)
        let decoded = try PlanIO.readPlan(from: planPath)

        #expect(decoded.diffBase == plan.diffBase)
        #expect(decoded.affectedPackages == plan.affectedPackages)
        #expect(decoded.packages.first?.name == "swift/trig")
        #expect(decoded.dependencyEdges == [["swift/trig", "swift/arc2d"]])
    }
}
