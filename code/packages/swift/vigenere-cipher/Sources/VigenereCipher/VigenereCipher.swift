// VigenereCipher.swift
// Part of coding-adventures -- an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// The Vigenere Cipher
// ============================================================================
//
// The Vigenere cipher is a *polyalphabetic substitution* cipher invented by
// Giovan Battista Bellaso in 1553 and later misattributed to Blaise de
// Vigenere. For 300 years it was considered "le chiffre indechiffrable"
// until Friedrich Kasiski published a general method for breaking it in 1863.
//
// How It Works (Encryption)
// -------------------------
//
// Unlike a Caesar cipher (one fixed shift), the Vigenere cipher uses a
// *keyword* to apply a different shift at each position:
//
//     Plaintext:  A  T  T  A  C  K  A  T  D  A  W  N
//     Keyword:    L  E  M  O  N  L  E  M  O  N  L  E
//     Shift:      11 4  12 14 13 11 4  12 14 13 11 4
//     Ciphertext: L  X  F  O  P  V  E  F  R  N  H  R
//
// Each plaintext letter is shifted forward by the amount indicated by the
// corresponding keyword letter (A=0, B=1, ... Z=25). Non-alphabetic
// characters pass through unchanged and do NOT advance the keyword position.
//
// How It Works (Decryption)
// -------------------------
//
// Reverse the process: shift each letter *backward* by the keyword amount.
//
// Cryptanalysis (Breaking the Cipher)
// ------------------------------------
//
// Step 1 -- Find the key length using the Index of Coincidence (IC).
// Step 2 -- Find each key letter using chi-squared analysis.

/// Errors that can occur during Vigenere cipher operations.
public enum VigenereCipherError: Error {
    case emptyKey
    case nonAlphabeticKey(String)
}

// ============================================================================
// English Letter Frequencies
// ============================================================================
//
// Expected frequencies for A-Z in typical English text, used by the
// chi-squared test to determine the most likely shift for each key position.

private let englishFreq: [Double] = [
    0.08167, 0.01492, 0.02782, 0.04253, 0.12702, 0.02228, // A-F
    0.02015, 0.06094, 0.06966, 0.00153, 0.00772, 0.04025, // G-L
    0.02406, 0.06749, 0.07507, 0.01929, 0.00095, 0.05987, // M-R
    0.06327, 0.09056, 0.02758, 0.00978, 0.02360, 0.00150, // S-X
    0.01974, 0.00074,                                       // Y-Z
]

// ============================================================================
// Helper: Convert a Character to its 0-25 alphabet index
// ============================================================================

private func alphaIndex(_ ch: Character) -> Int? {
    if ch.isUppercase, let ascii = ch.asciiValue {
        return Int(ascii) - 65
    } else if ch.isLowercase, let ascii = ch.asciiValue {
        return Int(ascii) - 97
    }
    return nil
}

// ============================================================================
// Helper: Validate the key
// ============================================================================

private func validateKey(_ key: String) throws -> [Int] {
    guard !key.isEmpty else { throw VigenereCipherError.emptyKey }
    var shifts: [Int] = []
    for ch in key {
        guard let idx = alphaIndex(ch) else {
            throw VigenereCipherError.nonAlphabeticKey(key)
        }
        shifts.append(idx)
    }
    return shifts
}

// ============================================================================
// encrypt(_:key:) -> String
// ============================================================================
//
// Encrypt plaintext using the Vigenere cipher with the given key.
//
// Rules:
//   * Key must be non-empty and contain only A-Z / a-z.
//   * Uppercase letters stay uppercase; lowercase stay lowercase.
//   * Non-alphabetic characters pass through unchanged.
//   * The key position advances only on alphabetic characters.
//
// Example:
//   encrypt("ATTACKATDAWN", key: "LEMON") --> "LXFOPVEFRNHR"
//   encrypt("Hello, World!", key: "key")  --> "Rijvs, Uyvjn!"

public func encrypt(_ plaintext: String, key: String) throws -> String {
    let shifts = try validateKey(key)
    let keyLen = shifts.count
    var keyIdx = 0
    var result = ""
    result.reserveCapacity(plaintext.count)

    for ch in plaintext {
        if let idx = alphaIndex(ch) {
            let shift = shifts[keyIdx % keyLen]
            let shifted = (idx + shift) % 26

            // Preserve the original case
            if ch.isUppercase {
                result.append(Character(UnicodeScalar(shifted + 65)!))
            } else {
                result.append(Character(UnicodeScalar(shifted + 97)!))
            }
            keyIdx += 1
        } else {
            // Non-alpha passes through; key does NOT advance
            result.append(ch)
        }
    }

    return result
}

// ============================================================================
// decrypt(_:key:) -> String
// ============================================================================
//
// Decrypt ciphertext by shifting each letter *backward* by the key amount.
// Exact inverse of encrypt.
//
// Example:
//   decrypt("LXFOPVEFRNHR", key: "LEMON") --> "ATTACKATDAWN"

public func decrypt(_ ciphertext: String, key: String) throws -> String {
    let shifts = try validateKey(key)
    let keyLen = shifts.count
    var keyIdx = 0
    var result = ""
    result.reserveCapacity(ciphertext.count)

    for ch in ciphertext {
        if let idx = alphaIndex(ch) {
            let shift = shifts[keyIdx % keyLen]
            // Shift backward, add 26 to avoid negative modulo
            let shifted = (idx - shift + 26) % 26

            if ch.isUppercase {
                result.append(Character(UnicodeScalar(shifted + 65)!))
            } else {
                result.append(Character(UnicodeScalar(shifted + 97)!))
            }
            keyIdx += 1
        } else {
            result.append(ch)
        }
    }

    return result
}

// ============================================================================
// indexOfCoincidence(_:) -> Double
// ============================================================================
//
// The Index of Coincidence (IC) measures how likely it is that two randomly
// chosen letters from a text are the same. English IC ~ 0.0667; random ~ 0.0385.
//
// Formula: IC = sum(n_i * (n_i - 1)) / (N * (N - 1))

private func indexOfCoincidence(_ text: String) -> Double {
    var counts = [Int](repeating: 0, count: 26)
    var total = 0

    for ch in text {
        if let idx = alphaIndex(ch) {
            counts[idx] += 1
            total += 1
        }
    }

    guard total > 1 else { return 0.0 }

    var sum = 0
    for c in counts {
        sum += c * (c - 1)
    }

    return Double(sum) / Double(total * (total - 1))
}

// ============================================================================
// findKeyLength(_:maxLength:) -> Int
// ============================================================================
//
// Estimate the key length of a Vigenere-encrypted ciphertext using
// Index of Coincidence analysis.
//
// For each candidate key length k (2..maxLength):
//   1. Split ciphertext into k groups (every k-th letter).
//   2. Compute IC of each group.
//   3. Average the ICs.
// The key length with the highest average IC is most likely correct.

public func findKeyLength(_ ciphertext: String, maxLength: Int = 20) -> Int {
    // Extract only alphabetic characters
    let alphaOnly = Array(ciphertext.filter { $0.isLetter })
    let n = alphaOnly.count

    if n < 2 { return 1 }

    var bestLength = 1
    var bestIC: Double = -1.0

    let limit = min(maxLength, n / 2)

    for k in 2...limit {
        var icSum: Double = 0.0

        for j in 0..<k {
            // Build group: every k-th character starting at position j
            var group = ""
            var pos = j
            while pos < n {
                group.append(alphaOnly[pos])
                pos += k
            }
            icSum += indexOfCoincidence(group)
        }

        let avgIC = icSum / Double(k)
        if avgIC > bestIC {
            bestIC = avgIC
            bestLength = k
        }
    }

    return bestLength
}

// ============================================================================
// chiSquared(_:total:) -> Double
// ============================================================================
//
// Chi-squared statistic against English letter frequencies.
// Lower values mean a better fit to English.

private func chiSquared(_ counts: [Int], total: Int) -> Double {
    var chi2: Double = 0.0
    for i in 0..<26 {
        let expected = Double(total) * englishFreq[i]
        if expected > 0 {
            let diff = Double(counts[i]) - expected
            chi2 += (diff * diff) / expected
        }
    }
    return chi2
}

// ============================================================================
// findKey(_:keyLength:) -> String
// ============================================================================
//
// Given a ciphertext and known key length, find the key by chi-squared
// analysis on each position.
//
// For each key position (0..keyLength-1):
//   1. Extract the group of letters at that position.
//   2. Try all 26 possible shifts.
//   3. The shift with the lowest chi-squared is the key letter.

public func findKey(_ ciphertext: String, keyLength: Int) -> String {
    // Extract only alpha characters as 0-25 indices
    var alphaIndices: [Int] = []
    for ch in ciphertext {
        if let idx = alphaIndex(ch) {
            alphaIndices.append(idx)
        }
    }
    let n = alphaIndices.count

    var keyChars: [Character] = []

    for pos in 0..<keyLength {
        // Gather letters at this key position
        var group: [Int] = []
        var idx = pos
        while idx < n {
            group.append(alphaIndices[idx])
            idx += keyLength
        }

        let groupSize = group.count
        if groupSize == 0 {
            keyChars.append("A")
            continue
        }

        // Try all 26 shifts, pick the one with lowest chi-squared
        var bestShift = 0
        var bestChi2 = Double.infinity

        for shift in 0..<26 {
            var counts = [Int](repeating: 0, count: 26)
            for val in group {
                let decrypted = (val - shift + 26) % 26
                counts[decrypted] += 1
            }

            let chi2 = chiSquared(counts, total: groupSize)
            if chi2 < bestChi2 {
                bestChi2 = chi2
                bestShift = shift
            }
        }

        keyChars.append(Character(UnicodeScalar(bestShift + 65)!))
    }

    return String(keyChars)
}

// ============================================================================
// breakCipher(_:) -> (key: String, plaintext: String)
// ============================================================================
//
// Automatic Vigenere cipher break. Combines findKeyLength and findKey
// to recover the key and plaintext without any prior knowledge.

public func breakCipher(_ ciphertext: String) throws -> (key: String, plaintext: String) {
    let keyLength = findKeyLength(ciphertext)
    let key = findKey(ciphertext, keyLength: keyLength)
    let plaintext = try decrypt(ciphertext, key: key)
    return (key: key, plaintext: plaintext)
}
