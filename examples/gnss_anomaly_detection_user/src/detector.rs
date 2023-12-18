use std::time::{Duration, SystemTime};
//use std::thread::sleep;
//use std::io;
//use std::net::TcpStream;
use chrono::{DateTime, Utc};
//use gpsd_proto::{Tpv, ResponseData};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Position {
    latitude: f64,
    longitude: f64,
    altitude: f32,
    system_timestamp: SystemTime, // field to store system time
    gps_time: DateTime<Utc>,      // field to store GPS time
    speed: f32,                   // speed in meters per second
}

impl Position {
    pub fn distance_to(&self, other: &Position) -> f64 {
        const EARTH_RADIUS: f64 = 6371e3; // Earth's radius in meters
        let lat_diff = (self.latitude.to_radians() - other.latitude.to_radians()).abs();
        let lon_diff = (self.longitude.to_radians() - other.longitude.to_radians()).abs();
        let a = (lat_diff / 2.0).sin().powi(2)
            + self.latitude.to_radians().cos()
                * other.latitude.to_radians().cos()
                * (lon_diff / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        EARTH_RADIUS * c
    }

    pub fn time_difference(&self, other: &Position) -> f64 {
        let sys_time_diff = other
            .system_timestamp
            .duration_since(self.system_timestamp)
            .unwrap_or(Duration::from_secs(0));
        sys_time_diff.as_secs_f64()
    }
    pub fn gps_time_difference(&self, other: &Position) -> f64 {
        let gps_time_diff = other
            .gps_time
            .signed_duration_since(self.gps_time)
            .num_seconds() as f64;
        gps_time_diff
    }
}
// Spoofing detection algorithm
pub fn is_plausible_movement(window_positions: &[Position]) -> bool {
    const MAX_DISTANCE_THRESHOLD: f64 = 10.0; // units in meters
    const MAX_ALTITUDE_CHANGE_THRESHOLD: f32 = 10.0; // units in meters
    const ALLOWABLE_DRIFT: u64 = 10_000_000; // units in nanoseconds per second
    const MAX_SPEED_THRESHOLD: f64 = 10.115; // maximum speed in meters per second

    let mut max_altitude_change = 0.0; // Initialize max_altitude_change to track the maximum change
    let mut distance_change = 0.0;
    let mut suspicious_detected = false;

    if window_positions.len() < 2 {
        suspicious_detected = true; // Need at least two positions for calculation.
        println!("Need at least 2 positions");
    }

    // Calculate time, altitude difference and distance between each pair of
    // consecutive positions.
    for i in 1..window_positions.len() {
        let distance = window_positions[i - 1].distance_to(&window_positions[i]);
        let altitude_change =
            (window_positions[i - 1].altitude - window_positions[i].altitude).abs();
        let sys_time_difference = window_positions[i - 1].time_difference(&window_positions[i]);
        let gps_time_difference = window_positions[i - 1].gps_time_difference(&window_positions[i]);

        let sys_time_difference_ns = (sys_time_difference * 1_000_000_000.0) as u64; //converting sys_time_difference from f64 to nanoseconds

        let gps_time_drift: u128 = (gps_time_difference - sys_time_difference).abs() as u128; // difference between gps_time_difference and system_time_difference.
        let threshold = sys_time_difference_ns * ALLOWABLE_DRIFT / 1_000_000_000; //converting the result in nanoseconds to seconds.
        let speed = distance / sys_time_difference;

        println!("gps_time_drift: {}", gps_time_drift);
        println!("threshold: {}", threshold);

        println!("sys_time_difference_ns: {}", sys_time_difference_ns);

        if distance > MAX_DISTANCE_THRESHOLD {
            suspicious_detected = true; // Distance exceeds plausible threshold.
            println!("Distance threshold exceeded");
        }

        if altitude_change > MAX_ALTITUDE_CHANGE_THRESHOLD {
            suspicious_detected = true; // altitude exceeds plausible threshold.
            println!("Altitude change threshold exceeded");
        }

        if gps_time_drift > threshold as u128 {
            suspicious_detected = true; // time difference between gps_time _difference and sys_time_difference exceeds
                                        // plausible thereshold.
            println!("gps_time_drift exceeded threshold");
        }
        if speed > MAX_SPEED_THRESHOLD {
            suspicious_detected = true; // Speed exceeds plausible threshold
            println!("speed threshold exceeded");
        }

        // Update the total_distance and max_altitude_change variables
        distance_change = distance;
        max_altitude_change = altitude_change;
    }

    println!("distance change: {}", distance_change);
    println!("Max altitude change: {}", max_altitude_change);

    if suspicious_detected {
        println!("Suspicious movement detected!!!!!!!!!!!!!!!!!!!!!*******************!!!!!!!!!!!!!!!!!!!!!!");
        //suspicious_detected = false; // Reset the flag to false
    } else {
        println!("No suspicious movement detected");
    }
    suspicious_detected
}

// timestamp string in RFC 3339 format and parses it into a SystemTime.
fn parse_timestamp(time_str: &str) -> SystemTime {
    println!("Time to be parsed {time_str:?}");
    match DateTime::parse_from_rfc3339(time_str) {
        Ok(dt) => dt.with_timezone(&Utc).into(),
        Err(_) => {
            println!("Error parsing timestamp: {}", time_str);
            SystemTime::now()
        }
    }
}
