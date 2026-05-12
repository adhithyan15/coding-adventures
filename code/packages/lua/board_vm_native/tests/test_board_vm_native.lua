package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local board_vm = require("coding_adventures.board_vm_native")

describe("coding_adventures.board_vm_native", function()
    it("loads the Rust native extension", function()
        assert.is_true(board_vm.available())
    end)

    it("exposes Rust-owned board target metadata", function()
        local targets = board_vm.known_targets()
        local uno = board_vm.detect_target("UNO R4 WiFi")
        local esp = board_vm.detect_target("esp32")
        local pico = board_vm.detect_target("pico")
        local pico_w = board_vm.detect_target("pico-w")

        assert.is_true(#targets >= 4)
        assert.are.equal("arduino-uno-r4-wifi", uno.board_id)
        assert.are.equal("arduino_uno_r4", uno.family)
        assert.are.equal("wifi", uno.wireless[1].transport)
        assert.are.equal("serial", uno.connection_options[1].transport)
        assert.is_true(uno.connection_options[2].ota_update)
        assert.are.equal("esp32-devkit-v1", esp.board_id)
        assert.are.equal("xtensa-esp32-none-elf", esp.rust_target)
        assert.are.equal("gpio", esp.onboard_led.kind)
        assert.are.equal("raspberry-pi-pico", pico.board_id)
        assert.are.equal(0, #pico.wireless)
        assert.are.equal("raspberry-pi-pico-w", pico_w.board_id)
        assert.are.equal("wireless_chip_gpio", pico_w.onboard_led.kind)
        assert.is_nil(board_vm.detect_target("not-a-board"))
    end)

    it("exposes Rust-owned connection and upload options", function()
        local options = board_vm.connection_options("uno-r4-wifi")
        local default = board_vm.select_connection_option("uno-r4-wifi")
        local wifi = board_vm.select_connection_option("uno-r4-wifi", { via = "Wi-Fi" })
        local ble = board_vm.select_connection_option("uno-r4-wifi", { via = "BLE" })
        local esp_upload = board_vm.esp_upload_options("esp32")
        local pico_upload = board_vm.pico_uf2_upload_options("pico-w")

        assert.are.equal("USB/serial", options[1].display_name)
        assert.are.equal("serial_port", options[1].endpoint_transport)
        assert.are.equal("serial", options[1].endpoint_scheme)
        assert.are.equal("board_vm_cobs_crc", options[1].wire_protocol)
        assert.are.equal("network_endpoint", options[2].requires)
        assert.are.equal("tcp_socket", options[2].endpoint_transport)
        assert.are.equal("tcp", options[2].endpoint_scheme)
        assert.is_true(options[2].ota_update)
        assert.are.equal("serial", default.transport)
        assert.are.equal("wifi", wifi.transport)
        assert.are.equal("bluetooth_le", ble.transport)
        assert.is_true(board_vm.connection_option_list("uno-r4-wifi"):find("Wi%-Fi") ~= nil)
        assert.are.equal(0x1000, esp_upload.offset)
        assert.are.equal(115200, esp_upload.baud_rate)
        assert.are.equal("pico-uf2", pico_upload.command)
        assert.are.equal("RPI-RP2", pico_upload.volume_label)
        assert.is_nil(board_vm.esp_upload_options("pico"))
    end)

    it("parses Bluetooth endpoints through Rust-owned metadata", function()
        local ble = board_vm.bluetooth_endpoint("ble://uno-r4-wifi/180f/2a19/2a1a")
        local rfcomm = board_vm.bluetooth_endpoint("btspp://ESP32-BoardVM:3")

        assert.are.equal("bluetooth_le", ble.transport)
        assert.are.equal("bluetooth_le_gatt", ble.endpoint_transport)
        assert.are.equal("ble", ble.endpoint_scheme)
        assert.are.equal("uno-r4-wifi", ble.device)
        assert.are.equal("180f", ble.service_uuid)
        assert.are.equal("2a19", ble.write_characteristic_uuid)
        assert.are.equal("2a1a", ble.notify_characteristic_uuid)
        assert.is_nil(ble.channel)

        assert.are.equal("bluetooth_classic", rfcomm.transport)
        assert.are.equal("bluetooth_classic_rfcomm", rfcomm.endpoint_transport)
        assert.are.equal("btspp", rfcomm.endpoint_scheme)
        assert.are.equal("ESP32-BoardVM", rfcomm.device)
        assert.are.equal(3, rfcomm.channel)
        assert.is_nil(board_vm.bluetooth_endpoint("tcp://board-vm.local:4170"))
    end)

    it("plans Bluetooth endpoint candidates through Rust-owned metadata", function()
        local candidates = board_vm.bluetooth_endpoint_candidates({
            {
                id = "ignore-me",
                service_uuids = { "180f" },
            },
            {
                id = "esp32-board-vm",
                name = "ESP32 Board VM",
                paired = true,
                board_vm_rfcomm_channels = { 3, 3, 31 },
            },
            {
                id = "uno-r4",
                name = "Uno R4 Board VM",
                address = "AA:BB:CC:DD:EE:FF",
                service_uuids = { "6E400001-B5A3-F393-E0A9-E50E24DCCA9E" },
            },
        })

        local rfcomm = nil
        local ble = nil
        for _, candidate in ipairs(candidates) do
            if candidate.endpoint.channel == 3 then
                rfcomm = candidate
            end
            if candidate.endpoint.service_uuid ~= nil then
                ble = candidate
            end
        end

        assert.are.equal(2, #candidates)
        assert.are.equal("ESP32 Board VM", rfcomm.display_name)
        assert.are.equal("bluetooth_classic_rfcomm", rfcomm.endpoint.endpoint_transport)
        assert.are.equal("btspp://esp32-board-vm:3", rfcomm.endpoint.endpoint)
        assert.is_true(rfcomm.paired)
        assert.is_false(rfcomm.requires_pairing)

        assert.are.equal("Uno R4 Board VM", ble.display_name)
        assert.are.equal("bluetooth_le_gatt", ble.endpoint.endpoint_transport)
        assert.are.equal("AA:BB:CC:DD:EE:FF", ble.device)
        assert.are.equal("6e400001-b5a3-f393-e0a9-e50e24dcca9e", ble.endpoint.service_uuid)
        assert.is_false(ble.paired)
        assert.is_true(ble.requires_pairing)
    end)

    it("can discover Bluetooth metadata through the Rust adapter", function()
        assert.are.equal("table", type(board_vm.bluetooth_devices()))
        assert.are.equal("table", type(board_vm.bluetooth_endpoint_candidates()))
    end)

    it("filters Bluetooth endpoint candidates to the selected transport", function()
        local option = board_vm.select_connection_option("uno-r4-wifi", { via = "BLE" })
        local endpoint = board_vm.bluetooth_connection_endpoint(option, {
            {
                id = "esp32-board-vm",
                name = "ESP32 Board VM",
                paired = true,
                board_vm_rfcomm_channels = { 3 },
            },
            {
                id = "uno-r4",
                name = "Uno R4 Board VM",
                address = "AA:BB:CC:DD:EE:FF",
                service_uuids = { "6E400001-B5A3-F393-E0A9-E50E24DCCA9E" },
            },
        })

        assert.are.equal(
            "ble://AA:BB:CC:DD:EE:FF" ..
            "?service=6e400001-b5a3-f393-e0a9-e50e24dcca9e" ..
            "&write=6e400002-b5a3-f393-e0a9-e50e24dcca9e" ..
            "&notify=6e400003-b5a3-f393-e0a9-e50e24dcca9e",
            endpoint
        )
    end)

    it("prompts for Bluetooth endpoints with pairing status", function()
        local rendered = {}
        local output = {
            write = function(_, text)
                table.insert(rendered, text)
            end,
        }
        local connection = board_vm.connect("uno-r4-wifi", {
            via = "BLE",
            bluetooth_devices = {
                {
                    id = "uno-a",
                    name = "Uno A",
                    address = "AA:BB:CC:DD:EE:01",
                    service_uuids = { "6E400001-B5A3-F393-E0A9-E50E24DCCA9E" },
                },
                {
                    id = "uno-b",
                    name = "Uno B",
                    address = "AA:BB:CC:DD:EE:02",
                    paired = true,
                    service_uuids = { "6E400001-B5A3-F393-E0A9-E50E24DCCA9E" },
                },
            },
            pick_bluetooth_endpoint = true,
            input = function()
                return "2"
            end,
            output = output,
        })

        assert.are.equal("bluetooth_le", connection:connection_transport())
        assert.are.equal(
            "ble://AA:BB:CC:DD:EE:02" ..
            "?service=6e400001-b5a3-f393-e0a9-e50e24dcca9e" ..
            "&write=6e400002-b5a3-f393-e0a9-e50e24dcca9e" ..
            "&notify=6e400003-b5a3-f393-e0a9-e50e24dcca9e",
            connection.endpoint
        )
        local text = table.concat(rendered)
        assert.is_true(text:find("1. Uno A [pairing required]", 1, true) ~= nil)
        assert.is_true(text:find("2. Uno B [paired]", 1, true) ~= nil)
        assert.is_true(text:find("Select Bluetooth endpoint [1-2]: ", 1, true) ~= nil)
    end)

    it("classifies host devices through Rust-owned discovery rules", function()
        local devices = board_vm.devices({
            "/dev/cu.usbmodem1101",
            "/dev/tty.usbserial-CP2102-esp32",
            "/dev/serial/by-id/usb-Raspberry_Pi_Pico_E660-DAPLINK-if00",
        })
        local esp = nil
        local pico = nil
        for _, device in ipairs(devices) do
            if device.target and device.target.board_id == "esp32-devkit-v1" then
                esp = device
            end
            if device.target and device.target.board_id == "raspberry-pi-pico" then
                pico = device
            end
        end

        assert.are.equal(3, #devices)
        assert.are.equal("/dev/cu.usbmodem1101", devices[1].port)
        assert.is_nil(devices[1].target)

        assert.is_not_nil(esp)
        assert.are.equal("esp32-devkit-v1", esp.target.board_id)
        assert.is_true(esp.tags[2] == "uart" or esp.tags[3] == "uart")

        assert.is_not_nil(pico)
        assert.are.equal("raspberry-pi-pico", pico.target.board_id)
        assert.is_true(pico.bootloader)
    end)

    it("selects devices and connects without exposing serial details to callers", function()
        local devices = board_vm.devices({
            "/dev/cu.usbmodem1101",
            "/dev/tty.usbserial-CP2102-esp32",
        })
        local selected = board_vm.select_device("esp32", { devices = devices })
        local connection = board_vm.connect("esp32", { devices = devices })

        assert.are.equal("/dev/tty.usbserial-CP2102-esp32", selected.port)
        assert.are.equal("esp32-devkit-v1", connection:board_id())
        assert.are.equal("/dev/tty.usbserial-CP2102-esp32", connection.port)
        assert.are.equal("serial", connection:connection_transport())
        assert.is_true(connection:serial_connection())
        assert.is_false(connection:wireless_connection())
    end)

    it("connects to wireless transports through injected endpoints", function()
        local transport = {
            frames = {},
            write = function(self, frame)
                table.insert(self.frames, frame)
            end,
        }
        local connection = board_vm.connect("uno-r4-wifi", {
            via = "Wi-Fi",
            transport = transport,
        })
        local results = connection:smoke({ host_name = "lua-test", host_nonce = 0x1234 })

        assert.is_nil(connection.port)
        assert.are.equal("wifi", connection:connection_transport())
        assert.is_true(connection:wireless_connection())
        assert.is_true(connection:ota_connection())
        assert.are.equal(2, #results)
        assert.are.equal(2, #transport.frames)
        assert.is_string(transport.frames[1])
        assert.are.equal(0, transport.frames[1]:byte(#transport.frames[1]))
    end)

    it("auto-selects Bluetooth endpoint candidates while connecting", function()
        local connection = board_vm.connect("uno-r4-wifi", {
            via = "BLE",
            bluetooth_devices = {
                {
                    id = "uno-r4",
                    name = "Uno R4 Board VM",
                    address = "AA:BB:CC:DD:EE:FF",
                    service_uuids = { "6E400001-B5A3-F393-E0A9-E50E24DCCA9E" },
                },
            },
        })

        assert.is_nil(connection.port)
        assert.are.equal("bluetooth_le", connection:connection_transport())
        assert.are.equal(
            "ble://AA:BB:CC:DD:EE:FF" ..
            "?service=6e400001-b5a3-f393-e0a9-e50e24dcca9e" ..
            "&write=6e400002-b5a3-f393-e0a9-e50e24dcca9e" ..
            "&notify=6e400003-b5a3-f393-e0a9-e50e24dcca9e",
            connection.endpoint
        )
    end)

    it("builds Bluetooth transports from Rust backend plans", function()
        local backend = {
            endpoint = board_vm.bluetooth_endpoint("btspp://ESP32-BoardVM:3"),
            backend = "macos_rfcomm",
            status = "ready",
            stream_path = "/dev/cu.ESP32-BoardVM",
            message = nil,
        }
        local transport = board_vm.BluetoothTransport.new({
            endpoint = "btspp://ESP32-BoardVM:3",
            timeout_ms = 500,
            backend = backend,
        })

        assert.are.equal("btspp://ESP32-BoardVM:3", transport.endpoint)
        assert.are.equal("ready", transport:status())
        assert.are.equal("/dev/cu.ESP32-BoardVM", transport.stream_path)
        assert.are.equal("bluetooth_classic_rfcomm", transport.backend.endpoint.endpoint_transport)
    end)

    it("delegates native Bluetooth transactions to Rust", function()
        local endpoint = "ble://uno-r4-wifi/180f/2a19/2a1a"
        local calls = {}
        local original = board_vm.bluetooth_transact
        local ok, err = pcall(function()
            board_vm.bluetooth_transact = function(actual_endpoint, frame)
                table.insert(calls, { endpoint = actual_endpoint, frame = frame })
                return "response"
            end
            local transport = board_vm.BluetoothTransport.new({
                endpoint = endpoint,
                timeout_ms = 500,
                backend = {
                    endpoint = board_vm.bluetooth_endpoint(endpoint),
                    backend = "macos_core_bluetooth",
                    status = "ready",
                    stream_path = nil,
                    native_transport = true,
                    message = nil,
                },
            })

            assert.is_true(transport.native_transport)
            assert.are.equal("response", transport:transact("request"))
            assert.are.equal(endpoint, calls[1].endpoint)
            assert.are.equal("request", calls[1].frame)
            assert.has_error(function()
                transport:write("request")
            end, "native Board VM Bluetooth transport requires transact(frame, options)")
        end)
        board_vm.bluetooth_transact = original
        if not ok then
            error(err)
        end
    end)

    it("connects Bluetooth sessions through Rust backend plans", function()
        local connection = board_vm.connect("esp32", {
            via = "bluetooth_classic",
            bluetooth_devices = {
                {
                    id = "esp32-board-vm",
                    name = "ESP32 Board VM",
                    paired = true,
                    board_vm_rfcomm_channels = { 3 },
                },
            },
            bluetooth_backend_plan = {
                endpoint = board_vm.bluetooth_endpoint("btspp://esp32-board-vm:3"),
                backend = "macos_rfcomm",
                status = "ready",
                stream_path = "/dev/cu.ESP32-BoardVM",
                message = nil,
            },
        })
        local session = connection:session()

        assert.is_nil(connection.port)
        assert.are.equal("bluetooth_classic", connection:connection_transport())
        assert.are.equal("btspp://esp32-board-vm:3", connection.endpoint)
        assert.are.equal("macos_rfcomm", session.transport.backend.backend)
        assert.are.equal("/dev/cu.ESP32-BoardVM", session.transport.stream_path)
    end)

    it("builds TCP transports for Wi-Fi endpoints", function()
        local connection = board_vm.connect("uno-r4-wifi", {
            via = "Wi-Fi",
            endpoint = "tcp://board-vm.local:4170",
        })
        local session = connection:session()

        assert.is_nil(connection.port)
        assert.are.equal("wifi", connection:connection_transport())
        assert.are.equal("tcp://board-vm.local:4170", connection.endpoint)
        assert.are.equal("tcp://board-vm.local:4170", session.transport.endpoint)
        assert.are.equal("board-vm.local", session.transport.host)
        assert.are.equal(4170, session.transport.port)
    end)

    it("transacts over LuaSocket-compatible TCP endpoints", function()
        local sent = {}
        local response_index = 0
        local fake_client = {
            settimeout = function(self, timeout)
                self.timeout = timeout
            end,
            connect = function(self, host, port)
                self.host = host
                self.port = port
                return true
            end,
            setoption = function(self, option, value)
                self[option] = value
                return true
            end,
            send = function(_, frame)
                table.insert(sent, frame)
                return #frame
            end,
            receive = function(_, size)
                assert.are.equal(1, size)
                response_index = response_index + 1
                return ({ "\3", "\4", "\0" })[response_index]
            end,
            close = function(self)
                self.closed = true
            end,
        }
        local previous_socket = package.loaded.socket
        package.loaded.socket = {
            tcp = function()
                return fake_client
            end,
        }

        local ok, err = pcall(function()
            local transport = board_vm.TcpTransport.new({
                endpoint = "tcp://board-vm.local:4170",
                timeout_ms = 500,
            })
            local response = transport:transact("\1\2\0", { timeout_ms = 250 })

            assert.are.equal("\3\4\0", response)
            assert.are.equal("\1\2\0", sent[1])
            assert.are.equal("board-vm.local", fake_client.host)
            assert.are.equal(4170, fake_client.port)
            assert.is_true(fake_client["tcp-nodelay"])
            transport:close()
            assert.is_true(fake_client.closed)
        end)
        package.loaded.socket = previous_socket
        if not ok then
            error(err)
        end
    end)

    it("rejects dispatch when no Lua transport endpoint is available", function()
        local connection = board_vm.connect("uno-r4-wifi", { via = "Wi-Fi" })

        local ok, err = pcall(function()
            connection:smoke()
        end)

        assert.is_false(ok)
        assert.is_true(tostring(err):find("requires a Board VM TCP endpoint") ~= nil)
    end)

    it("builds blink upload/run frames through Rust-owned protocol builders", function()
        local session = board_vm.session({ next_request_id = 7, program_id = 9 })
        local frames = session:blink_upload_run_frames({
            pin = 13,
            high_ms = 125,
            low_ms = 250,
            max_stack = 4,
            instruction_budget = 777,
            time_budget_ms = 50,
        })

        assert.are.equal(4, #frames)
        assert.are.equal(11, session.next_request_id)
        for _, frame in ipairs(frames) do
            assert.is_string(frame)
            assert.is_true(#frame > 0)
            assert.are.equal(0, frame:byte(#frame))
        end
    end)

    it("builds HELLO and CAPS_QUERY frames with session request ids", function()
        local session = board_vm.session({ next_request_id = 3 })
        local hello = session:hello_wire("lua-host", 0x1234)
        local caps = session:caps_query_wire()

        assert.is_string(hello)
        assert.is_string(caps)
        assert.are.equal(5, session.next_request_id)
        assert.are.equal(0, hello:byte(#hello))
        assert.are.equal(0, caps:byte(#caps))
    end)
end)
