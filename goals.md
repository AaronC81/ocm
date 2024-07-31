- It should not be possible to access the **value** without handling the **errors** somehow
    - Is there a way to enforce this statically?

If Rust had non-droppable types, I could have a `destructure` method to return both the value and
some undroppable sentinel which could only be consumed through an explicit `handle` call:

```rust
impl MultiError<T, E> {
    pub fn finalize(self) -> (T, Sentinel<E>) {
        // ...
    }
}

struct Sentinel(Vec<E>)
impl !Drop for Sentinel {}; // Not real!

impl Sentinel {
    // Only way to drop is by calling this, which implies some amount of error handling
    // But the compiler raises an error if it was never dropped because it implements `!Drop`
    pub fn handle_errors<T>(self, handler: impl FnOnce(Self) -> T) {
        // ...
    }
}
```

----

// TODO: better name
// (But designed for 'error box' cases...)
pub struct Errors<E> {
    errors: Vec<E>,
}
