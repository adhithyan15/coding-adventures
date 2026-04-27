// ============================================================================
// Reader.swift — MessageReader: Content-Length-framed JSON-RPC reader
// ============================================================================
//
// Reads one Content-Length-framed JSON-RPC message at a time from an
// InputStream (typically connected to stdin, but any InputStream works
// — including for testing).
//
// Wire format:
//
//   Content-Length: 97\r\n
//   \r\n
//   {"jsonrpc":"2.0","id":1,"method":"textDocument/hover",...}
//
// Why Content-Length framing?
// ---------------------------
// JSON has no self-delimiting structure at the byte stream level.
// You cannot tell where one JSON object ends without parsing the
// whole thing. Content-Length solves this: read headers, find the
// length, then read exactly that many bytes.
//
// Implementation notes
// --------------------
// We read the stream byte-by-byte for the header until we see the
// \r\n\r\n sentinel. Then we read exactly contentLength bytes for
// the payload. This is simple and correct for the LSP use case.
//
// ============================================================================

import Foundation

/// Reads Content-Length-framed JSON-RPC messages from an input stream.
///
/// Each call to `readMessage()` reads exactly one framed message, parses
/// the JSON, and returns a typed `JsonRpcMessage`.
///
/// Example:
///   let reader = MessageReader(Data("Content-Length: 38\r\n\r\n{...}".utf8))
///   let msg = try reader.readMessage()
public final class MessageReader: Sendable {
    /// The raw bytes to read from. We use Data + an index rather than
    /// InputStream because Data is simpler for testing and InputStream
    /// has platform-specific quirks.
    private let data: Data

    /// Current read position in the data buffer.
    /// Using a class wrapper for interior mutability with Sendable.
    private let state: ReadState

    /// Internal mutable state wrapper.
    private final class ReadState: @unchecked Sendable {
        var position: Int = 0

        init() {}
    }

    /// Create a reader from raw bytes.
    ///
    /// - Parameter data: The bytes containing one or more framed messages.
    public init(_ data: Data) {
        self.data = data
        self.state = ReadState()
    }

    /// Convenience initializer from a String.
    public convenience init(_ string: String) {
        self.init(Data(string.utf8))
    }

    // ----------------------------------------------------------------
    // readMessage — read, frame, parse, return typed Message
    // ----------------------------------------------------------------
    //
    // Reads the next Content-Length-framed message from the data and
    // returns a typed Request, Notification, or Response.
    //
    // Returns nil on clean EOF (no more data to read).
    // Throws JsonRpcError(-32700) on malformed JSON.
    // Throws JsonRpcError(-32600) on valid JSON that is not a message.
    //

    /// Read one framed message and return a typed message.
    ///
    /// - Returns: A `JsonRpcMessage`, or `nil` if no more messages remain.
    /// - Throws: `JsonRpcError` on malformed framing or invalid JSON.
    public func readMessage() throws -> JsonRpcMessage? {
        let raw = try readRaw()
        guard let raw else { return nil }

        // Parse the JSON string into a dictionary.
        guard let jsonData = raw.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: jsonData) as? [String: Any] else {
            throw JsonRpcError(
                code: JsonRpcErrorCodes.parseError,
                message: "Parse error: invalid JSON"
            )
        }

        return try parseMessage(obj)
    }

    // ----------------------------------------------------------------
    // readRaw — read one framed message as a raw JSON string
    // ----------------------------------------------------------------
    //
    // Returns the JSON string without parsing it. Useful for testing
    // or for proxy scenarios where the caller controls parsing.
    //
    // Returns nil on EOF.
    //

    /// Read one framed message and return the raw JSON payload string.
    ///
    /// - Returns: The JSON payload, or `nil` if no more data.
    /// - Throws: `JsonRpcError` on malformed framing.
    public func readRaw() throws -> String? {
        // Step 1: Read headers up to the \r\n\r\n blank line.
        let headerBytes = readUntilBlankLine()
        guard let headerBytes else { return nil }

        // Step 2: Extract Content-Length from the headers.
        let contentLength = try parseContentLength(headerBytes)

        // Step 3: Read exactly contentLength bytes as the JSON payload.
        guard state.position + contentLength <= data.count else {
            throw JsonRpcError(
                code: JsonRpcErrorCodes.parseError,
                message: "Parse error: stream ended before payload was complete"
            )
        }

        let payloadRange = state.position ..< (state.position + contentLength)
        let payload = data[payloadRange]
        state.position += contentLength

        guard let result = String(data: payload, encoding: .utf8) else {
            throw JsonRpcError(
                code: JsonRpcErrorCodes.parseError,
                message: "Parse error: payload is not valid UTF-8"
            )
        }

        return result
    }

    // ----------------------------------------------------------------
    // Private helpers
    // ----------------------------------------------------------------

    /// Read bytes one at a time until we see the sequence \r\n\r\n.
    ///
    /// Returns the header block as a string, or nil if the data is
    /// exhausted before any bytes are read (clean EOF).
    private func readUntilBlankLine() -> String? {
        let sentinel: [UInt8] = [0x0D, 0x0A, 0x0D, 0x0A] // \r\n\r\n
        var buffer: [UInt8] = []

        while state.position < data.count {
            let byte = data[state.position]
            state.position += 1
            buffer.append(byte)

            // Check if we have seen the blank-line sentinel at the end.
            if buffer.count >= 4 {
                let tail = Array(buffer[(buffer.count - 4)...])
                if tail == sentinel {
                    // Return everything before the sentinel.
                    let headerData = Data(buffer[0 ..< (buffer.count - 4)])
                    return String(data: headerData, encoding: .utf8)
                }
            }
        }

        // EOF before seeing sentinel.
        if buffer.isEmpty { return nil }
        return nil // partial header — treat as EOF
    }

    /// Parse the Content-Length value from the header block string.
    ///
    /// The header block looks like:
    ///   Content-Length: 97\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8
    ///
    /// We split on \r\n and search for the Content-Length line
    /// (case-insensitive, following the HTTP convention).
    private func parseContentLength(_ headerStr: String) throws -> Int {
        let lines = headerStr.split(separator: "\r\n", omittingEmptySubsequences: false)
            .map { String($0) }

        for line in lines {
            if line.lowercased().hasPrefix("content-length:") {
                let parts = line.split(separator: ":", maxSplits: 1)
                guard parts.count == 2 else { continue }
                let valueStr = parts[1].trimmingCharacters(in: .whitespaces)
                guard let n = Int(valueStr), n >= 0 else {
                    throw JsonRpcError(
                        code: JsonRpcErrorCodes.parseError,
                        message: "Parse error: invalid Content-Length value"
                    )
                }
                return n
            }
        }

        throw JsonRpcError(
            code: JsonRpcErrorCodes.parseError,
            message: "Parse error: missing Content-Length header"
        )
    }
}
