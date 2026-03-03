import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.howdy.gui

ApplicationWindow {
    id: window
    visible: true
    width: 800
    height: 850
    title: "Xero Howdy Qt"

    HowdyBackend {
        id: backend
        Component.onCompleted: {
            backend.load_video_devices()
            backend.check_pam_status()
            if (!backend.camera_configured) {
                cameraDialog.open()
            } else {
                backend.check_device()
                backend.refresh_models()
            }
        }
    }

    Dialog {
        id: cameraDialog
        title: "Select IR Camera"
        modal: true
        closePolicy: Popup.NoAutoClose
        anchors.centerIn: Overlay.overlay
        width: 540

        ColumnLayout {
            width: parent.width
            spacing: 16

            Label {
                Layout.fillWidth: true
                text: "Select the IR camera device to use for face recognition:"
                font.pixelSize: 14
                wrapMode: Text.Wrap
            }

            ComboBox {
                id: devicePicker
                Layout.fillWidth: true
                model: backend.video_devices
                font.pixelSize: 14
            }

            Label {
                Layout.fillWidth: true
                text: "No video devices found in /dev/"
                visible: devicePicker.count === 0
                color: palette.placeholderText
                font.italic: true
                font.pixelSize: 13
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                Button {
                    text: "▶ Test Camera"
                    font.pixelSize: 14
                    enabled: devicePicker.count > 0
                    onClicked: backend.test_camera(devicePicker.currentText)
                }

                Item { Layout.fillWidth: true }

                Button {
                    text: "Skip"
                    font.pixelSize: 14
                    onClicked: {
                        cameraDialog.close()
                        backend.check_device()
                        backend.refresh_models()
                    }
                }

                Button {
                    text: "Save & Continue"
                    font.pixelSize: 14
                    highlighted: true
                    enabled: devicePicker.count > 0
                    onClicked: {
                        backend.save_device_path(devicePicker.currentText)
                        cameraDialog.close()
                        backend.check_device()
                        backend.refresh_models()
                    }
                }
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 28
        spacing: 20

        // Header with emoji
        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "\u{1F44B} Xero Howdy Qt \u{1F44B}"
            font.pixelSize: 32
            font.bold: true
        }

        // Description
        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "Manage face authentication on Windows Hello supported devices using Howdy"
            font.pixelSize: 15
            color: palette.placeholderText
            horizontalAlignment: Text.AlignHCenter
        }

        // Device status banner
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 32
            radius: 8
            color: backend.device_supported ? "#2e7d32" : "#c62828"

            Label {
                anchors.centerIn: parent
                text: backend.device_supported
                    ? "● IR Camera Detected - Ready to Use"
                    : "▲ No Compatible IR Camera Found"
                color: "white"
                font.pixelSize: 14
                font.bold: true
            }
        }

        // Section header
        Label {
            text: "Registered Faces"
            font.pixelSize: 20
            font.bold: true
        }

        // Model list
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: palette.base
            border.color: palette.mid
            border.width: 1
            radius: 12

            ListView {
                id: modelList
                anchors.fill: parent
                anchors.margins: 14
                model: backend.face_models
                clip: true
                spacing: 6

                delegate: ItemDelegate {
                    width: modelList.width
                    height: 54

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 18
                        anchors.rightMargin: 18

                        Label {
                            Layout.fillWidth: true
                            text: modelData
                            font.pixelSize: 15
                            elide: Text.ElideRight
                        }

                        Button {
                            text: "× Remove"
                            flat: true
                            onClicked: {
                                var id = parseInt(modelData.trim().split(/\s+/)[0])
                                backend.remove_model(id)
                            }
                        }
                    }
                }

                Label {
                    anchors.centerIn: parent
                    text: "No faces registered yet"
                    visible: modelList.count === 0
                    color: palette.placeholderText
                    font.italic: true
                    font.pixelSize: 15
                }
            }
        }

        // Add new model section
        RowLayout {
            Layout.fillWidth: true
            spacing: 14

            TextField {
                id: newModelName
                Layout.fillWidth: true
                font.pixelSize: 15
                placeholderText: "Enter a name for this face (e.g., Home, Office, Glasses)..."
                onAccepted: addButton.clicked()
            }

            Button {
                id: addButton
                text: "+ Register Face"
                font.pixelSize: 15
                highlighted: true
                enabled: backend.device_supported
                onClicked: {
                    backend.add_model(newModelName.text)
                    newModelName.text = ""
                }
            }
        }

        // Centered action buttons
        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 18

            Button {
                text: "↻ Refresh"
                font.pixelSize: 14
                onClicked: backend.refresh_models()
            }

            Button {
                text: "◎ Test Recognition"
                font.pixelSize: 14
                enabled: backend.device_supported
                onClicked: backend.run_test()
            }

            Button {
                text: backend.howdy_enabled ? "○ Disable Howdy" : "● Enable Howdy"
                font.pixelSize: 14
                onClicked: backend.toggle_enabled()
            }
        }

        // PAM authentication toggles
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: pamColumn.implicitHeight + 28
            color: palette.alternateBase
            radius: 10

            ColumnLayout {
                id: pamColumn
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 14 }
                spacing: 0

                Label {
                    text: "PAM Authentication"
                    font.pixelSize: 15
                    font.bold: true
                    Layout.bottomMargin: 10
                }

                // SDDM login screen
                RowLayout {
                    Layout.fillWidth: true
                    Column {
                        spacing: 2
                        Label { text: "SDDM Login Screen"; font.pixelSize: 14; font.bold: true }
                        Label { text: "Unlock the display manager login screen with your face"; font.pixelSize: 12; color: palette.placeholderText }
                    }
                    Item { Layout.fillWidth: true }
                    Switch { checked: backend.pam_sddm; onToggled: backend.toggle_pam("/etc/pam.d/sddm") }
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: palette.mid; opacity: 0.4; Layout.topMargin: 4; Layout.bottomMargin: 4 }

                // KDE screen lock
                RowLayout {
                    Layout.fillWidth: true
                    Column {
                        spacing: 2
                        Label { text: "KDE Screen Lock"; font.pixelSize: 14; font.bold: true }
                        Label { text: "Unlock the KDE screen locker without typing your password"; font.pixelSize: 12; color: palette.placeholderText }
                    }
                    Item { Layout.fillWidth: true }
                    Switch { checked: backend.pam_kde; onToggled: backend.toggle_pam("/etc/pam.d/kde") }
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: palette.mid; opacity: 0.4; Layout.topMargin: 4; Layout.bottomMargin: 4 }

                // sudo
                RowLayout {
                    Layout.fillWidth: true
                    Column {
                        spacing: 2
                        Label { text: "sudo (Terminal)"; font.pixelSize: 14; font.bold: true }
                        Label { text: "Run sudo commands without typing your password"; font.pixelSize: 12; color: palette.placeholderText }
                    }
                    Item { Layout.fillWidth: true }
                    Switch { checked: backend.pam_sudo; onToggled: backend.toggle_pam("/etc/pam.d/sudo") }
                }

                Rectangle { Layout.fillWidth: true; height: 1; color: palette.mid; opacity: 0.4; Layout.topMargin: 4; Layout.bottomMargin: 4 }

                // system-local-login
                RowLayout {
                    Layout.fillWidth: true
                    Column {
                        spacing: 2
                        Label { text: "System Local Login"; font.pixelSize: 14; font.bold: true }
                        Label { text: "General TTY and local system login authentication"; font.pixelSize: 12; color: palette.placeholderText }
                    }
                    Item { Layout.fillWidth: true }
                    Switch { checked: backend.pam_system_login; onToggled: backend.toggle_pam("/etc/pam.d/system-local-login") }
                }
            }
        }

        // Status bar
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 40
            color: palette.alternateBase
            radius: 10

            Label {
                anchors.centerIn: parent
                width: parent.width - 24
                text: backend.status_message
                elide: Text.ElideRight
                horizontalAlignment: Text.AlignHCenter
                color: palette.text
                font.pixelSize: 14
            }
        }

        // Footer credits
        Row {
            Layout.alignment: Qt.AlignHCenter
            spacing: 4

            Label {
                text: "GUI Front-End for"
                font.pixelSize: 13
                color: palette.placeholderText
            }

            Label {
                text: "Howdy"
                font.pixelSize: 13
                font.bold: true
                font.italic: true
                color: "#2196F3"

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: Qt.openUrlExternally("https://github.com/boltgolt/howdy")
                }
            }

            Label {
                text: "- created by"
                font.pixelSize: 13
                color: palette.placeholderText
            }

            Label {
                text: "DarkXero"
                font.pixelSize: 13
                font.bold: true
                color: "#9C27B0"

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: Qt.openUrlExternally("https://github.com/XeroLinuxDev")
                }
            }
        }
    }
}
