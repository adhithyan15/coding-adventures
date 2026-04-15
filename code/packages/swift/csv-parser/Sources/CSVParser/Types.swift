public enum ParseState {
    case fieldStart
    case inUnquotedField
    case inQuotedField
    case inQuotedMaybeEnd
}

public typealias CsvRow = [String: String]
