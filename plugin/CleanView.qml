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
  property bool playSweep: false
  property bool sweepDone: false
  property bool heroMotion: false
  readonly property bool holdLanding: root.idle || root.scanning
  readonly property bool heroCentered: root.holdLanding || root.stageApply || root.stageDone

  Component.onCompleted: Qt.callLater(function() { root.heroMotion = true })

  function beginScan() {
    root.sweepDone = false
    root.playSweep = true
    hero.restartSweep()
    root.scanRequested()
  }

  onScanningChanged: {
    if (root.scanning)
      root.playSweep = true
    else
      root.playSweep = false
  }
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
  property int pinIndex: -1

  function toggleCategory(cat) {
    var opening = !root.expanded[cat]
    var idx = -1
    var rows = root.rows
    for (var i = 0; i < rows.length; i++) {
      if ((rows[i].kind === "header" && rows[i].category === cat) || rows[i].id === cat) {
        idx = i
        break
      }
    }
    var n = {}
    for (var k in root.expanded) n[k] = root.expanded[k]
    n[cat] = !n[cat]
    root.expanded = n
    root.pinIndex = opening ? idx : -1
    if (opening)
      Qt.callLater(root.revealPinnedRow)
  }

  function revealPinnedRow() {
    if (!list.visible || root.pinIndex < 0)
      return
    var idx = root.pinIndex
    var item = rowRepeater.itemAt(idx)
    if (!item)
      return
    root.pinIndex = -1
    var endY = item.y + item.height
    var rows = root.rows
    for (var i = idx + 1; i < rows.length; i++) {
      if (rows[i].kind === "header" || rows[i].kind === "group")
        break
      var child = rowRepeater.itemAt(i)
      if (child)
        endY = child.y + child.height
    }
    var viewBottom = list.contentY + list.height
    if (endY <= viewBottom - 8)
      return
    var maxY = Math.max(0, list.contentHeight - list.height)
    list.contentY = Math.max(0, Math.min(item.y, maxY))
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
    etch: root.idle && !root.playSweep && !root.stageApply
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
      if (root.scanning) return ""
      if (root.stageApply) return App.formatBytes(root.freedNow)
      if (root.stageDone) return App.formatBytes(root.freedNow)
      if (root.items.length && root.count) return App.formatBytes(root.bytes)
      if (root.items.length) return App.formatBytes(App.allBytes(root.items))
      return ""
    }
    subline: {
      if (root.scanning) return ""
      if (root.stageApply) {
        var cur = root.progressCurrent || ({})
        var name = App.cleanItemLabel(cur, function(k, v) { return tr(k, v) }, root.uiLang) || tr("clean.applying")
        return tr("clean.sprint", {
          name: name,
          n: Math.max(1, root.progressIndex),
          t: Math.max(root.progressTotal, 1)
        })
      }
      if (root.stageDone) {
        var n = Math.max(root.progressLog.length, root.progressTotal)
        var vars = {
          n: n,
          skipped: root.skipped,
          total: App.formatBytes(root.totals.cleaned)
        }
        if (root.skipped > 0)
          return tr("clean.doneSubFail", vars)
        return tr("clean.doneSub", vars)
      }
      if (root.items.length)
        return tr("clean.reviewHint", { count: root.count, total: root.items.length })
      return ""
    }
    foreground: root.foreground
    fontFamily: root.fontFamily
    uiLang: root.uiLang
    accent: root.accent
    urgent: root.urgent
    onSweepCycled: {
      root.sweepDone = true
      if (!root.scanning && !root.stageApply)
        root.playSweep = false
    }
  }

  Button {
    id: scanBtn
    z: 5
    visible: root.holdLanding && !root.stageApply && !root.stageDone
    anchors.horizontalCenter: hero.horizontalCenter
    anchors.top: hero.bottom
    anchors.topMargin: Style.space(32)
    text: (root.scanning || root.playSweep) ? tr("clean.scanningBtn") : tr("clean.scan")
    enabled: !root.scanning && !root.playSweep && !root.applying
    selected: true
    onClicked: root.beginScan()
  }

  Flickable {
    id: list
    visible: root.items.length > 0 && !root.stageApply && !root.stageDone
    opacity: root.holdLanding ? 0 : 1
    enabled: !root.holdLanding && visible
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: hero.bottom
    anchors.topMargin: Style.spacing.lg
    anchors.bottom: footer.top
    anchors.bottomMargin: Style.spacing.md
    clip: true
    Behavior on opacity {
      enabled: root.heroMotion
      NumberAnimation { duration: 420; easing.type: Easing.OutCubic }
    }
    boundsBehavior: Flickable.StopAtBounds
    contentWidth: width
    contentHeight: col.height
    onContentHeightChanged: {
      var maxY = Math.max(0, contentHeight - height)
      if (contentY > maxY)
        contentY = maxY
      if (root.pinIndex >= 0)
        Qt.callLater(root.revealPinnedRow)
    }

    Column {
      id: col
      width: list.width
      spacing: Style.spacing.sm

      Repeater {
        id: rowRepeater
        model: root.rows.length
        Rectangle {
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
        AppIcon {
          id: agentIcon
          readonly property bool isAiGroup: modelData.kind === "group" && !!modelData.group
          readonly property string resolvedIcon: {
            var _ = root.desktopIndex
            if (agentIcon.isAiGroup) {
              var n = App.aiGroupIcon(modelData.group, root.desktopIndex)
              if (n.indexOf("assets/") === 0)
                return Qt.resolvedUrl(n)
              return n
            }
            if (modelData.category === "apps" && modelData.kind !== "child")
              return App.lookupAppIcon(modelData, root.desktopIndex)
            return ""
          }
          visible: agentIcon.isAiGroup || (modelData.category === "apps" && modelData.kind !== "child" && !!resolvedIcon)
          anchors.left: itemBox.right
          anchors.leftMargin: visible ? Style.space(12) : 0
          anchors.verticalCenter: parent.verticalCenter
          width: visible ? Style.space(22) : 0
          height: Style.space(22)
          pixelSize: Style.space(22)
          iconName: resolvedIcon
          showFallback: agentIcon.isAiGroup
          fallbackText: {
            var _ = root.uiLang
            return App.cleanRowLabel(modelData, function(k, v) { return tr(k, v) }, root.uiLang)
          }
          appLibrary: root.appLibrary
          foreground: root.foreground
        }
        Column {
          anchors.left: agentIcon.visible ? agentIcon.right : itemBox.right
          anchors.leftMargin: Style.space(12)
          anchors.right: itemCount.left
          anchors.rightMargin: Style.spacing.sm
          anchors.verticalCenter: parent.verticalCenter
          spacing: 1
          Text {
            width: parent.width
            text: {
              var _ = root.uiLang
              var name = App.cleanRowLabel(modelData, function(k, v) { return tr(k, v) }, root.uiLang)
              var skip = App.cleanSkipReason(modelData.skip_reason, function(k, v) { return tr(k, v) })
              return name + (skip ? "  (" + skip + ")" : "")
            }
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
          anchors.right: itemReveal.visible ? itemReveal.left : (itemChevron.visible ? itemChevron.left : parent.right)
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
          visible: !!modelData.path
          anchors.right: itemChevron.visible ? itemChevron.left : parent.right
          anchors.rightMargin: itemChevron.visible ? Style.spacing.sm : 0
          anchors.verticalCenter: parent.verticalCenter
          width: visible ? Style.space(26) : 0
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
        Text {
          id: itemChevron
          visible: modelData.kind === "group"
          anchors.right: parent.right
          anchors.verticalCenter: parent.verticalCenter
          width: visible ? Style.space(22) : 0
          horizontalAlignment: Text.AlignRight
          text: modelData.open ? "⌄" : "›"
          color: root.foreground
          opacity: 0.45
          font.pixelSize: Style.font.body
        }
      }
    }
        }
      }
    }

  Item {
    visible: root.stageApply
    enabled: root.stageApply
    anchors.fill: parent

    ListView {
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.top: hero.bottom
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
            text: App.cleanItemLabel(modelData, function(k, v) { return tr(k, v) }, root.uiLang) || modelData.label || ""
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
    enabled: root.stageDone
    anchors.fill: parent

    ListView {
      visible: root.progressLog.length > 0
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.top: doneBack.bottom
      anchors.topMargin: Style.spacing.md
      anchors.bottom: parent.bottom
      anchors.bottomMargin: Style.spacing.md
      clip: true
      spacing: Style.spacing.sm
      boundsBehavior: Flickable.StopAtBounds
      model: root.progressLog.length
      delegate: Rectangle {
        required property int index
        readonly property var modelData: root.progressLog[index] || ({})
        width: ListView.view ? ListView.view.width : 0
        height: Style.space(48)
        radius: Style.cornerRadius
        color: root.cardFill(false, false)
        opacity: modelData.ok === false ? 0.55 : 1

        Text {
          id: doneMark
          anchors.left: parent.left
          anchors.leftMargin: Style.spacing.md
          anchors.verticalCenter: parent.verticalCenter
          width: Style.space(18)
          text: modelData.ok === false ? "✕" : "✓"
          color: modelData.ok === false ? root.urgent : root.foreground
          opacity: 0.8
        }
        Column {
          anchors.left: doneMark.right
          anchors.leftMargin: Style.space(12)
          anchors.right: doneSize.left
          anchors.rightMargin: Style.spacing.sm
          anchors.verticalCenter: parent.verticalCenter
          spacing: 1
          Text {
            width: parent.width
            text: App.cleanItemLabel(modelData, function(k, v) { return tr(k, v) }, root.uiLang) || modelData.label || ""
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
          id: doneSize
          anchors.right: parent.right
          anchors.rightMargin: Style.spacing.md
          anchors.verticalCenter: parent.verticalCenter
          text: App.formatBytes(modelData.bytes)
          color: root.foreground
          opacity: 0.7
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
        }
      }
    }

    Button {
      id: doneBack
      z: 5
      anchors.horizontalCenter: hero.horizontalCenter
      anchors.top: hero.bottom
      anchors.topMargin: Style.space(32)
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
    height: root.holdLanding ? 0 : Style.space(44)
    Behavior on height {
      enabled: root.heroMotion
      NumberAnimation { duration: 420; easing.type: Easing.InOutCubic }
    }

    Row {
      visible: root.items.length > 0 && !root.holdLanding
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
          onClicked: root.beginScan()
        }
      }
    }

    Button {
      visible: root.items.length > 0 && !root.holdLanding
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      text: root.applying ? tr("clean.applyingBtn") : tr("clean.apply", { bytes: App.formatBytes(root.bytes) })
      enabled: !root.applying && root.count > 0
      selected: true
      onClicked: root.applyRequested()
    }
  }
}
