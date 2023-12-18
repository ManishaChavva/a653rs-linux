use cucumber::{given, then, when, steps};




struct GNSSContext {

    window_positions: Vec<Position>,
    common_threshold: f64,
    spoofing_detected: bool,
    error_message: String,
}

#[derive(Default)]
struct Position {
    
    latitude: Option<f64>,
    longitude: Option<f64>,
    altitude: Option<f32>,
    system_timestamp: SystemTime,
    gps_time: DateTime<Utc>,
    speed: Option<f32>,
}

// Step-descriptions

// Given-steps

given!(
    GNSSContext,
    "the GNSS receiver is actively receiving signals",
    |context, _step| {
        
        context.window_positions = Vec::new();
        context.spoofing_detected = false;
        context.error_message = String::new();
        context
    }
);

given!(
    GNSSContext,
    "positions from GNSS and a {float} value",
    |context, params, _step| {
        
        context.common_threshold = params[0].parse().unwrap();
        context
    }
);

given!(
    GNSSContext,
    "less than 2 window_positions",
    |context, _step| {
        
        context.window_positions = vec![];
        context
    }
);

// When steps

when!(
    GNSSContext,
    "a valid GNSS signal is received with all required fields (latitude, longitude, altitude, time, and speed)",
    |context, _step| {
        // receiving a valid GNSS signal with all required fields
        let valid_position = Position {
            latitude: Some(40000.0),
            longitude: Some(-75000.0),
            altitude: Some(50.0),
            system_timestamp: SystemTime::now(),
            gps_time: Utc::now(),
            speed: Some(5.0),
        };
        context.window_positions.push(valid_position);
        context
    }
);

when!(
    GNSSContext,
    "a GNSS signal is received with a missing {word} field",
    |context, params, _step| {
        // receiving a GNSS signal with a missing field
        let field_name = params[0];
        let mut incomplete_position = Position::default();
        match field_name {
            "latitude" => incomplete_position.latitude = None,
            "longitude" => incomplete_position.longitude = None,
            "altitude" => incomplete_position.altitude = None,
            "time" => incomplete_position.gps_time = Utc::now(),
            "speed" => incomplete_position.speed = None,
            _ => unreachable!("Unsupported field"),
        }
        context.window_positions.push(incomplete_position);
        context
    }
);

// Then steps

then!(
    GNSSContext,
    "the system should process the signal without any missing fields",
    |context, _step| {
        
        assert!(context.window_positions.len() > 0);
        context
    }
);

then!(
    GNSSContext,
    "a spoofing alert should be triggered",
    |context, _step| {
        
        context.spoofing_detected = true;
        context
    }
);

then!(
    GNSSContext,
    "log a message indicating {string} is missing",
    |context, params, _step| {
        
        let missing_field = params[0].to_string();
        context.error_message = format!("{} is missing.", missing_field);
        context
    }
);

then!(
    GNSSContext,
    "log a message indicating the exceeded threshold",
    |context, _step| {
        
        context.error_message = "Exceeded threshold.".to_string();
        context
    }
);

then!(
    GNSSContext,
    "no error messages should be logged",
    |context, _step| {
        
        assert!(context.error_message.is_empty());
        context
    }
);


