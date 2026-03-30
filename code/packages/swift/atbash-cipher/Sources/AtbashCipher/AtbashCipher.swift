// AtbashCipher.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// The Atbash Cipher
// ============================================================================
//
// The Atbash cipher is one of the oldest known substitution ciphers,
// originally used with the Hebrew alphabet. The name "Atbash" comes from
// the first, last, second, and second-to-last letters of the Hebrew
// alphabet: Aleph-Tav-Beth-Shin.
//
// The cipher reverses the alphabet:
//
//     Plain:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
//     Cipher: Z Y X W V U T S R Q P O N M L K J I H G F E D C B A
//
// The Formula
// -----------
//
// Given a letter at position p (where A=0, B=1, ..., Z=25):
//
//     encrypted_position = 25 - p
//
// For example, 'H' is at position 7: 25 - 7 = 18, which is 'S'.
//
// Self-Inverse Property
// ---------------------
//
// The Atbash cipher is self-inverse: applying it twice returns the original.
//
//     f(f(x)) = 25 - (25 - x) = x
//
// This means encrypt() and decrypt() are the same operation.
//
// Case Preservation
// -----------------
//
// Uppercase letters produce uppercase results; lowercase produce lowercase.
// Non-alphabetic characters (digits, punctuation, spaces) pass through unchanged.
//
// ============================================================================

/// The AtbashCipher namespace provides `encrypt` and `decrypt` functions
/// for the Atbash cipher, a simple reverse-alphabet substitution cipher.
public enum AtbashCipher {

    // ASCII scalar values for reference:
    // A = 65, Z = 90
    // a = 97, z = 122

    /// Apply the Atbash substitution to a single Unicode scalar.
    ///
    /// The algorithm:
    /// 1. Check if the scalar is an uppercase (A-Z) or lowercase (a-z) letter.
    /// 2. If it's a letter, compute its position (0-25), reverse it (25 - pos),
    ///    and convert back to a scalar.
    /// 3. If it's not a letter, return it unchanged.
    ///
    /// - Parameter scalar: A Unicode scalar value
    /// - Returns: The Atbash-transformed scalar value
    private static func atbashScalar(_ scalar: Unicode.Scalar) -> Unicode.Scalar {
        let value = scalar.value

        // Uppercase letters: A(65) through Z(90)
        if value >= 65 && value <= 90 {
            let position = value - 65       // A=0, B=1, ..., Z=25
            let newPosition = 25 - position // Reverse: 0->25, 1->24, ..., 25->0
            return Unicode.Scalar(65 + newPosition)!
        }

        // Lowercase letters: a(97) through z(122)
        if value >= 97 && value <= 122 {
            let position = value - 97       // a=0, b=1, ..., z=25
            let newPosition = 25 - position // Reverse
            return Unicode.Scalar(97 + newPosition)!
        }

        // Non-alphabetic scalars pass through unchanged
        return scalar
    }

    /// Encrypt text using the Atbash cipher.
    ///
    /// Each letter is replaced by its reverse in the alphabet (A<->Z, B<->Y, etc.).
    /// Non-alphabetic characters pass through unchanged. Case is preserved.
    ///
    /// Because the Atbash cipher is self-inverse, this function is identical
    /// to ``decrypt(_:)``. Both are provided for API clarity.
    ///
    /// - Parameter text: The plaintext string to encrypt.
    /// - Returns: The encrypted string with each letter reversed in the alphabet.
    ///
    /// ## Examples
    ///
    /// ```swift
    /// AtbashCipher.encrypt("HELLO")             // "SVOOL"
    /// AtbashCipher.encrypt("hello")             // "svool"
    /// AtbashCipher.encrypt("Hello, World! 123") // "Svool, Dliow! 123"
    /// ```
    public static func encrypt(_ text: String) -> String {
        // Process the string's Unicode scalars, apply Atbash to each,
        // and reconstruct a String from the result.
        var result = ""
        result.reserveCapacity(text.count)
        for scalar in text.unicodeScalars {
            result.unicodeScalars.append(atbashScalar(scalar))
        }
        return result
    }

    /// Decrypt text using the Atbash cipher.
    ///
    /// Because the Atbash cipher is self-inverse (applying it twice returns
    /// the original), decryption is identical to encryption. This function
    /// exists for API clarity.
    ///
    /// - Parameter text: The ciphertext string to decrypt.
    /// - Returns: The decrypted (original) string.
    ///
    /// ## Examples
    ///
    /// ```swift
    /// AtbashCipher.decrypt("SVOOL")                       // "HELLO"
    /// AtbashCipher.decrypt(AtbashCipher.encrypt("secret")) // "secret"
    /// ```
    public static func decrypt(_ text: String) -> String {
        // Decryption IS encryption for Atbash.
        // Proof: f(f(x)) = 25 - (25 - x) = x
        return encrypt(text)
    }
}
