import XCTest
@testable import BlockRAM

final class BlockRAMTests: XCTestCase {
    
    func testSRAMCell() {
        let cell = SRAMCell()
        XCTAssertEqual(cell.value, 0)
        XCTAssertNil(cell.read(wordLine: 0))
        XCTAssertEqual(cell.read(wordLine: 1), 0)
        
        cell.write(wordLine: 1, bitLine: 1)
        XCTAssertEqual(cell.value, 1)
        
        cell.write(wordLine: 0, bitLine: 0)
        XCTAssertEqual(cell.value, 1) // Should remain 1 because wordLine is 0
    }
    
    func testSRAMArray() throws {
        let array = try SRAMArray(rows: 4, cols: 4)
        try array.write(row: 0, data: [1, 0, 1, 0])
        try array.write(row: 1, data: [0, 1, 0, 1])
        
        XCTAssertEqual(try array.read(row: 0), [1, 0, 1, 0])
        XCTAssertEqual(try array.read(row: 1), [0, 1, 0, 1])
        XCTAssertEqual(try array.read(row: 2), [0, 0, 0, 0])
    }
    
    func testSinglePortRAM() throws {
        let ram = try SinglePortRAM(depth: 16, width: 8, readMode: .readFirst)
        
        XCTAssertEqual(ram.depth, 16)
        XCTAssertEqual(ram.width, 8)
        
        // Write 11110000 to address 0
        _ = try ram.tick(clock: 0, address: 0, dataIn: [1,1,1,1,0,0,0,0], writeEnable: 1)
        let outA = try ram.tick(clock: 1, address: 0, dataIn: [1,1,1,1,0,0,0,0], writeEnable: 1)
        XCTAssertEqual(outA, [0,0,0,0,0,0,0,0]) // readFirst returns old value
        
        // Read address 0
        _ = try ram.tick(clock: 0, address: 0, dataIn: [0,0,0,0,0,0,0,0], writeEnable: 0)
        let outB = try ram.tick(clock: 1, address: 0, dataIn: [0,0,0,0,0,0,0,0], writeEnable: 0)
        XCTAssertEqual(outB, [1,1,1,1,0,0,0,0])
    }

    func testDualPortRAM() throws {
        let ram = try DualPortRAM(depth: 16, width: 8)
        
        // Write A to 0, Read B from 0
        _ = try ram.tick(clock: 0, addressA: 0, dataInA: [1,1,1,1,1,1,1,1], writeEnableA: 1, addressB: 0, dataInB: [0,0,0,0,0,0,0,0], writeEnableB: 0)
        let (outA, outB) = try ram.tick(clock: 1, addressA: 0, dataInA: [1,1,1,1,1,1,1,1], writeEnableA: 1, addressB: 0, dataInB: [0,0,0,0,0,0,0,0], writeEnableB: 0)
        XCTAssertEqual(outA, [0,0,0,0,0,0,0,0])
        XCTAssertEqual(outB, [0,0,0,0,0,0,0,0]) // readFirst on B as well
        
        // Next cycle read from B
        _ = try ram.tick(clock: 0, addressA: 0, dataInA: [0,0,0,0,0,0,0,0], writeEnableA: 0, addressB: 0, dataInB: [0,0,0,0,0,0,0,0], writeEnableB: 0)
        let (_, outB2) = try ram.tick(clock: 1, addressA: 0, dataInA: [0,0,0,0,0,0,0,0], writeEnableA: 0, addressB: 0, dataInB: [0,0,0,0,0,0,0,0], writeEnableB: 0)
        XCTAssertEqual(outB2, [1,1,1,1,1,1,1,1])
    }

    func testConfigurableBRAM() throws {
        let bram = try ConfigurableBRAM(totalBits: 64, width: 8)
        XCTAssertEqual(bram.depth, 8)
        
        try bram.reconfigure(width: 4)
        XCTAssertEqual(bram.depth, 16)
        
        _ = try bram.tickA(clock: 0, address: 2, dataIn: [1,0,1,0], writeEnable: 1)
        _ = try bram.tickA(clock: 1, address: 2, dataIn: [1,0,1,0], writeEnable: 1)
        
        _ = try bram.tickB(clock: 0, address: 2, dataIn: [0,0,0,0], writeEnable: 0)
        let outB = try bram.tickB(clock: 1, address: 2, dataIn: [0,0,0,0], writeEnable: 0)
        XCTAssertEqual(outB, [1,0,1,0])
    }
}
