# frozen_string_literal: true

require_relative "board_vm/version"
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
      case board.to_s.tr("-", "_")
      when "uno_r4", "uno_r4_wifi", "arduino_uno_r4", "arduino_uno_r4_wifi"
        :uno_r4_wifi
      else
        raise UnsupportedBoardError, "unsupported board: #{board.inspect}"
      end
    end
  end
end
