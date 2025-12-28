mod core {
    pub mod routes;
    pub mod chat_data;
}
use core::routes::Route;
use dioxus::prelude::*;
use dioxus::desctop;
use crate::core::chat_data::ChatDataProvider;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Launch desctop app with context
    desktop::launch_cfg(
        app,
        desktop::Config::default()
            .with_window(
                desktop::WindowBuilder::new()
                    .with_inner_size(764.0, 480.0)
                    .with_title("Syggrel Chat")
            )
    );

    Ok(())
}
