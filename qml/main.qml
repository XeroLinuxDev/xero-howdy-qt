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
            checkDevice()
            refreshModels()
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
            color: backend.deviceSupported ? "#2e7d32" : "#c62828"

            Label {
                anchors.centerIn: parent
                text: backend.deviceSupported
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
                model: backend.faceModels
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
                                backend.removeModel(id)
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
                enabled: backend.deviceSupported
                onClicked: {
                    backend.addModel(newModelName.text)
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
                onClicked: backend.refreshModels()
            }

            Button {
                text: "◎ Test Recognition"
                font.pixelSize: 14
                enabled: backend.deviceSupported
                onClicked: backend.runTest()
            }

            Button {
                text: backend.howdyEnabled ? "○ Disable Howdy" : "● Enable Howdy"
                font.pixelSize: 14
                onClicked: backend.toggleEnabled()
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
                text: backend.statusMessage
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
