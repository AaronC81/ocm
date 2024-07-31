/// Something which tracks a collection of errors.
/// 
/// This generalizes methods like [`ErrorSentinel::propagate`] which allow errors to be handled by
/// merging them into a different collection of errors.
/// 
/// [`ErrorSentinel::propagate`]: crate::ErrorSentinel::propagate
pub trait ErrorCollector<E> {
    /// Add a new error to the collection of errors.
    fn push_error(&mut self, error: E);
}
