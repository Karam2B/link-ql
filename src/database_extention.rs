#![deny(unused_must_use)]

use sqlx::Database;

pub trait DatabaseExt: Database {
    fn sanitize_start(into: &mut String);
    fn sanitize_end(into: &mut String);
    fn sanitize(string: &str, into: &mut String);
    type IdExpression;
    fn id_on_create_table_expression() -> Self::IdExpression;
    type SinglePrimaryKeyConstaint;
    fn single_primary_key_constaint_expression() -> Self::SinglePrimaryKeyConstaint;
    type MultiplePrimaryKeyConstaint;
    fn multiple_primary_key_constaint_expression() -> Self::MultiplePrimaryKeyConstaint;
}
