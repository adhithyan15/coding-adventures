-- discovery.lua -- Package Discovery via Recursive BUILD File Walk
-- ================================================================
--
-- This module walks a monorepo directory tree to discover packages. A
-- "package" is any directory that contains a BUILD file. The walk is
-- recursive: starting from the root, we list all subdirectories and descend
-- into each one, skipping known non-source directories (.git, .venv,
-- node_modules, etc.).
--
-- When we find a BUILD file in a directory, we stop recursing there and
-- register that directory as a package. This is the same approach used by
-- Bazel, Buck, and Pants.
--
-- Platform-specific BUILD files
-- -----------------------------
--
-- If we're on macOS and a ``BUILD_mac`` file exists, we use that instead of
-- ``BUILD``. Similarly, ``BUILD_linux`` on Linux, ``BUILD_windows`` on Windows.
--
-- Language inference
-- -----------------
--
-- We infer the language from the directory path. If the path contains
-- ``packages/python/X`` or ``programs/python/X``, the language is "python".
-- Similarly for "ruby", "go", "rust", "typescript", "elixir", and "lua".

-- lfs is optional — if available, we use it for directory listing.
-- Otherwise, we fall back to os.execute with ls/dir.
local lfs_ok, lfs = pcall(require, "lfs")

local Discovery = {}

-- KNOWN_LANGUAGES lists the language directory names we look for when
-- inferring which ecosystem a package belongs to.
local KNOWN_LANGUAGES = {
    "python", "ruby", "go", "rust", "typescript", "elixir", "lua", "perl",
    "swift", "haskell", "starlark", "wasm", "csharp", "fsharp", "dotnet",
}

-- SKIP_DIRS is the set of directory names that should never be traversed
-- during package discovery. These are known to contain non-source files
-- (caches, dependencies, build artifacts) that would waste time to scan.
local SKIP_DIRS = {}
for _, name in ipairs({
    ".git", ".hg", ".svn", ".venv", ".tox", ".mypy_cache",
    ".pytest_cache", ".ruff_cache", "__pycache__", "node_modules",
    "vendor", "dist", "build", "target", ".claude", "Pods",
    "_build", "deps", "coverage",
}) do
    SKIP_DIRS[name] = true
end

--- Read a file and return non-blank, non-comment lines.
--
-- Blank lines and lines starting with '#' are stripped out. Leading and
-- trailing whitespace is removed from each line.
--
-- @param filepath string The file to read.
-- @return table A list of cleaned lines.
function Discovery.read_lines(filepath)
    local f = io.open(filepath, "r")
    if not f then
        return {}
    end

    local lines = {}
    for line in f:lines() do
        local stripped = line:match("^%s*(.-)%s*$")
        if stripped ~= "" and not stripped:match("^#") then
            lines[#lines + 1] = stripped
        end
    end
    f:close()
    return lines
end

--- Check if a file exists and is not a directory.
--
-- @param path string The file path.
-- @return boolean
function Discovery.file_exists(path)
    if lfs_ok then
        local attr = lfs.attributes(path)
        return attr ~= nil and attr.mode == "file"
    else
        local f = io.open(path, "r")
        if f then
            f:close()
            return true
        end
        return false
    end
end

--- Check if a directory exists.
--
-- @param path string The directory path.
-- @return boolean
function Discovery.dir_exists(path)
    if lfs_ok then
        local attr = lfs.attributes(path)
        return attr ~= nil and attr.mode == "directory"
    else
        -- Fallback: try to open the directory with os.execute
        local ok = os.execute('cd "' .. path .. '" 2>/dev/null')
        return ok == true or ok == 0
    end
end

--- Detect the current operating system.
--
-- @return string "darwin", "linux", "windows", or "unknown".
function Discovery.detect_os()
    local sep = package.config:sub(1, 1)
    if sep == "\\" then
        return "windows"
    end
    -- On Unix, use uname to distinguish macOS from Linux.
    local handle = io.popen("uname -s 2>/dev/null")
    if handle then
        local result = handle:read("*a"):match("^%s*(.-)%s*$")
        handle:close()
        if result == "Darwin" then
            return "darwin"
        elseif result == "Linux" then
            return "linux"
        end
    end
    return "unknown"
end

--- Return the appropriate BUILD file path for the current platform.
--
-- Priority (most specific wins):
--   1. Platform-specific: BUILD_mac (macOS), BUILD_linux, BUILD_windows
--   2. Shared: BUILD_mac_and_linux (macOS or Linux)
--   3. Generic: BUILD (all platforms)
--   4. nil if no BUILD file exists
--
-- @param directory string The directory to check.
-- @param goos string|nil Optional OS override for testing.
-- @return string|nil The BUILD file path, or nil.
function Discovery.get_build_file(directory, goos)
    goos = goos or Discovery.detect_os()

    -- Step 1: Check for the most specific platform file.
    if goos == "darwin" then
        local path = directory .. "/BUILD_mac"
        if Discovery.file_exists(path) then return path end
    elseif goos == "linux" then
        local path = directory .. "/BUILD_linux"
        if Discovery.file_exists(path) then return path end
    elseif goos == "windows" then
        local path = directory .. "/BUILD_windows"
        if Discovery.file_exists(path) then return path end
    end

    -- Step 2: Check for the shared Unix file (macOS + Linux).
    if goos == "darwin" or goos == "linux" then
        local path = directory .. "/BUILD_mac_and_linux"
        if Discovery.file_exists(path) then return path end
    end

    -- Step 3: Fall back to the generic BUILD file.
    local path = directory .. "/BUILD"
    if Discovery.file_exists(path) then return path end

    return nil
end

--- Infer the programming language from the directory path.
--
-- We split the path into its component parts and look for a known language
-- directory name. The first match wins.
--
-- @param path string The package directory path.
-- @return string The inferred language, or "unknown".
function Discovery.infer_language(path)
    -- Normalize path separators to forward slashes.
    local normalized = path:gsub("\\", "/")
    for _, lang in ipairs(KNOWN_LANGUAGES) do
        -- Match as a full path component: /lang/ or starting with lang/
        if normalized:match("/" .. lang .. "/") or normalized:match("^" .. lang .. "/") then
            return lang
        end
    end
    return "unknown"
end

--- Build a qualified package name like "python/logic-gates".
--
-- @param path string The package directory path.
-- @param language string The inferred language.
-- @return string The qualified package name.
function Discovery.infer_package_name(path, language)
    -- Extract the directory basename.
    local normalized = path:gsub("\\", "/")
    local basename = normalized:match("([^/]+)$")
    return language .. "/" .. basename
end

--- List subdirectories of a directory.
--
-- @param directory string The parent directory.
-- @return table A sorted list of subdirectory paths.
function Discovery.list_subdirs(directory)
    local dirs = {}

    if lfs_ok then
        for name in lfs.dir(directory) do
            if name ~= "." and name ~= ".." then
                local full_path = directory .. "/" .. name
                local attr = lfs.attributes(full_path)
                if attr and attr.mode == "directory" then
                    dirs[#dirs + 1] = full_path
                end
            end
        end
    else
        -- Fallback: use ls or dir command.
        local sep = package.config:sub(1, 1)
        local cmd
        if sep == "\\" then
            cmd = 'dir /b /ad "' .. directory .. '" 2>nul'
        else
            cmd = 'ls -1 "' .. directory .. '" 2>/dev/null'
        end
        local handle = io.popen(cmd)
        if handle then
            for raw_name in handle:lines() do
                local name = raw_name:match("^%s*(.-)%s*$")
                if name ~= "" then
                    local full_path = directory .. "/" .. name
                    if Discovery.dir_exists(full_path) then
                        dirs[#dirs + 1] = full_path
                    end
                end
            end
            handle:close()
        end
    end

    table.sort(dirs)
    return dirs
end

--- Recursively walk directories and collect packages with BUILD files.
--
-- If the current directory's name is in the skip list, ignore it entirely.
-- If the current directory has a BUILD file, it is a package — register it
-- and don't recurse further. Otherwise, list all subdirectories and recurse.
--
-- @param directory string The current directory.
-- @param packages table Accumulator for discovered packages.
local function walk_dirs(directory, packages)
    -- Extract basename for skip check.
    local basename = directory:gsub("\\", "/"):match("([^/]+)$")
    if SKIP_DIRS[basename] then
        return
    end

    local build_file = Discovery.get_build_file(directory)

    if build_file then
        -- This directory is a package.
        local commands = Discovery.read_lines(build_file)
        local language = Discovery.infer_language(directory)
        local name = Discovery.infer_package_name(directory, language)

        packages[#packages + 1] = {
            name = name,
            path = directory,
            build_commands = commands,
            language = language,
        }
        return
    end

    -- Not a package — list subdirectories and recurse.
    local subdirs = Discovery.list_subdirs(directory)
    for _, subdir in ipairs(subdirs) do
        walk_dirs(subdir, packages)
    end
end

--- Discover all packages under the given root directory.
--
-- Recursively walks the directory tree, collecting packages with BUILD
-- files. The returned list is sorted by package name for deterministic
-- output.
--
-- @param root string The monorepo root directory.
-- @return table A list of package tables, sorted by name.
function Discovery.discover_packages(root)
    local packages = {}
    walk_dirs(root, packages)

    -- Sort by name for deterministic output.
    table.sort(packages, function(a, b)
        return a.name < b.name
    end)

    return packages
end

return Discovery
