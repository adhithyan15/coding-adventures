-- Test suite for cli_builder
local Errors = require("coding_adventures.cli_builder.errors")

describe("cli_builder errors", function()
    it("should create spec error", function()
        local err = Errors.SpecError("test error")
        assert.are.equal("spec_error", err.type)
        assert.are.equal("test error", err.message)
    end)
end)
