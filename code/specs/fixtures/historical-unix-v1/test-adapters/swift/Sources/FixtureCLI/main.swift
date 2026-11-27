// Test-only JSON adapter; the library returns typed in-memory traces.
import Foundation
import UnixHistorySimulator

func eventObject(_ event: Event) -> [String: Any] {
    switch event {
    case let .opened(path, inode, fd):
        ["call": "open", "path": path, "inode": inode, "fd": fd, "offset": 0]
    case let .openFailed(path, error):
        ["call": "open", "path": path, "error": error.code]
    case let .read(fd, inode, offset, bytes, nextOffset):
        ["call": "read", "fd": fd, "inode": inode, "offset": offset,
         "bytes": bytes, "nextOffset": nextOffset]
    case let .readFailed(fd, error):
        ["call": "read", "fd": fd, "error": error.code]
    case let .closed(fd):
        ["call": "close", "fd": fd]
    case let .closeFailed(fd, error):
        ["call": "close", "fd": fd, "error": error.code]
    case let .listed(path, inode, names):
        ["call": "list", "path": path, "inode": inode, "names": names]
    case let .listFailed(path, error):
        ["call": "list", "path": path, "error": error.code]
    }
}

let arguments = Array(CommandLine.arguments.dropFirst())
guard arguments.count == 1 else {
    preconditionFailure("expected exactly one shell command")
}

do {
    let trace = try Kernel.demo().runCommand(arguments[0])
    let payload: [String: Any] = [
        "output": trace.output,
        "events": trace.events.map(eventObject),
    ]
    var encoded = try JSONSerialization.data(withJSONObject: payload, options: [.fragmentsAllowed])
    encoded.append(0x0A)
    FileHandle.standardOutput.write(encoded)
} catch {
    preconditionFailure("fixture adapter failed: \(error)")
}
