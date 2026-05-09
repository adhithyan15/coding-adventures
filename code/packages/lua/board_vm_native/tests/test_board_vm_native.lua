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
        local esp_upload = board_vm.esp_upload_options("esp32")
        local pico_upload = board_vm.pico_uf2_upload_options("pico-w")

        assert.are.equal("USB/serial", options[1].display_name)
        assert.are.equal("network_endpoint", options[2].requires)
        assert.is_true(options[2].ota_update)
        assert.are.equal(0x1000, esp_upload.offset)
        assert.are.equal(115200, esp_upload.baud_rate)
        assert.are.equal("pico-uf2", pico_upload.command)
        assert.are.equal("RPI-RP2", pico_upload.volume_label)
        assert.is_nil(board_vm.esp_upload_options("pico"))
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
