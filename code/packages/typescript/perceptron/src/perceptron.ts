import { Matrix } from '../../matrix/src/matrix';
import { bce, bceDerivative } from '../../loss-functions/src/loss_functions';
import { sigmoid, sigmoidDerivative } from '../../activation-functions/src/activations';

export class Perceptron {
    learningRate: number;
    epochs: number;
    weights: Matrix | null;
    bias: number;

    constructor(learningRate: number = 0.1, epochs: number = 2000) {
        this.learningRate = learningRate;
        this.epochs = epochs;
        this.weights = null;
        this.bias = 0.0;
    }

    fit(xData: number[][], yData: number[][], logSteps: number): void {
        const features = new Matrix(xData);
        const trueLabels = new Matrix(yData);

        const wData = Array.from({ length: features.cols }, () => [0.0]);
        this.weights = new Matrix(wData);
        this.bias = 0.0;

        for (let epoch = 0; epoch <= this.epochs; epoch++) {
            let raw = features.dot(this.weights!);
            raw = raw.add(this.bias);

            const linearProbs: number[] = [];
            const linearTruth: number[] = [];
            const gradData: number[][] = [];

            for (let i = 0; i < features.rows; i++) {
                linearProbs.push(sigmoid(raw.data[i][0]));
                linearTruth.push(trueLabels.data[i][0]);
            }

            const logLoss = bce(linearTruth, linearProbs);
            const lossGrad = bceDerivative(linearTruth, linearProbs);

            let biasGrad = 0.0;
            for (let i = 0; i < features.rows; i++) {
                const actGrad = sigmoidDerivative(raw.data[i][0]);
                const combined = lossGrad[i] * actGrad;
                gradData.push([combined]);
                biasGrad += combined;
            }

            const gradMatrix = new Matrix(gradData);
            const weightGrads = features.transpose().dot(gradMatrix);

            const scaledWeights = weightGrads.scale(this.learningRate);
            this.weights = this.weights!.subtract(scaledWeights);
            this.bias -= biasGrad * this.learningRate;

            if (epoch % logSteps === 0) {
                console.log(`Epoch ${epoch.toString().padStart(4, ' ')} | BCE Loss: ${logLoss.toFixed(4)} | Bias: ${this.bias.toFixed(2)}`);
            }
        }
    }

    predict(xData: number[][]): number[] {
        if (!this.weights) {
            throw new Error("Predict called before fit()");
        }

        const features = new Matrix(xData);
        let raw = features.dot(this.weights);
        raw = raw.add(this.bias);

        const predictions: number[] = [];
        for (let i = 0; i < features.rows; i++) {
            predictions.push(sigmoid(raw.data[i][0]));
        }
        return predictions;
    }
}
