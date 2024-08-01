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

    /// Constructs a new `Fallible` with some errors.
    /// 
    /// ```
    /// # use multierror::Fallible;
    /// let mut f = Fallible::new_with_errors(42, vec!["an error"]);
    /// assert_eq!(f.len_errors(), 1);
    /// ```
    pub fn new_with_errors(value: T, errors: Vec<E>) -> Self {
        Fallible { value, errors }
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
    
    /// Consumes this `Fallible` and another one, returning a new `Fallible` with their values as a
    /// tuple `(this, other)` and the errors combined.
    /// 
    /// ```
    /// # use multierror::Fallible;
    /// let a = Fallible::new_with_errors(5, vec!["error 1", "error 2"]);
    /// let b = Fallible::new_with_errors(9, vec!["error 3"]);
    /// 
    /// let zipped = a.zip(b);
    /// 
    /// let (value, errors) = zipped.finalize();
    /// assert_eq!(value, (5, 9));
    /// assert_eq!(errors.len(), 3);
    /// # errors.ignore();
    /// ```
    pub fn zip<OT>(self, other: Fallible<OT, E>) -> Fallible<(T, OT), E> {
        Fallible::new_with_errors(
            (self.value, other.value),
            self.errors.into_iter().chain(other.errors).collect(),
        )
    }

    /// Applies a function to the value within this `Fallible`.
    /// 
    /// ```
    /// # use multierror::Fallible;
    /// let f = Fallible::new_with_errors("Hello".to_owned(), vec!["oh no!"]);
    /// let f_rev = f.map(|s| s.len());
    /// 
    /// let (value, errors) = f_rev.finalize();
    /// assert_eq!(value, 5);
    /// assert_eq!(errors.len(), 1);
    /// # errors.ignore();
    /// ```
    pub fn map<R>(self, func: impl FnOnce(T) -> R) -> Fallible<R, E> {
        Fallible::new_with_errors(
            func(self.value),
            self.errors,
        )
    }

    /// Applies a function to the errors within this `Fallible`.
    /// 
    /// ```
    /// # use multierror::Fallible;
    /// let f = Fallible::new_with_errors(42, vec!["oh no!", "something went wrong"]);
    /// let f_mapped = f.map_errors(|e| e.to_uppercase());
    /// 
    /// let (value, errors) = f_mapped.finalize();
    /// assert_eq!(value, 42);
    /// assert_eq!(errors.peek(), &["OH NO!".to_owned(), "SOMETHING WENT WRONG".to_owned()]);
    /// # errors.ignore();
    /// ```
    pub fn map_errors<R>(self, func: impl FnMut(E) -> R) -> Fallible<T, R> {
        Fallible::new_with_errors(
            self.value,
            self.errors.into_iter().map(func).collect(),
        )
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
    type WrappedInner = T;

    fn push_error(&mut self, error: E) {
        Fallible::push_error(self, error);
    }

    fn propagate(self, other: &mut impl ErrorCollector<E>) -> Self::WrappedInner {
        Fallible::propagate(self, other)
    }
}

impl<T, E, C: FromIterator<T>> FromIterator<Fallible<T, E>> for Fallible<C, E> {
    /// Enables an [`Iterator`] of `Fallible` items to be converted into a single `Fallible` whose
    /// item is a collection containing each of the items' values.
    /// 
    /// The errors are aggregated in order.
    /// 
    /// ```
    /// # use multierror::Fallible;
    /// let items = vec![
    ///     Fallible::new_with_errors(1, vec!["error 1", "error 2"]),
    ///     Fallible::new_with_errors(2, vec!["error 3"]),
    ///     Fallible::new_with_errors(3, vec!["error 4", "error 5"]),
    /// ];
    /// 
    /// let combined: Fallible<Vec<u32>, _> = items.into_iter().collect();
    /// 
    /// let (value, errors) = combined.finalize();
    /// assert_eq!(value, vec![1, 2, 3]);
    /// assert_eq!(errors.len(), 5);
    /// # errors.ignore();
    /// ```
    fn from_iter<I: IntoIterator<Item = Fallible<T, E>>>(iter: I) -> Self {
        let mut items = vec![];
        let mut errors = vec![];

        for item in iter {
            items.push(item.value);
            errors.extend(item.errors);
        }

        Fallible::new_with_errors(items.into_iter().collect(), errors)
    }
}
