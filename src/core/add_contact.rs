use dioxus::prelude::*;

#[component]
fn AddContact() -> Element {
    let mut contact_address = use_signal(|| String::new());    // Reactive state for peer address
    let mut socks5_proxy = use_signal(|| String::new());    // Reactive state for proxy
}