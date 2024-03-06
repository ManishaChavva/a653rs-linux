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

// Function to calculate distance between two positions
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

// Spoofing detection algorithm
fn is_plausible_movement(window_positions: &[Position]) -> bool {
    // Call the is_plausible_movement function from the detector module
    detector::is_plausible_movement(window_positions)
}

// Function to parse timestamp string into SystemTime
fn parse_timestamp(time_str: &str) -> SystemTime {
    // Parse timestamp using chrono library
    DateTime::parse_from_rfc3339(time_str)
        .unwrap_or_else(|_| SystemTime::now())
        .into()
}
// Step definition 
#[given("the position is at latitude: {float}, longitude: {float}, altitude: {float}, time: {string}, speed: {float}")]
async fn set_position(w: &mut World, latitude: f64, longitude: f64, altitude: f32, time_str: String, speed: f32) {
    let sys_timestamp = SystemTime::now();
    let gps_timestamp = parse_timestamp(&time_str);
    let new_position = Position {
        latitude,
        longitude,
        altitude,
        system_timestamp: sys_timestamp,
        gps_time: gps_timestamp,
        speed,
    };

    w.window_positions.push(new_position);
}

#[when("the vehicle moves {float}km {word} in the next {int} seconds")]
async fn move_vehicle(w: &mut World, distance: f64, direction: String, duration: u64) {
    

    // Calculate the change in latitude and longitude based on the direction
    let (lat_change, lon_change) = match direction.as_str() {
        "south" => (distance / 111.0, 0.0),
        "north" => (-distance / 111.0, 0.0),
        "east" => (0.0, distance / (111.0 * f64::cos(w.window_positions[0].latitude.to_radians()))),
        "west" => (0.0, -distance / (111.0 * f64::cos(w.window_positions[0].latitude.to_radians()))),
        _ => (0.0, 0.0), // No movement if direction is not recognized
    };

    // Calculate the time interval for each step of the movement
    let time_interval = Duration::from_secs(duration / (distance as u64));

    // Simulate the movement by updating the position at each time step
    let mut current_position = w.window_positions.last().unwrap().clone(); // Get the last known position
    for _ in 0..duration {
        
        current_position.latitude += lat_change;
        current_position.longitude += lon_change;

        current_position.system_timestamp += time_interval;

        
        current_position.gps_time = current_position.system_timestamp.into();

        
        w.window_positions.push(current_position.clone());
    }
}

#[then("no plausibility is assumed")]
async fn no_plausibility_assumed(w: &mut World) {
    
    assert!(!w.suspicious_detected);
}





#[given("the position data is incomplete or missing")]
async fn incomplete_or_missing_data(w: &mut World) {
    // Check if there are positions available
    if !w.window_positions.is_empty() {
        // Remove the last known position to simulate incomplete or missing data
        w.window_positions.pop();
    }
}

#[when("the system encounters missing or incomplete GPS data")]
async fn encounter_missing_data(w: &mut World) {
    
    // system detects missing data when the last known position is incomplete
    let last_position = w.window_positions.last().unwrap();
    if last_position.latitude.is_nan() || last_position.longitude.is_nan() || last_position.altitude.is_nan() || last_position.time.is_nan() || last_position.speed.is_nan() {
        
        // setting the suspicious_detected flag to true to indicate the system has encountered missing data
        w.suspicious_detected = true;
    }
}

#[then("no plausibility is assumed after missing GPS data")]
async fn no_plausibility_after_missing_data(w: &mut World) {
    
    assert!(w.suspicious_detected);
}


#[when("the vehicle experiences a sudden altitude increase of {int} meters in the next {int} seconds")]
async fn sudden_altitude_increase(w: &mut World, increase: i32, duration: i32) {
    
    let last_position = w.window_positions.last_mut().unwrap();
    last_position.altitude += increase as f32;

    
    let time_interval = Duration::from_secs(duration as u64);
    last_position.system_timestamp += time_interval;
    last_position.gps_time = last_position.system_timestamp.into();
}

#[then("plausibility is assumed")]
async fn plausibility_assumed(w: &mut World) {
    
    assert!(w.suspicious_detected);
}



#[when("the system detects a time drift of {int} seconds between GPS time and system time")]
async fn detect_time_drift(w: &mut World, drift: i32) {
    
    let last_position = w.window_positions.last_mut().unwrap();
    
    
    let drift_duration = Duration::from_secs(drift as u64);
    last_position.gps_time += drift_duration;

    
    last_position.system_timestamp += drift_duration;
}



#[when("the vehicle accelerates to a speed of {float} meters per second in the next {int} seconds")]
async fn accelerate_to_speed(w: &mut World, target_speed: f32, duration: i32) {
    
    let time_interval = Duration::from_secs(duration as u64);
    let last_position = w.window_positions.last_mut().unwrap();
    let initial_speed = last_position.speed;
    let acceleration = (target_speed - initial_speed) / duration as f32;
    
    
    for _ in 0..duration {
        last_position.speed += acceleration;
        last_position.system_timestamp += time_interval;
        last_position.gps_time = last_position.system_timestamp.into();
        w.window_positions.push(last_position.clone());
    }
}



#[when("the vehicle suddenly changes direction by {int} degrees to the {string} in the next {int} seconds")]
async fn change_direction(w: &mut World, degrees: i32, direction: String, duration: i32) {
    
    let last_position = w.window_positions.last_mut().unwrap();
    let angle_radians = degrees as f64 * (std::f64::consts::PI / 180.0);
    let (lat_change, lon_change) = match direction.as_str() {
        "east" => (0.0, angle_radians.cos()),
        "west" => (0.0, -angle_radians.cos()),
        "north" => (angle_radians.sin(), 0.0),
        "south" => (-angle_radians.sin(), 0.0),
        _ => (0.0, 0.0),
    };

    let time_interval = Duration::from_secs(duration as u64);
    let num_steps = duration * 10; // Dividing duration into smaller steps for smoother movement

    for _ in 0..num_steps {
        last_position.latitude += lat_change / num_steps as f64;
        last_position.longitude += lon_change / num_steps as f64;
        last_position.system_timestamp += time_interval / num_steps as u32;
        last_position.gps_time = last_position.system_timestamp.into();
        w.window_positions.push(last_position.clone());
    }
}




#[tokio::main]
async fn main() {
    World::run("tests/features/gnss_spoofing_detection").await;
}
