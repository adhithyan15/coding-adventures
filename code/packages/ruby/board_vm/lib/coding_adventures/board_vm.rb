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

    def pico_uf2_upload_command(
      board = :raspberry_pi_pico,
      image:,
      mount: nil,
      **overrides
    )
      options = pico_uf2_upload_options(board, **overrides)
      unless options
        raise UnsupportedBoardError, "Pico UF2 upload is not supported for #{board.inspect}"
      end

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
      cargo_workspace: DEFAULT_RUST_WORKSPACE,
      runner: CommandRunner.new,
      transport: nil,
      **options
    )
      if pick && port.nil? && device.nil?
        device = pick_device(board: board, devices: devices, input: input, output: output)
      end

      selection = connection_selection(board: board, port: port, device: device, devices: devices)
      connection = Connection.new(
        board: selection.fetch(:board),
        port: selection.fetch(:port),
        cargo_workspace: cargo_workspace,
        runner: runner,
        transport: transport,
        baud_rate: options.delete(:baud_rate) || options.delete(:baud) || DEFAULT_BAUD_RATE,
        timeout_ms: options.delete(:timeout_ms) || DEFAULT_TIMEOUT_MS,
        **options
      )
      connection.flash! if flash

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

    def connection_selection(board:, port:, device:, devices:)
      explicit_port = !port.nil?
      selected_device = resolve_device_reference(device, devices: devices) if device
      selected_device ||= select_device(board: board, devices: devices) if port.nil?
      port ||= selected_device && selected_device.fetch("port")

      normalized_board = if board == :auto
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

      { board: normalized_board, port: port }
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
  end
end
