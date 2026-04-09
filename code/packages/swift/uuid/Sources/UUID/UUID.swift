import Foundation
import MD5
import SHA1

public struct UUIDError: Error, CustomStringConvertible, Equatable {
    public let message: String
    public var description: String { message }
    public init(_ message: String) { self.message = message }
}

public struct UUID: Equatable, Comparable, Hashable, Sendable {
    private let _bytes: [UInt8]
    
    public init(bytes: [UInt8]) throws {
        guard bytes.count == 16 else {
            throw UUIDError("UUID bytes must be exactly 16, got \(bytes.count)")
        }
        self._bytes = bytes
    }
    
    public init(_ string: String) throws {
        let pattern = "^\\s*(?:urn:uuid:)?\\{?([0-9a-fA-F]{8})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{12})\\}?\\s*$"
        guard let regex = try? NSRegularExpression(pattern: pattern, options: .caseInsensitive),
              let match = regex.firstMatch(in: string, range: NSRange(string.startIndex..., in: string)) else {
            throw UUIDError("Invalid UUID string: '\(string)'")
        }
        
        var hex32 = ""
        for i in 1...5 {
            if let range = Range(match.range(at: i), in: string) {
                hex32 += String(string[range])
            }
        }
        
        var bytes = [UInt8]()
        var index = hex32.startIndex
        for _ in 0..<16 {
            let nextIndex = hex32.index(index, offsetBy: 2)
            if let byte = UInt8(hex32[index..<nextIndex], radix: 16) {
                bytes.append(byte)
            } else {
                throw UUIDError("Invalid hex in UUID: '\(string)'")
            }
            index = nextIndex
        }
        self._bytes = bytes
    }
    
    public var bytes: [UInt8] { _bytes }
    
    public var version: Int {
        Int((_bytes[6] >> 4) & 0xF)
    }
    
    public var variant: String {
        let top = (_bytes[8] >> 6) & 0x3
        if top == 0b00 || top == 0b01 {
            return "ncs"
        } else if top == 0b10 {
            return "rfc4122"
        } else if top == 0b11 {
            if ((_bytes[8] >> 5) & 0x1) == 0 {
                return "microsoft"
            } else {
                return "reserved"
            }
        }
        return "unknown"
    }
    
    public var isNil: Bool {
        _bytes == [UInt8](repeating: 0, count: 16)
    }
    
    public var isMax: Bool {
        _bytes == [UInt8](repeating: 0xFF, count: 16)
    }
    
    public static func == (lhs: UUID, rhs: UUID) -> Bool {
        lhs._bytes == rhs._bytes
    }
    
    public static func < (lhs: UUID, rhs: UUID) -> Bool {
        for i in 0..<16 {
            if lhs._bytes[i] != rhs._bytes[i] {
                return lhs._bytes[i] < rhs._bytes[i]
            }
        }
        return false
    }
    
    public func hash(into hasher: inout Hasher) {
        hasher.combine(_bytes)
    }
    
    // Namespaces
    public static let namespaceDNS  = try! UUID("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
    public static let namespaceURL  = try! UUID("6ba7b811-9dad-11d1-80b4-00c04fd430c8")
    public static let namespaceOID  = try! UUID("6ba7b812-9dad-11d1-80b4-00c04fd430c8")
    public static let namespaceX500 = try! UUID("6ba7b814-9dad-11d1-80b4-00c04fd430c8")
    
    public static let nilUUID = try! UUID("00000000-0000-0000-0000-000000000000")
    public static let maxUUID = try! UUID("ffffffff-ffff-ffff-ffff-ffffffffffff")
    
    public static func parse(_ text: String) throws -> UUID {
        try UUID(text)
    }
    
    public static func isValid(_ text: String) -> Bool {
        (try? UUID(text)) != nil
    }
    
    private static func setVersionVariant(_ raw: inout [UInt8], version: UInt8) {
        raw[6] = (raw[6] & 0x0F) | (version << 4)
        raw[8] = (raw[8] & 0x3F) | 0x80
    }
    
    public static func v4() -> UUID {
        var raw = [UInt8](repeating: 0, count: 16)
        for i in 0..<16 { raw[i] = UInt8.random(in: 0...255) }
        setVersionVariant(&raw, version: 4)
        return try! UUID(bytes: raw)
    }
    
    public static func v5(namespace: UUID, name: String) -> UUID {
        var data = Data(namespace.bytes)
        data.append(Data(name.utf8))
        let digest = [UInt8](sha1(data))
        var raw = Array(digest.prefix(16))
        setVersionVariant(&raw, version: 5)
        return try! UUID(bytes: raw)
    }
    
    public static func v3(namespace: UUID, name: String) -> UUID {
        var data = Data(namespace.bytes)
        data.append(Data(name.utf8))
        let digest = [UInt8](md5(data))
        var raw = Array(digest.prefix(16))
        setVersionVariant(&raw, version: 3)
        return try! UUID(bytes: raw)
    }
    
    private static let _gregorianOffset: UInt64 = 122192928000000000
    private static let _clockSeq: UInt16 = UInt16.random(in: 0...0x3FFF)
    private static let _nodeBytes: [UInt8] = {
        var b = [UInt8](repeating: 0, count: 6)
        for i in 0..<6 { b[i] = UInt8.random(in: 0...255) }
        b[0] |= 0x01
        return b
    }()
    
    public static func v1() -> UUID {
        let timestamp = UInt64(Date().timeIntervalSince1970 * 1e7) + _gregorianOffset
        
        let timeLow = UInt32(timestamp & 0xFFFFFFFF)
        let timeMid = UInt16((timestamp >> 32) & 0xFFFF)
        let timeHi = UInt16((timestamp >> 48) & 0x0FFF)
        let timeHiAndVersion = UInt16(0x1000) | timeHi
        
        let clockSeqHi = UInt8(0x80 | (_clockSeq >> 8))
        let clockSeqLow = UInt8(_clockSeq & 0xFF)
        
        var raw = [UInt8](repeating: 0, count: 16)
        raw[0] = UInt8((timeLow >> 24) & 0xFF)
        raw[1] = UInt8((timeLow >> 16) & 0xFF)
        raw[2] = UInt8((timeLow >> 8) & 0xFF)
        raw[3] = UInt8(timeLow & 0xFF)
        
        raw[4] = UInt8((timeMid >> 8) & 0xFF)
        raw[5] = UInt8(timeMid & 0xFF)
        
        raw[6] = UInt8((timeHiAndVersion >> 8) & 0xFF)
        raw[7] = UInt8(timeHiAndVersion & 0xFF)
        
        raw[8] = clockSeqHi
        raw[9] = clockSeqLow
        
        for i in 0..<6 {
            raw[10+i] = _nodeBytes[i]
        }
        
        return try! UUID(bytes: raw)
    }
    
    public static func v7() -> UUID {
        let tsMs = UInt64(Date().timeIntervalSince1970 * 1000)
        
        var randBytes = [UInt8](repeating: 0, count: 10)
        for i in 0..<10 { randBytes[i] = UInt8.random(in: 0...255) }
        
        var raw = [UInt8](repeating: 0, count: 16)
        raw[0] = UInt8((tsMs >> 40) & 0xFF)
        raw[1] = UInt8((tsMs >> 32) & 0xFF)
        raw[2] = UInt8((tsMs >> 24) & 0xFF)
        raw[3] = UInt8((tsMs >> 16) & 0xFF)
        raw[4] = UInt8((tsMs >> 8) & 0xFF)
        raw[5] = UInt8(tsMs & 0xFF)
        
        raw[6] = 0x70 | (randBytes[0] & 0x0F)
        raw[7] = randBytes[1]
        
        raw[8] = 0x80 | (randBytes[2] & 0x3F)
        for i in 3..<10 {
            raw[6 + i] = randBytes[i]
        }
        
        return try! UUID(bytes: raw)
    }
}

extension UUID: CustomStringConvertible {
    public var description: String {
        let hex = _bytes.map { String(format: "%02x", $0) }.joined()
        let p1 = hex.prefix(8)
        let p2 = hex.dropFirst(8).prefix(4)
        let p3 = hex.dropFirst(12).prefix(4)
        let p4 = hex.dropFirst(16).prefix(4)
        let p5 = hex.dropFirst(20)
        return "\(p1)-\(p2)-\(p3)-\(p4)-\(p5)"
    }
}
