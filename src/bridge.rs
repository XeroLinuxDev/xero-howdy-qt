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
        #[qproperty(QList_QString, video_devices)]
        #[qproperty(QString, status_message)]
        #[qproperty(bool, howdy_enabled)]
        #[qproperty(bool, device_supported)]
        #[qproperty(bool, camera_configured)]
        #[qproperty(bool, pam_sddm)]
        #[qproperty(bool, pam_kde)]
        #[qproperty(bool, pam_sudo)]
        #[qproperty(bool, pam_system_login)]
        #[qproperty(bool, pam_plasma_lm)]
        #[qproperty(bool, pam_polkit)]
        #[qproperty(bool, sddm_installed)]
        #[qproperty(bool, plasma_lm_installed)]
        #[qproperty(QString, app_version)]
        type HowdyBackend = super::HowdyBackendRust;

        /// Check if device has supported IR camera
        #[qinvokable]
        fn check_device(self: Pin<&mut HowdyBackend>);

        /// Refresh the list of face models
        #[qinvokable]
        fn refresh_models(self: Pin<&mut HowdyBackend>);

        /// Add a new face model with the given name (async)
        #[qinvokable]
        fn add_model(self: Pin<&mut HowdyBackend>, name: QString);

        /// Check for pending add_model result from the background thread
        #[qinvokable]
        fn check_add_result(self: Pin<&mut HowdyBackend>) -> bool;

        /// Discard the just-registered face (delete it)
        #[qinvokable]
        fn discard_face(self: Pin<&mut HowdyBackend>);

        /// Remove a face model by index
        #[qinvokable]
        fn remove_model(self: Pin<&mut HowdyBackend>, index: i32);

        /// Toggle howdy enabled/disabled
        #[qinvokable]
        fn toggle_enabled(self: Pin<&mut HowdyBackend>);

        /// Run a test of face recognition
        #[qinvokable]
        fn run_test(self: Pin<&mut HowdyBackend>);

        /// Scan /dev/ for video* devices and populate video_devices
        #[qinvokable]
        fn load_video_devices(self: Pin<&mut HowdyBackend>);

        /// Preview a device with mpv
        #[qinvokable]
        fn test_camera(self: Pin<&mut HowdyBackend>, device: QString);

        /// Write device_path into the howdy config file
        #[qinvokable]
        fn save_device_path(self: Pin<&mut HowdyBackend>, device: QString);

        /// Read all PAM files and update the pam_* properties
        #[qinvokable]
        fn check_pam_status(self: Pin<&mut HowdyBackend>);

        /// Add, uncomment or comment the howdy line in the given PAM file
        #[qinvokable]
        fn toggle_pam(self: Pin<&mut HowdyBackend>, file: QString);

        /// Detect which display managers (SDDM / plasma-login-manager) are installed
        #[qinvokable]
        fn detect_display_managers(self: Pin<&mut HowdyBackend>);
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
    video_devices: QList<QString>,
    status_message: QString,
    howdy_enabled: bool,
    device_supported: bool,
    camera_configured: bool,
    pam_sddm: bool,
    pam_kde: bool,
    pam_sudo: bool,
    pam_system_login: bool,
    pam_plasma_lm: bool,
    pam_polkit: bool,
    sddm_installed: bool,
    plasma_lm_installed: bool,
    app_version: QString,
}

const PAM_LINE: &str = "auth sufficient pam_howdy.so";
const PAM_SDDM: &str = "/etc/pam.d/sddm";
const PAM_KDE: &str = "/etc/pam.d/kde";
const PAM_SUDO: &str = "/etc/pam.d/sudo";
const PAM_SYSTEM_LOGIN: &str = "/etc/pam.d/system-local-login";
const PAM_PLASMA_LM: &str = "/etc/pam.d/plasmalogin";
const PAM_POLKIT: &str = "/etc/pam.d/polkit-1";
// polkit-agent-helper resolves PAM modules in a restricted sandbox; use the full path.
const PAM_POLKIT_LINE: &str = "auth sufficient /lib/security/pam_howdy.so";

/// Returns true if a non-commented howdy auth line is present in the PAM file content
fn pam_howdy_active(content: &str) -> bool {
    content.lines().any(|line| {
        let t = line.trim();
        !t.starts_with('#') && (t.contains("pam_howdy.so") || t.contains("howdy/pam.py"))
    })
}

fn pkexec_run(program: &str, args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("pkexec").arg(program).args(args).output()
}

/// Resolve the absolute path to the howdy binary
fn find_howdy() -> Option<String> {
    Command::new("which")
        .arg("howdy")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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

/// Returns true if the given package is installed (pacman -Q)
fn pacman_installed(pkg: &str) -> bool {
    Command::new("pacman")
        .args(["-Q", pkg])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

impl qobject::HowdyBackend {
    /// Detect which display managers are present and set the corresponding properties.
    /// Falls back to showing SDDM if neither is found.
    pub fn detect_display_managers(mut self: Pin<&mut Self>) {
        let version = env!("APP_VERSION").to_string();
        self.as_mut().set_app_version(QString::from(&version));

        let sddm = Path::new("/usr/bin/sddm").exists() || pacman_installed("sddm");
        let plm = Path::new("/usr/bin/plasma-login-manager").exists()
            || pacman_installed("plasma-login-manager");

        self.as_mut().set_sddm_installed(sddm);
        self.as_mut().set_plasma_lm_installed(plm);

        // Neither detected → fall back to showing SDDM (most common on Arch)
        if !sddm && !plm {
            self.as_mut().set_sddm_installed(true);
        }
    }

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
        let v4l2_output = Command::new("v4l2-ctl").arg("--list-devices").output();

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
            status_msg =
                "No video devices found (install v4l-utils for better detection)".to_string();
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
                                        if device.contains("/by-path/")
                                            || device.contains("/by-id/")
                                        {
                                            status_msg = format!(
                                                "Device configured (stable path): {}",
                                                device
                                            );
                                        } else {
                                            status_msg = format!("Device found: {}", device);
                                        }
                                    } else {
                                        status_msg =
                                            format!("Configured device {} not found", device);
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
            status_msg =
                "Video device(s) found - run 'sudo howdy config' to set device_path".to_string();
        }

        self.as_mut().set_device_supported(supported);
        self.as_mut().set_status_message(QString::from(&status_msg));
    }

    /// Refresh the list of face models from howdy
    pub fn refresh_models(mut self: Pin<&mut Self>) {
        let howdy = match find_howdy() {
            Some(p) => p,
            None => {
                self.as_mut()
                    .set_status_message(QString::from("Howdy is not installed"));
                return;
            }
        };
        // Try without elevated privileges first; the models dir is often user-readable.
        // Only fall back to pkexec if the unprivileged call fails — this avoids an
        // extra password prompt on every startup / after every add/remove.
        let output = Command::new(&howdy)
            .args(["list"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .or_else(|| Command::new("pkexec").args([&howdy, "list"]).output().ok());

        match output {
            Some(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let mut models = QList::<QString>::default();

                // Check for "No users registered" message (howdy outputs this when empty)
                let combined = format!("{}{}", stdout, stderr).to_lowercase();
                if combined.contains("no users") || combined.contains("no models") {
                    self.as_mut().set_face_models(models);
                    self.as_mut()
                        .set_status_message(QString::from("No faces registered"));
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
                    self.as_mut()
                        .set_status_message(QString::from("No faces registered"));
                }
            }
            None => {
                self.as_mut()
                    .set_status_message(QString::from("Failed to list models"));
            }
        }
    }

    /// Add a new face model — runs howdy asynchronously
    pub fn add_model(mut self: Pin<&mut Self>, name: QString) {
        let name_str = name.to_string();
        if name_str.is_empty() {
            self.as_mut()
                .set_status_message(QString::from("Failed: Please enter a model name"));
            return;
        }

        let howdy = match find_howdy() {
            Some(p) => p,
            None => {
                self.as_mut()
                    .set_status_message(QString::from("Failed: Howdy is not installed"));
                return;
            }
        };

        // Set initial status - this tells QML capture has started
        self.as_mut()
            .set_status_message(QString::from("Look at the camera..."));

        // Spawn a thread to run the blocking pkexec call
        let howdy_clone = howdy.clone();
        let name_clone = name_str.clone();

        std::thread::spawn(move || {
            let output = pkexec_run(&howdy_clone, &["add", "-y", &name_clone]);

            let result_msg = match output {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let face_id = stdout
                        .lines()
                        .find(|l| l.trim().starts_with("Face added as "))
                        .and_then(|l| {
                            l.trim()
                                .split_whitespace()
                                .last()
                                .and_then(|s| s.parse::<i32>().ok())
                        });
                    if let Some(id) = face_id {
                        let _ = std::fs::write("/tmp/howdy_new_face_id.txt", id.to_string());
                    }
                    "Face registered successfully".to_string()
                }
                Ok(out) => {
                    let combined = format!(
                        "{}{}",
                        String::from_utf8_lossy(&out.stdout),
                        String::from_utf8_lossy(&out.stderr)
                    )
                    .trim()
                    .to_string();
                    if combined.is_empty() {
                        format!("Failed (exit code: {:?})", out.status.code())
                    } else {
                        format!("Failed: {}", combined)
                    }
                }
                Err(e) => format!("Failed: {}", e),
            };

            let _ = std::fs::write("/tmp/howdy_add_result.txt", &result_msg);
        });
    }

    /// Check if there's a pending add_model result (called periodically from QML)
    /// Returns true if a result was found and processed
    pub fn check_add_result(mut self: Pin<&mut Self>) -> bool {
        if let Ok(result) = std::fs::read_to_string("/tmp/howdy_add_result.txt") {
            let _ = std::fs::remove_file("/tmp/howdy_add_result.txt");
            self.as_mut().set_status_message(QString::from(&result));
            return true;
        }
        false
    }

    /// Discard the just-registered face (delete it)
    pub fn discard_face(mut self: Pin<&mut Self>) {
        let howdy = match find_howdy() {
            Some(p) => p,
            None => return,
        };

        if let Ok(id_str) = std::fs::read_to_string("/tmp/howdy_new_face_id.txt") {
            let _ = std::fs::remove_file("/tmp/howdy_new_face_id.txt");
            if let Ok(id) = id_str.trim().parse::<i32>() {
                let _ = Command::new("pkexec")
                    .args([&howdy, "remove", "-y", &id.to_string()])
                    .output();
            }
        }
        self.as_mut()
            .set_status_message(QString::from("Face discarded"));
    }

    /// Remove a face model by its ID
    pub fn remove_model(mut self: Pin<&mut Self>, index: i32) {
        let howdy = match find_howdy() {
            Some(p) => p,
            None => {
                self.as_mut()
                    .set_status_message(QString::from("Howdy is not installed"));
                return;
            }
        };
        let output = Command::new("pkexec")
            .args([&howdy, "remove", "-y", &index.to_string()])
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    self.as_mut()
                        .set_status_message(QString::from("Model removed"));
                    self.refresh_models();
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    self.as_mut()
                        .set_status_message(QString::from(&format!("Failed: {}", stderr)));
                }
            }
            Err(e) => {
                self.as_mut()
                    .set_status_message(QString::from(&format!("Error: {}", e)));
            }
        }
    }

    /// Toggle howdy enabled/disabled
    pub fn toggle_enabled(mut self: Pin<&mut Self>) {
        let howdy = match find_howdy() {
            Some(p) => p,
            None => {
                self.as_mut()
                    .set_status_message(QString::from("Howdy is not installed"));
                return;
            }
        };
        let current = *self.as_ref().howdy_enabled();
        let arg = if current { "1" } else { "0" };

        let output = Command::new("pkexec")
            .args([&howdy, "disable", arg])
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    self.as_mut().set_howdy_enabled(!current);
                    let msg = if current {
                        "Howdy disabled"
                    } else {
                        "Howdy enabled"
                    };
                    self.as_mut().set_status_message(QString::from(msg));
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    self.as_mut()
                        .set_status_message(QString::from(&format!("Failed: {}", stderr)));
                }
            }
            Err(e) => {
                self.as_mut()
                    .set_status_message(QString::from(&format!("Error: {}", e)));
            }
        }
    }

    /// Scan /dev/ for video* devices and check if howdy's device_path is already configured
    pub fn load_video_devices(mut self: Pin<&mut Self>) {
        let mut device_list: Vec<String> = fs::read_dir("/dev")
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with("video") {
                    Some(format!("/dev/{}", name))
                } else {
                    None
                }
            })
            .collect();
        device_list.sort();

        let mut devices = QList::<QString>::default();
        for d in &device_list {
            devices.append_clone(&QString::from(d.as_str()));
        }
        self.as_mut().set_video_devices(devices);

        // Check if device_path is already set to something other than "none" in the config
        let config_candidates = [
            "/usr/lib/security/howdy/config.ini",
            "/lib/security/howdy/config.ini",
            "/etc/howdy/config.ini",
        ];
        let mut configured = false;
        for candidate in &config_candidates {
            if let Ok(content) = fs::read_to_string(candidate) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.starts_with('#') && trimmed.starts_with("device_path") {
                        if let Some(value) = trimmed.split('=').nth(1) {
                            let device = value.trim();
                            if !device.is_empty() && device != "none" && device != "null" {
                                configured = true;
                            }
                        }
                        break;
                    }
                }
                break;
            }
        }
        self.as_mut().set_camera_configured(configured);
    }

    /// Launch mpv to preview the selected camera device
    pub fn test_camera(mut self: Pin<&mut Self>, device: QString) {
        let device_str = device.to_string();
        if device_str.is_empty() {
            self.as_mut()
                .set_status_message(QString::from("No device selected"));
            return;
        }
        match Command::new("mpv").arg(&device_str).spawn() {
            Ok(_) => {
                self.as_mut().set_status_message(QString::from(&format!(
                    "Previewing {} — close mpv when done",
                    device_str
                )));
            }
            Err(e) => {
                self.as_mut()
                    .set_status_message(QString::from(&format!("Failed to launch mpv: {}", e)));
            }
        }
    }

    /// Write device_path into the howdy config file using pkexec
    pub fn save_device_path(mut self: Pin<&mut Self>, device: QString) {
        let device_str = device.to_string();
        if device_str.is_empty() {
            self.as_mut()
                .set_status_message(QString::from("No device selected"));
            return;
        }

        let config_candidates = [
            "/usr/lib/security/howdy/config.ini",
            "/lib/security/howdy/config.ini",
            "/etc/howdy/config.ini",
        ];

        let mut config_path: Option<&str> = None;
        let mut current_content = String::new();
        for candidate in &config_candidates {
            if Path::new(candidate).exists() {
                if let Ok(content) = fs::read_to_string(candidate) {
                    current_content = content;
                    config_path = Some(candidate);
                    break;
                }
            }
        }

        let config_path = match config_path {
            Some(p) => p,
            None => {
                self.as_mut()
                    .set_status_message(QString::from("Howdy config not found"));
                return;
            }
        };

        // Replace the device_path line, or append it if absent
        let mut found = false;
        let mut lines: Vec<String> = current_content
            .lines()
            .map(|line| {
                let trimmed = line.trim();
                if !trimmed.starts_with('#') && trimmed.starts_with("device_path") {
                    found = true;
                    format!("device_path = {}", device_str)
                } else {
                    line.to_string()
                }
            })
            .collect();
        if !found {
            lines.push(format!("device_path = {}", device_str));
        }
        let new_content = lines.join("\n") + "\n";

        // Write to a temp file, then copy it with elevated privileges
        let tmp = "/tmp/howdy_config_tmp.ini";
        if let Err(e) = fs::write(tmp, &new_content) {
            self.as_mut()
                .set_status_message(QString::from(&format!("Failed to write temp file: {}", e)));
            return;
        }

        let output = Command::new("pkexec")
            .args(["/usr/bin/cp", tmp, config_path])
            .output();
        let _ = fs::remove_file(tmp);

        match output {
            Ok(out) if out.status.success() => {
                self.as_mut()
                    .set_status_message(QString::from(&format!("Camera saved: {}", device_str)));
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.as_mut()
                    .set_status_message(QString::from(&format!("Failed to save: {}", stderr)));
            }
            Err(e) => {
                self.as_mut()
                    .set_status_message(QString::from(&format!("Error: {}", e)));
            }
        }
    }

    /// Read all PAM files and update the corresponding properties
    pub fn check_pam_status(mut self: Pin<&mut Self>) {
        let read = |p| {
            fs::read_to_string(p)
                .map(|c| pam_howdy_active(&c))
                .unwrap_or(false)
        };
        self.as_mut().set_pam_sddm(read(PAM_SDDM));
        self.as_mut().set_pam_kde(read(PAM_KDE));
        self.as_mut().set_pam_sudo(read(PAM_SUDO));
        self.as_mut().set_pam_system_login(read(PAM_SYSTEM_LOGIN));
        self.as_mut().set_pam_plasma_lm(read(PAM_PLASMA_LM));
        self.as_mut().set_pam_polkit(read(PAM_POLKIT));
    }

    /// Toggle the howdy line in the given PAM file path:
    ///   off → on : uncomment existing commented line, or insert a new one
    ///   on  → off: comment out the active line
    pub fn toggle_pam(mut self: Pin<&mut Self>, file: QString) {
        let file_path = file.to_string();

        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // No /etc/pam.d/ override yet — seed from vendor config if available
                let vendor = file_path.replace("/etc/pam.d/", "/usr/lib/pam.d/");
                match fs::read_to_string(&vendor) {
                    Ok(c) => c,
                    Err(_) => {
                        self.as_mut().set_status_message(QString::from(&format!(
                            "No PAM config found for {}",
                            file_path
                        )));
                        return;
                    }
                }
            }
            Err(e) => {
                self.as_mut().set_status_message(QString::from(&format!(
                    "Cannot read {}: {}",
                    file_path, e
                )));
                return;
            }
        };

        let currently_enabled = pam_howdy_active(&content);

        let new_content = if currently_enabled {
            // Disable: comment out every active howdy auth line
            content
                .lines()
                .map(|line| {
                    let t = line.trim();
                    if !t.starts_with('#')
                        && (t.contains("pam_howdy.so") || t.contains("howdy/pam.py"))
                    {
                        format!("#{}", line)
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        } else if file_path.as_str() == PAM_POLKIT {
            // For polkit: always use the full module path and remove any pre-existing
            // howdy lines (commented or active) to produce a clean, canonical file.
            let mut lines: Vec<String> = content
                .lines()
                .filter(|l| {
                    let t = l.trim();
                    !t.contains("pam_howdy.so") && !t.contains("howdy/pam.py")
                })
                .map(|l| l.to_string())
                .collect();
            let pos = if lines
                .first()
                .map(|l| l.starts_with("#%PAM-1.0"))
                .unwrap_or(false)
            {
                1
            } else {
                0
            };
            lines.insert(pos, PAM_POLKIT_LINE.to_string());
            lines.join("\n") + "\n"
        } else {
            let has_commented = content.lines().any(|line| {
                let t = line.trim();
                t.starts_with('#') && (t.contains("pam_howdy.so") || t.contains("howdy/pam.py"))
            });
            if has_commented {
                // Uncomment: strip leading # and optional space
                content
                    .lines()
                    .map(|line| {
                        let t = line.trim();
                        if t.starts_with('#')
                            && (t.contains("pam_howdy.so") || t.contains("howdy/pam.py"))
                        {
                            t.strip_prefix('#').unwrap_or(t).trim_start().to_string()
                        } else {
                            line.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n"
            } else {
                // Insert after #%PAM-1.0 header if present, otherwise at the top
                let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
                let pos = if lines
                    .first()
                    .map(|l| l.starts_with("#%PAM-1.0"))
                    .unwrap_or(false)
                {
                    1
                } else {
                    0
                };
                lines.insert(pos, PAM_LINE.to_string());
                lines.join("\n") + "\n"
            }
        };

        let tmp = "/tmp/howdy_pam_tmp";
        if let Err(e) = fs::write(tmp, &new_content) {
            self.as_mut()
                .set_status_message(QString::from(&format!("Failed to write temp file: {}", e)));
            return;
        }

        // polkit needs its systemd service sandbox relaxed so pam_howdy can open the
        // camera (/dev/video*).  polkit-agent-helper@.service ships with PrivateDevices=yes
        // which hides all devices — compare.py sees an empty /dev and captures 0 frames.
        // We do everything in one pkexec bash invocation to avoid multiple auth dialogs.
        if file_path.as_str() == PAM_POLKIT {
            let output = if !currently_enabled {
                let override_content =
                    "[Service]\nPrivateDevices=no\nDeviceAllow=char-video4linux rw\n";
                let _ = fs::write("/tmp/howdy_polkit_override.conf", override_content);
                let howdy = find_howdy().unwrap_or_else(|| "/usr/bin/howdy".to_string());
                let script = format!(
                    "cp /tmp/howdy_pam_tmp /etc/pam.d/polkit-1 && \
                     {h} set warn_no_device false && \
                     {h} set timeout_notice false && \
                     {h} set timeout 10 && \
                     mkdir -p '/etc/systemd/system/polkit-agent-helper@.service.d' && \
                     cp /tmp/howdy_polkit_override.conf \
                        '/etc/systemd/system/polkit-agent-helper@.service.d/override.conf' && \
                     systemctl daemon-reload && \
                     systemctl restart polkit-agent-helper.socket",
                    h = howdy
                );
                Command::new("pkexec")
                    .args(["/usr/bin/bash", "-c", &script])
                    .output()
            } else {
                let script =
                    "cp /tmp/howdy_pam_tmp /etc/pam.d/polkit-1 && \
                     rm -f '/etc/systemd/system/polkit-agent-helper@.service.d/override.conf' && \
                     systemctl daemon-reload && \
                     systemctl restart polkit-agent-helper.socket";
                Command::new("pkexec")
                    .args(["/usr/bin/bash", "-c", script])
                    .output()
            };
            let _ = fs::remove_file(tmp);
            let _ = fs::remove_file("/tmp/howdy_polkit_override.conf");
            match output {
                Ok(out) if out.status.success() => {
                    self.as_mut().set_pam_polkit(!currently_enabled);
                    let msg = if !currently_enabled {
                        "Howdy enabled for polkit (camera sandbox configured)"
                    } else {
                        "Howdy disabled for polkit"
                    };
                    self.as_mut().set_status_message(QString::from(msg));
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    self.as_mut().set_status_message(QString::from(&format!(
                        "Failed to configure polkit: {}",
                        stderr
                    )));
                }
                Err(e) => {
                    self.as_mut()
                        .set_status_message(QString::from(&format!("Error: {}", e)));
                }
            }
            return;
        }

        let output = Command::new("pkexec")
            .args(["/usr/bin/cp", tmp, file_path.as_str()])
            .output();
        let _ = fs::remove_file(tmp);

        match output {
            Ok(out) if out.status.success() => {
                let new_state = !currently_enabled;
                let fname = file_path.rsplit('/').next().unwrap_or(&file_path);
                let msg = if new_state {
                    format!("Howdy enabled in {}", fname)
                } else {
                    format!("Howdy disabled in {}", fname)
                };
                match file_path.as_str() {
                    PAM_SDDM => self.as_mut().set_pam_sddm(new_state),
                    PAM_KDE => self.as_mut().set_pam_kde(new_state),
                    PAM_SUDO => self.as_mut().set_pam_sudo(new_state),
                    PAM_SYSTEM_LOGIN => self.as_mut().set_pam_system_login(new_state),
                    PAM_PLASMA_LM => self.as_mut().set_pam_plasma_lm(new_state),
                    _ => {}
                }
                self.as_mut().set_status_message(QString::from(&msg));
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.as_mut().set_status_message(QString::from(&format!(
                    "Failed to update PAM: {}",
                    stderr
                )));
            }
            Err(e) => {
                self.as_mut()
                    .set_status_message(QString::from(&format!("Error: {}", e)));
            }
        }
    }

    /// Run face recognition test
    pub fn run_test(mut self: Pin<&mut Self>) {
        let howdy = match find_howdy() {
            Some(p) => p,
            None => {
                self.as_mut()
                    .set_status_message(QString::from("Howdy is not installed"));
                return;
            }
        };
        self.as_mut()
            .set_status_message(QString::from("Running test... Look at the camera"));

        let output = pkexec_run(&howdy, &["test"]);

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let result = if out.status.success() {
                    format!(
                        "Test complete: {}",
                        stdout.lines().last().unwrap_or("Success")
                    )
                } else {
                    format!("Test failed: {}", String::from_utf8_lossy(&out.stderr))
                };
                self.as_mut().set_status_message(QString::from(&result));
            }
            Err(e) => {
                self.as_mut()
                    .set_status_message(QString::from(&format!("Error: {}", e)));
            }
        }
    }
}
