import Foundation

/// Single-bit storage element modeled at the gate level.
public class SRAMCell {
    private var _value: Bit = 0

    public init() {}

    /// Read the stored bit if the cell is selected.
    public func read(wordLine: Bit) -> Bit? {
        guard wordLine == 1 else { return nil }
        return _value
    }

    /// Write a bit to the cell if selected.
    public func write(wordLine: Bit, bitLine: Bit) {
        if wordLine == 1 {
            _value = bitLine
        }
    }

    public var value: Bit { return _value }
}

/// 2D grid of SRAM cells with row/column addressing.
public class SRAMArray {
    private let _rows: Int
    private let _cols: Int
    private let _cells: [[SRAMCell]]

    public init(rows: Int, cols: Int) throws {
        guard rows >= 1 else { throw BlockRAMError.invalidArgument("rows must be >= 1, got \(rows)") }
        guard cols >= 1 else { throw BlockRAMError.invalidArgument("cols must be >= 1, got \(cols)") }

        self._rows = rows
        self._cols = cols
        
        var cells = [[SRAMCell]]()
        for _ in 0..<rows {
            var row = [SRAMCell]()
            for _ in 0..<cols {
                row.append(SRAMCell())
            }
            cells.append(row)
        }
        self._cells = cells
    }

    public func read(row: Int) throws -> [Bit] {
        try _validateRow(row)
        return _cells[row].map { $0.read(wordLine: 1)! }
    }

    public func write(row: Int, data: [Bit]) throws {
        try _validateRow(row)
        guard data.count == _cols else {
            throw BlockRAMError.invalidArgument("data length \(data.count) does not match cols \(_cols)")
        }
        
        for col in 0..<data.count {
            _cells[row][col].write(wordLine: 1, bitLine: data[col])
        }
    }

    public var shape: (Int, Int) {
        return (_rows, _cols)
    }

    private func _validateRow(_ row: Int) throws {
        guard row >= 0 && row < _rows else {
            throw BlockRAMError.outOfRange("row \(row) out of range [0, \(_rows - 1)]")
        }
    }
}
