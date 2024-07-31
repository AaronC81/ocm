use std::{fmt::Debug, thread::panicking};

use crate::{ErrorCollector, Fallible};

/// Represents errors which must be handled before this sentinel is dropped.
/// 
/// `ErrorSentinel` has a custom implementation of the [`Drop`] trait which checks that the errors
/// were handled in some way, and panics if not.
/// 
/// ```should_panic
/// # use multierror::ErrorSentinel;
/// {
///     let errors = ErrorSentinel::new(vec!["error 1", "error 2"]);
///     // Panic occurs here!
/// }
/// ```
/// 
/// Using a method which marks the errors as handled will suppress the panic:
/// 
/// ```
/// # use multierror::ErrorSentinel;
/// {
///     let errors = ErrorSentinel::new(vec!["error 1", "error 2"]);
///     errors.handle(|errs| {
///         for err in errs {
///             println!("error: {err}");
///         }    
///     });
///     // No panic, because errors were handled
/// }
/// ```
/// 
/// Methods which consider the errors "handled" are documented as such, and all also consume the
/// `ErrorSentinel`. Most often, you will want to do one of the following:
/// 
/// - Handle errors with some custom logic: [`handle`]
/// - Assert that there are no errors: [`unwrap`] or [`expect`]
/// - Delegate the errors to another object: [`propagate`]
/// 
/// [`handle`]: ErrorSentinel::handle
/// [`unwrap`]: ErrorSentinel::unwrap
/// [`expect`]: ErrorSentinel::expect
/// [`propagate`]: ErrorSentinel::propagate
pub struct ErrorSentinel<E> {
    /// The list of errors produced. Wrapped in an [`Option`] to permit moving the errors out of 
    /// `self`.
    errors: Option<Vec<E>>,

    /// Whether the errors have been handled. All error-handling methods consume `self`, but this
    /// is still required to indicate to the [`Drop`] implementation that the sentinel was dropped
    /// by being handled properly.
    handled: bool,
}

impl<E> ErrorSentinel<E> {
    /// Constructs a new unhandled `ErrorSentinel` with errors.
    /// 
    /// Usually this is not needed to be called manually, and an `ErrorSentinel` will be created by
    /// [`Fallible::finalize`] instead.
    /// 
    /// [`Fallible::finalize`]: crate::Fallible::finalize
    pub fn new(errors: Vec<E>) -> Self {
        Self {
            errors: Some(errors),
            handled: false,
        }
    }

    /// Constructs a new unhandled `ErrorSentinel` without any errors.
    pub fn new_empty() -> Self {
        Self {
            errors: Some(vec![]),
            handled: false,
        }
    }
    
    /// Handles the errors by executing a closure, returning the value which it evaluates to.
    /// 
    /// ```
    /// # use multierror::ErrorSentinel;
    /// let errors = ErrorSentinel::new(vec!["error 1", "error 2", "error 3"]);
    /// 
    /// let mut error_count = 0;
    /// errors.handle(|errs| {
    ///     for err in errs {
    ///         println!("error: {err}");
    ///         error_count += 1;
    ///     }    
    /// });
    /// 
    /// assert_eq!(error_count, 3);
    /// ```
    /// 
    /// The closure should implement appropriate error-handling logic for your application, such as
    /// printing messages.
    /// 
    /// `ErrorSentinel`'s checking can only go so far, and does not understand if you have actually
    /// done something appropriate with the errors. This would be considered a valid handling of
    /// errors:
    /// 
    /// ```
    /// # use multierror::ErrorSentinel;
    /// # let errors = ErrorSentinel::new(vec!["error 1"]);
    /// errors.handle(|_| ());
    /// ```
    pub fn handle<R>(mut self, handler: impl FnOnce(Vec<E>) -> R) -> R {
        self.handled = true;

        // Unwrap will not panic - this consumes `self` so it can't be called again
        handler(self.errors.take().unwrap())
    }

    /// Handles the errors by moving them into an [`ErrorCollector`], effectively postponing them to
    /// be handled later instead.
    /// 
    /// ```
    /// # use multierror::ErrorSentinel;
    /// let source = ErrorSentinel::new(vec!["error 1", "error 2"]);
    /// let mut dest = ErrorSentinel::new(vec!["error 3", "error 4", "error 5"]);
    /// 
    /// source.propagate(&mut dest);
    /// assert_eq!(dest.peek().len(), 5);
    /// # dest.ignore();
    /// ```
    pub fn propagate(self, other: &mut impl ErrorCollector<E>) {
        for error in self.into_errors_iter() {
            other.push_error(error);
        }
    }

    /// Handles the errors by ignoring them, dropping the list of errors.
    /// 
    /// ```
    /// # use multierror::ErrorSentinel;
    /// let errors = ErrorSentinel::new(vec!["error 1", "error 2", "error 3"]);
    /// errors.ignore();
    /// ```
    /// 
    /// This exists as an "escape hatch", but its use is strongly not recommended. There is probably
    /// a more suitable method for what you are trying to do. Consider using [`unwrap`] or
    /// [`expect`] if there should not be any errors in the `ErrorSentinel`, which will panic if
    /// this assumption is violated unlike `ignore`.
    /// 
    /// [`unwrap`]: ErrorSentinel::unwrap
    /// [`expect`]: ErrorSentinel::expect
    pub fn ignore(mut self) {
        self.handled = true;
    }

    /// Handles the errors by moving them into a new [`Fallible`] with a given value.
    /// 
    /// ```
    /// # use multierror::ErrorSentinel;
    /// let errors = ErrorSentinel::new(vec!["error 1", "error 2", "error 3"]);
    /// let fallible = errors.into_fallible(42);
    /// 
    /// assert_eq!(fallible.len_errors(), 3);
    /// ```
    /// 
    /// This can be useful for performing some logic which accumulates errors over time, and then
    /// finally creating a `Fallible` to return with a calculated value. Using an `ErrorSentinel`
    /// to accumulate the errors ensures that you cannot forget to return them.
    /// 
    /// ```
    /// # use multierror::{ErrorSentinel, Fallible, ErrorCollector};
    /// /// Sum the integer values in a sequence of strings.
    /// /// Any non-integer values are returned as errors.
    /// pub fn sum_ints<'a>(input: &[&'a str]) -> Fallible<u32, &'a str> {
    ///     let mut errors = ErrorSentinel::new_empty();
    ///     let mut sum = 0;
    /// 
    ///     for item in input {
    ///         match item.parse::<u32>() {
    ///             Ok(num) => sum += num,
    ///             Err(_) => errors.push_error(*item),
    ///         }
    ///     }
    /// 
    ///     errors.into_fallible(sum)
    /// }
    /// 
    /// let result = sum_ints(&["12", "a", "5", "b", "c", "2"]);
    /// let (value, errors) = result.finalize();
    /// 
    /// assert_eq!(value, 12 + 5 + 2);
    /// assert_eq!(errors.peek(), &["a", "b", "c"]);
    /// # errors.ignore();
    /// ```
    pub fn into_fallible<T>(self, value: T) -> Fallible<T, E> {
        let mut f = Fallible::new(value);
        self.propagate(&mut f);
        f
    }

    /// Consumes this `ErrorSentinel` to create an [`ErrorSentinelIter`], enabling errors to be
    /// handled as an iterator.
    /// 
    /// The iterator must be entirely consumed to consider the errors handled, else the iterator
    /// will panic on drop. Refer to the type-level documentation for [`ErrorSentinelIter`] for more
    /// details.
    /// 
    /// This is deliberately not an [`IntoIterator`] implementation, so that the decision to handle
    /// errors one-by-one is explicit, by calling this method.
    pub fn into_errors_iter(mut self) -> ErrorSentinelIter<E> {
        // Mark ourselves as handled - the responsibility is moved onto the iterator
        self.handled = true;

        let original_len = self.errors.as_ref().unwrap().len();
        ErrorSentinelIter {
            original_len,
            iter: self.errors.take().unwrap().into_iter(), 
        }
    }

    /// Inspect the list of errors, without considering them handled.
    /// 
    /// ```
    /// # use multierror::ErrorSentinel;
    /// let errors = ErrorSentinel::new(vec!["error 1", "error 2"]);
    /// assert_eq!(errors.peek(), &["error 1", "error 2"]);
    /// errors.ignore(); // Prevent panic
    /// ```
    pub fn peek(&self) -> &[E] {
        self.errors.as_ref().unwrap()
    }

    /// Handles the errors by panicking if there are any errors.
    /// 
    /// The panic message includes the [`Debug`] representation of the errors. If you would like
    /// to provide a custom message instead, use [`expect`].
    /// 
    /// [`expect`]: ErrorSentinel::expect
    /// 
    /// ```should_panic
    /// # use multierror::ErrorSentinel;
    /// let errors = ErrorSentinel::new(vec!["error 1", "error 2"]);
    /// errors.unwrap(); // Panics
    /// ```
    /// 
    /// ```
    /// # use multierror::ErrorSentinel;
    /// let errors = ErrorSentinel::new_ok();
    /// errors.unwrap(); // OK
    /// ```
    #[track_caller]
    pub fn unwrap(mut self)
    where E : Debug
    {
        self.handled = true;
        if !self.peek().is_empty() {
            panic!("called `unwrap` on a sentinel with errors: {:?}", self.errors.take().unwrap())
        }
    }

    /// Handles the errors by panicking with a message if there are any errors.
    /// 
    /// ```should_panic
    /// # use multierror::ErrorSentinel;
    /// let errors = ErrorSentinel::new(vec!["error 1", "error 2"]);
    /// errors.expect("something went wrong"); // Panics
    /// ```
    /// 
    /// ```
    /// # use multierror::ErrorSentinel;
    /// let errors = ErrorSentinel::new_ok();
    /// errors.expect("something went wrong"); // OK
    /// ```
    #[track_caller]
    pub fn expect(mut self, msg: &str)
    where E : Debug
    {
        self.handled = true;
        if !self.peek().is_empty() {
            panic!("{}", msg)
        }
    }
}

impl ErrorSentinel<!> {
    /// Constructs an `ErrorSentinel` which does not and will never contain errors, by using the
    /// never type [`!`] as the error type.
    pub fn new_ok() -> Self {
        Self {
            errors: Some(vec![]),
            handled: false,
        }
    }

    /// An alias for [`ignore`] which is only available when the error type is the never type [`!`].
    /// 
    /// In this case, an error can never occur, so it is safe to ignore errors. Using
    /// `safely_ignore` instead of `ignore` will signal to readers that this is a safe assumption,
    /// and will cause a compile error if the error type ever changes from `!`.
    /// 
    /// [`ignore`]: ErrorSentinel::ignore
    /// 
    /// ```
    /// # use multierror::ErrorSentinel;
    /// let errors = ErrorSentinel::new_ok();
    /// errors.safely_ignore(); // Prevents panic
    /// ```
    pub fn safely_ignore(self) {
        self.ignore()
    }
}

impl<E> Drop for ErrorSentinel<E> {
    fn drop(&mut self) {
        // Let's not add on our own panic if the thread's already panicking. Things are bad enough!
        if !panicking() && !self.handled {
            panic!("sentinel dropped without handling errors");
        }
    }
}

impl<E> ErrorCollector<E> for ErrorSentinel<E> {
    fn push_error(&mut self, error: E) {
        self.errors.as_mut().unwrap().push(error);
    }
}

/// An adapter for [`ErrorSentinel`] which implements [`Iterator`], so that errors can be handled
/// one-by-one. Created with [`ErrorSentinel::into_errors_iter`].
/// 
/// Like `ErrorSentinel`, this overrides [`Drop`] in order to panic if it contains unhandled errors
/// when dropped.
/// 
/// For the errors within the `ErrorSentinelIter` to be considered handled, then the iterator must
/// be **exhausted**. Every single error must have been iterated through. The most convenient way
/// to do this is with a `for` loop:
/// 
/// ```
/// # use multierror::ErrorSentinel;
/// let errors = ErrorSentinel::new(vec!["error 1", "error 2"]);
/// for error in errors.into_errors_iter() {
///     println!("error: {error}");
/// }
/// ```
/// 
/// If the loop breaks early, then not all errors may have been handled, causing a panic:
/// 
/// ```should_panic
/// # use multierror::ErrorSentinel;
/// {
///     let errors = ErrorSentinel::new(vec!["error 1", "error 2"]);
///     for error in errors.into_errors_iter() {
///         println!("error: {error}");
///         break; // Break after first error
///     }
///     // Panic occurs here!
/// }
/// ```
pub struct ErrorSentinelIter<E> {
    original_len: usize,
    iter: std::vec::IntoIter<E>,
}

impl<E> ErrorSentinelIter<E> {
    /// Whether the iterator is exhausted, and all errors have been handled.
    /// 
    /// ```
    /// # use multierror::ErrorSentinel;
    /// let mut error_iter = ErrorSentinel::new(vec!["error 1", "error 2"]).into_errors_iter();
    /// 
    /// assert!(!error_iter.is_handled());
    /// error_iter.next().unwrap();
    /// error_iter.next().unwrap();
    /// assert!(error_iter.is_handled());
    /// ```
    pub fn is_handled(&self) -> bool {
        self.len() == 0
    }
}

impl<E> Iterator for ErrorSentinelIter<E> {
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<E> ExactSizeIterator for ErrorSentinelIter<E> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<E> Drop for ErrorSentinelIter<E> {
    fn drop(&mut self) {
        // Let's not add on our own panic if the thread's already panicking. Things are bad enough!
        if !panicking() && !self.is_handled() {
            panic!(
                "sentinel iterator dropped without handling all errors: {} out of {} error(s) unhandled",
                self.len(),
                self.original_len,
            );
        }
    }
}
