# frozen_string_literal: true

module CodingAdventures
  module GraphNative
    class NodeNotFoundError < StandardError; end
    class EdgeNotFoundError < StandardError; end
    class NotConnectedError < StandardError; end
  end
end
