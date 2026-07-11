// atbash_cipher.hpp — the Atbash substitution cipher, in pure ISO C++17,
// header-only, in namespace ca::atbash. A faithful port of the Rust
// `atbash-cipher` crate.
// ===========================================================================
//
// Atbash is one of the oldest known ciphers. It reverses the alphabet — A maps
// to Z, B to Y, ..., Z to A — preserving case and passing every non-letter
// through unchanged:
//
//   Forward:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
//   Reversed: Z Y X W V U T S R Q P O N M L K J I H G F E D C B A
//
// For a letter at position p (A=0 .. Z=25) the new position is `25 - p`.
//
// SELF-INVERSE. Applying Atbash twice returns the original text
// (`25 - (25 - p) = p`), so `decrypt` is literally `encrypt`.
//
// This port operates byte-by-byte: only ASCII letters are substituted; every
// other byte (including the bytes of a UTF-8 sequence) passes through — exactly
// matching the crate, which transforms only ASCII letters.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_ATBASH_CIPHER_HPP
#define CA_ATBASH_CIPHER_HPP

#include <string>

namespace ca {
namespace atbash {

// Apply the Atbash substitution to one character: an ASCII letter is reversed
// within its case, any other byte is returned unchanged.
inline char atbash_char(char ch) {
    unsigned char c = static_cast<unsigned char>(ch);
    if (c >= 'A' && c <= 'Z') {
        return static_cast<char>('A' + (25 - (c - 'A')));
    }
    if (c >= 'a' && c <= 'z') {
        return static_cast<char>('a' + (25 - (c - 'a')));
    }
    return ch;
}

// Encrypt `text` by applying Atbash to every character.
inline std::string encrypt(const std::string& text) {
    std::string result;
    result.reserve(text.size());
    for (char ch : text) {
        result.push_back(atbash_char(ch));
    }
    return result;
}

// Decrypt `text`. Atbash is self-inverse, so decryption is encryption.
inline std::string decrypt(const std::string& text) { return encrypt(text); }

}  // namespace atbash
}  // namespace ca

#endif  // CA_ATBASH_CIPHER_HPP
