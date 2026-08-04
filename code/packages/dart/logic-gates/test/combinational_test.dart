import 'package:coding_adventures_logic_gates/coding_adventures_logic_gates.dart';
import 'package:test/test.dart';

void main() {
  group('multiplexers', () {
    test('mux2 and mux4 use LSB-first selects', () {
      expect(mux2(0, 1, 0), 0);
      expect(mux2(0, 1, 1), 1);
      final inputs = [0, 1, 0, 1];
      for (var index = 0; index < 4; index++) {
        expect(
          mux4(inputs[0], inputs[1], inputs[2], inputs[3], [
            index & 1,
            index >> 1,
          ]),
          inputs[index],
        );
      }
    });

    test('muxN selects 2, 8, and 16 inputs', () {
      expect(muxN([1, 0], [0]), 1);
      expect(mux8([0, 0, 0, 0, 0, 1, 0, 0], [1, 0, 1]), 1);
      final inputs = List<Bit>.generate(16, (index) => index == 13 ? 1 : 0);
      expect(muxN(inputs, [1, 0, 1, 1]), 1);
    });

    test('muxN validates shape and signals', () {
      expect(() => muxN([0, 1, 0], [0, 1]), throwsArgumentError);
      expect(() => muxN([0, 1, 0, 1], [0]), throwsArgumentError);
      expect(() => muxN([0, 2], [0]), throwsArgumentError);
    });
  });

  group('routing and coding', () {
    test('demux routes to exactly one output', () {
      expect(demux(1, [0]), [1, 0]);
      expect(demux(1, [1, 0]), [0, 1, 0, 0]);
      expect(demuxN(1, [1, 1], 4), [0, 0, 0, 1]);
      expect(demux(0, [1, 1]), [0, 0, 0, 0]);
    });

    test('decoder outputs every one-hot pattern', () {
      for (var index = 0; index < 8; index++) {
        final bits = [index & 1, (index >> 1) & 1, (index >> 2) & 1];
        final output = decoder(bits);
        expect(output.where((bit) => bit == 1).length, 1);
        expect(output[index], 1);
      }
    });

    test('encoder is the inverse of decoder', () {
      for (var index = 0; index < 8; index++) {
        final oneHot = List<Bit>.filled(8, 0)..[index] = 1;
        expect(encoder(oneHot), [
          index & 1,
          (index >> 1) & 1,
          (index >> 2) & 1,
        ]);
      }
    });

    test('encoder requires one-hot power-of-two input', () {
      expect(() => encoder([0, 0, 0, 0]), throwsArgumentError);
      expect(() => encoder([1, 1, 0, 0]), throwsArgumentError);
      expect(() => encoder([1, 0, 0]), throwsArgumentError);
    });

    test('priority encoder selects the highest active index', () {
      for (final testCase in [
        (inputs: [0, 0, 0, 0], encoded: [0, 0], valid: 0),
        (inputs: [1, 1, 1, 0], encoded: [0, 1], valid: 1),
        (inputs: [1, 0, 0, 1], encoded: [1, 1], valid: 1),
      ]) {
        final result = priorityEncoder(testCase.inputs);
        expect(result.$1, testCase.encoded);
        expect(result.$2, testCase.valid);
      }
    });

    test('tri-state output models high impedance with null', () {
      expect(triState(1, 1), 1);
      expect(triState(0, 1), 0);
      expect(triState(1, 0), isNull);
    });
  });
}
