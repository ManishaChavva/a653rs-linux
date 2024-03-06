use a653rs::partition;
use a653rs::prelude::PartitionExt;
use a653rs_linux::partition::ApexLogger;
use log::LevelFilter;

mod detector;

fn main() {
    ApexLogger::install_panic_hook();
    ApexLogger::install_logger(LevelFilter::Trace).unwrap();

    anomaly_detection_user::Partition.run()
}

#[partition(a653rs_linux::partition::ApexLinuxPartition)]
mod anomaly_detection_user {
    use crate::detector::Position;
    use a653rs_postcard::prelude::*;
    use core::time::Duration;
    use log::{info, warn};

    use super::detector;

    #[sampling_in(name = "position", msg_size = "16B",refresh_period = "10s")]
    struct PositionIn;

    #[sampling_out(name = "plausibility", msg_size = "32B")]
    struct PlausibilityOut;

    #[start(cold)]
    fn cold_start(mut ctx: start::Context) {
        // initialize both sampling ports
        ctx.create_position_in().unwrap();
        ctx.create_plausibility_out().unwrap();

        // create and start a periodic process
        ctx.create_periodic_anomaly_detection_user()
            .unwrap()
            .start()
            .unwrap();
    }

    // do the same as a cold_start
    #[start(warm)]
    fn warm_start(ctx: start::Context) {
        cold_start(ctx);
    }

    // this process requests a data from the gpsd at the beginning of each
    // partition window / MiF
    #[periodic(
        period = "0ms",
        time_capacity = "Infinite",
        stack_size = "8KB",
        base_priority = 1,
        deadline = "Soft"
    )]
    fn periodic_anomaly_detection_user(ctx: periodic_anomaly_detection_user::Context) {
        info!("started periodic_anomaly_detection_user process");
        let mut positions = Vec::new();

        loop {
            log::info!("entering loop body");

            #[derive(serde::Deserialize, serde::Serialize)]
            struct MyDataStruct {
                x: f32,
                y: u128,
            }

            let my_struct = MyDataStruct { x: 0.3, y: 12 };

            let (validity,position): (_, Position) = ctx.position_in.unwrap().recv_type().unwrap();

            // TODO read position struct from position_request
            let Ok((validity, new_position)): Result<(_, Position), _> = ctx
            .position_in.unwrap().recv_type() else {
                log::warn!("there was an error on deserialization of Position:");
                continue;
            };
            positions.push(new_position);

            if validity == Validity::Invalid {
                log::warn!("received outdated data");
                ctx.periodic_wait().unwrap();
                continue;
            }

            // TODO process position struct through the anomaly detector
            let is_plausible_movement = detector::is_plausible_movement(&positions);

            let result = is_plausible_movement;

            // TODO write the result to the user_response port

            ctx.plausibility_out.unwrap().send_type(&is_plausible_movement).unwrap(); 

            ctx.periodic_wait().unwrap();
        }

        /*

        // a periodic process does not actually return at the end of a partition window,
        // it just pauses itself once it is done with the work from the current MiF
        // see below at the `ctx.periodic_wait().unwrap()` call.
        loop {
            // first, send a request:

            // `ctx.get_time()` returns a [SystemTime], which might be `Infinite`, or just a
            // normal time. Thus we have to check that indeed a normal time was returned.
            let SystemTime::Normal(time) = ctx.get_time() else {
                panic!("could not read time");
            };
            info!("sending a request");

            // convert the current time to an u128 integer representing nanoseconds, and
            // serialize the integer to a byte array
            let time_in_nanoseconds = time.as_nanos();
            let buf = time_in_nanoseconds.to_le_bytes();

            // finally send the byte array to the ping_request port
            ctx.ping_request.unwrap().send(&buf).unwrap();

            // then receive a response, if any:

            // allocate a buffer on the stack for receival of the response
            let mut buf = [0u8; 32];

            // sample the user_response sampling port into `buf`
            // - validity indicates whether data received was sitting in the samplin port
            //   for no more than the refresh_period
            // - `bytes` is a subslice of `buf`, containing only the bytes actually read
            //   from the sampling port
            let (validity, bytes) = ctx.user_response.unwrap().receive(&mut buf).unwrap();

            // only if the message is valid and has the expected length try to process it
            if validity == Validity::Valid && bytes.len() == 32 {
                // deserialize the bytes into an u128
                let request_timestamp = u128::from_le_bytes(bytes[0..16].try_into().unwrap());
                let response_timestamp = u128::from_le_bytes(bytes[16..32].try_into().unwrap());
                // the difference is the time passed since sending the request for this response
                let round_trip = time_in_nanoseconds - request_timestamp;
                let req_to_gpsd = response_timestamp - request_timestamp;
                let resp_to_anomaly_user = time_in_nanoseconds - response_timestamp;

                // convert the integers of nanoseconds back to a [Duration]s for nicer logging
                let req_sent_to_resp_recv = Duration::from_nanos(round_trip as u64);
                let req_sent_to_resp_sent = Duration::from_nanos(req_to_gpsd as u64);
                let resp_sent_to_resp_recv = Duration::from_nanos(resp_to_anomaly_user as u64);

                // and log the results!
                info!("received valid response:\n\tround-trip {req_sent_to_resp_recv:?}\n\treq-to-gpsd {req_sent_to_resp_sent:?}\n\tresp-to-anomaly_user{resp_sent_to_resp_recv:?}");
            } else {
                warn!("response seems to be incomplete: {validity:?}, {bytes:?}");
            }

            // wait until the beginning of this partitions next MiF. In scheduling terms
            // this function would probably be called `yield()`.
            ctx.periodic_wait().unwrap();
        }
        */
    }
}
