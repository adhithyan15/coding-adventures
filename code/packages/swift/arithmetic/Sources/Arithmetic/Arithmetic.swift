// ============================================================================
// Arithmetic.swift
// ============================================================================

import LogicGates

public enum ArithmeticError: Error {
    case invalidInput
}

public func halfAdder(a: Int, b: Int) throws -> (sum: Int, carry: Int) {
    let sum = try xorGate(a, b)
    let carry = try andGate(a, b)
    return (sum, carry)
}

public func fullAdder(a: Int, b: Int, carryIn: Int) throws -> (sum: Int, carryOut: Int) {
    let ha1 = try halfAdder(a: a, b: b)
    let ha2 = try halfAdder(a: ha1.sum, b: carryIn)
    let carryOut = try orGate(ha1.carry, ha2.carry)
    return (ha2.sum, carryOut)
}

public func rippleCarryAdder(a: [Int], b: [Int], carryIn: Int = 0) throws -> (sum: [Int], carryOut: Int) {
    if a.count != b.count {
        throw ArithmeticError.invalidInput
    }
    var sum = [Int]()
    sum.reserveCapacity(a.count)
    var currentCarry = carryIn

    for i in 0..<a.count {
        let fa = try fullAdder(a: a[i], b: b[i], carryIn: currentCarry)
        sum.append(fa.sum)
        currentCarry = fa.carryOut
    }
    return (sum, currentCarry)
}

public enum ALUOp: String {
    case add = "add"
    case sub = "sub"
    case and = "and"
    case or  = "or"
    case xor = "xor"
    case not = "not"
}

public struct ALUResult {
    public let value: [Int]
    public let zero: Bool
    public let carry: Bool
    public let negative: Bool
    public let overflow: Bool
}

public class ALU {
    public let bitWidth: Int

    public init(bitWidth: Int = 8) {
        self.bitWidth = bitWidth
    }

    public func execute(op: ALUOp, a: [Int], b: [Int]) throws -> ALUResult {
        if a.count != bitWidth || b.count != bitWidth {
            throw ArithmeticError.invalidInput
        }

        var resultLine = [Int](repeating: 0, count: bitWidth)
        var aluCarryOut = 0

        switch op {
        case .add:
            let rc = try rippleCarryAdder(a: a, b: b, carryIn: 0)
            resultLine = rc.sum
            aluCarryOut = rc.carryOut
        case .sub:
            // Two's complement: A - B = A + NOT(B) + 1
            var notB = [Int](repeating: 0, count: bitWidth)
            for i in 0..<bitWidth {
                notB[i] = try notGate(b[i])
            }
            let rc = try rippleCarryAdder(a: a, b: notB, carryIn: 1)
            resultLine = rc.sum
            aluCarryOut = rc.carryOut
        case .and:
            for i in 0..<bitWidth {
                resultLine[i] = try andGate(a[i], b[i])
            }
        case .or:
            for i in 0..<bitWidth {
                resultLine[i] = try orGate(a[i], b[i])
            }
        case .xor:
            for i in 0..<bitWidth {
                resultLine[i] = try xorGate(a[i], b[i])
            }
        case .not:
            for i in 0..<bitWidth {
                resultLine[i] = try notGate(a[i])
            }
        }

        let isZero = !resultLine.contains(1)
        let isNegative = resultLine[bitWidth - 1] == 1
        
        // Overflow in 2s complement add/sub occurs if:
        // Carry into MSB != Carry out of MSB
        // Since we don't have the intermediate carry easily exposed,
        // we can derive signed overflow:
        // For Add: if both inputs have same sign, and result has different sign.
        // For Sub: since it's A + NOT(B) + 1, it's checking signs of A and NOT(B).
        var hasOverflow = false
        if op == .add {
            let signA = a[bitWidth - 1]
            let signB = b[bitWidth - 1]
            let signR = resultLine[bitWidth - 1]
            hasOverflow = (signA == signB) && (signA != signR)
        } else if op == .sub {
            let signA = a[bitWidth - 1]
            let signB = b[bitWidth - 1] // original B sign
            let signR = resultLine[bitWidth - 1]
            // We subtract B from A, so signB is inverted.
            hasOverflow = (signA != signB) && (signA != signR)
        }

        return ALUResult(
            value: resultLine,
            zero: isZero,
            carry: aluCarryOut == 1 && (op == .add || op == .sub),
            negative: isNegative,
            overflow: hasOverflow
        )
    }
}
