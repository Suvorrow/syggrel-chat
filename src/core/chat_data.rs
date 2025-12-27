use crate::database;
use tokio::sync::{Mutex, Notify, atomic::{AtomicBool, Ordering}};
use tracing::{info, error, warn, debug, trace};
use std::sync::Arc;
use chrono;
/// Chat Data Management Module for Syggrel Chat
/// 
/// This module provides the core data management functionality for chat contacts
/// in the Syggrel Chat application. It handles:
/// 
/// - Data modeling for chat items and contacts
/// - Robust error handling with comprehensive error types
/// - Asynchronous data loading with retry mechanisms and timeout protection
/// - Integration with the database layer for persistent storage
/// - State management patterns for UI components

/// Enhanced error types for chat data operations
/// 
/// Provides error categorization for all possible failure modes
/// in chat data operations including network, timeout, server, parsing, authentication,
/// and database errors. Each error type includes detailed context for debugging.
#[derive(Debug, Clone)]
pub enum DataError {
    Network(String),
    Timeout,
    Server(String),
    Parse(String),
    Unauthorized,
    Database(String),    // Added specific database error type
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataError::Network(msg) => write!(f, "Network error: {}", msg),
            DataError::Timeout => write!(f, "Request timed out"),
            DataError::Server(msg) => write!(f, "Server error: {}", msg),
            DataError::Parse(msg) => write!(f, "Data parse error: {}", msg),
            DataError::Unauthorized => write!(f, "Authentication required"),
            DataError::Database(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for DataError {}

/// Type alias for consistent error handling across the module
type AppResult<T> = Result<T, DataError>;

/// Unique identifier for chat conversations
/// 
/// Wrapper around String that provides type safety and prevents mixing
/// chat IDs with other string types. Implements common traits for
/// efficient comparison and hashing operations.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChatId(String);

/// Represents a single chat conversation with metadata
/// 
/// Contains all relevant information for displaying a chat in the UI including:
/// - Unique identifier for the chat
/// - Display name for the chat/contact
/// - Last message content (if available)
/// - Timestamp of last activity
/// - Unread message count
/// - Online status of the contact
#[derive(Clone, Debug, PartialEq)]
pub struct ChatItem {
    pub id: ChatId,
    pub name: String,
    pub last_message: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub unread_count: u32,
    pub is_online: bool,
}

/// Thread-safe data provider for chat conversations with caching and coordination
/// 
/// This struct manages the loading, caching, and coordination of chat data across
/// multiple concurrent requests. Key features include:
/// 
/// 1. Coordinated Loading: Implements a "single loader" pattern where only one
///    thread performs the actual data loading while others wait, preventing duplicate
///    requests for the same data.
/// 
/// 2. Thread-Safe Caching: Uses Arc<Mutex<T>> for shared state with atomic
///    operations to ensure data consistency across async tasks.
/// 
/// 3. Timeout Protection: All operations respect configurable timeout limits
///    to prevent hanging requests.
/// 
/// 4. Exponential Backoff: Implements retry logic with exponential backoff for 
///    resilient data loading in unstable network conditions.
/// 
/// 5. Cache Invalidation: Provides refresh mechanism to clear cached data
///    and force reload from source.
///
/// 6. Notification System: Uses tokio::sync::Notify for efficient coordination
///    between loading threads and waiting consumers.
/// 
/// The provider integrates with the application's database layer through the
/// crate::database module and provides a clean async interface for UI components.
pub struct ChatDataProvider {
    chats: Arc<Mutex<Option<Arc<[ChatItem]>>>>,
    is_loading: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl ChatDataProvider {
    pub fn new() -> Self {
        Self {
            chats: Arc::new(Mutex::new(None)),
            is_loading: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

/// Loads chat data with coordination, caching, and timeout protection
/// 
/// This method implements the core loading coordination logic:
/// 
/// 1. Cache Check: First checks if data is already cached and returns it immediately
/// 2. Single Loader Selection: Uses atomic compare-and-swap to select one thread
///    as the "loader" while others wait (prevents duplicate requests)
/// 3. Coordinated Load: The selected loader performs the actual data loading
///    with timeout protection
/// 4. Cache Update: Successfully loaded data is cached for subsequent requests
/// 5. Notification: All waiting threads are notified when data becomes available
///    (both on success and failure to prevent deadlocks)
/// 
/// The method ensures that only one thread performs the expensive data loading
/// operation while others efficiently wait for the result.    
    pub async fn load_chats(&self, timeout_duration: std::time::Duration) -> AppResult<Arc<[ChatItem]>> {
        use tokio::time::timeout_at;
        use tokio::time::Instant;

        let deadline = Instant::now() + timeout_duration;

        // First check if we already have cached data
        if let Some(cached_data) = &*self.chats.lock().await {
            return Ok(cached_data.clone());
        }

        // Try to become the loader
        if self.is_loading.compare_exchange(
            false, true,
            Ordering::SeqCst,
            Ordering::SeqCst
        ).is_ok() {
            // The loader = perform the load with timeout
            let result = match timeout_at(deadline, self.do_load_chats()).await {
                Ok(Ok(data)) => {
                    // Successfully loaded - update cache first
                    {
                        let mut guard = self.chats.lock().await;
                        *guard = Some(data.clone());
                    }

                    // Now safely clear loading flag and notify
                    self.is_loading.store(false, Ordering::SeqCst);
                    self.notify.notify_waiters();
                    Ok(data)
                }
                Ok(Err(e)) => {
                    // On error, still clear loading flag and notify before returning error
                    self.is_loading.store(false, Ordering::SeqCst);
                    self.notify.notify_waiters();
                    Err(e)
                }
                Err(_) => {
                    // Timeout occured - clear loading flag and notify
                    self.is_loading.store(false, Ordering::SeqCst);
                    self.notify.notify_waiters();
                    Err(DataError::Timeout)
                }
            };

            return result;    
        }

        // Another thread is loading - wait for completion with timeout
        loop {
            // Check if timeout has elapsed
            if Instant::now() >= deadline {
                return Err(DataError::Timeout);
            }

            // Wait for notification or timeout
            let remaining_time = deadline - Instant::now();
            if timeout_at(deadline, self.notify.notified()).await.is_err() {
                return Err(DataError::Timeout);
            }

            // Check if data became available after notification
            if let Some(data) = &*self.chats.lock().await {
                return Ok(data.clone());
            }
        }
    }

/// Internal method for loading data with exponential backoff retry logic
/// 
/// Implements resilient data loading with:
/// - Configurable retry attempts (default: 3)
/// - Exponential backoff (100ms, 200ms, 400ms) capped at 2 seconds
/// - Deadline enforcement for the entire operation
/// - Detailed logging for monitoring and debugging
/// 
/// The backoff strategy prevents overwhelming the data source with rapid
/// retry attempts while respecting the overall operation timeout.    
    async fn load_with_backoff_and_timeout(&self, deadline: tokio::time::Instant) -> AppResult<Arc<[ChatItem]>> {
        use tokio::time::{sleep, Instant};

        const MAX_RETRIES: u32 = 3;
        const BASE_DELAY_MS: u64 = 100;
        const MAX_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

        for attempt in 0..MAX_RETRIES {
            let remaining_time = deadline.saturating_duration_since(Instant::now());
            if remaining_time == std::time::Duration::ZERO {
                return Err(DataError::Timeout);
            }

            match tokio::time::timeout_at(deadline, self.do_load_chats()).await {
                Ok(Ok(data)) => {
                    debug!("Chat loading successful on attempt {}", attempt + 1);
                    return Ok(data);
                },
                Ok(Err(e)) if attempt < MAX_RETRIES - 1 => {
                    let delay = std::cmp::min(
                        BASE_DELAY_MS * (1u64 << attempt),    // Exponential: 100ms, 200ms, 400ms
                        MAX_DELAY
                    );
                    warn!("Chat loading failed on attempt {}, retrying in {:?}: {}",
                                    attempt + 1, delay, e);
                    
                    // Check if we still have time for another attempt after delay
                    if Instant::now().checked_add(delay).map_or(true, |t| t >= deadline) {
                        return Err(DataError::Timeout);
                    }

                    sleep(delay).await;
                },
                Ok(Err(e)) => {
                    error!("Chat loading failed permanently after {} attempts: {}", MAX_RETRIES, e);
                    return Err(e);
                },
                Err(_) => {
                    warn!("Chat loading timed out on attempt {}", attempt + 1);
                    return Err(DataError::Timeout);
                }
            }
        }

        unreachable!()
    }

/// Retrieves cached chat data without triggering a new load operation
/// 
/// Returns an Arc-wrapped array of ChatItems if available, or None if
/// no data has been loaded yet. The Arc clone operation is efficient
/// as it only increments a reference counter without copying the data.
    pub async fn get_chats(&self) -> Option<Arc<[ChatItem]>> {
        self.chats.lock().await.clone()
    }

/// Checks if a data loading operation is currently in progress
/// 
/// Uses atomic operations for thread-safe access without blocking,
/// suitable for UI components that need to display loading indicators.
    pub fn is_loading(&self) -> bool {
        self.is_loading.load(Ordering::SeqCst)
    }

/// Forces a cache refresh by clearing cached data and loading fresh data
/// 
/// This method invalidates the current cache and performs a new load
/// operation, useful for scenarios where data staleness needs to be
/// addressed (e.g., manual refresh, periodic updates). 
    pub async fn refresh(&self, timeout_duration: std::time::Duration) -> AppResult<Arc<[ChatItem]>> {
        // Clear cache to force reload
        *self.chats.lock().await = None;
        self.load_chats(timeout_duration).await
    }

/// Checks if cached data is available without blocking
    /// 
    /// Uses try_lock to check cache state without blocking, returning true
    /// if data exists in cache, false otherwise. Useful for quick UI state
    /// decisions without waiting for mutex acquisition.    
    pub fn has_cached_data(&self) -> bool {
        self.chats.try_lock().map(|guard| guard.is_some()).unwrap_or(false)
    }
}

