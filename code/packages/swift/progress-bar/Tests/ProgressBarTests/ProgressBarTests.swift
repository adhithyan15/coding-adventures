import XCTest
@testable import ProgressBar

final class ProgressBarTests: XCTestCase {
    final class Output: @unchecked Sendable {
        let lock = NSLock()
        var lines: [String] = []
        
        @Sendable func write(_ str: String) {
            lock.lock()
            lines.append(str)
            lock.unlock()
        }
    }
    
    func testFlatProgressBar() {
        let output = Output()
        let t = Tracker(total: 2, writer: output.write)
        t.start()
        
        t.send(Event(type: .started, name: "pkg-a"))
        Thread.sleep(forTimeInterval: 0.1) // Let renderer process
        
        t.send(Event(type: .finished, name: "pkg-a"))
        t.send(Event(type: .skipped, name: "pkg-b"))
        t.stop()
        
        XCTAssertEqual(t.completed, 2)
        XCTAssertEqual(t.total, 2)
        
        output.lock.lock()
        let finalLines = output.lines
        output.lock.unlock()
        
        XCTAssertTrue(finalLines.contains { $0.contains("Building: pkg-a") })
        XCTAssertTrue(finalLines.contains { $0.contains("done") })
        XCTAssertTrue(finalLines.contains { $0 == "\n" })
    }
    
    func testHierarchicalProgressBar() {
        let output = Output()
        let parent = Tracker(total: 3, writer: output.write, label: "Level")
        parent.start()
        
        let child = parent.child(total: 1, label: "Package")
        child.send(Event(type: .started, name: "pkg-a"))
        child.send(Event(type: .finished, name: "pkg-a"))
        child.finish()
        
        parent.stop()
        
        XCTAssertEqual(parent.completed, 1)
        XCTAssertEqual(child.completed, 1)
        
        output.lock.lock()
        let finalLines = output.lines
        output.lock.unlock()
        
        XCTAssertTrue(finalLines.contains { $0.contains("Level 1/3") })
        XCTAssertTrue(finalLines.contains { $0.contains("Package") == false }, "Parent updates should reflect its own status")
    }

    func testThreadSafety() {
        let t = Tracker(total: 100)
        t.start()
        
        let group = DispatchGroup()
        for i in 0..<100 {
            DispatchQueue.global().async(group: group) {
                t.send(Event(type: .started, name: "item-\(i)"))
                Thread.sleep(forTimeInterval: 0.001)
                t.send(Event(type: .finished, name: "item-\(i)"))
            }
        }
        
        group.wait()
        t.stop()
        
        XCTAssertEqual(t.completed, 100)
    }
}
