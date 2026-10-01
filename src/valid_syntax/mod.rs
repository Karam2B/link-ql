pub trait Members<'a> {
    type Identifier;
    fn identifier(&'a self) -> Self::Identifier;

    type Scoped;
    fn scoped(&'a self) -> Self::Scoped;

    type ScopedAliased;
    fn scoped_aliased(&'a self, alias: &'static str) -> Self::ScopedAliased;

    type NumScopedAliased;
    fn num_scoped_aliased(&'a self, num: usize, alias: &'static str) -> Self::NumScopedAliased;

    // members using dynamic types
    fn members(&self) -> (String, Vec<String>);
}

pub trait IdentifierOnly<'a> {
    type Identifier;
    fn identifier_only(&'a self) -> Self::Identifier;

    // members using dynamic types
    fn members2(&self) -> (String, Vec<String>);
}

impl<'a, T> IdentifierOnly<'a> for T
where
    T: Members<'a>,
{
    type Identifier = <T as Members<'a>>::Identifier;

    #[inline]
    fn identifier_only(&'a self) -> Self::Identifier {
        self.identifier()
    }

    #[inline]
    fn members2(&self) -> (String, Vec<String>) {
        self.members()
    }
}

pub trait TableExpressions<'a>: Members<'a> {
    type SnakeCase;
    type PascalCase;
    fn table_name_snake_case(&'a self) -> Self::SnakeCase;
    fn table_name_pascal_case(&'a self) -> Self::PascalCase;
}

pub trait MigrateExpression<'a> {
    type Migrate;
    fn migrate(&'a self) -> Self::Migrate;
}
