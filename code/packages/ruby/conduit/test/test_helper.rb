# frozen_string_literal: true

require "minitest/autorun"
require "net/http"
require "socket"

ENV["CONDUIT_DISABLE_NATIVE"] = "1"

require "coding_adventures_conduit"
