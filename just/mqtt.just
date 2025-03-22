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
