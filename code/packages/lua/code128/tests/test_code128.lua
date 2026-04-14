package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../barcode_layout_1d/src/?.lua;" ..
    "../../barcode_layout_1d/src/?/init.lua;" ..
    "../../paint_instructions/src/?.lua;" ..
    "../../paint_instructions/src/?/init.lua;" ..
    package.path
)

local code128 = require("coding_adventures.code128")

describe("code128 module", function()
    it("computes a known checksum", function()
        local values = {}
        for index = 1, #"Code 128" do
            values[#values + 1] = code128.value_for_code128_b_char(("Code 128"):sub(index, index))
        end
        assert.equal(64, code128.compute_code128_checksum(values))
    end)

    it("encodes start, checksum, and stop", function()
        local encoded = code128.encode_code128_b("Code 128")
        assert.equal("start", encoded[1].role)
        assert.equal("check", encoded[#encoded - 1].role)
        assert.equal("stop", encoded[#encoded].role)
    end)

    it("emits a paint scene", function()
        local scene = code128.draw_code128("Code 128")
        assert.equal("code128", scene.metadata.symbology)
        assert.equal("B", scene.metadata.code_set)
        assert.is_true(scene.width > 0)
    end)
end)
