import QtQuick
import Quickshell
import Quickshell.Io
import "App.js" as App

Item {
  id: root

  readonly property string home: Quickshell.env("HOME") || ""
  readonly property string bin: {
    var cargo = root.home + "/.cargo/bin/omakeeper"
    var local = root.home + "/.local/bin/omakeeper"
    return cargo
  }
  readonly property string tmpDir: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/omakeeper"
  property string lastError: ""
  property bool busy: proc.running
  property string kind: ""
  property string stdoutText: ""
  property var result: null
  property int revision: 0

  signal finished(string kind, var data, string error)
  signal progressed(string kind, var data)

  function run(nextKind, args) {
    if (proc.running) return false
    root.kind = nextKind
    root.lastError = ""
    root.stdoutText = ""
    proc.command = [root.bin].concat(args)
    proc.running = true
    return true
  }

  function scanClean() { return run("clean-scan", ["clean", "--dry-run", "--json"]) }
  function scanOptimize() { return run("optimize-scan", ["optimize", "--dry-run", "--json"]) }
  function scanUninstall() { return run("uninstall-scan", ["uninstall", "--dry-run", "--json"]) }
  function scanUninstallPkg(name) { return run("uninstall-pkg", ["uninstall", "--dry-run", "--json", name]) }
  function scanAnalyze(path) {
    var args = ["analyze", "--json"]
    if (path) args.push(path)
    return run("analyze", args)
  }
  function scanStatus() { return run("status", ["status", "--json"]) }
  function scanHistory() { return run("history", ["history", "--json"]) }

  function applyClean(ids) {
    var path = root.tmpDir + "/select.json"
    var payload = JSON.stringify(ids || [])
    root.kind = "clean-apply"
    root.lastError = ""
    proc.command = ["bash", "-lc",
      "mkdir -p " + JSON.stringify(root.tmpDir) +
      " && printf %s " + JSON.stringify(payload) + " > " + JSON.stringify(path) +
      " && exec " + JSON.stringify(root.bin) + " clean --yes --select-file " + JSON.stringify(path) + " --json"]
    proc.running = true
    return true
  }

  function applyOptimize(ids) {
    var path = root.tmpDir + "/select.json"
    var payload = JSON.stringify(ids || [])
    root.kind = "optimize-apply"
    root.lastError = ""
    proc.command = ["bash", "-lc",
      "mkdir -p " + JSON.stringify(root.tmpDir) +
      " && printf %s " + JSON.stringify(payload) + " > " + JSON.stringify(path) +
      " && exec " + JSON.stringify(root.bin) + " optimize --yes --select-file " + JSON.stringify(path)]
    proc.running = true
    return true
  }

  function applyUninstall(names, leftoverPaths) {
    root.kind = "uninstall-apply"
    root.lastError = ""
    var args = [root.bin, "uninstall", "--yes", "--json"]
    var extras = leftoverPaths || []
    for (var j = 0; j < extras.length; j++) {
      args.push("--leftover")
      args.push(extras[j])
    }
    for (var i = 0; i < names.length; i++) args.push(names[i])
    proc.command = args
    proc.running = true
    return true
  }

  function trashPaths(paths) {
    root.kind = "trash"
    root.lastError = ""
    var args = [root.bin, "trash", "--json"]
    for (var i = 0; i < paths.length; i++) args.push(paths[i])
    proc.command = args
    proc.running = true
    return true
  }

  Process {
    id: proc
    running: false
    command: []
    stdout: SplitParser {
      onRead: function(line) {
        root.stdoutText += String(line) + "\n"
        var data = App.parseJson(String(line))
        if (data && data.event)
          root.progressed(root.kind, data)
      }
    }
    stderr: StdioCollector {
      id: procErr
      waitForEnd: true
    }
    onExited: function(code) {
      var out = String(root.stdoutText || "")
      var err = String(procErr.text || "").trim()
      var data = App.lastJsonValue(out)
      if (code !== 0) {
        root.lastError = err || out.trim() || (root.kind + " failed")
        root.result = data
        root.revision += 1
        root.finished(root.kind, data, root.lastError)
        return
      }
      root.lastError = ""
      root.result = data
      root.revision += 1
      root.finished(root.kind, data, "")
    }
  }
}
