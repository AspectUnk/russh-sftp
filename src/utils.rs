use chrono::{DateTime, Utc};
use std::time::SystemTime;

pub fn unix(time: SystemTime) -> u32 {
    DateTime::<Utc>::from(time).timestamp() as u32
}
