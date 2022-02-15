use porter_core::Porter;
use tokio;
use warp;

mod dto;
mod service;

#[tokio::main]
async fn main() {
    let porter = Porter::new();
    porter = porter.init().await;
    let app = warp::path!("graphql");
    tokio::join!(async {
        warp::serve(app).run(([127, 0, 0, 1], 80)).await;
    },async {
        porter.start().await
    });
}
