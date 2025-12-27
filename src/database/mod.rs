/// Database module for Syggrel Chat
/// 
/// This module manages the SQLite database connection, handles migrations,
/// and provides functions for database operations. It uses SeaORM as the
/// ORM layer and maintains a single shared connection pool accessible
/// globally via the OnceCell pattern.
use crate::database::models::Contact;
use sea_orm::{
    ColumnTrait, EntityTrait, Database, DatabaseConnection, QueryFilter, QuerySelect
};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{info, error, instrument};

pub mod schema;
pub mod models;

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

#[instrument(skip())]
pub async fn load_contacts_from_db() -> Result<Vec<crate::chat_item::ChatItem>, String> {
    let db = get_db()
        .ok_or_else(|| {
            error!("Database not initialized - call init_db() first");
            "Database not initialized".to_string()
        })?;
    
    // Execute the query to fetch active contacts
    let active_contacts = entity::contact::Entity::find()
        .filter(entity::contact::Column::IsActive.eq(true))
        .all(&*db)    // Dereference Arc to get DatabaseConnection
        .await
        .map_err(|e| {
            error!("Database query failed: {}", e);
            "Failed to load contacts from database".to_string()
        })?;
    
    let chat_items: Vec<crate::chat_item::ChatItem > = active_contacts
        .into_iter()
        .map(|model| crate::chat_item::ChatItem {
            id: model.id.to_string(),    // Convert i32 to string for ChatItem
            name: model.display_name,
            last_message: String::new(),
            timestamp: model.last_seen.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
        })
        .collect();

    info!("Successfully loaded {} active contacts from database", chat_items.len());

    Ok(chat_items)
}