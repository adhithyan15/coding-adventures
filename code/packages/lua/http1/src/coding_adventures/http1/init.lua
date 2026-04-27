-- http1 — HTTP/1 request and response head parser with body framing detection
--
-- This module is part of the coding-adventures project, an educational
-- computing stack built from logic gates up through interpreters.
----
-- Usage:
--
--   local m = require("coding_adventures.http1")
--
-- ============================================================================

local http_core = require("coding_adventures.http_core")

local M = {}

M.VERSION = "0.1.0"

local function split_head_lines(input)
    local index = 1

    while true do
        if input:sub(index, index + 1) == "\r\n" then
            index = index + 2
        elseif input:sub(index, index) == "\n" then
            index = index + 1
        else
            break
        end
    end

    local lines = {}
    while true do
        if index > #input then
            error("incomplete HTTP/1 head")
        end

        local line_end = input:find("\n", index, true)
        if not line_end then
            error("incomplete HTTP/1 head")
        end

        local line = input:sub(index, line_end - 1):gsub("\r$", "")
        index = line_end + 1

        if line == "" then
            return lines, index
        end

        lines[#lines + 1] = line
    end
end

local function parse_headers(lines)
    local headers = {}

    for _, line in ipairs(lines) do
        local separator = line:find(":", 1, true)
        if not separator or separator == 1 then
            error("invalid HTTP/1 header: " .. line)
        end

        headers[#headers + 1] = http_core.header(
            line:sub(1, separator - 1):gsub("^%s+", ""):gsub("%s+$", ""),
            line:sub(separator + 1):gsub("^%s+", ""):gsub("%s+$", "")
        )
    end

    return headers
end

local function chunked_transfer_encoding(headers)
    for _, header in ipairs(headers) do
        if string.lower(header.name) == "transfer-encoding" then
            for piece in header.value:gmatch("([^,]+)") do
                if string.lower((piece:gsub("^%s+", ""):gsub("%s+$", ""))) == "chunked" then
                    return true
                end
            end
        end
    end

    return false
end

local function declared_content_length(headers)
    local value = http_core.find_header(headers, "Content-Length")
    if not value then
        return nil
    end
    if not value:match("^%d+$") then
        error("invalid Content-Length: " .. value)
    end
    return tonumber(value)
end

local function request_body_kind(headers)
    if chunked_transfer_encoding(headers) then
        return http_core.body_chunked()
    end

    local length = declared_content_length(headers)
    if not length or length == 0 then
        return http_core.body_none()
    end

    return http_core.body_content_length(length)
end

local function response_body_kind(status, headers)
    if (status >= 100 and status < 200) or status == 204 or status == 304 then
        return http_core.body_none()
    end
    if chunked_transfer_encoding(headers) then
        return http_core.body_chunked()
    end

    local length = declared_content_length(headers)
    if not length then
        return http_core.body_until_eof()
    end
    if length == 0 then
        return http_core.body_none()
    end

    return http_core.body_content_length(length)
end

function M.parse_request_head(input)
    local lines, body_offset = split_head_lines(input)
    if #lines == 0 then
        error("invalid HTTP/1 start line")
    end

    local parts = {}
    for part in lines[1]:gmatch("%S+") do
        parts[#parts + 1] = part
    end
    if #parts ~= 3 then
        error("invalid HTTP/1 start line: " .. lines[1])
    end

    local headers = parse_headers({ table.unpack(lines, 2) })
    return {
        head = http_core.request_head(parts[1], parts[2], http_core.parse_http_version(parts[3]), headers),
        body_offset = body_offset,
        body_kind = request_body_kind(headers),
    }
end

function M.parse_response_head(input)
    local lines, body_offset = split_head_lines(input)
    if #lines == 0 then
        error("invalid HTTP/1 status line")
    end

    local parts = {}
    for part in lines[1]:gmatch("%S+") do
        parts[#parts + 1] = part
    end
    if #parts < 2 then
        error("invalid HTTP/1 status line: " .. lines[1])
    end
    if not parts[2]:match("^%d+$") then
        error("invalid HTTP status: " .. parts[2])
    end

    local headers = parse_headers({ table.unpack(lines, 2) })
    return {
        head = http_core.response_head(
            http_core.parse_http_version(parts[1]),
            tonumber(parts[2]),
            table.concat(parts, " ", 3),
            headers
        ),
        body_offset = body_offset,
        body_kind = response_body_kind(tonumber(parts[2]), headers),
    }
end

return M
