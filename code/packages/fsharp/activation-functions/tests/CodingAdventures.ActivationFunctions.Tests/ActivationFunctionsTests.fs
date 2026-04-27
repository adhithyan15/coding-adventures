namespace CodingAdventures.ActivationFunctions.Tests

open System
open Xunit
open CodingAdventures.ActivationFunctions

type ActivationFunctionsTests() =
    let assertClose expected actual tolerance =
        Assert.True(abs (expected - actual) <= tolerance, $"Expected {expected} but got {actual}.")

    [<Fact>]
    member _.``Linear and its derivative match the identity definition``() =
        assertClose -3.0 (ActivationFunctions.linear -3.0) 1e-12
        assertClose 0.0 (ActivationFunctions.linear 0.0) 1e-12
        assertClose 5.0 (ActivationFunctions.linear 5.0) 1e-12

        assertClose 1.0 (ActivationFunctions.linearDerivative -3.0) 1e-12
        assertClose 1.0 (ActivationFunctions.linearDerivative 0.0) 1e-12
        assertClose 1.0 (ActivationFunctions.linearDerivative 5.0) 1e-12

    [<Fact>]
    member _.``Sigmoid matches the spec vectors``() =
        assertClose 0.5 (ActivationFunctions.sigmoid 0.0) 1e-12
        assertClose 0.7310585786300049 (ActivationFunctions.sigmoid 1.0) 1e-12
        assertClose 0.2689414213699951 (ActivationFunctions.sigmoid -1.0) 1e-12
        assertClose 0.9999546021312976 (ActivationFunctions.sigmoid 10.0) 1e-12
        assertClose 0.0 (ActivationFunctions.sigmoid -710.0) 1e-12
        assertClose 1.0 (ActivationFunctions.sigmoid 710.0) 1e-12

    [<Fact>]
    member _.``Sigmoid derivative matches the spec vectors``() =
        assertClose 0.25 (ActivationFunctions.sigmoidDerivative 0.0) 1e-12
        assertClose 0.19661193324148185 (ActivationFunctions.sigmoidDerivative 1.0) 1e-12
        assertClose 0.00004539580773595167 (ActivationFunctions.sigmoidDerivative 10.0) 1e-12

    [<Fact>]
    member _.``ReLU and its derivative match the piecewise definition``() =
        assertClose 5.0 (ActivationFunctions.relu 5.0) 1e-12
        assertClose 0.0 (ActivationFunctions.relu -3.0) 1e-12
        assertClose 0.0 (ActivationFunctions.relu 0.0) 1e-12

        assertClose 1.0 (ActivationFunctions.reluDerivative 5.0) 1e-12
        assertClose 0.0 (ActivationFunctions.reluDerivative -3.0) 1e-12
        assertClose 0.0 (ActivationFunctions.reluDerivative 0.0) 1e-12

    [<Fact>]
    member _.``Leaky ReLU and its derivative keep the negative slope``() =
        assertClose 5.0 (ActivationFunctions.leakyRelu 5.0) 1e-12
        assertClose -0.03 (ActivationFunctions.leakyRelu -3.0) 1e-12
        assertClose 0.0 (ActivationFunctions.leakyRelu 0.0) 1e-12

        assertClose 1.0 (ActivationFunctions.leakyReluDerivative 5.0) 1e-12
        assertClose 0.01 (ActivationFunctions.leakyReluDerivative -3.0) 1e-12
        assertClose 0.01 (ActivationFunctions.leakyReluDerivative 0.0) 1e-12

    [<Fact>]
    member _.``Tanh and its derivative match the reference values``() =
        assertClose 0.0 (ActivationFunctions.tanh 0.0) 1e-12
        assertClose 0.7615941559557649 (ActivationFunctions.tanh 1.0) 1e-12
        assertClose -0.7615941559557649 (ActivationFunctions.tanh -1.0) 1e-12

        assertClose 1.0 (ActivationFunctions.tanhDerivative 0.0) 1e-12
        assertClose 0.41997434161402614 (ActivationFunctions.tanhDerivative 1.0) 1e-12

    [<Fact>]
    member _.``Softplus and its derivative match the reference values``() =
        assertClose 0.6931471805599453 (ActivationFunctions.softplus 0.0) 1e-12
        assertClose 1.3132616875182228 (ActivationFunctions.softplus 1.0) 1e-12
        assertClose 0.31326168751822286 (ActivationFunctions.softplus -1.0) 1e-12
        Assert.True(ActivationFunctions.softplus 1000.0 > 999.0)

        assertClose 0.5 (ActivationFunctions.softplusDerivative 0.0) 1e-12
        assertClose (ActivationFunctions.sigmoid 1.0) (ActivationFunctions.softplusDerivative 1.0) 1e-12
        assertClose (ActivationFunctions.sigmoid -1.0) (ActivationFunctions.softplusDerivative -1.0) 1e-12

    [<Fact>]
    member _.``Symmetry and range properties hold for representative samples``() =
        for sample in [| -6.0; -1.5; -0.5; 0.5; 1.5; 6.0 |] do
            let sigmoid = ActivationFunctions.sigmoid sample
            Assert.InRange(sigmoid, 0.0, 1.0)
            assertClose sigmoid (1.0 - ActivationFunctions.sigmoid (-sample)) 1e-10

            let tanh = ActivationFunctions.tanh sample
            Assert.InRange(tanh, -1.0, 1.0)
            assertClose tanh (-ActivationFunctions.tanh (-sample)) 1e-10

            Assert.True(ActivationFunctions.sigmoidDerivative sample >= 0.0)
            Assert.True(ActivationFunctions.reluDerivative sample >= 0.0)
            Assert.True(ActivationFunctions.leakyReluDerivative sample >= 0.0)
            Assert.True(ActivationFunctions.tanhDerivative sample >= 0.0)
            Assert.True(ActivationFunctions.softplusDerivative sample >= 0.0)
