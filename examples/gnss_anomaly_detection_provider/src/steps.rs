use cucumber::{given, then, when, steps, World as _};
use std::time::{Duration, SystemTime};
use std::collections::HashMap;
use std::thread::sleep;
use std::io;
use std::net::TcpStream;
use chrono::{DateTime, Utc};
use gpsd_proto::{Tpv, ResponseData};

#[derive(cucumber::World, Debug, Default)]
struct World {
    window_positions: Vec<Position>,
    suspicious_detected: bool,
}

#[derive(Default)]
struct Position {
    latitude: f64,
    longitude: f64,
    altitude: f32,
    system_timestamp: SystemTime,
    gps_time: DateTime<Utc>,
    speed: f32,
}

/* 
//Function to calculate distance between two positions
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

    // Function to calculate time difference between two positions
    pub fn time_difference(&self, other: &Position) -> f64 {
        let sys_time_diff = other
            .system_timestamp
            .duration_since(self.system_timestamp)
            .unwrap_or(Duration::from_secs(0));
        sys_time_diff.as_secs_f64()
    }

    // Function to calculate GPS time difference between two positions
    pub fn gps_time_difference(&self, other: &Position) -> f64 {
        let gps_time_diff = other
            .gps_time
            .signed_duration_since(self.gps_time)
            .num_seconds() as f64;
        gps_time_diff
    }
}

/* ONCE ASK AND CHECK????????????????????????
// Spoofing detection algorithm
fn is_plausible_movement(window_positions: &[Position]) -> bool {
    // Call the is_plausible_movement function from the detector module
    detector::is_plausible_movement(window_positions)
} */

// Function to parse timestamp string into SystemTime
fn parse_timestamp(time_str: &str) -> SystemTime {
    // Parse timestamp using chrono library
    DateTime::parse_from_rfc3339(time_str)
        .unwrap_or_else(|_| SystemTime::now())
        .into()
}
*/

// Step definitions
        
// Scenario: No plausibility assumed after legitimate movement
#[given(regex = r"^the position is at latitude: ([-\d\.]+), longitude: ([-\d\.]+), altitude: ([\d\.]+), time: ([\d\-T:Z]+), speed: ([\d\.]+)$")]
fn given_vehicle_position(world: &mut World, lat: f64, long: f64, alt: f32, time_str: &str, speed: f32) {
    let system_timestamp = SystemTime::now(); 
    let gps_time = Utc.datetime_from_str(time_str, "%Y-%m-%dT%H:%M:%SZ").expect("Failed to parse time");

    let position = Position {
        latitude: lat,
        longitude: long,
        altitude: alt,
        system_timestamp, 
        gps_time,         
        speed,
    };
    world.window_positions.push(position);
}

#[when("the vehicle moves 1km south in the next 3 seconds")]
fn vehicle_moves_south(world: &mut World) {
    if let Some(last_position) = world.window_positions.last_mut() {
        
        last_position.latitude -= 0.009; // Rough approximation: 1km south depending on the latitude
        last_position.speed = 333.33; // 1km in 3 seconds implies a speed of 333.33 m/s
    }
}

#[then(regex = r"^(no )?plausibility is assumed$")]
fn then_plausibility_check(world: &mut World, no_plausibility: Option<&str>) {
    
    let is_plausible = world.plausibility_result;
    match no_plausibility {
        Some(_) => assert!(!is_plausible, "Plausibility was incorrectly assumed."),
        None => assert!(is_plausible, "Plausibility was incorrectly not assumed."),
    }
}




// Scenario: Plausibility assumed after sudden altitude change
	
#[when("the vehicle experiences a sudden altitude increase of 50 meters in the next 10 seconds")]
fn vehicle_experiences_sudden_altitude_increase(world: &mut World) {
    if let Some(last_position) = world.window_positions.last_mut() {
        // Simulate the sudden altitude increase by adding 50 meters to the current altitude
        last_position.altitude += 50.0;

        if let Ok(duration) = Duration::from_secs(10).try_into() {
            last_position.gps_time = last_position.gps_time + duration;
        }
        
        // Optionally, if you're tracking system_timestamp or speed and they're relevant to the scenario,
        // you should update them here as well to reflect the changes accurately.
    }
}

// Time drift for 5 seconds.

#[when("the system detects a time drift of 5 seconds between GPS time and system time")]
fn system_detects_time_drift(world: &mut World) {
    if let Some(last_position) = world.window_positions.last_mut() {
        // Assuming GPS time is ahead of system time by 5 seconds.
        
        let gps_time_drifted = last_position.gps_time + chrono::Duration::seconds(5);
        last_position.gps_time = gps_time_drifted;

    }
}

//Scenario: No plausibility assumed after excessive speed
	
#[when("the vehicle accelerates to a speed of 10 meters per second in the next 5 seconds")]
fn vehicle_accelerates_to_excessive_speed(world: &mut World) {
    if let Some(last_position) = world.window_positions.last_mut() {
      
        last_position.speed = 10.0;

        let time_after_acceleration = last_position.gps_time + chrono::Duration::seconds(5);
        last_position.gps_time = time_after_acceleration;

        
    }
}

//Scenario: Plausibility assumed after deviation in movement pattern

#[when("the vehicle suddenly changes direction by 90 degrees to the east in the next 2 seconds")]
fn vehicle_changes_direction(world: &mut World) {
    if let Some(last_position) = world.window_positions.last_mut() {
        
        last_position.longitude += 0.01;

        let time_after_change = last_position.gps_time + chrono::Duration::seconds(2);
        last_position.gps_time = time_after_change;

    }
}





#[tokio::main]
async fn main() {
    World::run("tests/features/gnss_spoofing_detection").await;
}
