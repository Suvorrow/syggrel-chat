use dioxus::prelude::*;
use crate::data::chat::{ChatDataProvider, ChatItem};
use crate::components::chat_list::ChatList;
use std::sync::Arc;

/// Home Page Component for Syggrel Chat Application
/// 
/// This component serves as the main dashboard/home screen of the Syggrel Chat application.
/// It displays a list of active chat conversations to the user with the following key features:
/// 
/// 1. Navigation Interface: Provides top navigation bar with menu toggle, app title,
///    and quick access buttons to Home, New Chat, and Settings pages. Includes a collapsible
///    sidebar menu accessible via the hamburger menu.
/// 
/// 2. Chat List Management: Dynamically loads and displays active chat conversations
///    using the ChatDataProvider context. Chats are sorted by most recent activity (timestamp).
/// 
/// 3. State Management: Handles multiple UI states including:
///    - Loading state: Shows spinner while fetching chat data
///    - Empty state: Shows "No active chats" message with "Start New Chat" button when no chats exist
///    - Active chats: Displays the list of conversations via ChatList component
/// 
/// 4. Responsive Design: Implements mobile-friendly navigation with collapsible sidebar
///    and appropriate accessibility attributes (ARIA labels, keyboard navigation support).
/// 
/// 5. Data Integration: Integrates with the application's data layer through ChatDataProvider
///    context to fetch, cache, and display chat data with proper error handling and loading states.
/// 
/// The component expects a ChatDataProvider context to be available in the component tree
/// (typically provided by a parent router or app wrapper component). The ChatList component
/// is responsible for rendering individual chat items in a scrollable list format.
/// 
/// Routes used:
/// - "/home" - Home page navigation
/// - "/new-chat" - Create new chat conversation
/// - "/settings" - Application settings
/// - "/contacts" - Contact management

#[component]
pub fn Home() -> Element {
    let show_menu = use_signal(|| false);
    let data_provider = use_context::<ChatDataProvider>();

    // Momoize expensive computations
    let sorted_chats = use_memo(&data_provider, |provider| {
        match provider.get_chats() {
            Some(chats) => {
                let mut sorted = chats.to_vec();
                sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                Some(sorted)
            }
            None => None,
        }
    });

    rsx! {
        div {
            class: "home-container",

            // Top navigation bar
            header {
                class: "top-bar",
                div {
                    class: "left-section",
                    button {
                        class: "menu-button",
                        aria_label: "Toggle menu",
                        onclick: move |_| show_menu.toggle(),
                        "☰ Menu"
                    }
                    h1 { "Syggrel Chat"}
                }
                div {
                    class: "right-section",
                    Link {
                        to: "/home",
                        class: "nav-button home-button"
                    } { "🏠" }
                    Link {
                        to: "/new-chat",
                        class: "nav-button new-chat-button"
                    } { "+" }
                    Link {
                        to: "/settings",
                        class: "nav-button settings-button"
                    } { "⚙️" }
                }
            }
            // Sidebar menu
            if *show_menu.read() {
                div {
                    class: "sidebar-menu",
                    role: "navigation",
                    aria_label: "Main menu",
                    Link {
                        to: "/contacts",
                        class: "menu-item",
                        onclick: move |_| show_menu.set(false)
                    } { "Contacts" }
                    Link {
                        to: "/home",
                        class: "menu-item",
                        onclick: move |_| show_menu.set(false)
                    } { "Home" }
                    Link {
                        to: "/settings",
                        class: "menu-item",
                        onclick: move |_| show_menu.set(false)
                    } { "Settings" }
                }
            }

            // Main content area
            main {
                class: "main-content",
                div {
                    class: "chat-list-container",

                    // Display chats based on state
                    match (data_provider.is_loading(), sorted_chats.as_ref()) {
                        (true, _) => rsx! {
                            div {
                                class: "loading-container",
                                aria_busy: "true",
                                div { class: "loading-spinner" }
                                p { "Loading chats..." }
                            }
                        },
                        (_, Some(chats)) if !chats.is_empty() => rsx! {
                            div {
                                class: "chat-list-content",
                                ChatList {
                                    chats: chats.clone()
                                }
                            }
                        },
                        (_, Some(chats)) if chats.is_empty() => rsx! {
                            div {
                                class: "empty-state",
                                p { "No active chats" }
                                Link {
                                    to: "/new-chat",
                                    class: "primary-button"
                                } { "Start New Chat" }
                            }
                        }
                    }
                }
            }
        }
    }
}