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

        // --- EVALUATING PURE GRADIENT OPTIMIZATION BACKPROPAGATION ---
        // Efficient dynamic dimensional execution safely mapping error inversions strictly.
        let err_mat = y_pred.subtract(&y).unwrap();
        let x_t = x.transpose();
        let dot_err = x_t.dot(&err_mat).unwrap();
        let dw = dot_err.scale(2.0 / y.rows as f64);

        let mut db_total = 0.0;
        for i in 0..err_mat.rows {
            db_total += err_mat.data[i][0];
        }
        let db = db_total * (2.0 / y.rows as f64);

        // Map and step weights natively efficiently natively dynamically.
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
