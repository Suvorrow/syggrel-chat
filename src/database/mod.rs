/// Initializes the database connection and runs migrations
/// 
/// This function establishes a connection to the SQLite database at the specified path,
/// creates the required tables if they don't exist (via migrations), and stores the
/// connection in a global static variable for use throughout the application.
/// The database is opened in read-write-create mode, meaning it will be created
/// if it doesn't exist. This function should be called once during application startup.
pub mod schema;
pub mod models;

use sea_orm::{Database, DatabaseConnection};
use std::sync::Arc;
use tokio::sync::OnceCell;

static DB: OnceCell<Arc<DatabaseConnection>> = OnceCell::const_new();

pub async fn init_db(db_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db_url = format!("sqlite:{}?mode=rwc", db_path);
    let db = Database::connect(&db_url)
        .await
        .map_err(|e| format!("Database connection failed: {}", e))?;

    // Run migrations
    run_migrations(&db).await?;

    DB.set(Arc::new(db)).map_err(|_| "Failed to set database connection")?;

    Ok(())
}

pub fn get_db() -> Option<Arc<DatabaseConnection>> {
    DB.get().cloned()
}

async fn run_migrations(db: &DatabaseConnection) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use sea_orm::{ConnectionTrait, Statement};

    // Create contacts table if it doesn't exist
    let create_table_sql = r#"
        CREATE TABLE IF NOT EXISTS contacts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            yggdrasil_address TEXT NOT NULL UNIQUE,
            socks5_proxy TEXT NOT NULL,
            display_name TEXT NOT NULL,
            is_active BOOLEAN DEFAULT TRUE,
            last_seen TIMESTAMP,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            is_hidden_peer BOOLEAN DEFAULT FALSE,
            notes TEXT
        )
    "#;

    db.execute(Statement::from_string(db.get_database_backend(), create_table_sql))
        .await
        .map_err(|e| format!("Migration failed: {}", e))?;

    Ok(())    
}

pub async fn ensure_db_initialized() -> Result<(), &'static str> {
    if DB.get().is_none() {
        return Err("Database not initialized. Call init_db() first.");
    }
    Ok(())
}