import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui
import "App.js" as App

Item {
  id: root

  property var shell: null
  property var manifest: null
  property string omarchyPath: Quickshell.env("OMARCHY_PATH")
  property bool opened: false
  property bool closingFromHost: false
  property int tab: 0

  property color foreground: Color.menu.text
  property color border: Color.menu.border
  property var borderSpec: Border.surfaceSpec("menu", "border", border, Math.max(1, Style.space(2)))
  property color scrim: Color.menu.scrim
  property string fontFamily: Style.font.menuFamily
  readonly property int cornerRadius: Style.cornerRadius

  property var cleanItems: []
  property var cleanSelected: ({})
  property string cleanPhase: "idle"
  property var cleanProgress: []
  property var cleanCurrent: ({})
  property int cleanIndex: 0
  property int cleanTotal: 0
  property int cleanSkipped: 0
  property real cleanElapsed: 0
  property var cleanStartedAt: 0
  property real cleanFreed: 0
  property var optimizeTasks: []
  property var optimizeSelected: ({})
  property var optimizeLog: []
  property int optimizeDone: 0
  property var packages: []
  property var pkgSelected: ({})
  property var pkgExpanded: ({})
  property var pkgLeftovers: ({})
  property string pkgQuery: ""
  property var analyzeReport: ({ path: "", entries: [], total_size: 0, overview: true })
  property var analyzeCrumbs: []
  property var statusSnap: ({})
  property var historyTotals: ({ cleaned: 0, uninstalled: 0, optimized: 0 })
  property int dataRev: 0
  property string confirmKind: ""
  property string confirmMessage: ""

  readonly property var tabs: [
    { id: "clean", label: "清理" },
    { id: "software", label: "软件" },
    { id: "optimize", label: "优化" },
    { id: "analyze", label: "分析" },
    { id: "status", label: "状态" }
  ]
  readonly property var tabTint: ["#07101f", "#1c1210", "#16140c", "#1a120e", "#14120c"]
  readonly property color panelColor: Qt.tint(Color.menu.background, Qt.rgba(
    0.12, 0.1, 0.08, 0.55
  ))

  readonly property var appLibrary: shell && shell.appLibrary ? shell.appLibrary : null
  property var iconFiles: ({})
  property int iconGen: 0
  readonly property string iconCachePath: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/omakeeper-icons.json"
  readonly property string iconScriptPath: (Quickshell.env("HOME") || "") + "/.config/omarchy/plugins/io.github.falser101.omakeeper/icon-index.py"
  readonly property var desktopIndex: {
    var _ = root.iconGen
    var values = []
    try { values = DesktopEntries.applications.values || [] } catch (e) { values = [] }
    var __len = values.length
    return App.mergeIconMaps(root.iconFiles, root.appLibrary, values)
  }

  function open(payloadJson) {
    root.closingFromHost = false
    root.opened = true
    win.visible = true
    Qt.callLater(function() { if (keyCatcher) keyCatcher.forceActiveFocus() })
    cli.scanHistory()
    if (root.tab === 4) cli.scanStatus()
  }

  function close() {
    root.closingFromHost = true
    root.opened = false
    win.visible = false
    root.closingFromHost = false
  }

  function dismiss() {
    if (root.shell && typeof root.shell.hide === "function")
      root.shell.hide((root.manifest && root.manifest.id) || "io.github.falser101.omakeeper")
    else
      root.close()
  }

  function toggle() {
    if (root.opened) root.dismiss()
    else root.open("{}")
  }

  function setTab(i) {
    root.tab = i
    if (i === 1 && !root.packages.length) cli.scanUninstall()
    if (i === 2 && !root.optimizeTasks.length) cli.scanOptimize()
    if (i === 3 && !(root.analyzeReport && root.analyzeReport.entries && root.analyzeReport.entries.length)) {
      root.analyzeCrumbs = [{ name: "概览", path: "" }]
      cli.scanAnalyze("")
    }
    if (i === 4) cli.scanStatus()
  }

  function crumbName(path) {
    if (!path) return "概览"
    var parts = String(path).split("/").filter(function(s) { return s.length > 0 })
    return parts.length ? parts[parts.length - 1] : "/"
  }

  function navigateAnalyze(path) {
    path = path || ""
    var crumbs = root.analyzeCrumbs.slice()
    if (!path) {
      root.analyzeCrumbs = [{ name: "概览", path: "" }]
      cli.scanAnalyze("")
      return
    }
    for (var i = 0; i < crumbs.length; i++) {
      if (crumbs[i].path === path) {
        root.analyzeCrumbs = crumbs.slice(0, i + 1)
        cli.scanAnalyze(path)
        return
      }
    }
    var last = crumbs.length ? crumbs[crumbs.length - 1] : null
    if (last && last.path === path)
      return
    crumbs.push({ name: root.crumbName(path), path: path })
    root.analyzeCrumbs = crumbs
    cli.scanAnalyze(path)
  }

  function copyMap(src) {
    var n = {}
    if (src) for (var k in src) n[k] = src[k]
    return n
  }

  function toggleClean(id) {
    var n = copyMap(root.cleanSelected)
    n[id] = !n[id]
    root.cleanSelected = n
    root.dataRev += 1
  }

  function toggleCleanCategory(cat) {
    var check = App.categoryCheck(root.cleanItems, root.cleanSelected, cat)
    var n = copyMap(root.cleanSelected)
    var on = check.state !== "all"
    for (var i = 0; i < check.ids.length; i++)
      n[check.ids[i]] = on
    root.cleanSelected = n
    root.dataRev += 1
  }

  function toggleCleanIds(ids) {
    ids = ids || []
    var n = copyMap(root.cleanSelected)
    var allOn = ids.length > 0
    for (var i = 0; i < ids.length; i++) {
      if (!n[ids[i]]) { allOn = false; break }
    }
    for (i = 0; i < ids.length; i++) n[ids[i]] = !allOn
    root.cleanSelected = n
    root.dataRev += 1
  }

  function revealPath(path) {
    if (!path) return
    Quickshell.execDetached(["xdg-open", path])
  }

  function setCleanSelection(on) {
    var n = copyMap(root.cleanSelected)
    var items = root.cleanItems || []
    for (var i = 0; i < items.length; i++) {
      if (items[i].skip_reason) n[items[i].id] = false
      else n[items[i].id] = !!on
    }
    root.cleanSelected = n
    root.dataRev += 1
  }

  function toggleTask(id) {
    var n = copyMap(root.optimizeSelected)
    n[id] = !n[id]
    root.optimizeSelected = n
    root.dataRev += 1
  }

  function togglePkg(name) {
    var n = copyMap(root.pkgSelected)
    n[name] = !n[name]
    root.pkgSelected = n
    root.dataRev += 1
  }

  function expandPkg(name) {
    var n = copyMap(root.pkgExpanded)
    n[name] = !n[name]
    root.pkgExpanded = n
    if (n[name] && !root.pkgLeftovers[name])
      cli.scanUninstallPkg(name)
    root.dataRev += 1
  }

  function ask(kind, message) {
    root.confirmKind = kind
    root.confirmMessage = message
    confirm.opened = true
  }

  function doConfirm() {
    confirm.opened = false
    if (root.confirmKind === "clean") {
      var picked = App.selectedItems(root.cleanItems, root.cleanSelected)
      root.cleanProgress = []
      root.cleanCurrent = picked.length ? picked[0] : ({})
      root.cleanIndex = 0
      root.cleanTotal = picked.length
      root.cleanSkipped = 0
      root.cleanStartedAt = Date.now()
      root.cleanPhase = "applying"
      cli.applyClean(App.selectedIds(root.cleanItems, root.cleanSelected))
    } else if (root.confirmKind === "optimize") {
      var ids = []
      for (var i = 0; i < root.optimizeTasks.length; i++) {
        var t = root.optimizeTasks[i]
        if (root.optimizeSelected[t.id] && t.status === "ready") ids.push(t.id)
      }
      root.optimizeLog = []
      root.optimizeDone = 0
      cli.applyOptimize(ids)
    } else if (root.confirmKind === "uninstall") {
      var names = []
      for (var p in root.pkgSelected) if (root.pkgSelected[p]) names.push(p)
      cli.applyUninstall(names)
    } else if (root.confirmKind.indexOf("trash:") === 0) {
      cli.trashPaths([root.confirmKind.substring(6)])
    }
    root.confirmKind = ""
  }

  Service {
    id: cli
    onProgressed: function(kind, data) {
      if (kind !== "clean-apply" || !data) return
      if (data.event === "item") {
        var log = root.cleanProgress.slice()
        log.push(data)
        root.cleanProgress = log
        root.cleanIndex = log.length
        root.cleanCurrent = data
        if (data.ok === false) root.cleanSkipped += 1
        if (data.freed != null) root.cleanFreed = Number(data.freed)
        root.dataRev += 1
      }
    }
    onFinished: function(kind, data, error) {
      if (error)
        root.confirmMessage = error
      if (kind === "clean-scan" && data && data.items) {
        root.cleanItems = data.items
        root.cleanSelected = App.primeSelection(data.items)
        root.cleanPhase = "review"
        root.dataRev += 1
      } else if (kind === "clean-apply") {
        root.cleanFreed = data && data.freed_bytes ? Number(data.freed_bytes) : App.selectedBytes(root.cleanItems, root.cleanSelected)
        if (data && data.skipped != null) root.cleanSkipped = Number(data.skipped)
        root.cleanElapsed = Date.now() - Number(root.cleanStartedAt || Date.now())
        root.cleanPhase = "done"
        root.historyTotals.cleaned += root.cleanFreed
        root.cleanItems = []
        root.cleanSelected = ({})
        cli.scanHistory()
      } else if (kind === "optimize-scan" && data && data.tasks) {
        root.optimizeTasks = data.tasks
        var sel = {}
        for (var i = 0; i < data.tasks.length; i++)
          sel[data.tasks[i].id] = !!data.tasks[i].selected
        root.optimizeSelected = sel
        root.dataRev += 1
      } else if (kind === "optimize-apply") {
        root.optimizeDone = data && data.applied ? data.applied : root.optimizeDone + 1
        var lines = root.optimizeLog.slice()
        if (data && data.tasks) {
          for (var j = 0; j < data.tasks.length; j++) {
            if (data.tasks[j].status === "applied")
              lines.push(data.tasks[j].label)
          }
        } else {
          lines.push("优化完成")
        }
        root.optimizeLog = lines
        cli.scanOptimize()
        cli.scanHistory()
      } else if (kind === "uninstall-scan" && data && data.packages) {
        root.packages = App.attachPackageIcons(data.packages, root.desktopIndex, root.appLibrary)
        root.dataRev += 1
      } else if (kind === "uninstall-pkg" && data) {
        var leftovers = copyMap(root.pkgLeftovers)
        var name = (data.packages && data.packages[0]) ? data.packages[0].name : ""
        leftovers[name] = data.leftovers || []
        root.pkgLeftovers = leftovers
        root.dataRev += 1
      } else if (kind === "uninstall-apply") {
        root.pkgSelected = ({})
        cli.scanUninstall()
        cli.scanHistory()
      } else if (kind === "analyze" && data) {
        root.analyzeReport = data
        root.dataRev += 1
      } else if (kind === "status" && data) {
        root.statusSnap = data
        root.dataRev += 1
      } else if (kind === "history" && data) {
        root.historyTotals = App.historyTotals(data)
        root.dataRev += 1
      } else if (kind === "trash") {
        var last = root.analyzeCrumbs.length ? root.analyzeCrumbs[root.analyzeCrumbs.length - 1].path : ""
        cli.scanAnalyze(last)
      }
    }
  }

  Timer {
    interval: 2500
    repeat: true
    running: root.opened && root.tab === 4
    onTriggered: if (!cli.busy) cli.scanStatus()
  }

  FileView {
    id: iconFileView
    path: root.iconCachePath
    watchChanges: true
    printErrors: false
    onLoaded: {
      var data = App.parseJson(text())
      if (data) {
        root.iconFiles = data
        root.iconGen += 1
      }
    }
  }

  FloatingWindow {
    id: win
    title: "Omakeeper"
    visible: false
    color: Qt.darker(root.tabTint[root.tab] || Color.menu.background, 1.0)
    implicitWidth: 1280
    implicitHeight: 820
    minimumSize: Qt.size(900, 600)

    onVisibleChanged: {
      if (!visible && !root.closingFromHost && root.shell && typeof root.shell.hide === "function")
        root.shell.hide((root.manifest && root.manifest.id) || "io.github.falser101.omakeeper")
    }

    BorderSurface {
      id: card
      anchors.fill: parent
      radius: 0
      color: "transparent"
      borderSpec: root.borderSpec
      padding: Style.spacing.panelPadding

      Item {
        id: keyCatcher
        anchors.fill: parent
        focus: true
        Keys.priority: Keys.BeforeItem
        Keys.onPressed: function(event) {
          if (confirm.opened) {
            event.accepted = confirm.handleKey(event)
            return
          }
          if (event.key === Qt.Key_Escape) {
            root.dismiss()
            event.accepted = true
          } else if (event.key === Qt.Key_Left) {
            root.setTab((root.tab + 4) % 5)
            event.accepted = true
          } else if (event.key === Qt.Key_Right) {
            root.setTab((root.tab + 1) % 5)
            event.accepted = true
          } else if (event.key >= Qt.Key_1 && event.key <= Qt.Key_5) {
            root.setTab(event.key - Qt.Key_1)
            event.accepted = true
          }
        }
      }

      Column {
        anchors.fill: parent
        anchors.topMargin: card.contentTopInset
        anchors.rightMargin: card.contentRightInset
        anchors.bottomMargin: card.contentBottomInset
        anchors.leftMargin: card.contentLeftInset
        spacing: Style.spacing.md

        Item {
          width: parent.width
          height: Style.space(40)

          Rectangle {
            anchors.horizontalCenter: parent.horizontalCenter
            height: Style.space(36)
            width: pillRow.implicitWidth + Style.space(12)
            radius: height / 2
            color: Qt.rgba(1, 1, 1, 0.07)

            Row {
              id: pillRow
              anchors.centerIn: parent
              spacing: 2
              Rectangle {
                width: Style.space(28)
                height: Style.space(28)
                radius: width / 2
                color: Qt.rgba(1, 1, 1, 0.08)
                anchors.verticalCenter: parent.verticalCenter
                Text {
                  anchors.centerIn: parent
                  text: "⌘"
                  color: root.foreground
                  font.pixelSize: Style.font.caption
                }
              }
              Repeater {
                model: root.tabs
                delegate: Rectangle {
                  required property var modelData
                  required property int index
                  width: tabLabel.implicitWidth + Style.space(22)
                  height: Style.space(28)
                  radius: height / 2
                  color: root.tab === index ? Qt.rgba(1, 1, 1, 0.88) : "transparent"
                  Text {
                    id: tabLabel
                    anchors.centerIn: parent
                    text: modelData.label
                    color: root.tab === index ? "#16120e" : root.foreground
                    font.family: root.fontFamily
                    font.pixelSize: Style.font.body
                    font.bold: root.tab === index
                  }
                  MouseArea {
                    anchors.fill: parent
                    onClicked: root.setTab(index)
                  }
                }
              }
            }
          }

          Text {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            text: "Esc 关闭窗口"
            color: root.foreground
            opacity: 0.35
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }
        }

        Item {
          width: parent.width
          height: parent.height - Style.space(52)

          CleanView {
            anchors.fill: parent
            visible: root.tab === 0
            items: root.cleanItems
            selected: root.cleanSelected
            revision: root.dataRev
            scanning: cli.busy && cli.kind === "clean-scan"
            applying: cli.busy && cli.kind === "clean-apply"
            totals: root.historyTotals
            freedNow: root.cleanFreed
            phase: root.cleanPhase
            progressLog: root.cleanProgress
            progressCurrent: root.cleanCurrent
            progressIndex: root.cleanIndex
            progressTotal: root.cleanTotal
            skipped: root.cleanSkipped
            elapsedMs: root.cleanElapsed
            foreground: root.foreground
            appLibrary: root.appLibrary
            desktopIndex: root.desktopIndex
            onToggleId: function(id) { root.toggleClean(id) }
            onToggleCategorySelect: function(cat) { root.toggleCleanCategory(cat) }
            onToggleIds: function(ids) { root.toggleCleanIds(ids) }
            onSetAllSelected: function(on) { root.setCleanSelection(on) }
            onRevealPath: function(path) { root.revealPath(path) }
            onScanRequested: cli.scanClean()
            onApplyRequested: root.ask("clean", "删除 " + App.formatBytes(App.selectedBytes(root.cleanItems, root.cleanSelected)) + " 缓存？")
            onResetRequested: {
              root.cleanPhase = "idle"
              root.cleanProgress = []
              root.cleanCurrent = ({})
              root.cleanIndex = 0
              root.cleanTotal = 0
            }
          }

          SoftwareView {
            anchors.fill: parent
            visible: root.tab === 1
            packages: root.packages
            selected: root.pkgSelected
            leftovers: root.pkgLeftovers
            expanded: root.pkgExpanded
            revision: root.dataRev
            query: root.pkgQuery
            scanning: cli.busy && cli.kind.indexOf("uninstall") === 0
            applying: cli.busy && cli.kind === "uninstall-apply"
            foreground: root.foreground
            appLibrary: root.appLibrary
            desktopIndex: root.desktopIndex
            onTogglePkg: function(name) { root.togglePkg(name) }
            onExpandPkg: function(name) { root.expandPkg(name) }
            onQueryChangedByUser: function(v) { root.pkgQuery = v; root.dataRev += 1 }
            onApplyRequested: {
              var n = 0
              for (var k in root.pkgSelected) if (root.pkgSelected[k]) n++
              root.ask("uninstall", "卸载 " + n + " 个软件及其残留？需要管理员权限。")
            }
          }

          OptimizeView {
            anchors.fill: parent
            visible: root.tab === 2
            tasks: root.optimizeTasks
            selected: root.optimizeSelected
            logLines: root.optimizeLog
            revision: root.dataRev
            scanning: cli.busy && cli.kind === "optimize-scan"
            applying: cli.busy && cli.kind === "optimize-apply"
            doneCount: root.optimizeDone
            foreground: root.foreground
            onToggleId: function(id) { root.toggleTask(id) }
            onScanRequested: cli.scanOptimize()
            onApplyRequested: root.ask("optimize", "执行选中的系统维护任务？")
          }

          AnalyzeView {
            anchors.fill: parent
            visible: root.tab === 3
            report: root.analyzeReport
            crumbs: root.analyzeCrumbs
            revision: root.dataRev
            scanning: cli.busy && cli.kind === "analyze"
            foreground: root.foreground
            onOpenPath: function(path) { root.navigateAnalyze(path) }
            onTrashPath: function(path) {
              root.ask("trash:" + path, "将此项移到回收站？\n" + path)
            }
          }

          StatusView {
            anchors.fill: parent
            visible: root.tab === 4
            snap: root.statusSnap
            revision: root.dataRev
            foreground: root.foreground
            appLibrary: root.appLibrary
            desktopIndex: root.desktopIndex
          }
        }
      }

      ConfirmDialog {
        id: confirm
        anchors.fill: parent
        message: root.confirmMessage
        confirmText: "确认"
        cancelText: "取消"
        background: Color.menu.background
        foreground: root.foreground
        onCanceled: confirm.opened = false
        onConfirmed: root.doConfirm()
      }

      Text {
        visible: cli.lastError.length > 0 && !confirm.opened
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        anchors.margins: Style.spacing.md
        text: cli.lastError
        color: Color.urgent
        font.pixelSize: Style.font.caption
        width: parent.width * 0.6
        wrapMode: Text.WordWrap
      }
    }
  }
}
