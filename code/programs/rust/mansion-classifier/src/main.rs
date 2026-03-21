use perceptron::Perceptron;

fn main() {
    println!("\n--- Booting Rust Mansion Classifier (OOP V2) ---");

    let house_data = vec![
        vec![4.5, 6.0], vec![3.8, 5.0], vec![1.5, 2.0],
        vec![0.9, 1.0], vec![5.5, 7.0], vec![2.0, 3.0],
    ];
    let target_data = vec![
        vec![1.0], vec![1.0], vec![0.0], vec![0.0], vec![1.0], vec![0.0],
    ];

    let mut model = Perceptron::new(0.1, 2000);
    model.fit(house_data.clone(), target_data.clone(), 400);

    println!("\n--- Final Inference ---");
    let predictions = model.predict(house_data);
    for (i, prob) in predictions.iter().enumerate() {
        let truth = if target_data[i][0] == 1.0 { "Mansion" } else { "Normal" };
        let guess = if *prob > 0.5 { "Mansion" } else { "Normal" };
        println!("House {} (Truth: {}) -> System: {} ({:.2}%)", i + 1, truth, guess, prob * 100.0);
    }
}
