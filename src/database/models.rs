use crate::database::schema::Entity;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contact {
    pub id: Option<i32>,
    pub yggdrasil_address: String,
    pub socks5_proxy: String,
    pub display_name: String,
    pub is_active: bool,
    pub last_seen: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub is_hidden_peer: bool,
    pub notes: Option<String>,
}

impl Contact {
    pub fn new(
        yggdrasil_address: impl Into<String>,
        socks5_proxy: impl Into<String>,
        display_name: impl Into<String>,
        is_hidden_peer: bool,
    ) -> Self {
        let now = chrono::Utc::now();    // Get current UTC timestamp for record creation
        Self {                           // Initialize new Contact struct
            id: None,                    // No database ID yet (will be assigned on insert)
            yggdrasil_address:yggdrasil_address.into(),    // Store the Yggdrasil network address
            socks5_proxy: socks5_proxy.into(),             // Store SOCKS5 proxy configuration
            display_name: display_name.into(),             // Store user-friendly contact name
            is_active: true,             // Default to active status
            last_seen: None,             // No activity recorded yet
            created_at: now,             // Set creation timestamp to current time
            updated_at: now,             // Set update timestamp to current time
            is_hidden_peer,              // Store whether this is a hidden peer (no TUN interface)
            notes: None,                 // No user notes initially
        }
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.yggdrasil_address.is_empty() {
            return Err(ValidationError::InvalidAddress);
        }
        if self.display_name.trim().is_empty() {
            return Err(ValidationError::InvalidDisplayName);
        }
        if self.is_hidden_peer && self.socks5_proxy.is_empty() {
            return Err(ValidationError::InvalidProxy);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    InvalidAddress,
    InvalidDisplayName,
    InvalidProxy,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::InvalidAddress => write!(f, "Unvalid Yggdrasil address"),
            ValidationError::InvalidDisplayName => write!(f, "Invalid display name"),
            ValidationError::InvalidProxy => write!(f, "Invalid SOCKS5 proxy configuration"),
        }
    }
}

impl std::error::Error for ValidationError {}

impl From<Contact> for ActiveModel {    // Implement conversion from Contact to SeaORM ActiveModel
    fn fro(contact: Contact) -> Self {    // Define the conversion function
        ActiveModel {                     // Create new ActiveModel instance
            id: match contact.id {        
                Some(id) => Set(id),      // If Contact has an ID, tell SeaORM to set it
                None => ActiveValue::NotSet,    // If no ID, tell SeaORM this field isn't set yet
            },
            yggdrasil_address: Set(contact.yggdrasil_address),
            socks5_proxy: Set(contact.socks5_proxy),
            display_name: Set(contact.display_name),
            is_active: Set(contact.is_active),
            last_seen: Set(contact.last_seen),
            created_at: Set(contact.created_at),
            updated_at: Set(contact.updated_at),
            is_hidden_peer: Set(contact.is_hidden_peer),
            notes: Set(contact.notes),
        }
    }
}

/// Converts a database query result row into a Contact struct
/// 
/// This implementation handles the conversion from SeaORM QueryResult (raw database row)
/// to our application-level Contact model. It safely extracts each field from the
/// database row, providing appropriate defaults for missing or invalid values
/// to ensure the application always receives a valid Contact object even if
/// the database row has incomplete data.
impl From<sea_orm::QueryResult> for Contact {
    fn from(row: sea_orm::QueryResult) -> Self {
        Self {
            id: row.try_get("", "id").ok(),
            yggdrasil_address:row.try_get("", "yggdrasil_address").unwrap_or_default(),
            socks5_proxy: row.try_get("", "socks5_proxy").unwrap_or_default(),
            display_name: row.try_get("", "display_name").unwrap_or_default(),
            is_active: row.try_get("", "is_active").unwrap_or(true),
            last_seen: row.try_get("", "last_seen").ok(),
            created_at: row.try_get("", "created_at").unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: row.try_get("", "updated_at").unwrap_or_else(|_| chrono::Utc::now()),
            is_hidden_peer: row.try_get("", "is_hidden_peer").unwrap_or(false),
            notes: row.try_get("", "notes").ok(),
        }
    }
}