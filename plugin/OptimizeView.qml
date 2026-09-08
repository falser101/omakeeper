import QtQuick
import qs.Commons
import qs.Ui
import "App.js" as App

Item {
  id: root
  property var tasks: []
  property var selected: ({})
  property var logLines: []
  property int revision: 0
  property bool scanning: false
  property bool applying: false
  property int doneCount: 0
  property color foreground: Color.menu.text
  property string fontFamily: Style.font.menuFamily

  signal toggleId(string id)
  signal scanRequested()
  signal applyRequested()

  readonly property int readyCount: {
    var _ = root.revision
    var n = 0
    for (var i = 0; i < root.tasks.length; i++) {
      if (root.selected[root.tasks[i].id] && root.tasks[i].status === "ready") n++
    }
    return n
  }
  readonly property int totalCount: {
    var _ = root.revision
    return root.tasks.filter(function(t) { return t.status === "ready" }).length
  }

  AnimalHero {
    id: hero
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: parent.top
    width: parent.width
    animal: "otter"
    mood: (root.applying || root.scanning) ? "busy" : "idle"
    creatureSize: Math.min(Style.space(280), parent.width * 0.3)
    headline: root.applying ? "正在深度优化系统" : (root.scanning ? "检查维护任务" : "系统优化")
    subline: root.applying
      ? ("已完成 " + root.doneCount + " / " + Math.max(root.readyCount, 1))
      : (root.readyCount + " 项就绪 · 用户态维护，不碰内核")
    foreground: root.foreground
    fontFamily: root.fontFamily
  }

  Rectangle {
    visible: root.applying && root.logLines.length
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: hero.bottom
    width: Math.min(parent.width * 0.55, Style.space(420))
    height: Math.min(Style.space(160), logCol.implicitHeight + Style.space(24))
    radius: Style.cornerRadius
    color: Qt.rgba(0, 0, 0, 0.28)

    Column {
      id: logCol
      anchors.fill: parent
      anchors.margins: Style.spacing.md
      spacing: 4
      Repeater {
        model: root.logLines
        delegate: Text {
          required property var modelData
          text: "✓  " + modelData
          color: root.foreground
          opacity: 0.8
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
        }
      }
    }
  }

  ListView {
    visible: !root.applying
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: hero.bottom
    anchors.bottom: footer.top
    anchors.bottomMargin: Style.spacing.md
    clip: true
    model: root.tasks
    spacing: 2
    delegate: Rectangle {
      required property var modelData
      width: ListView.view ? ListView.view.width : 0
      height: Style.space(36)
      radius: Style.cornerRadius
      color: mouse.containsMouse ? Qt.rgba(1, 1, 1, 0.05) : "transparent"
      opacity: modelData.status === "ready" ? 1 : 0.45

      MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        enabled: modelData.status === "ready"
        onClicked: root.toggleId(modelData.id)
      }

      Row {
        anchors.fill: parent
        anchors.leftMargin: Style.spacing.md
        anchors.rightMargin: Style.spacing.md
        spacing: Style.spacing.md
        CheckGlyph {
          anchors.verticalCenter: parent.verticalCenter
          checkState: modelData.status === "ready" && root.selected[modelData.id] ? "all" : "none"
          interactive: false
          opacity: modelData.status === "ready" ? 1 : 0.4
          foreground: root.foreground
        }
        Text {
          width: parent.width - Style.space(160)
          anchors.verticalCenter: parent.verticalCenter
          text: modelData.label
          elide: Text.ElideRight
          color: root.foreground
          font.family: root.fontFamily
        }
        Text {
          anchors.verticalCenter: parent.verticalCenter
          text: modelData.detail
          color: root.foreground
          opacity: 0.5
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
        }
      }
    }
  }

  Row {
    id: footer
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.bottom: parent.bottom
    spacing: Style.spacing.md
    Button {
      text: "刷新"
      enabled: !root.scanning && !root.applying
      onClicked: root.scanRequested()
    }
    Button {
      text: root.applying ? "优化中…" : "开始优化"
      selected: true
      enabled: !root.applying && root.readyCount > 0
      onClicked: root.applyRequested()
    }
  }
}
