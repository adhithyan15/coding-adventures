# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_sha1"
require "coding_adventures_md5"

require_relative "coding_adventures/uuid/version"

module CodingAdventures
  # UUID v1/v3/v4/v5/v7 generation and parsing (RFC 4122 + RFC 9562)
  module Uuid
  end
end
