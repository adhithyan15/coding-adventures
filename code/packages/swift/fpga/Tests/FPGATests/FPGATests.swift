import XCTest
@testable import FPGA

final class FPGATests: XCTestCase {
    
    func testLUT() throws {
        let andTable = Array(repeating: 0, count: 16)
        var tt = andTable
        tt[3] = 1 // I0=1, I1=1 -> 3
        
        let lut = try LUT(k: 4, truthTable: tt)
        XCTAssertEqual(try lut.evaluate(inputs: [0, 0, 0, 0]), 0)
        XCTAssertEqual(try lut.evaluate(inputs: [1, 1, 0, 0]), 1)
        XCTAssertEqual(try lut.evaluate(inputs: [0, 1, 0, 0]), 0)
    }

    func testSliceCombinational() throws {
        let slice = try Slice(lutInputs: 4)
        
        var andTt = Array(repeating: 0, count: 16)
        andTt[3] = 1
        var xorTt = Array(repeating: 0, count: 16)
        xorTt[1] = 1
        xorTt[2] = 1
        
        try slice.configure(lutATable: andTt, lutBTable: xorTt)
        
        let out = try slice.evaluate(inputsA: [1, 1, 0, 0], inputsB: [1, 0, 0, 0], clock: 0)
        XCTAssertEqual(out.outputA, 1) // AND
        XCTAssertEqual(out.outputB, 1) // XOR
    }

    func testSliceCarry() throws {
        let slice = try Slice(lutInputs: 4)
        var andTt = Array(repeating: 0, count: 16)
        andTt[3] = 1
        var xorTt = Array(repeating: 0, count: 16)
        xorTt[1] = 1
        xorTt[2] = 1
        
        try slice.configure(lutATable: andTt, lutBTable: xorTt, carryEnabled: true)
        
        // A AND B (generate)
        var out = try slice.evaluate(inputsA: [1, 1, 0, 0], inputsB: [1, 1, 0, 0], clock: 0)
        XCTAssertEqual(out.carryOut, 1)
        
        // A XOR B with carryIn=1 (propagate)
        out = try slice.evaluate(inputsA: [1, 1, 0, 0], inputsB: [1, 0, 0, 0], clock: 0, carryIn: 1)
        XCTAssertEqual(out.carryOut, 1)
    }

    func testSwitchMatrix() throws {
        let sm = try SwitchMatrix(ports: ["north", "south", "east", "clbOut"])
        try sm.connect(source: "clbOut", destination: "east")
        try sm.connect(source: "north", destination: "south")
        
        let routed = sm.route(inputs: ["clbOut": 1, "north": 0])
        XCTAssertEqual(routed["east"], 1)
        XCTAssertEqual(routed["south"], 0)
        XCTAssertNil(routed["north"])
    }

    func testIOBlock() throws {
        let io = try IOBlock(name: "pinA", mode: .input)
        try io.drivePad(value: 1)
        XCTAssertEqual(io.readInternal(), 1)
        
        io.configure(mode: .output)
        try io.driveInternal(value: 0)
        XCTAssertEqual(try io.readPad(), 0)
        
        io.configure(mode: .tristate)
        try io.driveInternal(value: 1)
        XCTAssertNil(try io.readPad())
    }

    func testFabric() throws {
        let bstream = Bitstream(
            io: ["testOut": IOConfig(mode: "output")]
        )
        
        let fpga = try FPGA(bitstream: bstream)
        try fpga.driveOutput(pinName: "testOut", value: 1)
        XCTAssertEqual(try fpga.readOutput(pinName: "testOut"), 1)
    }
}
