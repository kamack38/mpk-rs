use crate::utils::{empty_string_as_none, trim_string};
use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use futures::{stream, StreamExt};
use reqwest::{Client as ReqwestClient, Error};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::fmt::{Debug, Display};

const API_URLS: [&str; 3] = [
    "https://api.dla.sims.pl",
    "https://api.dlugoleka.sims.pl",
    "https://api.dlugoleka.mp.sims.pl",
];

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bus {
    side_number: String,
    #[serde(with = "ts_milliseconds", rename = "recieveTime")]
    receive_time: DateTime<Utc>,
    is_connected: bool,
    latitude: f32,
    longitude: f32,
    previous_latitude: f32,
    previous_longitude: f32,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    brigade: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    direction: Option<String>,
    /// Exists when `is_connected` is true
    #[serde(default, deserialize_with = "empty_string_as_none")]
    line: Option<String>,
    delay: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusStop {
    #[serde(rename = "busStopCode")]
    code: String,
    #[serde(rename = "busStopName")]
    name: String,
    #[serde(rename = "busStopLatitude")]
    latitude: f32,
    #[serde(rename = "busStopLongitude")]
    longitude: f32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timetable {
    line: TimetableLine,
    direction: TimetableDirection,
    #[serde(with = "ts_milliseconds")]
    timetable_departure_time: DateTime<Utc>,
    show_type: i32,
    departure_hide: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimetableLine {
    id: u32,
    name: String,
    #[serde(deserialize_with = "trim_string")]
    number: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimetableDirection {
    id: u32,
    name: String,
}

#[derive(Clone, Debug)]
pub struct Client {
    client: ReqwestClient,
}

impl Client {
    pub fn new() -> Self {
        Self {
            client: ReqwestClient::new(),
        }
    }

    async fn get_data<T: DeserializeOwned + Debug>(&self, endpoint: &str) -> (Vec<T>, Vec<Error>) {
        let results: Vec<Result<Vec<T>, Error>> = stream::iter(API_URLS)
            .then(|hostname| {
                let url = format!("{}/{}", hostname, endpoint);
                async move { self.client.get(&url).send().await?.json::<Vec<T>>().await }
            })
            .collect()
            .await;

        let (buses, errors): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);

        let buses = buses.into_iter().flat_map(Result::unwrap).collect();
        let errors = errors.into_iter().map(Result::unwrap_err).collect();
        (buses, errors)
    }

    pub async fn get_buses(&self) -> (Vec<Bus>, Vec<Error>) {
        self.get_data("vehicles").await
    }

    pub async fn get_bus_stops(&self) -> (Vec<BusStop>, Vec<Error>) {
        self.get_data("timetables/busStops").await
    }

    pub async fn get_timetable<T: Display>(
        &self,
        bus_stop_code: T,
    ) -> (Vec<Timetable>, Vec<Error>) {
        let endpoint = format!("timetables/busStops/{}", bus_stop_code);
        self.get_data(&endpoint).await
    }
}

#[test]
fn test_bus_from_json() {
    let json = r#"
    {
      "sideNumber": "1007",
      "recieveTime": 1740159556672,
      "isConnected": false,
      "latitude": 51.09502166666667,
      "longitude": 16.962031666666668,
      "previousLatitude": 51.095025,
      "previousLongitude": 16.96203,
      "brigade": "90701"
    }"#;

    let _v: Bus = serde_json::from_str(json).unwrap();
}

#[test]
fn test_bus_stop_from_json() {
    let json = r#"
    {
      "busStopCode": "18360",
      "busStopName": "Grzybowa",
      "busStopLatitude": 51.1589,
      "busStopLongitude": 16.8532
    }"#;

    let v: BusStop = serde_json::from_str(json).unwrap();
    let bus_stop = BusStop {
        code: "18360".to_string(),
        name: "Grzybowa".to_string(),
        latitude: 51.1589,
        longitude: 16.8532,
    };
    assert_eq!(v, bus_stop);
}

#[test]
fn test_timetable_from_json() {
    let json = r#"
    {
      "line": {
        "id": 911,
        "name": "LINIA 911",
        "number": " 911"
      },
      "direction": {
        "id": 3918,
        "name": "PL. GRUNWALDZKI"
      },
      "timetableDepartureTime": 1740174780000,
      "showType": -1,
      "departureHide": false
    }"#;

    let v: Timetable = serde_json::from_str(json).unwrap();
    let bus_stop = Timetable {
        line: TimetableLine {
            id: 911,
            name: "LINIA 911".to_string(),
            number: "911".to_string(),
        },
        direction: TimetableDirection {
            id: 3918,
            name: "PL. GRUNWALDZKI".to_string(),
        },
        timetable_departure_time: DateTime::from_timestamp_millis(1740174780000)
            .expect("invalid timestamp"),
        show_type: -1,
        departure_hide: false,
    };
    assert_eq!(v, bus_stop);
}
