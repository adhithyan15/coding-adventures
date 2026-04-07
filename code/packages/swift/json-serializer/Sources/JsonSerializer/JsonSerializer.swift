import Foundation
import JsonValue

public enum JsonSerializerError: Error {
    case invalidNumber(Double)
    case unknownType(String)
}

private let escapeTable: [Character: String] = [
    "\"": "\\\"",
    "\\": "\\\\",
    "\u{0008}": "\\b",
    "\u{000C}": "\\f",
    "\n": "\\n",
    "\r": "\\r",
    "\t": "\\t"
]

private func escapeJsonString(_ s: String) -> String {
    var parts: [String] = []
    for char in s {
        if let escaped = escapeTable[char] {
            parts.append(escaped)
        } else if let scalar = char.unicodeScalars.first, scalar.value < 0x20 {
            parts.append(String(format: "\\u%04x", scalar.value))
        } else {
            parts.append(String(char))
        }
    }
    return parts.joined()
}

private func formatNumber(_ n: Double) throws -> String {
    if n.isNaN || n.isInfinite {
        throw JsonSerializerError.invalidNumber(n)
    }
    // Simplistic formatting to try and omit .0 if it's effectively an integer
    let asInt = Int(exactly: n)
    if let asInt = asInt {
        return String(asInt)
    }
    return String(n)
}

public func serialize(_ value: JsonValue) throws -> String {
    switch value {
    case .null:
        return "null"
    case .bool(let b):
        return b ? "true" : "false"
    case .number(let n):
        return try formatNumber(n)
    case .string(let s):
        return "\"" + escapeJsonString(s) + "\""
    case .array(let elements):
        if elements.isEmpty { return "[]" }
        let parts = try elements.map { try serialize($0) }
        return "[" + parts.joined(separator: ",") + "]"
    case .object(let pairs):
        if pairs.isEmpty { return "{}" }
        let parts = try pairs.map { key, val in
            "\"" + escapeJsonString(key) + "\":" + (try serialize(val))
        }
        return "{" + parts.joined(separator: ",") + "}"
    }
}

private func serializePrettyRecursive(_ value: JsonValue, _ config: SerializerConfig, depth: Int) throws -> String {
    switch value {
    case .null, .bool, .number, .string:
        return try serialize(value)
    case .array(let elements):
        if elements.isEmpty { return "[]" }
        let indentUnit = String(repeating: config.indentChar, count: config.indentSize)
        let currentIndent = String(repeating: indentUnit, count: depth)
        let nextIndent = String(repeating: indentUnit, count: depth + 1)
        
        let lines = try elements.map { elem in
            nextIndent + (try serializePrettyRecursive(elem, config, depth: depth + 1))
        }
        return "[\n" + lines.joined(separator: ",\n") + "\n" + currentIndent + "]"
        
    case .object(let pairs):
        if pairs.isEmpty { return "{}" }
        let indentUnit = String(repeating: config.indentChar, count: config.indentSize)
        let currentIndent = String(repeating: indentUnit, count: depth)
        let nextIndent = String(repeating: indentUnit, count: depth + 1)
        
        let sortedKeys = config.sortKeys ? pairs.keys.sorted() : Array(pairs.keys)
        
        var lines: [String] = []
        for key in sortedKeys {
            guard let val = pairs[key] else { continue }
            let valStr = try serializePrettyRecursive(val, config, depth: depth + 1)
            lines.append(nextIndent + "\"" + escapeJsonString(key) + "\": " + valStr)
        }
        return "{\n" + lines.joined(separator: ",\n") + "\n" + currentIndent + "}"
    }
}

public func serializePretty(_ value: JsonValue, config: SerializerConfig? = nil) throws -> String {
    let cfg = config ?? SerializerConfig()
    var result = try serializePrettyRecursive(value, cfg, depth: 0)
    if cfg.trailingNewline {
        result += "\n"
    }
    return result
}

public func stringify(_ value: Any) throws -> String {
    // Converts native to JsonValue then to json compact string
    let jsonVal = try fromNative(value)
    return try serialize(jsonVal)
}

public func stringifyPretty(_ value: Any, config: SerializerConfig? = nil) throws -> String {
    let jsonVal = try fromNative(value)
    return try serializePretty(jsonVal, config: config)
}

private func fromNative(_ value: Any) throws -> JsonValue {
    if value is NSNull { return .null }
    if let b = value as? Bool { return .bool(b) }
    if let n = value as? Double { return .number(n) }
    if let n = value as? Int { return .number(Double(n)) }
    if let s = value as? String { return .string(s) }
    if let a = value as? [Any] {
        let elements = try a.map { try fromNative($0) }
        return .array(elements)
    }
    if let o = value as? [String: Any] {
        var pairs: [String: JsonValue] = [:]
        for (k, v) in o {
            pairs[k] = try fromNative(v)
        }
        return .object(pairs)
    }
    throw JsonSerializerError.unknownType("Cannot convert \(type(of: value)) to JsonValue")
}
