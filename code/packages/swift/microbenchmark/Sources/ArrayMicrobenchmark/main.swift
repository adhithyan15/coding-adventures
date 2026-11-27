// Command-line entry point for the paired indexed-array experiment.
import Foundation
import Microbenchmark
#if os(Windows)
import WinSDK
#elseif canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

do {
    switch try runCommandLine(
        arguments: Array(CommandLine.arguments.dropFirst()),
        environment: revisionEnvironment()
    ) {
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

/// Read only the revision metadata keys used by the command-line report.
private func revisionEnvironment() -> [String: String] {
    var result: [String: String] = [:]
    for key in ["GIT_REVISION", "GITHUB_SHA"] {
        if let value = platformEnvironmentValue(named: key) {
            result[key] = value
        }
    }
    return result
}

/// Reads one environment variable without materializing the full environment.
private func platformEnvironmentValue(named name: String) -> String? {
    #if os(Windows)
        return windowsEnvironmentValue(named: name)
    #elseif canImport(Darwin) || canImport(Glibc)
        return name.withCString { namePointer in
            guard let value = getenv(namePointer) else {
                return nil
            }
            return String(validatingUTF8: value)
        }
    #else
        return nil
    #endif
}

#if os(Windows)
private func windowsEnvironmentValue(named name: String) -> String? {
    let nameBuffer = Array(name.utf16) + [0]
    return nameBuffer.withUnsafeBufferPointer { namePointer -> String? in
        guard let nameAddress = namePointer.baseAddress else {
            return nil
        }

        let successCode = DWORD(bitPattern: ERROR_SUCCESS)
        let variableMissingCode = DWORD(bitPattern: ERROR_ENVVAR_NOT_FOUND)
        SetLastError(successCode)
        let required = GetEnvironmentVariableW(nameAddress, nil, 0)
        if required == 0 {
            let errorCode = GetLastError()
            if errorCode == variableMissingCode {
                return nil
            } else if errorCode == successCode {
                return ""
            }
            return nil
        }

        // Windows limits an individual environment string to 32,767 UTF-16
        // code units; allow one extra slot for the terminating NUL.
        let maximumBufferLength: DWORD = 32_768
        guard required <= maximumBufferLength else {
            return nil
        }
        var valueBuffer = [WCHAR](repeating: 0, count: Int(required))
        for _ in 0..<2 {
            SetLastError(successCode)
            let copied = valueBuffer.withUnsafeMutableBufferPointer { buffer in
                GetEnvironmentVariableW(nameAddress, buffer.baseAddress, DWORD(buffer.count))
            }
            if copied == 0 {
                let errorCode = GetLastError()
                if errorCode == variableMissingCode {
                    return nil
                } else if errorCode == successCode {
                    return ""
                }
                return nil
            }
            if Int(copied) < valueBuffer.count {
                return String(decoding: valueBuffer.prefix(Int(copied)), as: UTF16.self)
            }
            guard copied <= maximumBufferLength else {
                return nil
            }
            valueBuffer = [WCHAR](repeating: 0, count: Int(copied))
        }
        return nil
    }
}
#endif
