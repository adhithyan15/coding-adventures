# frozen_string_literal: true

require "socket"
require "test_helper"

module CodingAdventures
  module BoardVM
    class TestBoardVM < Minitest::Test
      def test_connect_yields_connection_without_flashing_by_default
        runner = FakeRunner.new
        yielded = nil

        connection = BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem1101",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner
        ) do |board|
          yielded = board
        end

        assert_same connection, yielded
        assert_equal :uno_r4_wifi, connection.board
        assert_equal "/dev/cu.usbmodem1101", connection.port
        assert_empty runner.calls
      end

      def test_known_targets_are_exposed_from_rust_registry
        targets = BoardVM.known_targets
        uno_r4_wifi = BoardVM.find_target("arduino-uno-r4-wifi")
        esp32 = BoardVM.find_target("esp32-devkit-v1")
        pico = BoardVM.find_target("raspberry-pi-pico")
        pico_w = BoardVM.find_target("raspberry-pi-pico-w")

        assert targets.any? { |target| target["board_id"] == "arduino-uno-r4-wifi" }
        assert_includes uno_r4_wifi["capabilities"], "transport.wifi"
        assert_includes uno_r4_wifi["capabilities"], "transport.bluetooth_le"
        assert_includes uno_r4_wifi["capabilities"], "led_matrix.frame"
        assert_includes uno_r4_wifi["capabilities"], "pwm.write"
        assert_includes uno_r4_wifi["capabilities"], "adc.read"
        assert_includes uno_r4_wifi["capabilities"], "dac.write_u12"
        assert_includes uno_r4_wifi["capabilities"], "i2c.open"
        assert_includes uno_r4_wifi["capabilities"], "i2c.write_u8"
        assert_includes uno_r4_wifi["capabilities"], "i2c.read_u8"
        assert_includes uno_r4_wifi["capabilities"], "i2c.write"
        assert_includes uno_r4_wifi["capabilities"], "i2c.read"
        assert_equal ["wifi", "bluetooth_le"], uno_r4_wifi["wireless"].map { |item| item["transport"] }
        assert uno_r4_wifi["wireless"].find { |item| item["transport"] == "wifi" }["ota_update"]
        assert_equal ["serial", "wifi", "bluetooth_le"], uno_r4_wifi["connection_options"].map { |item| item["transport"] }
        assert_equal ["wifi"], uno_r4_wifi["connection_options"].select { |item| item["ota_update"] }.map { |item| item["transport"] }
        assert_equal({ "rows" => 8, "columns" => 12 }, uno_r4_wifi["led_matrix"])
        assert_equal [
          {"bus" => 0, "name" => "Wire", "sda_pin" => 18, "scl_pin" => 19, "qwiic" => false, "notes" => "Header I2C bus on A4/SDA and A5/SCL"},
          {"bus" => 1, "name" => "Wire1", "sda_pin" => 27, "scl_pin" => 26, "qwiic" => true, "notes" => "UNO R4 WiFi Qwiic I2C bus"}
        ], uno_r4_wifi["i2c_buses"]
        assert_equal uno_r4_wifi["digital_pin_count"], uno_r4_wifi["digital_pins"].length
        d3 = uno_r4_wifi["digital_pins"].find { |pin| pin["pin"] == 3 }
        assert_equal "D3", d3["label"]
        assert d3["supports_input"]
        assert d3["supports_output"]
        assert d3["supports_pullup"]
        refute d3["supports_pulldown"]
        assert d3["supports_pwm"]
        assert d3["supports_interrupt"]
        refute d3["supports_adc"]
        a0 = uno_r4_wifi["digital_pins"].find { |pin| pin["pin"] == 14 }
        assert_equal "A0/D14", a0["label"]
        assert a0["supports_adc"]
        assert a0["supports_dac"]
        refute a0["supports_pwm"]
        assert_equal "esp32", esp32["family"]
        assert_equal "board-vm-esp32", esp32["runtime_id"]
        assert_equal({ "kind" => "gpio", "pin" => 2 }, esp32["onboard_led"])
        assert_nil esp32["led_matrix"]
        assert_includes esp32["capabilities"], "gpio.open"
        assert_includes esp32["capabilities"], "transport.bluetooth_classic"
        assert esp32["wireless"].all? { |item| item["command_transport"] }
        assert esp32["connection_options"].any? { |item| item["transport"] == "bluetooth_classic" && item["requires"] == "paired_device" }
        assert_equal [], pico["wireless"]
        assert_equal ["serial"], pico["connection_options"].map { |item| item["transport"] }
        refute_includes pico["capabilities"], "transport.wifi"
        assert_equal({ "kind" => "wireless_chip_gpio", "pin" => 0 }, pico_w["onboard_led"])
        assert_includes pico_w["capabilities"], "transport.wifi"
        assert_includes pico_w["capabilities"], "ota.wifi"
      end

      def test_connection_options_are_exposed_from_rust_registry
        options = BoardVM.connection_options(:uno_r4_wifi)

        assert_equal({
          "transport" => "serial",
          "display_name" => "USB/serial",
          "command_transport" => true,
          "ota_update" => false,
          "requires" => "serial_port",
          "endpoint_transport" => "serial_port",
          "endpoint_scheme" => "serial",
          "wire_protocol" => "board_vm_cobs_crc"
        }, options.first)
        assert_includes options, {
          "transport" => "wifi",
          "display_name" => "Wi-Fi",
          "command_transport" => true,
          "ota_update" => true,
          "requires" => "network_endpoint",
          "endpoint_transport" => "tcp_socket",
          "endpoint_scheme" => "tcp",
          "wire_protocol" => "board_vm_cobs_crc"
        }
      end

      def test_bluetooth_endpoints_are_parsed_by_rust_language_core
        ble = BoardVM.bluetooth_endpoint("ble://uno-r4-wifi/180f/2a19/2a1a")
        rfcomm = BoardVM.bluetooth_endpoint("btspp://ESP32-BoardVM:3")

        assert_equal "bluetooth_le", ble.fetch("transport")
        assert_equal "bluetooth_le_gatt", ble.fetch("endpoint_transport")
        assert_equal "ble", ble.fetch("endpoint_scheme")
        assert_equal "uno-r4-wifi", ble.fetch("device")
        assert_equal "180f", ble.fetch("service_uuid")
        assert_equal "2a19", ble.fetch("write_characteristic_uuid")
        assert_equal "2a1a", ble.fetch("notify_characteristic_uuid")
        assert_nil ble.fetch("channel")

        assert_equal "bluetooth_classic", rfcomm.fetch("transport")
        assert_equal "bluetooth_classic_rfcomm", rfcomm.fetch("endpoint_transport")
        assert_equal "btspp", rfcomm.fetch("endpoint_scheme")
        assert_equal "ESP32-BoardVM", rfcomm.fetch("device")
        assert_equal 3, rfcomm.fetch("channel")
        assert_nil BoardVM.bluetooth_endpoint("tcp://board-vm.local:4170")
      end

      def test_bluetooth_endpoint_candidates_are_planned_by_rust_language_core
        candidates = BoardVM.bluetooth_endpoint_candidates([
          {
            id: "ignore-me",
            service_uuids: ["180f"]
          },
          {
            id: "esp32-board-vm",
            name: "ESP32 Board VM",
            paired: true,
            board_vm_rfcomm_channels: [3, 3, 31]
          },
          {
            "id" => "uno-r4",
            "name" => "Uno R4 Board VM",
            "address" => "AA:BB:CC:DD:EE:FF",
            "service_uuids" => ["6E400001-B5A3-F393-E0A9-E50E24DCCA9E"]
          }
        ])

        assert_equal 2, candidates.length
        rfcomm = candidates.find { |candidate| candidate.fetch("endpoint").fetch("channel") == 3 }
        ble = candidates.find { |candidate| candidate.fetch("endpoint").fetch("service_uuid") }

        assert_equal "ESP32 Board VM", rfcomm.fetch("display_name")
        assert_equal "bluetooth_classic_rfcomm", rfcomm.fetch("endpoint").fetch("endpoint_transport")
        assert_equal "btspp://esp32-board-vm:3", rfcomm.fetch("endpoint").fetch("endpoint")
        assert rfcomm.fetch("paired")
        refute rfcomm.fetch("requires_pairing")

        assert_equal "Uno R4 Board VM", ble.fetch("display_name")
        assert_equal "bluetooth_le_gatt", ble.fetch("endpoint").fetch("endpoint_transport")
        assert_equal "AA:BB:CC:DD:EE:FF", ble.fetch("device")
        assert_equal "6e400001-b5a3-f393-e0a9-e50e24dcca9e", ble.fetch("endpoint").fetch("service_uuid")
        refute ble.fetch("paired")
        assert ble.fetch("requires_pairing")
      end

      def test_bluetooth_endpoint_candidates_default_to_host_discovery
        discovered = [{
          "id" => "esp32-board-vm",
          "name" => "ESP32 Board VM",
          "paired" => true,
          "board_vm_rfcomm_channels" => [3]
        }]

        BoardVM::Native.stub(:bluetooth_devices, discovered) do
          candidates = BoardVM.bluetooth_endpoint_candidates
          assert_equal 1, candidates.length
          assert_equal "btspp://esp32-board-vm:3", candidates.first.fetch("endpoint").fetch("endpoint")
        end
      end

      def test_bluetooth_connection_endpoint_filters_to_the_selected_transport
        option = BoardVM.select_connection_option(:uno_r4_wifi, transport: "BLE")

        endpoint = BoardVM.bluetooth_connection_endpoint(
          option,
          devices: [
            {
              "id" => "esp32-board-vm",
              "name" => "ESP32 Board VM",
              "paired" => true,
              "board_vm_rfcomm_channels" => [3]
            },
            {
              "id" => "uno-r4",
              "name" => "Uno R4 Board VM",
              "address" => "AA:BB:CC:DD:EE:FF",
              "service_uuids" => ["6E400001-B5A3-F393-E0A9-E50E24DCCA9E"]
            }
          ]
        )

        assert_equal "ble://AA:BB:CC:DD:EE:FF" \
          "?service=6e400001-b5a3-f393-e0a9-e50e24dcca9e" \
          "&write=6e400002-b5a3-f393-e0a9-e50e24dcca9e" \
          "&notify=6e400003-b5a3-f393-e0a9-e50e24dcca9e", endpoint
      end

      def test_connect_can_prompt_for_bluetooth_endpoint_with_pairing_status
        output = StringIO.new

        connection = BoardVM.uno_r4_wifi(
          via: "BLE",
          bluetooth_devices: [
            {
              "id" => "uno-a",
              "name" => "Uno A",
              "address" => "AA:BB:CC:DD:EE:01",
              "service_uuids" => ["6E400001-B5A3-F393-E0A9-E50E24DCCA9E"]
            },
            {
              "id" => "uno-b",
              "name" => "Uno B",
              "address" => "AA:BB:CC:DD:EE:02",
              "paired" => true,
              "service_uuids" => ["6E400001-B5A3-F393-E0A9-E50E24DCCA9E"]
            }
          ],
          pick_bluetooth_endpoint: true,
          input: StringIO.new("2\n"),
          output: output,
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new
        )

        assert_equal "bluetooth_le", connection.connection_transport
        assert_equal "ble://AA:BB:CC:DD:EE:02" \
          "?service=6e400001-b5a3-f393-e0a9-e50e24dcca9e" \
          "&write=6e400002-b5a3-f393-e0a9-e50e24dcca9e" \
          "&notify=6e400003-b5a3-f393-e0a9-e50e24dcca9e", connection.endpoint
        assert_includes output.string, "1. Uno A [pairing required]"
        assert_includes output.string, "2. Uno B [paired]"
        assert_includes output.string, "Select Bluetooth endpoint [1-2]: "
      end

      def test_connection_options_can_be_selected_without_exposing_ports
        default = BoardVM.select_connection_option(:uno_r4_wifi)
        wifi = BoardVM.select_connection_option(:uno_r4_wifi, transport: :wifi)
        friendly_wifi = BoardVM.select_connection_option(:uno_r4_wifi, transport: "Wi-Fi")
        friendly_serial = BoardVM.select_connection_option(:uno_r4_wifi, transport: "USB serial")
        ota = BoardVM.select_connection_option(:uno_r4_wifi, ota: true)

        assert_equal "serial", default.fetch("transport")
        assert_equal "wifi", wifi.fetch("transport")
        assert_equal "wifi", friendly_wifi.fetch("transport")
        assert_equal "serial", friendly_serial.fetch("transport")
        assert_equal "wifi", ota.fetch("transport")

        error = assert_raises(DeviceSelectionError) do
          BoardVM.select_connection_option(:raspberry_pi_pico, transport: :wifi)
        end
        assert_match(/No wifi connection option/, error.message)
        assert_includes error.message, "USB/serial"
      end

      def test_connection_option_picker_prompts_for_repl_use
        input = StringIO.new("2\n")
        output = StringIO.new

        option = BoardVM.pick_connection_option(:uno_r4_wifi, input: input, output: output)

        assert_equal "wifi", option.fetch("transport")
        assert_includes output.string, "1. USB/serial [commands] - requires serial_port"
        assert_includes output.string, "2. Wi-Fi [commands, OTA] - requires network_endpoint"
        assert_includes output.string, "Select connection [1-3]: "
      end

      def test_connect_records_the_rust_selected_serial_connection_option
        devices = BoardVM.devices(paths: ["/dev/tty.usbserial-CP2102-esp32"])

        connection = BoardVM.connect(
          board: :esp32,
          devices: devices,
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new
        )

        assert_equal "serial", connection.connection_transport
        assert_equal "USB/serial", connection.connection_option.fetch("display_name")
        assert connection.serial_connection?
        refute connection.wireless_connection?
      end

      def test_connect_can_use_a_wireless_connection_option_with_an_injected_endpoint
        transport = FakeWriteTransport.new

        connection = BoardVM.uno_r4_wifi(
          via: "Wi-Fi",
          smoke: true,
          transport: transport,
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new
        )

        assert_nil connection.port
        assert_equal "wifi", connection.connection_transport
        assert connection.wireless_connection?
        assert connection.ota_connection?
        assert_equal 2, transport.frames.length
      end

      def test_connect_auto_selects_bluetooth_endpoint_candidates
        connection = BoardVM.uno_r4_wifi(
          via: "BLE",
          bluetooth_devices: [
            {
              "id" => "uno-r4",
              "name" => "Uno R4 Board VM",
              "address" => "AA:BB:CC:DD:EE:FF",
              "service_uuids" => ["6E400001-B5A3-F393-E0A9-E50E24DCCA9E"]
            }
          ],
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new
        )

        assert_nil connection.port
        assert_equal "bluetooth_le", connection.connection_transport
        assert_equal "ble://AA:BB:CC:DD:EE:FF" \
          "?service=6e400001-b5a3-f393-e0a9-e50e24dcca9e" \
          "&write=6e400002-b5a3-f393-e0a9-e50e24dcca9e" \
          "&notify=6e400003-b5a3-f393-e0a9-e50e24dcca9e", connection.endpoint
      end

      def test_bluetooth_transport_uses_rust_backend_open_plan
        transport = BluetoothTransport.new(
          endpoint: "btspp://ESP32-BoardVM:3",
          timeout_ms: 500,
          backend: {
            "endpoint" => BoardVM.bluetooth_endpoint("btspp://ESP32-BoardVM:3"),
            "backend" => "macos_rfcomm",
            "status" => "ready",
            "stream_path" => "/dev/cu.ESP32-BoardVM",
            "message" => nil
          }
        )

        assert_equal "btspp://ESP32-BoardVM:3", transport.endpoint
        assert_equal "ready", transport.status
        assert_equal "/dev/cu.ESP32-BoardVM", transport.stream_path
        assert_equal "bluetooth_classic_rfcomm", transport.backend.fetch("endpoint").fetch("endpoint_transport")
      end

      def test_bluetooth_transport_delegates_native_transact_to_rust
        endpoint = "ble://uno-r4-wifi/180f/2a19/2a1a"
        calls = []
        original = BoardVM.method(:bluetooth_transact)
        verbose = $VERBOSE
        $VERBOSE = nil
        BoardVM.define_singleton_method(:bluetooth_transact) do |actual_endpoint, frame|
          calls << [actual_endpoint, frame]
          "response".b
        end

        transport = BluetoothTransport.new(
          endpoint: endpoint,
          timeout_ms: 500,
          backend: {
            "endpoint" => BoardVM.bluetooth_endpoint(endpoint),
            "backend" => "macos_core_bluetooth",
            "status" => "ready",
            "stream_path" => nil,
            "native_transport" => true,
            "message" => nil
          }
        )

        assert transport.native_transport?
        assert_equal "response".b, transport.transact("request".b)
        assert_equal [[endpoint, "request".b]], calls
        assert_raises(TransportError) { transport.write("request".b) }
      ensure
        BoardVM.define_singleton_method(:bluetooth_transact, original) if original
        $VERBOSE = verbose
      end

      def test_connect_builds_a_bluetooth_transport_from_rust_backend_plan
        connection = BoardVM.esp32(
          via: "bluetooth_classic",
          bluetooth_devices: [
            {
              "id" => "esp32-board-vm",
              "name" => "ESP32 Board VM",
              "paired" => true,
              "board_vm_rfcomm_channels" => [3]
            }
          ],
          bluetooth_backend_plan: {
            "endpoint" => BoardVM.bluetooth_endpoint("btspp://esp32-board-vm:3"),
            "backend" => "macos_rfcomm",
            "status" => "ready",
            "stream_path" => "/dev/cu.ESP32-BoardVM",
            "message" => nil
          },
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new
        )

        transport = connection.send(:active_transport)

        assert_nil connection.port
        assert_equal "bluetooth_classic", connection.connection_transport
        assert_equal "btspp://esp32-board-vm:3", connection.endpoint
        assert_instance_of BluetoothTransport, transport
        assert_equal "macos_rfcomm", transport.backend.fetch("backend")
        assert_equal "/dev/cu.ESP32-BoardVM", transport.stream_path
      end

      def test_connect_builds_a_tcp_transport_for_wifi_endpoints
        connection = BoardVM.uno_r4_wifi(
          via: "Wi-Fi",
          endpoint: "tcp://board-vm.local:4170",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new
        )

        assert_nil connection.port
        assert_equal "wifi", connection.connection_transport
        assert_equal "tcp://board-vm.local:4170", connection.endpoint

        transport = connection.send(:active_transport)
        assert_instance_of TcpTransport, transport
        assert_equal "tcp://board-vm.local:4170", transport.endpoint
      end

      def test_tcp_transport_transacts_with_a_local_endpoint
        server = TCPServer.new("127.0.0.1", 0)
        endpoint = "tcp://127.0.0.1:#{server.addr[1]}"
        request = "\x01\x02\x00".b
        response = "\x03\x04\x00".b

        thread = Thread.new do
          socket = server.accept
          frame = +"".b
          begin
            loop do
              byte = socket.readpartial(1)
              frame << byte
              break if byte == "\x00".b
            end
            socket.write(response)
            frame
          ensure
            socket.close
          end
        end

        transport = TcpTransport.new(endpoint: endpoint, timeout_ms: 500)

        assert_equal response, transport.transact(request, timeout_ms: 500)
        assert_equal request, thread.value
      ensure
        transport&.close
        server&.close
        thread&.kill if thread&.alive?
      end

      def test_connect_can_prompt_for_the_connection_option_after_the_board
        output = StringIO.new

        connection = BoardVM.uno_r4_wifi(
          pick_connection: true,
          input: StringIO.new("2\n"),
          output: output,
          transport: FakeWriteTransport.new,
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new
        )

        assert_equal "wifi", connection.connection_transport
        assert_nil connection.port
        assert_includes output.string, "Select connection [1-3]: "
      end

      def test_wireless_connection_requires_an_endpoint_before_dispatch
        connection = BoardVM.uno_r4_wifi(
          via: :wifi,
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new
        )

        error = assert_raises(TransportError) do
          connection.smoke!
        end
        assert_match(/requires a Board VM TCP endpoint/, error.message)
      end

      def test_targets_are_detected_from_rust_owned_aliases
        esp32 = BoardVM.detect_target("esp32")
        pico = BoardVM.detect_target("Raspberry Pi Pico")
        pico_w = BoardVM.find_target("pico-w")

        assert_equal "esp32-devkit-v1", esp32["board_id"]
        assert_equal "xtensa-esp32-none-elf", esp32["rust_target"]
        assert_equal "raspberry-pi-pico", pico["board_id"]
        assert_equal "raspberry-pi-pico-w", pico_w["board_id"]
        assert_nil BoardVM.detect_target("not-a-board")
        assert_equal :esp32_devkit_v1, BoardVM.normalize_board(:esp32)
        assert_equal :raspberry_pi_pico_w, BoardVM.normalize_board("pico-w")
      end

      def test_esp_upload_options_are_exposed_from_rust_language_core
        options = BoardVM.esp_upload_options(:esp32)

        assert_equal "esp32-devkit-v1", options["board_id"]
        assert_equal 115_200, options["baud_rate"]
        assert_equal 1_000, options["timeout_ms"]
        assert_equal true, options["reset_into_bootloader"]
        assert_equal 0x1000, options["offset"]
        assert_equal 0x400, options["block_size"]
        assert_equal 4 * 1024 * 1024, options["flash_size"]
        assert_equal true, options["verify_md5"]
        assert_equal false, options["stay_in_bootloader"]
        assert_nil BoardVM.esp_upload_options(:raspberry_pi_pico)

        overridden = BoardVM.esp_upload_options(:esp32, offset: 0x2000, verify_md5: false)
        assert_equal 0x2000, overridden["offset"]
        assert_equal false, overridden["verify_md5"]
      end

      def test_pico_uf2_upload_options_are_exposed_from_rust_language_core
        options = BoardVM.pico_uf2_upload_options(:pico)

        assert_equal "raspberry-pi-pico", options["board_id"]
        assert_equal "pico-uf2", options["command"]
        assert_equal "RPI-RP2", options["volume_label"]
        assert_equal ".uf2", options["image_extension"]
        assert_equal true, options["auto_detect_mount"]
        refute_nil BoardVM.pico_uf2_upload_options(:pico_w)
        assert_nil BoardVM.pico_uf2_upload_options(:esp32)
      end

      def test_pico_uf2_mounts_are_discovered_by_rust_language_core
        Dir.mktmpdir("board-vm-pico-uf2") do |root|
          mount = File.join(root, "RPI-RP2")
          Dir.mkdir(mount)
          File.write(File.join(mount, "INFO_UF2.TXT"), "UF2 Bootloader\nModel: Raspberry Pi RP2\n")
          File.write(File.join(mount, "INDEX.HTM"), "<html></html>")
          Dir.mkdir(File.join(root, "NOT-PICO"))

          assert_equal [mount], BoardVM.pico_uf2_mounts(roots: [root])
        end
      end

      def test_pico_uf2_mount_selects_single_discovered_mount
        Dir.mktmpdir("board-vm-pico-uf2") do |root|
          mount = File.join(root, "RPI-RP2")
          Dir.mkdir(mount)
          File.write(File.join(mount, "INFO_UF2.TXT"), "UF2 Bootloader\nModel: Raspberry Pi RP2\n")
          File.write(File.join(mount, "INDEX.HTM"), "<html></html>")

          assert_equal mount, BoardVM.pico_uf2_mount(roots: [root])
        end
      end

      def test_pico_uf2_mount_reports_multiple_discovered_mounts
        Dir.mktmpdir("board-vm-pico-uf2-a") do |root_a|
          Dir.mktmpdir("board-vm-pico-uf2-b") do |root_b|
            mount_a = File.join(root_a, "RPI-RP2")
            mount_b = File.join(root_b, "RPI-RP2")
            [mount_a, mount_b].each do |mount|
              Dir.mkdir(mount)
              File.write(File.join(mount, "INFO_UF2.TXT"), "UF2 Bootloader\nModel: Raspberry Pi RP2\n")
              File.write(File.join(mount, "INDEX.HTM"), "<html></html>")
            end

            error = assert_raises(DeviceSelectionError) do
              BoardVM.pico_uf2_mount(roots: [root_a, root_b])
            end
            assert_match(/Multiple Pico BOOTSEL UF2 mounts/, error.message)
            assert_includes error.message, mount_a
            assert_includes error.message, mount_b
          end
        end
      end

      def test_pico_uf2_upload_command_uses_rust_owned_options
        command = BoardVM.pico_uf2_upload_command(
          :pico,
          image: "/tmp/board-vm-pico.uf2",
          mount: "/Volumes/RPI-RP2"
        )

        assert_equal [
          "pico-uf2",
          "--image", "/tmp/board-vm-pico.uf2",
          "--mount", "/Volumes/RPI-RP2"
        ], command
      end

      def test_devices_are_discovered_and_classified_by_rust_language_core
        devices = BoardVM.devices(paths: [
          "/dev/cu.usbmodem1101",
          "/dev/tty.usbserial-CP2102-esp32",
          "/dev/serial/by-id/usb-Raspberry_Pi_Pico_E660-DAPLINK-if00"
        ])

        assert_equal "/dev/cu.usbmodem1101", devices.first["port"]
        assert_nil devices.first["target"]
        assert_includes devices.first["tags"], "usb_cdc"

        esp = devices.find { |device| device["port"].include?("usbserial") }
        assert_equal "esp32-devkit-v1", esp["target"]["board_id"]
        assert_includes esp["tags"], "uart"

        pico = devices.find { |device| device["port"].include?("Raspberry_Pi_Pico") }
        assert_equal "raspberry-pi-pico", pico["target"]["board_id"]
        assert_equal true, pico["bootloader"]

        rendered = BoardVM.device_list(devices)
        assert_includes rendered, "ESP32 DevKit V1"
        assert_includes rendered, "/dev/cu.usbmodem1101"
      end

      def test_runtime_device_selection_ignores_pico_bootloader_devices
        devices = BoardVM.devices(paths: [
          "/dev/serial/by-id/usb-Raspberry_Pi_Pico_E660-DAPLINK-if00",
          "/dev/serial/by-id/usb-Raspberry_Pi_Pico_Board_VM-if00"
        ])

        selected = BoardVM.select_runtime_device(board: :pico, devices: devices)

        assert_equal "/dev/serial/by-id/usb-Raspberry_Pi_Pico_Board_VM-if00", selected.fetch("port")
        refute selected.fetch("bootloader")
      end

      def test_board_specific_connect_can_select_the_only_device_without_a_port
        runner = FakeRunner.new
        devices = BoardVM.devices(paths: ["/dev/cu.usbmodem1101"])

        connection = BoardVM.uno_r4_wifi(
          devices: devices,
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner
        )

        assert_equal :uno_r4_wifi, connection.board
        assert_equal "/dev/cu.usbmodem1101", connection.port
        assert_empty runner.calls
      end

      def test_auto_connect_uses_a_confident_rust_classified_device
        runner = FakeRunner.new
        devices = BoardVM.devices(paths: ["/dev/tty.usbserial-CP2102-esp32"])

        connection = BoardVM.connect(
          devices: devices,
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner
        )

        assert_equal :esp32_devkit_v1, connection.board
        assert_equal "/dev/tty.usbserial-CP2102-esp32", connection.port
        assert_empty runner.calls
      end

      def test_auto_connect_displays_devices_when_board_is_ambiguous
        devices = BoardVM.devices(paths: [
          "/dev/cu.usbmodem1101",
          "/dev/cu.usbmodem2201"
        ])

        error = assert_raises(DeviceSelectionError) do
          BoardVM.connect(devices: devices, runner: FakeRunner.new)
        end

        assert_includes error.message, "Multiple Board VM devices found"
        assert_includes error.message, "/dev/cu.usbmodem1101"
        assert_includes error.message, "/dev/cu.usbmodem2201"
      end

      def test_pick_device_prompts_for_ambiguous_devices
        devices = BoardVM.devices(paths: [
          "/dev/cu.usbmodem1101",
          "/dev/cu.usbmodem2201"
        ])
        output = StringIO.new
        input = StringIO.new("2\n")

        selected = BoardVM.pick_device(devices: devices, input: input, output: output)

        assert_equal "/dev/cu.usbmodem2201", selected["port"]
        assert_includes output.string, "1. Unknown board"
        assert_includes output.string, "2. Unknown board"
        assert_includes output.string, "Select board [1-2]: "
      end

      def test_connect_can_use_interactive_picker_without_a_port
        runner = FakeRunner.new
        devices = BoardVM.devices(paths: [
          "/dev/cu.usbmodem1101",
          "/dev/tty.usbserial-CP2102-esp32"
        ])

        connection = BoardVM.connect(
          pick: true,
          devices: devices,
          input: StringIO.new("2\n"),
          output: StringIO.new,
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner
        )

        assert_equal :esp32_devkit_v1, connection.board
        assert_equal "/dev/tty.usbserial-CP2102-esp32", connection.port
        assert_empty runner.calls
      end

      def test_connect_flash_uploads_the_uno_r4_serialusb_vm_and_tracks_runtime_port
        upload = CommandResult.new(
          ["cargo"],
          "/repo/code/packages/rust",
          "Sketch uses 42000 bytes.\nNew upload port: /dev/cu.usbmodem2201 (serial)\n",
          "",
          0
        )
        runner = FakeRunner.new([upload])

        connection = BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem1101",
          flash: true,
          cargo_workspace: "/repo/code/packages/rust",
          arduino_core: "/arduino/core",
          arm_toolchain_bin: "/opt/arm/bin",
          bossac_path: "/tmp/bossa/bin",
          runner: runner
        )

        assert_equal "/dev/cu.usbmodem2201", connection.port
        assert_equal "/repo/code/packages/rust", runner.calls.first[:chdir]
        assert_equal [
          "cargo", "run",
          "-p", "board-vm-uno-r4-firmware",
          "--bin", "uno-r4-wifi-serialusb-artifact",
          "--",
          "--core", "/arduino/core",
          "--arm-toolchain-bin", "/opt/arm/bin",
          "--bossac-path", "/tmp/bossa/bin",
          "--port", "/dev/cu.usbmodem1101",
          "--baud", "115200",
          "--timeout-ms", "1000",
          "--upload"
        ], runner.calls.first[:argv]
      end

      def test_connect_flash_uploads_esp32_image_with_rust_owned_upload_options
        upload = CommandResult.new(
          ["cargo"],
          "/repo/code/packages/rust",
          "uploaded 4096 bytes\n",
          "",
          0
        )
        runner = FakeRunner.new([upload])

        connection = BoardVM.connect(
          board: :esp32,
          port: "/dev/cu.usbserial-110",
          flash: true,
          firmware_image: "/tmp/board-vm-esp32.bin",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          esp_upload_options: {
            offset: 0x2000,
            verify_md5: false,
            stay_in_bootloader: true
          }
        )

        assert_equal :esp32_devkit_v1, connection.board
        assert_equal "/repo/code/packages/rust", runner.calls.first[:chdir]
        assert_equal [
          "cargo", "run",
          "-p", "board-vm-cli",
          "--bin", "board-vm",
          "--",
          "esp-upload",
          "--port", "/dev/cu.usbserial-110",
          "--image", "/tmp/board-vm-esp32.bin",
          "--baud", "115200",
          "--timeout-ms", "1000",
          "--offset", "8192",
          "--block-size", "1024",
          "--flash-size", "4194304",
          "--no-verify",
          "--stay-in-bootloader"
        ], runner.calls.first[:argv]
      end

      def test_esp_upload_command_can_select_a_discovered_esp_device
        devices = BoardVM.devices(paths: [
          "/dev/cu.usbmodem1101",
          "/dev/tty.usbserial-CP2102-esp32"
        ])

        command = BoardVM.esp_upload_command(
          :esp32,
          devices: devices,
          image: "/tmp/board-vm-esp32.bin",
          offset: 0x2000,
          verify_md5: false
        )

        assert_equal [
          "esp-upload",
          "--port", "/dev/tty.usbserial-CP2102-esp32",
          "--image", "/tmp/board-vm-esp32.bin",
          "--baud", "115200",
          "--timeout-ms", "1000",
          "--offset", "8192",
          "--block-size", "1024",
          "--flash-size", "4194304",
          "--no-verify"
        ], command
      end

      def test_esp32_helper_flashes_the_selected_discovered_device
        upload = CommandResult.new(["cargo"], "/repo/code/packages/rust", "uploaded\n", "", 0)
        runner = FakeRunner.new([upload])
        devices = BoardVM.devices(paths: ["/dev/tty.usbserial-CP2102-esp32"])

        connection = BoardVM.esp32(
          devices: devices,
          flash: true,
          firmware_image: "/tmp/board-vm-esp32.bin",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner
        )

        assert_equal :esp32_devkit_v1, connection.board
        assert_equal "/dev/tty.usbserial-CP2102-esp32", connection.port
        assert_equal "/dev/tty.usbserial-CP2102-esp32", runner.calls.first[:argv][9]
      end

      def test_pico_helper_flashes_uf2_image_without_requiring_a_serial_port
        upload = CommandResult.new(["cargo"], "/repo/code/packages/rust", "copied uf2\n", "", 0)
        runner = FakeRunner.new([upload])
        runtime_devices = BoardVM.devices(paths: [
          "/dev/serial/by-id/usb-Raspberry_Pi_Pico_Board_VM-if00"
        ])

        connection = BoardVM.pico(
          flash: true,
          firmware_image: "/tmp/board-vm-pico.uf2",
          cargo_workspace: "/repo/code/packages/rust",
          pico_uf2_mount: "/Volumes/RPI-RP2",
          device_discovery: -> { runtime_devices },
          runner: runner
        )

        assert_equal :raspberry_pi_pico, connection.board
        assert_equal "/dev/serial/by-id/usb-Raspberry_Pi_Pico_Board_VM-if00", connection.port
        assert_equal "/repo/code/packages/rust", runner.calls.first[:chdir]
        assert_equal [
          "cargo", "run",
          "-p", "board-vm-cli",
          "--bin", "board-vm",
          "--",
          "pico-uf2",
          "--image", "/tmp/board-vm-pico.uf2",
          "--mount", "/Volumes/RPI-RP2"
        ], runner.calls.first[:argv]
      end

      def test_pico_flash_uses_auto_detected_bootsel_mount_by_default
        upload = CommandResult.new(["cargo"], "/repo/code/packages/rust", "copied uf2\n", "", 0)
        runner = FakeRunner.new([upload])
        runtime_devices = BoardVM.devices(paths: [
          "/dev/serial/by-id/usb-Raspberry_Pi_Pico_W_Board_VM-if00"
        ])

        Dir.mktmpdir("board-vm-pico-uf2") do |root|
          mount = File.join(root, "RPI-RP2")
          Dir.mkdir(mount)
          File.write(File.join(mount, "INFO_UF2.TXT"), "UF2 Bootloader\nModel: Raspberry Pi RP2\n")
          File.write(File.join(mount, "INDEX.HTM"), "<html></html>")

          connection = BoardVM.pico_w(
            flash: true,
            firmware_image: "/tmp/board-vm-pico-w.uf2",
            cargo_workspace: "/repo/code/packages/rust",
            pico_uf2_mount_roots: [root],
            device_discovery: -> { runtime_devices },
            runner: runner
          )

          assert_equal :raspberry_pi_pico_w, connection.board
          assert_equal "/dev/serial/by-id/usb-Raspberry_Pi_Pico_W_Board_VM-if00", connection.port
          assert_equal [
            "cargo", "run",
            "-p", "board-vm-cli",
            "--bin", "board-vm",
            "--",
            "pico-uf2",
            "--image", "/tmp/board-vm-pico-w.uf2",
            "--mount", mount
          ], runner.calls.first[:argv]
        end
      end

      def test_pico_flash_reports_missing_runtime_device_after_upload
        upload = CommandResult.new(["cargo"], "/repo/code/packages/rust", "copied uf2\n", "", 0)
        runner = FakeRunner.new([upload])

        error = assert_raises(DeviceSelectionError) do
          BoardVM.pico(
            flash: true,
            firmware_image: "/tmp/board-vm-pico.uf2",
            cargo_workspace: "/repo/code/packages/rust",
            pico_uf2_mount: "/Volumes/RPI-RP2",
            pico_runtime_port_wait_ms: 0,
            device_discovery: -> { [] },
            runner: runner
          )
        end

        assert_match(/Pico UF2 upload finished/, error.message)
        assert_equal 1, runner.calls.length
      end

      def test_pico_flash_can_immediately_smoke_rediscovered_runtime_port
        upload = CommandResult.new(["cargo"], "/repo/code/packages/rust", "copied uf2\n", "", 0)
        runner = FakeRunner.new([upload])
        transport = FakeWriteTransport.new
        runtime_devices = BoardVM.devices(paths: [
          "/dev/serial/by-id/usb-Raspberry_Pi_Pico_Board_VM-if00"
        ])

        connection = BoardVM.pico(
          flash: true,
          smoke: true,
          firmware_image: "/tmp/board-vm-pico.uf2",
          cargo_workspace: "/repo/code/packages/rust",
          pico_uf2_mount: "/Volumes/RPI-RP2",
          device_discovery: -> { runtime_devices },
          transport: transport,
          runner: runner
        )

        assert_equal :raspberry_pi_pico, connection.board
        assert_equal "/dev/serial/by-id/usb-Raspberry_Pi_Pico_Board_VM-if00", connection.port
        assert_equal 1, runner.calls.length
        assert_equal 2, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
      end

      def test_esp_upload_command_rejects_non_esp_targets
        error = assert_raises(UnsupportedBoardError) do
          BoardVM.esp_upload_command(:raspberry_pi_pico, port: "/dev/cu.usbmodem", image: "fw.bin")
        end

        assert_match(/ESP upload is not supported/, error.message)
      end

      def test_led_blink_dispatches_native_protocol_frames_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport,
          baud: 57_600,
          timeout_ms: 250
        ) do |board|
          result = board.led.blink(program_id: 9, budget: 32, host_nonce: 123)
        end

        assert_empty runner.calls
        assert_equal 6, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal transport.frames, result.frames
        assert_equal Array.new(6), result.responses
        assert_equal Array.new(6), result.decoded_responses
        assert_equal [:hello, :capabilities, :program_begin, :program_chunk, :program_end, :run],
          result.results.map(&:command)
      end

      def test_led_blink_tolerates_missing_background_run_response
        runner = FakeRunner.new
        transport = FakeTimeoutTransport.new(timeout_at: 6)
        result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport,
          timeout_ms: 5_000
        ) do |board|
          result = board.led.blink(high_ms: 100, low_ms: 125, budget: 40)
        end

        assert_empty runner.calls
        assert_equal 6, transport.frames.length
        assert_equal [:hello, :capabilities, :program_begin, :program_chunk, :program_end, :run],
          result.results.map(&:command)
        assert_equal 725, transport.timeout_values.last
        assert_nil result.results.last.response
        assert_nil result.results.last.decoded_response
        assert transport.closed?
      end

      def test_direct_session_run_still_raises_transport_timeout
        runner = FakeRunner.new
        transport = FakeTimeoutTransport.new(timeout_at: 1)

        error = assert_raises(TransportError) do
          BoardVM.uno_r4_wifi(
            port: "/dev/cu.usbmodem2201",
            cargo_workspace: "/repo/code/packages/rust",
            runner: runner,
            transport: transport,
            timeout_ms: 250
          ) do |board|
            board.session { |vm| vm.run(budget: 24) }
          end
        end

        assert_match(/timed out waiting for Board VM response/, error.message)
        refute transport.closed?
      end

      def test_session_stop_accepts_per_call_timeout
        runner = FakeRunner.new
        transport = FakeTransactTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport,
          timeout_ms: 250
        ) do |board|
          board.session.stop(timeout_ms: 10_000)
        end

        assert_equal [10_000], transport.timeout_values
      end

      def test_session_capabilities_uses_hardened_default_timeout
        runner = FakeRunner.new
        transport = FakeTransactTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport,
          timeout_ms: 250
        ) do |board|
          board.session.capabilities
        end

        assert_equal [BoardVM::DEFAULT_CAPABILITIES_TIMEOUT_MS], transport.timeout_values
      end

      def test_session_capabilities_honors_explicit_timeout
        runner = FakeRunner.new
        transport = FakeTransactTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport,
          timeout_ms: 250
        ) do |board|
          board.session.capabilities(timeout_ms: 750)
        end

        assert_equal [750], transport.timeout_values
      end

      def test_dispatch_protocol_frame_skips_stale_responses_until_request_and_kind_match
        runner = FakeRunner.new
        transport = FakeReadAfterTransactTransport.new(["stale-id", "stale-kind", "matched"])
        native_session = FakeDecodedSession.new(
          "stale-id" => {"request_id" => 6, "kind" => "run_report"},
          "stale-kind" => {"request_id" => 1, "kind" => "run_report"},
          "matched" => {"request_id" => 1, "kind" => "hello_ack"}
        )

        connection = BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport,
          timeout_ms: 250
        )

        response, decoded_response = connection.dispatch_protocol_frame(
          "hello-frame",
          native_session: native_session,
          expected_request_id: 1,
          expected_response_kind: "hello_ack"
        )

        assert_equal "matched", response
        assert_equal({"request_id" => 1, "kind" => "hello_ack"}, decoded_response)
        assert_equal ["hello-frame"], transport.frames
        assert_equal [250], transport.timeout_values
        assert_equal [250, 250], transport.read_timeout_values
      end

      def test_dispatch_protocol_frame_returns_matching_error_response_without_waiting
        runner = FakeRunner.new
        transport = FakeReadAfterTransactTransport.new(["error-response"])
        native_session = FakeDecodedSession.new(
          "error-response" => {"request_id" => 1, "kind" => "error", "error" => true}
        )

        connection = BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport,
          timeout_ms: 250
        )

        response, decoded_response = connection.dispatch_protocol_frame(
          "run-frame",
          native_session: native_session,
          expected_request_id: 1,
          expected_response_kind: "run_report"
        )

        assert_equal "error-response", response
        assert_equal({"request_id" => 1, "kind" => "error", "error" => true}, decoded_response)
        assert_equal ["run-frame"], transport.frames
        assert_equal [250], transport.timeout_values
        assert_empty transport.read_timeout_values
      end

      def test_time_now_dispatches_native_protocol_frames_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          result = board.time.now_ms(program_id: 10, budget: 24)
        end

        assert_empty runner.calls
        assert_equal 6, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal transport.frames, result.frames
        assert_equal [:hello, :capabilities, :program_begin, :program_chunk, :program_end, :run],
          result.results.map(&:command)
      end

      def test_time_sleep_ms_dispatches_native_protocol_frames_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          result = board.time.sleep_ms(250, program_id: 10, budget: 24)
        end

        assert_empty runner.calls
        assert_equal 6, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal transport.frames, result.frames
        assert_equal [:hello, :capabilities, :program_begin, :program_chunk, :program_end, :run],
          result.results.map(&:command)
      end

      def test_led_matrix_frame_dispatches_native_protocol_frames_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          result = board.led_matrix.heart(program_id: 10, budget: 24)
        end

        assert_empty runner.calls
        assert_equal 6, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal transport.frames, result.frames
        assert_equal [:hello, :capabilities, :program_begin, :program_chunk, :program_end, :run],
          result.results.map(&:command)
      end

      def test_pwm_write_dispatches_native_protocol_frames_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          result = board.pwm.write(pin: 3, duty: 0x8000, program_id: 10, budget: 24)
        end

        assert_empty runner.calls
        assert_equal 6, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal transport.frames, result.frames
        assert_equal [:hello, :capabilities, :program_begin, :program_chunk, :program_end, :run],
          result.results.map(&:command)
      end

      def test_adc_read_dispatches_native_protocol_frames_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          result = board.adc.read(pin: 14, program_id: 10, budget: 24)
        end

        assert_empty runner.calls
        assert_equal 6, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal transport.frames, result.frames
        assert_equal [:hello, :capabilities, :program_begin, :program_chunk, :program_end, :run],
          result.results.map(&:command)
      end

      def test_dac_write_dispatches_native_protocol_frames_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          result = board.dac.write(pin: 14, sample: 0x0800, program_id: 10, budget: 24)
        end

        assert_empty runner.calls
        assert_equal 6, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal transport.frames, result.frames
        assert_equal [:hello, :capabilities, :program_begin, :program_chunk, :program_end, :run],
          result.results.map(&:command)
      end

      def test_i2c_open_dispatches_native_protocol_frames_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          result = board.i2c.open(bus: 0, program_id: 10, budget: 24)
        end

        assert_empty runner.calls
        assert_equal 6, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal transport.frames, result.frames
        assert_equal [:hello, :capabilities, :program_begin, :program_chunk, :program_end, :run],
          result.results.map(&:command)
      end

      def test_i2c_write_u8_dispatches_native_protocol_frames_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        open_result = nil
        write_result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          open_result = board.i2c.open(bus: 0, program_id: 10, budget: 24)
          write_result = board.i2c.write_u8(address: 0x3c, byte: 0xa5, program_id: 11, budget: 24)
        end

        assert_empty runner.calls
        assert_equal 10, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal open_result.frames + write_result.frames, transport.frames
        assert_equal [:program_begin, :program_chunk, :program_end, :run],
          write_result.results.map(&:command)
      end

      def test_i2c_write_dispatches_native_protocol_frames_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        open_result = nil
        write_result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          open_result = board.i2c.open(bus: 0, program_id: 10, budget: 24)
          write_result = board.i2c.write(address: 0x3c, bytes: [0xde, 0xad, 0xbe], program_id: 11, budget: 24)
        end

        assert_empty runner.calls
        assert_equal 10, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal open_result.frames + write_result.frames, transport.frames
        assert_equal [:program_begin, :program_chunk, :program_end, :run],
          write_result.results.map(&:command)
      end

      def test_i2c_read_u8_dispatches_native_protocol_frames_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        open_result = nil
        read_result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          open_result = board.i2c.open(bus: 0, program_id: 10, budget: 24)
          read_result = board.i2c.read_u8(address: 0x3c, program_id: 11, budget: 24)
        end

        assert_empty runner.calls
        assert_equal 10, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal open_result.frames + read_result.frames, transport.frames
        assert_equal [:program_begin, :program_chunk, :program_end, :run],
          read_result.results.map(&:command)
      end

      def test_i2c_read_dispatches_native_protocol_frames_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        open_result = nil
        read_result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          open_result = board.i2c.open(bus: 0, program_id: 10, budget: 24)
          read_result = board.i2c.read(address: 0x3c, length: 3, program_id: 11, budget: 24)
        end

        assert_empty runner.calls
        assert_equal 10, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal open_result.frames + read_result.frames, transport.frames
        assert_equal [:program_begin, :program_chunk, :program_end, :run],
          read_result.results.map(&:command)
      end

      def test_store_program_dispatches_native_protocol_frame_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          result = board.store_program!(program_id: 10, slot: 2, boot_policy: :run_at_boot)
        end

        assert_empty runner.calls
        assert_equal 1, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal transport.frames.first, result.frame
        assert_equal :store_program, result.command
      end

      def test_gpio_read_dispatches_native_protocol_frames_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          result = board.gpio.read(pin: 13, mode: :pullup, program_id: 10, budget: 24)
        end

        assert_empty runner.calls
        assert_equal 6, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal transport.frames, result.frames
        assert_equal [:hello, :capabilities, :program_begin, :program_chunk, :program_end, :run],
          result.results.map(&:command)
      end

      def test_gpio_write_dispatches_native_protocol_frames_through_transport
        runner = FakeRunner.new
        transport = FakeWriteTransport.new
        result = nil

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          result = board.gpio.write(pin: 13, value: :high, program_id: 11, budget: 24)
        end

        assert_empty runner.calls
        assert_equal 6, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
        assert_equal transport.frames, result.frames
        assert_equal [:hello, :capabilities, :program_begin, :program_chunk, :program_end, :run],
          result.results.map(&:command)
      end

      def test_session_surface_dispatches_protocol_commands_with_native_frames
        runner = FakeRunner.new
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          transport: transport
        ) do |board|
          board.session do |session|
            hello = session.hello(host_nonce: 99)
            caps = session.capabilities
            upload = session.upload_blink(program_id: 4)
            time_upload = session.upload_time_now(program_id: 5)
            sleep_upload = session.upload_time_sleep_ms(program_id: 8, duration_ms: 250)
            gpio_upload = session.upload_gpio_read(program_id: 6, pin: 13, mode: :pullup)
            gpio_write_upload = session.upload_gpio_write(program_id: 7, pin: 13, value: true)
            gpio_open_upload = session.upload_gpio_open(program_id: 10, pin: 13, mode: :output)
            gpio_handle_read_upload = session.upload_gpio_handle_read(program_id: 11)
            gpio_handle_write_upload = session.upload_gpio_handle_write(program_id: 12, value: true)
            gpio_handle_close_upload = session.upload_gpio_handle_close(program_id: 13)
            led_matrix_upload = session.upload_led_matrix_frame(program_id: 14, preset: :heart)
            store = session.store_program(program_id: 4, slot: 2, boot_policy: :run_at_boot)
            raw_module = session.raw_module(code: "\x00".b, max_stack: 1, const_pool: "\xAA\x55".b)
            raw_upload = session.upload_raw_module(
              program_id: 9,
              code: "\x00".b,
              max_stack: 1,
              const_pool: "\xAA\x55".b
            )
            run = session.run(program_id: 4, budget: 77)
            run_with_handles = session.run(
              program_id: 4,
              budget: 77,
              keep_handles: true,
              background: false,
              time_budget_ms: 250
            )
            stop = session.stop

            assert_equal :hello, hello.command
            assert_equal :capabilities, caps.command
            assert_equal [:program_begin, :program_chunk, :program_end], upload.results.map(&:command)
            assert_equal [:program_begin, :program_chunk, :program_end],
              time_upload.results.map(&:command)
            assert_equal [:program_begin, :program_chunk, :program_end],
              sleep_upload.results.map(&:command)
            assert_equal [:program_begin, :program_chunk, :program_end],
              gpio_upload.results.map(&:command)
            assert_equal [:program_begin, :program_chunk, :program_end],
              gpio_write_upload.results.map(&:command)
            assert_equal [:program_begin, :program_chunk, :program_end],
              gpio_open_upload.results.map(&:command)
            assert_equal [:program_begin, :program_chunk, :program_end],
              gpio_handle_read_upload.results.map(&:command)
            assert_equal [:program_begin, :program_chunk, :program_end],
              gpio_handle_write_upload.results.map(&:command)
            assert_equal [:program_begin, :program_chunk, :program_end],
              gpio_handle_close_upload.results.map(&:command)
            assert_equal [:program_begin, :program_chunk, :program_end],
              led_matrix_upload.results.map(&:command)
            assert_equal :store_program, store.command
            assert raw_module.start_with?("BVM1")
            assert raw_module.end_with?("\xAA\x55".b)
            assert_equal [:program_begin, :program_chunk, :program_end],
              raw_upload.results.map(&:command)
            assert_equal :run, run.command
            assert_equal :run, run_with_handles.command
            assert_equal :stop, stop.command
          end
        end

        assert_empty runner.calls
        assert_equal 39, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
      end

      def test_session_run_command_accepts_repl_style_stop
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("stop")

          assert_equal [:stop], result.results.map(&:command)
          assert_equal result.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_smoke
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("smoke")

          assert_equal [:hello, :capabilities], result.results.map(&:command)
          assert_equal result.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_store_program
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("store-program 9 2 run-at-boot")

          assert_equal [:store_program], result.results.map(&:command)
          assert_equal result.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_blink
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("blink 24", program_id: 8)

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            result.results.map(&:command)
          assert_equal result.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_gpio_read
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("gpio-read 13 pullup 24", program_id: 9)

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            result.results.map(&:command)
          assert_equal result.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_gpio_write_and_levels
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          write = board.session.run_command("gpio-write 13 high 24", program_id: 9)
          high = board.session.run_command("gpio-high 13 24", program_id: 10)
          low = board.session.run_command("gpio-low 13 24", program_id: 11)

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            write.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            high.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            low.results.map(&:command)
          assert_equal write.frames + high.frames + low.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_gpio_handle_commands
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          open = board.session.run_command("gpio-open 13 output 24", program_id: 9)
          read = board.session.run_command("gpio-handle-read 24", program_id: 10)
          write = board.session.run_command("gpio-handle-write high 24", program_id: 11)
          close = board.session.run_command("gpio-handle-close 24", program_id: 12)

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            open.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            read.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            write.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            close.results.map(&:command)
          assert_equal open.frames + read.frames + write.frames + close.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_time_now
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("time-now 24", program_id: 9)

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            result.results.map(&:command)
          assert_equal result.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_time_sleep_ms
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("time-sleep-ms 250 24", program_id: 9)
          upload = board.session.run_command("upload-time-sleep-ms 125", program_id: 10)

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            result.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end],
            upload.results.map(&:command)
          assert_equal result.frames + upload.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_led_matrix_frame
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("matrix-frame heart 24", program_id: 9)
          upload = board.session.run_command(
            "upload-matrix-frame 0x3184a444 0x44042081 0x100a0040",
            program_id: 10
          )

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            result.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end],
            upload.results.map(&:command)
          assert_equal result.frames + upload.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_pwm_write
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("pwm-write 3 0x8000 24", program_id: 9)
          upload = board.session.run_command("upload-pwm-write 3 32768", program_id: 10)

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            result.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end],
            upload.results.map(&:command)
          assert_equal result.frames + upload.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_adc_read
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("adc-read 14 24", program_id: 9)
          upload = board.session.run_command("upload-adc-read 14", program_id: 10)

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            result.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end],
            upload.results.map(&:command)
          assert_equal result.frames + upload.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_dac_write
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("dac-write-u12 14 0x800 24", program_id: 9)
          upload = board.session.run_command("upload-dac-write-u12 14 2048", program_id: 10)

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            result.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end],
            upload.results.map(&:command)
          assert_equal result.frames + upload.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_i2c_open
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("i2c-open 0 24", program_id: 9)
          upload = board.session.run_command("upload-i2c-open 1", program_id: 10)

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            result.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end],
            upload.results.map(&:command)
          assert_equal result.frames + upload.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_i2c_write_u8
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("i2c-write-u8 60 165 24", program_id: 9)
          upload = board.session.run_command("upload-i2c-write-u8 60 165", program_id: 10)

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            result.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end],
            upload.results.map(&:command)
          assert_equal result.frames + upload.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_i2c_write
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("i2c.write 60 0xdeadbe 24", program_id: 9)
          upload = board.session.run_command("upload-i2c.write 60 0xdeadbe", program_id: 10)

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            result.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end],
            upload.results.map(&:command)
          assert_equal result.frames + upload.frames, transport.frames
        end
      end

      def test_session_run_command_rejects_invalid_i2c_write_hex_payload
        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: FakeWriteTransport.new
        ) do |board|
          error = assert_raises(ArgumentError) do
            board.session.run_command("upload-i2c.write 60 0xzz", program_id: 10)
          end
          assert_match(/hex payload must contain only hex digits/, error.message)
        end
      end

      def test_session_run_command_accepts_repl_style_i2c_read_u8
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("i2c-read-u8 60 24", program_id: 9)
          upload = board.session.run_command("upload-i2c-read-u8 60", program_id: 10)

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            result.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end],
            upload.results.map(&:command)
          assert_equal result.frames + upload.frames, transport.frames
        end
      end

      def test_session_run_command_accepts_repl_style_i2c_read
        transport = FakeWriteTransport.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: transport
        ) do |board|
          result = board.session.run_command("i2c.read 60 3 24", program_id: 9)
          upload = board.session.run_command("upload-i2c.read 60 3", program_id: 10)

          assert_equal [:program_begin, :program_chunk, :program_end, :run],
            result.results.map(&:command)
          assert_equal [:program_begin, :program_chunk, :program_end],
            upload.results.map(&:command)
          assert_equal result.frames + upload.frames, transport.frames
        end
      end

      def test_session_run_command_rejects_oversized_i2c_read_length
        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: FakeRunner.new,
          transport: FakeWriteTransport.new
        ) do |board|
          error = assert_raises(ArgumentError) do
            board.session.run_command("upload-i2c.read 60 33", program_id: 10)
          end
          assert_match(/I2C read length must be at most 32 bytes/, error.message)
        end
      end

      def test_board_descriptor_wraps_rust_decoded_capability_report
        decoded = {
          "kind" => "caps_report",
          "payload" => {
            "board_id" => "arduino-uno-r4-wifi",
            "runtime_id" => "board-vm-uno-r4",
            "max_program_bytes" => 1024,
            "max_stack_values" => 16,
            "max_handles" => 4,
            "supports_store_program" => false,
            "capabilities" => [
              {
                "id" => 1,
                "version" => 1,
                "flags" => 1,
                "name" => "gpio.open",
                "bytecode_callable" => true,
                "protocol_feature" => false,
                "board_metadata" => false,
                "flag_names" => ["bytecode_callable"]
              },
              {
                "id" => 0x7001,
                "version" => 1,
                "flags" => 2,
                "name" => "program.ram_exec",
                "bytecode_callable" => false,
                "protocol_feature" => true,
                "board_metadata" => false,
                "flag_names" => ["protocol_feature"]
              }
            ]
          }
        }

        descriptor = ProtocolResult.new(decoded_response: decoded).board_descriptor

        assert_equal "arduino-uno-r4-wifi", descriptor.board_id
        assert_equal "board-vm-uno-r4", descriptor.runtime_id
        assert_equal ["gpio.open", "program.ram_exec"], descriptor.capability_names
        assert descriptor.supports?("gpio.open")
        assert descriptor.supports?(0x7001)
        assert descriptor["gpio.open"].bytecode_callable?
        assert descriptor["program.ram_exec"].protocol_feature?
        refute descriptor.store_program?
        assert_equal ["gpio.open"], descriptor.gpio.map(&:name)
        assert_equal ["program.ram_exec"], descriptor.program.map(&:name)
      end

      def test_session_result_delegates_kind_and_payload_to_last_result
        result = SessionResult.new(results: [
          ProtocolResult.new(decoded_response: {"kind" => "hello_ack", "payload" => {"runtime_name" => "board-vm"}}),
          ProtocolResult.new(decoded_response: {"kind" => "run_report", "payload" => {"status" => "halted"}})
        ])

        assert_equal "run_report", result.kind
        assert_equal({"status" => "halted"}, result.payload)
      end

      def test_led_matrix_pattern_packs_to_uno_r4_frame_words
        pattern = <<~PATTERN
          ..##....##..
          ..##....##..
          ............
          #..........#
          .#........#.
          ..#......#..
          ...######...
          ............
        PATTERN

        words = Session.led_matrix_words(pattern: pattern)

        assert_equal [0x30C3_0C00, 0x0801_4022, 0x041F_8000], words
      end

      def test_native_session_builds_protocol_bytes_in_rust
        session = BoardVM::Native::Session.new

        hello = session.hello_wire("bvm", 0x1234_ABCD)
        assert_instance_of String, hello
        assert_operator hello.bytesize, :>, 0
        assert_equal 2, session.next_request_id

        default_nonce_hello = session.hello_wire("bvm", BoardVM::DEFAULT_HOST_NONCE)
        assert_operator default_nonce_hello.bytesize, :>, 0
        assert_equal 3, session.next_request_id

        module_bytes = session.blink_module(13, 250, 250, 4)
        assert_instance_of String, module_bytes
        assert_operator module_bytes.bytesize, :>, 0

        time_module_bytes = session.time_now_module(1)
        assert_instance_of String, time_module_bytes
        assert_operator time_module_bytes.bytesize, :>, 0

        sleep_module_bytes = session.time_sleep_ms_module(250, 1)
        assert_instance_of String, sleep_module_bytes
        assert_operator sleep_module_bytes.bytesize, :>, 0

        led_matrix_module_bytes = session.led_matrix_frame_module(
          0x3184_A444,
          0x4404_2081,
          0x100A_0040,
          3
        )
        assert_instance_of String, led_matrix_module_bytes
        assert_operator led_matrix_module_bytes.bytesize, :>, 0

        pwm_module_bytes = session.pwm_write_module(3, 0x8000, 2)
        assert_instance_of String, pwm_module_bytes
        assert_operator pwm_module_bytes.bytesize, :>, 0

        adc_module_bytes = session.adc_read_module(14, 1)
        assert_instance_of String, adc_module_bytes
        assert_operator adc_module_bytes.bytesize, :>, 0

        dac_module_bytes = session.dac_write_u12_module(14, 0x0800, 2)
        assert_instance_of String, dac_module_bytes
        assert_operator dac_module_bytes.bytesize, :>, 0

        i2c_module_bytes = session.i2c_open_module(0, 2)
        assert_instance_of String, i2c_module_bytes
        assert_operator i2c_module_bytes.bytesize, :>, 0

        i2c_write_module_bytes = session.i2c_write_u8_module(0x3c, 0xa5, 4)
        assert_instance_of String, i2c_write_module_bytes
        assert_operator i2c_write_module_bytes.bytesize, :>, 0

        i2c_write_bytes_module_bytes = session.i2c_write_module(0x3c, "\xDE\xAD\xBE".b, 4)
        assert_instance_of String, i2c_write_bytes_module_bytes
        assert_operator i2c_write_bytes_module_bytes.bytesize, :>, 0

        i2c_read_module_bytes = session.i2c_read_u8_module(0x3c, 3)
        assert_instance_of String, i2c_read_module_bytes
        assert_operator i2c_read_module_bytes.bytesize, :>, 0

        i2c_read_bytes_module_bytes = session.i2c_read_module(0x3c, 3, 4)
        assert_instance_of String, i2c_read_bytes_module_bytes
        assert_operator i2c_read_bytes_module_bytes.bytesize, :>, 0

        gpio_module_bytes = session.gpio_read_module(13, 2, 2)
        assert_instance_of String, gpio_module_bytes
        assert_operator gpio_module_bytes.bytesize, :>, 0

        gpio_write_module_bytes = session.gpio_write_module(13, 1, 3)
        assert_instance_of String, gpio_write_module_bytes
        assert_operator gpio_write_module_bytes.bytesize, :>, 0

        gpio_open_module_bytes = session.gpio_open_module(13, 1, 2)
        assert_instance_of String, gpio_open_module_bytes
        assert_operator gpio_open_module_bytes.bytesize, :>, 0

        gpio_handle_read_module_bytes = session.gpio_handle_read_module(2)
        assert_instance_of String, gpio_handle_read_module_bytes
        assert_operator gpio_handle_read_module_bytes.bytesize, :>, 0

        gpio_handle_write_module_bytes = session.gpio_handle_write_module(1, 3)
        assert_instance_of String, gpio_handle_write_module_bytes
        assert_operator gpio_handle_write_module_bytes.bytesize, :>, 0

        gpio_handle_close_module_bytes = session.gpio_handle_close_module(1)
        assert_instance_of String, gpio_handle_close_module_bytes
        assert_operator gpio_handle_close_module_bytes.bytesize, :>, 0

        store = session.store_program_wire(7, 2, 1)
        assert_instance_of String, store
        assert_operator store.bytesize, :>, 0
        assert_equal 4, session.next_request_id

        run = session.run_wire(7, BoardVM::Session::RUN_FLAG_KEEP_HANDLES_AFTER_RUN, 77, 250)
        assert_instance_of String, run
        assert_operator run.bytesize, :>, 0
        assert_equal 5, session.next_request_id

        stop = session.stop_wire
        assert_instance_of String, stop
        assert_operator stop.bytesize, :>, 0
        assert_equal 6, session.next_request_id

        frames = BoardVM::Native::Session.new.blink_upload_run_frames(7, 12, 13, 250, 250, 4)
        assert_equal 4, frames.length
        assert frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
      end

      def test_eject_blink_writes_a_board_agnostic_artifact
        runner = FakeRunner.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner
        ) do |board|
          board.eject.blink(to: "/tmp/ejected_blink.rs", slot: 2, boot_policy: :run_at_boot)
        end

        assert_equal [
          "cargo", "run",
          "-p", "board-vm-cli",
          "--bin", "board-vm",
          "--",
          "eject", "blink",
          "--out", "/tmp/ejected_blink.rs",
          "--program-id", "1",
          "--slot", "2",
          "--boot-policy", "run-at-boot"
        ], runner.calls.first[:argv]
      end

      def test_parse_new_upload_port_prefers_the_last_reported_port
        output = "New upload port: /dev/cu.usbmodemBOOT (serial)\n" \
          "New upload port: /dev/cu.usbmodemRUNTIME (serial)\n"

        assert_equal "/dev/cu.usbmodemRUNTIME", Connection.parse_new_upload_port(output)
      end

      def test_rejects_unknown_boards_before_running_commands
        runner = FakeRunner.new

        assert_raises(UnsupportedBoardError) do
          BoardVM.connect(board: :unknown_board, port: "/dev/cu.usbserial", runner: runner)
        end

        assert_empty runner.calls
      end
    end
  end
end
