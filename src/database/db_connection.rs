use crate::database::DatabaseConfig;
use sea_orm::{Database, DatabaseConnection, DbErr};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

pub struct DatabaseManager {
    connection: Arc<DatabaseConnection>,
    config: DatabaseConfig,
}

impl DatabaseManager {
    /// Creates a new DatabaseManager instance with connection pooling and validation
    pub async fn new(config: DatabaseConfig) -> Result<Self, DatabaseError> {
        let db_url = format!(
            "sqlite:{}?mode=rwc&busy_timeout={}&max_connections={}&journal_mode=WAL",
            config.path,
            config.busy_timeout,
            config.max_connections.unwrap_or(2)    // Default pool size
        );

        let connection = Database::connect(&db_url)
            .await
            .map_err(|e| DatabaseError::ConnectionFailed(e.to_string()))?;

        let manager = Self {
            connection: Arc::new(connection),
            config,
        };

        // Validate the connection works
        manager.validate_connection().await?;

        Ok(manager)
    }

    /// Validates that the database connection is functional
    async fn validate_connection(&self) -> Result<(), DatabaseError> {
        // Test query with timeout to prevent hanging
        let result = timeout(
            Duration::from_secs(10),
            self.connection.execute(sea_orm::Statement::from_string(
                sea_orm::SqlxSqliteQueryBuilder,
                "SELECT 1".to_string(),
            ))
        ).await;

        match result {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(DatabaseError::ConnectionFailed(format!("Validation query failed: {}", e))),
            Err(_) => Err(DatabaseError::Timeout("Connection validation timed out".to_string)),
        }
    }

    /// Get a clone of the database connection for use in queries
    pub fn get_connection(&self) -> Arc<DatabaseConnection> {
        self.connection.clone()
    }

    /// Get a reference to the database configuration
    pub fn get_config(&self) -> &DatabaseConfig {
        &self.config
    }

    /// Test the health of the database connection
    pub async fn health_check(&self) -> bool {
        self.validate_connection().await.is_ok()
    }
}

impl Drop for DatabaseManager {
    /// Ensure proper cleanup when DatabaseManager is dropped
    fn drop(&mut self) {
        // SeaORM handles connection cleanup automatically
        log::debug!("DatabaseManager dropped, connection will be closed");
    }
}

#[derive(Debug, Clone)]
pub enum DatabaseError {
    ConnectionFailed(String),
    InvalidConfig(String),
    MigrationFailed(String),
    Timeout(String),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::ConnectionFailed(msg) => write!(f, "Database connection failed: {}", msg),
            DatabaseError::InvalidConfig(msg) => write!(f, "Invalid database configuration: {}", msg),
            DatabaseError::MigrationFailed(msg) => write!(f, "Database migration failed: {}", msg),
            DatabaseError::Timeout(msg) => write!(f, "Database operation timed out: {}", msg),
        }
    }
}

impl std::error::Error for DatabaseError {}

impl From<DbErr> for DatabaseError {
    fn from(err: DbErr) -> Self {
        DatabaseError::ConnectionFailed(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::time::Duration;

    #[tokio::test]
    async fn test_database_manager_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_string_lossy().to_string();

        let config = DatabaseConfig {
            path: db_path,
            busy_timeout: 10000,
            max_connections: Some(2),
        };

        let result = DatabaseManager::new(config).await;
        assert!(result.is_ok());

        let manager = result.unwrap();
        assert!(manager.health_check().await);
    }

    #[tokio::test]
    async fn test_database_manager_with_invalid_path() {
        let config = DatabaseConfig {
            path: "/invalid/path/database.db".to_string(),
            busy_timeout: 10000,
            max_connections: Some(2),
        };

        let result = DatabaseManager::new(config).await;
        assert!(matches!(result, Err(DatabaseError::ConnectionFailed(_))));
    }

    #[tokio::test]
    async fn test_database_manager_get_connection() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_string_lossy().to_string();

        let config = DatabaseConfig {
            path: db_path,
            busy_timeout: 10000,
            max_connections: Some(2),
        };

        let manager = DatabaseManager::new(config).await.unwrap();
        let connection = manager.get_connection();

        // Verify we can get a connection
        assert!(connection.is_valid().await.is_ok());
    }

    #[tokio::test]
    async fn test_health_check() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_string_lossy().to_string();

        let config = DatabaseConfig {
            path: db_path,
            busy_timeout: 10000,
            max_connections: Some(2),
        };

        let manager = DatabaseManager::new(config).await.unwrap();
        assert!(manager.health_check().await);
    }
}