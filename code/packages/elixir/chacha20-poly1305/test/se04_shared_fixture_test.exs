defmodule CodingAdventures.ChaCha20Poly1305SharedFixtureTest do
  use ExUnit.Case, async: true
  import Bitwise

  alias CodingAdventures.ChaCha20Poly1305, as: CC

  @fixture_path Path.expand(
                  "../../../../specs/fixtures/se04-xchacha20-poly1305-v1/cases.json",
                  __DIR__
                )
  @fixture @fixture_path |> File.read!() |> Jason.decode!()

  defp from_hex(value), do: Base.decode16!(value, case: :lower)

  defp flip_byte(value, index, xor_byte) do
    <<prefix::binary-size(index), byte, suffix::binary>> = value
    prefix <> <<bxor(byte, xor_byte)>> <> suffix
  end

  test "closed v1 fixture metadata" do
    assert @fixture["schema_version"] == 1
    assert @fixture["profile"] == "se04-xchacha20-poly1305-v1"
    assert @fixture["authentication_failure"] == "authentication_failed"
    assert length(@fixture["hchacha20_cases"]) == 1
    assert length(@fixture["xchacha20_cases"]) == 2
    assert length(@fixture["aead_cases"]) == 3
    assert length(@fixture["mutations"]) == 5
  end

  test "reproduces every HChaCha20 case" do
    for test_case <- @fixture["hchacha20_cases"] do
      assert CC.hchacha20_subkey(
               from_hex(test_case["key_hex"]),
               from_hex(test_case["nonce_hex"])
             ) == from_hex(test_case["subkey_hex"]),
             test_case["id"]
    end
  end

  test "reproduces and reverses every raw XChaCha20 case" do
    for test_case <- @fixture["xchacha20_cases"] do
      input = from_hex(test_case["input_hex"])
      key = from_hex(test_case["key_hex"])
      nonce = from_hex(test_case["nonce_hex"])
      counter = test_case["counter"]
      output = CC.xchacha20_encrypt(input, key, nonce, counter)

      assert output == from_hex(test_case["output_hex"]), test_case["id"]
      assert CC.xchacha20_encrypt(output, key, nonce, counter) == input, test_case["id"]
    end
  end

  test "encrypts and decrypts every AEAD case byte-identically" do
    for test_case <- @fixture["aead_cases"] do
      key = from_hex(test_case["key_hex"])
      nonce = from_hex(test_case["nonce_hex"])
      aad = from_hex(test_case["aad_hex"])
      plaintext = from_hex(test_case["plaintext_hex"])
      ciphertext = from_hex(test_case["ciphertext_hex"])
      tag = from_hex(test_case["tag_hex"])

      assert CC.xchacha20_poly1305_encrypt(plaintext, key, nonce, aad) ==
               {ciphertext, tag},
             test_case["id"]

      assert CC.xchacha20_poly1305_decrypt(ciphertext, key, nonce, aad, tag) ==
               {:ok, plaintext},
             test_case["id"]
    end
  end

  test "maps every mutation to one authentication failure" do
    cases = Map.new(@fixture["aead_cases"], &{&1["id"], &1})

    for mutation <- @fixture["mutations"], byte_index <- mutation["byte_indices"] do
      source = Map.fetch!(cases, mutation["source_case"])

      values = %{
        "ciphertext" => from_hex(source["ciphertext_hex"]),
        "key" => from_hex(source["key_hex"]),
        "nonce" => from_hex(source["nonce_hex"]),
        "aad" => from_hex(source["aad_hex"]),
        "tag" => from_hex(source["tag_hex"])
      }

      changed =
        Map.update!(values, mutation["target"], fn value ->
          flip_byte(value, byte_index, mutation["xor_hex"] |> from_hex() |> :binary.first())
        end)

      assert CC.xchacha20_poly1305_decrypt(
               changed["ciphertext"],
               changed["key"],
               changed["nonce"],
               changed["aad"],
               changed["tag"]
             ) == {:error, :authentication_failed},
             "#{mutation["target"]} byte #{byte_index}"
    end
  end
end
