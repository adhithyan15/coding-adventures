# frozen_string_literal: true

module CodingAdventures
  module CodegenCore
    Artifact = Data.define(:target, :format, :body, :metadata) do
      def initialize(target:, format:, body:, metadata: {})
        super
      end
    end
  end
end
