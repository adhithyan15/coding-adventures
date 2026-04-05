// ScytaleCipher.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// The Scytale Cipher
// ============================================================================
//
// The Scytale (pronounced "SKIT-ah-lee") cipher is a *transposition* cipher
// from ancient Sparta (~700 BCE). Unlike substitution ciphers (Caesar, Atbash)
// which replace characters, the Scytale rearranges character positions using
// a columnar transposition.
//
// How Encryption Works
// --------------------
//
// 1. Write text row-by-row into a grid with `key` columns.
// 2. Pad the last row with spaces if needed.
// 3. Read column-by-column to produce ciphertext.
//
// Example: encrypt("HELLO WORLD", 3)
//
//     Grid (4 rows x 3 cols):
//         H E L
//         L O ' '
//         W O R
//         L D ' '
//
//     Columns: HLWL + EOOD + L R  = "HLWLEOODL R "
//
// How Decryption Works
// --------------------
//
// 1. Calculate rows = ceil(len / key).
// 2. Write ciphertext column-by-column.
// 3. Read row-by-row and strip trailing padding spaces.
//
// Why It's Insecure
// -----------------
//
// The key space is tiny: for a message of length n, there are only
// about n/2 possible keys. An attacker can try every key in milliseconds.

/// Errors that can occur during Scytale cipher operations.
public enum ScytaleCipherError: Error {
    case keyTooSmall(Int)
    case keyTooLarge(Int, textLength: Int)
}

/// A single brute-force decryption result.
public struct BruteForceResult {
    public let key: Int
    public let text: String
}

/// Encrypt text using the Scytale transposition cipher.
///
/// The text is written row-by-row into a grid with `key` columns,
/// then read column-by-column. All characters are preserved. The
/// last row is padded with spaces if needed.
///
/// - Parameters:
///   - text: The plaintext string to encrypt.
///   - key: The number of columns (>= 2, <= text length).
/// - Returns: The transposed ciphertext.
/// - Throws: `ScytaleCipherError` if key is out of range.
public func encrypt(_ text: String, key: Int) throws -> String {
    if text.isEmpty { return "" }

    let chars = Array(text)
    let n = chars.count

    guard key >= 2 else { throw ScytaleCipherError.keyTooSmall(key) }
    guard key <= n else { throw ScytaleCipherError.keyTooLarge(key, textLength: n) }

    // Calculate grid dimensions and pad
    let numRows = (n + key - 1) / key
    let paddedLen = numRows * key
    var padded = chars
    padded.append(contentsOf: Array(repeating: Character(" "), count: paddedLen - n))

    // Read column-by-column
    var result = ""
    result.reserveCapacity(paddedLen)
    for col in 0..<key {
        for row in 0..<numRows {
            result.append(padded[row * key + col])
        }
    }

    return result
}

/// Decrypt ciphertext that was encrypted with the Scytale cipher.
///
/// Trailing padding spaces are stripped.
///
/// - Parameters:
///   - text: The ciphertext to decrypt.
///   - key: The number of columns used during encryption.
/// - Returns: The decrypted plaintext (trailing pad stripped).
/// - Throws: `ScytaleCipherError` if key is out of range.
public func decrypt(_ text: String, key: Int) throws -> String {
    if text.isEmpty { return "" }

    let chars = Array(text)
    let n = chars.count

    guard key >= 2 else { throw ScytaleCipherError.keyTooSmall(key) }
    guard key <= n else { throw ScytaleCipherError.keyTooLarge(key, textLength: n) }

    let numRows = (n + key - 1) / key

    // Handle uneven grids (when n % key != 0, e.g. during brute-force)
    let fullCols = n % key == 0 ? key : n % key

    // Compute column start indices and lengths
    var colStarts = [Int]()
    var colLens = [Int]()
    var offset = 0
    for c in 0..<key {
        colStarts.append(offset)
        let colLen = (n % key == 0 || c < fullCols) ? numRows : numRows - 1
        colLens.append(colLen)
        offset += colLen
    }

    // Read row-by-row
    var result = ""
    result.reserveCapacity(n)
    for row in 0..<numRows {
        for col in 0..<key {
            if row < colLens[col] {
                result.append(chars[colStarts[col] + row])
            }
        }
    }

    // Strip trailing padding spaces
    while result.last == " " {
        result.removeLast()
    }

    return result
}

/// Try all possible Scytale keys and return decryption results.
///
/// Keys range from 2 to len/2.
///
/// - Parameter text: The ciphertext to brute-force.
/// - Returns: Array of `BruteForceResult` values.
public func bruteForce(_ text: String) -> [BruteForceResult] {
    let n = text.count
    if n < 4 { return [] }

    let maxKey = n / 2
    var results: [BruteForceResult] = []

    for candidateKey in 2...maxKey {
        if let decrypted = try? decrypt(text, key: candidateKey) {
            results.append(BruteForceResult(key: candidateKey, text: decrypted))
        }
    }

    return results
}
