use a653rs::bindings::Validity;
use a653rs::partition;
use a653rs::prelude::PartitionExt;
use a653rs_linux::partition::ApexLogger;
use log::LevelFilter;
//use std::time::{SystemTime, Duration as StdDuration};  //changes done
use std::collections::HashMap;
use std::thread::sleep;
use std::io;
use std::net::TcpStream;
//use core::time::Duration;
use chrono::Duration;
//use std::sync::{Arc, Mutex};
use chrono::{Utc, DateTime};
//use cucumber::{given, then, when, World as _};
#[macro_use]
extern crate log;
extern crate cucumber;
fn main() {
    ApexLogger::install_panic_hook();
    ApexLogger::install_logger(LevelFilter::Trace).unwrap();
     // Initialize the shared world state
     //let shared_world = Arc::new(Mutex::new(World::default()));
    gpsd::Partition.run()
}
//type SharedWorld = Arc<Mutex<dyn World>>;
#[partition(a653rs_linux::partition::ApexLinuxPartition)]
mod gpsd {
    use a653rs_postcard::prelude::*;
    use log::info;
    //use super::SharedWorld;
    #[sampling_out(name = "position", msg_size = "16B")]
    struct PositionOut;
    #[sampling_in(name = "plausibility", msg_size = "32B", refresh_period = "10s")]
    struct PlausibilityIn;
    #[start(cold)]
    fn cold_start(mut ctx: start::Context) {
        // intialize the request destination port
        ctx.create_position_out().unwrap();
        // intialize the response source port
        ctx.create_plausibility_in().unwrap();
        // launch the periodic process
        ctx.create_periodic_cucumber_test()
            .unwrap()
            .start()
            .unwrap();
    }
    #[start(warm)]
    fn warm_start(ctx: start::Context) {
        cold_start(ctx);
    }
    // the server process is super simple; all it does is receive a request and
    // respond to it
    #[periodic(
        period = "0ms",
        time_capacity = "Infinite",
        stack_size = "8KB",
        base_priority = 1,
        deadline = "Soft"
    )]
    fn periodic_gpsd(ctx: periodic_gpsd::Context) {           //added
        //let mut world = shared_world.lock().unwrap();       //added
        info!("started gpsd process");
        //world.window_positions.push(Position::default());   //added

        // Defining structure for the position data
        #[derive(serde::Deserialize, serde::Serialize)]
        pub struct Position {
            latitude: f64,
            longitude: f64,
            altitude: f32,
            system_timestamp: SystemTime, 
            gps_time: DateTime<Utc>,      
            speed: f32,
        }
        // TODO receiveing the position data request and respond with the position data
        let new_position = ctx.window_positions.last().expect("No position found").clone();  //ctx added
        ctx.position_out.unwrap().send_type(&new_position).unwrap();
        // TODO validity check logic
        let received_value = match ctx.plausibility_in.unwrap().recv_type::<bool>() {
            Ok(value) => value,  // Store the received boolean value
            Err(err) => {
        panic!("Error receiving bool value: {:?}", err);
    }
};

        //if let Ok((validity, received_request)) = ctx.plausibility_in.unwrap().recv_type::<bool>() {
        if let (validity, received_request) = received_value {  // new add
            if validity == Validity::Invalid {
                warn!("Received an invalid request");
                ctx.periodic_wait().unwrap();
                return;
            }
        } else {
            match ctx.position_out.unwrap().send_type(new_position) {
                Err(err) => {
                    error!("Error receiving position request: {:?}", err);
                    ctx.periodic_wait().unwrap();
                    return;
                }
                _ => {}
            }
        }
        ctx.periodic_wait().unwrap();
        
    }
    // Increased stack_size to 8MB so that we never run out of stack (otherwise
    // there might be a segmentation fault)
    #[aperiodic(
        time_capacity = "Infinite",
        stack_size = "8MB",
        base_priority = 1,
        deadline = "Soft"
    )]
    fn periodic_cucumber_test(ctx: periodic_cucumber_test::Context) {
        use cucumber::{given, then, when, World as _};
        use std::sync::atomic::{AtomicPtr, Ordering};
        // Use a static AtomicPtr to leak the ctx into the cucumber test
        static CONTEXT: AtomicPtr<periodic_cucumber_test::Context> =
            AtomicPtr::new(std::ptr::null_mut());
        CONTEXT.store(&ctx as *const _ as *mut _, Ordering::Relaxed);
        #[derive(cucumber::World)]
        pub struct World {
            //user: Option<String>,
            //capacity: usize,
            ctx: &'static periodic_cucumber_test::Context<'static>,
            window_positions: Vec<Position>,
            plausibility_result: Option<bool>,
            //suspicious_detected: bool,
        }

        impl Default for World {
            fn default() -> Self {
                Self {
                    //user: None,
                    //capacity: 0,
                    plausibility_result: None, // or else false?
                    window_positions: Vec::new(),
                    // get the original context from the AtomicPtr. This is not unsafe, because the
                    // ctx lifes at least as long as this process
                    ctx: unsafe {
                        &*(CONTEXT.load(Ordering::Relaxed) as *const _)
                            as &'static periodic_cucumber_test::Context<'static>
                    },
                }
            }
        }
        impl core::fmt::Debug for World {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct("World")
                    //.field("user", &self.user)
                    //.field("capacity", &self.capacity)
                    .field("plausibility_result", &self.plausibility_result) // added
                    .field("window_positions", &self.window_positions)       // added
                    .finish()
            }
        }
// Step definitions
        
// Scenario: No plausibility assumed after legitimate movement
#[given(regex = r"^the position is at latitude: ([-\d\.]+), longitude: ([-\d\.]+), altitude: ([\d\.]+), time: ([\d\-T:Z]+), speed: ([\d\.]+)$")]
fn given_vehicle_position(world: &mut World, window_positions: Vec::new()) {
    let gps_time = Utc.datetime_from_str(time_str, "%Y-%m-%dT%H:%M:%SZ").expect("Failed to parse time"); 
    let system_timestamp = SystemTime::now(); 
    
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

#[when(regex = r"^the vehicle moves (-?\d+)km (north|south|east|west) in the next (\d+) (second|seconds)$")]
fn vehicle_moves_south(world: &mut World) {
    if let Some(last_position) = world.window_positions.last_mut() {
        
        last_position.latitude -= 0.009; // Rough approximation: 1km south depending on the latitude
        last_position.speed = 333.33; // 1km in 3 seconds implies a speed of 333.33 m/s
        
    }
}
#[then(regex = r"^(no )?plausibility is assumed$")]
fn then_plausibility_check(world: &mut World, capture: &str) {
    // Determine plausibility assumption based on the capture group
    let no_plausibility = capture.trim() == "no";

    // Access the stored plausibility result from the world context
    let is_plausible = world.plausibility_result.unwrap_or(false); // Handling None case safely

    if no_plausibility {
        assert!(!is_plausible, "Plausibility was incorrectly assumed.");
    } else {
        assert!(is_plausible, "Plausibility was incorrectly not assumed.");
    }
}


// Scenario: Plausibility assumed after sudden altitude change
#[when(regex = r"^the vehicle experiences a sudden altitude increase of (\d+) meters in the next (\d+) seconds$")]
fn vehicle_experiences_sudden_altitude_increase(world: &mut World) {
    if let Some(last_position) = world.window_positions.last_mut() {
        // Simulate the sudden altitude increase by adding 50 meters to the current altitude
        last_position.altitude += 50.0;
        if let Ok(duration) = chrono::Duration::from_secs(10).try_into() {
            last_position.gps_time = last_position.gps_time + duration;
        }
        
    }
}
// Time drift for 5 seconds.
#[when(regex = r"^the system detects a time drift of (\d+) seconds between GPS time and system time$")]
fn system_detects_time_drift(world: &mut World) {
    if let Some(last_position) = world.window_positions.last_mut() {
        // Assuming GPS time is ahead of system time by 5 seconds.
        
        let gps_time_drifted = last_position.gps_time + chrono::Duration::seconds(5);
        last_position.gps_time = gps_time_drifted;
    }
}
//Scenario: No plausibility assumed after excessive speed
#[when(regex = r"^the vehicle accelerates to a speed of (\d+) meters per second in the next (\d+) seconds$")]
fn vehicle_accelerates_to_excessive_speed(world: &mut World) {
    if let Some(last_position) = world.window_positions.last_mut() {
      
        last_position.speed = 10.0;
        let time_after_acceleration = last_position.gps_time + chrono::Duration::seconds(5);
        last_position.gps_time = time_after_acceleration;
        
    }
}
//Scenario: Plausibility assumed after deviation in movement pattern
#[when(regex = r"^the vehicle suddenly changes direction by 90 degrees to the (north|south|east|west) in the next (\d+) seconds$")]
fn vehicle_changes_direction(world: &mut World) {
    if let Some(last_position) = world.window_positions.last_mut() {
        
        last_position.longitude += 0.01;
        let time_after_change = last_position.gps_time + chrono::Duration::seconds(2);
        last_position.gps_time = time_after_change;
    }
}
    
        info!("test 0");
        println!("test 1");
        futures::executor::block_on(World::run("/features"));
    }
}
