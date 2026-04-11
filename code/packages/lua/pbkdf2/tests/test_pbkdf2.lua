-- Tests for coding_adventures.pbkdf2 — RFC 8018

package.path = "../src/?.lua;" ..
               "../src/?/init.lua;" ..
               "../../hmac/src/?.lua;" ..
               "../../hmac/src/?/init.lua;" ..
               "../../sha256/src/?.lua;" ..
               "../../sha256/src/?/init.lua;" ..
               "../../sha512/src/?.lua;" ..
               "../../sha512/src/?/init.lua;" ..
               "../../md5/src/?.lua;" ..
               "../../md5/src/?/init.lua;" ..
               "../../sha1/src/?.lua;" ..
               "../../sha1/src/?/init.lua;" ..
               package.path

local pbkdf2 = require("coding_adventures.pbkdf2")

describe("RFC 6070 PBKDF2-HMAC-SHA1", function()
  it("vector 1 — c=1, dkLen=20", function()
    local dk = pbkdf2.pbkdf2_hmac_sha1_hex("password", "salt", 1, 20)
    assert.are.equal("0c60c80f961f0e71f3a9b524af6012062fe037a6", dk)
  end)

  it("vector 2 — c=4096, dkLen=20", function()
    local dk = pbkdf2.pbkdf2_hmac_sha1_hex("password", "salt", 4096, 20)
    assert.are.equal("4b007901b765489abead49d926f721d065a429c1", dk)
  end)

  it("vector 3 — long password and salt", function()
    local dk = pbkdf2.pbkdf2_hmac_sha1_hex(
      "passwordPASSWORDpassword",
      "saltSALTsaltSALTsaltSALTsaltSALTsalt",
      4096,
      25
    )
    assert.are.equal("3d2eec4fe41c849b80c8d83662c0e44a8b291a964cf2f07038", dk)
  end)

  it("vector 4 — null bytes in password and salt", function()
    local dk = pbkdf2.pbkdf2_hmac_sha1_hex("pass\x00word", "sa\x00lt", 4096, 16)
    assert.are.equal("56fa6aa75548099dcc37d7f03425e0c3", dk)
  end)
end)

describe("RFC 7914 PBKDF2-HMAC-SHA256", function()
  it("vector 1 — c=1, dkLen=64", function()
    local dk = pbkdf2.pbkdf2_hmac_sha256_hex("passwd", "salt", 1, 64)
    local expected =
      "55ac046e56e3089fec1691c22544b605" ..
      "f94185216dde0465e68b9d57c20dacbc" ..
      "49ca9cccf179b645991664b39d77ef31" ..
      "7c71b845b1e30bd509112041d3a19783"
    assert.are.equal(expected, dk)
  end)

  it("output length matches requested key_length", function()
    local dk = pbkdf2.pbkdf2_hmac_sha256("key", "salt", 1, 32)
    assert.are.equal(32, #dk)
  end)

  it("truncation is consistent with prefix of longer key", function()
    local short = pbkdf2.pbkdf2_hmac_sha256("key", "salt", 1, 16)
    local full  = pbkdf2.pbkdf2_hmac_sha256("key", "salt", 1, 32)
    assert.are.equal(short, full:sub(1, 16))
  end)

  it("multi-block: first 32 bytes match single-block result", function()
    local dk64 = pbkdf2.pbkdf2_hmac_sha256("password", "salt", 1, 64)
    local dk32 = pbkdf2.pbkdf2_hmac_sha256("password", "salt", 1, 32)
    assert.are.equal(64, #dk64)
    assert.are.equal(dk32, dk64:sub(1, 32))
  end)
end)

describe("PBKDF2-HMAC-SHA512", function()
  it("output length", function()
    assert.are.equal(64, #pbkdf2.pbkdf2_hmac_sha512("secret", "nacl", 1, 64))
  end)

  it("truncation consistent", function()
    local short = pbkdf2.pbkdf2_hmac_sha512("secret", "nacl", 1, 32)
    local full  = pbkdf2.pbkdf2_hmac_sha512("secret", "nacl", 1, 64)
    assert.are.equal(short, full:sub(1, 32))
  end)

  it("multi-block 128 bytes", function()
    assert.are.equal(128, #pbkdf2.pbkdf2_hmac_sha512("key", "salt", 1, 128))
  end)
end)

describe("hex variants", function()
  it("SHA1 hex matches RFC 6070 vector 1", function()
    assert.are.equal(
      "0c60c80f961f0e71f3a9b524af6012062fe037a6",
      pbkdf2.pbkdf2_hmac_sha1_hex("password", "salt", 1, 20)
    )
  end)

  it("SHA256 hex matches bytes", function()
    local raw = pbkdf2.pbkdf2_hmac_sha256("passwd", "salt", 1, 32)
    local hex = pbkdf2.pbkdf2_hmac_sha256_hex("passwd", "salt", 1, 32)
    -- Convert raw bytes to hex manually for comparison.
    local expected = raw:gsub(".", function(c)
      return string.format("%02x", string.byte(c))
    end)
    assert.are.equal(expected, hex)
  end)

  it("SHA512 hex matches bytes", function()
    local raw = pbkdf2.pbkdf2_hmac_sha512("secret", "nacl", 1, 64)
    local hex = pbkdf2.pbkdf2_hmac_sha512_hex("secret", "nacl", 1, 64)
    local expected = raw:gsub(".", function(c)
      return string.format("%02x", string.byte(c))
    end)
    assert.are.equal(expected, hex)
  end)
end)

describe("validation", function()
  it("empty password raises error", function()
    assert.has_error(
      function() pbkdf2.pbkdf2_hmac_sha256("", "salt", 1, 32) end,
      "PBKDF2 password must not be empty"
    )
  end)

  it("empty password SHA1 raises error", function()
    assert.has_error(
      function() pbkdf2.pbkdf2_hmac_sha1("", "salt", 1, 20) end,
      "PBKDF2 password must not be empty"
    )
  end)

  it("zero iterations raises error", function()
    assert.has_error(
      function() pbkdf2.pbkdf2_hmac_sha256("pw", "salt", 0, 32) end,
      "PBKDF2 iterations must be a positive integer"
    )
  end)

  it("negative iterations raises error", function()
    assert.has_error(
      function() pbkdf2.pbkdf2_hmac_sha256("pw", "salt", -1, 32) end,
      "PBKDF2 iterations must be a positive integer"
    )
  end)

  it("zero key_length raises error", function()
    assert.has_error(
      function() pbkdf2.pbkdf2_hmac_sha256("pw", "salt", 1, 0) end,
      "PBKDF2 key_length must be a positive integer"
    )
  end)

  it("empty salt is allowed", function()
    assert.are.equal(32, #pbkdf2.pbkdf2_hmac_sha256("password", "", 1, 32))
  end)

  it("is deterministic", function()
    local a = pbkdf2.pbkdf2_hmac_sha256("secret", "nacl", 100, 32)
    local b = pbkdf2.pbkdf2_hmac_sha256("secret", "nacl", 100, 32)
    assert.are.equal(a, b)
  end)

  it("different salts produce different keys", function()
    local a = pbkdf2.pbkdf2_hmac_sha256("password", "salt1", 1, 32)
    local b = pbkdf2.pbkdf2_hmac_sha256("password", "salt2", 1, 32)
    assert.are_not.equal(a, b)
  end)

  it("different passwords produce different keys", function()
    local a = pbkdf2.pbkdf2_hmac_sha256("password1", "salt", 1, 32)
    local b = pbkdf2.pbkdf2_hmac_sha256("password2", "salt", 1, 32)
    assert.are_not.equal(a, b)
  end)

  it("different iterations produce different keys", function()
    local a = pbkdf2.pbkdf2_hmac_sha256("password", "salt", 1, 32)
    local b = pbkdf2.pbkdf2_hmac_sha256("password", "salt", 2, 32)
    assert.are_not.equal(a, b)
  end)
end)
