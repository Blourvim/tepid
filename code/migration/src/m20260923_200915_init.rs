use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        let sql = include_str!("./migrations/m20260923_200915_init/up.sql");
        let db = manager.get_connection();
        db.execute_unprepared(sql).await?;
        Ok(())

    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        let sql = include_str!("./migrations/m20260923_200915_init/down.sql");
        let db = manager.get_connection();
        db.execute_unprepared(sql).await?;
        Ok(())

}
}
