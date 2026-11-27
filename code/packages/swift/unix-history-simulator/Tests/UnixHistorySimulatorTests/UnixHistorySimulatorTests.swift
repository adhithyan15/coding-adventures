import XCTest
@testable import UnixHistorySimulator

final class UnixHistorySimulatorTests: XCTestCase {
    func testFilesystemPathsAndByteRanges() throws {
        let filesystem = FileSystem.demo()
        XCTAssertEqual(try filesystem.openFile("/hello").inode, 2)
        XCTAssertEqual(try FileSystem.readRange(Array("abc".utf8), offset: 1, count: 9), Array("bc".utf8))
        XCTAssertEqual(try FileSystem.readRange(Array("abc".utf8), offset: 3, count: 1), [])
        XCTAssertThrowsError(try FileSystem.readRange(Array("abc".utf8), offset: -1, count: 1)) {
            XCTAssertEqual($0 as? SimulatorError, .EINVAL)
        }
        XCTAssertThrowsError(try filesystem.openFile("/missing")) {
            XCTAssertEqual($0 as? SimulatorError, .ENOENT)
        }
        XCTAssertThrowsError(try filesystem.openFile("/hello/..")) {
            XCTAssertEqual($0 as? SimulatorError, .ENOTDIR)
        }
        XCTAssertThrowsError(try filesystem.openFile("/hello/")) {
            XCTAssertEqual($0 as? SimulatorError, .ENOTDIR)
        }
    }

    func testDescriptorCapacityNegativeZeroAndReuse() throws {
        XCTAssertThrowsError(try DescriptorTable(capacity: -1)) {
            XCTAssertEqual($0 as? SimulatorError, .EINVAL)
        }
        let zero = try DescriptorTable(capacity: 0)
        XCTAssertThrowsError(try zero.allocate(inode: 2, bytes: [1])) {
            XCTAssertEqual($0 as? SimulatorError, .EMFILE)
        }

        let table = try DescriptorTable(capacity: 1)
        XCTAssertEqual(try table.allocate(inode: 2, bytes: Array("xyz".utf8)), 0)
        let first = try table.read(fd: 0, count: 2)
        XCTAssertEqual(first.inode, 2)
        XCTAssertEqual(first.offset, 0)
        XCTAssertEqual(first.bytes, Array("xy".utf8))
        XCTAssertEqual(first.nextOffset, 2)
        try table.close(fd: 0)
        XCTAssertEqual(try table.allocate(inode: 3, bytes: []), 0)
    }

    func testKernelTrace() throws {
        let trace = try Kernel.demo().runCommand("cat /hello")
        XCTAssertEqual(trace.output, "Hello, Unix!\n")
        XCTAssertEqual(trace.events, [
            .opened(path: "/hello", inode: 2, fd: 0),
            .read(fd: 0, inode: 2, offset: 0, bytes: 8, nextOffset: 8),
            .read(fd: 0, inode: 2, offset: 8, bytes: 5, nextOffset: 13),
            .read(fd: 0, inode: 2, offset: 13, bytes: 0, nextOffset: 13),
            .closed(fd: 0),
        ])
    }
}
