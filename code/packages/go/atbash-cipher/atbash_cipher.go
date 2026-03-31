// Package atbashcipher implements the Atbash cipher, one of the oldest known
// substitution ciphers.
//
// # What is the Atbash Cipher?
//
// The Atbash cipher reverses the alphabet: A maps to Z, B maps to Y, C maps
// to X, and so on. The name comes from the Hebrew alphabet: Aleph-Tav-Beth-Shin.
//
// The mapping looks like this:
//
//	Plain:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
//	Cipher: Z Y X W V U T S R Q P O N M L K J I H G F E D C B A
//
// # The Formula
//
// Given a letter at position p (where A=0, B=1, ..., Z=25):
//
//	encrypted_position = 25 - p
//
// For example, 'H' is at position 7: 25 - 7 = 18, which is 'S'.
//
// # Self-Inverse Property
//
// The Atbash cipher is self-inverse: applying it twice returns the original.
//
//	f(f(x)) = 25 - (25 - x) = x
//
// This means Encrypt and Decrypt are the same operation.
//
// # Case Preservation and Non-Alpha Passthrough
//
// Uppercase letters produce uppercase results, lowercase produce lowercase.
// Non-alphabetic characters (digits, punctuation, spaces) pass through unchanged.
//
// This package is part of the coding-adventures monorepo.
package atbashcipher

// atbashChar applies the Atbash substitution to a single rune.
//
// The algorithm:
//  1. Check if the rune is an uppercase letter (A-Z) or lowercase (a-z).
//  2. If it's a letter, compute its position (0-25), reverse it (25 - pos),
//     and convert back to a rune.
//  3. If it's not a letter, return it unchanged.
func atbashChar(r rune) rune {
	switch {
	// Uppercase letters: 'A' (65) through 'Z' (90)
	case r >= 'A' && r <= 'Z':
		position := r - 'A'         // A=0, B=1, ..., Z=25
		newPosition := 25 - position // Reverse: 0->25, 1->24, ..., 25->0
		return 'A' + newPosition     // Convert back to a letter

	// Lowercase letters: 'a' (97) through 'z' (122)
	case r >= 'a' && r <= 'z':
		position := r - 'a'         // a=0, b=1, ..., z=25
		newPosition := 25 - position // Reverse: 0->25, 1->24, ..., 25->0
		return 'a' + newPosition     // Convert back to a letter

	// Non-alphabetic runes pass through unchanged
	default:
		return r
	}
}

// Encrypt applies the Atbash cipher to the given text.
//
// Each letter is replaced by its reverse in the alphabet (A<->Z, B<->Y, etc.).
// Non-alphabetic characters pass through unchanged. Case is preserved.
//
// Because the Atbash cipher is self-inverse, this function is identical
// to [Decrypt]. Both are provided for API clarity.
//
// Examples:
//
//	Encrypt("HELLO")             // returns "SVOOL"
//	Encrypt("hello")             // returns "svool"
//	Encrypt("Hello, World! 123") // returns "Svool, Dliow! 123"
func Encrypt(text string) string {
	// We build the result rune-by-rune. Using a []rune ensures correct
	// handling of multi-byte UTF-8 characters (though Atbash only transforms
	// ASCII letters, other runes pass through safely).
	runes := []rune(text)
	result := make([]rune, len(runes))
	for i, r := range runes {
		result[i] = atbashChar(r)
	}
	return string(result)
}

// Decrypt applies the Atbash cipher to the given ciphertext, returning
// the original plaintext.
//
// Because the Atbash cipher is self-inverse (applying it twice returns
// the original), decryption is identical to encryption. This function
// exists for API clarity.
//
// Examples:
//
//	Decrypt("SVOOL")                      // returns "HELLO"
//	Decrypt(Encrypt("secret message"))    // returns "secret message"
func Decrypt(text string) string {
	// Decryption IS encryption for the Atbash cipher.
	// Proof: f(f(x)) = 25 - (25 - x) = x
	return Encrypt(text)
}
