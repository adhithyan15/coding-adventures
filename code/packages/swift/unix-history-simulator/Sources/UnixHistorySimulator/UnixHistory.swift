//
// UnixHistory.swift — Pure First Edition Unix teaching components.
//
// Filesystem contents and descriptor state live in memory. This module does
// not read fixtures, access the host filesystem, launch processes, or print.
//

public enum SimulatorError: Error, Equatable {
    case ENOENT
    case ENOTDIR
    case EISDIR
    case EBADF
    case EMFILE
    case EINVAL

    public var code: String {
        switch self {
        case .ENOENT: "ENOENT"
        case .ENOTDIR: "ENOTDIR"
        case .EISDIR: "EISDIR"
        case .EBADF: "EBADF"
        case .EMFILE: "EMFILE"
        case .EINVAL: "EINVAL"
        }
    }
}

public enum Event: Equatable {
    case opened(path: String, inode: Int, fd: Int)
    case openFailed(path: String, error: SimulatorError)
    case read(fd: Int, inode: Int, offset: Int, bytes: Int, nextOffset: Int)
    case readFailed(fd: Int, error: SimulatorError)
    case closed(fd: Int)
    case closeFailed(fd: Int, error: SimulatorError)
    case listed(path: String, inode: Int, names: [String])
    case listFailed(path: String, error: SimulatorError)

}

public struct Trace: Equatable {
    public let output: String
    public let events: [Event]

    public init(output: String, events: [Event]) {
        self.output = output
        self.events = events
    }
}

public struct FileSystem {
    private enum Node {
        case file(inode: Int, name: String, bytes: [UInt8])
        case directory(inode: Int, name: String, entries: [Node])

        var name: String {
            switch self {
            case let .file(_, name, _), let .directory(_, name, _): name
            }
        }

        var inode: Int {
            switch self {
            case let .file(inode, _, _), let .directory(inode, _, _): inode
            }
        }
    }

    private let root: Node

    public init(root: Int = 1) {
        self.root = .directory(
            inode: root,
            name: "/",
            entries: [
                .file(inode: 2, name: "hello", bytes: Array("Hello, Unix!\n".utf8)),
                .file(inode: 3, name: "empty", bytes: []),
                .directory(
                    inode: 4,
                    name: "etc",
                    entries: [.file(inode: 5, name: "notes", bytes: [])]
                ),
            ]
        )
    }

    public static func demo() -> FileSystem { FileSystem() }

    private func resolve(_ path: String) throws -> Node {
        guard !path.isEmpty, !path.utf8.contains(0) else { throw SimulatorError.EINVAL }
        let parts = path.split(separator: "/", omittingEmptySubsequences: true).map(String.init)
        var stack = [root]
        for part in parts where part != "." {
            if part == ".." {
                guard case .directory = stack[stack.count - 1] else { throw SimulatorError.ENOTDIR }
                if stack.count > 1 { stack.removeLast() }
                continue
            }
            guard case let .directory(_, _, entries) = stack[stack.count - 1] else {
                throw SimulatorError.ENOTDIR
            }
            guard let child = entries.first(where: { $0.name == part }) else {
                throw SimulatorError.ENOENT
            }
            stack.append(child)
        }
        let result = stack[stack.count - 1]
        if path.hasSuffix("/"), case .file = result { throw SimulatorError.ENOTDIR }
        return result
    }

    public func openFile(_ path: String) throws -> (inode: Int, bytes: [UInt8]) {
        switch try resolve(path) {
        case let .file(inode, _, bytes): (inode, bytes)
        case .directory: throw SimulatorError.EISDIR
        }
    }

    public func list(_ path: String) throws -> (inode: Int, names: [String]) {
        switch try resolve(path) {
        case .file: throw SimulatorError.ENOTDIR
        case let .directory(inode, _, entries): (inode, entries.map(\.name))
        }
    }

    public static func readRange(_ bytes: [UInt8], offset: Int, count: Int) throws -> [UInt8] {
        guard offset >= 0, count >= 0 else { throw SimulatorError.EINVAL }
        guard offset < bytes.count else { return [] }
        let length = min(count, bytes.count - offset)
        return Array(bytes[offset ..< offset + length])
    }
}

public final class DescriptorTable {
    private struct OpenFile {
        let inode: Int
        let bytes: [UInt8]
        var offset: Int
    }

    private let capacity: Int
    private var slots: [Int: OpenFile] = [:]

    public convenience init() {
        self.init(validatedCapacity: 4)
    }

    public convenience init(capacity: Int) throws {
        guard capacity >= 0 else { throw SimulatorError.EINVAL }
        self.init(validatedCapacity: capacity)
    }

    private init(validatedCapacity: Int) {
        capacity = validatedCapacity
    }

    public func allocate(inode: Int, bytes: [UInt8]) throws -> Int {
        guard let fd = (0 ..< capacity).first(where: { slots[$0] == nil }) else {
            throw SimulatorError.EMFILE
        }
        slots[fd] = OpenFile(inode: inode, bytes: bytes, offset: 0)
        return fd
    }

    public func read(fd: Int, count: Int) throws -> (inode: Int, offset: Int, bytes: [UInt8], nextOffset: Int) {
        guard count >= 0 else { throw SimulatorError.EINVAL }
        guard var entry = slots[fd] else { throw SimulatorError.EBADF }
        let chunk = try FileSystem.readRange(entry.bytes, offset: entry.offset, count: count)
        let offset = entry.offset
        entry.offset += chunk.count
        slots[fd] = entry
        return (entry.inode, offset, chunk, entry.offset)
    }

    public func close(fd: Int) throws {
        guard slots.removeValue(forKey: fd) != nil else { throw SimulatorError.EBADF }
    }

    public func offset(fd: Int) throws -> Int {
        guard let entry = slots[fd] else { throw SimulatorError.EBADF }
        return entry.offset
    }
}

public final class Kernel {
    private let filesystem: FileSystem
    private let descriptors: DescriptorTable

    public init(filesystem: FileSystem, descriptors: DescriptorTable) {
        self.filesystem = filesystem
        self.descriptors = descriptors
    }

    public static func demo(capacity: Int = 4) throws -> Kernel {
        Kernel(filesystem: .demo(), descriptors: try DescriptorTable(capacity: capacity))
    }

    public func openFile(_ path: String) throws -> (fd: Int, inode: Int) {
        let file = try filesystem.openFile(path)
        return (try descriptors.allocate(inode: file.inode, bytes: file.bytes), file.inode)
    }

    public func read(fd: Int, count: Int) throws -> (inode: Int, offset: Int, bytes: [UInt8], nextOffset: Int) {
        try descriptors.read(fd: fd, count: count)
    }

    public func close(fd: Int) throws { try descriptors.close(fd: fd) }
    public func list(_ path: String) throws -> (inode: Int, names: [String]) { try filesystem.list(path) }

    public func runCommand(_ command: String) -> Trace {
        var output = ""
        var events: [Event] = []
        let asciiWhitespace: Set<Character> = [" ", "\t", "\n", "\r", "\u{000B}", "\u{000C}"]
        let words = command.split(whereSeparator: { asciiWhitespace.contains($0) }).map(String.init)

        if words.count == 2, words[0] == "cat" {
            let path = words[1]
            do {
                let opened = try openFile(path)
                events.append(.opened(path: path, inode: opened.inode, fd: opened.fd))
                while true {
                    do {
                        let chunk = try read(fd: opened.fd, count: 8)
                        events.append(.read(
                            fd: opened.fd,
                            inode: chunk.inode,
                            offset: chunk.offset,
                            bytes: chunk.bytes.count,
                            nextOffset: chunk.nextOffset
                        ))
                        output += String(decoding: chunk.bytes, as: UTF8.self)
                        if chunk.bytes.isEmpty { break }
                    } catch let error as SimulatorError {
                        events.append(.readFailed(fd: opened.fd, error: error))
                        break
                    } catch {
                        events.append(.readFailed(fd: opened.fd, error: .EINVAL))
                        break
                    }
                }
                do {
                    try close(fd: opened.fd)
                    events.append(.closed(fd: opened.fd))
                } catch let error as SimulatorError {
                    events.append(.closeFailed(fd: opened.fd, error: error))
                } catch {
                    events.append(.closeFailed(fd: opened.fd, error: .EINVAL))
                }
            } catch let error as SimulatorError {
                events.append(.openFailed(path: path, error: error))
                output = "error: \(error.code)\n"
            } catch {
                events.append(.openFailed(path: path, error: .EINVAL))
                output = "error: EINVAL\n"
            }
        } else if words.first == "ls", words.count <= 2 {
            let path = words.count == 2 ? words[1] : "/"
            do {
                let listing = try list(path)
                events.append(.listed(path: path, inode: listing.inode, names: listing.names))
                output = listing.names.map { $0 + "\n" }.joined()
            } catch let error as SimulatorError {
                events.append(.listFailed(path: path, error: error))
                output = "error: \(error.code)\n"
            } catch {
                events.append(.listFailed(path: path, error: .EINVAL))
                output = "error: EINVAL\n"
            }
        } else if words.isEmpty {
            output = ""
        } else {
            output = "usage: cat PATH or ls [PATH]\n"
        }
        return Trace(output: output, events: events)
    }
}
