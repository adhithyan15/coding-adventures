# frozen_string_literal: true

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

      def test_led_blink_runs_the_generic_board_vm_smoke_command
        runner = FakeRunner.new

        BoardVM.uno_r4_wifi(
          port: "/dev/cu.usbmodem2201",
          cargo_workspace: "/repo/code/packages/rust",
          runner: runner,
          baud: 57_600,
          timeout_ms: 250
        ) do |board|
          board.led.blink(program_id: 9, budget: 32, host_nonce: 123)
        end

        assert_equal [
          "cargo", "run",
          "-p", "board-vm-cli",
          "--bin", "board-vm",
          "--",
          "smoke",
          "--port", "/dev/cu.usbmodem2201",
          "--baud", "57600",
          "--timeout-ms", "250",
          "--program-id", "9",
          "--budget", "32",
          "--host-nonce", "123"
        ], runner.calls.first[:argv]
      end

      def test_native_session_builds_protocol_bytes_in_rust
        session = BoardVM::Native::Session.new

        hello = session.hello_wire("bvm", 0x1234_ABCD)
        assert_instance_of String, hello
        assert_operator hello.bytesize, :>, 0
        assert_equal 2, session.next_request_id

        default_nonce_hello = session.hello_wire("bvm", BoardVM::DEFAULT_HOST_NONCE)
        assert_operator default_nonce_hello.bytesize, :>, 0

        module_bytes = session.blink_module(13, 250, 250, 4)
        assert_instance_of String, module_bytes
        assert_operator module_bytes.bytesize, :>, 0

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
          BoardVM.connect(board: :esp32, port: "/dev/cu.usbserial", runner: runner)
        end

        assert_empty runner.calls
      end
    end
  end
end
