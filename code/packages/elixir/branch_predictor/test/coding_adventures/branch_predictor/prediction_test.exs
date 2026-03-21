defmodule CodingAdventures.BranchPredictor.PredictionTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BranchPredictor.Prediction

  # ── Struct creation ─────────────────────────────────────────────────────

  test "new/1 with keyword list creates prediction" do
    pred = Prediction.new(predicted_taken: true, confidence: 0.9, address: 0x400)
    assert pred.predicted_taken == true
    assert pred.confidence == 0.9
    assert pred.address == 0x400
  end

  test "new/1 with map creates prediction" do
    pred = Prediction.new(%{predicted_taken: false, confidence: 0.5, address: 0x200})
    assert pred.predicted_taken == false
    assert pred.confidence == 0.5
    assert pred.address == 0x200
  end

  test "new/1 defaults confidence to 0.0" do
    pred = Prediction.new(predicted_taken: true)
    assert pred.confidence == 0.0
  end

  test "new/1 defaults address to nil" do
    pred = Prediction.new(predicted_taken: false)
    assert pred.address == nil
  end

  test "new/1 with all defaults" do
    pred = Prediction.new(predicted_taken: false)
    assert pred.predicted_taken == false
    assert pred.confidence == 0.0
    assert pred.address == nil
  end

  # ── Field access ─────────────────────────────────────────────────────────

  test "predicted_taken field is accessible" do
    pred = %Prediction{predicted_taken: true}
    assert pred.predicted_taken == true
  end

  test "confidence field is accessible" do
    pred = %Prediction{predicted_taken: false, confidence: 0.75}
    assert pred.confidence == 0.75
  end

  test "address field is accessible" do
    pred = %Prediction{predicted_taken: true, address: 0x1000}
    assert pred.address == 0x1000
  end

  # ── Struct immutability ──────────────────────────────────────────────────

  test "prediction struct is immutable — creating another doesn't change original" do
    pred1 = Prediction.new(predicted_taken: true, confidence: 0.9)
    _pred2 = Prediction.new(predicted_taken: false, confidence: 0.1)
    assert pred1.predicted_taken == true
    assert pred1.confidence == 0.9
  end

  # ── Edge cases ──────────────────────────────────────────────────────────

  test "address can be 0 (valid PC address)" do
    pred = Prediction.new(predicted_taken: true, address: 0)
    assert pred.address == 0
  end

  test "confidence can be exactly 0.0" do
    pred = Prediction.new(predicted_taken: false, confidence: 0.0)
    assert pred.confidence == 0.0
  end

  test "confidence can be exactly 1.0" do
    pred = Prediction.new(predicted_taken: true, confidence: 1.0)
    assert pred.confidence == 1.0
  end

  test "address can be a large integer" do
    pred = Prediction.new(predicted_taken: true, address: 0xFFFF_FFFF)
    assert pred.address == 0xFFFF_FFFF
  end
end
