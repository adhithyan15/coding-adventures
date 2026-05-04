package perceptron

import (
	"fmt"
	activation "github.com/adhithyan15/coding-adventures/code/packages/go/activation-functions"
	loss "github.com/adhithyan15/coding-adventures/code/packages/go/loss-functions"
	"github.com/adhithyan15/coding-adventures/code/packages/go/matrix"
)

type Perceptron struct {
	LearningRate float64
	Epochs       int
	Weights      *matrix.Matrix
	Bias         float64
}

func New(lr float64, epochs int) *Perceptron {
	result, _ := StartNew[*Perceptron]("perceptron.New", nil,
		func(op *Operation[*Perceptron], rf *ResultFactory[*Perceptron]) *OperationResult[*Perceptron] {
			op.AddProperty("lr", lr)
			op.AddProperty("epochs", epochs)
			return rf.Generate(true, false, &Perceptron{
				LearningRate: lr,
				Epochs:       epochs,
				Bias:         0.0,
			})
		}).GetResult()
	return result
}

func (p *Perceptron) Fit(xData [][]float64, yData [][]float64, logSteps int) {
	_, _ = StartNew[struct{}]("perceptron.Fit", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("logSteps", logSteps)
			features := matrix.New2D(xData)
			trueLabels := matrix.New2D(yData)

			weightsData := make([][]float64, features.Cols)
			for i := range weightsData {
				weightsData[i] = []float64{0.0}
			}
			p.Weights = matrix.New2D(weightsData)
			p.Bias = 0.0

			for epoch := 0; epoch <= p.Epochs; epoch++ {
				raw, _ := features.Dot(p.Weights)
				raw = raw.AddScalar(p.Bias)

				linearProbs := make([]float64, features.Rows)
				linearTruth := make([]float64, features.Rows)
				gradData := make([][]float64, features.Rows)

				for i := 0; i < features.Rows; i++ {
					linearProbs[i] = activation.Sigmoid(raw.Data[i][0])
					linearTruth[i] = trueLabels.Data[i][0]
				}

				logLoss, _ := loss.BCE(linearTruth, linearProbs)
				lossGrad, _ := loss.BCED(linearTruth, linearProbs)

				var biasGrad float64
				for i := 0; i < features.Rows; i++ {
					actGrad := activation.SigmoidDerivative(raw.Data[i][0])
					combined := lossGrad[i] * actGrad
					gradData[i] = []float64{combined}
					biasGrad += combined
				}

				gradMatrix := matrix.New2D(gradData)
				transposed := features.Transpose()
				weightGrads, _ := transposed.Dot(gradMatrix)

				scaledWeights := weightGrads.Scale(p.LearningRate)
				p.Weights, _ = p.Weights.Subtract(scaledWeights)
				p.Bias -= biasGrad * p.LearningRate

				if epoch%logSteps == 0 {
					fmt.Printf("Epoch %4d | BCE Loss: %.4f | Bias: %.2f\n", epoch, logLoss, p.Bias)
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

func (p *Perceptron) Predict(xData [][]float64) []float64 {
	result, _ := StartNew[[]float64]("perceptron.Predict", nil,
		func(op *Operation[[]float64], rf *ResultFactory[[]float64]) *OperationResult[[]float64] {
			if p.Weights == nil {
				fmt.Println("Error: Predict called before Fit()")
				return rf.Generate(true, false, nil)
			}

			features := matrix.New2D(xData)
			raw, _ := features.Dot(p.Weights)
			raw = raw.AddScalar(p.Bias)

			predictions := make([]float64, features.Rows)
			for i := 0; i < features.Rows; i++ {
				predictions[i] = activation.Sigmoid(raw.Data[i][0])
			}
			return rf.Generate(true, false, predictions)
		}).GetResult()
	return result
}
