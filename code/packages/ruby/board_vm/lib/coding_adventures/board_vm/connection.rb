# frozen_string_literal: true

module CodingAdventures
  module BoardVM
    class UnsupportedBoardError < ArgumentError; end
    class DeviceSelectionError < ArgumentError; end

    class Connection
      attr_reader :board, :cargo_workspace, :runner, :transport, :connection_option, :baud_rate, :timeout_ms, :endpoint
      attr_accessor :port

      def initialize(
        board:,
        port:,
        cargo_workspace:,
        runner:,
        transport:,
        connection_option:,
        endpoint:,
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
        bootloader_port_wait_ms: nil,
        firmware_image: nil,
        esp_image: nil,
        esp_upload_options: nil,
        device_discovery: nil,
        pico_uf2_mount: nil,
        pico_uf2_mount_roots: nil,
        pico_uf2_upload_options: nil,
        pico_runtime_port: true,
        pico_runtime_port_wait_ms: DEFAULT_PICO_RUNTIME_PORT_WAIT_MS,
        pico_runtime_port_poll_ms: DEFAULT_PICO_RUNTIME_PORT_POLL_MS,
        bluetooth_backend_plan: nil
      )
        @board = board
        @port = port
        @cargo_workspace = cargo_workspace
        @runner = runner
        @transport = transport
        @connection_option = connection_option
        @endpoint = endpoint
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
        @firmware_image = firmware_image || esp_image
        @esp_upload_options = esp_upload_options || {}
        @device_discovery = device_discovery || -> { BoardVM.devices }
        @pico_uf2_mount = pico_uf2_mount
        @pico_uf2_mount_roots = pico_uf2_mount_roots
        @pico_uf2_upload_options = pico_uf2_upload_options || {}
        @pico_runtime_port = pico_runtime_port
        @pico_runtime_port_wait_ms = pico_runtime_port_wait_ms
        @pico_runtime_port_poll_ms = pico_runtime_port_poll_ms
        @bluetooth_backend_plan = bluetooth_backend_plan
      end

      def led
        Led.new(self)
      end

      def time
        TimeApi.new(self)
      end

      def gpio
        Gpio.new(self)
      end

      def eject
        Ejector.new(self)
      end

      def capabilities
        session.board_descriptor
      end

      def smoke!(host_name: Session::DEFAULT_HOST_NAME, host_nonce: DEFAULT_HOST_NONCE, query_caps: true)
        session.smoke(host_name: host_name, host_nonce: host_nonce, query_caps: query_caps)
      end

      def session(**options)
        protocol_session = Session.new(self, **options)
        return protocol_session unless block_given?

        yield protocol_session
        protocol_session
      end

      def flash!
        if !serial_connection? && !pico_board?
          raise UnsupportedBoardError,
            "#{connection_display_name} flashing is known in target metadata, but Ruby host flashing over #{connection_transport} is not wired yet; choose via: :serial"
        end

        case board
        when :uno_r4_wifi
          flash_uno_r4_wifi!
        when :esp32_devkit_v1
          flash_esp32!
        when :raspberry_pi_pico, :raspberry_pi_pico_w
          flash_pico_uf2!
        else
          raise UnsupportedBoardError, "Ruby DSL flash currently supports :uno_r4_wifi, :esp32_devkit_v1, :raspberry_pi_pico, and :raspberry_pi_pico_w; got #{board.inspect}"
        end
      end

      def connection_transport
        connection_option && connection_option["transport"]
      end

      def serial_connection?
        connection_transport.nil? || connection_transport == "serial"
      end

      def wireless_connection?
        !serial_connection?
      end

      def ota_connection?
        !!(connection_option && connection_option["ota_update"])
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

      def gpio_read!(
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        host_nonce: DEFAULT_HOST_NONCE,
        pin:,
        mode: :input,
        max_stack: 2
      )
        ensure_uno_r4_wifi!

        session.gpio_read(
          program_id: program_id,
          budget: budget,
          pin: pin,
          mode: mode,
          max_stack: max_stack,
          handshake: true,
          query_caps: true,
          host_nonce: host_nonce
        )
      end

      def gpio_write!(
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        host_nonce: DEFAULT_HOST_NONCE,
        pin:,
        value:,
        max_stack: 3
      )
        ensure_uno_r4_wifi!

        session.gpio_write(
          program_id: program_id,
          budget: budget,
          pin: pin,
          value: value,
          max_stack: max_stack,
          handshake: true,
          query_caps: true,
          host_nonce: host_nonce
        )
      end

      def gpio_open!(
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        host_nonce: DEFAULT_HOST_NONCE,
        pin:,
        mode: :output,
        max_stack: 2
      )
        ensure_uno_r4_wifi!

        session.gpio_open(
          program_id: program_id,
          budget: budget,
          pin: pin,
          mode: mode,
          max_stack: max_stack,
          handshake: true,
          query_caps: true,
          host_nonce: host_nonce
        )
      end

      def gpio_handle_read!(
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        max_stack: 2
      )
        ensure_uno_r4_wifi!

        session.gpio_handle_read(
          program_id: program_id,
          budget: budget,
          max_stack: max_stack
        )
      end

      def gpio_handle_write!(
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        value:,
        max_stack: 3
      )
        ensure_uno_r4_wifi!

        session.gpio_handle_write(
          program_id: program_id,
          budget: budget,
          value: value,
          max_stack: max_stack
        )
      end

      def gpio_handle_close!(
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        max_stack: 1
      )
        ensure_uno_r4_wifi!

        session.gpio_handle_close(
          program_id: program_id,
          budget: budget,
          max_stack: max_stack
        )
      end

      def time_now!(
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        host_nonce: DEFAULT_HOST_NONCE,
        max_stack: 1
      )
        ensure_uno_r4_wifi!

        session.time_now(
          program_id: program_id,
          budget: budget,
          max_stack: max_stack,
          handshake: true,
          query_caps: true,
          host_nonce: host_nonce
        )
      end

      def time_sleep_ms!(
        duration_ms:,
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        host_nonce: DEFAULT_HOST_NONCE,
        max_stack: 1
      )
        ensure_uno_r4_wifi!

        session.time_sleep_ms(
          duration_ms: duration_ms,
          program_id: program_id,
          budget: budget,
          max_stack: max_stack,
          handshake: true,
          query_caps: true,
          host_nonce: host_nonce
        )
      end

      def store_program!(
        program_id: DEFAULT_PROGRAM_ID,
        slot: DEFAULT_EJECT_SLOT,
        boot_policy: DEFAULT_BOOT_POLICY
      )
        ensure_uno_r4_wifi!

        session.store_program(
          program_id: program_id,
          slot: slot,
          boot_policy: boot_policy
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

      def flash_uno_r4_wifi!
        result = runner.call(serial_usb_artifact_command(upload: true), chdir: cargo_workspace)
        handoff_port = self.class.parse_new_upload_port(result.output)
        self.port = handoff_port if handoff_port
        result
      end

      def flash_esp32!
        unless @firmware_image
          raise ArgumentError, "ESP flash requires firmware_image: or esp_image:"
        end

        result = runner.call(
          board_vm_cli_command(
            *BoardVM.esp_upload_command(
              board,
              port: port,
              image: @firmware_image,
              **esp_upload_overrides
            )
          ),
          chdir: cargo_workspace
        )
        result
      end

      def flash_pico_uf2!
        unless @firmware_image
          raise ArgumentError, "Pico UF2 flash requires firmware_image:"
        end

        result = runner.call(
          board_vm_cli_command(
            *BoardVM.pico_uf2_upload_command(
              board,
              image: @firmware_image,
              mount: @pico_uf2_mount,
              roots: @pico_uf2_mount_roots,
              **pico_uf2_upload_overrides
            )
          ),
          chdir: cargo_workspace
        )
        rediscover_pico_runtime_port! if @pico_runtime_port
        result
      end

      def rediscover_pico_runtime_port!
        timeout_seconds = @pico_runtime_port_wait_ms.to_f / 1000.0
        deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + timeout_seconds
        last_error = nil

        loop do
          begin
            selected = BoardVM.select_runtime_device(board: board, devices: @device_discovery.call)
            self.port = selected.fetch("port")
            return selected
          rescue DeviceSelectionError => error
            last_error = error
          end

          break if timeout_seconds <= 0 || Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline

          remaining = deadline - Process.clock_gettime(Process::CLOCK_MONOTONIC)
          poll_seconds = [@pico_runtime_port_poll_ms.to_f / 1000.0, 0.01].max
          sleep [poll_seconds, remaining].min
        end

        raise DeviceSelectionError,
          "Pico UF2 upload finished, but no runtime serial device was found for #{board.inspect}.\n#{last_error.message}"
      end

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

      def esp_upload_overrides
        overrides = {}
        @esp_upload_options.each do |key, value|
          overrides[key.to_sym] = value
        end
        overrides[:baud_rate] = baud_rate unless overrides.key?(:baud_rate)
        overrides[:timeout_ms] = timeout_ms unless overrides.key?(:timeout_ms)
        overrides
      end

      def pico_uf2_upload_overrides
        overrides = {}
        @pico_uf2_upload_options.each do |key, value|
          overrides[key.to_sym] = value
        end
        overrides
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
        return @transport if @transport

        if tcp_endpoint_connection?
          unless endpoint && !endpoint.to_s.empty?
            raise TransportError,
              "#{connection_display_name} requires a Board VM TCP endpoint; pass endpoint: \"tcp://host:port\" or choose via: :serial"
          end

          return @transport = TcpTransport.new(endpoint: endpoint, timeout_ms: timeout_ms)
        end

        if bluetooth_endpoint_connection?
          unless endpoint && !endpoint.to_s.empty?
            raise TransportError,
              "#{connection_display_name} requires a Board VM Bluetooth endpoint; pass endpoint: ... or choose via: :serial"
          end

          return @transport = BluetoothTransport.new(
            endpoint: endpoint,
            timeout_ms: timeout_ms,
            backend: @bluetooth_backend_plan
          )
        end

        unless serial_connection?
          raise TransportError,
            "#{connection_display_name} requires an injected Board VM transport endpoint; pass transport: or choose via: :serial"
        end
        if port.nil? || port.to_s.empty?
          raise TransportError, "USB/serial Board VM connection requires a serial port"
        end

        @transport = SerialTransport.new(port: port, baud_rate: baud_rate, timeout_ms: timeout_ms)
      end

      def tcp_endpoint_connection?
        connection_option &&
          (connection_option["endpoint_transport"] == "tcp_socket" ||
            connection_option["endpoint_scheme"] == "tcp")
      end

      def bluetooth_endpoint_connection?
        connection_option &&
          (["bluetooth_le", "bluetooth_classic"].include?(connection_option["transport"]) ||
            ["bluetooth_le_gatt", "bluetooth_classic_rfcomm"].include?(connection_option["endpoint_transport"]) ||
            ["ble", "btspp", "rfcomm"].include?(connection_option["endpoint_scheme"]))
      end

      def connection_display_name
        return "Board VM connection" unless connection_option

        connection_option["display_name"] || connection_option["transport"] || "Board VM connection"
      end

      def pico_board?
        %i[raspberry_pi_pico raspberry_pi_pico_w].include?(board)
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

    class TimeApi
      def initialize(connection)
        @connection = connection
      end

      def now_ms(
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        host_nonce: DEFAULT_HOST_NONCE,
        max_stack: 1
      )
        @connection.time_now!(
          program_id: program_id,
          budget: budget,
          host_nonce: host_nonce,
          max_stack: max_stack
        )
      end

      def sleep_ms(
        duration_ms,
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        host_nonce: DEFAULT_HOST_NONCE,
        max_stack: 1
      )
        @connection.time_sleep_ms!(
          duration_ms: duration_ms,
          program_id: program_id,
          budget: budget,
          host_nonce: host_nonce,
          max_stack: max_stack
        )
      end
    end

    class Gpio
      def initialize(connection)
        @connection = connection
      end

      def read(
        pin:,
        mode: :input,
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        host_nonce: DEFAULT_HOST_NONCE,
        max_stack: 2
      )
        @connection.gpio_read!(
          program_id: program_id,
          budget: budget,
          host_nonce: host_nonce,
          pin: pin,
          mode: mode,
          max_stack: max_stack
        )
      end

      def write(
        pin:,
        value:,
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        host_nonce: DEFAULT_HOST_NONCE,
        max_stack: 3
      )
        @connection.gpio_write!(
          program_id: program_id,
          budget: budget,
          host_nonce: host_nonce,
          pin: pin,
          value: value,
          max_stack: max_stack
        )
      end

      def high(pin:, **options)
        write(pin: pin, value: true, **options)
      end

      def low(pin:, **options)
        write(pin: pin, value: false, **options)
      end

      def open(
        pin:,
        mode: :output,
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        host_nonce: DEFAULT_HOST_NONCE,
        max_stack: 2
      )
        @connection.gpio_open!(
          program_id: program_id,
          budget: budget,
          host_nonce: host_nonce,
          pin: pin,
          mode: mode,
          max_stack: max_stack
        )
      end

      def handle_read(
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        max_stack: 2
      )
        @connection.gpio_handle_read!(
          program_id: program_id,
          budget: budget,
          max_stack: max_stack
        )
      end

      def handle_write(
        value:,
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        max_stack: 3
      )
        @connection.gpio_handle_write!(
          program_id: program_id,
          budget: budget,
          value: value,
          max_stack: max_stack
        )
      end

      def handle_close(
        program_id: DEFAULT_PROGRAM_ID,
        budget: DEFAULT_INSTRUCTION_BUDGET,
        max_stack: 1
      )
        @connection.gpio_handle_close!(
          program_id: program_id,
          budget: budget,
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
