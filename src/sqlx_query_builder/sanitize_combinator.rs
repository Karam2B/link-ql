#[derive(Clone)]
pub struct Sanitize<T>(pub T);

impl<T: SanitizeWrite> std::fmt::Display for Sanitize<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = String::new();
        self.0.write_content::<sqlx::Sqlite>(&mut buf);
        f.write_str(&buf)
    }
}

/// Writes identifier text with no surrounding quotes.
/// [`Sanitize`] adds one pair of quotes around the whole value.
pub trait SanitizeWrite {
    fn write_content<S: crate::database_extention::DatabaseExt>(&self, stmt: &mut String);
}

mod tuple_specifier {
    use super::SanitizeWrite;
    use crate::{
        database_extention::DatabaseExt,
        sqlx_query_builder::{
            OpExpression, RefExpression, RefOpExpression, StatementBuilder,
            sanitize_combinator::Sanitize,
        },
    };

    impl SanitizeWrite for str {
        fn write_content<S: DatabaseExt>(&self, stmt: &mut String) {
            S::sanitize(self, stmt);
        }
    }

    impl SanitizeWrite for String {
        fn write_content<S: DatabaseExt>(&self, stmt: &mut String) {
            S::sanitize(self, stmt);
        }
    }

    impl SanitizeWrite for std::sync::Arc<str> {
        fn write_content<S: DatabaseExt>(&self, stmt: &mut String) {
            S::sanitize(self, stmt);
        }
    }

    impl<T> SanitizeWrite for &T
    where
        T: SanitizeWrite + ?Sized,
    {
        fn write_content<S: DatabaseExt>(&self, stmt: &mut String) {
            (*self).write_content::<S>(stmt);
        }
    }

    impl SanitizeWrite for usize {
        fn write_content<S: DatabaseExt>(&self, stmt: &mut String) {
            const MAX_DEC_N: usize = usize::MAX.ilog10() as usize + 1;
            let mut buf = [0u8; MAX_DEC_N];
            let mut current_index = 0;
            let mut member = *self;
            if member == 0 {
                stmt.push('0');
                return;
            }
            while member > 0 {
                let least_significant_digit = member % 10;
                buf[current_index] = least_significant_digit as u8 + b'0';
                current_index += 1;
                member /= 10;
            }
            let mut buf = buf[..current_index].into_iter();
            while let Some(digit) = buf.next_back() {
                stmt.push(*digit as char);
            }
        }
    }

    macro_rules! impl_sanitize_tuple {
        ($($idx:tt: $T:ident),+) => {
            impl<$($T: SanitizeWrite),+> SanitizeWrite for ($($T,)+) {
                fn write_content<S: DatabaseExt>(&self, stmt: &mut String) {
                    $(
                        self.$idx.write_content::<S>(stmt);
                    )+
                }
            }
        };
    }

    impl_sanitize_tuple!(0: T0);
    impl_sanitize_tuple!(0: T0, 1: T1);
    impl_sanitize_tuple!(0: T0, 1: T1, 2: T2);
    impl_sanitize_tuple!(0: T0, 1: T1, 2: T2, 3: T3);
    impl_sanitize_tuple!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4);
    impl_sanitize_tuple!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5);
    impl_sanitize_tuple!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6);
    impl_sanitize_tuple!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7);

    impl<T> OpExpression for Sanitize<T> {}

    impl<T> RefOpExpression for Sanitize<T> {}

    impl<'a, S, T> RefExpression<'a, S> for Sanitize<T>
    where
        S: DatabaseExt,
        T: SanitizeWrite,
    {
        fn ref_expression<'q>(&'a self, ctx: &mut StatementBuilder<'q, S>) {
            S::sanitize_start(&mut ctx.stmt);
            self.0.write_content::<S>(&mut ctx.stmt);
            S::sanitize_end(&mut ctx.stmt);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::sqlx_query_builder::{Expression, RefExpression, StatementBuilder};
    use sqlx::Sqlite;

    use super::*;

    #[test]
    fn test_sanitize_many_tuple_spec() {
        let mut stmt = StatementBuilder::<Sqlite>::default();

        let local_string = String::from("name");

        let name = (local_string.as_str(), "_1", &4usize);

        Sanitize(name).ref_expression(&mut stmt);

        pretty_assertions::assert_eq!(stmt.stmt(), r#""name_14""#);
    }

    #[test]
    fn test_sanitize_implements_expression() {
        fn assert_expression<T>()
        where
            T: for<'q> Expression<'q, Sqlite>,
        {
        }

        // Test that Sanitize with static 2-tuple implements Expression
        assert_expression::<Sanitize<(&'static str, &'static str)>>();
    }

    #[test]
    fn test_sanitize_short_borrow_implements_expression() {
        let key = String::from("key");
        let migrate = Sanitize((key.as_str(), "_", "todo"));
        // must not require `'static` (concept_6 / RefTuple path)
        let _ = StatementBuilder::<Sqlite>::new(migrate);
    }
}
