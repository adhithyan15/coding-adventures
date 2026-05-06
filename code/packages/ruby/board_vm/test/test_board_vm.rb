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
            run = session.run(program_id: 4, budget: 77)

            assert_equal :hello, hello.command
            assert_equal :capabilities, caps.command
            assert_equal [:program_begin, :program_chunk, :program_end], upload.results.map(&:command)
            assert_equal :run, run.command
          end
        end

        assert_empty runner.calls
        assert_equal 6, transport.frames.length
        assert transport.frames.all? { |frame| frame.is_a?(String) && frame.bytesize.positive? }
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
