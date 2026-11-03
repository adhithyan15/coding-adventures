-- Tests for the build-tool command-line contract.
--
-- Argument parsing is intentionally process-free so malformed input can be
-- proven to fail before discovery opens a repository path.

package.path = "../lib/?.lua;" .. "../lib/?/init.lua;" .. package.path

local Cli = require("build_tool.cli")

describe("CLI argument validation", function()
    it("parses the supported options", function()
        local options, diagnostic = Cli.parse({
            "--root", "C:\\repo root",
            "--dry-run",
            "--language", "lua",
            "--force",
            "--validate-build-files",
        })

        assert.is_nil(diagnostic)
        assert.are.same({
            root = "C:\\repo root",
            dry_run = true,
            language = "lua",
            force = true,
            validate_build_files = true,
            help = false,
        }, options)
    end)

    for _, option in ipairs({"--root", "--language"}) do
        it("rejects a missing value for " .. option, function()
            local options, diagnostic = Cli.parse({option})

            assert.is_nil(options)
            assert.are.equal(
                "CLI_ARGUMENT_ERROR: " .. option .. " requires a value",
                diagnostic
            )
        end)

        it("does not consume another option as the value for " .. option, function()
            local options, diagnostic = Cli.parse({option, "--dry-run"})

            assert.is_nil(options)
            assert.are.equal(
                "CLI_ARGUMENT_ERROR: " .. option .. " requires a value",
                diagnostic
            )
        end)

        it("rejects an empty value for " .. option, function()
            local options, diagnostic = Cli.parse({option, ""})

            assert.is_nil(options)
            assert.are.equal(
                "CLI_ARGUMENT_ERROR: " .. option .. " requires a value",
                diagnostic
            )
        end)
    end

    it("accepts a single-hyphen root value", function()
        local options, diagnostic = Cli.parse({"--root", "-checkout"})

        assert.is_nil(diagnostic)
        assert.are.equal("-checkout", options.root)
    end)

    it("rejects unknown options without echoing their spelling", function()
        local hostile = "--config=/private/checkout/customer-name"
        local options, diagnostic = Cli.parse({hostile})

        assert.is_nil(options)
        assert.are.equal("CLI_ARGUMENT_ERROR: unknown option", diagnostic)
        assert.is_nil(diagnostic:find(hostile, 1, true))
        assert.is_nil(diagnostic:find("customer-name", 1, true))
    end)

    it("rejects positional arguments without echoing their spelling", function()
        local hostile = "/private/checkout/customer-name"
        local options, diagnostic = Cli.parse({hostile})

        assert.is_nil(options)
        assert.are.equal("CLI_ARGUMENT_ERROR: unexpected positional argument", diagnostic)
        assert.is_nil(diagnostic:find(hostile, 1, true))
        assert.is_nil(diagnostic:find("customer-name", 1, true))
    end)

    it("does not let help bypass validation of later options", function()
        local options, diagnostic = Cli.parse({"--help", "--unknown"})

        assert.is_nil(options)
        assert.are.equal("CLI_ARGUMENT_ERROR: unknown option", diagnostic)
    end)
end)
