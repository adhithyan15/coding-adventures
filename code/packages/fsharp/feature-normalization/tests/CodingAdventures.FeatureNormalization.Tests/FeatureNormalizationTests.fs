namespace CodingAdventures.FeatureNormalization.Tests

open System
open Xunit
open CodingAdventures.FeatureNormalization

type FeatureNormalizationTests() =
    let rows =
        [ [ 1000.0; 3.0; 1.0 ]
          [ 1500.0; 4.0; 0.0 ]
          [ 2000.0; 5.0; 1.0 ] ]

    let assertClose expected actual =
        Assert.True(abs (expected - actual) <= 1e-9, $"Expected {expected}, got {actual}.")

    [<Fact>]
    member _.``Standard scaler centers and scales columns``() =
        let scaler = FeatureNormalization.fitStandardScaler rows
        assertClose 1500.0 scaler.Means.[0]
        assertClose 4.0 scaler.Means.[1]

        let transformed = FeatureNormalization.transformStandard rows scaler
        assertClose -1.224744871391589 transformed.[0].[0]
        assertClose 0.0 transformed.[1].[0]
        assertClose 1.224744871391589 transformed.[2].[0]

    [<Fact>]
    member _.``Min-max scaler maps columns to unit range``() =
        let transformed =
            FeatureNormalization.transformMinMax rows (FeatureNormalization.fitMinMaxScaler rows)

        Assert.Equal<float list list>(
            [ [ 0.0; 0.0; 1.0 ]
              [ 0.5; 0.5; 0.0 ]
              [ 1.0; 1.0; 1.0 ] ],
            transformed)

    [<Fact>]
    member _.``Constant columns map to zero``() =
        let data = [ [ 1.0; 7.0 ]; [ 2.0; 7.0 ] ]
        let standard = FeatureNormalization.transformStandard data (FeatureNormalization.fitStandardScaler data)
        let minMax = FeatureNormalization.transformMinMax data (FeatureNormalization.fitMinMaxScaler data)
        assertClose 0.0 standard.[0].[1]
        assertClose 0.0 minMax.[0].[1]
