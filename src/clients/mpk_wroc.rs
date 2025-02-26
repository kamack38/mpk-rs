use chrono::{Duration, Utc};
use chrono_tz::Europe::Warsaw;
use diqwest::{error::Error as DiqwestError, WithDigestAuth};
use reqwest::{Client as ReqwestClient, Error as ReqwestError, Url};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::{borrow::Borrow, fmt::Debug};
use thiserror::Error;
use url::ParseError;

const HOSTNAME: &str = "https://impk.mpk.wroc.pl:8088/mobile";
const USERNAME: &str = "android-mpk";
const PASSWORD: &str = "g5crehAfUCh4Wust";
const SQL_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum VehicleType {
    #[serde(alias = "b")]
    Bus,
    #[serde(alias = "t")]
    Tram,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Bus {
    #[serde(alias = "v")]
    code: i32,
    #[serde(alias = "c")]
    course: i32,
    #[serde(rename = "x")]
    latitude: f32,
    #[serde(rename = "y")]
    longitude: f32,
    #[serde(alias = "l")]
    line: String,
    #[serde(rename = "type", alias = "t")]
    vehicle_type: VehicleType,
    #[serde(alias = "s")]
    symbol: String,
    #[serde(alias = "d")]
    direction: String,
    #[serde(alias = "e")]
    delay: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BusList {
    timestamp: String,
    buses: Vec<Bus>,
}

impl<'de> Deserialize<'de> for BusList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{SeqAccess, Visitor};
        use std::fmt;

        struct BusListVisitor;

        impl<'de> Visitor<'de> for BusListVisitor {
            type Value = BusList;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an array with a timestamp followed by bus objects")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let timestamp: String = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

                let mut buses = Vec::new();
                while let Some(bus) = seq.next_element()? {
                    buses.push(bus);
                }

                Ok(BusList { timestamp, buses })
            }
        }

        deserializer.deserialize_seq(BusListVisitor)
    }
}

#[derive(Error, Debug, Deserialize)]
#[error("{info}: {message}\nStack trace: {stack_trace}")]
#[serde(rename_all = "camelCase")]
pub struct MpkError {
    info: String,
    message: String,
    stack_trace: String,
}

#[derive(Error, Debug)]
pub enum ClientError {
    #[error(transparent)]
    HttpError(#[from] DiqwestError),
    #[error(transparent)]
    ReqwestError(#[from] ReqwestError),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error(transparent)]
    UrlParseError(#[from] ParseError),
    #[error(transparent)]
    MpkError(#[from] MpkError),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ApiResponse<T> {
    Success(T),
    Error(MpkError),
}

impl<T, E> From<ApiResponse<T>> for Result<T, E>
where
    E: std::convert::From<MpkError>,
{
    fn from(val: ApiResponse<T>) -> Result<T, E> {
        match val {
            ApiResponse::Success(items) => Ok(items),
            ApiResponse::Error(err) => Err(err.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct BusStop {
    #[serde(rename = "l")]
    label: String,
    #[serde(rename = "d")]
    direction: String,
    #[serde(rename = "t")]
    time: String,
    #[serde(rename = "c")]
    course: u32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct CourseInfo {
    #[serde(rename = "c")]
    course: u32,
    #[serde(rename = "p")]
    encoded: String,
    #[serde(rename = "r")]
    r: Vec<Course>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Course {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "t")]
    time: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct PostPlate {
    #[serde(rename = "l")]
    line: String,
    #[serde(rename = "p")]
    post: String,
    #[serde(rename = "s")]
    abbreviations: Vec<String>,
    #[serde(rename = "t")]
    time_table: Vec<PostPlateTimeTable>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct PostPlateTimeTable {
    #[serde(rename = "t")]
    vaild_from: String,
    #[serde(rename = "v")]
    values: Vec<PostPlateTableByDirection>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct PostPlateTableByDirection {
    #[serde(rename = "n")]
    direction: String,
    #[serde(rename = "d")]
    days: Vec<PostPlateDay>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct PostPlateDay {
    #[serde(rename = "d")]
    day_name: String,
    #[serde(rename = "o")]
    order: u32,
    #[serde(rename = "h")]
    hours: Vec<PostPlateHour>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct PostPlateHour {
    #[serde(rename = "h")]
    hour: i16,

    /// Minute in format `<number><abbreviation>`
    #[serde(rename = "m")]
    minutes: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Client {
    client: ReqwestClient,
    username: String,
    password: String,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            client: ReqwestClient::new(),
            username: USERNAME.to_string(),
            password: PASSWORD.to_string(),
        }
    }
}

impl Client {
    pub fn new() -> Self {
        Self {
            client: ReqwestClient::new(),
            username: USERNAME.to_string(),
            password: PASSWORD.to_string(),
        }
    }

    async fn get_data<T, I, K, V>(&self, params: I) -> Result<T, ClientError>
    where
        T: DeserializeOwned + Debug,
        I: IntoIterator,
        <I as IntoIterator>::Item: Borrow<(K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let url = Url::parse_with_params(HOSTNAME, params)?;

        let response = self
            .client
            .get(url)
            .send_with_digest_auth(&self.username, &self.password)
            .await?;

        // let text = response.text().await?;
        //
        // println!("{:?}", text);

        // Ok(Into::<Result<T, MpkError>>::into(serde_json::from_str::<
        //     ApiResponse<T>,
        // >(&text)?)?)

        response.json::<ApiResponse<T>>().await?.into()
    }

    pub async fn get_buses(&self) -> Result<BusList, ClientError> {
        let now = Utc::now().with_timezone(&Warsaw);
        let ten_seconds_ago = now - Duration::seconds(10);
        let formated_date = ten_seconds_ago.format(SQL_DATE_FORMAT).to_string();

        self.get_data(&[("function", "getPositions"), ("date", &formated_date)])
            .await
    }

    pub async fn get_post_info(&self, symbol: &str) -> Result<Vec<BusStop>, ClientError> {
        self.get_data(&[("function", "getPostInfo"), ("symbol", symbol)])
            .await
    }

    pub async fn get_course_posts<I, S>(&self, courses: I) -> Result<Vec<CourseInfo>, ClientError>
    where
        I: IntoIterator<Item = S>, // The function accepts any type that implements IntoIterator
        S: ToString,
    {
        self.get_data(&[
            ("function", "getCoursePosts"),
            (
                "courses",
                &courses
                    .into_iter()
                    .map(|item| item.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
            ),
        ])
        .await
    }

    pub async fn get_post_plate(&self, post: &str, line: &str) -> Result<PostPlate, ClientError> {
        self.get_data(&[
            ("function", "getPostPlate"),
            ("post", post),
            ("line", line),
            ("output", "json"),
        ])
        .await
    }
}

#[test]
fn test_bus_shortcut_from_json() {
    let json = r#"
 {
    "v": 8418,
    "c": 25626631,
    "x": 17.051289,
    "y": 51.11734,
    "l": "N",
    "t": "b",
    "s": "20903",
    "d": "29324",
    "e": 0
}"#;

    let _v: Bus = serde_json::from_str(json).unwrap();
}

#[test]
fn test_bus_full_from_json() {
    let json = r#"
{
  "code": 8418,
  "course": 25626631,
  "x": 17.098055,
  "y": 51.14295,
  "line": "N",
  "type": "BUS",
  "symbol": "24105",
  "direction": "29324",
  "delay": 27000
}"#;

    let _v: Bus = serde_json::from_str(json).unwrap();
}

#[test]
fn test_bus_stop_info() {
    let json = r#"
    [
    	{ "l": "250", "d": "20362", "t": "2025-02-26 23:38:00", "c": 25622727 },
    	{ "l": "250", "d": "9-OBORNIC", "t": "2025-02-27 00:33:00", "c": 25623045 },
    	{ "l": "250", "d": "20362", "t": "2025-02-27 00:58:00", "c": 25626027 }
    ]"#;

    let _v: Vec<BusStop> = serde_json::from_str(json).unwrap();
}

#[test]
fn test_courses_info() {
    let json = r#"
[
	{
		"c": 25622727,
		"p": "_ivH`~fBw@saAkKeE_XgQeRkEgWsbAaRLoZf[yKtSuLhq@rBf[}Gfb@tW_BrXtKoFfOzIre@~^fQzXR`SwDdKgk@dV_FaEqW ",
		"r": [
			{ "s": "20362", "t": "1900-01-01 23:34:00" },
			{ "s": "20327", "t": "1900-01-01 23:37:00" },
			{ "s": "20329", "t": "1900-01-01 23:38:00" },
			{ "s": "20312", "t": "1900-01-01 23:39:00" },
			{ "s": "20301", "t": "1900-01-01 23:40:00" },
			{ "s": "20916", "t": "1900-01-01 23:42:00" },
			{ "s": "120854", "t": "1900-01-01 23:43:00" },
			{ "s": "120852", "t": "1900-01-01 23:44:00" },
			{ "s": "120670", "t": "1900-01-01 23:45:00" },
			{ "s": "120628", "t": "1900-01-01 23:47:00" },
			{ "s": "120666", "t": "1900-01-01 23:49:00" },
			{ "s": "120616", "t": "1900-01-01 23:50:00" },
			{ "s": "120614", "t": "19 00-01-01 23:51:00" },
			{ "s": "10716", "t": "1900-01-01 23:53:00" },
			{ "s": "110654", "t": "1900-01-01 23:54:00" },
			{ "s": "10556", "t": "1900-01-01 23:55:00" },
			{ "s": "10534", "t": "1900-01-01 23:56:00" },
			{ "s": "110274", "t": "1900-01- 01 23:57:00" },
			{ "s": "10373", "t": "1900-01-01 23:58:00" },
			{ "s": "10365", "t": "1900-01-02 00:00:00" },
			{ "s": "1132 2", "t": "1900-01-02 00:02:00" },
			{ "s": "20362", "t": "1900-01-02 00:03:00" }
		]
	},
	{
		"c": 25623045,
		"p": "_i{vH}`~fBw@saA kKeE_XgQeRkEgWsbAaRLoZf[yKtSuLhq@rBf[wEh_@eAEiV}G_l@lHiNbd@enXjAe@wApA}J~I{J~IiA~@mBlAwAb@}AuB",
		"r": [
			{ "s": "20362", "t": "1900-01-01 00:29:00" },
			{ "s": "20327", "t": "1900-01-01 00:32:00" },
			{ "s": "20329", "t": "1900-01-01 00:33:00 " },
			{ "s": "20312", "t": "1900-01-01 00:34:00" },
			{ "s": "20301", "t": "1900-01-01 00:35:00" },
			{ "s": "20916", "t": "1900-01-01 00:37:00" },
			{ "s": "120854", "t": "1900-01-01 00:38:00" },
			{ "s": "120852", "t": "1900-01-01 00:39:00" },
			{ "s": "120670", "t": "1900-01-01 00:40:00" },
			{ "s": "120628", "t": "1900-01-01 00:42:00" },
			{ "s": "120666", "t": "19 00-01-01 00:44:00" },
			{ "s": "120610", "t": "1900-01-01 00:45:00" },
			{ "s": "120615", "t": "1900-01-01 00:46:00" },
			{ "s": "20717", "t": "1900-01-01 00:47:00" },
			{ "s": "23141", "t": "1900-01-01 00:49:00" },
			{ "s": "23143", "t": "1900-01-01 00:51:00" },
			{ "s": "23145", "t": "1900-01-01 00:52:00" },
			{ "s": "23149", "t": "1900-01-01 00:53:00" },
			{ "s": "9-OBORNIC", "t": "1900-01-01 00:54:00" }
		]
	}
]"#;
    let _v: Vec<CourseInfo> = serde_json::from_str(json).unwrap();
}
