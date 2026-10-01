#![allow(non_camel_case_types)]
#![allow(unexpected_cfgs)]

use sqlx::{Encode, Type};

use crate::database_extention::DatabaseExt;
pub mod basic_expressions;
pub mod combinators;
pub mod sanitize_combinator;
pub use basic_expressions::Bind;
pub use combinators::{Join, Prefixed};
pub use sanitize_combinator::Sanitize;
pub mod statements;
pub mod std_impls;
pub mod trait_objects;

pub mod essential_syntax {
    pub const OPEN_PARANTHESIS: &str = "(";
    pub const CLOSE_PARANTHESIS: &str = ")";
}

pub struct StatementBuilder<'q, S>
where
    S: DatabaseExt,
{
    pub(crate) stmt: String,
    count: usize,
    arg: S::Arguments<'q>,
}

impl<'q, S: DatabaseExt> StatementBuilder<'q, S> {
    pub fn stmt(&self) -> &str {
        &self.stmt
    }

    pub fn bind<V>(&mut self, value: V)
    where
        V: Encode<'q, S> + 'q + Type<S>,
    {
        use sqlx::Arguments;
        self.arg.add(value).expect("when does this ever fail?");
        self.count += 1;
        self.stmt.push_str(format!("${}", self.count).as_str());
    }

    pub fn sanitize(&mut self, display: &str) {
        S::sanitize_start(&mut self.stmt);
        S::sanitize(display, &mut self.stmt);
        S::sanitize_end(&mut self.stmt);
    }

    /// push str that is known to not cause sql injection,
    ///
    /// the type for the syntax is `&'static str`, because
    /// it is less likely to be created by a network request
    /// (unless you maliciously leak a string like `String::from(from_network).leak()`)
    /// that is fine because on the backend you can only protect against bad code not malicious one
    pub fn syntax(&mut self, str: &'static str) {
        self.stmt.push_str(str);
    }

    pub fn type_as_syntax<T: Type<S>>(&mut self) {
        use sqlx::TypeInfo;
        self.stmt.push_str(T::type_info().name());
    }

    pub fn unwrap(self) -> (String, S::Arguments<'q>) {
        (self.stmt, self.arg)
    }
}

// the only global constructor for StatementBuilder
impl<'q, S: DatabaseExt> Default for StatementBuilder<'q, S> {
    fn default() -> Self {
        StatementBuilder {
            stmt: String::new(),
            count: 0,
            arg: S::Arguments::default(),
        }
    }
}

pub trait RefOpExpression: OpExpression {}

pub trait RefExpression<'a, S>: RefOpExpression
where
    S: DatabaseExt,
{
    fn ref_expression<'q>(&'a self, ctx: &mut StatementBuilder<'q, S>);
}

// impl<'q, S, T> Expression<'q, S> for T
// where
//     S: DatabaseExt,
//     T: RefExpression<S>,
// {
//     fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
//         self.ref_expression(ctx);
//     }
// }

/// trait to lock implementing Expression over downstream's S
///
/// Independant from S in Expression<'q, S>
///
/// ## deprication
/// is_op used to be a part of the trait, but no longer
pub trait OpExpression {
    /// False when this value writes no SQL, so [`Join`](combinators::Join) can skip it.
    fn is_expression_present(&self) -> bool {
        true
    }
}

/// Representing an sql string/expression/statement.
///
/// # Type Generics `S`
/// representing types that implement `sqlx::Database` like `Sqlite` and `MySQL`
///
/// # lifetime Generics `'q`
/// represent the ability to send a reference to an in-memory database
///
/// example
/// ```ignore
///     use sqlx::Sqlite;
///     use linked_sql::{
///         connect_in_memory::ConnectInMemory,
///         query_builder::{Expression, OpExpression, QueryBuilder},
///         use_executor,
///     };
///
///     struct Str<'a>(&'a str);
///     impl<'q> OpExpression for Str<'q> {}
///     impl<'q> Expression<'q, Sqlite> for Str<'q> {
///         fn expression(self, ctx: &mut QueryBuilder<'q, Sqlite>) {
///             ctx.syntax(&"SELECT ");
///             ctx.bind(self.0);
///             ctx.syntax(&";");
///         }
///     }
///
///     #[tokio::main]
///     async fn main() {
///         let pool = Sqlite::connect_in_memory().await;
///
///         let mut statment = String::from("hello world");
///
///         let holding_lifetime = QueryBuilder::new(Str(
///             &   /*'statment*/   statment
///         ));
///         
///         // restricted region
///         // let _ = &mut statment
///
///         let lifetime_droped = use_executor!(fetch_one(&pool, holding_lifetime)).unwrap();
///     }
/// ```
///
/// the only reason why there is lifetime in expression interface is: because in Sqlite you can send a string reference to an in-memory-database (instead of serializing the ref to an owned String and sending it over the netword like MySQL and PostgreQL), so you would have to wait for the lifetime (impl Expression<Sqlite, 'q> for &'q str) to be droped before you can mutate or move the referenced string
///
/// in fact all impelentation of `sqlx::Encode` (used internally by `QueryBuilder::bind`) are static expect for `impl<'s> Encode<'s, Sqlite> for &'s str`
///
/// this lifetime itroduce restriction on all references made between `holding_lifetime` and `lifetime_droped`
///
/// Don't feel the need to abstract over this lifetime, look for statics instead of introducing a new lifetime (i.e. where T: Expression<'static, S>), especially when you are building over-the-netword backends where everything is static anyway, lifetime here is to have a perfect API that I don't need to refactor later.
///
/// # Implementing `Expression`
///
/// ## Composition
/// Types that implement `Expression` often are composable, there are only
/// two valid ways to compose generics.
///
/// 1. `where Generic: Expression<'q, S>`
///
/// ```ignore
/// struct ColEq<C, V> {
///     pub col: C,
///     pub value: V,
/// }
/// impl<'q, S, C, V> Expression<'q, S> for ColEq<C, V>
/// where
///     C: Expression<'q, S>,
///     V: Expression<'q, S>,
/// {
///     ... some code ...
/// }
/// ```
///
/// note that this put the responsibility of valid composition on the creator of the type,
/// this is valid `ColEq { col: "column", value: Bind(34) }` while
/// this is invalid `ColEq { col: "column", value: 34 }`, because i32
/// by itself does not implement `Expression`.
///
/// 2. `where Join<Generic>: Expression<'q, S>`
/// this is where you need to join multple `impl Expression`s together,
///
/// ## Non-operational implementation
/// all implementation should be operation -- meaning they add some content to the sql
/// statement, meaning types like (), Option<T>, Vec<T> should not implement Expression.
///
/// The only exception is `Join` and `Prefixed`, they allow these optional types
/// to be used
///
/// ## `Join`` type
///
/// important type you should be aware of is `Join`, it implements `Expression` when its `Item` generic represents
/// multiple `Expression`s.
/// ```ignore
/// fn test() {
///     let join = Join {
///         start: "SELECT ",
///         separator: ", ",
///         items: (
///             "column",
///             ColumnAs { column: Bind(34), as_: "new_column" }
///         )
///     };
///     
///     assert_eq!(join.string_only(), "SELECT column, $1 AS new_column");
/// }
/// ```
///
/// `Join` is the only type that can be used in generic composition. It
/// should not be directly used by consumer, but rather internally by
/// implementors of `Expression`.
///
/// ## `Bind` type
/// Constrain generics by `'q` is only for `Bind` type
///
/// ```ignore
/// impl<'q, S, V> Expression<'q, S> for Bind<V>
/// where
///     V: 'q + Encode<'q, S>,
/// ```
///
/// other types rely on composition, for example
///
/// ```ignore
/// struct ColEq<C, V> {
///     pub col: C,
///     pub value: V,
/// }
/// impl<'q, S, C, V> Expression<'q, S> for ColEq<C, V>
/// where
///     C: Expression<'q, S>,
///     V: Expression<'q, S>,
/// { ... some code ... }
///
/// fn valid_composition() {
///     let col_eq = ColEq {
///         col: "column",
///         value: Bind(34),
///     };
///     
///     assert_eq!(col_eq.string_only(), "column = $1");
/// }
/// ```
///
/// ## lifetime 'q
/// This lifetime represent the ability to send a reference to an in-memory database.
///
/// In generic composition, 'q is used like this:
/// `where Generic: Expression<'q, S>`
///
/// Generic should never be constrained by `'q`, like:
/// `where Generic: 'q + Expression<'q, S>`,
/// the only exception for `Bind`'s Generic.
pub trait Expression<'q, S>: OpExpression {
    fn expression(self, ctx: &mut StatementBuilder<'q, S>)
    where
        S: DatabaseExt;
}

impl<'q, S, T> Expression<'q, S> for T
where
    S: DatabaseExt,
    T: for<'a> RefExpression<'a, S>,
{
    fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
        self.ref_expression(ctx);
    }
}

impl<'q, S: DatabaseExt> StatementBuilder<'q, S> {
    pub fn new<Expr>(expr: Expr) -> Self
    where
        Expr: Expression<'q, S>,
    {
        let mut this = Self::default();

        expr.expression(&mut this);

        this
    }

    pub fn new_no_data<Expr>(expr: Expr) -> Option<String>
    where
        Expr: Expression<'q, S>,
    {
        let mut this = Self::default();

        expr.expression(&mut this);

        if this.count == 0 {
            Some(this.stmt)
        } else {
            None
        }
    }
}

// pub use __sanitize_many::SanitizeMany;
// pub use __sanitize_many::SanitizeManyTupleSpec;
// mod __sanitize_many {
//     use core::fmt;

//     use crate::{
//         database_extention::DatabaseExt,
//         sqlx_query_builder::{Expression, OpExpression, StatementBuilder},
//         tuple_trait::{Tuple, TupleAsRef, TupleSpec},
//     };

//     pub struct SanitizeManyTupleSpec<'expression_mut_context, 'expression_parameter, S: DatabaseExt>(
//         &'expression_mut_context mut StatementBuilder<'expression_parameter, S>,
//     );

//     impl<'c, 'a, 'q, S> TupleSpec<usize> for SanitizeManyTupleSpec<'a, 'q, S>
//     where
//         S: DatabaseExt,
//     {
//         type Output = ();

//         fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
//             &mut self,
//             member: usize,
//         ) -> Self::Output {
//             S::sanitize(member.to_string().as_str(), &mut self.0.stmt);
//         }
//     }

//     impl<'c, 'a, 'q, S> TupleSpec<&'c str> for SanitizeManyTupleSpec<'a, 'q, S>
//     where
//         S: DatabaseExt,
//     {
//         type Output = ();

//         fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
//             &mut self,
//             member: &'c str,
//         ) -> Self::Output {
//             S::sanitize(member, &mut self.0.stmt);
//         }
//     }

//     impl<'a, 'q, S> TupleSpec<String> for SanitizeManyTupleSpec<'a, 'q, S>
//     where
//         S: DatabaseExt,
//     {
//         type Output = ();

//         fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
//             &mut self,
//             member: String,
//         ) -> Self::Output {
//             S::sanitize(member.as_str(), &mut self.0.stmt);
//         }
//     }

//     impl<'a, 'q, S> TupleSpec<std::sync::Arc<str>> for SanitizeManyTupleSpec<'a, 'q, S>
//     where
//         S: DatabaseExt,
//     {
//         type Output = ();

//         fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
//             &mut self,
//             member: std::sync::Arc<str>,
//         ) -> Self::Output {
//             S::sanitize(member.as_ref(), &mut self.0.stmt);
//         }
//     }

//     impl<'a, 'q, S, AsR> TupleSpec<(AsR,)> for SanitizeManyTupleSpec<'a, 'q, S>
//     where
//         AsR: AsRef<str>,
//         S: DatabaseExt,
//     {
//         type Output = ();

//         fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
//             &mut self,
//             member: (AsR,),
//         ) -> Self::Output {
//             S::sanitize(member.0.as_ref(), &mut self.0.stmt);
//         }
//     }

//     impl<'a, 'q, S, T> TupleSpec<(SanitizeMany<T>,)> for SanitizeManyTupleSpec<'a, 'q, S>
//     where
//         S: DatabaseExt,
//         T: for<'s> Tuple<SanitizeManyTupleSpec<'s, 'q, S>>,
//     {
//         type Output = ();

//         fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
//             &mut self,
//             member: (SanitizeMany<T>,),
//         ) -> Self::Output {
//             Expression::expression(member.0, self.0);
//         }
//     }

//     impl<'a, 'q, S, T> TupleSpec<crate::collections::SingleIncremintalInt<T>>
//         for SanitizeManyTupleSpec<'a, 'q, S>
//     where
//         S: DatabaseExt,
//     {
//         type Output = ();

//         fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
//             &mut self,
//             _member: crate::collections::SingleIncremintalInt<T>,
//         ) -> Self::Output {
//             S::sanitize("id", &mut self.0.stmt);
//         }
//     }

//     impl<'a, 'q, S, T> TupleSpec<SanitizeMany<T>> for SanitizeManyTupleSpec<'a, 'q, S>
//     where
//         S: DatabaseExt,
//         T: for<'s> Tuple<SanitizeManyTupleSpec<'s, 'q, S>>,
//     {
//         type Output = ();

//         fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
//             &mut self,
//             member: SanitizeMany<T>,
//         ) -> Self::Output {
//             Expression::expression(member, self.0);
//         }
//     }

//     impl<'q, S> StatementBuilder<'q, S>
//     where
//         S: DatabaseExt,
//     {
//         pub fn sanitize_many<'a, T>(&mut self, data: T)
//         where
//             T: for<'s> Tuple<SanitizeManyTupleSpec<'s, 'q, S>>,
//         {
//             S::sanitize_start(&mut self.stmt);
//             T::own_tuple_mut_spec(data, SanitizeManyTupleSpec(self));
//             S::sanitize_end(&mut self.stmt);
//         }
//     }

//     #[derive(Clone)]
//     pub struct SanitizeMany<T>(pub T);

//     impl<T> OpExpression for SanitizeMany<T> {
//         fn is_op(&self) -> bool {
//             true
//         }
//     }
//     impl<'q, S, T> Expression<'q, S> for SanitizeMany<T>
//     where
//         T: for<'a> Tuple<SanitizeManyTupleSpec<'a, 'q, S>>,
//         S: DatabaseExt,
//     {
//         fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
//             ctx.sanitize_many(self.0);
//         }
//     }

//     pub struct ToStringSpec<'m>(&'m mut String);

//     impl<T> fmt::Display for SanitizeMany<T>
//     where
//         T: for<'a> TupleAsRef<'a>,
//         for<'a, 'b> <T as TupleAsRef<'a>>::Output: Tuple<ToStringSpec<'b>>,
//     {
//         fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//             let mut string = String::new();

//             let r = self.0.tuple_as_ref();

//             let _ = r.own_tuple_mut_spec(ToStringSpec(&mut string));

//             f.write_str(string.as_str())
//         }
//     }

//     impl<'m, T: AsRef<str>> TupleSpec<&'_ (T,)> for ToStringSpec<'m> {
//         type Output = ();

//         fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
//             &mut self,
//             member: &'_ (T,),
//         ) -> Self::Output {
//             self.0.push_str(member.0.as_ref());
//         }
//     }
//     impl<'m> TupleSpec<&'_ &'static str> for ToStringSpec<'m> {
//         type Output = ();

//         fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
//             &mut self,
//             member: &'_ &'static str,
//         ) -> Self::Output {
//             self.0.push_str(member.as_ref());
//         }
//     }

//     #[cfg(test)]
//     mod tests {
//         use crate::{
//             database_extention::DatabaseExt,
//             sqlx_query_builder::{Expression, OpExpression, StatementBuilder},
//         };
//         use sqlx::Sqlite;

//         pub struct TestSanitizeAndBuild<'q>(&'q str);

//         impl OpExpression for TestSanitizeAndBuild<'_> {
//             fn is_op(&self) -> bool {
//                 true
//             }
//         }
//         impl<'q, 'a, S> Expression<'q, S> for TestSanitizeAndBuild<'a>
//         where
//             S: DatabaseExt,
//         {
//             fn expression(self, ctx: &mut StatementBuilder<'q, S>) {
//                 let local = String::from(" local");
//                 ctx.sanitize_many((self.0, " world", local.as_str()));
//             }
//         }

//         #[test]
//         fn main() {
//             let borrow = String::from("hello");
//             let str = StatementBuilder::<Sqlite>::new(TestSanitizeAndBuild(&borrow))
//                 .unwrap()
//                 .0;

//             assert_eq!(str, "\"hello world local\"");
//         }
//     }
// }

// pub use __sanitize_many_v2::ImplAsRefStr;
// pub use __sanitize_many_v2::SanitizeManyV2;

// mod __sanitize_many_v2 {
//     use crate::{
//         database_extention::DatabaseExt,
//         sqlx_query_builder::{RefExpression, RefOpExpression, StatementBuilder},
//         tuple_trait::{Tuple, TupleAsRef, TupleSpec},
//     };

//     pub struct SanitizeManyTupleSpecV2<'expression_mut_context, 'expression_parameter, S: DatabaseExt>(
//         &'expression_mut_context mut StatementBuilder<'expression_parameter, S>,
//     );

//     fn usize_to_string(member: &usize, stmt: &mut String) {
//         const MAX_DEC_N: usize = usize::MAX.ilog10() as usize + 1;
//         let mut buf = [0u8; MAX_DEC_N];
//         let mut current_index = 0;
//         let mut member = *member;
//         if member == 0 {
//             stmt.push('0');
//             return;
//         } else {
//             while member > 0 {
//                 let least_significant_digit = member % 10;
//                 buf[current_index] = least_significant_digit as u8 + b'0';
//                 current_index += 1;
//                 member /= 10;
//             }
//         }

//         let mut buf = buf[..current_index].into_iter();

//         while let Some(digit) = buf.next_back() {
//             stmt.push(*digit as char);
//         }
//     }

//     impl<'c, 'a, 'q, S> TupleSpec<&str> for SanitizeManyTupleSpecV2<'a, 'q, S>
//     where
//         S: DatabaseExt,
//     {
//         type Output = ();

//         fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
//             &mut self,
//             member: &str,
//         ) -> Self::Output {
//             S::sanitize(member, &mut self.0.stmt);
//         }
//     }
//     impl<'c, 'a, 'q, S> TupleSpec<&usize> for SanitizeManyTupleSpecV2<'a, 'q, S>
//     where
//         S: DatabaseExt,
//     {
//         type Output = ();

//         fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
//             &mut self,
//             member: &usize,
//         ) -> Self::Output {
//             usize_to_string(member, &mut self.0.stmt);
//             // if there is bugs use
//             // S::sanitize(member.to_string().as_str(), &mut self.0.stmt);
//         }
//     }

//     pub struct ImplAsRefStr<T>(pub T);

//     impl<'a, 'q, S, T> TupleSpec<&ImplAsRefStr<T>> for SanitizeManyTupleSpecV2<'a, 'q, S>
//     where
//         T: AsRef<str>,
//         S: DatabaseExt,
//     {
//         type Output = ();

//         fn on_each<const LAST_INDEX: usize, const INDEX: usize>(
//             &mut self,
//             member: &ImplAsRefStr<T>,
//         ) -> Self::Output {
//             S::sanitize(member.0.as_ref(), &mut self.0.stmt);
//         }
//     }

//     #[derive(Clone)]
//     pub struct SanitizeManyV2<T>(pub T);

//     impl<S, T> RefExpression<S> for super::SanitizeMany<T>
//     where
//         S: DatabaseExt,
//         T: for<'r, 'q> TupleAsRef<'r, Output: for<'m> Tuple<SanitizeManyTupleSpecV2<'m, 'q, S>>>,
//     {
//         fn ref_expression<'q>(&self, ctx: &mut StatementBuilder<'q, S>) {
//             S::sanitize_start(&mut ctx.stmt);
//             self.0
//                 .tuple_as_ref()
//                 .own_tuple_mut_spec(SanitizeManyTupleSpecV2(ctx));
//             S::sanitize_end(&mut ctx.stmt);
//         }
//     }
//     impl<T> RefOpExpression for super::SanitizeMany<T> {
//         fn is_op_ref(&self) -> bool {
//             true
//         }
//     }
//     impl<T> RefOpExpression for SanitizeManyV2<T> {
//         fn is_op_ref(&self) -> bool {
//             true
//         }
//     }
//     impl<S, T> RefExpression<S> for SanitizeManyV2<T>
//     where
//         T: for<'r, 'q> TupleAsRef<'r, Output: for<'m> Tuple<SanitizeManyTupleSpecV2<'m, 'q, S>>>,
//         // T::Output: for<'m> Tuple<SanitizeManyTupleSpecV2<'m, 'q, S>>,
//         S: DatabaseExt,
//     {
//         fn ref_expression<'q>(&self, ctx: &mut StatementBuilder<'q, S>) {
//             S::sanitize_start(&mut ctx.stmt);

//             self.0
//                 .tuple_as_ref()
//                 .own_tuple_mut_spec(SanitizeManyTupleSpecV2(ctx));

//             S::sanitize_end(&mut ctx.stmt);
//         }
//     }

//     #[cfg(test)]

//     mod tests {
//         use crate::sqlx_query_builder::{SanitizeManyV2, StatementBuilder};
//         use sqlx::Sqlite;

//         #[test]
//         fn main() {
//             let borrow = String::from("hello");

//             todo!("continue here to allow unconstrained str ref to sanitize");
//             // let str = StatementBuilder::<Sqlite>::new(SanitizeManyV2((borrow.as_str(),)))
//             //     .unwrap()
//             //     .0;

//             // assert_eq!(str, "\"hello world local\"");
//         }
//     }
// }

#[cfg(test)]
mod fix_lifetime_tests {

    use crate::sqlx_query_builder::{
        StatementBuilder, statements::select_statement::SelectStatement,
    };
    use sqlx::Sqlite;

    #[test]
    fn main() {
        let (stmt, _args) = StatementBuilder::<Sqlite>::new(SelectStatement {
            select_items: ("1",),
            from: "t",
            joins: (),
            wheres: (),
            group_by: (),
            order: (),
            limit: (),
        })
        .unwrap();

        assert_eq!(stmt, r#"SELECT "1" FROM "t";"#);
    }
}
