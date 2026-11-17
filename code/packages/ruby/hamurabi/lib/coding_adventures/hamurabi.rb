# frozen_string_literal: true

require_relative "hamurabi/version"
require_relative "hamurabi/errors"
require_relative "hamurabi/status"
require_relative "hamurabi/order"
require_relative "hamurabi/state"
require_relative "hamurabi/sim"

module CodingAdventures
  # Pure, deterministic simulation core for the seminal 1968 mainframe
  # resource allocation game Hamurabi.
  module Hamurabi
    extend self
  end
end
