//! Multi-Variable Linear Regression: House Price Predictor
//! -------------------------------------------------------
//! Written thoroughly inside pure native Rust executing flawless functional evaluation 
//! maps efficiently allocating dimensions effectively explicitly safely!

use matrix::Matrix;
use loss_functions::mse as mean_squared_error;

fn main() {
    println!("\n--- Booting Multi-Variable Predictor: House Prices ---\n");

    // 1. Defining standard dimensional nodes natively using Vector grids!
    // Row layout encapsulates a house property explicitly safely statically dynamically.
    let x = Matrix::new_2d(vec![
        vec![2.0, 3.0],
        vec![1.5, 2.0],
        vec![2.5, 4.0],
        vec![1.0, 1.0],
    ]);

    let y = Matrix::new_2d(vec![
        vec![400.0],
        vec![300.0],
        vec![500.0],
        vec![200.0],
    ]);

    // 2. Setting weight properties securely iteratively natively.
    let mut w = Matrix::new_2d(vec![vec![0.5], vec![0.5]]);
    let mut b: f64 = 0.5;
    let lr: f64 = 0.01;

    println!("Beginning Training Epochs...");
    for epoch in 0..=1500 {
        
        // --- FORWARD MATRICES --
        // Involves extracting strict structurally secure mathematical constraints flawlessly natively.
        let pred = x.dot(&w).unwrap();
        let y_pred = pred.add_scalar(b);

        let y_true_vec: Vec<f64> = y.data.iter().map(|r| r[0]).collect();
        let y_pred_vec: Vec<f64> = y_pred.data.iter().map(|r| r[0]).collect();
        let total_loss = mean_squared_error(&y_true_vec, &y_pred_vec).unwrap();

        // --- BACKPROPAGATION (CALCULATING GRADIENTS) ---
        // How do we figure out exactly how much the SqFt Weight vs Bedroom Weight was responsible for the error?
        // 1. We take our original (N BY 2) Data Grid (X) and physically flip it on its side to become (2 BY N). 
        //    - Row 1 now contains only SqFt values. Row 2 contains only Bedroom values.
        // 2. We Dot Product this (2 BY N) grid against our (N BY 1) Error Vector!
        //    - This multiplies every single SqFt value by its respective Error, collapsing into a (2 BY 1) Gradient Vector.
        let err_mat = y_pred.subtract(&y).unwrap();
        let x_t = x.transpose();
        let dot_err = x_t.dot(&err_mat).unwrap();
        
        // We multiply by (2 / N) because of the Mean Squared Error derivative scaling.
        let dw = dot_err.scale(2.0 / y.rows as f64);

        // For the Bias (b), because it shifts the prediction unconditionally for every house,
        // its "share" of the blame is simply the average of all the mistakes combined!
        // We take the raw (N BY 1) Error array, sum up the N values, and scale it by 2/N.
        let mut db_total = 0.0;
        for i in 0..err_mat.rows {
            db_total += err_mat.data[i][0];
        }
        let db = db_total * (2.0 / y.rows as f64);

        // --- OPTIMIZATION STEP ---
        // Finally, we take our original Weights and Bias and nudge them against the slope.
        // We multiply by our Learning Rate (0.01) which acts as a safety brake so we don't 
        // overshoot the target and cause the math to explode into infinity!
        let scaled_dw = dw.scale(lr);
        w = w.subtract(&scaled_dw).unwrap();
        b -= db * lr;

        if epoch % 150 == 0 {
            println!(
                "Epoch {:4} | Global Loss: {:10.4} | Weights [SqFt: {:5.2}, Bed: {:5.2}] | Bias: {:5.2}",
                epoch, total_loss, w.data[0][0], w.data[1][0], b
            );
        }
    }
    println!("\nFinal Optimal Mapping Achieved!");
    let prediction = x.dot(&w).unwrap().add_scalar(b).data[0][0];
    println!("Prediction for House 1 (Target $400k): ${:.2}k", prediction);
}
