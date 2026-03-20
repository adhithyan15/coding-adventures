use perceptron::Perceptron;

fn main() {
    println!("\n--- Booting Rust Space Launch Predictor (OOP V2) ---");

    let shuttle_data = vec![
        vec![12.0, 15.0], vec![35.0, 85.0], vec![5.0, 5.0],
        vec![40.0, 95.0], vec![15.0, 30.0], vec![28.0, 60.0],
    ];
    let target_data = vec![
        vec![1.0], vec![0.0], vec![1.0], vec![0.0], vec![1.0], vec![0.0],
    ];

    let mut model = Perceptron::new(0.01, 3000);
    model.fit(shuttle_data.clone(), target_data.clone(), 500);

    println!("\n--- Final Inference ---");
    let predictions = model.predict(shuttle_data);
    for (i, prob) in predictions.iter().enumerate() {
        let truth = if target_data[i][0] == 1.0 { "Safe" } else { "Abort" };
        let guess = if *prob > 0.5 { "Safe" } else { "Abort" };
        println!("Scenario {} (Truth: {}) -> System: {} ({:.2}%)", i + 1, truth, guess, prob * 100.0);
    }
}
