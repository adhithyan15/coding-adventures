# frozen_string_literal: true

require_relative "board_vm/version"
require_relative "board_vm/capabilities"
require_relative "board_vm/command_runner"
require_relative "board_vm/native"
require_relative "board_vm/transport"
require_relative "board_vm/session"
require_relative "board_vm/connection"

module CodingAdventures
  module BoardVM
    DEFAULT_RUST_WORKSPACE = File.expand_path("../../../../rust", __dir__)
    DEFAULT_BAUD_RATE = 115_200
    DEFAULT_TIMEOUT_MS = 1_000
    DEFAULT_PROGRAM_ID = 1
    DEFAULT_INSTRUCTION_BUDGET = 12
    DEFAULT_HOST_NONCE = 0xB0A2_D001
    DEFAULT_EJECT_SLOT = 0
    DEFAULT_BOOT_POLICY = :run_if_no_host
    DEFAULT_PICO_RUNTIME_PORT_WAIT_MS = 5_000
    DEFAULT_PICO_RUNTIME_PORT_POLL_MS = 250

    module_function

    def devices(paths: nil)
      return Native.discover_devices if paths.nil?

      Native.classify_devices(paths.map(&:to_s))
    end

    def device_list(device_candidates = nil)
      device_candidates ||= devices
      return "No Board VM devices found." if device_candidates.empty?

      device_candidates.each_with_index.map do |device, index|
        target = device["target"]
        target_name = target ? target.fetch("display_name") : "Unknown board"
        confidence = device.fetch("target_confidence", 0)
        confidence_label = confidence.positive? ? ", #{confidence}% match" : ""
        tags = Array(device["tags"])
        status = tags.empty? ? "" : " [#{tags.join(", ")}]"

        "#{index + 1}. #{target_name} - #{device.fetch("port")}#{confidence_label}#{status}"
      end.join("\n")
    end

    def select_device(board: :auto, devices: nil)
      candidates = devices || self.devices
      normalized_board = board == :auto ? :auto : normalize_board(board)
      matches = if normalized_board == :auto
        candidates.select { |device| device_target_board(device) }
      else
        exact_matches = candidates.select { |device| device_target_board(device) == normalized_board }
        exact_matches.empty? ? candidates.select { |device| device_target_board(device).nil? } : exact_matches
      end

      matches = candidates if normalized_board == :auto && matches.empty? && candidates.length == 1
      return matches.first if matches.length == 1

      if candidates.empty?
        raise DeviceSelectionError,
          "No Board VM devices found. Plug in a board or pass an explicit device."
      end

      reason = if matches.empty? && normalized_board == :auto
        "Multiple Board VM devices found; choose one"
      elsif matches.empty?
        "No matching Board VM device found"
      else
        "Multiple Board VM devices match"
      end
      raise DeviceSelectionError, "#{reason}.\n#{device_list(candidates)}"
    end

    def runtime_devices(board: :auto, devices: nil)
      candidates = (devices || self.devices).reject { |device| device.fetch("bootloader", false) }
      normalized_board = board == :auto ? :auto : normalize_board(board)
      if normalized_board == :auto
        matches = candidates.select { |device| device_target_board(device) }
        return candidates if matches.empty? && candidates.length == 1

        return matches
      end

      exact_matches = candidates.select { |device| device_target_board(device) == normalized_board }
      exact_matches.empty? ? candidates.select { |device| device_target_board(device).nil? } : exact_matches
    end

    def select_runtime_device(board: :auto, devices: nil)
      candidates = devices || self.devices
      matches = runtime_devices(board: board, devices: candidates)
      return matches.first if matches.length == 1

      if candidates.empty?
        raise DeviceSelectionError,
          "No Board VM runtime serial devices found. Plug in a board or pass an explicit port."
      end

      reason = if matches.empty?
        "No matching runtime serial device found"
      else
        "Multiple runtime serial devices match"
      end
      raise DeviceSelectionError, "#{reason}.\n#{device_list(candidates)}"
    end

    def pick_device(board: :auto, devices: nil, input: $stdin, output: $stdout)
      candidates = devices || self.devices
      raise DeviceSelectionError, "No Board VM devices found. Plug in a board." if candidates.empty?

      begin
        return select_device(board: board, devices: candidates)
      rescue DeviceSelectionError
        # Fall through to an explicit numbered picker for REPL/script use.
      end

      output.puts device_list(candidates)
      output.print "Select board [1-#{candidates.length}]: "
      choice = input.gets
      index = Integer(choice.to_s.strip, exception: false)
      unless index && index.between?(1, candidates.length)
        raise DeviceSelectionError, "Invalid Board VM device selection: #{choice.inspect}"
      end

      selected = candidates.fetch(index - 1)
      requested_board = board == :auto ? :auto : normalize_board(board)
      selected_board = device_target_board(selected)
      if requested_board != :auto && selected_board && selected_board != requested_board
        raise DeviceSelectionError,
          "Selected #{selected.fetch("port")} is #{selected_board}, not #{requested_board}."
      end

      selected
    end

    def known_targets
      Native.known_targets
    end

    def detect_target(selector)
      Native.detect_target(selector.to_s)
    end

    def find_target(board_id)
      detect_target(board_id)
    end

    def connection_options(board)
      target = detect_target(board)
      unless target
        raise UnsupportedBoardError, "unsupported board: #{board.inspect}"
      end

      target.fetch("connection_options")
    end

    def connection_option_list(board)
      options = connection_options(board)
      return "No Board VM connection options found for #{board}." if options.empty?

      options.each_with_index.map do |option, index|
        badges = []
        badges << "commands" if option["command_transport"]
        badges << "OTA" if option["ota_update"]
        badge_label = badges.empty? ? "" : " [#{badges.join(", ")}]"

        "#{index + 1}. #{option.fetch("display_name")}#{badge_label} - requires #{option.fetch("requires")}"
      end.join("\n")
    end

    def select_connection_option(board, transport: nil, ota: false)
      options = connection_options(board)
      matches = options.select { |option| option["command_transport"] }
      matches = matches.select { |option| option["ota_update"] } if ota

      if transport
        normalized_transport = normalize_connection_transport(transport)
        selected = options.find { |option| option["transport"] == normalized_transport }
        return selected if selected && (!ota || selected["ota_update"])

        raise DeviceSelectionError,
          "No #{normalized_transport} connection option for #{board.inspect}.\n#{connection_option_list(board)}"
      end

      serial = matches.find { |option| option["transport"] == "serial" }
      return serial if serial && !ota
      return matches.first if matches.length == 1

      reason = matches.empty? ? "No matching connection option" : "Multiple connection options match"
      raise DeviceSelectionError, "#{reason} for #{board.inspect}.\n#{connection_option_list(board)}"
    end

    def pick_connection_option(board, input: $stdin, output: $stdout)
      options = connection_options(board)
      raise DeviceSelectionError, "No Board VM connection options found for #{board.inspect}." if options.empty?

      output.puts connection_option_list(board)
      output.print "Select connection [1-#{options.length}]: "
      choice = input.gets
      index = Integer(choice.to_s.strip, exception: false)
      unless index && index.between?(1, options.length)
        raise DeviceSelectionError, "Invalid Board VM connection selection: #{choice.inspect}"
      end

      options.fetch(index - 1)
    end

    def esp_upload_options(board = :esp32_devkit_v1, **overrides)
      options = Native.esp_upload_options(board.to_s)
      return nil unless options

      overrides.each do |key, value|
        options[key.to_s] = value
      end
      options
    end

    def pico_uf2_upload_options(board = :raspberry_pi_pico, **overrides)
      options = Native.pico_uf2_upload_options(board.to_s)
      return nil unless options

      overrides.each do |key, value|
        options[key.to_s] = value
      end
      options
    end

    def pico_uf2_mounts(roots: nil)
      Native.pico_uf2_mounts(roots&.map(&:to_s))
    end

    def pico_uf2_mount(roots: nil)
      mounts = pico_uf2_mounts(roots: roots)
      return mounts.first if mounts.length == 1

      if mounts.empty?
        raise DeviceSelectionError,
          "No Pico BOOTSEL UF2 mount found. Hold BOOTSEL while plugging in the Pico/Pico W."
      end

      mount_list = mounts.each_with_index.map { |mount, index| "#{index + 1}. #{mount}" }.join("\n")
      raise DeviceSelectionError, "Multiple Pico BOOTSEL UF2 mounts found; choose one.\n#{mount_list}"
    end

    def pico_uf2_upload_command(
      board = :raspberry_pi_pico,
      image:,
      mount: nil,
      roots: nil,
      auto_mount: true,
      **overrides
    )
      options = pico_uf2_upload_options(board, **overrides)
      unless options
        raise UnsupportedBoardError, "Pico UF2 upload is not supported for #{board.inspect}"
      end

      mount ||= pico_uf2_mount(roots: roots) if auto_mount
      command = [
        options.fetch("command"),
        "--image", image.to_s
      ]
      command << "--mount" << mount.to_s unless mount.nil?
      command
    end

    def esp_upload_command(
      board = :esp32_devkit_v1,
      port: nil,
      device: nil,
      devices: nil,
      image:,
      **overrides
    )
      options = esp_upload_options(board, **overrides)
      unless options
        raise UnsupportedBoardError, "ESP upload is not supported for #{board.inspect}"
      end

      selected_device = resolve_device_reference(device, devices: devices) if device
      selected_device ||= select_device(board: board, devices: devices) if port.nil?
      port ||= selected_device && selected_device.fetch("port")

      command = [
        "esp-upload",
        "--port", port.to_s,
        "--image", image.to_s,
        "--baud", options.fetch("baud_rate").to_s,
        "--timeout-ms", options.fetch("timeout_ms").to_s,
        "--offset", options.fetch("offset").to_s,
        "--block-size", options.fetch("block_size").to_s
      ]
      flash_size = options["flash_size"]
      command << "--flash-size" << flash_size.to_s unless flash_size.nil?
      command << "--no-reset" unless options.fetch("reset_into_bootloader")
      command << "--no-verify" unless options.fetch("verify_md5")
      command << "--stay-in-bootloader" if options.fetch("stay_in_bootloader")
      command
    end

    def connect(
      board: :auto,
      port: nil,
      device: nil,
      devices: nil,
      pick: false,
      input: $stdin,
      output: $stdout,
      flash: false,
      smoke: false,
      cargo_workspace: DEFAULT_RUST_WORKSPACE,
      runner: CommandRunner.new,
      transport: nil,
      via: nil,
      connection_option: nil,
      pick_connection: false,
      **options
    )
      selection = connection_selection(
        board: board,
        port: port,
        device: device,
        devices: devices,
        flash: flash,
        via: via,
        connection_option: connection_option,
        pick_connection: pick_connection,
        pick: pick,
        input: input,
        output: output
      )
      connection = Connection.new(
        board: selection.fetch(:board),
        port: selection.fetch(:port),
        cargo_workspace: cargo_workspace,
        runner: runner,
        transport: transport,
        connection_option: selection.fetch(:connection_option),
        baud_rate: options.delete(:baud_rate) || options.delete(:baud) || DEFAULT_BAUD_RATE,
        timeout_ms: options.delete(:timeout_ms) || DEFAULT_TIMEOUT_MS,
        **options
      )
      connection.flash! if flash
      connection.smoke! if smoke

      return connection unless block_given?

      yield connection
      connection
    end

    def uno_r4_wifi(**options, &block)
      connect(board: :uno_r4_wifi, **options, &block)
    end

    def esp32_devkit_v1(**options, &block)
      connect(board: :esp32_devkit_v1, **options, &block)
    end

    def esp32(**options, &block)
      esp32_devkit_v1(**options, &block)
    end

    def raspberry_pi_pico(**options, &block)
      connect(board: :raspberry_pi_pico, **options, &block)
    end

    def pico(**options, &block)
      raspberry_pi_pico(**options, &block)
    end

    def raspberry_pi_pico_w(**options, &block)
      connect(board: :raspberry_pi_pico_w, **options, &block)
    end

    def pico_w(**options, &block)
      raspberry_pi_pico_w(**options, &block)
    end

    def connection_selection(
      board:,
      port:,
      device:,
      devices:,
      flash: false,
      via: nil,
      connection_option: nil,
      pick_connection: false,
      pick: false,
      input: $stdin,
      output: $stdout
    )
      explicit_port = !port.nil?
      normalized_board = board == :auto ? nil : normalize_board(board)
      selected_connection_option = if normalized_board
        resolve_connection_option(
          normalized_board,
          via: via,
          connection_option: connection_option,
          pick_connection: pick_connection,
          input: input,
          output: output
        )
      end
      needs_device_for_board = normalized_board.nil? && port.nil?
      needs_serial_port = connection_uses_serial_port?(selected_connection_option) &&
        !flash_without_port?(normalized_board || board, flash)
      selected_device = resolve_device_reference(device, devices: devices) if device

      if selected_device.nil? && port.nil? && pick && (needs_device_for_board || needs_serial_port)
        selected_device = pick_device(board: board, devices: devices, input: input, output: output)
      end
      if selected_device.nil? && port.nil? && (needs_device_for_board || needs_serial_port)
        selected_device = select_device(board: board, devices: devices)
      end
      port ||= selected_device && selected_device.fetch("port")

      normalized_board ||= if board == :auto
        inferred_board = selected_device && device_target_board(
          selected_device,
          minimum_confidence: 60
        )
        inferred_board ||= :uno_r4_wifi if explicit_port
        unless inferred_board
          raise DeviceSelectionError,
            "Could not infer the board for #{port || "the selected device"}.\n#{device_list(devices || self.devices)}"
        end
        inferred_board
      else
        normalize_board(board)
      end

      selected_connection_option ||= resolve_connection_option(
        normalized_board,
        via: via,
        connection_option: connection_option,
        pick_connection: pick_connection,
        input: input,
        output: output
      )

      { board: normalized_board, port: port, connection_option: selected_connection_option }
    end

    def resolve_connection_option(board, via:, connection_option:, pick_connection:, input:, output:)
      return normalize_connection_option_hash(connection_option) if connection_option
      return pick_connection_option(board, input: input, output: output) if pick_connection

      select_connection_option(board, transport: via)
    end

    def normalize_connection_option_hash(connection_option)
      connection_option.each_with_object({}) do |(key, value), normalized|
        normalized[key.to_s] = value
      end
    end

    def connection_uses_serial_port?(connection_option)
      connection_option.nil? || connection_option.fetch("transport") == "serial"
    end

    def flash_without_port?(board, flash)
      flash && pico_board_symbol?(board)
    end

    def pico_board_symbol?(board)
      %i[raspberry_pi_pico raspberry_pi_pico_w].include?(normalize_board(board))
    rescue UnsupportedBoardError
      false
    end

    def resolve_device_reference(device, devices: nil)
      return device if device.is_a?(Hash)

      candidates = devices || self.devices
      if device.is_a?(Integer)
        selected = candidates[device]
        raise DeviceSelectionError, "No Board VM device at index #{device}." unless selected

        return selected
      end

      needle = device.to_s
      selected = candidates.find do |candidate|
        candidate["id"] == needle || candidate["port"] == needle
      end
      unless selected
        raise DeviceSelectionError,
          "No Board VM device named #{needle.inspect}.\n#{device_list(candidates)}"
      end

      selected
    end

    def device_target_board(device, minimum_confidence: 0)
      return nil if device.fetch("target_confidence", 0).to_i < minimum_confidence

      target = device["target"]
      return nil unless target

      board_symbol(target.fetch("board_id"))
    end

    def normalize_board(board)
      target = detect_target(board)
      unless target
        raise UnsupportedBoardError, "unsupported board: #{board.inspect}"
      end

      board_symbol(target.fetch("board_id"))
    end

    def board_symbol(board_id)
      case board_id
      when "arduino-uno-r4-wifi"
        :uno_r4_wifi
      when "arduino-uno-r4-minima"
        :uno_r4_minima
      when "esp32-devkit-v1"
        :esp32_devkit_v1
      when "raspberry-pi-pico"
        :raspberry_pi_pico
      when "raspberry-pi-pico-w"
        :raspberry_pi_pico_w
      else
        board_id.to_s.tr("-", "_").to_sym
      end
    end

    def normalize_connection_transport(transport)
      normalized = transport.to_s.strip.downcase.tr("-", "_").tr(" /", "__")
      case normalized
      when "usb", "usb_serial", "serial_port"
        "serial"
      when "wi_fi", "wireless"
        "wifi"
      when "ble", "bluetooth", "bluetooth_low_energy"
        "bluetooth_le"
      else
        normalized
      end
    end
  end
end
