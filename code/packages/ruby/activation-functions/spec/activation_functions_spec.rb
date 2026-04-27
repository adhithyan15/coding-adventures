require_relative '../lib/activation_functions'

RSpec.describe ActivationFunctions do
  it 'computes sigmoid' do
    expect(ActivationFunctions.sigmoid(0)).to be_within(0.0001).of(0.5)
  end

  it 'computes relu' do
    expect(ActivationFunctions.relu(5)).to eq(5.0)
    expect(ActivationFunctions.relu(-5)).to eq(0.0)
  end

  it 'computes tanh' do
    expect(ActivationFunctions.tanh(0)).to be_within(0.0001).of(0.0)
  end
end
