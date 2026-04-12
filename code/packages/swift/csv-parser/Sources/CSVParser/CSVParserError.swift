public enum CSVParserError: Error, CustomStringConvertible, Equatable {
    case unclosedQuoteError
    
    public var description: String {
        switch self {
        case .unclosedQuoteError:
            return "Unclosed quoted field: EOF reached inside a quoted field"
        }
    }
}
