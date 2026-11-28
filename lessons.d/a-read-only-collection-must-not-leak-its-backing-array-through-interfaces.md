# A read-only collection must not leak its backing array through interfaces

### The trap

An API can expose `IReadOnlyList<T>` and still leak writable storage through a
concrete wrapper's additional interfaces or properties. In particular, a
read-only facade may forward a synchronization object that is the backing
array itself, letting callers cast and mutate values behind an immutable
document.

### The fix

Keep the array private and expose only a narrow `IReadOnlyList<T>` facade that
implements indexing, count, and enumeration. Test both normal collection use
and the absence of writable collection or backing-storage escape paths.
