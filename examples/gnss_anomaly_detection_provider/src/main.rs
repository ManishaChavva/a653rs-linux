use a653rs::partition;
use a653rs::prelude::PartitionExt;
use a653rs_linux::partition::ApexLogger;
use log::LevelFilter;

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
    use chrono::NaiveDateTime;
    use log::{debug, info, trace};

    // Defining structure for the position data
    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Position {
        latitude: f64,
        longitude: f64,
        altitude: f32,

        /// field to store system time
        system_timestamp: SystemTime,

        /// field to store GPS time
        gps_time: NaiveDateTime,

        /// speed in meters per second
        speed: f32,
    }

    //use super::SharedWorld;
    #[sampling_out(name = "position", msg_size = "1KB")]
    struct PositionOut;

    #[sampling_in(name = "plausibility", msg_size = "1KB", refresh_period = "10s")]
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

    /// Send a position out, wait for one partition period, and receive the
    /// result
    fn send_and_receive(ctx: &periodic_cucumber_test::Context, position: &Position) -> bool {
        debug!("sending out position, waiting for plausibility response");
        trace!("{position:?}");
        ctx.position_out.unwrap().send_type(position).unwrap();
        ctx.periodic_wait().unwrap();

        let (validity, plausibility) = ctx.plausibility_in.unwrap().recv_type::<bool>().unwrap();
        assert_eq!(validity, Validity::Valid);
        debug!("done waiting for plausibility response");

        plausibility
    }

    // Increased stack_size to 8MB so that we never run out of stack (otherwise
    #[periodic(
        period = "0ms",
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
            window_positions: Vec<(Position, bool)>,
        }

        impl Default for World {
            fn default() -> Self {
                debug!("initializing new cucumber world");
                Self {
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
                    .field("window_positions", &self.window_positions) // added
                    .finish()
            }
        }

        // Step definitions

        // Scenario: No plausibility assumed after legitimate movement
        #[given(
            regex = r"^the position is at latitude: ([-\d\.]+), longitude: ([-\d\.]+), altitude: ([\d\.]+), time: ([\d\-T:Z]+), speed: ([\d\.]+)$"
        )]
        fn given_vehicle_position(
            world: &mut World,
            lat: f64,
            lon: f64,
            alt: f32,
            time: String,
            speed: f32,
        ) {
            let gps_time = NaiveDateTime::parse_from_str(&time, "%Y-%m-%dT%H:%M:%SZ")
                .expect("Failed to parse time");
            // .to_utc();

            let system_timestamp = world.ctx.get_time();

            let position = Position {
                latitude: lat,
                longitude: lon,
                altitude: alt,
                system_timestamp,
                gps_time,
                speed,
            };

            let plausibility = send_and_receive(world.ctx, &position);
            world.window_positions.push((position, plausibility));
        }

        #[when(
            regex = r"^the vehicle moves (-?\d+)km (north|south|east|west) in the next (\d+) (second|seconds)$"
        )]
        fn vehicle_moves_south(world: &mut World) {
            if let Some((last_position, _)) = world.window_positions.last_mut() {
                last_position.latitude -= 0.009; // Rough approximation: 1km south depending on the latitude
                last_position.speed = 333.33; // 1km in 3 seconds implies a
                                              // speed of 333.33 m/s
            }
        }
        #[then(regex = r"^(no )?plausibility is assumed$")]
        fn then_plausibility_check(world: &mut World, capture: String) {
            // Determine plausibility assumption based on the capture group
            let no_plausibility = capture.trim() == "no";

            // Access the stored plausibility result from the world context
            let is_plausible = world
                .window_positions
                .iter()
                .all(|(_, plausibility)| *plausibility); // Handling None case safely

            if no_plausibility {
                assert!(!is_plausible, "Plausibility was incorrectly assumed.");
            } else {
                assert!(is_plausible, "Plausibility was incorrectly not assumed.");
            }
        }

        // Scenario: Plausibility assumed after sudden altitude change
        #[when(
            regex = r"^the vehicle experiences a sudden altitude increase of (\d+) meters in the next (\d+) seconds$"
        )]
        fn vehicle_experiences_sudden_altitude_increase(world: &mut World) {
            if let Some((last_position, _)) = world.window_positions.last_mut() {
                // Simulate the sudden altitude increase by adding 50 meters to the current
                // altitude
                last_position.altitude += 50.0;
                let duration = chrono::Duration::new(10, 0).unwrap();
                last_position.gps_time += duration;
            }
        }
        // Time drift for 5 seconds.
        #[when(
            regex = r"^the system detects a time drift of (\d+) seconds between GPS time and system time$"
        )]
        fn system_detects_time_drift(world: &mut World) {
            if let Some((last_position, _)) = world.window_positions.last_mut() {
                // Assuming GPS time is ahead of system time by 5 seconds.

                let gps_time_drifted = last_position.gps_time + chrono::Duration::seconds(5);
                last_position.gps_time = gps_time_drifted;
            }
        }
        //Scenario: No plausibility assumed after excessive speed
        #[when(
            regex = r"^the vehicle accelerates to a speed of (\d+) meters per second in the next (\d+) seconds$"
        )]
        fn vehicle_accelerates_to_excessive_speed(world: &mut World) {
            if let Some((last_position, _)) = world.window_positions.last_mut() {
                last_position.speed = 10.0;
                let time_after_acceleration = last_position.gps_time + chrono::Duration::seconds(5);
                last_position.gps_time = time_after_acceleration;
            }
        }
        //Scenario: Plausibility assumed after deviation in movement pattern
        #[when(
            regex = r"^the vehicle suddenly changes direction by 90 degrees to the (north|south|east|west) in the next (\d+) seconds$"
        )]
        fn vehicle_changes_direction(world: &mut World) {
            if let Some((last_position, _)) = world.window_positions.last_mut() {
                last_position.longitude += 0.01;
                let time_after_change = last_position.gps_time + chrono::Duration::seconds(2);
                last_position.gps_time = time_after_change;
            }
        }

        info!("starting cucumber test harness");
        futures::executor::block_on(World::run("/features"));
        info!("done with cucumber test harness");
    }
}
