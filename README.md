# Xero Howdy Qt

A Qt-based graphical interface for managing [**Howdy**](https://github.com/boltgolt/howdy) face authentication on Linux systems with Windows Hello compatible hardware.

<p align="center">

<img width="598" height="653" alt="image" src="https://github.com/user-attachments/assets/7785ca98-c655-4d4c-99c3-2acfa4775159" />

</p>

## Hardware Requirements

- **IR Camera**: A Windows Hello compatible infrared camera is required. Found in:
  - Modern laptops (Dell, Lenovo ThinkPad, HP EliteBook, Microsoft Surface, etc.)
  - External webcams (e.g., Logitech Brio)

- **Supported Devices**: Your IR camera must be recognized by Linux and accessible via `/dev/video*`

## Testing The App

This application is designed exclusively for **XeroLinux** and will not work on other distributions.

To test without installing the package:

```bash
# Install dependencies
sudo pacman -S rust clang qt6-base qt6-declarative howdy-bin

# Clone and run
git clone https://github.com/XeroLinuxDev/xero-howdy-qt.git
cd xero-howdy-qt
cargo run
```

## Usage

1. **Launch the application** - It will automatically detect if compatible hardware is present
   - Green banner: IR camera detected and ready
   - Red banner: No compatible camera found

2. **Register a face**:
   - Enter a name in the text field (e.g., "Default", "Glasses", "Low Light")
   - Click "Register Face"
   - Look at your IR camera when prompted
   - Authenticate with your password via polkit

3. **Test recognition**:
   - Click "Test Recognition" to verify your face is being detected properly

4. **Manage faces**:
   - View all registered faces in the list
   - Click "Remove" to delete a specific face model
   - Use "Refresh" to reload the list

5. **Enable/Disable**:
   - Toggle Howdy on or off system-wide using the Enable/Disable button
