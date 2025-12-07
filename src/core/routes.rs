#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/menu")]
    Menu {},
    #[route("/settings")]
    Settings {},
}