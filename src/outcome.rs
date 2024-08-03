use std::fmt::Debug;

use crate::{ErrorCollector, ErrorSentinel};

/// Contains a value, and any errors produced while obtaining that value.
/// 
/// `Outcome<T>` can be used like a `Result<T, Vec<E>>`, except it has _both_ the `Ok` and `Err`
/// variants at the same time. This is useful for modelling procedures which should try to
/// accumulate as many errors as possible before failing, even if fatal, such as parsing.
/// 
/// # Creation
/// 
/// Instances of `Outcome` can be created in a number of different ways, depending on what you
/// are trying to achieve, and how you are producing errors.
/// 
/// Use [`new_with_errors`] to construct "raw" from an existing value and list of errors:
/// 
/// [`new_with_errors`]: Outcome::new_with_errors
/// 
/// ```
/// # use multierror::Outcome;
/// Outcome::new_with_errors(42, vec!["something that went wrong"]);
/// ```
/// 
/// Use [`build`] to compute a value while accumulating errors:
/// 
/// [`build`]: Outcome::build
/// 
/// ```
/// # use multierror::{Outcome, ErrorCollector};
/// Outcome::build(|errs| {
///     let mut sum = 0;
///     for i in 0..10 {
///         if i % 3 == 0 {
///             errs.push_error("don't like multiples of 3");
///         }
///         sum += i;
///     }
///     sum
/// });
/// ```
/// 
/// # Finalization
/// 
/// If you have an `Outcome` and need to get the value and errors out, call [`finalize`]. This gives
/// both the inner value and an [`ErrorSentinel`], a special type which **ensures** that the errors
/// are handled in some way before it is dropped. If the errors are unhandled, it will cause a panic
/// to alert you to your logic error. See the documentation for [`ErrorSentinel`] for details.
/// 
/// [`finalize`]: Outcome::finalize
/// 
/// ```
/// # use multierror::Outcome;
/// fn something() -> Outcome<u32, String> {
///     // ...
///     # Outcome::new(0)
/// }
/// 
/// let o = something();
/// let (value, errors) = o.finalize();
/// 
/// println!("value is {value}");
/// 
/// // This iteration counts as handling the error, as per the `ErrorSentinel::into_errors_iter`
/// // docs. If we didn't handle the errors, using this method or some other one, our program
/// // would panic when `errors` was dropped.
/// for err in errors.into_errors_iter() {
///     println!("error: {err}");
/// }
/// ```
/// 
/// # Combination
/// 
/// `Outcome` provides some functional combinators to transform and combine instances together.
/// These are useful for modularizing complex pieces of functionality which could all produce errors
/// individually, but which you will need to collect together later.
/// 
/// - Transform values and/or errors: [`map`], [`map_errors`]
/// - Unwrap a value by moving its errors elsewhere: [`propagate`]
/// - Fold two values and combine their errors: [`integrate`]
/// - Bundle values into a collection and combine their errors: [`zip`], [`from_iter`]
/// - Extract the value by asserting there are no errors: [`unwrap`], [`expect`]
/// 
/// [`map`]: Outcome::map
/// [`map_errors`]: Outcome::map_errors
/// [`propagate`]: Outcome::propagate
/// [`integrate`]: Outcome::integrate
/// [`zip`]: Outcome::zip
/// [`from_iter`]: Outcome::from_iter
/// [`unwrap`]: Outcome::unwrap
/// [`expect`]: Outcome::expect
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Outcome<T, E> {
    value: T,
    errors: Vec<E>,
}

impl<T, E> Outcome<T, E> {
    /// Constructs a new `Outcome` with a value and no errors.
    /// 
    /// ```
    /// # use multierror::Outcome;
    /// let mut o = Outcome::new(42);
    /// assert_eq!(o.len_errors(), 0);
    /// # o.push_error(0); // resolve type
    /// ```
    #[must_use]
    pub fn new(value: T) -> Self {
        Outcome { value, errors: vec![] }
    }

    /// Constructs a new `Outcome` with some errors.
    /// 
    /// ```
    /// # use multierror::Outcome;
    /// let mut o = Outcome::new_with_errors(42, vec!["an error"]);
    /// assert_eq!(o.len_errors(), 1);
    /// ```
    #[must_use]
    pub fn new_with_errors(value: T, errors: Vec<E>) -> Self {
        Outcome { value, errors }
    }
    
    /// A convenience function to construct a new `Outcome` by accumulating errors over time, and
    /// finally returning some value.
    /// 
    /// ```
    /// # use multierror::{Outcome, ErrorCollector};
    /// fn sub_task() -> Outcome<u32, String> {
    ///     Outcome::new_with_errors(
    ///         42,
    ///         vec!["struggled to compute meaning of life".to_owned()]
    ///     )
    /// }
    /// 
    /// let o = Outcome::build(|errs| {
    ///     // Produce some errors of our own...
    ///     errs.push_error("what are we doing?".to_owned());
    /// 
    ///     // ...or propagate errors from another `Outcome`
    ///     let value = sub_task().propagate(errs);
    /// 
    ///     value + 1
    /// });
    /// 
    /// let (value, errors) = o.finalize();
    /// assert_eq!(value, 42 + 1);
    /// assert_eq!(errors.len(), 2);
    /// # errors.ignore();
    /// ```
    #[must_use]
    pub fn build<F>(func: F) -> Self
    where
        F: FnOnce(&mut ErrorSentinel<E>) -> T,
    {
        let mut sentinel = ErrorSentinel::empty();
        let value = func(&mut sentinel);
        sentinel.into_outcome(value)
    }

    /// Adds a new error to this `Outcome`.
    /// 
    /// ```
    /// # use multierror::Outcome;
    /// let mut o = Outcome::new(42);
    /// o.push_error("oh no!");
    /// 
    /// assert!(o.has_errors());
    /// ```
    pub fn push_error(&mut self, error: E) {
        self.errors.push(error);
    }

    /// Moves the errors from this `Outcome` into an [`ErrorCollector`], and unwraps it to return
    /// its value.
    /// 
    /// ```
    /// # use multierror::Outcome;
    /// let mut source = Outcome::new(42);
    /// source.push_error("oh no!");
    /// source.push_error("another error!");
    /// let mut dest = Outcome::new(123);
    /// dest.push_error("one last failure!");
    /// 
    /// let source_value = source.propagate(&mut dest);
    /// assert_eq!(dest.len_errors(), 3);
    /// assert_eq!(source_value, 42);
    /// ```
    #[must_use = "propagate returns the inner value; use `integrate` if you wish to merge values in-place"]
    pub fn propagate(self, other: &mut impl ErrorCollector<E>) -> T {
        for error in self.errors.into_iter() {
            other.push_error(error);
        }

        self.value
    }

    /// Moves the errors from this `Outcome` into another `Outcome`, and apply a mapping function
    /// to transform the value within that `Outcome` based on the value within this one.
    /// 
    /// ```
    /// # use multierror::Outcome;
    /// let mut source = Outcome::new(42);
    /// source.push_error("oh no!");
    /// source.push_error("another error!");
    /// let mut dest = Outcome::new(123);
    /// dest.push_error("one last failure!");
    /// 
    /// // Integrate by adding the values
    /// source.integrate(&mut dest, |acc, x| *acc += x);
    /// 
    /// // Check result
    /// let (value, errors) = dest.finalize();
    /// assert_eq!(value, 123 + 42);
    /// assert_eq!(errors.len(), 3);
    /// # errors.ignore();
    /// ```
    pub fn integrate<OT>(self, other: &mut Outcome<OT, E>, func: impl FnOnce(&mut OT, T)) {
        func(&mut other.value, self.value);

        for error in self.errors.into_iter() {
            other.push_error(error);
        }
    }
    
    /// Consumes this `Outcome` and another one, returning a new `Outcome` with their values as a
    /// tuple `(this, other)` and the errors combined.
    /// 
    /// ```
    /// # use multierror::Outcome;
    /// let a = Outcome::new_with_errors(5, vec!["error 1", "error 2"]);
    /// let b = Outcome::new_with_errors(9, vec!["error 3"]);
    /// 
    /// let zipped = a.zip(b);
    /// 
    /// let (value, errors) = zipped.finalize();
    /// assert_eq!(value, (5, 9));
    /// assert_eq!(errors.len(), 3);
    /// # errors.ignore();
    /// ```
    #[must_use]
    pub fn zip<OT>(self, other: Outcome<OT, E>) -> Outcome<(T, OT), E> {
        Outcome::new_with_errors(
            (self.value, other.value),
            self.errors.into_iter().chain(other.errors).collect(),
        )
    }

    /// Applies a function to the value within this `Outcome`.
    /// 
    /// ```
    /// # use multierror::Outcome;
    /// let o = Outcome::new_with_errors("Hello".to_owned(), vec!["oh no!"]);
    /// let o_rev = o.map(|s| s.len());
    /// 
    /// let (value, errors) = o_rev.finalize();
    /// assert_eq!(value, 5);
    /// assert_eq!(errors.len(), 1);
    /// # errors.ignore();
    /// ```
    #[must_use]
    pub fn map<R>(self, func: impl FnOnce(T) -> R) -> Outcome<R, E> {
        Outcome::new_with_errors(
            func(self.value),
            self.errors,
        )
    }

    /// Applies a function to the errors within this `Outcome`.
    /// 
    /// ```
    /// # use multierror::Outcome;
    /// let o = Outcome::new_with_errors(42, vec!["oh no!", "something went wrong"]);
    /// let o_mapped = o.map_errors(|e| e.to_uppercase());
    /// 
    /// let (value, errors) = o_mapped.finalize();
    /// assert_eq!(value, 42);
    /// assert_eq!(errors.peek(), &["OH NO!".to_owned(), "SOMETHING WENT WRONG".to_owned()]);
    /// # errors.ignore();
    /// ```
    #[must_use]
    pub fn map_errors<R>(self, func: impl FnMut(E) -> R) -> Outcome<T, R> {
        Outcome::new_with_errors(
            self.value,
            self.errors.into_iter().map(func).collect(),
        )
    }

    /// Extracts the inner value, panicking if there are any errors.
    /// 
    /// The panic message includes the [`Debug`] representation of the errors. If you would like
    /// to provide a custom message instead, use [`expect`].
    /// 
    /// [`expect`]: Outcome::expect
    /// 
    /// ```should_panic
    /// # use multierror::Outcome;
    /// let o = Outcome::new_with_errors(42, vec!["error 1", "error 2"]);
    /// o.unwrap(); // Panics
    /// ```
    /// 
    /// ```
    /// # use multierror::Outcome;
    /// let o: Outcome<_, String> = Outcome::new(42);
    /// let value = o.unwrap();
    /// assert_eq!(value, 42);
    /// ```
    #[track_caller]
    pub fn unwrap(self) -> T
    where E : Debug
    {
        if self.is_success() {
            self.value
        } else {
            panic!("called `unwrap` on a Outcome with errors: {:?}", self.errors)
        }
    }

    /// Extracts the inner value, panicking with a message if there are any errors.
    /// 
    /// ```should_panic
    /// # use multierror::Outcome;
    /// let o = Outcome::new_with_errors(42, vec!["error 1", "error 2"]);
    /// o.expect("something went wrong"); // Panics
    /// ```
    /// 
    /// ```
    /// # use multierror::Outcome;
    /// let o: Outcome<_, String> = Outcome::new(42);
    /// let value = o.expect("something went wrong");
    /// assert_eq!(value, 42);
    /// ```
    #[track_caller]
    pub fn expect(self, msg: &str) -> T
    where E : Debug
    {
        if self.is_success() {
            self.value
        } else {
            panic!("{msg}")
        }
    }

    /// Converts this `Outcome` into a [`Result`]:
    /// 
    /// - If there are no errors, produces an [`Ok`] with the value.
    /// - Otherwise, produces an [`Err`] with an [`ErrorSentinel`], discarding the value. This means
    ///   you **must** handle the errors before they are dropped, as with [`finalize`].
    /// 
    /// [`finalize`]: Outcome::finalize
    #[must_use = "if there are errors, discarding the `Result` will panic immediately"]
    pub fn into_result(self) -> Result<T, ErrorSentinel<E>> {
        if self.is_success() {
            Ok(self.value)
        } else {
            Err(self.into_errors())
        }
    }

    /// Converts this `Outcome` into an [`ErrorSentinel`], discarding the value.
    /// 
    /// You **must** handle the errors before they are dropped, as with [`finalize`].
    /// 
    /// [`finalize`]: Outcome::finalize
    #[must_use = "discarding the `ErrorSentinel` will panic immediately"]
    pub fn into_errors(self) -> ErrorSentinel<E> {
        ErrorSentinel::new(self.errors)
    }

    /// Returns `true` if this `Outcome` has any errors.
    /// 
    /// Opposite of [`is_success`](#method.is_success).
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.len_errors() > 0
    }

    /// Returns `true` if this `Outcome` has no errors.
    /// 
    /// Opposite of [`has_errors`](#method.has_errors).
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.len_errors() == 0
    }

    /// The number of errors within this `Outcome`.
    /// 
    /// ```
    /// # use multierror::Outcome;
    /// let mut o = Outcome::new(42);
    /// o.push_error("this went wrong");
    /// o.push_error("that went wrong");
    /// 
    /// assert_eq!(o.len_errors(), 2);
    /// ```
    #[must_use]
    pub fn len_errors(&self) -> usize {
        self.errors.len()
    }

    /// Consumes and deconstructs this `Outcome` into its value and an [`ErrorSentinel`].
    /// 
    /// The `ErrorSentinel` verifies that any errors are handled before it is dropped, most likely
    /// by calling [`handle`]. Failure to do this will cause a panic, even if there were no errors.
    /// See the [`ErrorSentinel`] docs for more details.
    /// 
    /// [`handle`]: ErrorSentinel::handle
    /// 
    /// ```
    /// # use multierror::Outcome;
    /// let mut o = Outcome::new(42);
    /// o.push_error("this went wrong");
    ///
    /// let (value, errors) = o.finalize();
    /// assert_eq!(value, 42);
    /// 
    /// errors.handle(|errs| {
    ///     for err in &errs {
    ///         println!("error: {err}");
    ///     }
    ///     assert_eq!(errs.len(), 1);
    /// });
    /// ```
    #[must_use = "discarding the `ErrorSentinel` will panic immediately"]
    pub fn finalize(self) -> (T, ErrorSentinel<E>) {
        (self.value, ErrorSentinel::new(self.errors))
    }
}

impl<T, E> ErrorCollector<E> for Outcome<T, E> {
    type WrappedInner = T;

    fn push_error(&mut self, error: E) {
        Outcome::push_error(self, error);
    }

    fn propagate(self, other: &mut impl ErrorCollector<E>) -> Self::WrappedInner {
        Outcome::propagate(self, other)
    }
}

impl<T, E, C: FromIterator<T>> FromIterator<Outcome<T, E>> for Outcome<C, E> {
    /// Enables an [`Iterator`] of `Outcome` items to be converted into a single `Outcome` whose
    /// item is a collection containing each of the items' values.
    /// 
    /// The errors are aggregated in order.
    /// 
    /// ```
    /// # use multierror::Outcome;
    /// let items = vec![
    ///     Outcome::new_with_errors(1, vec!["error 1", "error 2"]),
    ///     Outcome::new_with_errors(2, vec!["error 3"]),
    ///     Outcome::new_with_errors(3, vec!["error 4", "error 5"]),
    /// ];
    /// 
    /// let combined: Outcome<Vec<u32>, _> = items.into_iter().collect();
    /// 
    /// let (value, errors) = combined.finalize();
    /// assert_eq!(value, vec![1, 2, 3]);
    /// assert_eq!(errors.len(), 5);
    /// # errors.ignore();
    /// ```
    fn from_iter<I: IntoIterator<Item = Outcome<T, E>>>(iter: I) -> Self {
        let mut items = vec![];
        let mut errors = vec![];

        for item in iter {
            items.push(item.value);
            errors.extend(item.errors);
        }

        Outcome::new_with_errors(items.into_iter().collect(), errors)
    }
}
