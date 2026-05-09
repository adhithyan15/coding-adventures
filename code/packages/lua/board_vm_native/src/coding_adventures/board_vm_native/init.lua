local M = {}

local function dirname(path)
    return path:match("^(.*)[/\\][^/\\]+$")
end

local native_module = false

local function load_native()
    if native_module ~= false then
        return native_module
    end

    local ok, module = pcall(require, "board_vm_native")
    if ok then
        native_module = module
        return native_module
    end

    local source = debug.getinfo(1, "S").source
    local file = source:sub(1, 1) == "@" and source:sub(2) or source
    local base = dirname(file) .. "/../../../target/release/"
    local candidates = {
        base .. "libboard_vm_native.dylib",
        base .. "libboard_vm_native.so",
        base .. "board_vm_native.dll",
    }

    for _, path in ipairs(candidates) do
        local loader = package.loadlib(path, "luaopen_board_vm_native")
        if loader then
            native_module = loader()
            return native_module
        end
    end

    native_module = nil
    return native_module
end

local function native()
    local module = load_native()
    if module == nil then
        error("board_vm_native extension is not available")
    end
    return module
end

function M.available()
    return load_native() ~= nil
end

function M.defaults()
    return native().defaults()
end

function M.known_targets()
    return native().known_targets()
end

function M.detect_target(selector)
    return native().detect_target(selector)
end

function M.connection_options(selector)
    return native().connection_options(selector)
end

function M.esp_upload_options(selector)
    return native().esp_upload_options(selector or "esp32")
end

function M.pico_uf2_upload_options(selector)
    return native().pico_uf2_upload_options(selector or "pico")
end

function M.devices(paths)
    if paths then
        return native().classify_devices(paths)
    end
    return native().devices()
end

local Session = {}
Session.__index = Session

function Session.new(options)
    options = options or {}
    return setmetatable({
        next_request_id = options.next_request_id or 1,
        program_id = options.program_id or M.defaults().program_id,
        run_flags = options.run_flags or M.defaults().run_flags,
        instruction_budget = options.instruction_budget or M.defaults().instruction_budget,
        time_budget_ms = options.time_budget_ms or 0,
    }, Session)
end

function Session:_frame(result)
    self.next_request_id = result.next_request_id
    return result.frame
end

function Session:hello_wire(host_name, host_nonce)
    return self:_frame(native().hello_wire(
        self.next_request_id,
        host_name or "bvm-lua",
        host_nonce or 0x1234abcd
    ))
end

function Session:caps_query_wire()
    return self:_frame(native().caps_query_wire(self.next_request_id))
end

function Session:blink_module(options)
    options = options or {}
    return native().blink_module(
        options.pin or 13,
        options.high_ms or 250,
        options.low_ms or 250,
        options.max_stack or 4
    )
end

function Session:program_begin_wire(program_id, module_bytes)
    return self:_frame(native().program_begin_wire(
        self.next_request_id,
        program_id,
        module_bytes
    ))
end

function Session:program_chunk_wire(program_id, offset, chunk)
    return self:_frame(native().program_chunk_wire(
        self.next_request_id,
        program_id,
        offset,
        chunk
    ))
end

function Session:program_end_wire(program_id)
    return self:_frame(native().program_end_wire(self.next_request_id, program_id))
end

function Session:run_wire(options)
    options = options or {}
    return self:_frame(native().run_wire(
        self.next_request_id,
        options.program_id or self.program_id,
        options.flags or self.run_flags,
        options.instruction_budget or self.instruction_budget,
        options.time_budget_ms or self.time_budget_ms
    ))
end

function Session:blink_upload_run_frames(options)
    options = options or {}
    local program_id = options.program_id or self.program_id
    local module_bytes = self:blink_module(options)
    return {
        self:program_begin_wire(program_id, module_bytes),
        self:program_chunk_wire(program_id, 0, module_bytes),
        self:program_end_wire(program_id),
        self:run_wire({
            program_id = program_id,
            flags = options.flags or self.run_flags,
            instruction_budget = options.instruction_budget or self.instruction_budget,
            time_budget_ms = options.time_budget_ms or self.time_budget_ms,
        }),
    }
end

M.Session = Session

function M.session(options)
    return Session.new(options)
end

return M
