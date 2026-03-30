// Analysis.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - Cryptanalysis of the Caesar Cipher
// ============================================================================
//
// The Caesar cipher is famously weak. With only 25 possible non-trivial
// shifts (shift 0 is the identity), an attacker can simply try all of them.
// This is called a **brute-force attack** and it takes negligible time
// even by hand.
//
// A more sophisticated approach is **frequency analysis**, which exploits
// the fact that letters in natural language are not equally common. In
// English, 'E' appears about 12.7% of the time, while 'Z' appears only
// about 0.07%. Because the Caesar cipher shifts ALL letters by the same
// amount, the frequency distribution of letters is preserved — it's just
// shifted. By comparing the observed frequencies in the ciphertext with
// the known frequencies of English, we can deduce the shift.
//
// ============================================================================
// History of Frequency Analysis
// ============================================================================
//
// Frequency analysis was first described by the Arab polymath Al-Kindi
// in the 9th century, in his work "A Manuscript on Deciphering
// Cryptographic Messages." This technique rendered all simple substitution
// ciphers (including the Caesar cipher) breakable, and drove the
// development of more complex ciphers like the Vigenere cipher.
//
// ============================================================================


// MARK: - Brute Force Attack

/// The result of attempting to decrypt a ciphertext with a specific shift.
///
/// Used by `bruteForce(_:)` to return all 26 possible decryptions of a
/// Caesar-encrypted message. The analyst can then read through the results
/// to find the one that produces readable text.
///
/// ## Fields
/// - `shift`: The shift value that was tried (0 through 25).
/// - `plaintext`: The result of decrypting the ciphertext with this shift.
public struct BruteForceResult: Equatable, Sendable {
    /// The shift value used to produce this decryption (0-25).
    public let shift: Int

    /// The decrypted text produced by this shift.
    public let plaintext: String

    /// Creates a new brute-force result.
    ///
    /// - Parameters:
    ///   - shift: The shift value (0-25).
    ///   - plaintext: The decrypted text.
    public init(shift: Int, plaintext: String) {
        self.shift = shift
        self.plaintext = plaintext
    }
}


/// Performs a brute-force attack on Caesar-encrypted ciphertext.
///
/// Tries all 26 possible shifts (0 through 25) and returns the decrypted
/// text for each. The caller can inspect the results to find the one that
/// produces readable English (or whatever the original language was).
///
/// This attack is trivial because the Caesar cipher has such a tiny key
/// space — only 26 possibilities. By contrast, a modern cipher like AES-256
/// has 2^256 possible keys, making brute force computationally infeasible.
///
/// - Parameter ciphertext: The encrypted text to attack.
/// - Returns: An array of 26 `BruteForceResult` values, one for each
///   possible shift from 0 to 25.
///
/// ## Example
///
/// ```swift
/// let results = bruteForce("KHOOR")
/// // results[0]  → BruteForceResult(shift: 0, plaintext: "KHOOR")
/// // results[3]  → BruteForceResult(shift: 3, plaintext: "HELLO")  ← readable!
/// // results[25] → BruteForceResult(shift: 25, plaintext: "LIPPS")
/// ```
///
/// ## Complexity
///
/// - Time: O(26 * n) where n is the length of the ciphertext. Since 26
///   is a constant, this simplifies to O(n).
/// - Space: O(26 * n) for storing all 26 decryptions.
public func bruteForce(_ ciphertext: String) -> [BruteForceResult] {
    // Try every possible shift from 0 to 25. For each shift, decrypt the
    // entire ciphertext and store the result. The attacker then reads
    // through the results looking for the one that makes sense.
    //
    // Note: shift 0 always returns the ciphertext unchanged. We include
    // it for completeness — the results array is indexed by shift value,
    // so results[s] gives the decryption for shift s.
    return (0..<26).map { shift in
        BruteForceResult(shift: shift, plaintext: decrypt(ciphertext, shift: shift))
    }
}


// MARK: - Letter Frequency Table

/// Known letter frequencies in English text, expressed as proportions (0.0 to 1.0).
///
/// These values are derived from large-scale analysis of English text corpora.
/// The most common letter is 'E' at approximately 12.7%, followed by 'T' at
/// about 9.1%. The least common letters are 'Z' (0.07%) and 'Q' (0.10%).
///
/// ## Why These Numbers Matter
///
/// In any sufficiently long English text, the letter distribution will
/// approximate these values. The Caesar cipher preserves this distribution
/// but shifts it. So if we see that the most common letter in the
/// ciphertext is 'H' instead of 'E', we can guess the shift is 3
/// (since E + 3 = H).
///
/// ## Source
///
/// These frequencies are based on analysis of the Oxford English Corpus
/// and other large text collections. They represent averages — individual
/// texts may vary, especially short ones.
///
/// ```
/// E ████████████▊         12.70%
/// T █████████▏             9.06%
/// A ████████▏              8.17%
/// O ███████▌               7.51%
/// I ██████▉                6.97%
/// N ██████▋                6.75%
/// S ██████▎                6.33%
/// H ██████▏                6.09%
/// R █████▉                 5.99%
/// ...
/// Z ▏                      0.07%
/// ```
public let englishFrequencies: [Character: Double] = [
    "a": 0.0817, "b": 0.0150, "c": 0.0278, "d": 0.0425,
    "e": 0.1270, "f": 0.0223, "g": 0.0202, "h": 0.0609,
    "i": 0.0697, "j": 0.0015, "k": 0.0077, "l": 0.0403,
    "m": 0.0241, "n": 0.0675, "o": 0.0751, "p": 0.0193,
    "q": 0.0010, "r": 0.0599, "s": 0.0633, "t": 0.0906,
    "u": 0.0276, "v": 0.0098, "w": 0.0236, "x": 0.0015,
    "y": 0.0197, "z": 0.0007,
]


// MARK: - Frequency Analysis

/// Uses frequency analysis to determine the most likely shift used to
/// encrypt the given ciphertext.
///
/// This function counts the frequency of each letter in the ciphertext,
/// then compares the observed distribution against the known English letter
/// frequency distribution for each possible shift. The shift that produces
/// the best match (lowest chi-squared statistic) is returned as the answer.
///
/// ## The Chi-Squared Statistic
///
/// For each candidate shift `s`, we "unshift" the observed frequencies by
/// `s` positions and compare against expected English frequencies. The
/// chi-squared statistic measures how far the observed frequencies deviate
/// from expected:
///
///   chi2 = sum over all letters of: (observed - expected)^2 / expected
///
/// A lower chi-squared value means a better fit. We pick the shift with
/// the lowest chi-squared score.
///
/// ## Limitations
///
/// - Works best on longer texts (50+ characters of actual letters).
/// - Assumes the plaintext is English. Other languages have different
///   frequency distributions.
/// - Very short texts may not have enough statistical signal.
/// - If the plaintext doesn't follow normal English letter distributions
///   (e.g., "zzzzz"), the analysis will fail.
///
/// - Parameter ciphertext: The encrypted text to analyze.
/// - Returns: A `BruteForceResult` with the most likely shift and the
///   corresponding decrypted plaintext. If the ciphertext contains no
///   letters, returns shift 0 with the original text.
///
/// ## Example
///
/// ```swift
/// let result = frequencyAnalysis("KHOOR ZRUOG")
/// // result.shift == 3
/// // result.plaintext == "HELLO WORLD"
/// ```
public func frequencyAnalysis(_ ciphertext: String) -> BruteForceResult {
    // ── Step 1: Count letter frequencies in the ciphertext ──────────────
    //
    // We convert everything to lowercase for counting, since the Caesar
    // cipher treats 'A' and 'a' identically for frequency purposes.
    var counts: [Character: Int] = [:]
    var totalLetters = 0

    for char in ciphertext.lowercased() {
        if char.isLetter && char.isASCII {
            counts[char, default: 0] += 1
            totalLetters += 1
        }
    }

    // If there are no letters to analyze, there's nothing to do.
    // Return shift 0 (identity) and the original text.
    guard totalLetters > 0 else {
        return BruteForceResult(shift: 0, plaintext: ciphertext)
    }

    // ── Step 2: Convert counts to proportions ──────────────────────────
    //
    // We divide each count by the total number of letters to get the
    // observed frequency as a proportion (0.0 to 1.0). This makes it
    // comparable to our reference English frequencies.
    let total = Double(totalLetters)
    var observed: [Character: Double] = [:]
    for (char, count) in counts {
        observed[char] = Double(count) / total
    }

    // ── Step 3: Try each possible shift and compute chi-squared ────────
    //
    // For each candidate shift s (0 to 25), we ask: "If the original
    // plaintext was shifted by s to produce this ciphertext, what would
    // the letter frequencies look like?" We then compare against known
    // English frequencies using the chi-squared statistic.
    //
    // Concretely: if the ciphertext letter 'H' has frequency 0.127 and
    // we're testing shift 3, then we check if English letter 'E' (which
    // is 'H' shifted back by 3) has a similar frequency. If it does for
    // all letters, shift 3 is likely correct.
    var bestShift = 0
    var bestChiSquared = Double.infinity

    let aScalar = UnicodeScalar("a").value

    for shift in 0..<26 {
        var chiSquared = 0.0

        for i in 0..<26 {
            // The letter in the ciphertext at position i
            let cipherLetter = Character(UnicodeScalar(aScalar + UInt32(i))!)

            // If this ciphertext letter came from a plaintext letter shifted
            // by `shift`, then the original plaintext letter was at position
            // (i - shift) mod 26.
            let plainIndex = ((i - shift) % 26 + 26) % 26
            let plainLetter = Character(UnicodeScalar(aScalar + UInt32(plainIndex))!)

            // The observed frequency of this letter in the ciphertext
            let obs = observed[cipherLetter] ?? 0.0

            // The expected frequency of the corresponding plaintext letter
            // in English
            let exp = englishFrequencies[plainLetter] ?? 0.0

            // Chi-squared contribution: (observed - expected)^2 / expected
            // We add a tiny epsilon to avoid division by zero for very rare
            // letters (Q, Z).
            if exp > 0 {
                chiSquared += (obs - exp) * (obs - exp) / exp
            }
        }

        if chiSquared < bestChiSquared {
            bestChiSquared = chiSquared
            bestShift = shift
        }
    }

    // ── Step 4: Decrypt with the best shift and return ─────────────────
    return BruteForceResult(
        shift: bestShift,
        plaintext: decrypt(ciphertext, shift: bestShift)
    )
}
