import Foundation
import JsonValue

public struct SerializerConfig {
    public var indentSize: Int
    public var indentChar: String
    public var sortKeys: Bool
    public var trailingNewline: Bool

    public init(
        indentSize: Int = 2,
        indentChar: String = " ",
        sortKeys: Bool = false,
        trailingNewline: Bool = false
    ) {
        self.indentSize = indentSize
        self.indentChar = indentChar
        self.sortKeys = sortKeys
        self.trailingNewline = trailingNewline
    }
}
