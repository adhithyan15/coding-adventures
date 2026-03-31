# CodingAdventures::EventLoop (Perl)

Event emitter and tick-based scheduler — the heartbeat of any interactive
application.

## Usage

```perl
use CodingAdventures::EventLoop;

my $loop = CodingAdventures::EventLoop->new();

# Push-based: named events
$loop->on("damage", sub { my $d = shift; $hp -= $d->{amount} });
$loop->once("startup", sub { print "Started!\n" });
$loop->emit("startup", {});
$loop->emit("startup", {});  # once handler does NOT fire again
$loop->emit("damage",  { amount => 10 });

# Pull-based: tick scheduler
$loop->on_tick(sub { my $dt = shift; $pos += $vel * $dt });
$loop->run(60, 1/60);    # 60 frames at 60 Hz

print $loop->{elapsed_time};  # ~1.0
print $loop->{tick_count};    # 60
```

## API

### `new()` → $loop
### `on($event, $cb)` — register persistent handler
### `once($event, $cb)` — register one-shot handler
### `off($event [, $cb])` — remove all or specific handler
### `emit($event, $data)` — fire all handlers for event
### `on_tick($cb)` — register tick handler `sub { my $dt = shift; ... }`
### `tick([$dt])` — one time step (default dt=1.0)
### `run([$n, $dt])` — n ticks (default n=1, dt=1.0)
### `step([$dt])` — alias for run(1, dt)

Fields: `elapsed_time`, `tick_count`

## License

MIT
