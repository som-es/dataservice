use std::error::Error;

use sqlx::PgPool;

pub trait Enhancer {
    type Source;
    type Error: Error;

    fn source(&self) -> &Self::Source;
}

pub struct PostgresEnhancer(pub PgPool);

impl Enhancer for PostgresEnhancer {
    type Source = PgPool;
    type Error = sqlx::Error;

    fn source(&self) -> &Self::Source {
        &self.0
    }
}
