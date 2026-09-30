use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.first().map(String::as_str) == Some("seed") {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
        let db = sea_orm_migration::sea_orm::Database::connect(&url)
            .await
            .expect("connect failed");
        migration::seed::seed(&db).await.expect("seed failed");
        println!("seeded");
    } else {
        cli::run_cli(migration::Migrator).await;
    }
}
