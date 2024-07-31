use crate::ErrorSentinel;

/// Contains a value, plus possibly one or more errors produced by the procedures which obtained
/// that value.
pub struct Fallible<T, E> {
    value: T,
    errors: Vec<E>,
}

impl<T, E> Fallible<T, E> {
    /// Constructs a new `Fallible` with a value and no errors.
    /// 
    /// ```
    /// # use multierror::Fallible;
    /// let mut f = Fallible::new(42);
    /// assert_eq!(f.len_errors(), 0);
    /// # f.push_error(0); // resolve type
    /// ```
    pub fn new(value: T) -> Self {
        Fallible { value, errors: vec![] }
    }

    /// Adds a new error to this `Fallible`.
    /// 
    /// ```
    /// # use multierror::Fallible;
    /// let mut f = Fallible::new(42);
    /// f.push_error("oh no!");
    /// 
    /// assert!(f.has_errors());
    /// ```
    pub fn push_error(&mut self, error: E) {
        self.errors.push(error);
    }

    /// Returns `true` if this `Fallible` has any errors.
    /// 
    /// Opposite of [`is_success`](#method.is_success).
    pub fn has_errors(&self) -> bool {
        self.len_errors() > 0
    }

    /// Returns `true` if this `Fallible` has no errors.
    /// 
    /// Opposite of [`has_errors`](#method.has_errors).
    pub fn is_success(&self) -> bool {
        self.len_errors() == 0
    }

    /// The number of errors within this `Fallible`.
    /// 
    /// ```
    /// # use multierror::Fallible;
    /// let mut f = Fallible::new(42);
    /// f.push_error("this went wrong");
    /// f.push_error("that went wrong");
    /// 
    /// assert_eq!(f.len_errors(), 2);
    /// ```
    pub fn len_errors(&self) -> usize {
        self.errors.len()
    }

    /// Consumes and deconstructs this `Fallible` into its value and an [`ErrorSentinel`].
    /// 
    /// The `ErrorSentinel` verifies that any errors are handled before it is dropped, most likely
    /// by calling [`handle`]. Failure to do this will cause a panic, even if there were no errors.
    /// See the [`ErrorSentinel`] docs for more details.
    /// 
    /// [`handle`]: ErrorSentinel::handle
    /// 
    /// ```
    /// # use multierror::Fallible;
    /// let mut f = Fallible::new(42);
    /// f.push_error("this went wrong");
    ///
    /// let (value, errors) = f.finalize();
    /// assert_eq!(value, 42);
    /// 
    /// errors.handle(|errs| {
    ///     for err in &errs {
    ///         println!("error: {err}");
    ///     }
    ///     assert_eq!(errs.len(), 1);
    /// });
    /// ```
    #[must_use]
    pub fn finalize(self) -> (T, ErrorSentinel<E>) {
        (self.value, ErrorSentinel::new(self.errors))
    }
}
