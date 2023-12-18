Feature: GNSS Spoofing Detection

Scenario: Valid GNSS Signal Processing
    Given the GNSS receiver is actively receiving signals
    When a valid GNSS signal is received with all required fields (latitude, longitude, altitude, time, and speed)
    Then the system should process the signal without any missing fields
    And no error messages should be logged



Scenario Outline: GNSS Signal Plausibility Check with a Common Threshold
    Given the positions from GNSS and a <common_threshold> value
    When the distance, altitude change, gps_time_drift, or speed between two consecutive positions exceeds the <common threshold>
    Then a spoofing alert should be triggered
    And log a message indicating the exceeded threshold

Examples:
  | common_threshold |
  | 10.0             |


Scenario: length of window_position
    When less than 2 window_positions
    Then  a spoofing alert should be triggered
    And   log a message indicating "Need at least 2 positions"






