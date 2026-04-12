public struct CSVParser {
    
    public static func parseCSV(_ source: String) throws -> [CsvRow] {
        try parseCSVWithDelimiter(source, delimiter: ",")
    }
    
    public static func parseCSVWithDelimiter(_ source: String, delimiter: Character) throws -> [CsvRow] {
        let rawRows = try tokeniseRows(source: source, delimiter: delimiter)
        if rawRows.isEmpty { return [] }
        
        let header = rawRows[0]
        let dataRows = Array(rawRows.dropFirst())
        
        if dataRows.isEmpty { return [] }
        
        return dataRows.map { buildRowMap(header: header, data: $0) }
    }
    
    private static func tokeniseRows(source: String, delimiter: Character) throws -> [[String]] {
        var rows: [[String]] = []
        var currentRow: [String] = []
        var fieldBuf = ""
        var state = ParseState.fieldStart
        
        let characters = Array(source)
        let len = characters.count
        var i = 0
        
        while i < len {
            let ch = characters[i]
            
            switch state {
            case .fieldStart:
                if ch == "\"" {
                    state = .inQuotedField
                } else if ch == delimiter {
                    currentRow.append("")
                } else if isNewlineStart(ch) {
                    if !currentRow.isEmpty {
                        currentRow.append("")
                    }
                    i = consumeNewline(characters, i)
                    rows.append(currentRow)
                    currentRow = []
                } else {
                    fieldBuf.append(ch)
                    state = .inUnquotedField
                }
                
            case .inUnquotedField:
                if ch == delimiter {
                    currentRow.append(fieldBuf)
                    fieldBuf = ""
                    state = .fieldStart
                } else if isNewlineStart(ch) {
                    currentRow.append(fieldBuf)
                    fieldBuf = ""
                    i = consumeNewline(characters, i)
                    rows.append(currentRow)
                    currentRow = []
                    state = .fieldStart
                } else {
                    fieldBuf.append(ch)
                }
                
            case .inQuotedField:
                if ch == "\"" {
                    state = .inQuotedMaybeEnd
                } else {
                    fieldBuf.append(ch)
                }
                
            case .inQuotedMaybeEnd:
                if ch == "\"" {
                    fieldBuf.append("\"")
                    state = .inQuotedField
                } else if ch == delimiter {
                    currentRow.append(fieldBuf)
                    fieldBuf = ""
                    state = .fieldStart
                } else if isNewlineStart(ch) {
                    currentRow.append(fieldBuf)
                    fieldBuf = ""
                    i = consumeNewline(characters, i)
                    rows.append(currentRow)
                    currentRow = []
                    state = .fieldStart
                } else {
                    fieldBuf.append(ch)
                    state = .inUnquotedField
                }
            }
            
            i += 1
        }
        
        if state == .inQuotedField {
            throw CSVParserError.unclosedQuoteError
        }
        
        if state == .inUnquotedField {
            currentRow.append(fieldBuf)
        } else if state == .inQuotedMaybeEnd {
            currentRow.append(fieldBuf)
        }
        
        if !currentRow.isEmpty {
            rows.append(currentRow)
        }
        
        return rows
    }
    
    private static func buildRowMap(header: [String], data: [String]) -> CsvRow {
        var row: CsvRow = [:]
        for idx in 0..<header.count {
            let colName = header[idx]
            if idx < data.count {
                row[colName] = data[idx]
            } else {
                row[colName] = ""
            }
        }
        return row
    }
    
    private static func isNewlineStart(_ ch: Character) -> Bool {
        return ch == "\n" || ch == "\r" || ch == "\r\n"
    }
    
    private static func consumeNewline(_ source: [Character], _ i: Int) -> Int {
        if source[i] == "\r\n" {
            // Already clustered as a single character, no need to peek ahead
            return i
        }
        if source[i] == "\r" && i + 1 < source.count && source[i + 1] == "\n" {
            // Fallback just in case `Array(source)` isn't perfectly clustered or if we switched to unicodeScalars in the future
            return i + 1
        }
        return i
    }
}
