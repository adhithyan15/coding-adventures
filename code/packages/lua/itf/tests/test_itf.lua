package.path = (
    "../src/?.lua;" ..
    "../src/?/init.lua;" ..
    "../../barcode_layout_1d/src/?.lua;" ..
    "../../barcode_layout_1d/src/?/init.lua;" ..
    "../../paint_instructions/src/?.lua;" ..
    "../../paint_instructions/src/?/init.lua;" ..
    package.path
)

local itf = require("coding_adventures.itf")

describe("itf module", function()
    it("rejects odd-length input", function()
        assert.has_error(function()
            itf.normalize_itf("12345")
        end)
    end)

    it("encodes digit pairs", function()
        local encoded = itf.encode_itf("123456")
        assert.equal(3, #encoded)
        assert.equal("12", encoded[1].pair)
    end)

    it("emits start and stop runs", function()
        local runs = itf.expand_itf_runs("123456")
        assert.equal("start", runs[1].role)
        assert.equal("stop", runs[#runs].role)
    end)
end)
