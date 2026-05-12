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

function M.bluetooth_endpoint(endpoint)
    return native().bluetooth_endpoint(endpoint)
end

function M.bluetooth_backend(endpoint)
    return native().bluetooth_backend(endpoint)
end

function M.bluetooth_transact(endpoint, frame)
    return native().bluetooth_transact(endpoint, frame)
end

function M.bluetooth_devices()
    return native().bluetooth_devices()
end

function M.bluetooth_endpoint_candidates(devices)
    devices = devices or M.bluetooth_devices()
    return native().bluetooth_endpoint_candidates(devices)
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

local function normalize_connection_transport(transport)
    local normalized = tostring(transport):lower():gsub("%-", "_"):gsub("[ /]", "_")
    local aliases = {
        usb = "serial",
        usb_serial = "serial",
        serial_port = "serial",
        wi_fi = "wifi",
        wireless = "wifi",
        ble = "bluetooth_le",
        bluetooth = "bluetooth_le",
        bluetooth_low_energy = "bluetooth_le",
    }
    return aliases[normalized] or normalized
end

local function connection_uses_serial_port(connection_option)
    return connection_option == nil or connection_option.transport == "serial"
end

local function connection_uses_tcp_endpoint(connection_option)
    return connection_option ~= nil and (
        connection_option.endpoint_transport == "tcp_socket" or
        connection_option.endpoint_scheme == "tcp"
    )
end

local function connection_uses_bluetooth_endpoint(connection_option)
    return connection_option ~= nil and (
        connection_option.transport == "bluetooth_le" or
        connection_option.transport == "bluetooth_classic" or
        connection_option.endpoint_transport == "bluetooth_le_gatt" or
        connection_option.endpoint_transport == "bluetooth_classic_rfcomm" or
        connection_option.endpoint_scheme == "ble" or
        connection_option.endpoint_scheme == "btspp" or
        connection_option.endpoint_scheme == "rfcomm"
    )
end

local function bluetooth_candidate_matches_connection(candidate, connection_option)
    local endpoint = candidate.endpoint or {}
    return (
        connection_option.transport ~= nil and endpoint.transport == connection_option.transport
    ) or (
        connection_option.endpoint_transport ~= nil and endpoint.endpoint_transport == connection_option.endpoint_transport
    ) or (
        connection_option.endpoint_scheme ~= nil and endpoint.endpoint_scheme == connection_option.endpoint_scheme
    )
end

local function bluetooth_endpoint_choice_list(candidates)
    local lines = {}
    for index, candidate in ipairs(candidates) do
        local endpoint = candidate.endpoint or {}
        local display_name = candidate.display_name or candidate.device or endpoint.endpoint
        table.insert(lines, tostring(index) .. ". " .. tostring(display_name) .. " - " .. tostring(endpoint.endpoint))
    end
    return table.concat(lines, "\n")
end

function M.bluetooth_connection_endpoint(connection_option, devices)
    local matches = {}
    for _, candidate in ipairs(M.bluetooth_endpoint_candidates(devices)) do
        if bluetooth_candidate_matches_connection(candidate, connection_option) then
            table.insert(matches, candidate)
        end
    end

    if #matches == 1 then
        return tostring(matches[1].endpoint.endpoint)
    end

    local display_name = connection_option.display_name or connection_option.transport or "Bluetooth"
    if #matches == 0 then
        error(display_name ..
            " found no Board VM Bluetooth endpoints; pair or power on the board, pass endpoint = ..., or choose via = \"serial\"")
    end

    error("Multiple Board VM Bluetooth endpoints match " ..
        display_name .. "; pass endpoint = ...\n" .. bluetooth_endpoint_choice_list(matches))
end

local function parse_tcp_endpoint(endpoint)
    local authority = tostring(endpoint or ""):gsub("^tcp://", "")
    local host, port = authority:match("^%[([^%]]+)%]:(%d+)$")
    if host == nil then
        host, port = authority:match("^([^:]+):(%d+)$")
    end
    if host == nil or port == nil then
        error("Board VM TCP endpoint must look like tcp://host:port")
    end
    return host, tonumber(port)
end

local TcpTransport = {}
TcpTransport.__index = TcpTransport

function TcpTransport.new(options)
    options = options or {}
    local endpoint = options.endpoint
    local host, port = parse_tcp_endpoint(endpoint)
    return setmetatable({
        endpoint = tostring(endpoint),
        host = host,
        port = port,
        timeout_ms = options.timeout_ms or 1000,
        socket = nil,
    }, TcpTransport)
end

function TcpTransport:_socket_module()
    local ok, socket = pcall(require, "socket")
    if not ok then
        error("Board VM Lua TCP transport requires LuaSocket or an injected transport endpoint")
    end
    return socket
end

function TcpTransport:_io()
    if self.socket then
        return self.socket
    end
    local socket = self:_socket_module()
    local client = assert(socket.tcp())
    client:settimeout(self.timeout_ms / 1000)
    assert(client:connect(self.host, self.port))
    if client.setoption then
        pcall(function()
            client:setoption("tcp-nodelay", true)
        end)
    end
    self.socket = client
    return self.socket
end

function TcpTransport:write(frame)
    assert(self:_io():send(frame))
end

function TcpTransport:transact(frame, options)
    options = options or {}
    self:write(frame)
    local client = self:_io()
    client:settimeout((options.timeout_ms or self.timeout_ms) / 1000)
    local chunks = {}
    while true do
        local byte = assert(client:receive(1))
        table.insert(chunks, byte)
        if byte:byte(1) == 0 then
            return table.concat(chunks)
        end
    end
end

function TcpTransport:close()
    if self.socket then
        self.socket:close()
        self.socket = nil
    end
end

M.TcpTransport = TcpTransport

local BluetoothTransport = {}
BluetoothTransport.__index = BluetoothTransport

function BluetoothTransport.new(options)
    options = options or {}
    local endpoint = tostring(options.endpoint or "")
    local backend = options.backend or M.bluetooth_backend(endpoint)
    if backend == nil then
        error("unsupported Board VM Bluetooth endpoint: " .. endpoint)
    end
    return setmetatable({
        endpoint = endpoint,
        timeout_ms = options.timeout_ms or 1000,
        backend = backend,
        stream_path = backend.stream_path,
        native_transport = not not backend.native_transport,
        file = nil,
    }, BluetoothTransport)
end

function BluetoothTransport:status()
    return self.backend.status
end

function BluetoothTransport:_io()
    if self.file then
        return self.file
    end
    if self.backend.status ~= "ready" or self.stream_path == nil or tostring(self.stream_path) == "" then
        error("failed to open Board VM Bluetooth endpoint " ..
            self.endpoint .. ": " .. tostring(self.backend.message or "Bluetooth backend is not ready"))
    end
    local file, err = io.open(self.stream_path, "r+b")
    if file == nil then
        error("failed to open Board VM Bluetooth stream " .. tostring(self.stream_path) .. ": " .. tostring(err))
    end
    self.file = file
    return self.file
end

function BluetoothTransport:write(frame)
    if self.native_transport then
        error("native Board VM Bluetooth transport requires transact(frame, options)")
    end

    assert(self:_io():write(frame))
    assert(self:_io():flush())
end

function BluetoothTransport:transact(frame, options)
    if self.native_transport then
        return M.bluetooth_transact(self.endpoint, frame)
    end

    options = options or {}
    self:write(frame)
    local chunks = {}
    while true do
        local byte = self:_io():read(1)
        if byte == nil or byte == "" then
            error("Board VM Bluetooth endpoint " .. self.endpoint .. " closed")
        end
        table.insert(chunks, byte)
        if byte:byte(1) == 0 then
            return table.concat(chunks)
        end
    end
end

function BluetoothTransport:close()
    if self.file then
        self.file:close()
        self.file = nil
    end
end

M.BluetoothTransport = BluetoothTransport

local function clone_table(value)
    if type(value) ~= "table" then
        return value
    end

    local copy = {}
    for key, item in pairs(value) do
        copy[key] = item
    end
    return copy
end

function M.connection_option_list(selector)
    local options = M.connection_options(selector)
    if #options == 0 then
        return "No Board VM connection options found for " .. tostring(selector) .. "."
    end

    local lines = {}
    for index, option in ipairs(options) do
        local badges = {}
        if option.command_transport then
            table.insert(badges, "commands")
        end
        if option.ota_update then
            table.insert(badges, "OTA")
        end
        local badge_label = ""
        if #badges > 0 then
            badge_label = " [" .. table.concat(badges, ", ") .. "]"
        end
        lines[index] = string.format(
            "%d. %s%s - requires %s",
            index,
            option.display_name,
            badge_label,
            option.requires
        )
    end
    return table.concat(lines, "\n")
end

function M.select_connection_option(selector, options)
    options = options or {}
    local connection_options = M.connection_options(selector)
    local matches = {}
    for _, option in ipairs(connection_options) do
        if option.command_transport and (not options.ota or option.ota_update) then
            table.insert(matches, option)
        end
    end

    local transport = options.transport or options.via
    if transport then
        local normalized = normalize_connection_transport(transport)
        for _, option in ipairs(connection_options) do
            if option.transport == normalized and (not options.ota or option.ota_update) then
                return clone_table(option)
            end
        end
        error(
            "No " .. normalized .. " connection option for " .. tostring(selector) ..
            ".\n" .. M.connection_option_list(selector)
        )
    end

    if not options.ota then
        for _, option in ipairs(matches) do
            if option.transport == "serial" then
                return clone_table(option)
            end
        end
    end
    if #matches == 1 then
        return clone_table(matches[1])
    end

    local reason = #matches == 0 and "No matching connection option" or "Multiple connection options match"
    error(reason .. " for " .. tostring(selector) .. ".\n" .. M.connection_option_list(selector))
end

function M.device_list(device_candidates)
    local candidates = device_candidates or M.devices()
    if #candidates == 0 then
        return "No Board VM devices found."
    end

    local lines = {}
    for index, device in ipairs(candidates) do
        local target_name = "Unknown board"
        if device.target then
            target_name = device.target.display_name
        end
        local confidence = ""
        if (device.target_confidence or 0) > 0 then
            confidence = string.format(", %d%% match", device.target_confidence)
        end
        local tags = ""
        if device.tags and #device.tags > 0 then
            tags = " [" .. table.concat(device.tags, ", ") .. "]"
        end
        lines[index] = string.format("%d. %s - %s%s%s", index, target_name, device.port, confidence, tags)
    end
    return table.concat(lines, "\n")
end

function M.select_device(selector, options)
    selector = selector or "auto"
    options = options or {}
    local candidates = options.devices or options.device_candidates
    if candidates == nil then
        candidates = M.devices(options.paths)
    end

    local device = options.device
    if type(device) == "table" then
        return device
    elseif type(device) == "number" then
        if candidates[device] then
            return candidates[device]
        end
        error("No Board VM device at index " .. tostring(device) .. ".")
    elseif device ~= nil then
        local needle = tostring(device)
        for _, candidate in ipairs(candidates) do
            if candidate.id == needle or candidate.port == needle then
                return candidate
            end
        end
        error("No Board VM device named " .. string.format("%q", needle) .. ".\n" .. M.device_list(candidates))
    end

    local target = selector == "auto" and nil or M.detect_target(selector)
    if selector ~= "auto" and target == nil then
        error("unsupported board: " .. tostring(selector))
    end

    local matches = {}
    if target == nil then
        for _, candidate in ipairs(candidates) do
            if candidate.target then
                table.insert(matches, candidate)
            end
        end
        if #matches == 0 and #candidates == 1 then
            table.insert(matches, candidates[1])
        end
    else
        for _, candidate in ipairs(candidates) do
            if candidate.target and candidate.target.board_id == target.board_id then
                table.insert(matches, candidate)
            end
        end
        if #matches == 0 then
            for _, candidate in ipairs(candidates) do
                if candidate.target == nil then
                    table.insert(matches, candidate)
                end
            end
        end
    end

    if #matches == 1 then
        return matches[1]
    end
    if #candidates == 0 then
        error("No Board VM devices found. Plug in a board or pass an explicit device.")
    end

    local reason
    if #matches == 0 and target == nil then
        reason = "Multiple Board VM devices found; choose one"
    elseif #matches == 0 then
        reason = "No matching Board VM device found"
    else
        reason = "Multiple Board VM devices match"
    end
    error(reason .. ".\n" .. M.device_list(candidates))
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
        transport = options.transport,
        timeout_ms = options.timeout_ms or 1000,
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

function Session:dispatch_wire(frame)
    if not self.transport then
        error("Board VM Lua session requires a transport endpoint with write(frame) or transact(frame, options)")
    end
    local response = nil
    if type(self.transport.transact) == "function" then
        response = self.transport:transact(frame, { timeout_ms = self.timeout_ms })
    elseif type(self.transport.write) == "function" then
        self.transport:write(frame)
    else
        error("Board VM Lua transport must expose write(frame) or transact(frame, options)")
    end
    return { frame = frame, response = response }
end

function Session:hello(host_name, host_nonce)
    return self:dispatch_wire(self:hello_wire(host_name, host_nonce))
end

function Session:capabilities()
    return self:dispatch_wire(self:caps_query_wire())
end

function Session:smoke(options)
    options = options or {}
    return {
        self:hello(options.host_name, options.host_nonce),
        self:capabilities(),
    }
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

local Connection = {}
Connection.__index = Connection

function Connection.new(options)
    return setmetatable({
        target = options.target,
        port = options.port,
        device = options.device,
        connection_option = options.connection_option,
        transport = options.transport,
        endpoint = options.endpoint,
        timeout_ms = options.timeout_ms or 1000,
        bluetooth_backend_plan = options.bluetooth_backend_plan,
    }, Connection)
end

function Connection:board_id()
    return self.target.board_id
end

function Connection:connection_transport()
    if self.connection_option then
        return self.connection_option.transport
    end
    return nil
end

function Connection:serial_connection()
    local transport = self:connection_transport()
    return transport == nil or transport == "serial"
end

function Connection:wireless_connection()
    return not self:serial_connection()
end

function Connection:ota_connection()
    return self.connection_option and self.connection_option.ota_update or false
end

function Connection:session(options)
    options = options or {}
    options.transport = options.transport or self:active_transport()
    options.timeout_ms = options.timeout_ms or self.timeout_ms
    return Session.new(options)
end

function Connection:smoke(options)
    return self:session():smoke(options)
end

function Connection:active_transport()
    if self.transport then
        return self.transport
    end
    if connection_uses_tcp_endpoint(self.connection_option) then
        if self.endpoint == nil or tostring(self.endpoint) == "" then
            error((self.connection_option.display_name or "Board VM TCP connection") ..
                " requires a Board VM TCP endpoint; pass endpoint = \"tcp://host:port\" or choose via = \"serial\"")
        end
        self.transport = TcpTransport.new({
            endpoint = self.endpoint,
            timeout_ms = self.timeout_ms,
        })
        return self.transport
    end
    if connection_uses_bluetooth_endpoint(self.connection_option) then
        if self.endpoint == nil or tostring(self.endpoint) == "" then
            error((self.connection_option.display_name or "Board VM Bluetooth connection") ..
                " requires a Board VM Bluetooth endpoint; pass endpoint = ... or choose via = \"serial\"")
        end
        self.transport = BluetoothTransport.new({
            endpoint = self.endpoint,
            timeout_ms = self.timeout_ms,
            backend = self.bluetooth_backend_plan,
        })
        return self.transport
    end
    return nil
end

M.Connection = Connection

function M.connect(selector, options)
    if type(selector) == "table" then
        options = selector
        selector = options.selector or options.board or "auto"
    end
    selector = selector or "auto"
    options = options or {}

    local target = selector == "auto" and nil or M.detect_target(selector)
    if selector ~= "auto" and target == nil then
        error("unsupported board: " .. tostring(selector))
    end

    local connection_option = nil
    if target then
        connection_option = options.connection_option or M.select_connection_option(target.board_id, {
            via = options.via,
            transport = options.transport_name,
            ota = options.ota,
        })
    end

    local needs_device_for_target = target == nil and options.port == nil
    local needs_serial_port = connection_uses_serial_port(connection_option) and options.port == nil
    local device = options.device
    if type(device) ~= "table" and (device ~= nil or needs_device_for_target or needs_serial_port) then
        device = M.select_device(selector, {
            device = device,
            devices = options.devices or options.device_candidates,
            paths = options.paths,
        })
    end

    local port = options.port
    if port == nil and device then
        port = device.port
    end

    if target == nil then
        if device and device.target then
            target = device.target
        elseif port then
            target = M.detect_target("uno-r4-wifi")
        else
            error("Could not infer the board for the selected device.\n" .. M.device_list(options.devices))
        end
    end

    connection_option = connection_option or options.connection_option or M.select_connection_option(target.board_id, {
        via = options.via,
        transport = options.transport_name,
        ota = options.ota,
    })

    local endpoint = options.endpoint
    if endpoint == nil and connection_uses_bluetooth_endpoint(connection_option) then
        endpoint = M.bluetooth_connection_endpoint(connection_option, options.bluetooth_devices)
    end

    return Connection.new({
        target = target,
        port = port,
        device = device,
        connection_option = connection_option,
        transport = options.transport,
        endpoint = endpoint,
        timeout_ms = options.timeout_ms,
        bluetooth_backend_plan = options.bluetooth_backend_plan,
    })
end

return M
