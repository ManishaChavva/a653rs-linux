Feature: GNSS Spoofing Detection

  Scenario: No plausibility assumed after legitimate movement
    Given the position is at latitude: 55.671315, longitude: 12.521528333, altitude: 9.2, time: "2032-06-20T13:49:00:00Z", speed: 0.684 
    When the vehicle moves 1km south in the next 3 seconds
    Then no plausibility is assumed 

  Scenario: Plausibility assumed after sudden altitude change
    Given the position is at latitude: 40.7128, longitude: -74.0060, altitude: 10, time: "2032-06-20T14:00:00:00Z", speed: 0.5
    When the vehicle experiences a sudden altitude increase of 50 meters in the next 10 seconds
    Then plausibility is assumed

  Scenario: Plausibility assumed after time drift detected
    Given the position is at latitude: 37.7749, longitude: -122.4194, altitude: 5, time: "2032-06-20T14:10:00:00Z", speed: 1.0
    When the system detects a time drift of 5 seconds between GPS time and system time
    Then plausibility is assumed

  Scenario: No plausibility assumed after excessive speed
    Given the position is at latitude: 34.0522, longitude: -118.2437, altitude: 5, time: "2032-06-20T14:20:00:00Z", speed: 2.5
    When the vehicle accelerates to a speed of 10 meters per second in the next 5 seconds
    Then no plausibility is assumed 

  Scenario: Plausibility assumed after deviation in movement pattern
    Given the position is at latitude: 51.5074, longitude: -0.1278, altitude: 9, time: "2032-06-20T14:30:00:00Z", speed: 1.2
    When the vehicle suddenly changes direction by 90 degrees to the east in the next 2 seconds
    Then plausibility is assumed

  Scenario: No plausibility assumed after missing GPS data
    Given the position data is incomplete or missing
    When the system encounters missing or incomplete GPS data
    Then no plausibility is assumed
