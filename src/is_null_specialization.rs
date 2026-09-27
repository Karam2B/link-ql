use super::IsNull;

impl<T> IsNull for T {
    default fn is_null() -> bool {
        false
    }
}
