#![cfg(not(target_arch = "wasm32"))]
mod util;
use plotinator_test_util::{bifrost_current, mbed_pid_v6_regular, mbed_status_v6_regular};
use util::*;

#[test]
fn test_snapshot_open_app() {
    let mut harness = HarnessWrapper::new("default_app_window");
    harness.run();
    let homepage = harness.get_homepage_node();
    let main_window = homepage.parent().unwrap();
    assert_eq!(main_window.accesskit_node().role(), Role::Window);
    assert!(main_window.is_focused());

    harness.save_snapshot();
}

#[test]
fn test_snapshot_open_mqtt_config_window_connect_disabled() {
    let mut harness = HarnessWrapper::new("default_mqtt_config_window");
    let mqtt_button = harness.get_mqtt_connect_button();
    assert!(mqtt_button.accesskit_node().is_clickable());

    mqtt_button.click();
    harness.run();

    let mqtt_cfg_window = harness.get_mqtt_configuration_window();

    let _broker_addr_txt_input = mqtt_cfg_window
        .get_by(|n| n.role() == Role::TextInput && n.value().is_some_and(|v| v == "127.0.0.1"));

    let connect_button = mqtt_cfg_window.get_by_role_and_label(Role::Button, "Connect");
    assert!(connect_button.accesskit_node().is_disabled());

    harness.save_snapshot_with_threshold(CiThreshold(2.5));
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

    // Place the cursor in the middle plot area to see that the cursor "alignment-lines" are present
    // across the plot areas
    let center_pos = harness.get_screen_rect().center();
    let left_center_pos = harness.get_screen_rect().left_center();
    let offset_right = left_center_pos.x + center_pos.x / 2.;
    let cursor_pos = Pos2::new(left_center_pos.x + offset_right, left_center_pos.y);
    harness.input_event(egui::Event::PointerMoved(cursor_pos));
    harness.step();

    // We allow a larger diff threshold because this has a lot of narrow lines, which will give rise to
    // a higher diff from GPU to GPU
    harness.save_snapshot_with_threshold(CiThreshold(2.0));
}

#[test]
fn test_snapshot_drop_load_hdf5_bifrost_current() {
    let mut harness = HarnessWrapper::new("dropped_hdf5_bifrost_current");
    harness.drop_file(bifrost_current());
    harness.run();
    // We allow a larger diff threshold because this has a lot of narrow lines, which will give rise to
    // a higher diff from GPU to GPU
    harness.save_snapshot_with_threshold(CiThreshold(2.0));
}

#[test]
fn test_snapshot_open_loaded_files() {
    let mut harness = HarnessWrapper::new("open_loaded_files");
    harness.drop_file(mbed_status_v6_regular());
    harness.drop_file(mbed_pid_v6_regular());
    harness.run();
    // Experience shows that another two steps are required before the loaded files button is rendered
    harness.run_steps(2);

    // Check that we can now click the loaded files button
    let loaded_files_button = harness.get_loaded_files_button();
    assert!(loaded_files_button.accesskit_node().is_clickable());

    // Click and render the loaded files window
    loaded_files_button.click();
    harness.step();

    harness.save_snapshot_with_threshold(CiThreshold(6.0));
}

#[test]
fn test_snapshot_open_loaded_files_open_log_window() {
    let mut harness = HarnessWrapper::new("open_loaded_files_click_mbed_PID");
    harness.drop_file(mbed_status_v6_regular());
    harness.drop_file(mbed_pid_v6_regular());
    harness.run();
    // Experience shows that another two steps are required before the loaded files button is rendered
    harness.run_steps(2);
    // Check that we can now click the loaded files button
    let loaded_files_button = harness.get_loaded_files_button();
    assert!(loaded_files_button.accesskit_node().is_clickable());

    // Click and render the loaded files window
    loaded_files_button.click();
    harness.run_steps(2);

    // Get the Mbed PID button from the loaded logs window and click it
    let loaded_files_window = harness.get_loaded_files_window();
    let loaded_mbed_pid_button = loaded_files_window.get_by(|n| {
        n.role() == Role::Button && n.label().is_some_and(|l| l.contains("Mbed PID v6 #"))
    });

    assert!(loaded_mbed_pid_button.accesskit_node().is_clickable());

    loaded_mbed_pid_button.click();
    harness.step();

    harness.save_snapshot_with_threshold(CiThreshold(7.0));
}
