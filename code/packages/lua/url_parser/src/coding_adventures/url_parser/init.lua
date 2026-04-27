-- url-parser -- RFC 1738 URL parser with relative resolution and percent-encoding
--
-- This module is part of the coding-adventures project, an educational
-- computing stack built from logic gates up through interpreters.
--
-- A URL (Uniform Resource Locator) is a string that identifies a resource on
-- the internet. It has a well-defined structure:
--
--   scheme://userinfo@host:port/path?query#fragment
--
-- For example:
--   https://user:pass@example.com:8080/path/to/page?q=1&lang=en#section2
--
-- This parser decomposes a URL string into its constituent parts using a
-- single left-to-right pass, handling edge cases like IPv6 addresses,
-- percent-encoded characters, and relative URL resolution.
--
-- Usage:
--   local url_parser = require("coding_adventures.url_parser")
--   local url = url_parser.parse("https://example.com/path?q=1#frag")
--   print(url.scheme)   -- "https"
--   print(url.host)     -- "example.com"
--   print(url.path)     -- "/path"
--   print(url.query)    -- "q=1"
--   print(url.fragment) -- "frag"
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Default Ports
-- ============================================================================
-- Each well-known scheme has a default port number. When no explicit port is
-- given in the URL, these defaults apply. This table maps scheme names
-- (lowercased) to their default port numbers.

local DEFAULT_PORTS = {
    http  = 80,
    https = 443,
    ftp   = 21,
}

-- ============================================================================
-- Scheme Validation
-- ============================================================================
-- A valid URL scheme must:
--   1. Start with a letter (a-z)
--   2. Contain only letters, digits, '+', '-', or '.'
--
-- The regex pattern ^[a-z][a-z0-9+.-]*$ captures this rule. We use Lua's
-- string.match with character classes to validate.

local function is_valid_scheme(s)
    -- A scheme must start with a letter, followed by letters, digits, +, -, .
    -- Lua patterns: %a = letter, %w = alphanumeric
    -- We need to also allow +, -, . after the first character.
    if #s == 0 then return false end
    if not string.match(string.sub(s, 1, 1), "^%a$") then return false end
    if #s > 1 then
        -- Check remaining characters: each must be letter, digit, +, -, or .
        for i = 2, #s do
            local c = string.sub(s, i, i)
            if not string.match(c, "^[%a%d%+%.%-]$") then
                return false
            end
        end
    end
    return true
end

-- ============================================================================
-- Percent Encoding
-- ============================================================================
-- Percent-encoding replaces unsafe characters with %XX where XX is the
-- uppercase hex representation of the byte value. For example:
--   space (0x20) -> %20
--   @ (0x40)    -> %40
--
-- The "unreserved" characters that do NOT need encoding are:
--   A-Z a-z 0-9 - _ . ~ /
--
-- Truth table for "should this character be encoded?":
--   Character  | In unreserved set? | Encode?
--   -----------|-------------------|--------
--   'A'        | Yes               | No
--   ' '        | No                | Yes
--   '/'        | Yes (we keep it)  | No
--   '@'        | No                | Yes

function M.percent_encode(input)
    -- Walk each byte of the input string. If it's in the unreserved set,
    -- keep it as-is. Otherwise, replace it with %XX.
    local result = {}
    for i = 1, #input do
        local byte = string.byte(input, i)
        local c = string.char(byte)
        -- Unreserved characters: A-Z, a-z, 0-9, -, _, ., ~, /
        if string.match(c, "^[%a%d%-%._~/]$") then
            result[#result + 1] = c
        else
            result[#result + 1] = string.format("%%%02X", byte)
        end
    end
    return table.concat(result)
end

-- ============================================================================
-- Percent Decoding
-- ============================================================================
-- The inverse of percent-encoding: replace %XX sequences with the
-- corresponding byte. For example:
--   %20 -> space
--   %41 -> 'A'
--
-- Invalid sequences (e.g., %GG or truncated %2) produce an error.

function M.percent_decode(input)
    local result = {}
    local i = 1
    while i <= #input do
        local c = string.sub(input, i, i)
        if c == "%" then
            -- We need at least 2 more characters for the hex digits
            if i + 2 > #input then
                return nil, "invalid_percent_encoding"
            end
            local hex = string.sub(input, i + 1, i + 2)
            -- Validate that both characters are valid hex digits
            if not string.match(hex, "^%x%x$") then
                return nil, "invalid_percent_encoding"
            end
            result[#result + 1] = string.char(tonumber(hex, 16))
            i = i + 3
        else
            result[#result + 1] = c
            i = i + 1
        end
    end
    return table.concat(result), nil
end

-- ============================================================================
-- Parse: The Core Algorithm
-- ============================================================================
-- Parsing a URL is done in a single left-to-right pass. The algorithm:
--
--   1. Look for "://" to split scheme from the rest (authority-based URL).
--      If not found, try "scheme:path" form (e.g., mailto:user@example.com).
--      If neither found, return error "missing_scheme".
--
--   2. Extract fragment (everything after '#').
--
--   3. Extract query (everything after '?').
--
--   4. Extract path (everything from first '/').
--
--   5. In the authority portion, look for '@' to extract userinfo.
--
--   6. Parse the host:port. Handle IPv6 brackets [::1], and for IPv4/hostnames
--      split on the last ':' if the suffix is all digits.
--
--   7. Lowercase the host and scheme.
--
-- Visual diagram of URL structure:
--
--   https://user:pass@example.com:8080/path/to/page?query=1#fragment
--   \___/   \_______/ \_________/ \__/\____________/\_______/\______/
--   scheme   userinfo    host     port    path       query   fragment

function M.parse(input)
    if type(input) ~= "string" or #input == 0 then
        return nil, "missing_scheme"
    end

    local raw = input
    local scheme, rest

    -- Step 1: Find the scheme.
    -- Look for "://" which indicates an authority-based URL.
    local scheme_end = string.find(input, "://", 1, true)
    if scheme_end then
        scheme = string.lower(string.sub(input, 1, scheme_end - 1))
        if not is_valid_scheme(scheme) then
            return nil, "invalid_scheme"
        end
        rest = string.sub(input, scheme_end + 3)
    else
        -- Try "scheme:path" form (e.g., mailto:foo@bar.com)
        local colon = string.find(input, ":", 1, true)
        if colon and colon > 1 then
            local candidate = string.lower(string.sub(input, 1, colon - 1))
            if is_valid_scheme(candidate) then
                scheme = candidate
                local path_part = string.sub(input, colon + 1)

                -- For scheme:path URLs, parse fragment and query from path
                local fragment_val = nil
                local query_val = nil

                local hash_pos = string.find(path_part, "#", 1, true)
                if hash_pos then
                    fragment_val = string.sub(path_part, hash_pos + 1)
                    path_part = string.sub(path_part, 1, hash_pos - 1)
                end

                local q_pos = string.find(path_part, "?", 1, true)
                if q_pos then
                    query_val = string.sub(path_part, q_pos + 1)
                    path_part = string.sub(path_part, 1, q_pos - 1)
                end

                return {
                    scheme   = scheme,
                    userinfo = nil,
                    host     = nil,
                    port     = nil,
                    path     = path_part,
                    query    = query_val,
                    fragment = fragment_val,
                    raw      = raw,
                }, nil
            end
        end
        return nil, "missing_scheme"
    end

    -- Step 2: Extract fragment (everything after the first '#' in the rest).
    local fragment = nil
    local hash_pos = string.find(rest, "#", 1, true)
    if hash_pos then
        fragment = string.sub(rest, hash_pos + 1)
        rest = string.sub(rest, 1, hash_pos - 1)
    end

    -- Step 3: Extract query (everything after the first '?').
    local query = nil
    local q_pos = string.find(rest, "?", 1, true)
    if q_pos then
        query = string.sub(rest, q_pos + 1)
        rest = string.sub(rest, 1, q_pos - 1)
    end

    -- Step 4: Extract path (everything from the first '/').
    local path = "/"
    local slash_pos = string.find(rest, "/", 1, true)
    if slash_pos then
        path = string.sub(rest, slash_pos)
        rest = string.sub(rest, 1, slash_pos - 1)
    end

    -- Now 'rest' is the authority: [userinfo@]host[:port]
    local authority = rest

    -- Step 5: Extract userinfo (everything before '@').
    local userinfo = nil
    local at_pos = string.find(authority, "@", 1, true)
    if at_pos then
        userinfo = string.sub(authority, 1, at_pos - 1)
        authority = string.sub(authority, at_pos + 1)
    end

    -- Step 6: Parse host and port from the remaining authority string.
    local host = nil
    local port = nil

    if #authority > 0 then
        -- Check for IPv6: starts with '['
        if string.sub(authority, 1, 1) == "[" then
            -- IPv6 address: find the closing ']'
            local bracket_end = string.find(authority, "]", 2, true)
            if bracket_end then
                host = string.sub(authority, 1, bracket_end)
                local after_bracket = string.sub(authority, bracket_end + 1)
                -- Check for port after ']:'
                if string.sub(after_bracket, 1, 1) == ":" then
                    local port_str = string.sub(after_bracket, 2)
                    if #port_str > 0 and string.match(port_str, "^%d+$") then
                        port = tonumber(port_str)
                    end
                end
            else
                -- Malformed IPv6, just use as host
                host = authority
            end
        else
            -- IPv4 or hostname: split on the LAST colon, but only if
            -- everything after the colon is digits (a port number).
            --
            -- We search for the last ':' because hostnames can't contain colons
            -- but IPv6 can (handled above). The port is only valid if it's
            -- a sequence of digits.
            local last_colon = nil
            for i = #authority, 1, -1 do
                if string.sub(authority, i, i) == ":" then
                    last_colon = i
                    break
                end
            end
            if last_colon then
                local port_str = string.sub(authority, last_colon + 1)
                if #port_str > 0 and string.match(port_str, "^%d+$") then
                    host = string.sub(authority, 1, last_colon - 1)
                    port = tonumber(port_str)
                else
                    host = authority
                end
            else
                host = authority
            end
        end

        -- Lowercase the host
        if host then
            host = string.lower(host)
        end

        -- Empty host becomes nil
        if host and #host == 0 then
            host = nil
        end
    end

    return {
        scheme   = scheme,
        userinfo = userinfo,
        host     = host,
        port     = port,
        path     = path,
        query    = query,
        fragment = fragment,
        raw      = raw,
    }, nil
end

-- ============================================================================
-- Effective Port
-- ============================================================================
-- Returns the port to actually use for a connection. If the URL has an
-- explicit port, use that. Otherwise, look up the default for the scheme.
--
--   URL                         | effective_port
--   ----------------------------|---------------
--   http://example.com          | 80
--   http://example.com:9090     | 9090
--   https://example.com         | 443
--   ftp://files.example.com     | 21
--   custom://example.com        | nil

function M.effective_port(url)
    if url.port then
        return url.port
    end
    if url.scheme then
        return DEFAULT_PORTS[url.scheme]
    end
    return nil
end

-- ============================================================================
-- Authority
-- ============================================================================
-- Reconstructs the authority portion of a URL from its components:
--   [userinfo@]host[:port]
--
-- Examples:
--   {host="example.com"}                    -> "example.com"
--   {host="example.com", port=8080}         -> "example.com:8080"
--   {userinfo="user", host="example.com"}   -> "user@example.com"

function M.authority(url)
    local parts = {}
    if url.userinfo then
        parts[#parts + 1] = url.userinfo
        parts[#parts + 1] = "@"
    end
    if url.host then
        parts[#parts + 1] = url.host
    end
    if url.port then
        parts[#parts + 1] = ":"
        parts[#parts + 1] = tostring(url.port)
    end
    return table.concat(parts)
end

-- ============================================================================
-- URL to String
-- ============================================================================
-- Reconstructs a full URL string from a parsed url_table. This is the inverse
-- of parse(): it reassembles scheme, authority, path, query, and fragment
-- back into a single string.
--
-- The reassembly follows this pattern:
--   scheme://authority/path?query#fragment

function M.to_url_string(url)
    local parts = {}
    if url.scheme then
        parts[#parts + 1] = url.scheme
        -- If there's a host, it's an authority-based URL (scheme://...)
        -- If not, it's a scheme:path URL (mailto:...)
        if url.host then
            parts[#parts + 1] = "://"
        else
            parts[#parts + 1] = ":"
        end
    end
    -- Add authority (userinfo@host:port)
    if url.host then
        if url.userinfo then
            parts[#parts + 1] = url.userinfo
            parts[#parts + 1] = "@"
        end
        parts[#parts + 1] = url.host
        if url.port then
            parts[#parts + 1] = ":"
            parts[#parts + 1] = tostring(url.port)
        end
    end
    -- Path
    if url.path then
        parts[#parts + 1] = url.path
    end
    -- Query
    if url.query then
        parts[#parts + 1] = "?"
        parts[#parts + 1] = url.query
    end
    -- Fragment
    if url.fragment then
        parts[#parts + 1] = "#"
        parts[#parts + 1] = url.fragment
    end
    return table.concat(parts)
end

-- ============================================================================
-- Dot Segment Removal
-- ============================================================================
-- When resolving relative URLs, the resulting path may contain "." (current
-- directory) and ".." (parent directory) segments. These must be removed
-- to produce a clean, canonical path.
--
-- The algorithm (from RFC 3986, Section 5.2.4):
--   Input:  /a/b/c/./../../g
--   Steps:  /a/b/c/./../../g
--           /a/b/c/../../g     (removed ".")
--           /a/b/../../g       (removed "c" via "..")
--           /a/../../g         (removed "b" via "..")
--           /../../g           (removed "a" via "..")
--           /g                 (can't go above root, just "/g")
--
-- We implement this by splitting the path into segments and using a stack:
--   - "." is ignored (stay in current directory)
--   - ".." pops the last segment (go up one level)
--   - anything else is pushed onto the stack

local function remove_dot_segments(path)
    if not path or path == "" then
        return ""
    end

    -- Split path into segments by '/'
    local segments = {}
    local i = 1
    while i <= #path do
        local slash_pos = string.find(path, "/", i, true)
        if slash_pos then
            segments[#segments + 1] = string.sub(path, i, slash_pos - 1)
            i = slash_pos + 1
        else
            segments[#segments + 1] = string.sub(path, i)
            i = #path + 1
        end
    end

    -- Use a stack to resolve . and ..
    local stack = {}
    local starts_with_slash = string.sub(path, 1, 1) == "/"

    for _, seg in ipairs(segments) do
        if seg == "." then
            -- Current directory: skip
        elseif seg == ".." then
            -- Parent directory: pop last segment (if any)
            if #stack > 0 and stack[#stack] ~= "" then
                stack[#stack] = nil
            end
        else
            stack[#stack + 1] = seg
        end
    end

    local result = table.concat(stack, "/")
    if starts_with_slash and (string.sub(result, 1, 1) ~= "/") then
        result = "/" .. result
    end

    -- If the original path ended with / or /. or /.., ensure trailing slash
    local last_seg = segments[#segments]
    if last_seg == "." or last_seg == ".." or (string.sub(path, -1) == "/") then
        if string.sub(result, -1) ~= "/" then
            result = result .. "/"
        end
    end

    return result
end

-- ============================================================================
-- Merge Paths
-- ============================================================================
-- When resolving a relative URL against a base, we need to merge the base
-- path with the relative path. The rules (RFC 3986, Section 5.2.3):
--
--   1. If the base has authority and empty path, use "/" + relative.
--   2. Otherwise, take everything up to the last "/" in the base path,
--      then append the relative path.
--
-- Example:
--   base path: /a/b/c    relative: d/e
--   merge: /a/b/ + d/e = /a/b/d/e

local function merge_paths(base_url, relative_path)
    if base_url.host and (not base_url.path or base_url.path == "" or base_url.path == "/") then
        return "/" .. relative_path
    end

    local base_path = base_url.path or ""
    -- Find the last '/' in the base path
    local last_slash = nil
    for i = #base_path, 1, -1 do
        if string.sub(base_path, i, i) == "/" then
            last_slash = i
            break
        end
    end

    if last_slash then
        return string.sub(base_path, 1, last_slash) .. relative_path
    else
        return relative_path
    end
end

-- ============================================================================
-- Relative URL Resolution
-- ============================================================================
-- Given a base URL and a relative reference, produce the target URL. This
-- follows RFC 3986, Section 5.2.2:
--
--   Relative Reference | Resolution
--   -------------------|------------------------------------------
--   "" (empty)         | base without fragment
--   "#frag"            | base with new fragment
--   "//host/path"      | scheme-relative (use base scheme)
--   "/path"            | absolute path (use base scheme + authority)
--   "path"             | merge with base path + remove dots
--   "?query"           | use base path with new query
--
-- This is how browsers resolve relative links: an <a href="page2.html">
-- on https://example.com/dir/page1.html becomes https://example.com/dir/page2.html

function M.resolve(base_input, relative)
    -- Parse the base URL
    local base, err
    if type(base_input) == "string" then
        base, err = M.parse(base_input)
        if not base then
            return nil, err
        end
    else
        base = base_input
    end

    -- Empty relative reference: return base without fragment
    if not relative or relative == "" then
        return {
            scheme   = base.scheme,
            userinfo = base.userinfo,
            host     = base.host,
            port     = base.port,
            path     = base.path,
            query    = base.query,
            fragment = nil,
            raw      = relative or "",
        }, nil
    end

    -- Try to parse the relative reference as a full URL
    -- If it has a scheme, it's an absolute URL -- just return it parsed
    local rel_has_scheme = false
    local colon_pos = string.find(relative, ":", 1, true)
    local slash_pos = string.find(relative, "/", 1, true)
    if colon_pos and (not slash_pos or colon_pos < slash_pos) then
        local candidate = string.sub(relative, 1, colon_pos - 1)
        if is_valid_scheme(string.lower(candidate)) then
            rel_has_scheme = true
        end
    end

    if rel_has_scheme then
        local parsed, parse_err = M.parse(relative)
        if not parsed then
            return nil, parse_err
        end
        parsed.path = remove_dot_segments(parsed.path)
        return parsed, nil
    end

    -- Fragment-only reference
    if string.sub(relative, 1, 1) == "#" then
        return {
            scheme   = base.scheme,
            userinfo = base.userinfo,
            host     = base.host,
            port     = base.port,
            path     = base.path,
            query    = base.query,
            fragment = string.sub(relative, 2),
            raw      = relative,
        }, nil
    end

    -- Query-only reference
    if string.sub(relative, 1, 1) == "?" then
        local frag = nil
        local q_part = string.sub(relative, 2)
        local hash = string.find(q_part, "#", 1, true)
        if hash then
            frag = string.sub(q_part, hash + 1)
            q_part = string.sub(q_part, 1, hash - 1)
        end
        return {
            scheme   = base.scheme,
            userinfo = base.userinfo,
            host     = base.host,
            port     = base.port,
            path     = base.path,
            query    = q_part,
            fragment = frag,
            raw      = relative,
        }, nil
    end

    -- Scheme-relative reference (starts with "//")
    if string.sub(relative, 1, 2) == "//" then
        local parsed, parse_err = M.parse(base.scheme .. ":" .. relative)
        if not parsed then
            return nil, parse_err
        end
        parsed.path = remove_dot_segments(parsed.path)
        parsed.raw = relative
        return parsed, nil
    end

    -- Now we need to extract fragment and query from relative
    local rel_fragment = nil
    local rel_query = nil
    local rel_path = relative

    local hash = string.find(rel_path, "#", 1, true)
    if hash then
        rel_fragment = string.sub(rel_path, hash + 1)
        rel_path = string.sub(rel_path, 1, hash - 1)
    end

    local qmark = string.find(rel_path, "?", 1, true)
    if qmark then
        rel_query = string.sub(rel_path, qmark + 1)
        rel_path = string.sub(rel_path, 1, qmark - 1)
    end

    -- Absolute path reference (starts with "/")
    if string.sub(rel_path, 1, 1) == "/" then
        return {
            scheme   = base.scheme,
            userinfo = base.userinfo,
            host     = base.host,
            port     = base.port,
            path     = remove_dot_segments(rel_path),
            query    = rel_query,
            fragment = rel_fragment,
            raw      = relative,
        }, nil
    end

    -- Relative path: merge with base
    local merged = merge_paths(base, rel_path)
    return {
        scheme   = base.scheme,
        userinfo = base.userinfo,
        host     = base.host,
        port     = base.port,
        path     = remove_dot_segments(merged),
        query    = rel_query,
        fragment = rel_fragment,
        raw      = relative,
    }, nil
end

return M
