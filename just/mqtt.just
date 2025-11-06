BROKER_CONTAINER_NAME := "plotinator3000-test-broker"

# Runs the latest eclipse mosquitto docker image in detached mode
[group("MQTT")]
run-mqtt-broker: stop-remove-mqtt-broker
    docker run --name {{BROKER_CONTAINER_NAME}} --detach --network=host eclipse-mosquitto

[group("MQTT"), private]
stop-remove-mqtt-broker:
    docker stop {{BROKER_CONTAINER_NAME}} || echo "No test broker running"
    docker rm {{BROKER_CONTAINER_NAME}} || echo "No test broker container"

# Publishes random numbers within a specified range to an MQTT topic
[group("MQTT")]
pub-random TOPIC="sensors/random" MIN="0" MAX="100" REPEAT="100" DELAY="1.0":
    #!/usr/bin/env bash
    set -euo pipefail
    for ((i=1; i<={{REPEAT}}; i++)); do
        VALUE=$(({{MIN}} + $RANDOM % {{MAX}}))
        decimal=$(($RANDOM + $RANDOM + $RANDOM))
        JSON="{ \"value\": ${VALUE}.${decimal} }"
        mosquitto_pub -t "{{TOPIC}}" -m "$JSON"
        sleep {{DELAY}}
    done

[group("MQTT")]
pub-random-multiple-values TOPIC="sensors/random" MIN="0" MAX="100" REPEAT="100" DELAY="1.0":
    #!/usr/bin/env bash
    set -euo pipefail
    for ((i=1; i<={{REPEAT}}; i++)); do
        VALUE1=$(({{MIN}} + $RANDOM % {{MAX}}))
        decimal1=$(($RANDOM + $RANDOM + $RANDOM))
        VALUE2=$(({{MIN}} + $RANDOM % {{MAX}}))
        decimal2=$(($RANDOM + $RANDOM + $RANDOM))
        JSON="{ \"value1\": ${VALUE1}.${decimal1}, \"value2\": ${VALUE2}.${decimal2} }"
        mosquitto_pub -t "{{TOPIC}}" -m "$JSON"
        sleep {{DELAY}}
    done

[group("MQTT")]
pub-random-buffered-values TOPIC="sensors/random" MIN="0" MAX="100" REPEAT="100" DELAY="1.0":
    #!/usr/bin/env bash
    set -euo pipefail
    for ((i=1; i<={{REPEAT}}; i++)); do
        # Generate all values with their own timestamps
        JSON="["
        for j in {1..4}; do
            value=$(({{MIN}} + RANDOM % {{MAX}}))
            decimal=$((RANDOM + RANDOM + RANDOM))
            timestamp=$(date +%s.%N)
            JSON="${JSON}{\"value\": ${value}.${decimal}, \"timestamp\": \"${timestamp}\"},"
        done
        value=$(({{MIN}} + RANDOM % {{MAX}}))
        decimal=$((RANDOM + RANDOM + RANDOM))
        timestamp=$(date +%s.%N)
        JSON="${JSON}{\"value\": ${value}.${decimal}, \"timestamp\": \"${timestamp}\"}]"
        mosquitto_pub -t "{{TOPIC}}" -m "${JSON}"
        sleep {{DELAY}}
    done

# Simultaneously publishes random data to multiple topics for ~1 minute (depending on broker performance etc.)
[group("MQTT")]
pub-multi-random:
    just pub-random "debug/sensors/temperature" "29" "32" "10000" "0.0001" &
    just pub-random "debug/sensors/humidity" "30" "90" "200" "0.5" &
    just pub-random "debug/sensors/pressure" "980" "1030" "100" "1" &
    just pub-random-multiple-values "debug/sensors/gps" "20" "30" "1000" "0.05" &
    just pub-random-buffered-values "debug/sensors/mag" "2950" "3030" "5000" "0.01" &

[group("MQTT")]
pub-frame-altimeter MINUTES="3" TOPIC="dt/tc/frame-altimeter/1/height/10hz":
    #!/usr/bin/env bash
    set -euo pipefail
    END=$(( $(date +%s) + {{MINUTES}}*60 ))

    ALTITUDE=10
    ADD_VAL=1

    while [ $(date +%s) -lt $END ]; do

        # Gradually increase latitude by a tiny fixed amount each second
        ALTITUDE=$((ALTITUDE + ADD_VAL))    # roughly 0.00001 degrees per second
        # Convert back to float strings with dot
        altitude_str=$(printf "%d.%03d" $((ALTITUDE)) $((ALTITUDE % 10)))
        mosquitto_pub -h localhost -t "{{TOPIC}}" -m "${altitude_str}"
        sleep 0.1
    done

[group("MQTT")]
pub-frame-gps HOST="localhost" MINUTES="3" TOPIC="dt/tc/frame-gps/x/gps" GPS_NUM="1" LAT_ADD="100":
    #!/usr/bin/env bash
    set -euo pipefail

    LAT_INT=56217778    # 56.217778 * 1e6
    LON_INT=10147778    # 10.147778 * 1e6
    ALT=66.62
    SPEED_INT=52        # 0.052 * 1000
    GPS_NR={{GPS_NUM}}
    MODE=3
    SATELLITES=10

    END=$(( $(date +%s) + {{MINUTES}}*60 ))

    while [ $(date +%s) -lt $END ]; do
        TIMESTAMP=$(date +%s%N)
        GPS_TIME=$(date -u +"%Y-%m-%dT%H:%M:%S.000Z")

        # Gradually increase latitude by a tiny fixed amount each second
        LAT_INT=$((LAT_INT + {{LAT_ADD}}))    # roughly 0.00001 degrees per second
        # Small random drift for longitude
        LON_INT=$((LON_INT + RANDOM % 10 - 5))
        SPEED_INT=$((SPEED_INT + RANDOM % 5 + 2))
        # Convert back to float strings with dot
        LAT=$(printf "%d.%06d" $((LAT_INT / 1000000)) $((LAT_INT % 1000000)))
        LON=$(printf "%d.%06d" $((LON_INT / 1000000)) $((LON_INT % 1000000)))
        SPEED=$(printf "30.%03d" $SPEED_INT)

        JSON="{\"gps_nr\": $GPS_NR, \"timestamp\": $TIMESTAMP, \"mode\": $MODE, \"gps_status\": {\"hdop\": 0.8, \"vdop\": 1.5, \"pdop\": 1.7, \"satellites\": $SATELLITES}, \"position\": {\"lat\": $LAT, \"lon\": $LON, \"alt\": $ALT}, \"speed\": $SPEED, \"gps_time\": \"$GPS_TIME\"}"

        mosquitto_pub -h {{HOST}} -t "{{TOPIC}}" -m "$JSON"
        sleep 1
    done

[group("MQTT")]
pub-frame-gps-circle MINUTES="3" TOPIC="dt/tc/frame-gps/x/gps" GPS_NUM="1" RADIUS_METERS="200":
    #!/usr/bin/env bash
    set -euo pipefail

    # Starting position (center of circle)
    CENTER_LAT_INT=56217778  # 56.217778 * 1e6
    CENTER_LON_INT=10147778  # 10.147778 * 1e6
    ALT=66.62
    SPEED_INT=52  # 0.052 * 1000
    GPS_NR={{GPS_NUM}}
    MODE=3
    SATELLITES=10

    # Circle parameters
    RADIUS_METERS={{RADIUS_METERS}}
    SECONDS_PER_ROUND=60
    SLEEP_INTERVAL=1

    # Conversion factors (approximate at latitude ~56°)
    # 1 degree latitude ≈ 111,320 meters
    # 1 degree longitude ≈ 111,320 * cos(latitude) ≈ 62,000 meters at 56°N
    METERS_PER_LAT_DEGREE=111320
    METERS_PER_LON_DEGREE=62000

    RADIUS_MULTIPLIER=100
    STEP=0

    END=$(( $(date +%s) + {{MINUTES}}*60 ))

    while [ $(date +%s) -lt $END ]; do
        TIMESTAMP=$(date +%s%N)
        GPS_TIME=$(date -u +"%Y-%m-%dT%H:%M:%S.000Z")

        # Calculate angle in radians (0 to 2*PI over 60 seconds)
        # angle = (step / 60) * 2 * PI
        ANGLE=$(echo "scale=10; ($STEP / $SECONDS_PER_ROUND) * 2 * 3.14159265359" | bc)

        SIN=$(echo "scale=10; s($ANGLE)" | bc -l)
        COS=$(echo "scale=10; c($ANGLE)" | bc -l)

        # Calculate offsets in meters, then convert to degrees
        LAT_OFFSET_METERS=$(echo "scale=10; $RADIUS_METERS * $COS * $RADIUS_MULTIPLIER / 100" | bc)
        LON_OFFSET_METERS=$(echo "scale=10; $RADIUS_METERS * $SIN * $RADIUS_MULTIPLIER / 100" | bc)

        # Convert to microdegrees (integer)
        LAT_OFFSET=$(echo "scale=0; $LAT_OFFSET_METERS * 1000000 / $METERS_PER_LAT_DEGREE" | bc)
        LON_OFFSET=$(echo "scale=0; $LON_OFFSET_METERS * 1000000 / $METERS_PER_LON_DEGREE" | bc)

        # Calculate current position
        LAT_INT=$((CENTER_LAT_INT + LAT_OFFSET))
        LON_INT=$((CENTER_LON_INT + LON_OFFSET))

        # Add small random jitter
        LAT_INT=$((LAT_INT + RANDOM % 10 - 5))
        LON_INT=$((LON_INT + RANDOM % 10 - 5))

        # Update speed with variation
        SPEED_INT=$((SPEED_INT + RANDOM % 5 + 2))

        # Convert back to float strings with dot
        LAT=$(printf "%d.%06d" $((LAT_INT / 1000000)) $((LAT_INT % 1000000)))
        LON=$(printf "%d.%06d" $((LON_INT / 1000000)) $((LON_INT % 1000000)))
        SPEED=$(printf "30.%03d" $SPEED_INT)

        JSON="{\"gps_nr\": $GPS_NR, \"timestamp\": $TIMESTAMP, \"mode\": $MODE, \"gps_status\": {\"hdop\": 0.8, \"vdop\": 1.5, \"pdop\": 1.7, \"satellites\": $SATELLITES}, \"position\": {\"lat\": $LAT, \"lon\": $LON, \"alt\": $ALT}, \"speed\": $SPEED, \"gps_time\": \"$GPS_TIME\"}"

        mosquitto_pub -h localhost -t "{{TOPIC}}" -m "$JSON"

        # Advance step by sleep interval
        STEP=$((STEP + SLEEP_INTERVAL))

        # Check if we completed a full circle
        if [ $STEP -ge $SECONDS_PER_ROUND ]; then
            STEP=0
            # Randomly adjust radius for next round: ±20%
            RADIUS_CHANGE=$((RANDOM % 41 - 20))  # -20 to +20
            RADIUS_MULTIPLIER=$((RADIUS_MULTIPLIER + RADIUS_CHANGE))
            [ $RADIUS_MULTIPLIER -lt 50 ] && RADIUS_MULTIPLIER=50
            [ $RADIUS_MULTIPLIER -gt 150 ] && RADIUS_MULTIPLIER=150
        fi

        sleep $SLEEP_INTERVAL
    done
