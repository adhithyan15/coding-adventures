from perceptron import Perceptron

def main():
    print("\n--- Booting Python Space Launch Predictor (OOP V2) ---")
    shuttle_data = [
        [12.0, 15.0], [35.0, 85.0], [5.0, 5.0],
        [40.0, 95.0], [15.0, 30.0], [28.0, 60.0]
    ]
    target_data = [
        [1.0], [0.0], [1.0], [0.0], [1.0], [0.0]
    ]

    model = Perceptron(learning_rate=0.01, epochs=3000)
    model.fit(shuttle_data, target_data, log_steps=500)

    print("\n--- Final Inference ---")
    predictions = model.predict(shuttle_data)
    for i, prob in enumerate(predictions):
        truth = "Safe" if target_data[i][0] == 1.0 else "Abort"
        guess = "Safe" if prob > 0.5 else "Abort"
        print(f"Scenario {i+1} (Truth: {truth}) -> System: {guess} ({prob*100:.2f}%)")

if __name__ == "__main__":
    main()
