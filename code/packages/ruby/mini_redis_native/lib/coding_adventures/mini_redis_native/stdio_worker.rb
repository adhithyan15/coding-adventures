# frozen_string_literal: true

require_relative "worker"

CodingAdventures::MiniRedisNative.run_stdio_worker
