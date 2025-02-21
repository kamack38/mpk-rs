use mpk_rs::clients::sims::Client;
use tokio;

#[tokio::main]
async fn main() {
    let client = Client::new();
    let buses = client.get_buses().await;
    println!("{:?}", buses);
    let bus_stops = client.get_bus_stops().await;
    println!("{:?}", bus_stops);
    let timetables = client.get_timetable(31918010).await;
    println!("{:?}", timetables);
}
