import Foundation
import XCTest

@testable import CliBuilder

final class CliBuilderTests: XCTestCase {
  func testValidationRejectsDuplicateFlagIDsAndCycles() throws {
    let duplicate = """
      {
        "cli_builder_spec_version": "1.0",
        "name": "tool",
        "description": "A tool",
        "flags": [
          {"id":"same", "long":"first", "type":"boolean"},
          {"id":"same", "long":"second", "type":"boolean"}
        ]
      }
      """
    let result = try validateSpec(duplicate)
    XCTAssertFalse(result.isValid)
    XCTAssertTrue(result.errors.joined(separator: "\n").contains("duplicate flag id"))

    let cycle = """
      {
        "cli_builder_spec_version": "1.0",
        "name": "tool",
        "description": "A tool",
        "flags": [
          {"id":"a", "long":"a", "type":"boolean", "requires":["b"]},
          {"id":"b", "long":"b", "type":"boolean", "requires":["a"]}
        ]
      }
      """
    let cycleResult = try validateSpec(cycle)
    XCTAssertFalse(cycleResult.isValid)
    XCTAssertTrue(cycleResult.errors.contains { $0.contains("circular requires") })
  }

  func testTokenClassifierHandlesStacksInlineValuesAndSingleDashLong() {
    let classifier = TokenClassifier([
      FlagDefinition(id: "verbose", shortName: "v", longName: "verbose", type: .count),
      FlagDefinition(id: "output", shortName: "o", longName: "output", type: .string),
      FlagDefinition(id: "classpath", singleDashLong: "classpath", type: .string),
    ])
    XCTAssertEqual(classifier.classify("-vvv").kind, .stackedFlags)
    XCTAssertEqual(classifier.classify("-classpath").kind, .singleDashLong)
    XCTAssertEqual(classifier.classify("--output=file").kind, .longFlagWithValue)
    XCTAssertEqual(classifier.classify("-ofile").kind, .shortFlagWithValue)
    XCTAssertEqual(classifier.classify("--").kind, .endOfFlags)
  }

  func testParsesRootFlagsArgumentsAndCounts() throws {
    let outcome = try parse(baseSpec, ["paint", "--output", "out.png", "-vv", "scene.cad"])
    guard case .parsed(let result) = outcome else { return XCTFail("expected parsed result") }
    XCTAssertEqual(result.commandPath, ["paint"])
    XCTAssertEqual(result.flags["output"], .string("out.png"))
    XCTAssertEqual(result.flags["verbose"], .int(2))
    XCTAssertEqual(result.arguments["input"], .string("scene.cad"))
    XCTAssertEqual(result.explicitFlags, ["output", "verbose", "verbose"])
  }

  func testEnumDefaultWhenPresentAndNestedCommandInheritance() throws {
    let root = try parse(baseSpec, ["paint", "--color", "scene.cad"])
    guard case .parsed(let rootResult) = root else { return XCTFail("expected parsed result") }
    XCTAssertEqual(rootResult.flags["color"], .string("always"))
    XCTAssertEqual(rootResult.arguments["input"], .string("scene.cad"))

    let nested = try parse(baseSpec, ["paint", "serve", "status", "--port", "8080", "-v"])
    guard case .parsed(let nestedResult) = nested else { return XCTFail("expected parsed result") }
    XCTAssertEqual(nestedResult.commandPath, ["paint", "serve", "status"])
    XCTAssertEqual(nestedResult.flags["port"], .int(8080))
    XCTAssertEqual(nestedResult.flags["verbose"], .int(1))
  }

  func testHelpAndVersionBuiltins() throws {
    let help = try parse(baseSpec, ["paint", "serve", "--help"])
    guard case .help(let result) = help else { return XCTFail("expected help") }
    XCTAssertTrue(result.text.contains("USAGE"))
    XCTAssertTrue(result.text.contains("GLOBAL OPTIONS"))
    XCTAssertEqual(result.commandPath, ["paint", "serve"])

    let version = try parse(baseSpec, ["paint", "--version"])
    XCTAssertEqual(version, .version(VersionResult(version: "1.2.3")))
  }

  func testDependencyConflictAndExclusiveGroupErrorsAreAggregated() throws {
    XCTAssertThrowsError(try parse(baseSpec, ["paint", "--profile", "scene.cad"])) { error in
      guard let parseErrors = error as? ParseErrors else { return XCTFail("expected ParseErrors") }
      XCTAssertTrue(parseErrors.errors.contains { $0.errorType == "missing_dependency_flag" })
    }

    let spec = """
      {
        "cli_builder_spec_version":"1.0",
        "name":"tool",
        "description":"A tool",
        "flags":[
          {"id":"json", "long":"json", "type":"boolean", "conflicts_with":["yaml"]},
          {"id":"yaml", "long":"yaml", "type":"boolean", "conflicts_with":["json"]}
        ],
        "mutually_exclusive_groups":[
          {"id":"format", "flag_ids":["json","yaml"], "required":false}
        ]
      }
      """
    XCTAssertThrowsError(try parse(spec, ["tool", "--json", "--yaml"])) { error in
      guard let parseErrors = error as? ParseErrors else { return XCTFail("expected ParseErrors") }
      XCTAssertEqual(parseErrors.errors.filter { $0.errorType == "conflicting_flags" }.count, 1)
      XCTAssertEqual(
        parseErrors.errors.filter { $0.errorType == "exclusive_group_violation" }.count, 1)
    }
  }

  func testVariadicArgumentsUseLastWinsPartitioning() throws {
    let spec = """
      {
        "cli_builder_spec_version":"1.0",
        "name":"cp",
        "description":"Copy files",
        "arguments":[
          {"id":"sources", "name":"SOURCE", "type":"string", "variadic":true, "variadic_min":1},
          {"id":"destination", "name":"DEST", "type":"string", "required":true}
        ]
      }
      """
    let outcome = try parse(spec, ["cp", "a.txt", "b.txt", "/dest"])
    guard case .parsed(let result) = outcome else { return XCTFail("expected parsed result") }
    XCTAssertEqual(result.arguments["sources"], .array([.string("a.txt"), .string("b.txt")]))
    XCTAssertEqual(result.arguments["destination"], .string("/dest"))
  }

  func testRepeatableValuesCoercionEndOfFlagsAndSuggestions() throws {
    let spec = """
      {
        "cli_builder_spec_version":"1.0",
        "name":"tool",
        "description":"A tool",
        "flags":[
          {"id":"define", "short":"D", "long":"define", "type":"string", "repeatable":true},
          {"id":"jobs", "long":"jobs", "type":"integer"}
        ],
        "arguments":[
          {"id":"values", "name":"VALUE", "type":"string", "required":false, "variadic":true, "variadic_min":0}
        ]
      }
      """
    let outcome = try parse(
      spec, ["tool", "-Done", "--define=two", "--jobs", "4", "--", "--literal"])
    guard case .parsed(let result) = outcome else { return XCTFail("expected parsed result") }
    XCTAssertEqual(result.flags["define"], .array([.string("one"), .string("two")]))
    XCTAssertEqual(result.flags["jobs"], .int(4))
    XCTAssertEqual(result.arguments["values"], .array([.string("--literal")]))

    XCTAssertThrowsError(try parse(spec, ["tool", "--jbos", "4"])) { error in
      guard let parseErrors = error as? ParseErrors else { return XCTFail("expected ParseErrors") }
      XCTAssertEqual(parseErrors.errors.first?.errorType, "unknown_flag")
      XCTAssertEqual(parseErrors.errors.first?.suggestion, "Did you mean \"--jobs\"?")
    }
  }

  func testPosixSubcommandFirstAndTraditionalModes() throws {
    let posix = """
      {
        "cli_builder_spec_version":"1.0", "name":"tool", "description":"A tool", "parsing_mode":"posix",
        "flags":[{"id":"verbose", "short":"v", "type":"boolean"}],
        "arguments":[{"id":"rest", "name":"REST", "type":"string", "required":false, "variadic":true, "variadic_min":0}]
      }
      """
    let posixOutcome = try parse(posix, ["tool", "input", "-v"])
    guard case .parsed(let posixResult) = posixOutcome else {
      return XCTFail("expected parsed result")
    }
    XCTAssertEqual(posixResult.flags["verbose"], .bool(false))
    XCTAssertEqual(posixResult.arguments["rest"], .array([.string("input"), .string("-v")]))

    let subcommandFirst = """
      {
        "cli_builder_spec_version":"1.0", "name":"tool", "description":"A tool", "parsing_mode":"subcommand_first",
        "commands":[{"id":"run", "name":"run", "description":"Run", "arguments":[], "flags":[], "commands":[]}]
      }
      """
    XCTAssertThrowsError(try parse(subcommandFirst, ["tool", "rum"])) { error in
      guard let parseErrors = error as? ParseErrors else { return XCTFail("expected ParseErrors") }
      XCTAssertEqual(parseErrors.errors.first?.errorType, "unknown_command")
      XCTAssertEqual(parseErrors.errors.first?.suggestion, "Did you mean \"run\"?")
    }

    let traditional = """
      {
        "cli_builder_spec_version":"1.0", "name":"tar", "description":"Archive", "parsing_mode":"traditional",
        "flags":[
          {"id":"extract", "short":"x", "type":"boolean"},
          {"id":"verbose", "short":"v", "type":"count"},
          {"id":"file", "short":"f", "type":"string"}
        ]
      }
      """
    let traditionalOutcome = try parse(traditional, ["tar", "xvf", "archive.tar"])
    guard case .parsed(let traditionalResult) = traditionalOutcome else {
      return XCTFail("expected parsed result")
    }
    XCTAssertEqual(traditionalResult.flags["extract"], .bool(true))
    XCTAssertEqual(traditionalResult.flags["verbose"], .int(1))
    XCTAssertEqual(traditionalResult.flags["file"], .string("archive.tar"))
  }

  func testSpecLoaderReadsAFile() throws {
    let url = FileManager.default.temporaryDirectory
      .appendingPathComponent("swift-cli-builder-\(UUID().uuidString).json")
    try baseSpec.write(to: url, atomically: true, encoding: .utf8)
    defer { try? FileManager.default.removeItem(at: url) }
    let parser = try Parser(specPath: url.path, argv: ["paint", "scene.cad"])
    guard case .parsed(let result) = try parser.parse() else {
      return XCTFail("expected parsed result")
    }
    XCTAssertEqual(result.arguments["input"], .string("scene.cad"))
  }

  private func parse(_ json: String, _ argv: [String]) throws -> ParseOutcome {
    let spec = try SpecLoader().load(from: json)
    return try Parser(spec: spec, argv: argv).parse()
  }

  private var baseSpec: String {
    """
    {
      "cli_builder_spec_version":"1.0",
      "name":"paint",
      "description":"Paint things",
      "version":"1.2.3",
      "parsing_mode":"gnu",
      "builtin_flags":{"help":true,"version":true},
      "global_flags":[
        {"id":"verbose", "short":"v", "long":"verbose", "description":"Verbose output", "type":"count"}
      ],
      "flags":[
        {"id":"output", "short":"o", "long":"output", "description":"Output path", "type":"string"},
        {"id":"color", "long":"color", "description":"Color mode", "type":"enum", "enum_values":["always","never"], "default_when_present":"always"},
        {"id":"profile", "long":"profile", "description":"Enable profiling", "type":"boolean", "requires":["config"]},
        {"id":"config", "long":"config", "description":"Config file", "type":"string"}
      ],
      "arguments":[
        {"id":"input", "display_name":"INPUT", "description":"Input source", "type":"string", "required":true}
      ],
      "commands":[
        {
          "id":"serve", "name":"serve", "aliases":["srv"], "description":"Serve content", "inherit_global_flags":true,
          "flags":[{"id":"port", "long":"port", "description":"Port", "type":"integer"}],
          "arguments":[],
          "commands":[
            {"id":"status", "name":"status", "description":"Show status", "flags":[], "arguments":[], "commands":[]}
          ]
        }
      ]
    }
    """
  }
}
