# Learning: Gradient Descent & Optimizers

Once we have a **Loss Function** (which tells the network how terrible its answer was) and a **Derivative** (which tells the network the mathematical slope of its mistake), we need a mechanism to actually *apply* those fixes.

This mechanism is called an **Optimizer**.

Optimization algorithms define the exact rules for how weights and biases change over time. By far the most famous family of optimizers belongs to **Gradient Descent**.

## 1. What is Gradient Descent?

Imagine you are blindfolded at the top of a mountain (a high Error/Loss value), and you want to get to the very bottom of the valley (0 Error/Loss). 

Because you are blindfolded, you can't see the valley. However, if you feel the slope of the ground directly under your feet (the *Gradient*), you can figure out which direction points downwards. By taking a step in that direction, you will naturally descend the mountain. If you repeat this thousands of times, you will eventually reach the bottom.

## 2. Stochastic Gradient Descent (SGD)

SGD is the vanilla, foundational "Hello World" version of Gradient Descent. 

It takes three arguments:
1. The `weights` (Where you currently are on the mountain).
2. The `gradients` (The slope of the ground under your feet).
3. The `learning_rate` (How massive of a step you take down the hill).

**The Equation:**
`new_weight = old_weight - (learning_rate * gradient)`

*Why do we subtract?*
If the slope is positive (going uphill to the right), we want to go left. We subtract a positive number, moving us negatively.
If the slope is negative (going uphill to the left), we want to go right. We subtract a negative number, effectively adding and moving us positively!

## 3. Why a Dedicated Package?

By placing this inside its own pure `gradient-descent` package rather than locking it inside a neural network, we keep the math modular. 

In the future, we can add much more complex forms of Gradient Descent to this same package. For instance:
- **Momentum**: If you are walking down a steep hill, you pick up speed. Momentum remembers the previous slopes, helping you blow past tiny potholes (local minimums).
- **Adam** (Adaptive Moment Estimation): Automatically adjusts the `learning_rate` for every individual weight based on how much it has been changing recently. 

But for now, the pure, unadulterated `SGD` is the only tool we need to start training algorithms from scratch!
