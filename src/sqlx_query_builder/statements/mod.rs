pub mod add_column_statement;
pub mod create_table_statement;
pub mod create_trigger;
pub mod delete_statement;
pub mod insert_statement;
pub mod select_statement;
pub mod update_statement;

pub trait Inverse {
    type InverseStatement;
    fn inverse(&self) -> Self::InverseStatement;
}
