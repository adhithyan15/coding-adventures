// Http1.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// Http1 — HTTP/1 request and response head parser with body framing detection
// ============================================================================
//
// Usage:
//
//   import Http1
//
// ============================================================================

import HttpCore
import Foundation

public enum Http1Error: Error, Equatable {
    case incompleteHead
    case invalidStartLine(String)
    case invalidStatusLine(String)
    case invalidHeader(String)
    case invalidStatus(String)
    case invalidContentLength(String)
}

public struct ParsedRequestHead: Equatable {
    public let head: RequestHead
    public let bodyOffset: Int
    public let bodyKind: BodyKind

    public init(head: RequestHead, bodyOffset: Int, bodyKind: BodyKind) {
        self.head = head
        self.bodyOffset = bodyOffset
        self.bodyKind = bodyKind
    }
}

public struct ParsedResponseHead: Equatable {
    public let head: ResponseHead
    public let bodyOffset: Int
    public let bodyKind: BodyKind

    public init(head: ResponseHead, bodyOffset: Int, bodyKind: BodyKind) {
        self.head = head
        self.bodyOffset = bodyOffset
        self.bodyKind = bodyKind
    }
}

public enum Http1 {
    public static func parseRequestHead(_ input: Data) throws -> ParsedRequestHead {
        let (lines, bodyOffset) = try splitHeadLines(input)
        guard let startLine = lines.first else {
            throw Http1Error.invalidStartLine("")
        }

        let parts = startLine.split(whereSeparator: \.isWhitespace).map(String.init)
        guard parts.count == 3 else {
            throw Http1Error.invalidStartLine(startLine)
        }

        let version = try HttpVersion.parse(parts[2])
        let headers = try parseHeaders(Array(lines.dropFirst()))
        return ParsedRequestHead(
            head: RequestHead(method: parts[0], target: parts[1], version: version, headers: headers),
            bodyOffset: bodyOffset,
            bodyKind: try requestBodyKind(headers),
        )
    }

    public static func parseResponseHead(_ input: Data) throws -> ParsedResponseHead {
        let (lines, bodyOffset) = try splitHeadLines(input)
        guard let statusLine = lines.first else {
            throw Http1Error.invalidStatusLine("")
        }

        let parts = statusLine.split(whereSeparator: \.isWhitespace).map(String.init)
        guard parts.count >= 2 else {
            throw Http1Error.invalidStatusLine(statusLine)
        }

        let version = try HttpVersion.parse(parts[0])
        guard let status = Int(parts[1]) else {
            throw Http1Error.invalidStatus(parts[1])
        }

        let headers = try parseHeaders(Array(lines.dropFirst()))
        return ParsedResponseHead(
            head: ResponseHead(version: version, status: status, reason: parts.dropFirst(2).joined(separator: " "), headers: headers),
            bodyOffset: bodyOffset,
            bodyKind: try responseBodyKind(status: status, headers: headers),
        )
    }

    private static func splitHeadLines(_ input: Data) throws -> ([String], Int) {
        let bytes = Array(input)
        var index = 0

        while index < bytes.count {
            if index + 1 < bytes.count, bytes[index] == 13, bytes[index + 1] == 10 {
                index += 2
                continue
            }
            if bytes[index] == 10 {
                index += 1
                continue
            }
            break
        }

        var lines: [String] = []
        while true {
            guard index < bytes.count else {
                throw Http1Error.incompleteHead
            }

            let lineStart = index
            while index < bytes.count, bytes[index] != 10 {
                index += 1
            }
            guard index < bytes.count else {
                throw Http1Error.incompleteHead
            }

            let lineEnd = index > lineStart && bytes[index - 1] == 13 ? index - 1 : index
            let line = String(decoding: bytes[lineStart..<lineEnd], as: UTF8.self)
            index += 1

            if line.isEmpty {
                return (lines, index)
            }
            lines.append(line)
        }
    }

    private static func parseHeaders(_ lines: [String]) throws -> [Header] {
        try lines.map { line in
            guard let separator = line.firstIndex(of: ":"), separator > line.startIndex else {
                throw Http1Error.invalidHeader(line)
            }

            return Header(
                name: String(line[..<separator]).trimmingCharacters(in: .whitespacesAndNewlines),
                value: String(line[line.index(after: separator)...]).trimmingCharacters(in: .whitespacesAndNewlines),
            )
        }
    }

    private static func requestBodyKind(_ headers: [Header]) throws -> BodyKind {
        if hasChunkedTransferEncoding(headers) {
            return .chunked
        }

        let length = try declaredContentLength(headers)
        if let length, length > 0 {
            return .contentLength(length)
        }
        return .none
    }

    private static func responseBodyKind(status: Int, headers: [Header]) throws -> BodyKind {
        if (100..<200).contains(status) || status == 204 || status == 304 {
            return .none
        }
        if hasChunkedTransferEncoding(headers) {
            return .chunked
        }

        guard let length = try declaredContentLength(headers) else {
            return .untilEof
        }
        if length == 0 {
            return .none
        }
        return .contentLength(length)
    }

    private static func declaredContentLength(_ headers: [Header]) throws -> Int? {
        guard let value = findHeader(headers, name: "Content-Length") else {
            return nil
        }
        guard let length = Int(value), length >= 0 else {
            throw Http1Error.invalidContentLength(value)
        }
        return length
    }

    private static func hasChunkedTransferEncoding(_ headers: [Header]) -> Bool {
        headers
            .filter { $0.name.caseInsensitiveCompare("Transfer-Encoding") == .orderedSame }
            .contains { header in
                header.value
                    .split(separator: ",")
                    .map { $0.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() }
                    .contains("chunked")
            }
    }
}
