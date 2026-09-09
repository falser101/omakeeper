import QtQuick
import qs.Commons
import qs.Ui
import "App.js" as App
import "I18n.js" as I18n

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
  property color accent: Color.accent
  property color urgent: Color.urgent
  property color pageBg: Color.menu.background
  property string fontFamily: Style.font.menuFamily
  property string uiLang: "en"
  function tr(key, vars) { return I18n.t(key, vars, root.uiLang) }

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

  PixelField {
    id: hero
    anchors.horizontalCenter: parent.horizontalCenter
    y: Style.space(12)
    width: parent.width
    mood: (root.applying || root.scanning) ? "busy" : "idle"
    drawField: false
    markScale: App.CONTENT_MARK_SCALE
    creatureSize: Style.space(96)
    headline: root.applying ? tr("opt.applying") : (root.scanning ? tr("opt.scanning") : tr("opt.title"))
    subline: root.applying
      ? tr("opt.progress", { n: root.doneCount, t: Math.max(root.readyCount, 1) })
      : tr("opt.readyHint", { n: root.readyCount })
    foreground: root.foreground
    fontFamily: root.fontFamily
    uiLang: root.uiLang
    accent: root.accent
    urgent: root.urgent
  }

  Rectangle {
    visible: root.applying && root.logLines.length
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: hero.bottom
    width: Math.min(parent.width * 0.55, Style.space(420))
    height: Math.min(Style.space(160), logCol.implicitHeight + Style.space(24))
    radius: Style.cornerRadius
    color: Util.alpha(root.pageBg, 0.55)

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
      color: mouse.containsMouse ? Util.alpha(root.foreground, 0.07) : "transparent"
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
      text: tr("opt.refresh")
      enabled: !root.scanning && !root.applying
      onClicked: root.scanRequested()
    }
    Button {
      text: root.applying ? tr("opt.running") : tr("opt.start")
      selected: true
      enabled: !root.applying && root.readyCount > 0
      onClicked: root.applyRequested()
    }
  }
}
