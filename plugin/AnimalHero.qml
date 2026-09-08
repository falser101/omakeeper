import QtQuick
import qs.Commons

Item {
  id: root
  property string animal: "cat"
  property string mood: "idle"
  property bool spinning: false
  property real creatureSize: Style.space(280)
  property string headline: ""
  property string subline: ""
  property color foreground: Color.menu.text
  property string fontFamily: Style.font.menuFamily
  property bool showCaption: true

  readonly property bool busy: root.mood === "busy" || root.spinning
  readonly property int frameCount: 6
  property int frame: 0
  readonly property url pose: Qt.resolvedUrl("assets/" + root.animal + "-" + (root.frame % root.frameCount) + ".png")

  implicitHeight: root.creatureSize + (root.showCaption ? Style.space(88) : 0)

  Timer {
    interval: root.busy ? 140 : 280
    running: root.visible
    repeat: true
    onTriggered: root.frame = (root.frame + 1) % root.frameCount
  }

  onAnimalChanged: root.frame = 0

  Item {
    id: stage
    anchors.horizontalCenter: parent.horizontalCenter
    y: 0
    width: root.creatureSize
    height: root.creatureSize

    Image {
      anchors.fill: parent
      source: root.pose
      fillMode: Image.PreserveAspectFit
      smooth: false
      mipmap: false
      asynchronous: false
      cache: true
    }
  }

  Text {
    visible: root.showCaption
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: stage.bottom
    anchors.topMargin: Style.spacing.md
    text: root.headline
    color: root.foreground
    font.family: root.fontFamily
    font.pixelSize: Style.font.heading + 8
    font.bold: true
  }

  Text {
    visible: root.showCaption
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: stage.bottom
    anchors.topMargin: Style.spacing.md + Style.font.heading + 14
    text: root.subline
    color: root.foreground
    opacity: 0.65
    font.family: root.fontFamily
    font.pixelSize: Style.font.body
  }
}
