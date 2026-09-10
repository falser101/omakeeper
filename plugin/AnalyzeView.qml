import QtQuick
import qs.Commons
import qs.Ui
import "App.js" as App
import "I18n.js" as I18n

Item {
  id: root
  property var report: ({ path: "", entries: [], total_size: 0, overview: false })
  property var crumbs: []
  property int revision: 0
  property bool scanning: false
  property color foreground: Color.menu.text
  property color accent: Color.accent
  property color urgent: Color.urgent
  property color pageBg: Color.menu.background
  property string fontFamily: Style.font.menuFamily
  property string uiLang: "en"
  function tr(key, vars) { return I18n.t(key, vars, root.uiLang) }

  signal openPath(string path)
  signal trashPath(string path)
  signal revealPath(string path)
  signal refreshRequested()

  property string hoveredPath: ""
  property bool menuOpen: false
  property var menuEntry: null
  property real menuX: 0
  property real menuY: 0

  readonly property bool lightish: {
    var c = root.pageBg
    return (c.r * 0.2126 + c.g * 0.7152 + c.b * 0.0722) > 0.5
  }
  readonly property var entries: {
    var _ = root.revision
    return (root.report && root.report.entries) ? root.report.entries : []
  }
  readonly property int itemCount: {
    var n = Number(root.report && root.report.total_files)
    return n > 0 ? n : root.entries.length
  }
  readonly property real totalSize: Number((root.report && root.report.total_size) || 0)
  readonly property real diskUsed: Number((root.report && root.report.disk_used) || 0)
  readonly property real diskTotal: Number((root.report && root.report.disk_total) || 0)
  readonly property real diskFrac: root.diskTotal > 0 ? Math.max(0, Math.min(1, root.diskUsed / root.diskTotal)) : 0
  readonly property var tiles: {
    var _ = root.revision
    var w = map.width
    var h = map.height
    if (w < 8 || h < 8) return []
    return App.squarify(root.entries, 0, 0, w, h)
  }

  function tileFill(index, remainder) {
    if (remainder)
      return Qt.hsla(0.08, 0.16, root.lightish ? 0.46 : 0.26, 1)
    var hues = [0.11, 0.055, 0.02, 0.78, 0.58, 0.13, 0.07, 0.30, 0.55, 0.09]
    return Qt.hsla(hues[index % hues.length], 0.40, root.lightish ? 0.50 : 0.38, 1)
  }

  function isRemainder(entry) {
    return !!(entry && entry.remainder)
  }

  function isProtected(entry) {
    return !!(entry && (entry.protected || entry.remainder))
  }

  function revealTarget(entry) {
    if (!entry || !entry.path) return ""
    if (entry.is_dir) return entry.path
    var p = String(entry.path)
    var i = p.lastIndexOf("/")
    return i <= 0 ? "/" : p.substring(0, i)
  }

  function openMenu(entry, item, mx, my) {
    if (!entry || root.isRemainder(entry) || !entry.path) {
      root.menuOpen = false
      return
    }
    var p = item.mapToItem(root, mx, my)
    root.menuEntry = entry
    root.menuX = Math.max(8, Math.min(p.x, root.width - 228))
    root.menuY = Math.max(8, Math.min(p.y, root.height - 108))
    root.menuOpen = true
  }

  function activateEntry(entry) {
    root.menuOpen = false
    if (!entry || root.isRemainder(entry) || !entry.is_dir || !entry.path)
      return
    root.openPath(entry.path)
  }

  function closeMenu() { root.menuOpen = false }

  Column {
    anchors.fill: parent
    spacing: Style.space(8)

    Item {
      width: parent.width
      height: Style.space(28)

      Flickable {
        id: crumbRow
        anchors.left: parent.left
        anchors.right: statsBox.left
        anchors.rightMargin: Style.spacing.md
        height: parent.height
        clip: true
        flickableDirection: Flickable.HorizontalFlick
        contentWidth: crumbInner.implicitWidth
        contentHeight: height
        boundsBehavior: Flickable.StopAtBounds

        Row {
          id: crumbInner
          spacing: 4
          height: parent.height

          Text {
            text: "󰋜"
            color: root.foreground
            opacity: 0.55
            font.family: root.fontFamily
            font.pixelSize: Style.font.body
            verticalAlignment: Text.AlignVCenter
            height: crumbRow.height
          }

          Repeater {
            model: root.crumbs
            delegate: Row {
              required property var modelData
              required property int index
              spacing: 4
              height: crumbRow.height

              Text {
                visible: index > 0
                text: "›"
                color: root.foreground
                opacity: 0.35
                font.family: root.fontFamily
                font.pixelSize: Style.font.body
                verticalAlignment: Text.AlignVCenter
                height: parent.height
              }
              Text {
                text: modelData.name
                color: root.foreground
                opacity: index === root.crumbs.length - 1 ? 1 : 0.5
                font.family: root.fontFamily
                font.pixelSize: Style.font.body
                font.bold: index === root.crumbs.length - 1
                verticalAlignment: Text.AlignVCenter
                height: parent.height
                MouseArea {
                  anchors.fill: parent
                  enabled: index !== root.crumbs.length - 1
                  cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                  onClicked: {
                    root.closeMenu()
                    root.openPath(modelData.path)
                  }
                }
              }
            }
          }
        }

        onContentWidthChanged: contentX = Math.max(0, contentWidth - width)
      }

      Row {
        id: statsBox
        anchors.right: parent.right
        height: parent.height
        spacing: Style.space(10)

        Column {
          anchors.verticalCenter: parent.verticalCenter
          spacing: 4
          width: Math.max(usageLabel.implicitWidth, Style.space(160))

          Rectangle {
            width: parent.width
            height: 3
            radius: 2
            color: Util.alpha(root.foreground, 0.12)
            Rectangle {
              width: parent.width * root.diskFrac
              height: parent.height
              radius: 2
              color: root.accent
            }
          }
          Text {
            id: usageLabel
            text: tr("analyze.usage", {
              current: App.formatBytes(root.totalSize),
              disk: App.formatBytesPair(root.diskUsed, root.diskTotal)
            })
            color: root.foreground
            opacity: 0.55
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
            wrapMode: Text.NoWrap
          }
        }

        Text {
          anchors.verticalCenter: parent.verticalCenter
          text: "󰑐"
          color: root.foreground
          opacity: refreshMouse.containsMouse ? 0.9 : 0.4
          font.family: root.fontFamily
          font.pixelSize: Style.font.body
          MouseArea {
            id: refreshMouse
            anchors.fill: parent
            anchors.margins: -6
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: {
              root.closeMenu()
              root.refreshRequested()
            }
          }
        }
      }
    }

    Row {
      width: parent.width
      height: parent.height - Style.space(28) - Style.space(22) - Style.space(16)
      spacing: Style.spacing.md

      Item {
        id: sidebar
        width: Math.min(Style.space(220), parent.width * 0.24)
        height: parent.height

        Column {
          anchors.fill: parent
          spacing: Style.space(10)

          Item {
            width: parent.width
            height: Style.space(108)

            Canvas {
              id: ring
              anchors.horizontalCenter: parent.horizontalCenter
              width: Style.space(96)
              height: Style.space(96)
              property real frac: root.diskFrac
              onFracChanged: requestPaint()
              onWidthChanged: requestPaint()
              onHeightChanged: requestPaint()
              Component.onCompleted: requestPaint()
              onPaint: {
                var ctx = getContext("2d")
                if (!ctx) return
                ctx.clearRect(0, 0, width, height)
                var cx = width / 2
                var cy = height / 2
                var r = Math.min(cx, cy) - 6
                ctx.lineWidth = 7
                ctx.lineCap = "round"
                var fg = root.foreground
                ctx.strokeStyle = Qt.rgba(fg.r, fg.g, fg.b, 0.14)
                ctx.beginPath()
                ctx.arc(cx, cy, r, 0, Math.PI * 2)
                ctx.stroke()
                var f = Math.max(0, Math.min(1, ring.frac))
                if (f > 0.002) {
                  var ac = root.accent
                  ctx.strokeStyle = Qt.rgba(ac.r, ac.g, ac.b, 0.95)
                  ctx.beginPath()
                  ctx.arc(cx, cy, r, -Math.PI / 2, -Math.PI / 2 + f * Math.PI * 2)
                  ctx.stroke()
                }
              }
            }

            Text {
              anchors.centerIn: ring
              text: "󰉋"
              color: root.foreground
              opacity: 0.55
              font.family: root.fontFamily
              font.pixelSize: Style.space(22)
            }
          }

          Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            text: tr("analyze.items", { n: root.itemCount, size: App.formatBytes(root.totalSize) })
            color: root.foreground
            opacity: 0.55
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }

          Text {
            text: tr("analyze.currentDir")
            color: root.foreground
            opacity: 0.38
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }

          Item {
            width: parent.width
            height: Math.max(40, parent.height - Style.space(168))

          ListView {
            id: side
            anchors.fill: parent
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            model: root.entries
            delegate: Rectangle {
              required property var modelData
              width: side.width
              height: Style.space(32)
              radius: 6
              color: {
                if (root.hoveredPath === modelData.path)
                  return Util.alpha(root.foreground, 0.10)
                return "transparent"
              }
              MouseArea {
                id: smouse
                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                cursorShape: modelData.is_dir ? Qt.PointingHandCursor : Qt.ArrowCursor
                onEntered: root.hoveredPath = modelData.path
                onExited: if (root.hoveredPath === modelData.path) root.hoveredPath = ""
                onClicked: function(ev) {
                  if (ev.button === Qt.RightButton)
                    root.openMenu(modelData, smouse, ev.x, ev.y)
                  else
                    root.activateEntry(modelData)
                }
              }
              Text {
                id: kindIcon
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                text: modelData.is_dir ? "󰉋" : "󰈔"
                color: root.foreground
                opacity: modelData.protected ? 0.35 : 0.55
                font.family: root.fontFamily
                font.pixelSize: Style.font.caption
              }
              Text {
                anchors.left: kindIcon.right
                anchors.leftMargin: 6
                anchors.right: sizeLabel.left
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                text: modelData.name
                elide: Text.ElideRight
                color: root.foreground
                opacity: modelData.protected ? 0.55 : 0.92
                font.family: root.fontFamily
                font.pixelSize: Style.font.caption
              }
              Text {
                id: sizeLabel
                anchors.right: chevron.left
                anchors.rightMargin: 2
                anchors.verticalCenter: parent.verticalCenter
                text: App.formatBytes(modelData.size)
                color: root.foreground
                opacity: 0.42
                font.family: root.fontFamily
                font.pixelSize: Style.font.caption
              }
              Text {
                id: chevron
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                text: modelData.protected ? "󰌾" : (modelData.is_dir ? "›" : "")
                color: root.foreground
                opacity: 0.28
                font.family: root.fontFamily
                font.pixelSize: Style.font.caption
              }
            }
          }
          }
        }
      }

      Item {
        id: map
        width: Math.max(80, parent.width - sidebar.width - parent.spacing)
        height: parent.height

        Repeater {
          model: root.tiles
          delegate: Rectangle {
            id: heightBox
            required property var modelData
            required property int index
            readonly property var entry: modelData.entry
            readonly property bool rem: root.isRemainder(entry)
            readonly property bool hot: !rem && root.hoveredPath === entry.path
            readonly property bool canLabel: width >= 72 && height >= 48
            readonly property bool showIcon: width >= 96 && height >= 72
            readonly property bool showSize: height >= 56
            x: modelData.x + 2
            y: modelData.y + 2
            width: Math.max(0, modelData.w - 4)
            height: Math.max(0, modelData.h - 4)
            radius: 8
            color: root.tileFill(index, rem)
            opacity: tmouse.containsMouse || hot ? 1 : 0.90
            border.width: hot || tmouse.containsMouse ? 1 : 0
            border.color: Util.alpha("#ffffff", 0.28)

            MouseArea {
              id: tmouse
              anchors.fill: parent
              hoverEnabled: true
              acceptedButtons: Qt.LeftButton | Qt.RightButton
              cursorShape: (!rem && entry && entry.is_dir) ? Qt.PointingHandCursor : Qt.ArrowCursor
              onEntered: if (entry && entry.path) root.hoveredPath = entry.path
              onExited: if (root.hoveredPath === (entry && entry.path)) root.hoveredPath = ""
              onClicked: function(ev) {
                if (ev.button === Qt.RightButton)
                  root.openMenu(entry, tmouse, ev.x, ev.y)
                else
                  root.activateEntry(entry)
              }
            }

            Column {
              visible: heightBox.canLabel
              anchors.centerIn: parent
              width: Math.max(0, parent.width - 20)
              spacing: 4

              Text {
                visible: heightBox.showIcon
                anchors.horizontalCenter: parent.horizontalCenter
                text: rem ? "󰉔" : (entry && entry.is_dir ? "󰉋" : "󰈔")
                color: "#f6f1e6"
                opacity: 0.9
                font.family: root.fontFamily
                font.pixelSize: Style.font.body
              }
              Text {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.NoWrap
                elide: Text.ElideRight
                text: rem
                  ? tr("analyze.more", { n: entry.leftover || 0 })
                  : (entry ? entry.name : "")
                color: "#f6f1e6"
                font.family: root.fontFamily
                font.pixelSize: Style.font.caption
                font.bold: true
              }
              Text {
                visible: heightBox.showSize
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.NoWrap
                elide: Text.ElideRight
                text: entry ? App.formatBytes(entry.size) : ""
                color: "#f6f1e6"
                opacity: 0.8
                font.family: root.fontFamily
                font.pixelSize: Style.font.caption
              }
            }
          }
        }

        Text {
          visible: root.scanning && root.tiles.length === 0
          anchors.centerIn: parent
          text: tr("analyze.scanning")
          color: root.foreground
          font.family: root.fontFamily
        }

        Rectangle {
          visible: root.scanning && root.tiles.length > 0
          anchors.fill: parent
          color: Util.alpha(root.pageBg, 0.35)
          Text {
            anchors.centerIn: parent
            text: tr("analyze.scanning")
            color: root.foreground
            font.family: root.fontFamily
          }
        }
      }
    }

    Text {
      text: tr("analyze.hint")
      color: root.foreground
      opacity: 0.38
      font.family: root.fontFamily
      font.pixelSize: Style.font.caption
    }
  }

  MouseArea {
    visible: root.menuOpen
    anchors.fill: parent
    z: 19
    onClicked: root.closeMenu()
  }

  Rectangle {
    visible: root.menuOpen && root.menuEntry
    x: root.menuX
    y: root.menuY
    z: 20
    width: Style.space(220)
    height: menuCol.implicitHeight + 12
    radius: 10
    color: root.pageBg
    border.width: 1
    border.color: Util.alpha(root.foreground, 0.16)

    Column {
      id: menuCol
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.top: parent.top
      anchors.margins: 6
      spacing: 2

      Rectangle {
        width: parent.width
        height: Style.space(32)
        radius: 6
        color: revealHover.containsMouse ? Util.alpha(root.foreground, 0.10) : "transparent"
        Text {
          anchors.verticalCenter: parent.verticalCenter
          anchors.left: parent.left
          anchors.leftMargin: 10
          text: tr("analyze.reveal")
          color: root.foreground
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
        }
        MouseArea {
          id: revealHover
          anchors.fill: parent
          hoverEnabled: true
          onClicked: {
            var target = root.revealTarget(root.menuEntry)
            root.closeMenu()
            if (target) root.revealPath(target)
          }
        }
      }

      Rectangle {
        width: parent.width - 16
        height: 1
        anchors.horizontalCenter: parent.horizontalCenter
        color: Util.alpha(root.foreground, 0.10)
      }

      Rectangle {
        width: parent.width
        height: Style.space(32)
        radius: 6
        color: (root.isProtected(root.menuEntry) || !trashHover.containsMouse)
          ? "transparent"
          : Util.alpha(root.urgent, 0.12)
        Text {
          anchors.verticalCenter: parent.verticalCenter
          anchors.left: parent.left
          anchors.leftMargin: 10
          width: parent.width - 16
          text: root.isProtected(root.menuEntry) ? tr("analyze.viewOnly") : tr("analyze.trash")
          color: root.isProtected(root.menuEntry) ? root.foreground : root.urgent
          opacity: root.isProtected(root.menuEntry) ? 0.45 : 0.95
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
          wrapMode: Text.NoWrap
          elide: Text.ElideRight
        }
        MouseArea {
          id: trashHover
          anchors.fill: parent
          hoverEnabled: true
          enabled: !root.isProtected(root.menuEntry)
          onClicked: {
            var entry = root.menuEntry
            root.closeMenu()
            if (entry && entry.path) root.trashPath(entry.path)
          }
        }
      }
    }
  }
}
