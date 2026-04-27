local M = {}

M.VERSION = "0.1.0"

function M.header(name, value)
    return { name = name, value = value }
end

function M.http_version(major, minor)
    return { major = major, minor = minor }
end

function M.parse_http_version(text)
    local major, minor = text:match("^HTTP/(%d+)%.(%d+)$")
    if not major then
        error("invalid HTTP version: " .. text)
    end
    return M.http_version(tonumber(major), tonumber(minor))
end

function M.http_version_to_string(version)
    return string.format("HTTP/%d.%d", version.major, version.minor)
end

function M.body_kind(mode, length)
    return { mode = mode, length = length }
end

function M.body_none()
    return M.body_kind("none", nil)
end

function M.body_content_length(length)
    return M.body_kind("content-length", length)
end

function M.body_until_eof()
    return M.body_kind("until-eof", nil)
end

function M.body_chunked()
    return M.body_kind("chunked", nil)
end

function M.request_head(method, target, version, headers)
    return {
        method = method,
        target = target,
        version = version,
        headers = headers,
    }
end

function M.response_head(version, status, reason, headers)
    return {
        version = version,
        status = status,
        reason = reason,
        headers = headers,
    }
end

function M.find_header(headers, name)
    local lowered = string.lower(name)
    for _, header in ipairs(headers) do
        if string.lower(header.name) == lowered then
            return header.value
        end
    end
    return nil
end

function M.parse_content_length(headers)
    local value = M.find_header(headers, "Content-Length")
    if not value or not value:match("^%d+$") then
        return nil
    end
    return tonumber(value)
end

function M.parse_content_type(headers)
    local value = M.find_header(headers, "Content-Type")
    if not value then
        return nil
    end

    local parts = {}
    for piece in string.gmatch(value, "([^;]+)") do
        parts[#parts + 1] = (piece:gsub("^%s+", ""):gsub("%s+$", ""))
    end

    local media_type = parts[1]
    if not media_type or media_type == "" then
        return nil
    end

    local charset = nil
    for index = 2, #parts do
        local key, raw_value = parts[index]:match("^([^=]+)=(.+)$")
        if key and string.lower((key:gsub("^%s+", ""):gsub("%s+$", ""))) == "charset" then
            charset = raw_value:gsub("^%s+", ""):gsub("%s+$", ""):gsub('^"', ""):gsub('"$', "")
            break
        end
    end

    return media_type, charset
end

return M
