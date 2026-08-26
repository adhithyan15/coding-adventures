/// Applies the fixed Atbash substitution to [text].
///
/// Atbash mirrors each ASCII letter within its own alphabet: `A` becomes `Z`,
/// `B` becomes `Y`, and the same rule applies independently to lowercase
/// letters. Every other UTF-16 code unit is copied unchanged. Working at the
/// code-unit level is safe here because an ASCII code unit can never be part of
/// a surrogate pair.
String encrypt(String text) =>
    String.fromCharCodes(text.codeUnits.map(_mirror));

/// Reverses Atbash encryption.
///
/// Mirroring twice is the identity, so decryption is exactly encryption.
String decrypt(String text) => encrypt(text);

int _mirror(int codeUnit) {
  const upperA = 0x41;
  const upperZ = 0x5a;
  const lowerA = 0x61;
  const lowerZ = 0x7a;

  if (codeUnit >= upperA && codeUnit <= upperZ) {
    return upperA + upperZ - codeUnit;
  }
  if (codeUnit >= lowerA && codeUnit <= lowerZ) {
    return lowerA + lowerZ - codeUnit;
  }
  return codeUnit;
}
