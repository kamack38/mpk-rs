use mpk_rs::clients::mpk_wroc::Client;
use tokio;

#[tokio::main]
async fn main() {
    let client = Client::new();
    let buses = client.get_buses().await;
    println!("{:?}", buses);
    let stop_info = client.get_post_info("20329").await.unwrap();
    println!("{:?}", stop_info);
    let courses_info = client.get_course_posts(["25622727", "25623045"]).await;
    println!("{:?}", courses_info);
    let post_plate = client.get_post_plate("20329", "250").await;
    println!("{:?}", post_plate);
}
