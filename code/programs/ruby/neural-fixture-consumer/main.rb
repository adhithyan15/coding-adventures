# frozen_string_literal: true

require 'json'

module NeuralFixtureConsumer
  LANE_ID = 'ruby-native'
  MAXIMUM_BYTES = 1 << 20

  class StrictHash < Hash
    def []=(key, value)
      raise JSON::ParserError, "duplicate key: #{key}" if key?(key)

      super
    end
  end

  module_function

  def exact_keys!(value, expected, context)
    raise ArgumentError, "#{context}: expected object" unless value.is_a?(Hash)

    actual = value.keys.sort
    wanted = expected.sort
    raise ArgumentError, "#{context}: unexpected keys" unless actual == wanted

    value
  end

  def finite_number!(value, context)
    number = Float(value)
    raise ArgumentError, "#{context}: expected finite number" unless value.is_a?(Numeric) && number.finite?

    number
  rescue TypeError, ArgumentError
    raise ArgumentError, "#{context}: expected finite number"
  end

  def load_fixture(path)
    stat = File.stat(path)
    unless stat.file? && stat.size.positive? && stat.size <= MAXIMUM_BYTES
      raise ArgumentError, 'fixture must be a non-empty regular file no larger than 1 MiB'
    end

    JSON.parse(
      File.binread(path, MAXIMUM_BYTES + 1),
      object_class: StrictHash,
      array_class: Array,
      create_additions: false,
      allow_nan: false,
      max_nesting: 32
    )
  rescue Errno::ENOENT, Errno::EACCES => e
    raise ArgumentError, "open fixture: #{e.message}"
  end

  def evaluate(document)
    exact_keys!(document, %w[schema_version id title stage question concepts model dataset training expected], 'fixture')
    raise ArgumentError, 'unsupported fixture identity' unless document['schema_version'] == 1 && document['id'] == 'weighted-neuron-forward' && document['stage'] == 'forward'
    raise ArgumentError, 'forward-only fixture must not contain training' unless document['training'].nil?

    model = exact_keys!(document['model'], %w[kind input_count layers], 'model')
    raise ArgumentError, 'expected one two-input neuron' unless model['kind'] == 'single-neuron' && model['input_count'] == 2 && model['layers'].is_a?(Array) && model['layers'].length == 1

    layer = exact_keys!(model['layers'][0], %w[name weights biases activation], 'layer')
    unless layer['name'] == 'output' && layer['activation'] == 'identity' && layer['weights'].is_a?(Array) && layer['weights'].length == 2 && layer['biases'].is_a?(Array) && layer['biases'].length == 1
      raise ArgumentError, 'unsupported layer contract'
    end

    dataset = exact_keys!(document['dataset'], %w[input_labels target_labels rows], 'dataset')
    expected = exact_keys!(document['expected'], %w[absolute_tolerance forward first_step], 'expected')
    raise ArgumentError, 'forward-only fixture must not contain a first step' unless expected['first_step'].nil?
    unless dataset['rows'].is_a?(Array) && dataset['rows'].length == 1 && expected['forward'].is_a?(Array) && expected['forward'].length == 1
      raise ArgumentError, 'expected one data row and one forward expectation'
    end

    row = exact_keys!(dataset['rows'][0], %w[label input target], 'row')
    forward = exact_keys!(expected['forward'][0], %w[row prediction], 'forward expectation')
    tolerance = finite_number!(expected['absolute_tolerance'], 'absolute tolerance')
    unless row['input'].is_a?(Array) && row['input'].length == 2 && forward['prediction'].is_a?(Array) && forward['prediction'].length == 1 && forward['row'] == row['label'] && tolerance.positive?
      raise ArgumentError, 'invalid row or expectation shape'
    end

    inputs = row['input'].each_with_index.map { |value, index| finite_number!(value, "input[#{index}]") }
    weights = layer['weights'].each_with_index.map do |vector, index|
      raise ArgumentError, 'each input must have one output weight' unless vector.is_a?(Array) && vector.length == 1

      finite_number!(vector[0], "weight[#{index}]")
    end
    bias = finite_number!(layer['biases'][0], 'bias')
    stored = finite_number!(forward['prediction'][0], 'stored prediction')
    contributions = inputs.zip(weights).map { |input, weight| input * weight }
    preactivation = contributions.sum(bias)
    maximum_error = (preactivation - stored).abs
    raise ArgumentError, 'non-finite arithmetic result' unless contributions.all?(&:finite?) && preactivation.finite? && maximum_error.finite?

    {
      'schema_version' => 1,
      'lane_id' => LANE_ID,
      'fixture_id' => document['id'],
      'row' => row['label'],
      'contributions' => contributions,
      'bias' => bias,
      'preactivation' => preactivation,
      'prediction' => [preactivation],
      'maximum_absolute_error' => maximum_error,
      'passes' => maximum_error <= tolerance
    }
  end

  def run(arguments, stdout: $stdout)
    unless arguments.length == 2 && arguments[0] == '--fixture' && !arguments[1].empty?
      raise ArgumentError, 'usage: neural-fixture-consumer --fixture PATH'
    end

    receipt = evaluate(load_fixture(arguments[1]))
    raise ArgumentError, 'prediction exceeded the fixture tolerance' unless receipt['passes']

    stdout.puts(JSON.generate(receipt))
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    NeuralFixtureConsumer.run(ARGV)
  rescue StandardError => e
    warn(e.message)
    exit(1)
  end
end
