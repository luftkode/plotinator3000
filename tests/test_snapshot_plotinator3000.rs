mod util;
use plotinator_test_util::{
    bifrost_current::bifrost_current, mbed_pid_v6_regular, mbed_status_v6_regular,
};
use util::*;

#[test]
fn test_snapshot_open_app() {
    let mut harness = HarnessWrapper::new("default_app_window");
    harness.run();
    let homepage = harness.get_homepage_node();
    let main_window = homepage.parent().unwrap();
    assert_eq!(main_window.role(), Role::Window);
    assert!(main_window.is_focused());

    harness.save_snapshot();
}

#[test]
fn test_snapshot_open_mqtt_config_window_connect_disabled() {
    let mut harness = HarnessWrapper::new("default_mqtt_config_window");
    let mqtt_button = harness.get_mqtt_connect_button();
    assert!(mqtt_button.is_clickable());

    mqtt_button.click();
    harness.run();

    let mqtt_cfg_window = harness.get_mqtt_configuration_window();

    let _broker_addr_txt_input = mqtt_cfg_window
        .get_by(|n| n.role() == Role::TextInput && n.value().is_some_and(|v| v == "127.0.0.1"));

    let connect_button = mqtt_cfg_window.get_by_role_and_label(Role::Button, "Connect");
    assert!(connect_button.is_disabled());

    harness.save_snapshot();
}

#[test]
fn test_snapshot_drop_load_mbed_status_regular_v6() {
    let mut harness = HarnessWrapper::new("dropped_mbed_status_regular_v6");
    harness.drop_file(mbed_status_v6_regular());
    harness.run();
    harness.save_snapshot();
}

#[test]
fn test_snapshot_drop_load_mbed_status_pid_v6_with_cursor_on_plot_window() {
    let mut harness = HarnessWrapper::new("dropped_mbed_pid_regular_v6");
    harness.drop_file(mbed_pid_v6_regular());
    harness.run();

    // Place the cursor in the middle plot area to see that the cursor "alighment-lines" are present
    // across the plot areas
    let center_pos = harness.get_screen_rect().center();
    let left_center_pos = harness.get_screen_rect().left_center();
    let offset_right = left_center_pos.x + center_pos.x / 2.;
    let cursor_pos = Pos2::new(left_center_pos.x + offset_right, left_center_pos.y);
    harness.input_event(egui::Event::PointerMoved(cursor_pos));
    harness.step();

    // We allow a larger diff threshold because this has a lot of narrow lines, which will give rise to
    // a higher diff from GPU to GPU
    harness.save_snapshot_with_threshold(CiThreshold(62.0));
}

#[test]
fn test_snapshot_drop_load_hdf5_bifrost_current() {
    let mut harness = HarnessWrapper::new("dropped_hdf5_bifrost_current");
    harness.drop_file(bifrost_current());
    harness.run();
    // We allow a larger diff threshold because this has a lot of narrow lines, which will give rise to
    // a higher diff from GPU to GPU
    harness.save_snapshot_with_threshold(CiThreshold(10.0));
}
