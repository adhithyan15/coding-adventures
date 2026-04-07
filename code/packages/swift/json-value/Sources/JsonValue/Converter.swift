import Foundation
import Lexer
import Parser
import JsonParser

/// Unescape a parsed JSON string.
private func unescapeJsonString(_ s: String) -> String {
    var result = ""
    var i = s.startIndex
    while i < s.endIndex {
        if s[i] == "\\" {
            let nextIndex = s.index(after: i)
            if nextIndex < s.endIndex {
                let nextCh = s[nextIndex]
                switch nextCh {
                case "\"", "\\", "/":
                    result.append(nextCh)
                    i = s.index(after: nextIndex)
                case "b":
                    result.append("\u{0008}")
                    i = s.index(after: nextIndex)
                case "f":
                    result.append("\u{000C}")
                    i = s.index(after: nextIndex)
                case "n":
                    result.append("\n")
                    i = s.index(after: nextIndex)
                case "r":
                    result.append("\r")
                    i = s.index(after: nextIndex)
                case "t":
                    result.append("\t")
                    i = s.index(after: nextIndex)
                case "u":
                    let endOfHex = s.index(nextIndex, offsetBy: 4, limitedBy: s.endIndex)
                    if let endOfHex = endOfHex, endOfHex < s.endIndex || endOfHex == s.endIndex {
                        let hexStart = s.index(after: nextIndex)
                        let hexString = String(s[hexStart..<endOfHex])
                        if let scalarValue = UInt32(hexString, radix: 16),
                           let scalar = UnicodeScalar(scalarValue) {
                            result.append(Character(scalar))
                            i = endOfHex
                        } else {
                            result.append(s[i])
                            i = nextIndex
                        }
                    } else {
                        result.append(s[i])
                        i = nextIndex
                    }
                default:
                    result.append(s[i])
                    i = nextIndex
                }
            } else {
                result.append(s[i])
                i = s.index(after: i)
            }
        } else {
            result.append(s[i])
            i = s.index(after: i)
        }
    }
    return result
}

private let valueTokenTypes: Set<String> = ["STRING", "NUMBER", "TRUE", "FALSE", "NULL"]

private func convertToken(_ token: Token) throws -> JsonValue {
    switch token.type {
    case "STRING":
        return .string(unescapeJsonString(token.value))
    case "NUMBER":
        if let d = Double(token.value) {
            return .number(d)
        }
        throw JsonValueError.message("Invalid number format: \(token.value)")
    case "TRUE":
        return .bool(true)
    case "FALSE":
        return .bool(false)
    case "NULL":
        return .null
    default:
        throw JsonValueError.message("Unexpected token type: \(token.type)")
    }
}

private func extractPair(_ pairNode: ASTNode) throws -> (String, JsonValue) {
    var key: String? = nil
    var value: JsonValue? = nil
    for child in pairNode.children {
        if case .token(let t) = child, t.type == "STRING" {
            key = t.value
        } else if case .node(let n) = child, n.ruleName == "value" {
            value = try fromAst(ASTChild.node(n))
        }
    }
    guard let k = key else { throw JsonValueError.message("Pair node has no STRING key") }
    guard let v = value else { throw JsonValueError.message("Pair node has no value") }
    return (k, v)
}

private func convertObjectNode(_ node: ASTNode) throws -> JsonValue {
    var pairs: [String: JsonValue] = [:]
    for child in node.children {
        if case .node(let n) = child, n.ruleName == "pair" {
            let (k, v) = try extractPair(n)
            pairs[k] = v
        }
    }
    return .object(pairs)
}

private func convertArrayNode(_ node: ASTNode) throws -> JsonValue {
    var elements: [JsonValue] = []
    for child in node.children {
        if case .node(let n) = child, n.ruleName == "value" {
            elements.append(try fromAst(ASTChild.node(n)))
        } else if case .token(let t) = child, valueTokenTypes.contains(t.type) {
            elements.append(try convertToken(t))
        }
    }
    return .array(elements)
}

private func convertValueNode(_ node: ASTNode) throws -> JsonValue {
    for child in node.children {
        if case .node(let n) = child {
            if n.ruleName == "object" || n.ruleName == "array" {
                return try convertAstNode(n)
            }
        } else if case .token(let t) = child {
            if valueTokenTypes.contains(t.type) {
                return try convertToken(t)
            }
        }
    }
    throw JsonValueError.message("value node has no meaningful child")
}

private func convertAstNode(_ node: ASTNode) throws -> JsonValue {
    switch node.ruleName {
    case "value": return try convertValueNode(node)
    case "object": return try convertObjectNode(node)
    case "array": return try convertArrayNode(node)
    case "pair":
        let (_, v) = try extractPair(node)
        return v
    default:
        throw JsonValueError.message("Unknown AST rule: \(node.ruleName)")
    }
}

/// Convert a json-parser AST node or Token into a typed JsonValue.
public func fromAst(_ child: ASTChild) throws -> JsonValue {
    switch child {
    case .token(let t):
        return try convertToken(t)
    case .node(let n):
        return try convertAstNode(n)
    }
}

/// Parse JSON text into a JsonValue.
public func parse(_ text: String) throws -> JsonValue {
    do {
        let ast = try parseJson(text)
        return try fromAst(ASTChild.node(ast))
    } catch {
        throw JsonValueError.message("Failed to parse JSON: \(error)")
    }
}

/// Convert a JsonValue tree into native Swift types using `Any`.
public func toNative(_ value: JsonValue) -> Any {
    switch value {
    case .null: return NSNull()
    case .bool(let b): return b
    case .number(let n): return n
    case .string(let s): return s
    case .array(let a): return a.map(toNative)
    case .object(let o): return o.mapValues(toNative)
    }
}
