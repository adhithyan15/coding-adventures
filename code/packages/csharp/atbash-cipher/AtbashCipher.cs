namespace CodingAdventures.AtbashCipher;

/// <summary>
/// The fixed reverse-alphabet substitution cipher.
///
/// Atbash has no key at all: A maps to Z, B maps to Y, and so on. That makes
/// it easy to understand and even easier to break, but it is a perfect example
/// of a self-inverse transformation.
/// </summary>
public static class AtbashCipher
{
    private static char Transform(char ch)
    {
        if (ch is >= 'A' and <= 'Z')
        {
            var position = ch - 'A';
            return (char)('A' + (25 - position));
        }

        if (ch is >= 'a' and <= 'z')
        {
            var position = ch - 'a';
            return (char)('a' + (25 - position));
        }

        return ch;
    }

    /// <summary>
    /// Apply the Atbash substitution to every ASCII letter in the input.
    /// </summary>
    public static string Encrypt(string text)
    {
        ArgumentNullException.ThrowIfNull(text);

        var chars = text.ToCharArray();
        for (var i = 0; i < chars.Length; i++)
        {
            chars[i] = Transform(chars[i]);
        }

        return new string(chars);
    }

    /// <summary>
    /// Decrypt Atbash text.
    ///
    /// Since Atbash is self-inverse, decryption is the same operation as
    /// encryption.
    /// </summary>
    public static string Decrypt(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        return Encrypt(text);
    }
}
