require_relative '../lib/gradient_descent'

RSpec.describe GradientDescent do
  it 'computes sgd' do
    res = GradientDescent.sgd([1.0, 2.0], [0.1, 0.2], 0.5)
    expect(res[0]).to be_within(0.0001).of(0.95)
    expect(res[1]).to be_within(0.0001).of(1.9)
  end
end
