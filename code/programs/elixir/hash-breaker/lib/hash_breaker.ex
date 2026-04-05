# hash-breaker — Demonstrating why MD5 is cryptographically broken.
#
# Three attacks against MD5:
#   1. Known Collision Pairs (Wang & Yu, 2004)
#   2. Length Extension Attack (forge hash without secret)
#   3. Birthday Attack on truncated hash (birthday paradox)

defmodule HashBreaker do
  @moduledoc """
  Demonstrates three attacks showing why MD5 is cryptographically broken.
  Each attack prints educational output explaining the concept.
  """

  import Bitwise

  # ── Wang/Yu Collision Pair ────────────────────────────────────────────────
  # Two 128-byte messages that produce the SAME MD5 hash.

  @collision_a Base.decode16!(
    "D131DD02C5E6EEC4693D9A0698AFF95C" <>
    "2FCAB58712467EAB4004583EB8FB7F89" <>
    "55AD340609F4B30283E488832571415A" <>
    "085125E8F7CDC99FD91DBDF280373C5B" <>
    "D8823E3156348F5BAE6DACD436C919C6" <>
    "DD53E2B487DA03FD02396306D248CDA0" <>
    "E99F33420F577EE8CE54B67080A80D1E" <>
    "C69821BCB6A8839396F9652B6FF72A70"
  )

  @collision_b Base.decode16!(
    "D131DD02C5E6EEC4693D9A0698AFF95C" <>
    "2FCAB50712467EAB4004583EB8FB7F89" <>
    "55AD340609F4B30283E4888325F1415A" <>
    "085125E8F7CDC99FD91DBD7280373C5B" <>
    "D8823E3156348F5BAE6DACD436C919C6" <>
    "DD53E23487DA03FD02396306D248CDA0" <>
    "E99F33420F577EE8CE54B67080280D1E" <>
    "C69821BCB6A8839396F965AB6FF72A70"
  )

  # ── MD5 T-table and shifts for inline compression ─────────────────────────

  @t_table (for i <- 0..63 do
    :math.sin(i + 1) |> abs() |> Kernel.*(4_294_967_296) |> floor() |> Bitwise.band(0xFFFFFFFF)
  end)

  @shifts [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21
  ]

  @mask32 0xFFFFFFFF

  def main do
    IO.puts("")
    IO.puts("======================================================================")
    IO.puts("           MD5 HASH BREAKER — Why MD5 Is Broken")
    IO.puts("======================================================================")
    IO.puts("  Three attacks showing MD5 must NEVER be used for security:")
    IO.puts("    1. Known collision pairs (Wang & Yu, 2004)")
    IO.puts("    2. Length extension attack (forge MAC without secret)")
    IO.puts("    3. Birthday attack on truncated hash (birthday paradox)")
    IO.puts("======================================================================")
    IO.puts("")

    attack_1()
    attack_2()
    attack_3()

    IO.puts(String.duplicate("=", 72))
    IO.puts("CONCLUSION")
    IO.puts(String.duplicate("=", 72))
    IO.puts("")
    IO.puts("MD5 is broken in three distinct ways:")
    IO.puts("  1. COLLISION RESISTANCE: known pairs exist (and can be generated)")
    IO.puts("  2. LENGTH EXTENSION: Merkle-Damgard structure leaks internal state")
    IO.puts("  3. BIRTHDAY BOUND: only 2^64 (and dedicated attacks beat even that)")
    IO.puts("")
    IO.puts("Use SHA-256 or SHA-3 for security. Use HMAC (not raw hash) for MACs.")
    IO.puts("")
  end

  # ── Attack 1 ──────────────────────────────────────────────────────────────

  defp attack_1 do
    IO.puts(String.duplicate("=", 72))
    IO.puts("ATTACK 1: Known MD5 Collision Pair (Wang & Yu, 2004)")
    IO.puts(String.duplicate("=", 72))
    IO.puts("")
    IO.puts("Two different 128-byte messages that produce the SAME MD5 hash.")
    IO.puts("This was the breakthrough that proved MD5 is broken for security.")
    IO.puts("")

    IO.puts("Block A (hex):")
    IO.puts(hex_dump(@collision_a))
    IO.puts("")
    IO.puts("Block B (hex):")
    IO.puts(hex_dump(@collision_b))
    IO.puts("")

    bytes_a = :binary.bin_to_list(@collision_a)
    bytes_b = :binary.bin_to_list(@collision_b)
    diffs = Enum.zip(bytes_a, bytes_b)
      |> Enum.with_index()
      |> Enum.filter(fn {{a, b}, _i} -> a != b end)
      |> Enum.map(fn {_, i} -> i end)

    IO.puts("Blocks differ at #{length(diffs)} byte positions: #{inspect(diffs)}")
    for pos <- diffs do
      a_byte = Enum.at(bytes_a, pos)
      b_byte = Enum.at(bytes_b, pos)
      IO.puts("  Byte #{pos}: A=0x#{hex_byte(a_byte)}  B=0x#{hex_byte(b_byte)}")
    end
    IO.puts("")

    hash_a = CodingAdventures.Md5.md5_hex(@collision_a)
    hash_b = CodingAdventures.Md5.md5_hex(@collision_b)
    IO.puts("MD5(A) = #{hash_a}")
    IO.puts("MD5(B) = #{hash_b}")
    match_str = if hash_a == hash_b, do: "YES — COLLISION!", else: "No (unexpected)"
    IO.puts("Match?   #{match_str}")
    IO.puts("")
    IO.puts("Lesson: MD5 collisions are REAL. Never use MD5 for integrity or auth.")
    IO.puts("")
  end

  # ── Attack 2 ──────────────────────────────────────────────────────────────

  defp attack_2 do
    IO.puts(String.duplicate("=", 72))
    IO.puts("ATTACK 2: Length Extension Attack")
    IO.puts(String.duplicate("=", 72))
    IO.puts("")
    IO.puts("Given md5(secret + message) and len(secret + message), we can forge")
    IO.puts("md5(secret + message + padding + evil_data) WITHOUT knowing the secret!")
    IO.puts("")

    secret = "supersecretkey!!"
    message = "amount=100&to=alice"
    original_data = secret <> message
    original_hash = CodingAdventures.Md5.md5(original_data)
    original_hex = Base.encode16(original_hash, case: :lower)

    IO.puts("Secret (unknown to attacker): #{inspect(secret)}")
    IO.puts("Message:                      #{inspect(message)}")
    IO.puts("MAC = md5(secret || message): #{original_hex}")
    IO.puts("Length of (secret || message): #{byte_size(original_data)} bytes")
    IO.puts("")

    evil_data = "&amount=1000000&to=mallory"
    IO.puts("Evil data to append: #{inspect(evil_data)}")
    IO.puts("")

    # Step 1: Extract state from hash (four LE 32-bit words)
    <<a::little-32, b::little-32, c::little-32, d::little-32>> = original_hash
    IO.puts("Step 1: Extract MD5 internal state from the hash")
    IO.puts("  A = 0x#{hex_word(a)}, B = 0x#{hex_word(b)}, C = 0x#{hex_word(c)}, D = 0x#{hex_word(d)}")
    IO.puts("")

    # Step 2: Compute padding
    padding = md5_padding(byte_size(original_data))
    IO.puts("Step 2: Compute MD5 padding for the original message")
    IO.puts("  Padding (#{byte_size(padding)} bytes): #{Base.encode16(padding, case: :lower)}")
    IO.puts("")

    processed_len = byte_size(original_data) + byte_size(padding)
    IO.puts("Step 3: Total bytes processed so far: #{processed_len}")
    IO.puts("")

    # Step 4: Forge
    forged_input = evil_data <> md5_padding(processed_len + byte_size(evil_data))
    state = compress_blocks({a, b, c, d}, forged_input)
    {fa, fb, fc, fd} = state
    forged_hash = <<fa::little-32, fb::little-32, fc::little-32, fd::little-32>>
    forged_hex = Base.encode16(forged_hash, case: :lower)

    IO.puts("Step 4: Initialize hasher with extracted state, feed evil_data")
    IO.puts("  Forged hash: #{forged_hex}")
    IO.puts("")

    # Step 5: Verify
    actual_full = original_data <> padding <> evil_data
    actual_hex = CodingAdventures.Md5.md5_hex(actual_full)

    IO.puts("Step 5: Verify — compute actual md5(secret || message || padding || evil_data)")
    IO.puts("  Actual hash: #{actual_hex}")
    match_str = if forged_hex == actual_hex, do: "YES — FORGED!", else: "No (bug)"
    IO.puts("  Match?       #{match_str}")
    IO.puts("")
    IO.puts("The attacker forged a valid MAC without knowing the secret!")
    IO.puts("")
    IO.puts("Why HMAC fixes this:")
    IO.puts("  HMAC = md5(key XOR opad || md5(key XOR ipad || message))")
    IO.puts("  The outer hash prevents length extension because the attacker")
    IO.puts("  cannot extend past the outer md5() boundary.")
    IO.puts("")
  end

  # ── Attack 3 ──────────────────────────────────────────────────────────────

  defp attack_3 do
    IO.puts(String.duplicate("=", 72))
    IO.puts("ATTACK 3: Birthday Attack on Truncated MD5 (32-bit)")
    IO.puts(String.duplicate("=", 72))
    IO.puts("")
    IO.puts("The birthday paradox: with N possible hash values, expect a collision")
    IO.puts("after ~sqrt(N) random inputs. For 32-bit hash: sqrt(2^32) = 2^16 = 65536.")
    IO.puts("")

    # Use a deterministic seed for reproducibility
    :rand.seed(:exsss, {42, 42, 42})
    birthday_loop(%{}, 0)

    IO.puts("")
    IO.puts("This is a GENERIC attack — it works against any hash function.")
    IO.puts("The defense is a longer hash: SHA-256 has 2^128 birthday bound,")
    IO.puts("while MD5 has only 2^64 (and dedicated attacks are even faster).")
    IO.puts("")
  end

  defp birthday_loop(seen, attempts) do
    msg = :rand.bytes(8)
    full_hash = CodingAdventures.Md5.md5(msg)
    <<truncated::binary-4, _rest::binary>> = full_hash
    attempts = attempts + 1

    case Map.get(seen, truncated) do
      nil ->
        birthday_loop(Map.put(seen, truncated, msg), attempts)

      other_msg when other_msg != msg ->
        IO.puts("COLLISION FOUND after #{attempts} attempts!")
        IO.puts("")
        IO.puts("  Message 1: #{Base.encode16(other_msg, case: :lower)}")
        IO.puts("  Message 2: #{Base.encode16(msg, case: :lower)}")
        IO.puts("  Truncated MD5 (4 bytes): #{Base.encode16(truncated, case: :lower)}")
        IO.puts("  Full MD5 of msg1: #{CodingAdventures.Md5.md5_hex(other_msg)}")
        IO.puts("  Full MD5 of msg2: #{CodingAdventures.Md5.md5_hex(msg)}")
        IO.puts("")
        IO.puts("  Expected ~65536 attempts (2^16), got #{attempts}")
        ratio = Float.round(attempts / 65536, 2)
        IO.puts("  Ratio: #{ratio}x the theoretical expectation")

      _same_msg ->
        birthday_loop(seen, attempts)
    end
  end

  # ── Helpers ───────────────────────────────────────────────────────────────

  defp hex_dump(data) do
    data
    |> :binary.bin_to_list()
    |> Enum.chunk_every(16)
    |> Enum.map(fn row ->
      "  " <> Enum.map_join(row, "", &hex_byte/1)
    end)
    |> Enum.join("\n")
  end

  defp hex_byte(b), do: Integer.to_string(b, 16) |> String.downcase() |> String.pad_leading(2, "0")
  defp hex_word(w), do: Integer.to_string(w, 16) |> String.downcase() |> String.pad_leading(8, "0")

  defp md5_padding(message_len) do
    remainder = rem(message_len, 64)
    pad_len = rem(55 - remainder + 64, 64)
    bit_len = message_len * 8
    <<0x80>> <> :binary.copy(<<0>>, pad_len) <> <<bit_len::little-64>>
  end

  # ── Inline MD5 compression ───────────────────────────────────────────────

  defp compress_blocks(state, <<>>), do: state
  defp compress_blocks(state, <<block::binary-64, rest::binary>>) do
    compress_blocks(md5_compress(state, block), rest)
  end
  defp compress_blocks(state, _partial), do: state

  defp md5_compress({a0, b0, c0, d0}, block) do
    words = parse_words(block)
    {a, b, c, d} = run_rounds(0, a0, b0, c0, d0, words)
    {
      band(a0 + a, @mask32),
      band(b0 + b, @mask32),
      band(c0 + c, @mask32),
      band(d0 + d, @mask32)
    }
  end

  defp parse_words(<<>>), do: []
  defp parse_words(<<w::little-32, rest::binary>>), do: [w | parse_words(rest)]

  defp run_rounds(64, a, b, c, d, _words), do: {a, b, c, d}
  defp run_rounds(i, a, b, c, d, words) do
    {f, g} = cond do
      i < 16 -> {band(bor(band(b, c), band(bnot(b), d)), @mask32), i}
      i < 32 -> {band(bor(band(d, b), band(bnot(d), c)), @mask32), rem(5 * i + 1, 16)}
      i < 48 -> {band(bxor(bxor(b, c), d), @mask32), rem(3 * i + 5, 16)}
      true   -> {band(bxor(c, bor(b, band(bnot(d), @mask32))), @mask32), rem(7 * i, 16)}
    end

    t_val = Enum.at(@t_table, i)
    s_val = Enum.at(@shifts, i)
    m_val = Enum.at(words, g)
    sum = band(a + f + t_val + m_val, @mask32)
    rotated = rotl32(sum, s_val)
    new_b = band(b + rotated, @mask32)

    run_rounds(i + 1, d, new_b, b, c, words)
  end

  defp rotl32(x, n) do
    band(bor(bsl(x, n), bsr(x, 32 - n)), @mask32)
  end
end

HashBreaker.main()
