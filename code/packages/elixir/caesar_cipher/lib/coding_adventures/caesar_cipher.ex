defmodule CodingAdventures.CaesarCipher do
  @moduledoc """
  # The Caesar Cipher — History's First Encryption Algorithm

  The Caesar cipher is a **substitution cipher** named after Julius Caesar, who
  used it to protect military communications around 58 BC. Each letter in the
  plaintext is replaced by a letter a fixed number of positions down the alphabet.

  ## How It Works

  Imagine the alphabet written in a circle. To encrypt with a shift of 3:

      Plain:    A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
      Cipher:   D E F G H I J K L M N O P Q R S T U V W X Y Z A B C

  The letter 'A' becomes 'D', 'B' becomes 'E', and so on. When we reach the end
  of the alphabet, we wrap around: 'X' becomes 'A', 'Y' becomes 'B', 'Z' becomes 'C'.

  ## The Mathematics

  Encryption is modular arithmetic on letter positions (A=0, B=1, ..., Z=25):

      encrypt(letter, shift) = (letter + shift) mod 26
      decrypt(letter, shift) = (letter - shift) mod 26

  This means decryption is just encryption with a negated shift! And since there
  are only 26 possible shifts (0 through 25), the cipher can always be broken by
  trying all of them — a technique called **brute force**.

  ## Worked Example

      Plaintext:  "HELLO"
      Shift:      3

      H (7)  + 3 = 10 mod 26 = 10 → K
      E (4)  + 3 =  7 mod 26 =  7 → H
      L (11) + 3 = 14 mod 26 = 14 → O
      L (11) + 3 = 14 mod 26 = 14 → O
      O (14) + 3 = 17 mod 26 = 17 → R

      Ciphertext: "KHOOR"

  ## Why It Matters

  The Caesar cipher is the foundation of cryptography. While trivially breakable
  today, it introduces core concepts that appear in every modern cipher:

  - **Key space** — The set of all possible keys (here, 26 shifts).
  - **Brute force** — Trying every key until one works.
  - **Frequency analysis** — Using statistical patterns in language to crack codes.
  - **Modular arithmetic** — The mathematical backbone of all modern encryption.

  Understanding Caesar is the first step toward understanding AES, RSA, and
  every encryption algorithm that keeps the internet secure.
  """

  # ---------------------------------------------------------------------------
  # English Letter Frequencies
  # ---------------------------------------------------------------------------
  #
  # These are the expected frequencies of each letter in typical English text,
  # expressed as proportions (summing to ~1.0). They come from large-scale
  # analysis of English corpora.
  #
  # We use these for frequency analysis: if we decrypt ciphertext with the
  # *correct* shift, the letter distribution should closely match these values.
  # The chi-squared statistic measures how close the match is.
  #
  #     Letter | Freq   | Letter | Freq
  #     -------|--------|--------|------
  #       E    | 0.127  |   T    | 0.091
  #       A    | 0.082  |   O    | 0.075
  #       I    | 0.070  |   N    | 0.067
  #       S    | 0.063  |   H    | 0.061
  #       R    | 0.060  |   ...  | ...
  #
  # 'E' is by far the most common letter in English — roughly 12.7% of all
  # letters. This is the single most useful fact in classical cryptanalysis.

  @english_frequencies %{
    ?A => 0.08167, ?B => 0.01492, ?C => 0.02782, ?D => 0.04253,
    ?E => 0.12702, ?F => 0.02228, ?G => 0.02015, ?H => 0.06094,
    ?I => 0.06966, ?J => 0.00153, ?K => 0.00772, ?L => 0.04025,
    ?M => 0.02406, ?N => 0.06749, ?O => 0.07507, ?P => 0.01929,
    ?Q => 0.00095, ?R => 0.05987, ?S => 0.06327, ?T => 0.09056,
    ?U => 0.02758, ?V => 0.00978, ?W => 0.02361, ?X => 0.00150,
    ?Y => 0.01974, ?Z => 0.00074
  }

  # ---------------------------------------------------------------------------
  # Encryption
  # ---------------------------------------------------------------------------

  @doc """
  Encrypts plaintext using the Caesar cipher with the given shift.

  Each letter is shifted forward in the alphabet by `shift` positions. The shift
  wraps around using modular arithmetic, so a shift of 27 is the same as a shift
  of 1. Non-alphabetic characters (digits, spaces, punctuation) pass through
  unchanged. Letter case is preserved.

  ## Truth Table (shift = 3)

      | Input | Output | Reason                        |
      |-------|--------|-------------------------------|
      | 'A'   | 'D'    | 0 + 3 = 3 → 'D'             |
      | 'Z'   | 'C'    | 25 + 3 = 28 mod 26 = 2 → 'C'|
      | 'a'   | 'd'    | lowercase preserved           |
      | '5'   | '5'    | non-alpha passes through      |
      | ' '   | ' '    | spaces pass through           |

  ## Examples

      iex> CodingAdventures.CaesarCipher.encrypt("HELLO", 3)
      "KHOOR"

      iex> CodingAdventures.CaesarCipher.encrypt("abc", 1)
      "bcd"

      iex> CodingAdventures.CaesarCipher.encrypt("Hello, World!", 13)
      "Uryyb, Jbeyq!"

      iex> CodingAdventures.CaesarCipher.encrypt("xyz", 3)
      "abc"

  """
  @spec encrypt(String.t(), integer()) :: String.t()
  def encrypt(text, shift) do
    # Normalize the shift to 0..25 range. This handles:
    #   - Shifts > 25 (e.g., 27 → 1)
    #   - Negative shifts (e.g., -1 → 25)
    #   - Zero shift (returns text unchanged)
    normalized_shift = Integer.mod(shift, 26)

    text
    |> String.to_charlist()
    |> Enum.map(&shift_char(&1, normalized_shift))
    |> List.to_string()
  end

  # ---------------------------------------------------------------------------
  # Decryption
  # ---------------------------------------------------------------------------

  @doc """
  Decrypts ciphertext by reversing the Caesar cipher with the given shift.

  Decryption is simply encryption with the negated shift. If a message was
  encrypted with shift 3, we decrypt with shift -3 (equivalently, shift 23).

  This works because:

      encrypt(decrypt(letter, s), s) = letter
      (letter - s + s) mod 26 = letter mod 26 = letter

  ## Examples

      iex> CodingAdventures.CaesarCipher.decrypt("KHOOR", 3)
      "HELLO"

      iex> CodingAdventures.CaesarCipher.decrypt("bcd", 1)
      "abc"

  """
  @spec decrypt(String.t(), integer()) :: String.t()
  def decrypt(text, shift) do
    # Negating the shift and encrypting gives us decryption.
    # This is the beauty of symmetric ciphers — the same algorithm
    # works in both directions, just with an inverted key.
    encrypt(text, -shift)
  end

  # ---------------------------------------------------------------------------
  # ROT13 — A Special Case
  # ---------------------------------------------------------------------------

  @doc """
  Applies ROT13 — the Caesar cipher with shift 13.

  ROT13 is special because 13 is exactly half of 26 (the alphabet size).
  This means ROT13 is its own inverse:

      ROT13(ROT13(text)) = text

  Applying ROT13 twice gets you back to the original. This self-inverse
  property made ROT13 popular for hiding spoilers and punchlines on Usenet
  in the 1980s — you could "decrypt" just by running the same function again.

  ## Why shift 13?

  With 26 letters, shift 13 maps:

      A↔N  B↔O  C↔P  D↔Q  E↔R  F↔S  G↔T
      H↔U  I↔V  J↔W  K↔X  L↔Y  M↔Z

  Each letter swaps with its partner 13 positions away. No letter maps to
  itself (except for non-alpha characters, which pass through unchanged).

  ## Examples

      iex> CodingAdventures.CaesarCipher.rot13("Hello")
      "Uryyb"

      iex> CodingAdventures.CaesarCipher.rot13("Uryyb")
      "Hello"

  """
  @spec rot13(String.t()) :: String.t()
  def rot13(text) do
    encrypt(text, 13)
  end

  # ---------------------------------------------------------------------------
  # Brute Force Attack
  # ---------------------------------------------------------------------------

  @doc """
  Tries all 25 non-trivial shifts (1 through 25) and returns the results.

  Since the Caesar cipher has only 26 possible keys (shifts 0-25), we can
  simply try every one. Shift 0 is skipped because it produces the original
  ciphertext unchanged.

  Returns a list of `{shift, plaintext}` tuples, one for each shift from 1 to 25.
  A human can then scan the results to find the one that produces readable text.

  ## Computational Complexity

  This is O(25 * n) where n is the length of the text — essentially O(n).
  The tiny key space is what makes the Caesar cipher insecure. Compare this
  to AES-256, which has 2^256 possible keys — brute force would take longer
  than the age of the universe.

  ## Examples

      iex> results = CodingAdventures.CaesarCipher.brute_force("KHOOR")
      iex> length(results)
      25
      iex> Enum.find(results, fn {_s, text} -> text == "HELLO" end)
      {3, "HELLO"}

  """
  @spec brute_force(String.t()) :: [{pos_integer(), String.t()}]
  def brute_force(ciphertext) do
    # Try every shift from 1 to 25. For each shift, decrypt the ciphertext
    # (which is the same as encrypting with the negated shift).
    #
    # We skip shift 0 because decrypting with shift 0 returns the ciphertext
    # unchanged — not useful information.
    Enum.map(1..25, fn candidate_shift ->
      {candidate_shift, decrypt(ciphertext, candidate_shift)}
    end)
  end

  # ---------------------------------------------------------------------------
  # Frequency Analysis
  # ---------------------------------------------------------------------------

  @doc """
  Uses chi-squared frequency analysis to find the most likely shift.

  Instead of trying all shifts and reading each one (brute force), frequency
  analysis uses statistics to automatically identify the correct shift.

  ## How It Works

  1. For each possible shift (0-25), decrypt the ciphertext.
  2. Count how often each letter appears in the decrypted text.
  3. Compare those counts to the expected English letter frequencies using
     the **chi-squared statistic**:

         χ² = Σ (observed - expected)² / expected

  4. The shift that produces the *lowest* χ² value is the best match for
     English — and therefore the most likely original plaintext.

  ## Why Chi-Squared?

  The chi-squared test measures how far an observed distribution is from an
  expected one. A χ² of 0 means perfect match. The larger the value, the
  worse the fit. By minimizing χ², we find the decryption that looks most
  like real English.

  ## Limitations

  - Short texts may not have enough letters for reliable analysis.
  - Non-English text will produce incorrect results.
  - Texts with very unusual letter distributions may fool the analysis.

  Returns `{best_shift, plaintext}` where `best_shift` is the most likely
  encryption shift that was used, and `plaintext` is the decrypted text.

  ## Examples

      iex> ciphertext = CodingAdventures.CaesarCipher.encrypt("THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG", 3)
      iex> {shift, _plaintext} = CodingAdventures.CaesarCipher.frequency_analysis(ciphertext)
      iex> shift
      3

  """
  @spec frequency_analysis(String.t()) :: {non_neg_integer(), String.t()}
  def frequency_analysis(ciphertext) do
    # Try all 26 shifts (including 0) and score each one.
    # The shift with the lowest chi-squared score wins.
    {best_shift, _score} =
      0..25
      |> Enum.map(fn candidate_shift ->
        decrypted = decrypt(ciphertext, candidate_shift)
        score = chi_squared(decrypted)
        {candidate_shift, score}
      end)
      |> Enum.min_by(fn {_candidate_shift, score} -> score end)

    {best_shift, decrypt(ciphertext, best_shift)}
  end

  # ---------------------------------------------------------------------------
  # Private Helpers
  # ---------------------------------------------------------------------------

  # Shifts a single character by the given amount.
  #
  # Pattern matching separates three cases:
  #   1. Uppercase letter (A-Z): shift within 65..90
  #   2. Lowercase letter (a-z): shift within 97..122
  #   3. Anything else: return unchanged
  #
  # The formula `rem(char - base + amount, 26) + base` works by:
  #   - Normalizing to 0..25 (subtract the base)
  #   - Adding the shift
  #   - Wrapping with mod 26
  #   - Converting back to ASCII (add the base)

  defp shift_char(char, amount) when char in ?A..?Z do
    Integer.mod(char - ?A + amount, 26) + ?A
  end

  defp shift_char(char, amount) when char in ?a..?z do
    Integer.mod(char - ?a + amount, 26) + ?a
  end

  defp shift_char(char, _amount), do: char

  # Computes the chi-squared statistic for a piece of text against
  # English letter frequencies.
  #
  # Steps:
  #   1. Count occurrences of each letter (case-insensitive).
  #   2. For each letter A-Z, compute:
  #        observed  = count of that letter in the text
  #        expected  = english_frequency[letter] * total_letter_count
  #        χ²_term   = (observed - expected)² / expected
  #   3. Sum all 26 terms.
  #
  # A lower score means the text's letter distribution more closely
  # matches typical English.

  defp chi_squared(text) do
    # Count only alphabetic characters, converted to uppercase.
    counts =
      text
      |> String.to_charlist()
      |> Enum.filter(fn char -> char in ?A..?Z or char in ?a..?z end)
      |> Enum.map(fn char ->
        if char in ?a..?z, do: char - 32, else: char
      end)
      |> Enum.frequencies()

    total = Enum.sum(Map.values(counts))

    # If there are no letters, return a high score (worst possible match).
    if total == 0 do
      1_000_000.0
    else
      Enum.reduce(?A..?Z, 0.0, fn letter, sum_acc ->
        observed = Map.get(counts, letter, 0)
        expected_freq = Map.get(@english_frequencies, letter, 0.0)
        expected_count = expected_freq * total

        # Guard against division by zero for letters with ~0 expected frequency.
        if expected_count > 0.0 do
          sum_acc + :math.pow(observed - expected_count, 2) / expected_count
        else
          sum_acc
        end
      end)
    end
  end
end
