module github.com/adhithyan15/coding-adventures/code/programs/go/house-price-predictor

go 1.21

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/feature-normalization v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/loss-functions v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/matrix v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/feature-normalization => ../../../packages/go/feature-normalization
	github.com/adhithyan15/coding-adventures/code/packages/go/loss-functions => ../../../packages/go/loss-functions
	github.com/adhithyan15/coding-adventures/code/packages/go/matrix => ../../../packages/go/matrix
)
