# frozen_string_literal: true

require "json"
require "minitest/autorun"
require_relative "../lib/coding_adventures_chacha20_poly1305"

class TestSE04SharedFixture < Minitest::Test
  CC = CodingAdventures::Chacha20Poly1305
  FIXTURE_PATH = File.expand_path(
    "../../../../specs/fixtures/se04-xchacha20-poly1305-v1/cases.json",
    __dir__
  )
  FIXTURE = JSON.parse(File.binread(FIXTURE_PATH))

  def from_hex(value)
    [value].pack("H*").b
  end

  def test_closed_metadata
    assert_equal 1, FIXTURE.fetch("schema_version")
    assert_equal "se04-xchacha20-poly1305-v1", FIXTURE.fetch("profile")
    assert_equal "authentication_failed", FIXTURE.fetch("authentication_failure")
    assert_equal 1, FIXTURE.fetch("hchacha20_cases").length
    assert_equal 2, FIXTURE.fetch("xchacha20_cases").length
    assert_equal 3, FIXTURE.fetch("aead_cases").length
    assert_equal 5, FIXTURE.fetch("mutations").length
  end

  def test_hchacha20_cases
    FIXTURE.fetch("hchacha20_cases").each do |test_case|
      assert_equal(
        from_hex(test_case.fetch("subkey_hex")),
        CC.hchacha20_subkey(
          from_hex(test_case.fetch("key_hex")),
          from_hex(test_case.fetch("nonce_hex"))
        ),
        test_case.fetch("id")
      )
    end
  end

  def test_raw_xchacha20_cases
    FIXTURE.fetch("xchacha20_cases").each do |test_case|
      input = from_hex(test_case.fetch("input_hex"))
      key = from_hex(test_case.fetch("key_hex"))
      nonce = from_hex(test_case.fetch("nonce_hex"))
      counter = test_case.fetch("counter")
      output = CC.xchacha20_encrypt(input, key, nonce, counter)

      assert_equal from_hex(test_case.fetch("output_hex")), output, test_case.fetch("id")
      assert_equal input, CC.xchacha20_encrypt(output, key, nonce, counter), test_case.fetch("id")
    end
  end

  def test_aead_cases_encrypt_and_decrypt_byte_identically
    FIXTURE.fetch("aead_cases").each do |test_case|
      key = from_hex(test_case.fetch("key_hex"))
      nonce = from_hex(test_case.fetch("nonce_hex"))
      aad = from_hex(test_case.fetch("aad_hex"))
      plaintext = from_hex(test_case.fetch("plaintext_hex"))
      expected_ciphertext = from_hex(test_case.fetch("ciphertext_hex"))
      expected_tag = from_hex(test_case.fetch("tag_hex"))

      assert_equal(
        [expected_ciphertext, expected_tag],
        CC.xchacha20_poly1305_encrypt(plaintext, key, nonce, aad),
        test_case.fetch("id")
      )
      assert_equal(
        plaintext,
        CC.xchacha20_poly1305_decrypt(
          expected_ciphertext,
          key,
          nonce,
          aad,
          expected_tag
        ),
        test_case.fetch("id")
      )
    end
  end

  def test_mutations_have_one_authentication_failure
    cases = FIXTURE.fetch("aead_cases").to_h { |test_case| [test_case.fetch("id"), test_case] }

    FIXTURE.fetch("mutations").each do |mutation|
      source = cases.fetch(mutation.fetch("source_case"))
      originals = {
        "ciphertext" => from_hex(source.fetch("ciphertext_hex")),
        "key" => from_hex(source.fetch("key_hex")),
        "nonce" => from_hex(source.fetch("nonce_hex")),
        "aad" => from_hex(source.fetch("aad_hex")),
        "tag" => from_hex(source.fetch("tag_hex"))
      }

      mutation.fetch("byte_indices").each do |byte_index|
        changed = originals.transform_values(&:dup)
        target = mutation.fetch("target")
        xor_byte = mutation.fetch("xor_hex").to_i(16)
        changed.fetch(target).setbyte(byte_index, changed.fetch(target).getbyte(byte_index) ^ xor_byte)

        error = assert_raises(RuntimeError) do
          CC.xchacha20_poly1305_decrypt(
            changed.fetch("ciphertext"),
            changed.fetch("key"),
            changed.fetch("nonce"),
            changed.fetch("aad"),
            changed.fetch("tag")
          )
        end
        assert_equal "Authentication failed: tag mismatch", error.message
      end
    end
  end
end
