import QtQuick
import qs.Commons
import qs.Ui

Item {
  id: root
  property string checkState: "none" // none | partial | all
  property color foreground: Color.menu.text
  property color accent: Color.accent
  property bool interactive: true
  signal clicked()

  width: Style.space(16)
  height: Style.space(16)

  readonly property bool on: root.checkState === "all" || root.checkState === "partial"

  BorderSurface {
    anchors.fill: parent
    radius: 0
    color: root.on ? Style.selectedFillFor(root.foreground, root.accent) : "transparent"
    borderSpec: root.on
      ? Border.controlSpec("selected", root.foreground, root.accent)
      : Border.controlSpec("normal", root.foreground, root.accent)

    Text {
      anchors.centerIn: parent
      visible: root.checkState === "all"
      text: "✓"
      color: Style.selectedStateColor(root.foreground, root.accent)
      font.pixelSize: Math.round(root.height * 0.85)
      font.bold: true
    }

    Rectangle {
      visible: root.checkState === "partial"
      width: Math.max(Style.space(8), parent.width * 0.5)
      height: Math.max(2, Style.space(2))
      anchors.centerIn: parent
      color: Style.selectedStateColor(root.foreground, root.accent)
    }
  }

  MouseArea {
    anchors.fill: parent
    enabled: root.interactive
    onClicked: root.clicked()
  }
}
