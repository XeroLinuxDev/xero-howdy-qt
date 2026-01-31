#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qlist.h");
        type QList_QString = cxx_qt_lib::QList<cxx_qt_lib::QString>;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QList_QString, face_models)]
        #[qproperty(QString, status_message)]
        #[qproperty(bool, howdy_enabled)]
        #[qproperty(bool, device_supported)]
        type HowdyBackend = super::HowdyBackendRust;

        /// Check if device has supported IR camera
        #[qinvokable]
        fn check_device(self: Pin<&mut HowdyBackend>);

        /// Refresh the list of face models
        #[qinvokable]
        fn refresh_models(self: Pin<&mut HowdyBackend>);

        /// Add a new face model with the given name
        #[qinvokable]
        fn add_model(self: Pin<&mut HowdyBackend>, name: QString);

        /// Remove a face model by index
        #[qinvokable]
        fn remove_model(self: Pin<&mut HowdyBackend>, index: i32);

        /// Toggle howdy enabled/disabled
        #[qinvokable]
        fn toggle_enabled(self: Pin<&mut HowdyBackend>);

        /// Run a test of face recognition
        #[qinvokable]
        fn run_test(self: Pin<&mut HowdyBackend>);
    }
}

use core::pin::Pin;
use cxx_qt_lib::{QList, QString};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Default)]
pub struct HowdyBackendRust {
    face_models: QList<QString>,
    status_message: QString,
    howdy_enabled: bool,
    device_supported: bool,
}

/// Check if howdy is disabled in config content
fn is_howdy_disabled(config_content: &str) -> bool {
    for line in config_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("disabled") {
            if let Some(value) = trimmed.split('=').nth(1) {
                let val = value.trim();
                return val.eq_ignore_ascii_case("true") || val == "1";
            }
        }
    }
    false // Default: not disabled (enabled)
}

impl qobject::HowdyBackend {
    /// Check if device has a supported IR camera for howdy
    pub fn check_device(mut self: Pin<&mut Self>) {
        // Check multiple conditions for device support:
        // 1. Check if howdy is installed
        // 2. Use v4l2-ctl to detect video devices (per Arch Wiki)
        // 3. Check howdy config for device path
        // 4. Prefer stable paths (/dev/v4l/by-path/) over /dev/videoX

        let mut supported = false;
        let mut status_msg = String::new();

        // Check if howdy command exists
        let howdy_exists = Command::new("which")
            .arg("howdy")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !howdy_exists {
            status_msg = "Howdy is not installed".to_string();
            self.as_mut().set_device_supported(false);
            self.as_mut().set_status_message(QString::from(&status_msg));
            return;
        }

        // Use v4l2-ctl to list devices (recommended by Arch Wiki)
        let v4l2_output = Command::new("v4l2-ctl")
            .arg("--list-devices")
            .output();

        let has_video_devices = match &v4l2_output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                !stdout.trim().is_empty()
            }
            _ => {
                // Fallback: check /dev/v4l/by-path/ for stable device paths (per Arch Wiki)
                Path::new("/dev/v4l/by-path").exists()
                    && fs::read_dir("/dev/v4l/by-path")
                        .map(|entries| entries.count() > 0)
                        .unwrap_or(false)
            }
        };

        if !has_video_devices {
            status_msg = "No video devices found (install v4l-utils for better detection)".to_string();
            self.as_mut().set_device_supported(false);
            self.as_mut().set_status_message(QString::from(&status_msg));
            return;
        }

        // Check howdy config file for device_path
        // Priority order per Arch Wiki: /lib/security/howdy/config.ini is the main config
        let config_paths = [
            "/lib/security/howdy/config.ini",
            "/usr/lib/security/howdy/config.ini",
            "/etc/howdy/config.ini",
        ];

        let mut device_path_configured = false;
        for config_path in &config_paths {
            if Path::new(config_path).exists() {
                if let Ok(content) = fs::read_to_string(config_path) {
                    // Check if device_path is set and not "none"
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("device_path") {
                            if let Some(value) = trimmed.split('=').nth(1) {
                                let device = value.trim();
                                if !device.is_empty() && device != "none" && device != "null" {
                                    // Check if the configured device actually exists
                                    // Supports both /dev/videoX and stable /dev/v4l/by-path/ paths
                                    if Path::new(device).exists() {
                                        device_path_configured = true;
                                        supported = true;
                                        // Show friendly path info
                                        if device.contains("/by-path/") || device.contains("/by-id/") {
                                            status_msg = format!("Device configured (stable path): {}", device);
                                        } else {
                                            status_msg = format!("Device found: {}", device);
                                        }
                                    } else {
                                        status_msg = format!("Configured device {} not found", device);
                                    }
                                }
                            }
                            break;
                        }
                    }

                    // Also check if howdy is disabled in config
                    let howdy_enabled = !is_howdy_disabled(&content);
                    self.as_mut().set_howdy_enabled(howdy_enabled);
                }
                break;
            }
        }

        if !device_path_configured && has_video_devices {
            // Devices exist but may not be configured
            supported = true;
            status_msg = "Video device(s) found - run 'sudo howdy config' to set device_path".to_string();
        }

        self.as_mut().set_device_supported(supported);
        self.as_mut().set_status_message(QString::from(&status_msg));
    }

    /// Refresh the list of face models from howdy
    pub fn refresh_models(mut self: Pin<&mut Self>) {
        let output = Command::new("pkexec")
            .args(["howdy", "list"])
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let mut models = QList::<QString>::default();

                // Check for "No users registered" message (howdy outputs this when empty)
                let combined = format!("{}{}", stdout, stderr).to_lowercase();
                if combined.contains("no users") || combined.contains("no models") {
                    self.as_mut().set_face_models(models);
                    self.as_mut().set_status_message(QString::from("No faces registered"));
                    return;
                }

                // Parse howdy list output - each line after header is a model
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    // Skip empty lines, header lines, and separator lines
                    if trimmed.is_empty()
                        || trimmed.starts_with("ID")
                        || trimmed.contains("──")
                        || trimmed.contains("---")
                    {
                        continue;
                    }
                    // Parse lines like: "0  model_name  2024-01-15"
                    if let Some(first_char) = trimmed.chars().next() {
                        if first_char.is_ascii_digit() {
                            models.append_clone(&QString::from(trimmed));
                        }
                    }
                }

                let count = models.len();
                self.as_mut().set_face_models(models);
                if count > 0 {
                    self.as_mut().set_status_message(QString::from(&format!(
                        "Found {} registered face(s)",
                        count
                    )));
                } else {
                    self.as_mut().set_status_message(QString::from("No faces registered"));
                }
            }
            Err(e) => {
                self.as_mut().set_status_message(QString::from(&format!("Error: {}", e)));
            }
        }
    }

    /// Add a new face model
    pub fn add_model(mut self: Pin<&mut Self>, name: QString) {
        let name_str = name.to_string();
        if name_str.is_empty() {
            self.as_mut().set_status_message(QString::from("Please enter a model name"));
            return;
        }

        self.as_mut().set_status_message(QString::from("Adding model... Look at the camera"));

        // Note: This runs in blocking mode. For better UX, this should be async
        let output = Command::new("pkexec")
            .args(["howdy", "add", "-y", &name_str])
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    self.as_mut().set_status_message(QString::from("Model added successfully"));
                    self.refresh_models();
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    self.as_mut().set_status_message(QString::from(&format!("Failed: {}", stderr)));
                }
            }
            Err(e) => {
                self.as_mut().set_status_message(QString::from(&format!("Error: {}", e)));
            }
        }
    }

    /// Remove a face model by its ID
    pub fn remove_model(mut self: Pin<&mut Self>, index: i32) {
        let output = Command::new("pkexec")
            .args(["howdy", "remove", "-y", &index.to_string()])
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    self.as_mut().set_status_message(QString::from("Model removed"));
                    self.refresh_models();
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    self.as_mut().set_status_message(QString::from(&format!("Failed: {}", stderr)));
                }
            }
            Err(e) => {
                self.as_mut().set_status_message(QString::from(&format!("Error: {}", e)));
            }
        }
    }

    /// Toggle howdy enabled/disabled
    pub fn toggle_enabled(mut self: Pin<&mut Self>) {
        let current = *self.as_ref().howdy_enabled();
        let arg = if current { "1" } else { "0" };

        let output = Command::new("pkexec")
            .args(["howdy", "disable", arg])
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    self.as_mut().set_howdy_enabled(!current);
                    let msg = if current { "Howdy disabled" } else { "Howdy enabled" };
                    self.as_mut().set_status_message(QString::from(msg));
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    self.as_mut().set_status_message(QString::from(&format!("Failed: {}", stderr)));
                }
            }
            Err(e) => {
                self.as_mut().set_status_message(QString::from(&format!("Error: {}", e)));
            }
        }
    }

    /// Run face recognition test
    pub fn run_test(mut self: Pin<&mut Self>) {
        self.as_mut().set_status_message(QString::from("Running test... Look at the camera"));

        let output = Command::new("pkexec")
            .args(["howdy", "test"])
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let result = if out.status.success() {
                    format!("Test complete: {}", stdout.lines().last().unwrap_or("Success"))
                } else {
                    format!("Test failed: {}", String::from_utf8_lossy(&out.stderr))
                };
                self.as_mut().set_status_message(QString::from(&result));
            }
            Err(e) => {
                self.as_mut().set_status_message(QString::from(&format!("Error: {}", e)));
            }
        }
    }
}
