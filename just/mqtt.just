BROKER_CONTAINER_NAME := "plotinator3000-test-broker"

# Runs the latest eclipse mosquitto docker image in detached mode
[group("MQTT")]
run-mqtt-broker: stop-remove-mqtt-broker
    docker run --name {{BROKER_CONTAINER_NAME}} --detach --network=host eclipse-mosquitto

[group("MQTT"), private]
stop-remove-mqtt-broker:
    docker stop {{BROKER_CONTAINER_NAME}} || echo "No test broker running"
    docker rm {{BROKER_CONTAINER_NAME}} || echo "No test broker container"

# Publish a msg to a topic repeadtedly with a set delay
[group("MQTT")]
pub-mqtt TOPIC="dt/speed" MSG='{ \"speed\": 12.4951 }' REPEAT="100000" REPEAT_DELAY="0.1":
    mosquitto_pub -t "{{TOPIC}}" -m "{{MSG}}" --repeat {{REPEAT}} --repeat-delay {{REPEAT_DELAY}}

# Publishes random numbers within a specified range to an MQTT topic
[group("MQTT")]
pub-random TOPIC="sensors/random" MIN="0" MAX="100" REPEAT="100" DELAY="1.0":
    #!/usr/bin/env bash
    set -euo pipefail
    for ((i=1; i<={{REPEAT}}; i++)); do
        VALUE=$(({{MIN}} + $RANDOM % {{MAX}}))
        decimal=$(($RANDOM + $RANDOM + $RANDOM))
        JSON="{ \"value\": ${VALUE} }"
        mosquitto_pub -t "{{TOPIC}}" -m "$JSON"
        sleep {{DELAY}}
    done

# Simultaneously publishes random data to multiple topics for ~1 minute (depending on broker performance etc.)
[group("MQTT")]
pub-multi-random:
    just pub-random "debug/sensors/temperature" "29" "32" "10000" "0.0001" &
    just pub-random "debug/sensors/mag" "1980" "3030" "5000" "0.01" &
    just pub-random "debug/sensors/humidity" "30" "90" "200" "0.5" &
    just pub-random "debug/sensors/pressure" "980" "1030" "100" "1" &
