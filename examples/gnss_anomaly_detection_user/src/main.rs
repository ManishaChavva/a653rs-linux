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
    use a653rs::bindings::Validity;
    use a653rs_postcard::prelude::*;
    //use core::time::Duration;
    use chrono::{DateTime, Utc};

    use log::{info, warn};
    //use log::{info, warn};

    use super::detector;

    #[sampling_in(name = "position", msg_size = "1KB", refresh_period = "10s")]
    struct PositionIn;

    #[sampling_out(name = "plausibility", msg_size = "1KB")]
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

    // this process requests a data from the provider at the beginning of each
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
                latitude: f64,
                longitude: f64,
                altitude: f32,
                system_timestamp: SystemTime,
                gps_time: DateTime<Utc>,
                speed: f32,
            }

            let new_position: Position = match ctx.position_in.unwrap().recv_type() {
                Ok((Validity::Valid, position)) => position,
                Ok((Validity::Invalid, _)) => {
                    warn!("receivied invalid position, timing issue?");
                    ctx.periodic_wait().unwrap();
                    continue;
                }
                Err(e) => {
                    warn!("there was an error on deserialization of Position:\n{e:?}");
                    ctx.periodic_wait().unwrap();
                    continue;
                }
            };

            // TODO read position struct from position_request
            positions.push(new_position);

            // TODO process position struct through the anomaly detector
            let is_plausible_movement: bool = detector::is_plausible_movement(&positions);

            // TODO write the result to the user_response port
            ctx.plausibility_out
                .unwrap()
                .send_type(is_plausible_movement)
                .unwrap();

            ctx.periodic_wait().unwrap();
        }
    }
}
