use std::{fmt::Debug, thread::panicking};

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
/// [`propagate`]: ErrorSentinel::expect
pub struct ErrorSentinel<E> {
    /// The list of errors produced. Wrapped in an [`Option`] to permit moving the errors out of 
    /// `self`.
    errors: Option<Vec<E>>,

    /// Whether the errors have been handled. All error-handling methods consume `self`, but this
    /// is still required to indicate to the [`Drop`] implementation that the sentinel was dropped
    /// by being handled properly.
    handled: bool,
}

impl ErrorSentinel<!> {
    /// Constructs an `ErrorSentinel` with no errors.
    pub fn new_ok() -> Self {
        Self {
            errors: Some(vec![]),
            handled: false,
        }
    }    
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

impl<E> Drop for ErrorSentinel<E> {
    fn drop(&mut self) {
        // Let's not add on our own panic if the thread's already panicking. Things are bad enough!
        if !panicking() && !self.handled {
            panic!("sentinel dropped without handling errors");
        }
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
