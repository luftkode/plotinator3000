# Run snapshot tests
[group("Snapshot Tests"), no-exit-message]
test-snapshots *ARGS="":
    cargo {{test}} test_snapshot {{ARGS}}

# Run snapshot tests and update the snapshots
[group("Snapshot Tests"), no-exit-message]
test-snapshots-update $UPDATE_SNAPSHOT="1":
    cargo {{test}} test_snapshot

# Delete all the snapshot images
[group("Snapshot Tests"), no-exit-message]
clean-snapshots:
    rm -f tests/snapshots/*
