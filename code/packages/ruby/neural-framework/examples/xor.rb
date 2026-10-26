# frozen_string_literal: true

require "neural_framework"

module CodingAdventures
  module NeuralFramework
    # A small native command, not a Python training wrapper. Receipt generation
    # and bounded checkpoint stdin are the only I/O; the reusable model stays pure.
    module XORExample
      module_function

      INPUTS = [[0, 0], [0, 1], [1, 0], [1, 1]].freeze
      TARGETS = [[0], [1], [1], [0]].freeze

      def run(arguments, input, output)
        raise ArgumentError, "expected no arguments or --resume" unless arguments.empty? || arguments == ["--resume"]
        model = if arguments.empty?
          DenseNetwork.seeded([2, 4, 1], %w[tanh sigmoid], seed: 7)
        else
          DenseNetwork.from_json(input.read(MAX_JSON_BYTES + 1))
        end
        initial = model.gradients(INPUTS, TARGETS).loss
        model.fit(INPUTS, TARGETS, arguments.empty? ? 3000 : 1, 1.0)
        output.puts JSON.generate({checkpoint: model.to_checkpoint, predictions: model.predict(INPUTS), initial_loss: initial, final_loss: model.gradients(INPUTS, TARGETS).loss}, allow_nan: false)
      end
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    CodingAdventures::NeuralFramework::XORExample.run(ARGV, $stdin, $stdout)
  rescue ArgumentError => error
    warn error.message
    exit 1
  end
end
