using Xunit;
using Mosaic.Flux;

namespace Mosaic.Flux.Tests;

public record DtState(int V = 0);

public sealed record DtBump : IMosaicAction<DtState>
{
    public DtState Apply(DtState state) => state with { V = state.V + 1 };
}

public class DevToolsTests
{
    [Fact]
    public void Callable()
    {
        var m = DevTools.Create<DtState>();
        m(new DtBump(), new DtState(0), new DtState(1));
    }

    [Fact]
    public void CustomStoreName()
    {
        var m = DevTools.Create<DtState>("my-grid");
        m(new DtBump(), new DtState(0), new DtState(1));
    }

    [Fact]
    public void IntegratesWithStore()
    {
        var probeRuns = 0;
        var store = new MosaicStore<DtState>(
            new DtState(),
            new Middleware<DtState>[]
            {
                DevTools.Create<DtState>(),
                (_, _, _) => probeRuns++,
            });
        store.Dispatch(new DtBump());
        store.Dispatch(new DtBump());
        Assert.Equal(2, probeRuns);
        Assert.Equal(2, store.State.V);
    }
}
