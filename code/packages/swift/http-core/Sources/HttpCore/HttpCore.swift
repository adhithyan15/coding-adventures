import Foundation

public let HTTPCoreVersion = "0.1.0"

public struct Header: Equatable {
    public let name: String
    public let value: String

    public init(name: String, value: String) {
        self.name = name
        self.value = value
    }
}

public enum HttpCoreError: Error, Equatable {
    case invalidVersion(String)
}

public struct HttpVersion: Equatable, CustomStringConvertible {
    public let major: Int
    public let minor: Int

    public init(major: Int, minor: Int) {
        self.major = major
        self.minor = minor
    }

    public static func parse(_ text: String) throws -> HttpVersion {
        guard text.hasPrefix("HTTP/") else {
            throw HttpCoreError.invalidVersion(text)
        }

        let parts = text.dropFirst(5).split(separator: ".", maxSplits: 1).map(String.init)
        guard parts.count == 2, let major = Int(parts[0]), let minor = Int(parts[1]) else {
            throw HttpCoreError.invalidVersion(text)
        }

        return HttpVersion(major: major, minor: minor)
    }

    public var description: String {
        "HTTP/\(major).\(minor)"
    }
}

public enum BodyKind: Equatable {
    case none
    case contentLength(Int)
    case untilEof
    case chunked
}

public struct RequestHead: Equatable {
    public let method: String
    public let target: String
    public let version: HttpVersion
    public let headers: [Header]

    public init(method: String, target: String, version: HttpVersion, headers: [Header]) {
        self.method = method
        self.target = target
        self.version = version
        self.headers = headers
    }

    public func header(_ name: String) -> String? {
        findHeader(headers, name: name)
    }

    public func contentLength() -> Int? {
        parseContentLength(headers)
    }

    public func contentType() -> (String, String?)? {
        parseContentType(headers)
    }
}

public struct ResponseHead: Equatable {
    public let version: HttpVersion
    public let status: Int
    public let reason: String
    public let headers: [Header]

    public init(version: HttpVersion, status: Int, reason: String, headers: [Header]) {
        self.version = version
        self.status = status
        self.reason = reason
        self.headers = headers
    }

    public func header(_ name: String) -> String? {
        findHeader(headers, name: name)
    }

    public func contentLength() -> Int? {
        parseContentLength(headers)
    }

    public func contentType() -> (String, String?)? {
        parseContentType(headers)
    }
}

public func findHeader(_ headers: [Header], name: String) -> String? {
    headers.first { $0.name.caseInsensitiveCompare(name) == .orderedSame }?.value
}

public func parseContentLength(_ headers: [Header]) -> Int? {
    guard let value = findHeader(headers, name: "Content-Length") else {
        return nil
    }
    return Int(value)
}

public func parseContentType(_ headers: [Header]) -> (String, String?)? {
    guard let value = findHeader(headers, name: "Content-Type") else {
        return nil
    }

    let parts = value.split(separator: ";").map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
    guard let mediaType = parts.first, !mediaType.isEmpty else {
        return nil
    }

    var charset: String?
    for piece in parts.dropFirst() {
        let pair = piece.split(separator: "=", maxSplits: 1).map(String.init)
        if pair.count == 2 && pair[0].trimmingCharacters(in: .whitespacesAndNewlines).caseInsensitiveCompare("charset") == .orderedSame {
            charset = pair[1].trimmingCharacters(in: .whitespacesAndNewlines.union(CharacterSet(charactersIn: "\"")))
            break
        }
    }

    return (String(mediaType), charset)
}
