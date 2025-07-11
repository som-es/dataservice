use sqlx::PgPool;


pub trait Enhancer {
    type Source;

    fn source(&self) -> &Self::Source;

}

pub struct PostgresEnhancer(PgPool);

impl Enhancer for PostgresEnhancer {
    type Source = PgPool;

    fn source(&self) -> &Self::Source {
        &self.0
    }
}

pub trait VoteResultEnhancer: Enhancer {

}