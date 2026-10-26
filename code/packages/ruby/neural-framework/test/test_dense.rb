# frozen_string_literal: true

require_relative "test_helper"

class DenseTests < Minitest::Test
  include FixtureAssertions

  def test_paper_every_gradient_and_update
    paper = fixture("dense.json").fetch("paper")
    model = NET.from_checkpoint(paper.fetch("checkpoint"))
    trace = model.gradients(paper["inputs"], paper["targets"])
    close_data(trace.predictions, paper["predictions"], 1e-12)
    close_data(trace.loss, paper["loss"], 1e-12)
    close_data(trace.weight_gradients, paper["weight_gradients"], 1e-12)
    close_data(trace.bias_gradients, paper["bias_gradients"], 1e-12)
    before = model.train_step(paper["inputs"], paper["targets"], paper["learning_rate"])
    assert_equal trace, before
    close_data(model.to_checkpoint, paper["updated_checkpoint"], 1e-12)
    close_data(model.predict(paper["inputs"]), paper["updated_predictions"], 1e-12)
    close_data(model.gradients(paper["inputs"], paper["targets"]).loss, paper["updated_loss"], 1e-12)
  end

  def test_seed_and_native_xor_resume_full_model
    data = fixture("dense.json")
    seed = data["seed"]
    close_data(NET.seeded(seed["widths"], seed["activations"], seed: seed["value"]).to_checkpoint["layers"][0]["weights"], seed["first_weights"], 1e-12)
    xor = data["xor"]
    model = NET.seeded(xor["widths"], xor["activations"], seed: xor["seed"])
    losses = model.fit(xor["inputs"], xor["targets"], 3000, 1.0)
    assert_equal 3001, losses.length
    assert_operator losses[-1], :<, 0.01
    resumed = NET.from_json(model.to_json)
    2.times do |i|
      model.train_step(xor["inputs"], xor["targets"], 1.0)
      resumed.train_step(xor["inputs"], xor["targets"], 1.0)
      assert_equal 3001 + i, resumed.step
      close_data(resumed.to_checkpoint, model.to_checkpoint)
      close_data(resumed.predict(xor["inputs"]), model.predict(xor["inputs"]))
      close_data(resumed.gradients(xor["inputs"], xor["targets"]).loss, losses[-1], 1e-5)
    end
  end

  def test_every_activation_and_parameter_central_finite_difference
    inputs = [[0.2, 0.7], [-0.3, 0.6]]
    targets = [[0.1, -0.2], [0.3, 0.4]]
    %w[linear sigmoid tanh relu].each do |activation|
      model = NET.seeded([2, 3, 2, 2], [activation] * 3, seed: 19)
      if activation == "relu"
        active = model.to_checkpoint
        active["layers"].each do |layer|
          layer["weights"].each { |row| row.map! { |value| value.abs + 0.1 } }
          layer["biases"].fill(1.0)
        end
        model = NET.from_checkpoint(active)
      end
      trace = model.gradients(inputs, targets)
      if activation == "relu"
        assert trace.weight_gradients.flatten.all? { |value| value.abs > 1e-8 }
        assert trace.bias_gradients.flatten.all? { |value| value.abs > 1e-8 }
      end
      base = model.to_checkpoint
      base["layers"].each_with_index do |layer, i|
        ["weights", "biases"].each do |field|
          coordinates = if field == "weights"
            layer[field].each_index.flat_map { |r| layer[field][r].each_index.map { |c| [r, c] } }
          else
            Array.new(layer[field].length) { |j| [j] }
          end
          coordinates.each do |coordinate|
            values = [-1, 1].map do |sign|
              cp = JSON.parse(JSON.generate(base))
              array = cp["layers"][i][field]
              array = array[coordinate[0]] if coordinate.length == 2
              array[coordinate[-1]] += sign * 1e-6
              NET.from_checkpoint(cp).gradients(inputs, targets).loss
            end
            expected = (values[1] - values[0]) / 2e-6
            actual = (field == "weights") ? trace.weight_gradients[i][coordinate[0]][coordinate[1]] : trace.bias_gradients[i][coordinate[0]]
            assert_in_delta expected, actual, 1e-6, "#{activation} #{i} #{field} #{coordinate}"
          end
        end
      end
    end
  end

  def test_shared_55_case_corpus
    corpus = fixture("checkpoints.json")
    assert_equal %w[cases contract], corpus.keys.sort
    assert_equal "NN40", corpus["contract"]
    cases = corpus["cases"]
    assert_equal 55, cases.length
    assert_equal 55, cases.map { |c| c["id"] }.uniq.length
    assert_equal 3, cases.count { |c| c["accept"] }
    %w[null-weight-cell null-weight-cell-mixed null-bias-cell null-bias-cell-mixed null-weight-cell-later-layer null-bias-cell-later-layer].each do |id|
      assert_equal false, cases.find { |c| c["id"] == id }.fetch("accept")
    end
    cases.each do |entry|
      assert_equal %w[accept checkpoint id], entry.keys.sort
      if entry["accept"]
        model = NET.from_checkpoint(entry["checkpoint"])
        close_data(model.to_checkpoint, entry["checkpoint"])
        assert_equal model.to_checkpoint, NET.from_json(model.to_json).to_checkpoint
      else
        assert_raises(ArgumentError, entry["id"]) { NET.from_checkpoint(entry["checkpoint"]) }
      end
    end
  end

  def test_extreme_activation_and_zero_fit
    assert_equal [[1.0]], linear(1.0, 0.0, "sigmoid").predict([[1000]])
    assert_equal [[0.0]], linear(1.0, 0.0, "sigmoid").predict([[-1000]])
    assert_equal [[[0.0]]], linear(1.0, 0.0, "relu").gradients([[0]], [[1]]).weight_gradients
    model = linear
    checkpoint = model.to_checkpoint
    assert_equal [1.0], model.fit([[1]], [[0]], 0, 1.0)
    assert_equal checkpoint, model.to_checkpoint
  end
end
