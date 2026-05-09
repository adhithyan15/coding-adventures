import io
import pathlib
import tempfile

from board_vm_native import (
    BoardDevice,
    BoardDescriptor,
    BoardTarget,
    Connection,
    EspUploadOptions,
    PicoUf2UploadOptions,
    ProtocolResult,
    Session,
    TcpTransport,
    bluetooth_endpoint,
    connect,
    connection_option_list,
    connection_options,
    device_list,
    devices,
    detect_target,
    esp_upload_command,
    esp_upload_options,
    find_target,
    known_targets,
    pico_uf2_mount,
    pico_uf2_upload_command,
    pico_uf2_mounts,
    pico_uf2_upload_options,
    pick_connection_option,
    pick_device,
    pico,
    runtime_devices,
    select_connection_option,
    select_device,
    select_runtime_device,
)


class FakeWriteTransport:
    def __init__(self):
        self.frames = []

    def write(self, frame):
        self.frames.append(frame)


class FakeRunner:
    def __init__(self):
        self.calls = []

    def __call__(self, command, *, cwd=None):
        self.calls.append((command, cwd))
        return {"command": command, "cwd": cwd}


def test_native_session_builds_protocol_bytes_in_rust():
    session = Session()

    hello = session.hello(host_nonce=0x1234_ABCD)
    assert isinstance(hello.frame, bytes)
    assert len(hello.frame) > 0
    assert session.next_request_id == 2

    caps = session.capabilities()
    assert isinstance(caps.frame, bytes)
    assert len(caps.frame) > 0
    assert session.next_request_id == 3

    module = session.blink_module(pin=13, high_ms=250, low_ms=250, max_stack=4)
    assert isinstance(module, bytes)
    assert len(module) > 0

    gpio_module = session.gpio_read_module(pin=13, mode="pullup", max_stack=2)
    assert isinstance(gpio_module, bytes)
    assert len(gpio_module) > 0

    gpio_write_module = session.gpio_write_module(pin=13, value=True, max_stack=3)
    assert isinstance(gpio_write_module, bytes)
    assert len(gpio_write_module) > 0

    gpio_open_module = session.gpio_open_module(pin=13, mode="output", max_stack=2)
    assert isinstance(gpio_open_module, bytes)
    assert len(gpio_open_module) > 0

    gpio_handle_read_module = session.gpio_handle_read_module(max_stack=2)
    assert isinstance(gpio_handle_read_module, bytes)
    assert len(gpio_handle_read_module) > 0

    gpio_handle_write_module = session.gpio_handle_write_module(value=True, max_stack=3)
    assert isinstance(gpio_handle_write_module, bytes)
    assert len(gpio_handle_write_module) > 0

    gpio_handle_close_module = session.gpio_handle_close_module(max_stack=1)
    assert isinstance(gpio_handle_close_module, bytes)
    assert len(gpio_handle_close_module) > 0

    time_module = session.time_now_module(max_stack=1)
    assert isinstance(time_module, bytes)
    assert len(time_module) > 0

    sleep_module = session.time_sleep_ms_module(250, max_stack=1)
    assert isinstance(sleep_module, bytes)
    assert len(sleep_module) > 0

    raw_module = session.raw_module(code=b"\x00", max_stack=1, const_pool=b"\xAA\x55")
    assert raw_module.startswith(b"BVM1")
    assert raw_module.endswith(b"\xAA\x55")

    stop = session.stop()
    assert isinstance(stop.frame, bytes)
    assert len(stop.frame) > 0
    assert session.next_request_id == 4


def test_known_targets_are_exposed_from_rust_registry():
    targets = known_targets()
    uno_r4_wifi = find_target("arduino-uno-r4-wifi")
    esp32 = find_target("esp32-devkit-v1")
    pico = find_target("raspberry-pi-pico")
    pico_w = find_target("raspberry-pi-pico-w")

    assert all(isinstance(target, BoardTarget) for target in targets)
    assert uno_r4_wifi is not None
    assert "transport.wifi" in uno_r4_wifi.capabilities
    assert "transport.bluetooth_le" in uno_r4_wifi.capabilities
    assert uno_r4_wifi.wireless_transports == ["wifi", "bluetooth_le"]
    assert uno_r4_wifi.supports_wifi is True
    assert uno_r4_wifi.supports_bluetooth is True
    assert uno_r4_wifi.supports_ota_update is True
    assert uno_r4_wifi.command_transports == ["serial", "wifi", "bluetooth_le"]
    assert uno_r4_wifi.ota_transports == ["wifi"]
    assert uno_r4_wifi.supports_command_transport("serial") is True
    assert esp32 is not None
    assert esp32.family == "esp32"
    assert esp32.runtime_id == "board-vm-esp32"
    assert esp32.onboard_led_pin == 2
    assert "gpio.open" in esp32.capabilities
    assert "transport.bluetooth_classic" in esp32.capabilities
    assert all(item["command_transport"] for item in esp32.wireless)
    assert any(
        item["transport"] == "bluetooth_classic" and item["requires"] == "paired_device"
        for item in esp32.connection_options
    )
    assert pico is not None
    assert pico.wireless == []
    assert [item["transport"] for item in pico.connection_options] == ["serial"]
    assert "transport.wifi" not in pico.capabilities
    assert pico_w is not None
    assert pico_w.onboard_led == {"kind": "wireless_chip_gpio", "pin": 0}
    assert "transport.wifi" in pico_w.capabilities
    assert "ota.wifi" in pico_w.capabilities


def test_connection_options_are_exposed_from_rust_registry():
    options = connection_options("uno-r4-wifi")

    assert options[0] == {
        "transport": "serial",
        "display_name": "USB/serial",
        "command_transport": True,
        "ota_update": False,
        "requires": "serial_port",
        "endpoint_transport": "serial_port",
        "endpoint_scheme": "serial",
        "wire_protocol": "board_vm_cobs_crc",
    }
    assert {
        "transport": "wifi",
        "display_name": "Wi-Fi",
        "command_transport": True,
        "ota_update": True,
        "requires": "network_endpoint",
        "endpoint_transport": "tcp_socket",
        "endpoint_scheme": "tcp",
        "wire_protocol": "board_vm_cobs_crc",
    } in options
    assert "Wi-Fi [commands, OTA]" in connection_option_list("uno-r4-wifi")


def test_bluetooth_endpoints_are_parsed_by_rust_language_core():
    ble = bluetooth_endpoint("ble://uno-r4-wifi/180f/2a19/2a1a")
    rfcomm = bluetooth_endpoint("btspp://ESP32-BoardVM:3")

    assert ble is not None
    assert ble["transport"] == "bluetooth_le"
    assert ble["endpoint_transport"] == "bluetooth_le_gatt"
    assert ble["endpoint_scheme"] == "ble"
    assert ble["device"] == "uno-r4-wifi"
    assert ble["service_uuid"] == "180f"
    assert ble["write_characteristic_uuid"] == "2a19"
    assert ble["notify_characteristic_uuid"] == "2a1a"
    assert ble["channel"] is None

    assert rfcomm is not None
    assert rfcomm["transport"] == "bluetooth_classic"
    assert rfcomm["endpoint_transport"] == "bluetooth_classic_rfcomm"
    assert rfcomm["endpoint_scheme"] == "btspp"
    assert rfcomm["device"] == "ESP32-BoardVM"
    assert rfcomm["channel"] == 3
    assert bluetooth_endpoint("tcp://board-vm.local:4170") is None


def test_connection_options_can_be_selected_without_exposing_ports():
    default = select_connection_option("uno-r4-wifi")
    wifi = select_connection_option("uno-r4-wifi", transport="wifi")
    friendly_wifi = select_connection_option("uno-r4-wifi", transport="Wi-Fi")
    friendly_serial = select_connection_option("uno-r4-wifi", transport="USB serial")
    ota = select_connection_option("uno-r4-wifi", ota=True)

    assert default["transport"] == "serial"
    assert wifi["transport"] == "wifi"
    assert friendly_wifi["transport"] == "wifi"
    assert friendly_serial["transport"] == "serial"
    assert ota["transport"] == "wifi"

    try:
        select_connection_option("raspberry-pi-pico", transport="wifi")
    except ValueError as error:
        message = str(error)
    else:
        raise AssertionError("expected wifi selection to fail for non-wireless Pico")
    assert "No wifi connection option" in message
    assert "USB/serial" in message


def test_connection_option_picker_prompts_for_repl_use():
    output = io.StringIO()

    option = pick_connection_option(
        "uno-r4-wifi",
        input_func=lambda _prompt: "2",
        output=output,
    )

    assert option["transport"] == "wifi"
    rendered = output.getvalue()
    assert "1. USB/serial [commands] - requires serial_port" in rendered
    assert "2. Wi-Fi [commands, OTA] - requires network_endpoint" in rendered
    assert "Select connection [1-3]: " in rendered


def test_connect_records_the_rust_selected_serial_connection_option():
    found = devices(["/dev/tty.usbserial-CP2102-esp32"])

    connection = connect("esp32", device_candidates=found)

    assert connection.connection_transport == "serial"
    assert connection.connection_option["display_name"] == "USB/serial"
    assert connection.serial_connection is True
    assert connection.wireless_connection is False


def test_connect_can_use_a_wireless_connection_option_with_an_injected_endpoint():
    transport = FakeWriteTransport()

    connection = connect(
        "uno-r4-wifi",
        via="Wi-Fi",
        smoke=True,
        transport=transport,
    )

    assert connection.port is None
    assert connection.connection_transport == "wifi"
    assert connection.wireless_connection is True
    assert connection.ota_connection is True
    assert len(transport.frames) == 2


def test_connect_builds_a_tcp_transport_for_wifi_endpoints():
    connection = connect(
        "uno-r4-wifi",
        via="Wi-Fi",
        endpoint="tcp://board-vm.local:4170",
    )

    assert connection.port is None
    assert connection.endpoint == "tcp://board-vm.local:4170"
    assert connection.connection_transport == "wifi"
    transport = connection._active_transport()
    assert isinstance(transport, TcpTransport)
    assert transport.endpoint == "tcp://board-vm.local:4170"
    assert transport.host == "board-vm.local"
    assert transport.port == 4170


def test_wifi_endpoint_dispatch_requires_endpoint_when_not_injected():
    connection = connect("uno-r4-wifi", via="Wi-Fi")

    try:
        connection.smoke()
    except ValueError as error:
        message = str(error)
    else:
        raise AssertionError("expected missing TCP endpoint to fail")

    assert "requires a Board VM TCP endpoint" in message


def test_connect_can_prompt_for_the_connection_option_after_the_board():
    output = io.StringIO()

    connection = connect(
        "uno-r4-wifi",
        pick_connection=True,
        input_func=lambda _prompt: "2",
        output=output,
        transport=FakeWriteTransport(),
    )

    assert connection.connection_transport == "wifi"
    assert connection.port is None
    assert "Select connection [1-3]: " in output.getvalue()


def test_targets_are_detected_from_rust_owned_aliases():
    esp32 = detect_target("esp32")
    pico = detect_target("Raspberry Pi Pico")
    pico_w = find_target("pico-w")

    assert esp32 is not None
    assert esp32.board_id == "esp32-devkit-v1"
    assert esp32.rust_target == "xtensa-esp32-none-elf"
    assert pico is not None
    assert pico.board_id == "raspberry-pi-pico"
    assert pico_w is not None
    assert pico_w.board_id == "raspberry-pi-pico-w"
    assert detect_target("not-a-board") is None


def test_esp_upload_options_are_exposed_from_rust_language_core():
    options = esp_upload_options("esp32")

    assert isinstance(options, EspUploadOptions)
    assert options.board_id == "esp32-devkit-v1"
    assert options.baud_rate == 115_200
    assert options.timeout_ms == 1_000
    assert options.reset_into_bootloader is True
    assert options.offset == 0x1000
    assert options.block_size == 0x400
    assert options.flash_size == 4 * 1024 * 1024
    assert options.verify_md5 is True
    assert options.stay_in_bootloader is False
    assert esp_upload_options("pico") is None

    overridden = esp_upload_options("esp32", offset=0x2000, verify_md5=False)
    assert overridden is not None
    assert overridden.offset == 0x2000
    assert overridden.verify_md5 is False


def test_pico_uf2_upload_options_are_exposed_from_rust_language_core():
    options = pico_uf2_upload_options("pico")

    assert isinstance(options, PicoUf2UploadOptions)
    assert options.board_id == "raspberry-pi-pico"
    assert options.command == "pico-uf2"
    assert options.volume_label == "RPI-RP2"
    assert options.image_extension == ".uf2"
    assert options.auto_detect_mount is True
    assert pico_uf2_upload_options("pico-w") is not None
    assert pico_uf2_upload_options("esp32") is None


def test_pico_uf2_mounts_are_discovered_by_rust_language_core():
    with tempfile.TemporaryDirectory(prefix="board-vm-pico-uf2") as root:
        mount = pathlib.Path(root) / "RPI-RP2"
        mount.mkdir()
        (mount / "INFO_UF2.TXT").write_text(
            "UF2 Bootloader\nModel: Raspberry Pi RP2\n",
            encoding="utf-8",
        )
        (mount / "INDEX.HTM").write_text("<html></html>", encoding="utf-8")
        (pathlib.Path(root) / "NOT-PICO").mkdir()

        assert pico_uf2_mounts([root]) == [str(mount)]


def test_pico_uf2_mount_selects_single_discovered_mount():
    with tempfile.TemporaryDirectory(prefix="board-vm-pico-uf2") as root:
        mount = pathlib.Path(root) / "RPI-RP2"
        mount.mkdir()
        (mount / "INFO_UF2.TXT").write_text(
            "UF2 Bootloader\nModel: Raspberry Pi RP2\n",
            encoding="utf-8",
        )
        (mount / "INDEX.HTM").write_text("<html></html>", encoding="utf-8")

        assert pico_uf2_mount([root]) == str(mount)


def test_pico_uf2_mount_reports_multiple_discovered_mounts():
    with (
        tempfile.TemporaryDirectory(prefix="board-vm-pico-uf2-a") as root_a,
        tempfile.TemporaryDirectory(prefix="board-vm-pico-uf2-b") as root_b,
    ):
        mount_a = pathlib.Path(root_a) / "RPI-RP2"
        mount_b = pathlib.Path(root_b) / "RPI-RP2"
        for mount in (mount_a, mount_b):
            mount.mkdir()
            (mount / "INFO_UF2.TXT").write_text(
                "UF2 Bootloader\nModel: Raspberry Pi RP2\n",
                encoding="utf-8",
            )
            (mount / "INDEX.HTM").write_text("<html></html>", encoding="utf-8")

        try:
            pico_uf2_mount([root_a, root_b])
        except ValueError as error:
            message = str(error)
        else:
            raise AssertionError("expected ambiguous Pico BOOTSEL mount selection to fail")

        assert "Multiple Pico BOOTSEL UF2 mounts" in message
        assert str(mount_a) in message
        assert str(mount_b) in message


def test_esp_upload_command_uses_rust_owned_options():
    command = esp_upload_command(
        "esp32",
        port="/dev/cu.usbserial-110",
        image="/tmp/board-vm-esp32.bin",
        offset=0x2000,
        verify_md5=False,
        stay_in_bootloader=True,
    )

    assert command == [
        "esp-upload",
        "--port",
        "/dev/cu.usbserial-110",
        "--image",
        "/tmp/board-vm-esp32.bin",
        "--baud",
        "115200",
        "--timeout-ms",
        "1000",
        "--offset",
        "8192",
        "--block-size",
        "1024",
        "--flash-size",
        "4194304",
        "--no-verify",
        "--stay-in-bootloader",
    ]


def test_pico_uf2_upload_command_uses_rust_owned_options():
    command = pico_uf2_upload_command(
        "pico",
        image="/tmp/board-vm-pico.uf2",
        mount="/Volumes/RPI-RP2",
    )

    assert command == [
        "pico-uf2",
        "--image",
        "/tmp/board-vm-pico.uf2",
        "--mount",
        "/Volumes/RPI-RP2",
    ]


def test_pico_uf2_upload_command_auto_selects_single_mount():
    with tempfile.TemporaryDirectory(prefix="board-vm-pico-uf2") as root:
        mount = pathlib.Path(root) / "RPI-RP2"
        mount.mkdir()
        (mount / "INFO_UF2.TXT").write_text(
            "UF2 Bootloader\nModel: Raspberry Pi RP2\n",
            encoding="utf-8",
        )
        (mount / "INDEX.HTM").write_text("<html></html>", encoding="utf-8")

        command = pico_uf2_upload_command(
            "pico-w",
            image="/tmp/board-vm-pico-w.uf2",
            roots=[root],
        )

    assert command == [
        "pico-uf2",
        "--image",
        "/tmp/board-vm-pico-w.uf2",
        "--mount",
        str(mount),
    ]


def test_esp_upload_command_can_select_a_discovered_esp_device():
    found = devices([
        "/dev/cu.usbmodem1101",
        "/dev/tty.usbserial-CP2102-esp32",
    ])

    selected = select_device("esp32", device_candidates=found)
    command = esp_upload_command(
        "esp32",
        device_candidates=found,
        image="/tmp/board-vm-esp32.bin",
        offset=0x2000,
        verify_md5=False,
    )

    assert selected.port == "/dev/tty.usbserial-CP2102-esp32"
    assert command == [
        "esp-upload",
        "--port",
        "/dev/tty.usbserial-CP2102-esp32",
        "--image",
        "/tmp/board-vm-esp32.bin",
        "--baud",
        "115200",
        "--timeout-ms",
        "1000",
        "--offset",
        "8192",
        "--block-size",
        "1024",
        "--flash-size",
        "4194304",
        "--no-verify",
    ]


def test_esp_upload_command_rejects_non_esp_targets():
    try:
        esp_upload_command("pico", port="/dev/cu.usbmodem", image="fw.bin")
    except ValueError as error:
        assert "ESP upload is not supported" in str(error)
    else:
        raise AssertionError("expected ValueError")


def test_devices_are_classified_by_rust_language_core():
    found = devices([
        "/dev/cu.usbmodem1101",
        "/dev/tty.usbserial-CP2102-esp32",
        "/dev/serial/by-id/usb-Raspberry_Pi_Pico_E660-DAPLINK-if00",
    ])

    assert all(isinstance(device, BoardDevice) for device in found)
    assert found[0].port == "/dev/cu.usbmodem1101"
    assert found[0].target is None
    assert "usb_cdc" in found[0].tags

    esp = next(device for device in found if "usbserial" in device.port)
    assert esp.target is not None
    assert esp.target.board_id == "esp32-devkit-v1"
    assert "uart" in esp.tags

    pico = next(device for device in found if "Raspberry_Pi_Pico" in device.port)
    assert pico.target is not None
    assert pico.target.board_id == "raspberry-pi-pico"
    assert pico.bootloader is True

    rendered = device_list(found)
    assert "ESP32 DevKit V1" in rendered
    assert "/dev/cu.usbmodem1101" in rendered


def test_runtime_device_selection_ignores_pico_bootloader_devices():
    found = devices(
        [
            "/dev/serial/by-id/usb-Raspberry_Pi_Pico_E660-DAPLINK-if00",
            "/dev/serial/by-id/usb-Raspberry_Pi_Pico_Board_VM-if00",
        ]
    )

    selected = select_runtime_device("pico", device_candidates=found)
    runtime = runtime_devices("pico", device_candidates=found)

    assert selected.port == "/dev/serial/by-id/usb-Raspberry_Pi_Pico_Board_VM-if00"
    assert selected.bootloader is False
    assert [device.port for device in runtime] == [selected.port]


def test_pick_device_prompts_for_ambiguous_devices():
    found = devices([
        "/dev/cu.usbmodem1101",
        "/dev/cu.usbmodem2201",
    ])
    output = io.StringIO()

    selected = pick_device(
        device_candidates=found,
        input_func=lambda _prompt: "2",
        output=output,
    )

    assert selected.port == "/dev/cu.usbmodem2201"
    assert "1. Unknown board" in output.getvalue()
    assert "2. Unknown board" in output.getvalue()
    assert "Select board [1-2]: " in output.getvalue()


def test_python_connect_auto_selects_device_and_dispatches_native_session():
    found = devices(["/dev/tty.usbserial-CP2102-esp32"])
    transport = FakeWriteTransport()

    connection = connect(
        "esp32",
        device_candidates=found,
        transport=transport,
    )
    result = connection.session().run_command("blink 42", program_id=7)

    assert isinstance(connection, Connection)
    assert connection.board_id == "esp32-devkit-v1"
    assert connection.port == "/dev/tty.usbserial-CP2102-esp32"
    assert [item.command for item in result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert result.frames == transport.frames


def test_python_connect_can_smoke_auto_selected_device():
    found = devices(["/dev/tty.usbserial-CP2102-esp32"])
    transport = FakeWriteTransport()

    connection = connect(
        "esp32",
        smoke=True,
        device_candidates=found,
        transport=transport,
    )

    assert connection.port == "/dev/tty.usbserial-CP2102-esp32"
    assert len(transport.frames) == 2
    assert all(isinstance(frame, bytes) and frame for frame in transport.frames)


def test_python_esp_connect_flash_uses_rust_owned_upload_command():
    found = devices(["/dev/tty.usbserial-CP2102-esp32"])
    runner = FakeRunner()

    connection = connect(
        "esp32",
        flash=True,
        firmware_image="/tmp/board-vm-esp32.bin",
        device_candidates=found,
        cargo_workspace="/repo/code/packages/rust",
        runner=runner,
        esp_upload_options={"offset": 0x2000, "verify_md5": False},
    )

    assert connection.port == "/dev/tty.usbserial-CP2102-esp32"
    assert runner.calls == [
        (
            [
                "cargo",
                "run",
                "-p",
                "board-vm-cli",
                "--bin",
                "board-vm",
                "--",
                "esp-upload",
                "--port",
                "/dev/tty.usbserial-CP2102-esp32",
                "--image",
                "/tmp/board-vm-esp32.bin",
                "--baud",
                "115200",
                "--timeout-ms",
                "1000",
                "--offset",
                "8192",
                "--block-size",
                "1024",
                "--flash-size",
                "4194304",
                "--no-verify",
            ],
            "/repo/code/packages/rust",
        )
    ]


def test_python_pico_flash_rediscover_runtime_port():
    runner = FakeRunner()
    runtime_devices = devices(["/dev/serial/by-id/usb-Raspberry_Pi_Pico_Board_VM-if00"])

    connection = pico(
        flash=True,
        firmware_image="/tmp/board-vm-pico.uf2",
        pico_uf2_mount="/Volumes/RPI-RP2",
        pico_runtime_port_wait_ms=0,
        device_discovery=lambda: runtime_devices,
        cargo_workspace="/repo/code/packages/rust",
        runner=runner,
    )

    assert connection.board_id == "raspberry-pi-pico"
    assert connection.port == "/dev/serial/by-id/usb-Raspberry_Pi_Pico_Board_VM-if00"
    assert runner.calls == [
        (
            [
                "cargo",
                "run",
                "-p",
                "board-vm-cli",
                "--bin",
                "board-vm",
                "--",
                "pico-uf2",
                "--image",
                "/tmp/board-vm-pico.uf2",
                "--mount",
                "/Volumes/RPI-RP2",
            ],
            "/repo/code/packages/rust",
        )
    ]


def test_python_pico_flash_can_smoke_rediscovered_runtime_port():
    runner = FakeRunner()
    transport = FakeWriteTransport()
    runtime_devices = devices(["/dev/serial/by-id/usb-Raspberry_Pi_Pico_Board_VM-if00"])

    connection = pico(
        flash=True,
        smoke=True,
        firmware_image="/tmp/board-vm-pico.uf2",
        pico_uf2_mount="/Volumes/RPI-RP2",
        pico_runtime_port_wait_ms=0,
        device_discovery=lambda: runtime_devices,
        cargo_workspace="/repo/code/packages/rust",
        runner=runner,
        transport=transport,
    )

    assert connection.port == "/dev/serial/by-id/usb-Raspberry_Pi_Pico_Board_VM-if00"
    assert len(runner.calls) == 1
    assert len(transport.frames) == 2


def test_session_smoke_dispatches_hello_and_capabilities():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.smoke(host_nonce=123)

    assert [item.command for item in result.results] == ["hello", "capabilities"]
    assert result.frames == transport.frames


def test_session_dispatches_frames_through_write_transport():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.blink(program_id=7, instruction_budget=24, handshake=True, query_caps=True)

    assert [item.command for item in result.results] == [
        "hello",
        "capabilities",
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert result.frames == transport.frames
    assert all(isinstance(frame, bytes) and frame for frame in result.frames)
    assert result.responses == [None] * 6
    assert result.decoded_responses == [None] * 6


def test_session_smoke_dispatches_hello_and_capabilities():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.smoke(host_nonce=123)

    assert [item.command for item in result.results] == ["hello", "capabilities"]
    assert result.frames == transport.frames


def test_run_command_accepts_repl_style_blink():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.run_command("blink 42", program_id=9)

    assert [item.command for item in result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert result.frames == transport.frames


def test_run_command_accepts_repl_style_stop():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.run_command("stop")

    assert [item.command for item in result.results] == ["stop"]
    assert result.frames == transport.frames


def test_store_program_dispatches_rust_owned_wire_frame():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.store_program(program_id=9, slot=2, boot_policy="run-at-boot")
    command = session.run_command("store-program 10 3 store-only")

    assert result.command == "store_program"
    assert isinstance(result.frame, bytes)
    assert result.frame == transport.frames[0]
    assert [item.command for item in command.results] == ["store_program"]
    assert command.frames == transport.frames[1:]


def test_run_accepts_configurable_rust_owned_flags():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.run(
        program_id=9,
        instruction_budget=42,
        keep_handles=True,
        background=False,
        time_budget_ms=250,
    )

    assert result.command == "run"
    assert isinstance(result.frame, bytes)
    assert result.frame == transport.frames[0]
    assert session.next_request_id == 2


def test_run_command_accepts_repl_style_gpio_read():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.run_command("gpio-read 13 pullup 24", program_id=9)

    assert [item.command for item in result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert result.frames == transport.frames


def test_run_command_accepts_repl_style_gpio_write_and_levels():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.run_command("gpio-write 13 high 24", program_id=9)
    high = session.run_command("gpio-high 13 24", program_id=10)
    low = session.run_command("gpio-low 13 24", program_id=11)

    assert [item.command for item in result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert [item.command for item in high.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert [item.command for item in low.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert result.frames + high.frames + low.frames == transport.frames


def test_run_command_accepts_repl_style_gpio_handle_commands():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    open_result = session.run_command("gpio-open 13 output 24", program_id=9)
    read = session.run_command("gpio-handle-read 24", program_id=10)
    write = session.run_command("gpio-handle-write high 24", program_id=11)
    close = session.run_command("gpio-handle-close 24", program_id=12)

    assert [item.command for item in open_result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert [item.command for item in read.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert [item.command for item in write.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert [item.command for item in close.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert open_result.frames + read.frames + write.frames + close.frames == transport.frames


def test_run_command_accepts_repl_style_time_now():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.run_command("time-now 42", program_id=9)

    assert [item.command for item in result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert result.frames == transport.frames


def test_run_command_accepts_repl_style_time_sleep_ms():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.run_command("time-sleep-ms 250 42", program_id=9)
    upload = session.run_command("upload-time-sleep-ms 125", program_id=10)

    assert [item.command for item in result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
        "run",
    ]
    assert [item.command for item in upload.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
    ]
    assert result.frames + upload.frames == transport.frames


def test_upload_raw_module_uses_rust_owned_module_builder():
    transport = FakeWriteTransport()
    session = Session(transport=transport)

    result = session.upload_raw_module(
        program_id=12,
        code=b"\x00",
        max_stack=1,
        const_pool=b"\xAA\x55",
    )

    assert [item.command for item in result.results] == [
        "program_begin",
        "program_chunk",
        "program_end",
    ]
    assert result.frames == transport.frames


def test_board_descriptor_wraps_rust_decoded_capability_payload():
    descriptor = BoardDescriptor(
        {
            "board_id": "arduino-uno-r4-wifi",
            "runtime_id": "board-vm-uno-r4",
            "max_program_bytes": 1024,
            "max_stack_values": 16,
            "max_handles": 4,
            "supports_store_program": False,
            "capabilities": [
                {"id": 1, "version": 1, "flags": 1, "name": "gpio.open"},
                {
                    "id": 0x7001,
                    "version": 1,
                    "flags": 2,
                    "name": "program.ram_exec",
                    "protocol_feature": True,
                    "flag_names": ["protocol_feature"],
                },
            ],
        }
    )

    assert descriptor.board_id == "arduino-uno-r4-wifi"
    assert descriptor.runtime_id == "board-vm-uno-r4"
    assert descriptor.capability_names == ["gpio.open", "program.ram_exec"]
    assert descriptor.supports("gpio.open")
    assert descriptor.supports(0x7001)
    assert descriptor.capability("gpio.open").name == "gpio.open"
    assert descriptor.capability("gpio.open").bytecode_callable
    assert descriptor.capability("program.ram_exec").protocol_feature
    assert descriptor.capability("program.ram_exec").flag_names == ["protocol_feature"]

    result = ProtocolResult(
        command="capabilities",
        frame=b"frame",
        decoded_response={"kind": "caps_report", "payload": descriptor.raw},
    )
    assert result.board_descriptor.capability_names == descriptor.capability_names
