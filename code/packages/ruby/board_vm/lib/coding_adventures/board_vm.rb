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
    DEFAULT_CAPABILITIES_TIMEOUT_MS = 5_000
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

    def bluetooth_endpoint(endpoint)
      Native.bluetooth_endpoint(endpoint.to_s)
    end

    def bluetooth_backend(endpoint)
      Native.bluetooth_backend(endpoint.to_s)
    end

    def bluetooth_transact(endpoint, frame)
      Native.bluetooth_transact(endpoint.to_s, frame.b)
    end

    def bluetooth_devices
      Native.bluetooth_devices
    end

    def bluetooth_endpoint_candidates(devices = nil)
      devices ||= bluetooth_devices
      normalized = Array(devices).map do |device|
        id = bluetooth_device_field(device, "id")
        raise ArgumentError, "Bluetooth discovered device id is required" if id.nil?

        {
          "id" => id.to_s,
          "name" => bluetooth_optional_device_field(device, "name"),
          "address" => bluetooth_optional_device_field(device, "address"),
          "paired" => !!bluetooth_device_field(device, "paired"),
          "service_uuids" => bluetooth_device_array_field(device, "service_uuids").map(&:to_s),
          "characteristic_uuids" => bluetooth_device_array_field(device, "characteristic_uuids").map(&:to_s),
          "board_vm_rfcomm_channels" => bluetooth_device_array_field(device, "board_vm_rfcomm_channels").map { |channel| Integer(channel) }
        }
      end

      Native.bluetooth_endpoint_candidates(normalized)
    end

    def bluetooth_connection_endpoint(connection_option, devices: nil, pick: false, input: $stdin, output: $stdout)
      option = normalize_connection_option_hash(connection_option)
      matches = bluetooth_endpoint_matches(option, devices)

      return matches.first.fetch("endpoint").fetch("endpoint").to_s if matches.length == 1
      return pick_bluetooth_endpoint(option, candidates: matches, input: input, output: output) if pick

      display_name = option["display_name"] || option["transport"] || "Bluetooth"
      if matches.empty?
        raise DeviceSelectionError,
          "#{display_name} found no Board VM Bluetooth endpoints; " \
          "pair or power on the board, pass endpoint: ..., or choose via: :serial"
      end

      raise DeviceSelectionError,
        "Multiple Board VM Bluetooth endpoints match #{display_name}; pass endpoint: ...\n" \
        "#{bluetooth_endpoint_choice_list(matches)}"
    end

    def pick_bluetooth_endpoint(connection_option, devices: nil, candidates: nil, input: $stdin, output: $stdout)
      option = normalize_connection_option_hash(connection_option)
      matches = candidates || bluetooth_endpoint_matches(option, devices)
      return matches.first.fetch("endpoint").fetch("endpoint").to_s if matches.length == 1

      display_name = option["display_name"] || option["transport"] || "Bluetooth"
      if matches.empty?
        raise DeviceSelectionError,
          "#{display_name} found no Board VM Bluetooth endpoints; " \
          "pair or power on the board, pass endpoint: ..., or choose via: :serial"
      end

      output.puts bluetooth_endpoint_choice_list(matches)
      output.print "Select Bluetooth endpoint [1-#{matches.length}]: "
      choice = input.gets
      index = Integer(choice.to_s.strip, exception: false)
      unless index && index.between?(1, matches.length)
        raise DeviceSelectionError, "Invalid Board VM Bluetooth endpoint selection: #{choice.inspect}"
      end

      matches.fetch(index - 1).fetch("endpoint").fetch("endpoint").to_s
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
      endpoint: nil,
      bluetooth_devices: nil,
      via: nil,
      connection_option: nil,
      pick_connection: false,
      pick_bluetooth_endpoint: false,
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
      resolved_endpoint = endpoint
      if resolved_endpoint.nil? && connection_uses_bluetooth_endpoint?(selection.fetch(:connection_option))
        resolved_endpoint = bluetooth_connection_endpoint(
          selection.fetch(:connection_option),
          devices: bluetooth_devices,
          pick: pick || pick_bluetooth_endpoint,
          input: input,
          output: output
        )
      end
      connection = Connection.new(
        board: selection.fetch(:board),
        port: selection.fetch(:port),
        cargo_workspace: cargo_workspace,
        runner: runner,
        transport: transport,
        connection_option: selection.fetch(:connection_option),
        endpoint: resolved_endpoint,
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

    def connection_uses_bluetooth_endpoint?(connection_option)
      return false if connection_option.nil?

      %w[bluetooth_le bluetooth_classic].include?(connection_option["transport"]) ||
        %w[bluetooth_le_gatt bluetooth_classic_rfcomm].include?(connection_option["endpoint_transport"]) ||
        %w[ble btspp rfcomm].include?(connection_option["endpoint_scheme"])
    end

    def bluetooth_candidate_matches_connection?(candidate, connection_option)
      endpoint = candidate.fetch("endpoint", {})
      transport = connection_option["transport"]
      endpoint_transport = connection_option["endpoint_transport"]
      endpoint_scheme = connection_option["endpoint_scheme"]

      (transport && endpoint["transport"] == transport) ||
        (endpoint_transport && endpoint["endpoint_transport"] == endpoint_transport) ||
        (endpoint_scheme && endpoint["endpoint_scheme"] == endpoint_scheme)
    end

    def bluetooth_endpoint_matches(connection_option, devices)
      option = normalize_connection_option_hash(connection_option)
      bluetooth_endpoint_candidates(devices).select do |candidate|
        bluetooth_candidate_matches_connection?(candidate, option)
      end
    end

    def bluetooth_endpoint_choice_list(candidates)
      candidates.each_with_index.map do |candidate, index|
        endpoint = candidate.fetch("endpoint")
        display_name = candidate["display_name"] || candidate["device"] || endpoint.fetch("endpoint")
        pairing = bluetooth_candidate_pairing_status(candidate)
        "#{index + 1}. #{display_name}#{pairing} - #{endpoint.fetch("endpoint")}"
      end.join("\n")
    end

    def bluetooth_candidate_pairing_status(candidate)
      return " [pairing required]" if candidate["requires_pairing"]
      return " [paired]" if candidate["paired"]

      ""
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

    def bluetooth_device_field(device, key)
      return device[key] if device.respond_to?(:key?) && device.key?(key)

      symbol_key = key.to_sym
      return device[symbol_key] if device.respond_to?(:key?) && device.key?(symbol_key)

      nil
    end

    def bluetooth_optional_device_field(device, key)
      value = bluetooth_device_field(device, key)
      value.nil? ? nil : value.to_s
    end

    def bluetooth_device_array_field(device, key)
      Array(bluetooth_device_field(device, key))
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
