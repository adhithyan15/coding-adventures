import BuildToolCore
import Testing

struct StarlarkTests {
    @Test
    func evaluatesSimpleStarlarkBuild() throws {
        let content = """
        load("code/packages/starlark/library-rules/typescript_library.star", "ts_library")

        _targets = [
            ts_library(
                name = "arithmetic",
                srcs = ["src/**/*.ts", "tests/**/*.ts"],
                deps = ["typescript/logic-gates"],
                test_runner = "vitest",
            ),
        ]
        """

        let result = try StarlarkEvaluator.evaluateBuildContent(content)
        #expect(result.targets.count == 1)
        #expect(result.targets.first?.name == "arithmetic")
        #expect(result.targets.first?.deps == ["typescript/logic-gates"])
        #expect(
            StarlarkEvaluator.generateCommands(for: result.targets[0])
                == ["npm install --silent", "npx vitest run --coverage"]
        )
    }

    @Test
    func expandAffectedSetIncludesPrerequisites() {
        let graph = DirectedGraph()
        graph.addEdge(from: "swift/trig", to: "swift/point2d")
        graph.addEdge(from: "swift/point2d", to: "swift/arc2d")

        let expanded = BuildTool.expandAffectedSetWithPrereqs(graph, affectedSet: ["swift/arc2d"])
        #expect(expanded == ["swift/trig", "swift/point2d", "swift/arc2d"])
    }
}
