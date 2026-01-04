pub use sea_orm_migration::prelude::*;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260104_092035_create_bd_tbl::Migration),
            Box::new(m20260104_092136_create_usr_tbl::Migration),
        ]
    }
}

mod m20260104_092035_create_bd_tbl;
mod m20260104_092136_create_usr_tbl;
