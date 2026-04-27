-- Tests for http1

local m = require("coding_adventures.http1")

describe("http1", function()
    it("has a VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    it("parses a simple request", function()
        local parsed = m.parse_request_head("GET / HTTP/1.0\r\nHost: example.com\r\n\r\n")
        assert.equals("GET", parsed.head.method)
        assert.equals("/", parsed.head.target)
        assert.same({ mode = "none", length = nil }, parsed.body_kind)
    end)

    it("parses request content length framing", function()
        local parsed = m.parse_request_head("POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello")
        assert.same({ mode = "content-length", length = 5 }, parsed.body_kind)
    end)

    it("parses a response head", function()
        local parsed = m.parse_response_head("HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody")
        assert.equals(200, parsed.head.status)
        assert.equals("OK", parsed.head.reason)
        assert.same({ mode = "content-length", length = 4 }, parsed.body_kind)
    end)

    it("uses until eof without content length", function()
        local parsed = m.parse_response_head("HTTP/1.0 200 OK\r\nServer: Venture\r\n\r\n")
        assert.same({ mode = "until-eof", length = nil }, parsed.body_kind)
    end)

    it("treats bodyless statuses as bodyless", function()
        local parsed = m.parse_response_head("HTTP/1.1 204 No Content\r\nContent-Length: 12\r\n\r\n")
        assert.same({ mode = "none", length = nil }, parsed.body_kind)
    end)

    it("accepts lf-only input and preserves duplicate headers", function()
        local parsed = m.parse_response_head("\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload")
        assert.same({ "a=1", "b=2" }, { parsed.head.headers[1].value, parsed.head.headers[2].value })
    end)

    it("rejects invalid headers", function()
        local ok, err = pcall(function()
            m.parse_request_head("GET / HTTP/1.1\r\nHost example.com\r\n\r\n")
        end)

        assert.is_false(ok)
        assert.is_truthy(string.match(err, "invalid HTTP/1 header:"))
    end)

    it("rejects invalid content length", function()
        local ok, err = pcall(function()
            m.parse_response_head("HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n")
        end)

        assert.is_false(ok)
        assert.is_truthy(string.match(err, "invalid Content%-Length:"))
    end)
end)
