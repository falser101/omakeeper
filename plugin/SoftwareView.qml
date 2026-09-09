import QtQuick
import Quickshell
import qs.Commons
import qs.Ui
import "App.js" as App
import "I18n.js" as I18n

Item {
  id: root
  property var packages: []
  property var selected: ({})
  property var leftovers: ({})
  property var leftoverSelected: ({})
  property var requiredBy: ({})
  property var expanded: ({})
  property int revision: 0
  property string query: ""
  property string subtab: "remove"

  onSubtabChanged: {
    root.addingAutostart = false
    root.autostartQuery = ""
    if (root.subtab === "autostart")
      root.autostartScanRequested()
  }
  property bool scanning: false
  property bool applying: false
  property string phase: "idle" // idle | applying | done
  property real freedNow: 0
  property int removedPkgs: 0
  property int removedFiles: 0
  property color foreground: Color.menu.text
  property color selectedBg: Color.menu.selectedBackground
  property color accent: Color.accent
  property color urgent: Color.urgent
  property color pageBg: Color.menu.background
  property string fontFamily: Style.font.menuFamily
  property string uiLang: "en"
  function tr(key, vars) { return I18n.t(key, vars, root.uiLang) }
  property var appLibrary: null
  property var desktopIndex: ({})
  property var autostartItems: []
  property var autostartAvailable: []
  property bool addingAutostart: false
  property string autostartQuery: ""

  property bool removeSweep: false
  property bool removeSweepDone: false
  property var removedIcons: []
  readonly property bool stageDone: root.phase === "done" && !root.applying && !root.removeSweep
  readonly property bool stageApply: !root.stageDone && (root.applying || root.phase === "applying" || root.removeSweep)
  readonly property bool loading: root.scanning && root.packages.length === 0 && !root.stageApply && !root.stageDone
  property bool playSweep: true
  property bool sweepDone: false
  readonly property bool holdLanding: (root.loading || root.playSweep) && !root.stageApply && !root.stageDone

  function startIntro() {
    if (root.stageApply || root.stageDone)
      return
    root.sweepDone = false
    root.playSweep = true
    Qt.callLater(function() {
      if (hero)
        hero.restartSweep()
    })
  }

  function parkLanding() {
    root.playSweep = true
    root.sweepDone = false
  }

  onVisibleChanged: {
    if (root.visible)
      root.startIntro()
    else
      root.parkLanding()
  }
  onEnabledChanged: {
    if (root.enabled && root.visible)
      root.startIntro()
    else
      root.parkLanding()
  }
  onLoadingChanged: {
    if (!root.loading && root.sweepDone)
      root.playSweep = false
  }
  function snapshotRemovedIcons() {
    var names = root.selectedNames || []
    var out = []
    for (var i = 0; i < names.length && out.length < 3; i++) {
      var name = names[i]
      var icon = App.iconNameForPackage(name, root.desktopIndex)
      for (var j = 0; j < root.packages.length; j++) {
        if (root.packages[j].name === name) {
          if (root.packages[j].icon)
            icon = root.packages[j].icon
          break
        }
      }
      out.push({ name: name, icon: icon || "" })
    }
    root.removedIcons = out
  }

  onApplyingChanged: {
    if (root.applying) {
      root.snapshotRemovedIcons()
      root.removeSweepDone = false
      root.removeSweep = true
      Qt.callLater(function() {
        if (removeHero)
          removeHero.restartSweep()
      })
    } else if (root.removeSweepDone) {
      root.removeSweep = false
    }
  }
  onPhaseChanged: {
    if (root.phase === "idle") {
      root.removeSweep = false
      root.removeSweepDone = false
      root.removedIcons = []
    }
  }

  signal togglePkg(string name)
  signal toggleLeftover(string path)
  signal expandPkg(string name)
  signal applyRequested()
  signal queryChangedByUser(string value)
  signal resetRequested()
  signal autostartScanRequested()
  signal autostartSet(string id, bool enabled)
  signal autostartAdd(string id)
  signal autostartRemove(string id)

  readonly property string homeDir: {
    try { return String(Quickshell.env("HOME") || "") } catch (e) { return "" }
  }

  function displayPath(path) {
    var p = String(path || "")
    var home = root.homeDir
    if (home && p.indexOf(home) === 0)
      return "~" + p.slice(home.length)
    return p
  }

  function cardFill(hover, header) {
    var t = hover ? (header ? 0.14 : 0.11) : (header ? 0.09 : 0.07)
    var bg = root.pageBg
    var fg = root.foreground
    return Qt.rgba(bg.r * (1 - t) + fg.r * t, bg.g * (1 - t) + fg.g * t, bg.b * (1 - t) + fg.b * t, 1)
  }

  function freeDeps(name) {
    var deps = root.requiredBy[name] || []
    var out = []
    for (var i = 0; i < deps.length; i++) {
      if (!deps[i].protected) out.push(deps[i])
    }
    return out
  }

  function parentCheck(name) {
    if (!root.selected[name]) return "none"
    var deps = root.freeDeps(name)
    if (!deps.length) return "all"
    var n = 0
    for (var i = 0; i < deps.length; i++) {
      if (root.selected[deps[i].name]) n++
    }
    if (n === deps.length) return "all"
    return "partial"
  }

  function hasDetails(name) {
    var rb = root.requiredBy[name] || []
    var lo = root.leftovers[name] || []
    return rb.length > 0 || lo.length > 0
  }

  function pkgPick(name) {
    var deps = root.freeDeps(name)
    var picked = root.selected[name] ? 1 : 0
    for (var i = 0; i < deps.length; i++) {
      if (root.selected[deps[i].name]) picked++
    }
    return { picked: picked, total: 1 + deps.length }
  }

  function pkgSelectedBytes(name, bytes) {
    var n = root.selected[name] ? Number(bytes || 0) : 0
    var deps = root.requiredBy[name] || []
    for (var i = 0; i < deps.length; i++) {
      if (!deps[i].protected && root.selected[deps[i].name])
        n += Number(deps[i].bytes || 0)
    }
    var rows = root.leftovers[name] || []
    for (var j = 0; j < rows.length; j++) {
      if (root.leftoverSelected[rows[j].path])
        n += Number(rows[j].bytes || 0)
    }
    return n
  }

  function pkgTotalBytes(name, bytes) {
    var n = Number(bytes || 0)
    var deps = root.freeDeps(name)
    for (var i = 0; i < deps.length; i++)
      n += Number(deps[i].bytes || 0)
    var rows = root.leftovers[name] || []
    for (var j = 0; j < rows.length; j++)
      n += Number(rows[j].bytes || 0)
    return n
  }

  function depHint(name) {
    var deps = root.requiredBy[name] || []
    if (!deps.length || !root.selected[name]) return ""
    var locked = []
    var missing = 0
    for (var i = 0; i < deps.length; i++) {
      if (deps[i].protected) locked.push(deps[i].name)
      else if (!root.selected[deps[i].name]) missing++
    }
    if (locked.length) return tr("soft.lockedDeps", { deps: locked.join(", ") })
    if (missing) return tr("soft.needDeps", { n: missing })
    return ""
  }

  property string selectedLetter: ""
  property bool railLock: false
  property bool railDragging: false
  property bool thumbArmed: false
  property real railPressY: 0

  readonly property var visiblePkgs: {
    var _ = root.revision
    return App.filterPackages(root.packages, root.query)
  }
  readonly property var letterPresent: {
    var _ = root.revision
    var pkgs = root.visiblePkgs
    var present = {}
    for (var i = 0; i < pkgs.length; i++)
      present[App.packageLetter(pkgs[i].name)] = true
    return present
  }
  readonly property var indexLetters: {
    var _ = root.letterPresent
    var out = root.letterPresent["#"] ? ["#"] : []
    for (var c = 0; c < 26; c++) out.push(String.fromCharCode(65 + c))
    return out
  }
  function peekTopIndex() {
    var pkgs = root.visiblePkgs
    if (!list || !pkgs.length) return -1
    var y0 = list.contentY
    var xs = [12, Math.max(12, list.width * 0.25)]
    var ys = [1, 8, 18, 32, 52]
    var i, j, idx
    for (i = 0; i < xs.length; i++) {
      for (j = 0; j < ys.length; j++) {
        idx = list.indexAt(xs[i], y0 + ys[j])
        if (idx >= 0 && idx < pkgs.length)
          return idx
      }
    }
    var kids = list.contentItem ? list.contentItem.children : []
    var best = -1
    var bestY = 1e9
    for (i = 0; i < kids.length; i++) {
      var it = kids[i]
      if (!it || it === list.footerItem) continue
      if (typeof it.index !== "number") continue
      if (it.y + it.height <= y0 + 0.5) continue
      if (it.y < bestY) {
        bestY = it.y
        best = it.index
      }
    }
    if (best >= 0 && best < pkgs.length)
      return best
    return -1
  }

  readonly property string topLetter: {
    var _ = list.contentY
    var __ = list.moving
    var pkgs = root.visiblePkgs
    if (!pkgs.length) return ""
    var idx = root.peekTopIndex()
    if (idx < 0) return root.selectedLetter
    return App.packageLetter(pkgs[idx].name)
  }

  onTopLetterChanged: {
    if (root.railLock || root.railDragging) return
    if (!root.topLetter) return
    var animate = root.thumbArmed && !list.moving && !list.flicking && !listJump.running
    root.setSelectedLetter(root.topLetter, animate)
    root.thumbArmed = true
  }

  function letterIndexOf(letter) {
    var letters = root.indexLetters
    for (var i = 0; i < letters.length; i++) {
      if (letters[i] === letter) return i
    }
    return 0
  }

  function thumbYFor(letter) {
    var slot = letterRail.slotH
    var top = root.letterIndexOf(letter) * slot
    var y = top + (slot - thumb.height) / 2
    var maxY = Math.max(0, letterRail.height - thumb.height)
    if (y < 0) return 0
    if (y > maxY) return maxY
    return y
  }

  function setSelectedLetter(letter, animate) {
    if (!letter) return
    root.selectedLetter = letter
    var dest = root.thumbYFor(letter)
    if (!letterRail.visible) {
      thumb.y = dest
      thumb.scale = 1
      return
    }
    if (!animate) {
      thumbTravel.stop()
      thumb.y = dest
      thumb.scale = 1
      return
    }
    if (Math.abs(thumb.y - dest) < 0.5) return
    thumbTravel.stop()
    thumb.scale = 1
    var dist = Math.abs(dest - thumb.y)
    var ms = Math.max(260, Math.min(540, 200 + dist * 1.2))
    thumbYAnim.from = thumb.y
    thumbYAnim.to = dest
    thumbYAnim.duration = ms
    takeoffAnim.duration = Math.max(70, Math.round(ms * 0.22))
    landAnim.duration = Math.max(140, Math.round(ms * 0.78))
    thumbTravel.start()
  }

  function firstIndexForLetter(letter) {
    var pkgs = root.visiblePkgs
    if (!pkgs.length || !letter) return 0
    var first = -1
    var next = -1
    for (var j = 0; j < pkgs.length; j++) {
      var L = App.packageLetter(pkgs[j].name)
      if (L === letter) {
        if (first < 0) first = j
      } else if (letter !== "#" && L !== "#" && L > letter) {
        if (next < 0) next = j
      }
    }
    if (first >= 0) return first
    if (next >= 0) return next
    return Math.max(0, pkgs.length - 1)
  }

  function jumpToLetter(letter, instant) {
    var pkgs = root.visiblePkgs
    if (!pkgs.length || !letter) return
    var i = root.firstIndexForLetter(letter)
    if (instant) {
      listJump.stop()
      list.forceLayout()
      list.positionViewAtIndex(i, ListView.Beginning)
      root.railLock = false
      return
    }
    list.cancelFlick()
    var from = list.contentY
    list.forceLayout()
    list.positionViewAtIndex(i, ListView.Beginning)
    var to = list.contentY
    if (Math.abs(to - from) < 2) {
      root.railLock = false
      return
    }
    list.contentY = from
    listJump.stop()
    listJump.from = from
    listJump.to = to
    listJump.duration = Math.max(260, Math.min(540, 200 + Math.abs(to - from) * 0.18))
    listJump.start()
  }

  function letterFromY(py) {
    var letters = root.indexLetters
    if (!letters.length || letterRail.height <= 0) return ""
    var i = Math.floor(py / letterRail.height * letters.length)
    if (i < 0) i = 0
    if (i >= letters.length) i = letters.length - 1
    return letters[i]
  }

  function jumpFromY(py, instant) {
    var letter = root.letterFromY(py)
    if (!letter) return
    root.railLock = true
    root.setSelectedLetter(letter, !instant)
    root.jumpToLetter(letter, instant)
  }
  readonly property var selectedNames: {
    var _ = root.revision
    var out = []
    for (var i = 0; i < root.packages.length; i++) {
      if (root.selected[root.packages[i].name]) out.push(root.packages[i].name)
    }
    return out
  }
  readonly property real selectedBytes: {
    var _ = root.revision
    var n = 0
    for (var i = 0; i < root.packages.length; i++) {
      if (root.selected[root.packages[i].name]) n += Number(root.packages[i].bytes || 0)
    }
    for (var name in root.leftovers) {
      if (!root.selected[name]) continue
      var rows = root.leftovers[name] || []
      for (var j = 0; j < rows.length; j++) {
        if (root.leftoverSelected[rows[j].path])
          n += Number(rows[j].bytes || 0)
      }
    }
    return n
  }

  readonly property var visibleAutostart: {
    var _ = root.revision
    var q = String(root.autostartQuery || "").toLowerCase()
    var src = root.addingAutostart ? (root.autostartAvailable || []) : (root.autostartItems || [])
    if (!q) return src
    var out = []
    for (var i = 0; i < src.length; i++) {
      var hay = ((src[i].name || "") + " " + (src[i].description || "") + " " + (src[i].id || "")).toLowerCase()
      if (hay.indexOf(q) >= 0) out.push(src[i])
    }
    return out
  }

  readonly property int autostartOnCount: {
    var _ = root.revision
    var n = 0
    var items = root.autostartItems || []
    for (var i = 0; i < items.length; i++) {
      if (items[i].enabled) n++
    }
    return n
  }

  Row {
    id: subtabs
    z: 2
    visible: !root.holdLanding && !root.stageApply && !root.stageDone
    spacing: Style.spacing.sm
    y: 0

    Repeater {
      model: [
        { id: "remove", label: tr("soft.remove") },
        { id: "autostart", label: tr("soft.autostart") }
      ]
      delegate: Rectangle {
        required property var modelData
        width: label.implicitWidth + Style.space(24)
        height: Style.space(28)
        radius: height / 2
        color: root.subtab === modelData.id ? root.selectedBg : Util.alpha(root.foreground, 0.05)
        Text {
          id: label
          anchors.centerIn: parent
          text: modelData.label
          color: root.foreground
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
          font.bold: root.subtab === modelData.id
        }
        MouseArea {
          anchors.fill: parent
          onClicked: root.subtab = modelData.id
        }
      }
    }
  }

  TextField {
    id: search
    z: 2
    visible: !root.holdLanding && !root.stageApply && !root.stageDone
    anchors.right: parent.right
    width: Style.space(220)
    placeholderText: root.subtab === "autostart"
      ? (root.addingAutostart ? tr("soft.autostartPick") : tr("soft.searchAutostart"))
      : tr("soft.search")
    text: root.subtab === "autostart" ? root.autostartQuery : root.query
    onTextEdited: {
      if (root.subtab === "autostart")
        root.autostartQuery = text
      else
        root.queryChangedByUser(text)
    }
  }

  PixelField {
    id: hero
    visible: !root.stageApply && !root.stageDone
    anchors.horizontalCenter: parent.horizontalCenter
    width: parent.width
    y: root.holdLanding
      ? App.landingHeroY(parent.height, implicitHeight, Style.space(24))
      : Style.space(12)
    mood: "idle"
    drawField: false
    sweep: root.playSweep
    interactive: !root.playSweep
    animateMark: !root.holdLanding
    markScale: root.holdLanding ? 1.0 : App.CONTENT_MARK_SCALE
    creatureSize: root.holdLanding
      ? Math.min(Style.space(220), parent.width * 0.22)
      : Style.space(96)
    showCaption: false
    headline: ""
    subline: ""
    uiLang: root.uiLang
    accent: root.accent
    urgent: root.urgent
    Behavior on y {
      enabled: !root.holdLanding
      NumberAnimation { duration: 280; easing.type: Easing.OutCubic }
    }
    onSweepCycled: {
      root.sweepDone = true
      if (!root.loading)
        root.playSweep = false
    }
  }

  ListView {
    id: list
    visible: !root.holdLanding && root.subtab === "remove" && !root.stageApply && !root.stageDone
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.rightMargin: visible && root.visiblePkgs.length ? Style.space(32) : 0
    anchors.top: hero.bottom
    anchors.topMargin: Style.spacing.lg
    anchors.bottom: footer.top
    anchors.bottomMargin: Style.spacing.md
    clip: true
    spacing: Style.spacing.sm
    model: root.visiblePkgs
    boundsBehavior: Flickable.StopAtBounds
    footer: Item {
      width: list.width
      height: Math.max(0, list.height - Style.space(56))
    }
    cacheBuffer: Math.max(256, list.height * 2)
    onMovementStarted: {
      if (listJump.running) listJump.stop()
      root.railLock = false
    }
    onMovementEnded: {
      if (root.railLock || root.railDragging) return
      if (root.topLetter)
        root.setSelectedLetter(root.topLetter, false)
    }

    delegate: Rectangle {
      required property int index
      required property var modelData
      width: list.width
      height: body.implicitHeight + Style.spacing.md * 2
      radius: Math.max(Style.cornerRadius, Style.space(8))
      color: root.cardFill(mouse.containsMouse, true)

      MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        onClicked: root.expandPkg(modelData.name)
      }

      Column {
        id: body
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: Style.spacing.md
        anchors.rightMargin: Style.spacing.md
        anchors.topMargin: Style.spacing.md
        spacing: Style.spacing.xs

        Item {
          width: parent.width
          height: Style.space(40)

          CheckGlyph {
            id: pkgBox
            z: 2
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            checkState: {
              var _ = root.revision
              return root.parentCheck(modelData.name)
            }
            foreground: root.foreground
            onClicked: root.togglePkg(modelData.name)
          }

          Item {
            id: pkgIcon
            anchors.left: pkgBox.right
            anchors.leftMargin: Style.space(10)
            anchors.verticalCenter: parent.verticalCenter
            width: Style.space(22)
            height: Style.space(22)
            AppIcon {
              anchors.fill: parent
              pixelSize: parent.width
              iconName: modelData.icon || App.iconNameForPackage(modelData.name, root.desktopIndex)
              fallbackText: modelData.name
              appLibrary: root.appLibrary
              foreground: root.foreground
            }
          }

          Column {
            anchors.left: pkgIcon.right
            anchors.leftMargin: Style.space(12)
            anchors.right: pkgSelected.left
            anchors.rightMargin: Style.spacing.md
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2
            Text {
              width: parent.width
              text: {
                var _ = root.revision
                var pick = root.pkgPick(modelData.name)
                return modelData.name + "  " + tr("cat.selected", { picked: pick.picked, total: pick.total })
              }
              elide: Text.ElideRight
              color: root.foreground
              font.family: root.fontFamily
              font.pixelSize: Style.font.body
              font.bold: true
            }
            Text {
              width: parent.width
              text: {
                var _ = root.revision
                var hint = root.depHint(modelData.name)
                if (hint) return hint
                return modelData.description || ""
              }
              elide: Text.ElideRight
              color: root.foreground
              opacity: 0.45
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
            }
          }

          Text {
            id: pkgChevron
            visible: {
              var _ = root.revision
              return root.hasDetails(modelData.name)
            }
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            width: visible ? Style.space(22) : 0
            horizontalAlignment: Text.AlignRight
            text: root.expanded[modelData.name] ? "⌄" : "›"
            color: root.foreground
            opacity: 0.45
            font.pixelSize: Style.font.body
            MouseArea {
              anchors.fill: parent
              onClicked: root.expandPkg(modelData.name)
            }
          }

          Text {
            id: pkgTotal
            anchors.right: pkgChevron.visible ? pkgChevron.left : parent.right
            anchors.rightMargin: pkgChevron.visible ? Style.spacing.sm : 0
            anchors.verticalCenter: parent.verticalCenter
            text: {
              var _ = root.revision
              return " / " + App.formatBytes(root.pkgTotalBytes(modelData.name, modelData.bytes))
            }
            color: root.foreground
            opacity: 0.45
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }

          Text {
            id: pkgSelected
            anchors.right: pkgTotal.left
            anchors.verticalCenter: parent.verticalCenter
            text: {
              var _ = root.revision
              return App.formatBytes(root.pkgSelectedBytes(modelData.name, modelData.bytes))
            }
            color: root.foreground
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }
        }

        Column {
          visible: !!root.expanded[modelData.name]
          width: parent.width
          leftPadding: Style.space(40)
          spacing: Style.spacing.xs

          Text {
            visible: (root.requiredBy[modelData.name] || []).length > 0
            text: tr("soft.requiredBy")
            color: root.foreground
            opacity: 0.45
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }

          Repeater {
            model: root.requiredBy[modelData.name] || []
            delegate: Item {
              required property var modelData
              width: parent.width
              height: Style.space(48)
              opacity: modelData.protected ? 0.45 : 1

              CheckGlyph {
                id: depBox
                z: 2
                anchors.left: parent.left
                anchors.leftMargin: Style.space(16)
                anchors.verticalCenter: parent.verticalCenter
                checkState: {
                  var _ = root.revision
                  return modelData.protected ? "none" : (root.selected[modelData.name] ? "all" : "none")
                }
                interactive: !modelData.protected
                foreground: root.foreground
                onClicked: {
                  if (!modelData.protected)
                    root.togglePkg(modelData.name)
                }
              }
              AppIcon {
                id: depIcon
                anchors.left: depBox.right
                anchors.leftMargin: Style.space(10)
                anchors.verticalCenter: parent.verticalCenter
                width: Style.space(22)
                height: Style.space(22)
                pixelSize: Style.space(22)
                iconName: App.iconNameForPackage(modelData.name, root.desktopIndex)
                fallbackText: modelData.name
                appLibrary: root.appLibrary
                foreground: root.foreground
              }
              Column {
                anchors.left: depIcon.right
                anchors.leftMargin: Style.space(12)
                anchors.right: depSize.left
                anchors.rightMargin: Style.spacing.md
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2
                Text {
                  width: parent.width
                  text: modelData.name
                  elide: Text.ElideRight
                  color: root.foreground
                  font.family: root.fontFamily
                  font.pixelSize: Style.font.body
                }
                Text {
                  width: parent.width
                  visible: modelData.protected || !!modelData.description
                  text: modelData.protected ? tr("soft.protectedDep") : (modelData.description || "")
                  elide: Text.ElideRight
                  color: root.foreground
                  opacity: 0.45
                  font.family: root.fontFamily
                  font.pixelSize: Style.font.caption
                }
              }
              Text {
                id: depSize
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                text: App.formatBytes(modelData.bytes)
                color: root.foreground
                opacity: 0.45
                font.family: root.fontFamily
                font.pixelSize: Style.font.caption
              }
              MouseArea {
                anchors.fill: parent
                enabled: !modelData.protected
                onClicked: root.togglePkg(modelData.name)
              }
            }
          }

          Text {
            visible: (root.leftovers[modelData.name] || []).length > 0
            text: tr("soft.leftovers")
            color: root.foreground
            opacity: 0.45
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }

          Repeater {
            model: root.leftovers[modelData.name] || []
            delegate: Item {
              required property var modelData
              width: parent.width
              height: Style.space(48)

              CheckGlyph {
                id: leftoverBox
                z: 2
                anchors.left: parent.left
                anchors.leftMargin: Style.space(16)
                anchors.verticalCenter: parent.verticalCenter
                checkState: {
                  var _ = root.revision
                  return root.leftoverSelected[modelData.path] ? "all" : "none"
                }
                foreground: root.foreground
                onClicked: root.toggleLeftover(modelData.path)
              }
              Column {
                anchors.left: leftoverBox.right
                anchors.leftMargin: Style.space(10)
                anchors.right: leftoverSize.left
                anchors.rightMargin: Style.spacing.md
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2
                Text {
                  width: parent.width
                  text: root.displayPath(modelData.path)
                  elide: Text.ElideMiddle
                  color: root.foreground
                  font.family: root.fontFamily
                  font.pixelSize: Style.font.body
                }
                Text {
                  width: parent.width
                  text: tr("soft.leftoverHint")
                  elide: Text.ElideRight
                  color: root.foreground
                  opacity: 0.45
                  font.family: root.fontFamily
                  font.pixelSize: Style.font.caption
                }
              }
              Text {
                id: leftoverSize
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                text: App.formatBytes(modelData.bytes)
                color: root.foreground
                opacity: 0.45
                font.family: root.fontFamily
                font.pixelSize: Style.font.caption
              }
              MouseArea {
                anchors.fill: parent
                onClicked: root.toggleLeftover(modelData.path)
              }
            }
          }
        }
      }
    }
  }

  ListView {
    id: autoList
    visible: !root.holdLanding && root.subtab === "autostart" && !root.stageApply && !root.stageDone
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: hero.bottom
    anchors.topMargin: Style.spacing.lg
    anchors.bottom: autoFooter.top
    anchors.bottomMargin: Style.spacing.md
    clip: true
    spacing: Style.spacing.sm
    model: root.visibleAutostart
    boundsBehavior: Flickable.StopAtBounds

    delegate: Rectangle {
      required property var modelData
      width: autoList.width
      height: Style.space(56)
      radius: Math.max(Style.cornerRadius, Style.space(8))
      color: root.cardFill(rowMouse.containsMouse, true)
      opacity: modelData.locked ? 0.55 : 1

      MouseArea {
        id: rowMouse
        anchors.fill: parent
        hoverEnabled: true
        enabled: !modelData.locked || root.addingAutostart
        onClicked: {
          if (root.addingAutostart)
            root.autostartAdd(modelData.id)
          else if (!modelData.locked)
            root.autostartSet(modelData.id, !modelData.enabled)
        }
      }

      CheckGlyph {
        id: autoBox
        anchors.left: parent.left
        anchors.leftMargin: Style.spacing.md
        anchors.verticalCenter: parent.verticalCenter
        checkState: (!root.addingAutostart && modelData.enabled) ? "all" : "none"
        interactive: !modelData.locked && !root.addingAutostart
        foreground: root.foreground
        accent: root.accent
        onClicked: {
          if (!modelData.locked)
            root.autostartSet(modelData.id, !modelData.enabled)
        }
      }

      Item {
        id: autoIcon
        anchors.left: autoBox.right
        anchors.leftMargin: Style.space(10)
        anchors.verticalCenter: parent.verticalCenter
        width: Style.space(22)
        height: Style.space(22)
        AppIcon {
          anchors.fill: parent
          pixelSize: parent.width
          iconName: modelData.icon || ""
          fallbackText: modelData.name || modelData.id
          appLibrary: root.appLibrary
          foreground: root.foreground
        }
      }

      Column {
        anchors.left: autoIcon.right
        anchors.leftMargin: Style.space(10)
        anchors.right: autoTail.left
        anchors.rightMargin: Style.spacing.sm
        anchors.verticalCenter: parent.verticalCenter
        spacing: 2
        Text {
          width: parent.width
          text: modelData.name || modelData.id
          elide: Text.ElideRight
          color: root.foreground
          font.family: root.fontFamily
          font.pixelSize: Style.font.body
        }
        Text {
          width: parent.width
          text: {
            if (root.addingAutostart)
              return modelData.description || ""
            if (modelData.locked)
              return tr("soft.autostartLocked")
            if (modelData.enabled)
              return tr("soft.autostartOn") + "  ·  " + (modelData.user_added ? tr("soft.autostartUser") : tr("soft.autostartSystem"))
            return tr("soft.autostartOff")
          }
          elide: Text.ElideRight
          color: root.foreground
          opacity: 0.5
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
        }
      }

      Text {
        id: autoTail
        visible: !root.addingAutostart && !!modelData.user_added
        anchors.right: parent.right
        anchors.rightMargin: Style.spacing.md
        anchors.verticalCenter: parent.verticalCenter
        text: tr("soft.autostartRemove")
        color: root.urgent
        opacity: 0.8
        font.family: root.fontFamily
        font.pixelSize: Style.font.caption
        MouseArea {
          anchors.fill: parent
          anchors.margins: -6
          onClicked: root.autostartRemove(modelData.id)
        }
      }
    }
  }

  Text {
    visible: autoList.visible && root.visibleAutostart.length === 0
    anchors.centerIn: parent
    text: root.addingAutostart ? tr("soft.autostartPick") : tr("soft.autostartEmpty")
    color: root.foreground
    opacity: 0.5
    font.family: root.fontFamily
  }

  NumberAnimation {
    id: listJump
    target: list
    property: "contentY"
    easing.type: Easing.InOutCubic
    onRunningChanged: {
      if (running) return
      root.railLock = false
      if (!root.railDragging && root.topLetter)
        root.setSelectedLetter(root.topLetter, true)
    }
  }

  Item {
    id: letterRail
    visible: list.visible && root.visiblePkgs.length > 0
    z: 5
    anchors.right: parent.right
    anchors.top: list.top
    anchors.bottom: list.bottom
    width: Style.space(26)

    readonly property int count: Math.max(1, root.indexLetters.length)
    readonly property real slotH: height / count

    onHeightChanged: {
      if (!thumbTravel.running && root.selectedLetter)
        thumb.y = root.thumbYFor(root.selectedLetter)
    }
    onVisibleChanged: {
      if (visible && root.selectedLetter) {
        thumbTravel.stop()
        thumb.y = root.thumbYFor(root.selectedLetter)
        thumb.scale = 1
      }
    }

    Rectangle {
      id: track
      anchors.horizontalCenter: parent.horizontalCenter
      width: Math.max(2, Style.space(2))
      height: parent.height
      radius: width / 2
      color: root.foreground
      opacity: 0.12
    }

    Rectangle {
      id: thumb
      z: 1
      width: Style.space(14)
      height: Math.max(1, letterRail.slotH - Style.space(2))
      radius: Style.space(4)
      x: (parent.width - width) / 2
      y: 0
      color: root.accent
      transformOrigin: Item.Center
      scale: 1
    }

    ParallelAnimation {
      id: thumbTravel
      NumberAnimation {
        id: thumbYAnim
        target: thumb
        property: "y"
        easing.type: Easing.InOutCubic
      }
      SequentialAnimation {
        NumberAnimation {
          id: takeoffAnim
          target: thumb
          property: "scale"
          to: 1.06
          easing.type: Easing.OutCubic
        }
        NumberAnimation {
          id: landAnim
          target: thumb
          property: "scale"
          to: 1.0
          easing.type: Easing.InOutCubic
        }
      }
    }

    Column {
      z: 2
      anchors.fill: parent
      Repeater {
        model: root.indexLetters
        delegate: Item {
          required property var modelData
          width: parent.width
          height: letterRail.slotH
          readonly property bool onThumb: root.selectedLetter === modelData
          Text {
            anchors.centerIn: parent
            text: modelData
            color: parent.onThumb ? root.pageBg : root.foreground
            opacity: parent.onThumb ? 1 : (root.letterPresent[modelData] ? 0.55 : 0.16)
            font.family: root.fontFamily
            font.pixelSize: Math.max(8, Math.min(Style.font.caption, parent.height - 1))
            font.bold: parent.onThumb
          }
        }
      }
    }

    MouseArea {
      z: 3
      anchors.fill: parent
      hoverEnabled: true
      onPressed: function(mouse) {
        root.railDragging = false
        root.railPressY = mouse.y
        root.jumpFromY(mouse.y, false)
      }
      onPositionChanged: function(mouse) {
        if (!pressed) return
        if (Math.abs(mouse.y - root.railPressY) > 6)
          root.railDragging = true
        if (root.railDragging)
          root.jumpFromY(mouse.y, true)
      }
      onReleased: {
        root.railDragging = false
        if (listJump.running)
          return
        root.railLock = false
        if (root.topLetter)
          root.setSelectedLetter(root.topLetter, false)
      }
      onCanceled: {
        root.railDragging = false
        if (!listJump.running)
          root.railLock = false
      }
    }
  }



  Item {
    visible: root.stageApply
    enabled: root.stageApply
    anchors.fill: parent

    PixelField {
      id: removeHero
      anchors.horizontalCenter: parent.horizontalCenter
      width: parent.width
      y: App.landingHeroY(parent.height, implicitHeight, Style.space(24))
      showMark: true
      drawField: false
      marquee: false
      mood: "idle"
      markScale: 1.0
      showCaption: false
      interactive: false
      sweep: true
      accent: root.accent
      urgent: root.urgent
      foreground: root.foreground
      fontFamily: root.fontFamily
      onSweepCycled: {
        root.removeSweepDone = true
        if (!root.applying)
          root.removeSweep = false
      }
    }

    Column {
      anchors.horizontalCenter: parent.horizontalCenter
      anchors.bottom: parent.bottom
      anchors.bottomMargin: Style.space(48)
      width: parent.width
      spacing: 6
      Row {
        visible: root.removedIcons.length > 0
        anchors.horizontalCenter: parent.horizontalCenter
        spacing: Style.spacing.md
        Repeater {
          model: root.removedIcons
          AppIcon {
            required property var modelData
            pixelSize: Style.space(44)
            iconName: modelData.icon || App.iconNameForPackage(modelData.name, root.desktopIndex)
            fallbackText: modelData.name
            appLibrary: root.appLibrary
            foreground: root.foreground
          }
        }
      }
      Text {
        width: parent.width
        text: App.formatBytes(root.freedNow)
        color: root.foreground
        horizontalAlignment: Text.AlignHCenter
        font.family: root.fontFamily
        font.pixelSize: Style.font.heading
        font.bold: true
      }
      Text {
        width: parent.width
        text: tr("soft.removingHint", { n: root.removedPkgs })
        color: root.foreground
        opacity: 0.62
        horizontalAlignment: Text.AlignHCenter
        font.family: root.fontFamily
        font.pixelSize: Style.font.caption
      }
    }
  }

  Item {
    visible: root.stageDone
    enabled: root.stageDone
    anchors.fill: parent

    PixelField {
      anchors.fill: parent
      fillHost: true
      showMark: true
      drawField: false
      marquee: false
      mood: "celebrate"
      markScale: 0.72
      showCaption: false
      interactive: false
      etch: false
      stamps: false
      accent: root.accent
      urgent: root.urgent
      foreground: root.foreground
      fontFamily: root.fontFamily
    }

    Column {
      anchors.horizontalCenter: parent.horizontalCenter
      anchors.bottom: parent.bottom
      anchors.bottomMargin: Style.space(36)
      width: parent.width
      spacing: Style.spacing.md

      Row {
        visible: root.removedIcons.length > 0
        anchors.horizontalCenter: parent.horizontalCenter
        spacing: Style.spacing.md
        Repeater {
          model: root.removedIcons
          AppIcon {
            required property var modelData
            pixelSize: Style.space(48)
            iconName: modelData.icon || App.iconNameForPackage(modelData.name, root.desktopIndex)
            fallbackText: modelData.name
            appLibrary: root.appLibrary
            foreground: root.foreground
          }
        }
      }
      Text {
        width: parent.width
        text: App.formatBytes(root.freedNow)
        color: root.foreground
        horizontalAlignment: Text.AlignHCenter
        font.family: root.fontFamily
        font.pixelSize: Style.font.heading
        font.bold: true
      }
      Text {
        width: parent.width
        text: tr("soft.doneSub", {
          pkgs: root.removedPkgs,
          files: root.removedFiles,
          bytes: App.formatBytes(root.freedNow)
        })
        color: root.foreground
        opacity: 0.7
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
        font.family: root.fontFamily
        font.pixelSize: Style.font.caption
      }
      Button {
        anchors.horizontalCenter: parent.horizontalCenter
        text: tr("soft.back")
        selected: true
        onClicked: root.resetRequested()
      }
    }
  }

  Rectangle {
    id: autoFooter
    visible: !root.holdLanding && root.subtab === "autostart" && !root.stageApply && !root.stageDone
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.bottom: parent.bottom
    height: Style.space(48)
    color: "transparent"

    Text {
      anchors.left: parent.left
      anchors.verticalCenter: parent.verticalCenter
      text: root.addingAutostart
        ? tr("soft.autostartPick")
        : (tr("soft.autostartEnabled", { n: root.autostartOnCount }) + "  ·  " + tr("soft.autostartHint"))
      color: root.foreground
      opacity: 0.7
      font.family: root.fontFamily
      font.pixelSize: Style.font.caption
    }

    Button {
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      text: root.addingAutostart ? tr("soft.autostartDone") : tr("soft.autostartAdd")
      selected: true
      onClicked: {
        root.addingAutostart = !root.addingAutostart
        root.autostartQuery = ""
      }
    }
  }

  Rectangle {
    id: footer
    visible: !root.holdLanding && root.subtab === "remove" && !root.stageApply && !root.stageDone
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.bottom: parent.bottom
    height: Style.space(48)
    color: "transparent"

    Row {
      anchors.left: parent.left
      anchors.verticalCenter: parent.verticalCenter
      spacing: Style.spacing.sm
      Repeater {
        model: root.selectedNames.slice(0, 8)
        delegate: Item {
          required property var modelData
          width: Style.space(22)
          height: Style.space(22)
          AppIcon {
            anchors.fill: parent
            pixelSize: parent.width
            iconName: App.iconNameForPackage(modelData, root.desktopIndex)
            fallbackText: modelData
            appLibrary: root.appLibrary
            foreground: root.foreground
          }
        }
      }
      Text {
        anchors.verticalCenter: parent.verticalCenter
        text: tr("soft.selected", { n: root.selectedNames.length, bytes: App.formatBytes(root.selectedBytes) })
        color: root.foreground
        font.family: root.fontFamily
      }
    }

    Button {
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      text: root.applying ? tr("soft.removing") : tr("soft.removeN", { n: root.selectedNames.length })
      enabled: !root.applying && root.selectedNames.length > 0
      selected: true
      onClicked: root.applyRequested()
    }
  }
}
