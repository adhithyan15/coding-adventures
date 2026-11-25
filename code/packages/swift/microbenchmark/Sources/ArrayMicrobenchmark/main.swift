// Command-line entry point for the paired indexed-array experiment.
import Foundation
import Microbenchmark

do {
    switch try runCommandLine(arguments: Array(CommandLine.arguments.dropFirst())) {
    case .help:
        let usage = "Usage: array-microbenchmark [--pairs N] [--confidence P] [--factor F] [--seed N] [--target-ns N] [--max-batch N] [--revision ID]\n"
        try FileHandle.standardOutput.write(contentsOf: Data(usage.utf8))
    case .report(let report):
        let output = try encodeReport(report) + "\n"
        try FileHandle.standardOutput.write(contentsOf: Data(output.utf8))
    }
} catch {
    let diagnostic = "array-microbenchmark: \(error.localizedDescription)\n"
    try? FileHandle.standardError.write(contentsOf: Data(diagnostic.utf8))
    exit(EXIT_FAILURE)
}
