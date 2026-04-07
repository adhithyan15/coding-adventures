// ============================================================================
// Bitset.swift
// ============================================================================

public enum BitsetError: Error {
    case invalidCharacter(char: Character, position: Int)
}

public class Bitset: Equatable, CustomStringConvertible {
    public private(set) var size: Int
    private var words: [UInt64]

    public var capacity: Int {
        return words.count * 64
    }

    public init(size: Int) {
        precondition(size >= 0, "Size must be non-negative")
        self.size = size
        let numWords = size == 0 ? 0 : (size + 63) / 64
        self.words = Array(repeating: 0, count: numWords)
    }

    public convenience init(fromInteger value: UInt64) {
        if value == 0 {
            self.init(size: 0)
            return
        }
        let size = 64 - value.leadingZeroBitCount
        self.init(size: size)
        self.words[0] = value
    }

    public convenience init(fromBinaryStr s: String) throws {
        self.init(size: s.count)
        for (i, char) in s.enumerated() {
            let bitPos = s.count - 1 - i
            if char == "1" {
                self.set(bitPos)
            } else if char != "0" {
                throw BitsetError.invalidCharacter(char: char, position: i)
            }
        }
    }

    private func ensureCapacity(forIndex i: Int) {
        if i < self.capacity { return }
        var newCapacity = self.capacity == 0 ? 64 : self.capacity
        while newCapacity <= i {
            newCapacity *= 2
        }
        let newNumWords = newCapacity / 64
        while self.words.count < newNumWords {
            self.words.append(0)
        }
    }

    private func cleanTrailingBits() {
        if size == 0 { return }
        let bitOffset = size % 64
        if bitOffset != 0 {
            let mask = (1 as UInt64 &<< bitOffset) - 1
            if words.count > 0 {
                words[words.count - 1] &= mask
            }
        }
    }

    public func set(_ i: Int) {
        precondition(i >= 0, "Index must be non-negative")
        ensureCapacity(forIndex: i)
        if i >= size {
            size = i + 1
        }
        let wordIndex = i / 64
        let bitOffset = i % 64
        words[wordIndex] |= (1 as UInt64 &<< bitOffset)
    }

    public func clear(_ i: Int) {
        precondition(i >= 0, "Index must be non-negative")
        if i >= size { return }
        let wordIndex = i / 64
        let bitOffset = i % 64
        words[wordIndex] &= ~(1 as UInt64 &<< bitOffset)
    }

    public func test(_ i: Int) -> Bool {
        precondition(i >= 0, "Index must be non-negative")
        if i >= size { return false }
        let wordIndex = i / 64
        let bitOffset = i % 64
        return (words[wordIndex] & (1 as UInt64 &<< bitOffset)) != 0
    }

    public func toggle(_ i: Int) {
        precondition(i >= 0, "Index must be non-negative")
        ensureCapacity(forIndex: i)
        if i >= size {
            size = i + 1
        }
        let wordIndex = i / 64
        let bitOffset = i % 64
        words[wordIndex] ^= (1 as UInt64 &<< bitOffset)
    }

    public func and(_ other: Bitset) -> Bitset {
        let maxLen = Swift.max(self.size, other.size)
        let result = Bitset(size: maxLen)
        for i in 0..<result.words.count {
            let aWord = i < self.words.count ? self.words[i] : 0
            let bWord = i < other.words.count ? other.words[i] : 0
            result.words[i] = aWord & bWord
        }
        result.cleanTrailingBits()
        return result
    }

    public func or(_ other: Bitset) -> Bitset {
        let maxLen = Swift.max(self.size, other.size)
        let result = Bitset(size: maxLen)
        for i in 0..<result.words.count {
            let aWord = i < self.words.count ? self.words[i] : 0
            let bWord = i < other.words.count ? other.words[i] : 0
            result.words[i] = aWord | bWord
        }
        result.cleanTrailingBits()
        return result
    }

    public func xor(_ other: Bitset) -> Bitset {
        let maxLen = Swift.max(self.size, other.size)
        let result = Bitset(size: maxLen)
        for i in 0..<result.words.count {
            let aWord = i < self.words.count ? self.words[i] : 0
            let bWord = i < other.words.count ? other.words[i] : 0
            result.words[i] = aWord ^ bWord
        }
        result.cleanTrailingBits()
        return result
    }

    public func not() -> Bitset {
        let result = Bitset(size: self.size)
        for i in 0..<result.words.count {
            result.words[i] = ~self.words[i]
        }
        result.cleanTrailingBits()
        return result
    }

    public func andNot(_ other: Bitset) -> Bitset {
        let maxLen = Swift.max(self.size, other.size)
        let result = Bitset(size: maxLen)
        for i in 0..<result.words.count {
            let aWord = i < self.words.count ? self.words[i] : 0
            let bWord = i < other.words.count ? other.words[i] : 0
            result.words[i] = aWord & ~bWord
        }
        result.cleanTrailingBits()
        return result
    }

    public func popcount() -> Int {
        var count = 0
        for w in words {
            count += w.nonzeroBitCount
        }
        return count
    }

    public func any() -> Bool {
        for w in words {
            if w != 0 { return true }
        }
        return false
    }

    public func all() -> Bool {
        if size == 0 { return true }
        let numFullWords = size / 64
        for i in 0..<numFullWords {
            if words[i] != UInt64.max { return false }
        }
        let remainder = size % 64
        if remainder > 0 {
            let mask = (1 as UInt64 &<< remainder) - 1
            if words[words.count - 1] & mask != mask { return false }
        }
        return true
    }

    public func none() -> Bool {
        return !any()
    }

    public func toInteger() -> UInt64? {
        if size == 0 { return 0 }
        for i in 1..<words.count {
            if words[i] != 0 { return nil } // Overflow
        }
        return words[0]
    }

    public func toBinaryStr() -> String {
        if size == 0 { return "" }
        var result = ""
        for i in (0..<size).reversed() {
            result += test(i) ? "1" : "0"
        }
        return result
    }

    public var description: String {
        return "Bitset(\(toBinaryStr()))"
    }

    public static func == (lhs: Bitset, rhs: Bitset) -> Bool {
        if lhs.size != rhs.size { return false }
        let maxWords = Swift.max(lhs.words.count, rhs.words.count)
        for i in 0..<maxWords {
            let aWord = i < lhs.words.count ? lhs.words[i] : 0
            let bWord = i < rhs.words.count ? rhs.words[i] : 0
            if aWord != bWord { return false }
        }
        return true
    }
}
