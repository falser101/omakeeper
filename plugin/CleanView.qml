import QtQuick
import QtQuick.Controls
import Quickshell
import qs.Commons
import qs.Ui
import "App.js" as App
import "I18n.js" as I18n

Item {
  id: root
  property var items: []
  property var selected: ({})
  property var expanded: ({})
  property int revision: 0
  property bool scanning: false
  property bool applying: false
  property var totals: ({ cleaned: 0, uninstalled: 0, optimized: 0 })
  property color foreground: Color.menu.text
  property color accent: Color.accent
  property color urgent: Color.urgent
  property color pageBg: Color.menu.background

  // Opaque enough that the edge pixel-field cannot punch through labels.
  function cardFill(hover, header) {
    var t = hover ? (header ? 0.14 : 0.11) : (header ? 0.09 : 0.07)
    var bg = root.pageBg
    var fg = root.foreground
    return Qt.rgba(bg.r * (1 - t) + fg.r * t, bg.g * (1 - t) + fg.g * t, bg.b * (1 - t) + fg.b * t, 1)
  }
  property string fontFamily: Style.font.menuFamily
  property string uiLang: "en"
  function tr(key, vars) { return I18n.t(key, vars, root.uiLang) }
  property var appLibrary: null
  property var desktopIndex: ({})
  property real freedNow: 0
  property string phase: "idle" // idle | review | applying | done
  property var progressLog: []
  property var progressCurrent: ({})
  property int progressIndex: 0
  property int progressTotal: 0
  property int skipped: 0
  property real elapsedMs: 0

  signal toggleId(string id)
  signal toggleAll()
  signal toggleCategorySelect(string category)
  signal toggleIds(var ids)
  signal setAllSelected(bool on)
  signal revealPath(string path)
  signal scanRequested()
  signal applyRequested()
  signal resetRequested()

  readonly property string homeDir: {
    try { return String(Quickshell.env("HOME") || "") } catch (e) { return "" }
  }

  readonly property bool idle: !root.scanning && !root.applying && !root.items.length && root.phase !== "done" && root.phase !== "applying"
  readonly property bool stageApply: root.applying || root.phase === "applying"
  readonly property bool stageDone: root.phase === "done" && !root.applying
  readonly property var rows: {
    var _ = root.revision
    return App.groupCleanRows(root.items, root.expanded)
  }
  readonly property real bytes: {
    var _ = root.revision
    return App.selectedBytes(root.items, root.selected)
  }
  readonly property int count: {
    var _ = root.revision
    return App.selectedCount(root.items, root.selected)
  }

  onItemsChanged: root.expanded = ({})

  function toggleCategory(cat) {
    var n = {}
    for (var k in root.expanded) n[k] = root.expanded[k]
    n[cat] = !n[cat]
    root.expanded = n
  }

  PixelField {
    id: hero
    visible: !root.stageApply && !root.stageDone
    anchors.horizontalCenter: parent.horizontalCenter
    width: parent.width
    y: (root.idle || root.scanning)
      ? Math.max(Style.space(24), (footer.y - implicitHeight) / 2)
      : Style.space(12)
    mood: root.scanning ? "busy" : "idle"
    drawField: false
    etch: root.idle
    stamps: false
    interactive: true
    markScale: (root.items.length && !root.scanning) ? App.CONTENT_MARK_SCALE : 1.0
    Behavior on y {
      NumberAnimation { duration: 280; easing.type: Easing.OutCubic }
    }
    creatureSize: root.idle
      ? Math.min(Style.space(220), parent.width * 0.22)
      : Style.space(96)
    headline: {
      if (root.scanning) return tr("clean.scanning")
      if (root.applying) return tr("clean.applying")
      if (root.phase === "done" && root.idle) return App.formatBytes(root.freedNow)
      if (root.items.length && root.count) return App.formatBytes(root.bytes)
      if (root.items.length) return App.formatBytes(App.allBytes(root.items))
      return tr("clean.ready")
    }
    subline: {
      if (root.scanning) return tr("clean.scanningHint")
      if (root.applying) return tr("clean.applyingHint")
      if (root.phase === "done" && root.idle) return tr("clean.doneHint", { bytes: App.formatBytes(root.totals.cleaned) })
      if (root.items.length)
        return tr("clean.reviewHint", { count: root.count, total: root.items.length })
      return tr("clean.scanHint")
    }
    foreground: root.foreground
    fontFamily: root.fontFamily
    uiLang: root.uiLang
    accent: root.accent
    urgent: root.urgent
  }

  ListView {
    id: list
    visible: root.items.length > 0 && !root.stageApply && !root.stageDone
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: hero.bottom
    anchors.topMargin: Style.spacing.lg
    anchors.bottom: footer.top
    anchors.bottomMargin: Style.spacing.md
    clip: true
    spacing: Style.spacing.sm
    boundsBehavior: Flickable.StopAtBounds
    model: root.rows.length
    delegate: Rectangle {
      required property int index
      readonly property var modelData: root.rows[index] || ({ kind: "header", category: "", bytes: 0, count: 0, open: false })
      readonly property var catCheck: {
        var _ = root.revision
        if (modelData.kind !== "header")
          return ({ state: "none", selectedBytes: 0, bytes: 0, picked: 0, selectable: 0 })
        return App.categoryCheck(root.items, root.selected, modelData.category)
      }
      readonly property var catMeta: App.cleanCategoryMeta(modelData.category)
      width: list.width
      height: modelData.kind === "header" ? Style.space(64) : Style.space(48)
      readonly property var rowCheck: {
        var _ = root.revision
        if (modelData.kind === "header") return catCheck
        if (modelData.kind === "group")
          return App.idsCheck(root.items, root.selected, modelData.childIds || [])
        return ({
          state: modelData.skip_reason ? "none" : (root.selected[modelData.id] ? "all" : "none"),
          selectedBytes: 0
        })
      }
      radius: modelData.kind === "header" ? Math.max(Style.cornerRadius, Style.space(8)) : Style.cornerRadius
      color: root.cardFill(mouse.containsMouse, modelData.kind === "header")
      opacity: modelData.skip_reason ? 0.45 : 1

      MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        enabled: modelData.kind === "header" || !modelData.skip_reason
        onClicked: {
          if (modelData.kind === "header" || modelData.kind === "group")
            root.toggleCategory(modelData.kind === "header" ? modelData.category : modelData.id)
          else
            root.toggleId(modelData.id)
        }
      }

      Item {
        visible: modelData.kind === "header"
        anchors.fill: parent
        anchors.leftMargin: Style.spacing.md
        anchors.rightMargin: Style.spacing.md

        CheckGlyph {
          id: catBox
          z: 2
          anchors.left: parent.left
          anchors.verticalCenter: parent.verticalCenter
          checkState: catCheck.state
          foreground: root.foreground
          onClicked: root.toggleCategorySelect(modelData.category)
        }
        Image {
          id: catGlyph
          anchors.left: catBox.right
          anchors.leftMargin: Style.space(10)
          anchors.verticalCenter: parent.verticalCenter
          width: Style.space(22)
          height: Style.space(22)
          source: Qt.resolvedUrl("assets/" + catMeta.asset)
          fillMode: Image.PreserveAspectFit
          smooth: true
          asynchronous: true
          cache: true
        }
        Column {
          anchors.left: catGlyph.right
          anchors.leftMargin: Style.space(12)
          anchors.right: catSelected.left
          anchors.rightMargin: Style.spacing.md
          anchors.verticalCenter: parent.verticalCenter
          spacing: 2
          Text {
            width: parent.width
            text: tr("cat." + catMeta.key + ".title") + "  "
              + tr("cat.selected", { picked: catCheck.picked, total: catCheck.selectable })
            elide: Text.ElideRight
            color: root.foreground
            font.family: root.fontFamily
            font.pixelSize: Style.font.body
            font.bold: true
          }
          Text {
            width: parent.width
            text: tr("cat." + catMeta.key + ".hint")
            elide: Text.ElideRight
            color: root.foreground
            opacity: 0.45
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }
        }
        Text {
          id: catChevron
          anchors.right: parent.right
          anchors.verticalCenter: parent.verticalCenter
          width: Style.space(22)
          horizontalAlignment: Text.AlignRight
          text: modelData.open ? "⌄" : "›"
          color: root.foreground
          opacity: 0.45
          font.pixelSize: Style.font.body
        }
        Text {
          id: catTotal
          anchors.right: catChevron.left
          anchors.rightMargin: Style.spacing.sm
          anchors.verticalCenter: parent.verticalCenter
          text: " / " + App.formatBytes(modelData.bytes)
          color: root.foreground
          opacity: 0.45
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
        }
        Text {
          id: catSelected
          anchors.right: catTotal.left
          anchors.verticalCenter: parent.verticalCenter
          text: App.formatBytes(catCheck.selectedBytes)
          color: root.foreground
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
        }
      }

      Item {
        visible: modelData.kind === "item" || modelData.kind === "group" || modelData.kind === "child"
        anchors.fill: parent
        anchors.leftMargin: Style.spacing.md + Style.space(16) + Style.spacing.sm
          + (modelData.kind === "child" ? Style.space(18) : 0)
        anchors.rightMargin: Style.spacing.md

        CheckGlyph {
          id: itemBox
          z: 2
          anchors.left: parent.left
          anchors.verticalCenter: parent.verticalCenter
          checkState: rowCheck.state
          interactive: !modelData.skip_reason
          opacity: modelData.skip_reason ? 0.4 : 1
          foreground: root.foreground
          onClicked: {
            if (modelData.kind === "group")
              root.toggleIds(modelData.childIds)
            else
              root.toggleId(modelData.id)
          }
        }
        Text {
          id: itemChevron
          visible: modelData.kind === "group"
          anchors.left: itemBox.right
          anchors.leftMargin: Style.spacing.sm
          anchors.verticalCenter: parent.verticalCenter
          width: visible ? Style.space(14) : 0
          text: modelData.open ? "⌄" : "›"
          color: root.foreground
          opacity: 0.45
          font.pixelSize: Style.font.body
        }
        Column {
          anchors.left: itemChevron.visible ? itemChevron.right : itemBox.right
          anchors.leftMargin: Style.spacing.sm
          anchors.right: itemCount.left
          anchors.rightMargin: Style.spacing.sm
          anchors.verticalCenter: parent.verticalCenter
          spacing: 1
          Text {
            width: parent.width
            text: modelData.label + (modelData.skip_reason ? "  (" + modelData.skip_reason + ")" : "")
            elide: Text.ElideRight
            color: root.foreground
            font.family: root.fontFamily
            font.pixelSize: Style.font.body
          }
          Text {
            width: parent.width
            visible: !!modelData.path
            text: App.shortPath(modelData.path, root.homeDir)
            elide: Text.ElideMiddle
            color: root.foreground
            opacity: 0.4
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }
        }
        Text {
          id: itemCount
          visible: modelData.kind === "group"
          width: visible ? implicitWidth : 0
          anchors.right: itemSize.left
          anchors.rightMargin: Style.spacing.md
          anchors.verticalCenter: parent.verticalCenter
          text: tr("clean.items", { n: modelData.count })
          color: root.foreground
          opacity: 0.45
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
        }
        Text {
          id: itemSize
          anchors.right: itemReveal.left
          anchors.rightMargin: Style.spacing.sm
          anchors.verticalCenter: parent.verticalCenter
          text: App.formatBytes(modelData.bytes)
          color: root.foreground
          opacity: 0.7
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
        }
        Rectangle {
          id: itemReveal
          z: 3
          anchors.right: parent.right
          anchors.verticalCenter: parent.verticalCenter
          width: Style.space(26)
          height: Style.space(26)
          radius: width / 2
          color: revealMouse.containsMouse ? Util.alpha(root.foreground, 0.10) : "transparent"
          border.width: 1
          border.color: Util.alpha(root.foreground, 0.22)
          Image {
            anchors.fill: parent
            anchors.margins: Style.space(5)
            source: Qt.resolvedUrl("assets/cat-folder.png")
            fillMode: Image.PreserveAspectFit
            smooth: true
          }
          MouseArea {
            id: revealMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: {
              mouse.accepted = true
              root.revealPath(modelData.path)
            }
          }
        }
      }
    }
  }

  Item {
    visible: root.stageApply
    anchors.fill: parent

    PixelField {
      id: applyHero
      anchors.horizontalCenter: parent.horizontalCenter
      y: Style.space(24)
      width: parent.width
      mood: "busy"
      drawField: false
      creatureSize: Math.min(Style.space(180), parent.width * 0.22)
      headline: App.formatBytes(root.freedNow)
      subline: {
        var cur = root.progressCurrent || ({})
        var name = cur.label || tr("clean.applying")
        var n = Math.max(1, root.progressIndex)
        var t = Math.max(root.progressTotal, 1)
        return tr("clean.sprint", { name: name, n: n, t: t })
      }
      foreground: root.foreground
      fontFamily: root.fontFamily
    }

    ListView {
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.top: applyHero.bottom
      anchors.topMargin: Style.spacing.md
      anchors.bottom: parent.bottom
      anchors.bottomMargin: Style.spacing.md
      clip: true
      spacing: 2
      boundsBehavior: Flickable.StopAtBounds
      model: {
        var _ = root.revision
        return root.progressLog.length
      }
      onCountChanged: if (count > 0) positionViewAtEnd()
      delegate: Rectangle {
        required property int index
        readonly property var modelData: root.progressLog[index] || ({})
        width: ListView.view ? ListView.view.width : 0
        height: Style.space(28)
        color: "transparent"
        Row {
          anchors.fill: parent
          anchors.leftMargin: Style.spacing.md
          anchors.rightMargin: Style.spacing.md
          spacing: Style.spacing.sm
          Text {
            width: Style.space(18)
            anchors.verticalCenter: parent.verticalCenter
            text: modelData.ok === false ? "✕" : "✓"
            color: modelData.ok === false ? root.urgent : root.foreground
            opacity: 0.8
          }
          Text {
            width: Math.max(40, parent.width - Style.space(120))
            anchors.verticalCenter: parent.verticalCenter
            text: modelData.label || ""
            elide: Text.ElideRight
            color: root.foreground
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }
          Text {
            anchors.verticalCenter: parent.verticalCenter
            text: App.formatBytes(modelData.bytes)
            color: root.foreground
            opacity: 0.45
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }
        }
      }
    }
  }

  Item {
    visible: root.stageDone
    anchors.fill: parent

    PixelField {
      id: doneHero
      anchors.horizontalCenter: parent.horizontalCenter
      anchors.verticalCenter: parent.verticalCenter
      anchors.verticalCenterOffset: -Style.space(24)
      width: parent.width
      mood: "celebrate"
      drawField: false
      creatureSize: Math.min(Style.space(200), parent.width * 0.22)
      headline: App.formatBytes(root.freedNow)
      subline: tr("clean.doneSub", {
        mins: App.fourKMinutes(root.freedNow),
        skipped: root.skipped,
        bytes: App.formatBytes(root.totals.cleaned)
      })
      foreground: root.foreground
      fontFamily: root.fontFamily
    }

    Button {
      anchors.horizontalCenter: parent.horizontalCenter
      anchors.top: doneHero.bottom
      anchors.topMargin: Style.spacing.md
      text: tr("clean.back")
      selected: true
      onClicked: root.resetRequested()
    }
  }

  Item {
    id: footer
    visible: !root.stageApply && !root.stageDone
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.bottom: parent.bottom
    height: Style.space(44)

    Button {
      visible: root.idle || root.scanning
      anchors.horizontalCenter: parent.horizontalCenter
      anchors.verticalCenter: parent.verticalCenter
      text: root.scanning ? tr("clean.scanningBtn") : tr("clean.scan")
      enabled: !root.scanning && !root.applying
      selected: true
      onClicked: root.scanRequested()
    }

    Row {
      visible: root.items.length > 0 && !root.scanning
      anchors.left: parent.left
      anchors.verticalCenter: parent.verticalCenter
      spacing: Style.spacing.md

      Text {
        anchors.verticalCenter: parent.verticalCenter
        text: tr("clean.selectedCount", { n: root.count })
        color: root.foreground
        font.family: root.fontFamily
        font.pixelSize: Style.font.body
      }
      Text {
        anchors.verticalCenter: parent.verticalCenter
        text: tr("clean.selectAll")
        color: root.foreground
        opacity: linkAll.containsMouse ? 1 : 0.55
        font.family: root.fontFamily
        font.pixelSize: Style.font.body
        font.underline: linkAll.containsMouse
        MouseArea {
          id: linkAll
          anchors.fill: parent
          hoverEnabled: true
          cursorShape: Qt.PointingHandCursor
          onClicked: root.setAllSelected(true)
        }
      }
      Text {
        anchors.verticalCenter: parent.verticalCenter
        text: tr("clean.selectNone")
        color: root.foreground
        opacity: linkNone.containsMouse ? 1 : 0.55
        font.family: root.fontFamily
        font.pixelSize: Style.font.body
        font.underline: linkNone.containsMouse
        MouseArea {
          id: linkNone
          anchors.fill: parent
          hoverEnabled: true
          cursorShape: Qt.PointingHandCursor
          onClicked: root.setAllSelected(false)
        }
      }
      Text {
        anchors.verticalCenter: parent.verticalCenter
        text: tr("clean.rescan")
        color: root.foreground
        opacity: linkScan.containsMouse ? 1 : 0.45
        font.family: root.fontFamily
        font.pixelSize: Style.font.caption
        MouseArea {
          id: linkScan
          anchors.fill: parent
          hoverEnabled: true
          cursorShape: Qt.PointingHandCursor
          enabled: !root.applying
          onClicked: root.scanRequested()
        }
      }
    }

    Button {
      visible: root.items.length > 0 && !root.scanning
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      text: root.applying ? tr("clean.applyingBtn") : tr("clean.apply", { bytes: App.formatBytes(root.bytes) })
      enabled: !root.applying && root.count > 0
      selected: true
      onClicked: root.applyRequested()
    }
  }
}
