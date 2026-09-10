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
  property string phase: "idle"
  property var lastOptimizeAt: 0
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
  signal resetRequested()

  readonly property bool stageApply: root.applying || root.phase === "applying"
  readonly property bool stageDone: root.phase === "done" && !root.applying
  readonly property bool holdLanding: root.scanning && root.tasks.length === 0 && !root.stageApply && !root.stageDone
  readonly property bool heroCentered: root.holdLanding || root.stageApply || root.stageDone
  property bool heroMotion: false

  Component.onCompleted: Qt.callLater(function() { root.heroMotion = true })

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
    visible: true
    z: 4
    anchors.horizontalCenter: parent.horizontalCenter
    width: parent.width
    y: root.heroCentered
      ? App.landingHeroY(parent.height, implicitHeight, Style.space(24))
      : Style.space(12)
    mood: "idle"
    drawField: false
    etch: false
    stamps: false
    sweep: root.scanning || root.stageApply
    interactive: !root.scanning && !root.stageApply
    animateMark: root.heroMotion
    showCaption: !root.holdLanding
    markScale: root.heroCentered ? 1.0 : App.CONTENT_MARK_SCALE
    Behavior on y {
      enabled: root.heroMotion
      NumberAnimation { duration: 560; easing.type: Easing.InOutCubic }
    }
    creatureSize: root.heroCentered
      ? Math.min(Style.space(220), parent.width * 0.22)
      : Style.space(96)
    Behavior on creatureSize {
      enabled: root.heroMotion
      NumberAnimation { duration: 560; easing.type: Easing.InOutCubic }
    }
    headline: {
      if (root.holdLanding) return ""
      if (root.stageApply) return tr("opt.applying")
      if (root.stageDone) return tr("opt.done")
      return tr("opt.title")
    }
    subline: {
      if (root.holdLanding) return ""
      if (root.stageApply)
        return tr("opt.progress", { n: root.doneCount, t: Math.max(root.readyCount, root.logLines.length, 1) })
      if (root.stageDone) {
        return tr("opt.doneSub", {
          n: Math.max(root.doneCount, root.logLines.length),
          when: App.relativeWhen(root.lastOptimizeAt || Date.now(), function(k, v) { return tr(k, v) })
        })
      }
      var when = App.relativeWhen(root.lastOptimizeAt, function(k, v) { return tr(k, v) })
      var due = App.optimizeDue(root.lastOptimizeAt, root.tasks)
      var n = root.readyCount
      if (due === "never") return tr("opt.lastNever", { n: n })
      if (due === "fresh") return tr("opt.lastFresh", { when: when, n: n })
      if (due === "journal") return tr("opt.lastJournal", { when: when })
      if (due === "stale") return tr("opt.lastDue", { when: when, n: n })
      return tr("opt.lastOk", { when: when, n: n })
    }
    foreground: root.foreground
    fontFamily: root.fontFamily
    uiLang: root.uiLang
    accent: root.accent
    urgent: root.urgent
  }

  ListView {
    visible: root.stageApply && root.logLines.length
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: hero.bottom
    anchors.topMargin: Style.spacing.md
    anchors.bottom: parent.bottom
    anchors.bottomMargin: Style.spacing.md
    clip: true
    spacing: 2
    boundsBehavior: Flickable.StopAtBounds
    model: root.logLines
    onCountChanged: if (count > 0) positionViewAtEnd()
    delegate: Text {
      required property var modelData
      width: ListView.view ? ListView.view.width : 0
      leftPadding: Style.spacing.md
      rightPadding: Style.spacing.md
      height: Style.space(28)
      verticalAlignment: Text.AlignVCenter
      text: modelData
      color: root.foreground
      opacity: 0.8
      elide: Text.ElideRight
      font.family: root.fontFamily
      font.pixelSize: Style.font.caption
    }
  }

  ListView {
    visible: root.stageDone && root.logLines.length
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: doneBack.bottom
    anchors.topMargin: Style.spacing.md
    anchors.bottom: parent.bottom
    clip: true
    spacing: 2
    boundsBehavior: Flickable.StopAtBounds
    model: root.logLines
    delegate: Text {
      required property var modelData
      width: ListView.view ? ListView.view.width : 0
      leftPadding: Style.spacing.md
      rightPadding: Style.spacing.md
      height: Style.space(28)
      verticalAlignment: Text.AlignVCenter
      text: modelData
      color: root.foreground
      opacity: 0.8
      elide: Text.ElideRight
      font.family: root.fontFamily
      font.pixelSize: Style.font.caption
    }
  }

  Button {
    id: doneBack
    z: 5
    visible: root.stageDone
    anchors.horizontalCenter: hero.horizontalCenter
    anchors.top: hero.bottom
    anchors.topMargin: Style.space(32)
    text: tr("opt.back")
    selected: true
    onClicked: root.resetRequested()
  }

  ListView {
    visible: !root.holdLanding && !root.stageApply && !root.stageDone
    opacity: root.holdLanding ? 0 : 1
    enabled: visible
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
          text: App.optimizeLabel(modelData, function(k, v) { return tr(k, v) })
          elide: Text.ElideRight
          color: root.foreground
          font.family: root.fontFamily
        }
        Text {
          anchors.verticalCenter: parent.verticalCenter
          text: App.optimizeDetail(modelData, function(k, v) { return tr(k, v) })
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
    visible: !root.holdLanding && !root.stageApply && !root.stageDone
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.bottom: parent.bottom
    spacing: Style.spacing.md
    Button {
      text: tr("opt.refresh")
      enabled: !root.scanning && !root.applying
      onClicked: root.scanRequested()
    }
    Button {
      text: tr("opt.start")
      selected: true
      enabled: !root.applying && root.readyCount > 0
      onClicked: root.applyRequested()
    }
  }
}
