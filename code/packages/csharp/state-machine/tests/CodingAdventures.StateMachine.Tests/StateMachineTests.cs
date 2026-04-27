using CodingAdventures.StateMachine;

namespace CodingAdventures.StateMachine.Tests;

public class StateMachineTests
{
    [Fact]
    public void Dfa_ProcessesTurnstile()
    {
        var dfa = new DFA(
            ["locked", "unlocked"],
            ["coin", "push"],
            new Dictionary<(string State, string Event), string>
            {
                [("locked", "coin")] = "unlocked",
                [("locked", "push")] = "locked",
                [("unlocked", "coin")] = "unlocked",
                [("unlocked", "push")] = "locked",
            },
            "locked",
            ["unlocked"]);

        Assert.Equal("unlocked", dfa.Process("coin"));
        Assert.True(dfa.IsAccepting());
    }
}
