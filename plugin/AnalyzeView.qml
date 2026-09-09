import QtQuick
import qs.Commons
import qs.Ui
import "App.js" as App
import "I18n.js" as I18n

Item {
  id: root
  property var report: ({ path: "", entries: [], total_size: 0, overview: true })
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

  readonly property var tiles: {
    var _ = root.revision
    var entries = (root.report && root.report.entries) ? root.report.entries : []
    return App.squarify(entries, 0, 0, map.width || 800, map.height || 480)
  }

  Flickable {
    id: crumbRow
    z: 2
    width: parent.width
    height: Style.space(22)
    clip: true
    flickableDirection: Flickable.HorizontalFlick
    contentWidth: crumbInner.implicitWidth
    contentHeight: height
    boundsBehavior: Flickable.StopAtBounds

    Row {
      id: crumbInner
      spacing: 6
      height: parent.height
      Repeater {
        model: root.crumbs
        delegate: Text {
          required property var modelData
          required property int index
          text: modelData.name + (index < root.crumbs.length - 1 ? "  ›" : "")
          color: root.foreground
          opacity: index === root.crumbs.length - 1 ? 1 : 0.5
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
          verticalAlignment: Text.AlignVCenter
          height: crumbRow.height
          MouseArea {
            anchors.fill: parent
            onClicked: root.openPath(modelData.path)
          }
        }
      }
    }

    onContentWidthChanged: contentX = Math.max(0, contentWidth - width)
  }

  PixelField {
    id: jup
    anchors.horizontalCenter: parent.horizontalCenter
    y: Style.space(12)
    width: parent.width
    creatureSize: Style.space(96)
    mood: root.scanning ? "busy" : "idle"
    drawField: false
    markScale: App.CONTENT_MARK_SCALE
    showCaption: true
    headline: App.formatBytes((root.report && root.report.total_size) || 0)
    subline: root.tr("tab.analyze")
    foreground: root.foreground
    fontFamily: root.fontFamily
    uiLang: root.uiLang
    accent: root.accent
    urgent: root.urgent
  }

  Row {
    anchors.top: jup.bottom
    anchors.topMargin: Style.spacing.md
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.bottom: parent.bottom
    spacing: Style.spacing.md

    Rectangle {
      width: Math.min(Style.space(240), parent.width * 0.26)
      height: parent.height
      color: "transparent"

      ListView {
        id: side
        anchors.fill: parent
        clip: true
        model: (root.report && root.report.entries) ? root.report.entries : []
        delegate: Rectangle {
          required property var modelData
          width: side.width
          height: Style.space(28)
          color: smouse.containsMouse ? Util.alpha(root.foreground, 0.08) : "transparent"
          radius: 6
          MouseArea {
            id: smouse
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onClicked: function(ev) {
              if (ev.button === Qt.RightButton) root.trashPath(modelData.path)
              else if (modelData.is_dir) root.openPath(modelData.path)
            }
          }
          Text {
            anchors.left: parent.left
            anchors.right: sizeLabel.left
            anchors.verticalCenter: parent.verticalCenter
            text: (modelData.is_dir ? "▸  " : "   ") + modelData.name
            elide: Text.ElideRight
            color: root.foreground
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }
          Text {
            id: sizeLabel
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            text: App.formatBytes(modelData.size)
            color: root.foreground
            opacity: 0.55
            font.pixelSize: Style.font.caption
          }
        }
      }
    }

    Item {
      id: map
      width: parent.width - Style.space(256)
      height: parent.height

      Repeater {
        model: root.tiles
        delegate: Rectangle {
          required property var modelData
          required property int index
          x: modelData.x + 1
          y: modelData.y + 1
          width: Math.max(0, modelData.w - 2)
          height: Math.max(0, modelData.h - 2)
          radius: 8
          color: App.tileColor(index, modelData.entry.name)
          opacity: tmouse.containsMouse ? 0.95 : 0.82
          MouseArea {
            id: tmouse
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onClicked: function(ev) {
              if (ev.button === Qt.RightButton) root.trashPath(modelData.entry.path)
              else if (modelData.entry.is_dir) root.openPath(modelData.entry.path)
            }
          }
          Text {
            visible: parent.width > 80 && parent.height > 36
            anchors.centerIn: parent
            width: parent.width - 12
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            text: modelData.entry.name + "\n" + App.formatBytes(modelData.entry.size)
            color: "#1a120c"
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
            font.bold: true
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
    }
  }
}
