# frozen_string_literal: true

module CodingAdventures
  module BoardVM
    class UnsupportedBoardError < ArgumentError; end

    class Connection
      attr_reader :board, :cargo_workspace, :runner, :transport, :baud_rate, :timeout_ms
      attr_accessor :port

      def initialize(
        board:,
        port:,
        cargo_workspace:,
        runner:,
        transport:,
        baud_rate:,
        timeout_ms:,
        arduino_core: nil,
        arm_toolchain_bin: nil,
        arm_gcc: nil,
        arm_gxx: nil,
        arm_ar: nil,
        arm_compat_root: nil,
        bossac_path: nil,
        arduino_cli: nil,
        objcopy: nil,
        rustc: nil,
        target_dir: nil,
        bootloader_touch: true,
        bootloader_touch_timeout_ms: nil,
        bootloader_touch_settle_ms: nil,
        bootloader_port_wait_ms: nil
      )
        @board = board
        @port = port
        @cargo_workspace = cargo_workspace
        @runner = runner
        @transport = transport
        @baud_rate = baud_rate
        @timeout_ms = timeout_ms
        @arduino_core = arduino_core
        @arm_toolchain_bin = arm_toolchain_bin
        @arm_gcc = arm_gcc
        @arm_gxx = arm_gxx
        @arm_ar = arm_ar
        @arm_compat_root = arm_compat_root
        @bossac_path = bossac_path
        @arduino_cli = arduino_cli
        @objcopy = objcopy
        @rustc = rustc
        @target_dir = target_dir
        @bootloader_touch = bootloader_touch
        @bootloader_touch_timeout_ms = bootloader_touch_timeout_ms
        @bootloader_touch_settle_ms = bootloader_touch_settle_ms
        @bootloader_port_wait_ms = bootloader_port_wait_ms
      end

      def led
        Led.new(self)
      end

      def eject
        Ejector.new(self)
      end

      def capabilities
        session.board_descriptor
      end

      def session(**options)
        protocol_session = Session.new(self, **options)
        return protocol_session unless block_given?

        yield protocol_session
        protocol_session
      end

      def flash!
        ensure_uno_r4_wifi!

        result = runner.call(serial_usb_artifact_command(upload: true), chdir: cargo_workspace)
        handoff_port = self.class.parse_new_upload_port(result.output)
        self.port = handoff_port if handoff_port
        result
      end

      def blink!(
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        host_nonce: DEFAULT_HOST_NONCE,
        pin: 13,
        high_ms: 250,
        low_ms: 250,
        max_stack: 4
      )
        ensure_uno_r4_wifi!

        session.blink(
          program_id: program_id,
          budget: budget,
          pin: pin,
          high_ms: high_ms,
          low_ms: low_ms,
          max_stack: max_stack,
          handshake: true,
          query_caps: true,
          host_nonce: host_nonce
        )
      end

      def eject_blink!(
        output:,
        program_id: DEFAULT_PROGRAM_ID,
        slot: DEFAULT_EJECT_SLOT,
        boot_policy: DEFAULT_BOOT_POLICY
      )
        runner.call(
          board_vm_cli_command(
            "eject", "blink",
            "--out", output,
            "--program-id", program_id.to_s,
            "--slot", slot.to_s,
            "--boot-policy", boot_policy_name(boot_policy)
          ),
          chdir: cargo_workspace
        )
      end

      def self.parse_new_upload_port(output)
        port = nil
        output.each_line do |line|
          marker_index = line.index("New upload port:")
          next if marker_index.nil?

          candidate = line[(marker_index + "New upload port:".length)..-1].strip
          candidate = candidate.split(/\s+/, 2).first
          port = candidate unless candidate.nil? || candidate.empty?
        end
        port
      end

      def dispatch_protocol_frame(frame, native_session:)
        response = dispatch_frame(frame)
        [response, decode_response(native_session, response)]
      end

      private

      def ensure_uno_r4_wifi!
        return if board == :uno_r4_wifi

        raise UnsupportedBoardError, "Ruby DSL currently supports :uno_r4_wifi; got #{board.inspect}"
      end

      def serial_usb_artifact_command(upload:)
        command = cargo_command(
          "run",
          "-p", "board-vm-uno-r4-firmware",
          "--bin", "uno-r4-wifi-serialusb-artifact",
          "--"
        )
        append_option(command, "--core", @arduino_core)
        append_option(command, "--rustc", @rustc)
        append_option(command, "--arm-toolchain-bin", @arm_toolchain_bin)
        append_option(command, "--arm-gcc", @arm_gcc)
        append_option(command, "--arm-gxx", @arm_gxx)
        append_option(command, "--arm-ar", @arm_ar)
        append_option(command, "--arm-compat-root", @arm_compat_root)
        append_option(command, "--target-dir", @target_dir)
        append_option(command, "--objcopy", @objcopy)
        append_option(command, "--arduino-cli", @arduino_cli)
        append_option(command, "--bossac-path", @bossac_path)
        append_option(command, "--port", port)
        append_option(command, "--baud", baud_rate)
        append_option(command, "--timeout-ms", timeout_ms)
        append_option(command, "--bootloader-touch-timeout-ms", @bootloader_touch_timeout_ms)
        append_option(command, "--bootloader-touch-settle-ms", @bootloader_touch_settle_ms)
        append_option(command, "--bootloader-port-wait-ms", @bootloader_port_wait_ms)
        command << "--no-bootloader-touch" unless @bootloader_touch
        command << "--upload" if upload
        command
      end

      def board_vm_cli_command(*args)
        cargo_command("run", "-p", "board-vm-cli", "--bin", "board-vm", "--", *args)
      end

      def dispatch_frame(frame)
        if active_transport.respond_to?(:transact)
          active_transport.transact(frame, timeout_ms: timeout_ms)
        elsif active_transport.respond_to?(:write)
          active_transport.write(frame)
          nil
        else
          raise TransportError, "Board VM transport must respond to #transact or #write"
        end
      end

      def active_transport
        @transport ||= SerialTransport.new(port: port, baud_rate: baud_rate, timeout_ms: timeout_ms)
      end

      def decode_response(session, response)
        return nil if response.nil?

        session.decode_response(response)
      end

      def cargo_command(*args)
        ["cargo"] + args
      end

      def append_option(command, option, value)
        return if value.nil?

        command << option << value.to_s
      end

      def boot_policy_name(policy)
        case policy
        when :store_only then "store-only"
        when :run_at_boot then "run-at-boot"
        when :run_if_no_host then "run-if-no-host"
        else policy.to_s
        end
      end
    end

    class Led
      def initialize(connection)
        @connection = connection
      end

      def blink(
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        host_nonce: DEFAULT_HOST_NONCE,
        pin: 13,
        high_ms: 250,
        low_ms: 250,
        max_stack: 4
      )
        @connection.blink!(
          program_id: program_id,
          budget: budget,
          host_nonce: host_nonce,
          pin: pin,
          high_ms: high_ms,
          low_ms: low_ms,
          max_stack: max_stack
        )
      end
    end

    class Ejector
      def initialize(connection)
        @connection = connection
      end

      def blink(
        to:,
        program_id: DEFAULT_PROGRAM_ID,
        slot: DEFAULT_EJECT_SLOT,
        boot_policy: DEFAULT_BOOT_POLICY
      )
        @connection.eject_blink!(
          output: to,
          program_id: program_id,
          slot: slot,
          boot_policy: boot_policy
        )
      end
    end
  end
end
