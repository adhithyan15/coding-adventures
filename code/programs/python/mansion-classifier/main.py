from perceptron import Perceptron

def main():
    print("\n--- Booting Python Mansion Classifier (OOP V2) ---")
    house_data = [
        [4.5, 6.0], [3.8, 5.0], [1.5, 2.0],
        [0.9, 1.0], [5.5, 7.0], [2.0, 3.0]
    ]
    target_data = [
        [1.0], [1.0], [0.0], [0.0], [1.0], [0.0]
    ]

    model = Perceptron(learning_rate=0.1, epochs=2000)
    model.fit(house_data, target_data, log_steps=400)

    print("\n--- Final Probability Inferences ---")
    predictions = model.predict(house_data)
    for i, prob in enumerate(predictions):
        truth = "Mansion" if target_data[i][0] == 1.0 else "Normal"
        guess = "Mansion" if prob > 0.5 else "Normal"
        print(f"House {i+1} (Truth: {truth}) -> System: {guess} ({prob*100:.2f}%)")

if __name__ == "__main__":
    main()
