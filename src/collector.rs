/// Something which tracks a collection of errors.
/// 
/// This generalizes methods like [`ErrorSentinel::propagate`] which allow errors to be handled by
/// merging them into a different collection of errors.
/// 
/// [`ErrorSentinel::propagate`]: crate::ErrorSentinel::propagate
pub trait ErrorCollector<E> {
    /// The type returned by [`propagate`].
    /// 
    /// [`propagate`]: ErrorCollector::propagate
    type WrappedInner;

    /// Add a new error to the collection of errors.
    fn push_error(&mut self, error: E);

    /// Consumes this collector and pushes all of its errors into a different collector. If the type
    /// is wrapping some kind of value, it may return it too.
    fn propagate(self, other: &mut impl ErrorCollector<E>) -> Self::WrappedInner;
}
