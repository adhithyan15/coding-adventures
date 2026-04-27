// ============================================================================
// Writer.swift — MessageWriter: typed Message → framed byte stream
// ============================================================================
//
// Writes one Content-Length-framed JSON-RPC message at a time to a
// mutable Data buffer (or any output mechanism).
//
// Framing format:
//
//   Content-Length: <n>\r\n
//   \r\n
//   <UTF-8 JSON payload, exactly n bytes>
//
// The Content-Length value is the BYTE length of the UTF-8-encoded
// JSON payload, NOT the character count. For ASCII-only JSON these
// are identical, but multi-byte Unicode characters (e.g. "€" is 3
// bytes in UTF-8) make them differ.
//
// Example:
//   let output = DataOutput()
//   let writer = MessageWriter(output)
//   try writer.writeMessage(.response(Response(id: AnySendable(1), result: AnySendable(["ok": true]))))
//
// ============================================================================

import Foundation

// ============================================================================
// OutputTarget protocol
// ============================================================================
//
// An abstraction over "something you can write bytes to". In production this
// wraps FileHandle.standardOutput. In tests it wraps a Data buffer.
//

/// Protocol for any target that can receive bytes.
///
/// This abstraction allows the writer to work with both FileHandle
/// (production) and Data buffers (testing).
public protocol OutputTarget: AnyObject, Sendable {
    /// Write raw bytes to the output.
    func write(_ data: Data)
}

/// A simple in-memory output target for testing.
///
/// Collects all written bytes in a Data buffer that can be inspected
/// after the test completes.
public final class DataOutput: OutputTarget, @unchecked Sendable {
    /// All bytes written so far.
    public private(set) var data = Data()

    public init() {}

    public func write(_ newData: Data) {
        data.append(newData)
    }

    /// The accumulated output as a UTF-8 string.
    public var string: String {
        String(data: data, encoding: .utf8) ?? ""
    }
}

/// An output target that wraps a FileHandle (e.g. stdout).
public final class FileHandleOutput: OutputTarget, @unchecked Sendable {
    private let handle: FileHandle

    public init(_ handle: FileHandle) {
        self.handle = handle
    }

    public func write(_ data: Data) {
        handle.write(data)
    }
}

// ============================================================================
// MessageWriter
// ============================================================================

/// Writes Content-Length-framed JSON-RPC messages to an output target.
///
/// Each call to `writeMessage` produces exactly one framed message
/// on the underlying output.
///
/// Example:
///   let output = DataOutput()
///   let writer = MessageWriter(output)
///   try writer.writeMessage(.response(Response(id: AnySendable(1), result: AnySendable(true))))
public final class MessageWriter: Sendable {
    private let output: OutputTarget

    /// Create a writer that sends framed messages to the given output.
    public init(_ output: OutputTarget) {
        self.output = output
    }

    // ----------------------------------------------------------------
    // writeMessage — serialize and frame a typed message
    // ----------------------------------------------------------------
    //
    // Converts the message to its wire-format dictionary, serializes it
    // with JSONSerialization (compact, no extra whitespace), then writes
    // the Content-Length header followed by the payload.
    //

    /// Serialize a typed message and write it as a Content-Length-framed message.
    ///
    /// - Parameter msg: The message to write.
    /// - Throws: If JSON serialization fails.
    public func writeMessage(_ msg: JsonRpcMessage) throws {
        let dict = messageToMap(msg)
        let jsonData = try JSONSerialization.data(withJSONObject: dict, options: [.sortedKeys])
        guard let jsonStr = String(data: jsonData, encoding: .utf8) else {
            throw JsonRpcError(code: JsonRpcErrorCodes.internalError, message: "Failed to encode JSON as UTF-8")
        }
        writeRaw(jsonStr)
    }

    // ----------------------------------------------------------------
    // writeRaw — frame and write a pre-serialized JSON string
    // ----------------------------------------------------------------
    //
    // Use when you already have the JSON string and do not need message
    // parsing — for example, in tests or proxy scenarios.
    //

    /// Write a raw JSON string as a Content-Length-framed message.
    ///
    /// - Parameter json: The JSON payload string.
    public func writeRaw(_ json: String) {
        // Force UTF-8 encoding for byte-accurate length calculation.
        let payload = Data(json.utf8)
        let header = "Content-Length: \(payload.count)\r\n\r\n"
        let headerData = Data(header.utf8)

        // Write header + payload together.
        output.write(headerData + payload)
    }
}
