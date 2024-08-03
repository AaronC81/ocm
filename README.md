# ocm

The `ocm` crate provides `Outcome<T, E>`, an ergonomic type to model operations which can
produce several errors while always returning some value.

This is useful for operations like parsing, where you'd like to gather as many errors as possible
before failing.

```rust
use ocm::{Outcome, ErrorCollector};

// Some operation which always returns a value, but may produce errors while doing so
pub fn sum_ints<'a>(input: &[&'a str]) -> Outcome<u32, String> {
    Outcome::build(|errors| {
        let mut sum = 0;

        for item in input {
            match item.parse::<u32>() {
                Ok(num) => sum += num,
                Err(_) => errors.push_error(format!("not a number: {item}")),
            }
        }

        sum
    })
}

let outcome = sum_ints(&["123", "456", "abc", "789", "def"]);

// `Outcome::finalize` breaks the outcome into:
//   - The value
//   - An `ErrorSentinel`, which dynamically ensures that errors are handled
let (value, errors) = outcome.finalize();
println!("Sum value: {value}");
if errors.any() {
    println!("Errors:");
    for err in errors.into_errors_iter() {
        println!("  - {err}");
    }
}
```

Just bundling a value and errors together risks that the errors are accidentally ignored. We'd like
to be sure that the errors are handled appropriately.

Unfortunately, Rust lacks the linear type system which would be required to do this statically, so
instead it is done at runtime, using panics to signal that errors were dropped without being
handled:

```rust,should_panic
use ocm::Outcome;

let outcome = Outcome::new_with_errors(42, vec!["error 1", "error 2"]);
let (value, errors) = outcome.finalize();

println!("Value: {value}");
// Whoops! `errors` was never handled - this will panic.
```
