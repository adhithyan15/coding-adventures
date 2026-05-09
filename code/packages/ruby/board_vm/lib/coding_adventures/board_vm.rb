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

    def esp_upload_command(board = :esp32_devkit_v1, port:, image:, **overrides)
      options = esp_upload_options(board, **overrides)
      unless options
        raise UnsupportedBoardError, "ESP upload is not supported for #{board.inspect}"
      end

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
      board: :uno_r4_wifi,
      port:,
      flash: false,
      cargo_workspace: DEFAULT_RUST_WORKSPACE,
      runner: CommandRunner.new,
      transport: nil,
      **options
    )
      connection = Connection.new(
        board: normalize_board(board),
        port: port,
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
