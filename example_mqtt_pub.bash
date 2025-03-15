#!/usr/bin/env bash

# Configuration
MQTT_BROKER="localhost"
MQTT_PORT="1883"
MQTT_BASE_TOPIC="system/$(hostname)"
INTERVAL=2

# Function to get CPU usage directly from /proc/stat
get_cpu_usage() {
    awk '{print $1}' /proc/loadavg
}

get_free_memory() {
    free | grep Mem | awk '{print $3}'
}

# Check if mosquitto_pub is installed
if ! command -v mosquitto_pub &> /dev/null; then
    echo "Error: mosquitto_pub is not installed. Please install mosquitto-clients package."
    exit 1
fi

# Main loop
echo "Starting system monitoring using /proc..."
echo "Publishing to $MQTT_BROKER:$MQTT_PORT under $MQTT_BASE_TOPIC/"
echo "Press Ctrl+C to stop"

while true; do
    # Get metrics and publish to specific topics
    cpu=$(get_cpu_usage)
    memory=$(get_free_memory)

    # Publish each metric
    mosquitto_pub -h "$MQTT_BROKER" -p "$MQTT_PORT" -t "$MQTT_BASE_TOPIC/cpu/usage/percent" -m "$cpu"
    mosquitto_pub -h "$MQTT_BROKER" -p "$MQTT_PORT" -t "$MQTT_BASE_TOPIC/memory/free" -m "$memory"

    # Log to console
    echo "CPU Usage (1m avg): $cpu% | Free memory: $memory"

    sleep $INTERVAL
done
