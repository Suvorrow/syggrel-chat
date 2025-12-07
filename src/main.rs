mod core {
    pub mod routes;
}
use core::routes::Route;
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch_cfg(
        app,
        dioxus::desktop::Config::default()
            .with_window(
                dioxus::desktop::WindowBuilder::new()
                    .with_inner_size(764.0, 480.0)
                    .with_title("Syggrel chat")
            )
    );
}
