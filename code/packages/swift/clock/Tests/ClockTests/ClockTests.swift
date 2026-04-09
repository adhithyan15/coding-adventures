import XCTest
@testable import Clock

final class ClockTests: XCTestCase {
    
    func testClockInitialization() {
        let clock = Clock(frequencyHz: 2_000_000) // 2 MHz
        XCTAssertEqual(clock.frequencyHz, 2_000_000)
        XCTAssertEqual(clock.cycle, 0)
        XCTAssertEqual(clock.value, 0)
        XCTAssertEqual(clock.totalTicks, 0)
        XCTAssertEqual(clock.periodNs, 500.0)
    }

    func testClockTick() {
        let clock = Clock()
        
        let edge1 = clock.tick()
        XCTAssertEqual(edge1.value, 1)
        XCTAssertEqual(edge1.isRising, true)
        XCTAssertEqual(edge1.isFalling, false)
        XCTAssertEqual(edge1.cycle, 1)
        XCTAssertEqual(clock.cycle, 1)
        XCTAssertEqual(clock.totalTicks, 1)
        
        let edge2 = clock.tick()
        XCTAssertEqual(edge2.value, 0)
        XCTAssertEqual(edge2.isRising, false)
        XCTAssertEqual(edge2.isFalling, true)
        XCTAssertEqual(edge2.cycle, 1)
        XCTAssertEqual(clock.cycle, 1)
        XCTAssertEqual(clock.totalTicks, 2)
    }

    func testClockRun() {
        let clock = Clock()
        let edges = clock.run(cycles: 3)
        XCTAssertEqual(edges.count, 6)
        XCTAssertEqual(clock.cycle, 3)
        XCTAssertEqual(clock.totalTicks, 6)
    }

    func testClockListener() {
        let clock = Clock()
        var receivedEdges: [ClockEdge] = []
        
        let token = clock.registerListener { edge in
            receivedEdges.append(edge)
        }
        
        clock.run(cycles: 1)
        XCTAssertEqual(receivedEdges.count, 2)
        XCTAssertEqual(receivedEdges[0].isRising, true)
        XCTAssertEqual(receivedEdges[1].isFalling, true)
        
        do {
            try clock.unregisterListener(token)
        } catch {
            XCTFail("Should not throw")
        }
        
        clock.run(cycles: 1)
        XCTAssertEqual(receivedEdges.count, 2) // Should not increase
    }

    func testUnregisterUnknownListener() {
        let clock = Clock()
        XCTAssertThrowsError(try clock.unregisterListener(UUID())) { error in
            XCTAssertEqual(error as? ClockError, ClockError.listenerNotFound)
        }
    }

    func testClockReset() {
        let clock = Clock()
        clock.run(cycles: 5)
        XCTAssertEqual(clock.cycle, 5)
        XCTAssertEqual(clock.totalTicks, 10)
        
        clock.reset()
        XCTAssertEqual(clock.cycle, 0)
        XCTAssertEqual(clock.totalTicks, 0)
        XCTAssertEqual(clock.value, 0)
    }
    
    func testClockDivider() throws {
        let master = Clock(frequencyHz: 1_000)
        let divider = try ClockDivider(source: master, divisor: 4)
        
        XCTAssertEqual(divider.output.frequencyHz, 250)
        
        master.run(cycles: 4)
        XCTAssertEqual(divider.output.cycle, 1) // 4 master cycles = 1 divided cycle
        XCTAssertEqual(divider.output.totalTicks, 2)
        
        master.run(cycles: 8)
        XCTAssertEqual(divider.output.cycle, 3)
    }

    func testClockDividerInvalidDivisor() {
        let master = Clock()
        XCTAssertThrowsError(try ClockDivider(source: master, divisor: 1)) { error in
            XCTAssertEqual(error as? ClockError, ClockError.invalidDivisor(1))
        }
    }

    func testMultiPhaseClock() throws {
        let master = Clock()
        let multi = try MultiPhaseClock(source: master, phases: 4)
        
        XCTAssertEqual(multi.phases, 4)
        XCTAssertEqual(multi.activePhase, 0)
        
        for p in 0..<4 {
            XCTAssertEqual(multi.getPhase(p), 0)
        }
        
        master.tick() // Rising edge 1
        XCTAssertEqual(multi.getPhase(0), 1)
        XCTAssertEqual(multi.getPhase(1), 0)
        master.tick() // Falling edge 1
        
        master.tick() // Rising edge 2
        XCTAssertEqual(multi.getPhase(0), 0)
        XCTAssertEqual(multi.getPhase(1), 1)
    }

    func testMultiPhaseClockInvalidPhases() {
        let master = Clock()
        XCTAssertThrowsError(try MultiPhaseClock(source: master, phases: 1)) { error in
            XCTAssertEqual(error as? ClockError, ClockError.invalidPhases(1))
        }
    }
}
