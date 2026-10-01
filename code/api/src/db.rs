use std::{error::Error, time::Duration};

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend};

use crate::bloom::Bloom;

pub async fn hydrate(db: DatabaseConnection) -> Result<Bloom, Box<dyn Error>> {
    let mut bloom_filter = Bloom::new(2_000_000);

    //todo: partial hydration here to make startup faster and easier for development,
    //should be replaced with a enviroment flag
    let rows = db
        .query_all(sea_orm::Statement::from_string(
            DbBackend::Postgres,
            "SELECT key_hash FROM revoked_keys LIMIT 100",
        ))
        .await?;

    for (i, row) in rows.iter().enumerate() {
        let token: String = row
            .try_get("", "key_hash")
            .map_err(|e| format!("failed to read 'token' column: {e}"))?;
        if i == 0 {
            println!("{:?}", &token)
        }
        bloom_filter.push(&token);
    }

    println!("Hydration complete");
    Ok(bloom_filter)
}

pub async fn start_db(connection_string: &str) -> Result<DatabaseConnection, sea_orm::DbErr> {
    let mut opt = sea_orm::ConnectOptions::new(connection_string);
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(false);

    let db = sea_orm::Database::connect(opt).await?;
    Ok(db)
}

