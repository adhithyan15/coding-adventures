# frozen_string_literal: true

require_relative "coding_adventures/conduit/version"
require_relative "coding_adventures/conduit/request"
require_relative "coding_adventures/conduit/route"
require_relative "coding_adventures/conduit/router"
require_relative "coding_adventures/conduit/application"

unless ENV["CONDUIT_DISABLE_NATIVE"] == "1"
  require "conduit_native"
end

require_relative "coding_adventures/conduit/server"
