import QtQuick
import qs.Commons
import "App.js" as App

Item {
  id: root
  property var snap: ({})
  property int revision: 0
  property color foreground: Color.menu.text
  property string fontFamily: Style.font.menuFamily
  property var appLibrary: null
  property var desktopIndex: ({})
  clip: true

  readonly property var cpu: snap.cpu || {}
  readonly property var mem: snap.memory || {}
  readonly property var net: snap.network || {}
  readonly property var disk: (snap.disks && snap.disks[0]) ? snap.disks[0] : {}
  readonly property var procs: snap.processes || []
  readonly property int gap: Style.spacing.md
  // minmax(auto-fill): columns from how many readable cards fit, not a named breakpoint.
  // 4 KPIs never use 3 columns — a leftover single card looks like a layout error.
  readonly property int statCols: fitCols(Style.space(280), 4, false)
  readonly property int bandCols: fitCols(Style.space(240), 2, true)
  readonly property real statCellW: cellW(statCols)
  readonly property real bandCellW: cellW(bandCols)

  function fitCols(minCard, maxCols, allowOdd) {
    var g = root.gap
    var n = Math.floor((root.width + g) / (minCard + g))
    if (n < 1)
      n = 1
    if (n > maxCols)
      n = maxCols
    if (!allowOdd && maxCols === 4 && n === 3)
      n = 2
    return n
  }

  function cellW(cols) {
    return (root.width - root.gap * Math.max(0, cols - 1)) / Math.max(1, cols)
  }

  Column {
    id: top
    width: parent.width
    spacing: root.gap

    Grid {
      width: parent.width
      columns: root.statCols
      columnSpacing: root.gap
      rowSpacing: root.gap

      Repeater {
        model: [
          {
            title: "健康度",
            value: String(snap.health_score == null ? "—" : snap.health_score),
            detail: App.healthLabel(snap.health_score) + "  ·  " + (snap.host || "") + "  ·  " + (snap.uptime || ""),
            art: "sun"
          },
          {
            title: "CPU",
            value: (cpu.usage == null ? "—" : Math.round(cpu.usage) + "%"),
            detail: "负载 " + ((cpu.load && cpu.load[0]) ? cpu.load[0].toFixed(2) : "—") + " / " + (cpu.logical_cpu || "?") + " 核"
          },
          {
            title: "内存",
            value: (mem.used_percent == null ? "—" : Math.round(mem.used_percent) + "%"),
            detail: App.formatBytes(mem.used) + " / " + App.formatBytes(mem.total)
          },
          {
            title: "磁盘",
            value: disk.available ? App.formatBytes(disk.available) : "—",
            detail: (disk.mount || "/") + "  已用 " + (disk.used_percent == null ? "—" : Math.round(disk.used_percent) + "%")
          }
        ]
        delegate: Rectangle {
          required property var modelData
          width: root.statCellW
          height: Style.space(120)
          radius: Style.cornerRadius
          color: Qt.rgba(1, 1, 1, 0.045)

          Loader {
            active: modelData.art === "sun"
            anchors.right: parent.right
            anchors.rightMargin: Style.spacing.md
            anchors.verticalCenter: parent.verticalCenter
            width: Style.space(88)
            height: Style.space(88)
            sourceComponent: AnimalHero {
              creatureSize: Style.space(88)
              animal: "chameleon"
              mood: Number(root.snap.health_score || 100) < 70 ? "busy" : "idle"
              showCaption: false
            }
          }

          Column {
            anchors.fill: parent
            anchors.margins: Style.spacing.md
            spacing: 4
            Text {
              text: modelData.title
              color: root.foreground
              opacity: 0.55
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
            }
            Text {
              text: modelData.value
              color: root.foreground
              font.family: root.fontFamily
              font.pixelSize: Style.font.heading
              font.bold: true
            }
            Text {
              width: parent.width * (modelData.art === "sun" ? 0.55 : 0.95)
              text: modelData.detail
              wrapMode: Text.WordWrap
              color: root.foreground
              opacity: 0.55
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
            }
          }
        }
      }
    }

    Grid {
      width: parent.width
      columns: root.bandCols
      columnSpacing: root.gap
      rowSpacing: root.gap

      Rectangle {
        width: root.bandCellW
        height: Style.space(88)
        radius: Style.cornerRadius
        color: Qt.rgba(1, 1, 1, 0.045)
        Column {
          anchors.fill: parent
          anchors.margins: Style.spacing.md
          Text { text: "网络"; color: root.foreground; opacity: 0.55; font.pixelSize: Style.font.caption }
          Text {
            text: "↓ " + App.formatBytes(net.down_bps || 0) + "/s    ↑ " + App.formatBytes(net.up_bps || 0) + "/s"
            color: root.foreground
            font.family: root.fontFamily
            font.pixelSize: Style.font.subtitle
          }
          Text {
            text: "僵尸进程 " + (snap.zombie_count || 0)
            color: root.foreground
            opacity: 0.5
            font.pixelSize: Style.font.caption
          }
        }
      }

      Rectangle {
        width: root.bandCellW
        height: Style.space(88)
        radius: Style.cornerRadius
        color: Qt.rgba(1, 1, 1, 0.045)
        Column {
          anchors.fill: parent
          anchors.margins: Style.spacing.md
          Text { text: "CPU 柱状"; color: root.foreground; opacity: 0.55; font.pixelSize: Style.font.caption }
          Text {
            text: App.bar((cpu.usage || 0) / 100, 28)
            color: root.foreground
            font.family: "monospace"
            font.pixelSize: Style.font.body
          }
        }
      }
    }
  }

  Rectangle {
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: top.bottom
    anchors.topMargin: root.gap
    anchors.bottom: parent.bottom
    radius: Style.cornerRadius
    color: Qt.rgba(1, 1, 1, 0.03)

    ListView {
      anchors.fill: parent
      anchors.margins: Style.spacing.sm
      clip: true
      model: root.procs
      header: Row {
        width: parent.width
        height: Style.space(24)
        Item { width: Style.space(26); height: 1 }
        Text { width: parent.width * 0.40; text: "进程"; color: root.foreground; opacity: 0.45; font.pixelSize: Style.font.caption }
        Text { width: parent.width * 0.15; text: "PID"; color: root.foreground; opacity: 0.45; font.pixelSize: Style.font.caption }
        Text { width: parent.width * 0.2; text: "CPU"; color: root.foreground; opacity: 0.45; font.pixelSize: Style.font.caption }
        Text { width: parent.width * 0.2; text: "内存"; color: root.foreground; opacity: 0.45; font.pixelSize: Style.font.caption }
      }
      delegate: Row {
        required property var modelData
        width: ListView.view ? ListView.view.width : 0
        height: Style.space(28)
        spacing: Style.spacing.sm
        Item {
          width: Style.space(22)
          height: Style.space(22)
          anchors.verticalCenter: parent.verticalCenter
          AppIcon {
            anchors.fill: parent
            pixelSize: parent.width
            iconName: App.iconNameForProcess(modelData.name, root.desktopIndex)
            fallbackText: modelData.name
            appLibrary: root.appLibrary
            foreground: root.foreground
          }
        }
        Text {
          width: parent.width * 0.40
          text: modelData.name
          elide: Text.ElideRight
          color: root.foreground
          font.family: root.fontFamily
          anchors.verticalCenter: parent.verticalCenter
        }
        Text { width: parent.width * 0.15; text: String(modelData.pid); color: root.foreground; opacity: 0.6 }
        Text {
          width: parent.width * 0.2
          text: modelData.cpu == null ? "—" : Number(modelData.cpu).toFixed(1) + "%"
          color: Number(modelData.cpu || 0) > 50 ? "#d07070" : root.foreground
        }
        Text { width: parent.width * 0.2; text: App.formatBytes(modelData.rss); color: root.foreground; opacity: 0.7 }
      }
    }
  }
}
