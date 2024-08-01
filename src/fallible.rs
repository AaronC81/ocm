use crate::{ErrorCollector, ErrorSentinel};

/// Contains a value, plus possibly one or more errors produced by the procedures which obtained
/// that value.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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

    /// Moves the errors from this `Fallible` into an [`ErrorCollector`], and unwraps it to return
    /// its value.
    /// 
    /// ```
    /// # use multierror::Fallible;
    /// let mut source = Fallible::new(42);
    /// source.push_error("oh no!");
    /// source.push_error("another error!");
    /// let mut dest = Fallible::new(123);
    /// dest.push_error("one last failure!");
    /// 
    /// let source_value = source.propagate(&mut dest);
    /// assert_eq!(dest.len_errors(), 3);
    /// assert_eq!(source_value, 42);
    /// ```
    pub fn propagate(self, other: &mut impl ErrorCollector<E>) -> T {
        for error in self.errors.into_iter() {
            other.push_error(error);
        }

        self.value
    }

    /// Moves the errors from this `Fallible` into another `Fallible`, and apply a mapping function
    /// to transform the value within that `Fallible` based on the value within this one.
    /// 
    /// ```
    /// # use multierror::Fallible;
    /// let mut source = Fallible::new(42);
    /// source.push_error("oh no!");
    /// source.push_error("another error!");
    /// let mut dest = Fallible::new(123);
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
    pub fn integrate<OT>(self, other: &mut Fallible<OT, E>, func: impl FnOnce(&mut OT, T)) {
        func(&mut other.value, self.value);

        for error in self.errors.into_iter() {
            other.push_error(error);
        }
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

impl<T, E> ErrorCollector<E> for Fallible<T, E> {
    fn push_error(&mut self, error: E) {
        Fallible::push_error(self, error);
    }
}
