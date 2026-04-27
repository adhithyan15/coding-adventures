local http_core = require("coding_adventures.http_core")

describe("http_core", function()
    it("parses and renders versions", function()
        local version = http_core.parse_http_version("HTTP/1.1")
        assert.are.equal(1, version.major)
        assert.are.equal(1, version.minor)
        assert.are.equal("HTTP/1.1", http_core.http_version_to_string(version))
    end)

    it("looks up headers case insensitively", function()
        local headers = { http_core.header("Content-Type", "text/plain") }
        assert.are.equal("text/plain", http_core.find_header(headers, "content-type"))
    end)

    it("parses content helpers", function()
        local headers = {
            http_core.header("Content-Length", "42"),
            http_core.header("Content-Type", "text/html; charset=utf-8"),
        }

        assert.are.equal(42, http_core.parse_content_length(headers))

        local media_type, charset = http_core.parse_content_type(headers)
        assert.are.equal("text/html", media_type)
        assert.are.equal("utf-8", charset)
    end)

    it("constructs body kinds", function()
        assert.are.same({ mode = "none", length = nil }, http_core.body_none())
        assert.are.same({ mode = "content-length", length = 7 }, http_core.body_content_length(7))
        assert.are.same({ mode = "until-eof", length = nil }, http_core.body_until_eof())
        assert.are.same({ mode = "chunked", length = nil }, http_core.body_chunked())
    end)
end)
