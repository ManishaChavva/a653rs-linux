use a653rs::bindings::Validity;
use a653rs::partition;
use a653rs::prelude::PartitionExt;
use a653rs_linux::partition::ApexLogger;
use log::LevelFilter;

#[macro_use]
extern crate log;

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

        /*loop {
            info!("forwarding request as response ");

            // allocate a buffer to receive into
            let mut buf = [0u8; 32];

            // receive a request, storing it to `buf`
            ctx.position_request.unwrap().receive(&mut buf).unwrap();

            // `ctx.get_time()` returns a [SystemTime], which might be `Infinite`, or just a
            // normal time. Thus we have to check that indeed a normal time was returned.
            let SystemTime::Normal(time) = ctx.get_time() else {
                panic!("could not read time");
            };

            // convert the current time to an u128 integer representing nanoseconds, and
            // serialize the integer to a byte array
            let time_in_nanoseconds = time.as_nanos();
            buf[16..32].copy_from_slice(&time_in_nanoseconds.to_le_bytes());

            // send the contents of `buf` back as response
            ctx.position_response.unwrap().send(&buf).unwrap();

            // wait until the next partition window / MiF
            ctx.periodic_wait().unwrap();
        }*/
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
        }

        impl Default for World {
            fn default() -> Self {
                Self {
                    user: None,
                    capacity: 0,
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

        info!("test 0");
        println!("test 1");

        futures::executor::block_on(World::run("/features"));
    }
}