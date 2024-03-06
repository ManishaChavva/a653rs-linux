use a653rs::bindings::Validity;
use a653rs::partition;
use a653rs::prelude::PartitionExt;
use a653rs_linux::partition::ApexLogger;
use log::LevelFilter;
use cucumber::{given, then, when, steps, World as _};

#[macro_use]
extern crate log;
extern crate cucumber;

fn main() {
    ApexLogger::install_panic_hook();
    ApexLogger::install_logger(LevelFilter::Trace).unwrap();

    gpsd::Partition.run()
}

#[partition(a653rs_linux::partition::ApexLinuxPartition)]
mod gpsd {
    use a653rs_postcard::prelude::*;
    use log::info;

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
    fn periodic_gpsd(ctx: periodic_gpsd::Context) {
        info!("started gpsd process");

        // Defining structure for the position data
        #[derive(serde::Deserialize, serde::Serialize)]
        struct Position {
            x: f32,
            y: u128,
        }

        // TODO receiveing the position data request and respond with the position data

        let new_position = Position { x: 1.5, y: 42 };
        ctx.position_out.unwrap().send_type(&new_position).unwrap();

        // TODO validity check logic

        if let Ok((validity, received_request)) = ctx.plausibility_in.unwrap().recv_type::<bool>() {
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
        struct World {
            user: Option<String>,
            capacity: usize,
            ctx: &'static periodic_cucumber_test::Context<'static>,
            window_positions: Vec<Position>,
            suspicious_detected: bool,
        }

        impl Default for World {
            fn default() -> Self {
                Self {
                    user: None,
                    capacity: 0,
                    suspicious_detected: false,// correct?
                    window_positions: None, // correct?

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
                    .field("user", &self.user)
                    .field("capacity", &self.capacity)
                    .finish()
            }
        }

        #[given(expr = "the GNSS receiver is actively receiving signals")] // Cucumber Expression
        async fn a(w: &mut World) {
            println!("{:?}", w.ctx.get_time());
            // ctx.position_out.unwrap().send_type(5u8).unwrap();
            // w.hypervisor_handle.periodic_wait();
        }

        #[then(expr = "plausibility shall be (true|false)")] // Cucumber Expression
        async fn b(w: &mut World, expexcted_plausibility: bool) {
            // let ( validity, plausibility): (_, bool) =
            // w.hypervisor_handle.plausibility_in.unwrap().recv_type().
            // unwrap(); assert_eq!(validity, Validity::Valid);
            // assert_eq!(plausibility, expexcted_plausibility);
            // w.hypervisor_handle.periodic_wait();
        }

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
        
        
        
        ////////////***////////////////////////
        
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
          

        info!("test 0");
        println!("test 1");

        futures::executor::block_on(World::run("/features"));
    }
}
