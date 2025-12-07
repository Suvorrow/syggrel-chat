use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "contacts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    #[sea_orm(unique, indexed)]
    pub yggdrasil_address: String,    // The Yggdrasil IPv6 address
    #[sea_orm(indexed)]
    pub socks5_proxy: String,    // SOCKS5 proxy for hidden peers
    pub display_name: String,    // User-friendly name
    pub is_active: bool,    // Connection status
    pub last_seen: Option<DateTimeUtc>,    // Last activity timestamp
    pub created_at: DateTimeUtc,    // Record creation time
    pub updated_at: DateTimeUtc,    // Last update time
    pub is_hidden_peer: bool,
    pub notes: Option<String>,
}

#[dertive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            is_active: Set(true),
            ..Default::default()
        }
    }

    fn before_save(mut self, _insert: bool) -> Result<Self, DdErr> {
        self.updated_at = Set(chrono::Utc::now());
        Ok(self)
    }
}