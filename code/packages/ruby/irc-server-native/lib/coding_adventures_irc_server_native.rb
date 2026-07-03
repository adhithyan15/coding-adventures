# frozen_string_literal: true

require_relative "coding_adventures/irc_server_native/version"

# Load the compiled native extension (lib/irc_server_native.{so,bundle,dll}),
# which runs Init_irc_server_native and defines
# CodingAdventures::IrcServerNative::NativeServer + ::Error.
require "irc_server_native"

require_relative "coding_adventures/irc_server_native/server"
