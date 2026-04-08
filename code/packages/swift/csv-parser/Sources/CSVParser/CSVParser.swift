import Foundation

public struct UnclosedQuoteError: Error, Equatable {
    public let message: String
    public init(_ message: String) {
        self.message = message
    }
}

fileprivate enum ParseState {
    case fieldStart
    case inUnquotedField
    case inQuotedField
    case inQuotedMaybeEnd
}

public func parseCSV(_ source: String, delimiter: Character = ",") throws -> [[String: String]] {
    let rawRows = try scan(source: source, delimiter: delimiter)
    
    if rawRows.isEmpty {
        return []
    }
    
    let header = rawRows[0]
    let dataRows = Array(rawRows.dropFirst())
    
    if dataRows.isEmpty {
        return []
    }
    
    var result: [[String: String]] = []
    for row in dataRows {
        result.append(zipRow(header: header, row: row))
    }
    
    return result
}

private func scan(source: String, delimiter: Character) throws -> [[String]] {
    let sentinel: Character = "\n"
    var chars = Array(source)
    chars.append(sentinel)
    
    var state = ParseState.fieldStart
    var currentField: [Character] = []
    var currentRow: [String] = []
    var allRows: [[String]] = []
    
    var i = 0
    let n = chars.count
    
    func finishRow(_ row: [String]) {
        if row.count == 1 && row[0] == "" { return }
        allRows.append(row)
    }
    
    while i < n {
        let ch = chars[i]
        
        switch state {
        case .fieldStart:
            if ch == "\"" {
                state = .inQuotedField
                i += 1
            } else if ch == delimiter {
                currentRow.append("")
                i += 1
            } else if ch == "\n" || ch == "\r" || ch == "\r\n" {
                currentRow.append("")
                finishRow(currentRow)
                currentRow = []
                state = .fieldStart
                i += 1
            } else {
                state = .inUnquotedField
                currentField.append(ch)
                i += 1
            }
            
        case .inUnquotedField:
            if ch == delimiter {
                currentRow.append(String(currentField))
                currentField = []
                state = .fieldStart
                i += 1
            } else if ch == "\n" || ch == "\r" || ch == "\r\n" {
                currentRow.append(String(currentField))
                currentField = []
                finishRow(currentRow)
                currentRow = []
                state = .fieldStart
                i += 1
            } else {
                currentField.append(ch)
                i += 1
            }
            
        case .inQuotedField:
            if ch == "\"" {
                state = .inQuotedMaybeEnd
                i += 1
            } else {
                currentField.append(ch)
                i += 1
            }
            
        case .inQuotedMaybeEnd:
            if ch == "\"" {
                currentField.append("\"")
                state = .inQuotedField
                i += 1
            } else if ch == delimiter {
                currentRow.append(String(currentField))
                currentField = []
                state = .fieldStart
                i += 1
            } else if ch == "\n" || ch == "\r" || ch == "\r\n" {
                currentRow.append(String(currentField))
                currentField = []
                finishRow(currentRow)
                currentRow = []
                state = .fieldStart
                i += 1
            } else {
                state = .inUnquotedField
                // do not advance i
            }
        }
    }
    
    if state == .inQuotedField {
        throw UnclosedQuoteError("Unclosed quoted field at end of input. A field was opened with '\"' but the matching closing '\"' was never found.")
    }
    
    if !currentRow.isEmpty || !currentField.isEmpty {
        if !currentField.isEmpty {
            currentRow.append(String(currentField))
        }
        if !currentRow.isEmpty {
            finishRow(currentRow)
        }
    }
    
    return allRows
}

private func zipRow(header: [String], row: [String]) -> [String: String] {
    var result: [String: String] = [:]
    for (colIndex, colName) in header.enumerated() {
        if colIndex < row.count {
            result[colName] = row[colIndex]
        } else {
            result[colName] = ""
        }
    }
    return result
}
