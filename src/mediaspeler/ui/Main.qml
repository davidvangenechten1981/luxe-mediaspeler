import QtQuick
import QtQuick.Controls
import Mediaspeler 1.0

ApplicationWindow {
    id: window
    width: 960
    height: 540
    visible: true
    title: "Luxe mediaspeler"
    color: "black"

    MpvVideo {
        objectName: "videoSurface"
        anchors.fill: parent
    }

    // Knop over het beeld: bewijst dat overlays werken
    Button {
        objectName: "overlayPlayButton"
        text: "Pauze / Afspelen"
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 24
        onClicked: player.togglePause()
    }

    Text {
        objectName: "hwdecLabel"
        id: hwdecLabel
        color: "white"
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        anchors.margins: 12
        text: player.hwdecInfo()
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: hwdecLabel.text = player.hwdecInfo()
    }

    Rectangle {
        objectName: "errorBanner"
        visible: errorText.text.length > 0
        color: "#b00020"
        height: errorText.implicitHeight + 16
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top

        Text {
            id: errorText
            anchors.centerIn: parent
            color: "white"
            wrapMode: Text.Wrap
            width: parent.width - 32
        }
    }

    Connections {
        target: player
        function onErrorOccurred(message) { errorText.text = message }
    }
}
