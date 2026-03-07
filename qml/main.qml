import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.howdy.gui

ApplicationWindow {
    id: window
    visible: true
    width: 800
    height: 950
    title: "Xero Howdy Qt"

    HowdyBackend {
        id: backend
        Component.onCompleted: {
            backend.detect_display_managers()
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

    // Watch status_message changes — this uses Qt's automatic Q_PROPERTY
    // change notification which is more reliable than a custom cxx-qt signal
    // with parameter marshalling. Any message other than the initial
    // "Look at the camera..." is treated as a completion result.
    Connections {
        target: backend
        function onStatusMessageChanged() {
            if (!registerDialog.capturing) return
            var msg = backend.status_message
            if (msg === "Look at the camera...") return
            registerDialog.capturing = false
            if (msg === "Face registered successfully") {
                registerDialog.close()
                refreshTimer.start()
            } else {
                registerDialog.captureFailed = true
                registerDialog.failureMessage = msg
            }
        }
    }

    // Refresh after the dialog close animation completes so the Qt thread
    // is not blocked before the popup has visually closed.
    Timer {
        id: refreshTimer
        interval: 400
        onTriggered: backend.refresh_models()
    }

    // ── Camera selection dialog (first-run) ──────────────────────────────────
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

    // ── Register face dialog ─────────────────────────────────────────────────
    Dialog {
        id: registerDialog
        title: "Register Face"
        modal: true
        closePolicy: Popup.NoAutoClose
        anchors.centerIn: Overlay.overlay
        width: 460

        property string modelName: ""
        property bool capturing: false
        property bool captureFailed: false
        property bool captureSuccess: false
        property string failureMessage: ""

        onClosed: {
            capturing = false
            captureFailed = false
            captureSuccess = false
            failureMessage = ""
        }

        // Safety net: if the background thread never responds (pkexec hung,
        // howdy couldn't detect a face, etc.) stop waiting after 90 seconds.
        Timer {
            id: captureTimeout
            interval: 90000
            running: registerDialog.capturing
            onTriggered: {
                registerDialog.capturing = false
                registerDialog.captureFailed = true
                registerDialog.failureMessage = "Timed out — face was not detected.\nMake sure you are in front of the camera and try again."
            }
        }

        // Poll for add_model result while capturing
        Timer {
            id: resultPoller
            interval: 500
            running: registerDialog.capturing
            repeat: true
            onTriggered: {
                var hasResult = backend.check_add_result()
                if (hasResult) {
                    var msg = backend.status_message
                    registerDialog.capturing = false
                    if (msg === "Face registered successfully") {
                        registerDialog.captureSuccess = true
                    } else {
                        registerDialog.captureFailed = true
                        registerDialog.failureMessage = msg
                    }
                }
            }
        }

        ColumnLayout {
            width: parent.width
            spacing: 18

            Rectangle {
                Layout.alignment: Qt.AlignHCenter
                width: 200; height: 230
                color: "transparent"

                Rectangle {
                    anchors.centerIn: parent
                    width: 180; height: 210
                    radius: 90
                    color: "transparent"
                    border.width: 2
                    border.color: registerDialog.captureSuccess ? "#66BB6A"
                                : registerDialog.captureFailed ? "#ef5350"
                                : registerDialog.capturing     ? "#FFA726"
                                :                               "#66BB6A"
                    SequentialAnimation on opacity {
                        running: registerDialog.capturing
                        loops: Animation.Infinite
                        NumberAnimation { to: 0.3; duration: 700 }
                        NumberAnimation { to: 1.0; duration: 700 }
                    }
                }

                Rectangle {
                    anchors.centerIn: parent
                    anchors.verticalCenterOffset: -10
                    width: 90; height: 105
                    radius: 45
                    color: palette.alternateBase
                    border.width: 1
                    border.color: palette.mid
                }

                Row {
                    anchors.centerIn: parent
                    anchors.verticalCenterOffset: -22
                    spacing: 22
                    Rectangle { width: 12; height: 8; radius: 4; color: palette.text; opacity: 0.7 }
                    Rectangle { width: 12; height: 8; radius: 4; color: palette.text; opacity: 0.7 }
                }

                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 170; height: 2
                    color: "#FFA726"
                    opacity: 0.85
                    visible: registerDialog.capturing
                    SequentialAnimation on y {
                        running: registerDialog.capturing
                        loops: Animation.Infinite
                        NumberAnimation { to: 20;  duration: 800; easing.type: Easing.InOutSine }
                        NumberAnimation { to: 195; duration: 800; easing.type: Easing.InOutSine }
                    }
                }
            }

            Label {
                Layout.alignment: Qt.AlignHCenter
                font.pixelSize: 15
                font.bold: true
                text: registerDialog.captureSuccess  ? "Face Captured Successfully!"
                    : registerDialog.capturing      ? "Hold still — scanning your face…"
                    : registerDialog.captureFailed  ? "Face not detected"
                    :                                 "Ready to capture"
            }

            Label {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                font.pixelSize: 13
                color: registerDialog.captureSuccess ? "#66BB6A" : (registerDialog.captureFailed ? "#ef5350" : palette.placeholderText)
                text: registerDialog.captureSuccess
                    ? "Your face has been captured.\nClick 'Save Face' to confirm or 'Cancel' to discard."
                    : registerDialog.capturing
                        ? "Please look directly at the camera and keep still."
                        : registerDialog.captureFailed
                            ? "Could not detect your face.\n\n" +
                              "• Make sure your face is well-lit\n" +
                              "• Look directly at the IR camera\n" +
                              "• Remove glasses if worn\n" +
                              "• Move closer to the camera\n\n" +
                              registerDialog.failureMessage
                            : "Look directly at the IR camera.\n" +
                              "Ensure good lighting and keep your face centred.\n" +
                              "Click Start Capture when ready."
            }

            Rectangle {
                Layout.fillWidth: true
                height: 4; radius: 2
                color: palette.alternateBase
                visible: registerDialog.capturing

                Rectangle {
                    height: parent.height; radius: parent.radius
                    color: "#FFA726"
                    SequentialAnimation on width {
                        running: registerDialog.capturing
                        loops: Animation.Infinite
                        NumberAnimation { to: registerDialog.width - 48; duration: 1000; easing.type: Easing.InOutSine }
                        NumberAnimation { to: 0; duration: 1000; easing.type: Easing.InOutSine }
                    }
                }
            }

            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 14

                Button {
                    text: registerDialog.captureSuccess ? "Save Face" : (registerDialog.captureFailed ? "Try Again" : "Start Capture")
                    font.pixelSize: 14
                    highlighted: true
                    enabled: !registerDialog.capturing
                    onClicked: {
                        if (registerDialog.captureSuccess) {
                            backend.refresh_models()
                            registerDialog.close()
                        } else if (registerDialog.captureFailed) {
                            registerDialog.captureFailed = false
                            registerDialog.capturing = true
                            backend.add_model(registerDialog.modelName)
                        } else {
                            registerDialog.capturing = true
                            backend.add_model(registerDialog.modelName)
                        }
                    }
                }
            }
        }
    }

    // ── Main UI ──────────────────────────────────────────────────────────────
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 28
        spacing: 16

        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "\u{1F44B} Xero Howdy Qt \u{1F44B}"
            font.pixelSize: 32
            font.bold: true
        }

        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "v" + backend.app_version
            font.pixelSize: 14
            color: palette.placeholderText
        }

        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "Manage face authentication on Windows Hello supported devices using Howdy"
            font.pixelSize: 15
            color: palette.placeholderText
            horizontalAlignment: Text.AlignHCenter
        }

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

        Label {
            text: "Registered Faces"
            font.pixelSize: 20
            font.bold: true
        }

        // Face list with circuit decorations
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 180
            color: palette.base
            border.color: "#9C27B0"
            border.width: 1
            radius: 12

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 8

                // Header with face icon and circuits
                RowLayout {
                    Layout.alignment: Qt.AlignHCenter
                    spacing: 8

                    // Left circuit
                    ColumnLayout {
                        spacing: 0
                        Rectangle { width: 30; height: 2; color: "#9C27B0" }
                        Rectangle { width: 2; height: 8; color: "#9C27B0" }
                        Rectangle { width: 30; height: 2; color: "#9C27B0" }
                        Rectangle { width: 2; height: 6; color: "#9C27B0"; x: 10 }
                        Rectangle { width: 20; height: 2; color: "#9C27B0"; x: 10 }
                    }

                    // Face + Camera icon
                    RowLayout {
                        spacing: 6
                        Rectangle {
                            width: 28; height: 32
                            radius: 14
                            color: "transparent"
                            border.width: 2
                            border.color: "#9C27B0"
                            Rectangle {
                                anchors.centerIn: parent
                                width: 12; height: 14; radius: 6
                                color: "#9C27B0"; opacity: 0.3
                            }
                            Row {
                                anchors.centerIn: parent
                                anchors.verticalCenterOffset: -4
                                spacing: 6
                                Rectangle { width: 4; height: 4; radius: 2; color: "#9C27B0" }
                                Rectangle { width: 4; height: 4; radius: 2; color: "#9C27B0" }
                            }
                        }
                        Rectangle {
                            width: 24; height: 20
                            radius: 4
                            color: "transparent"
                            border.width: 2
                            border.color: "#9C27B0"
                            Rectangle {
                                anchors.centerIn: parent
                                width: 10; height: 10; radius: 5
                                color: "#9C27B0"; opacity: 0.5
                            }
                            Rectangle {
                                width: 4; height: 4; radius: 2
                                color: "#9C27B0"
                                anchors.right: parent.right
                                anchors.top: parent.top
                                anchors.margins: 2
                            }
                        }
                    }

                    // Right circuit (mirrored)
                    ColumnLayout {
                        spacing: 0
                        Rectangle { width: 30; height: 2; color: "#9C27B0"; x: 20 }
                        Rectangle { width: 2; height: 6; color: "#9C27B0"; x: 30 }
                        Rectangle { width: 30; height: 2; color: "#9C27B0" }
                        Rectangle { width: 2; height: 8; color: "#9C27B0" }
                        Rectangle { width: 30; height: 2; color: "#9C27B0" }
                    }
                }

                // Face list
                ListView {
                    id: modelList
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    model: backend.face_models
                    clip: true
                    spacing: 4

                    delegate: ItemDelegate {
                        width: modelList.width
                        height: 36

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 8
                            anchors.rightMargin: 8

                            Label {
                                Layout.fillWidth: true
                                text: modelData
                                font.pixelSize: 13
                                elide: Text.ElideRight
                            }

                            Button {
                                text: "×"
                                flat: true
                                font.pixelSize: 12
                                onClicked: {
                                    var id = parseInt(modelData.trim().split(/\s+/)[0])
                                    backend.remove_model(id)
                                }
                            }
                        }
                    }

                    Label {
                        anchors.centerIn: parent
                        text: "No faces registered"
                        visible: modelList.count === 0
                        color: palette.placeholderText
                        font.italic: true
                        font.pixelSize: 13
                    }
                }
            }
        }

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
                enabled: backend.device_supported && newModelName.text.trim() !== ""
                onClicked: {
                    registerDialog.modelName = newModelName.text.trim()
                    newModelName.text = ""
                    registerDialog.open()
                }
            }
        }

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

        // PAM section — capped so it never squeezes the face list above
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(pamColumn.implicitHeight + 28, 250)
            color: palette.alternateBase
            radius: 10
            clip: true

            ScrollView {
                anchors.fill: parent
                anchors.margins: 14
                contentWidth: availableWidth
                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

                ColumnLayout {
                    id: pamColumn
                    width: parent.width
                    spacing: 0

                    Label {
                        text: "PAM Authentication"
                        font.pixelSize: 15
                        font.bold: true
                        Layout.bottomMargin: 8
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        visible: backend.sddm_installed
                        Column {
                            spacing: 2
                            Label { text: "SDDM Login Screen"; font.pixelSize: 14; font.bold: true }
                            Label { text: "Unlock the display manager login screen with your face"; font.pixelSize: 12; color: palette.placeholderText }
                        }
                        Item { Layout.fillWidth: true }
                        Switch { checked: backend.pam_sddm; onToggled: backend.toggle_pam("/etc/pam.d/sddm") }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: palette.mid; opacity: 0.4; Layout.topMargin: 4; Layout.bottomMargin: 4; visible: backend.sddm_installed }

                    RowLayout {
                        Layout.fillWidth: true
                        visible: backend.plasma_lm_installed
                        Column {
                            spacing: 2
                            Label { text: "Plasma Login Manager"; font.pixelSize: 14; font.bold: true }
                            Label { text: "Unlock the Plasma login manager screen with your face"; font.pixelSize: 12; color: palette.placeholderText }
                        }
                        Item { Layout.fillWidth: true }
                        Switch { checked: backend.pam_plasma_lm; onToggled: backend.toggle_pam("/etc/pam.d/plasmalogin") }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: palette.mid; opacity: 0.4; Layout.topMargin: 4; Layout.bottomMargin: 4; visible: backend.plasma_lm_installed }

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

                    RowLayout {
                        Layout.fillWidth: true
                        Column {
                            spacing: 2
                            Label { text: "pkexec / Polkit"; font.pixelSize: 14; font.bold: true }
                            Label { text: "Authenticate polkit and pkexec prompts with your face"; font.pixelSize: 12; color: palette.placeholderText }
                        }
                        Item { Layout.fillWidth: true }
                        Switch { checked: backend.pam_polkit; onToggled: backend.toggle_pam("/etc/pam.d/polkit-1") }
                    }

                    Rectangle { Layout.fillWidth: true; height: 1; color: palette.mid; opacity: 0.4; Layout.topMargin: 4; Layout.bottomMargin: 4 }
                }
            }
        }

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

        Row {
            Layout.alignment: Qt.AlignHCenter
            spacing: 4

            Label { text: "GUI Front-End for"; font.pixelSize: 13; color: palette.placeholderText }

            Label {
                text: "Howdy"
                font.pixelSize: 13; font.bold: true; font.italic: true
                color: "#2196F3"
                MouseArea {
                    anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                    onClicked: Qt.openUrlExternally("https://github.com/boltgolt/howdy")
                }
            }

            Label { text: "- created by"; font.pixelSize: 13; color: palette.placeholderText }

            Label {
                text: "DarkXero"
                font.pixelSize: 13; font.bold: true
                color: "#9C27B0"
                MouseArea {
                    anchors.fill: parent; cursorShape: Qt.PointingHandCursor
                    onClicked: Qt.openUrlExternally("https://github.com/XeroLinuxDev")
                }
            }
        }
    }
}
