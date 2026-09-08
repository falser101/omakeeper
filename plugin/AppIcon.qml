import QtQuick
import QtQuick.Window
import Quickshell
import qs.Commons

Item {
  id: root
  property string iconName: ""
  property var appLibrary: null
  property string fallbackText: "?"
  property color foreground: Color.menu.text
  property int pixelSize: 28

  width: Math.max(16, pixelSize)
  height: Math.max(16, pixelSize)
  implicitWidth: Math.max(16, pixelSize)
  implicitHeight: Math.max(16, pixelSize)
  clip: false

  readonly property string resolved: {
    var n = String(root.iconName || "").trim()
    if (!n)
      return ""
    if (n.indexOf("file://") === 0 || n.indexOf("image://") === 0 || n.indexOf("qrc:") === 0)
      return n
    if (n.charAt(0) === "/")
      return Util.fileUrl(n)
    if (root.appLibrary && root.appLibrary.iconIndex && root.appLibrary.iconIndex[n])
      return Util.fileUrl(String(root.appLibrary.iconIndex[n]))
    if (root.appLibrary && typeof root.appLibrary.iconSource === "function") {
      var fromLib = String(root.appLibrary.iconSource(n) || "")
      if (fromLib.length)
        return fromLib
    }
    var themed = String(Quickshell.iconPath(n, true) || "")
    if (!themed.length)
      themed = String(Quickshell.iconPath(n) || "")
    if (!themed.length)
      return ""
    if (themed.indexOf("file://") === 0 || themed.indexOf("image://") === 0)
      return themed
    if (themed.charAt(0) === "/")
      return Util.fileUrl(themed)
    return themed
  }

  Rectangle {
    anchors.fill: parent
    radius: Math.max(4, root.width * 0.22)
    color: Qt.rgba(1, 1, 1, 0.16)
    border.width: 1
    border.color: Qt.rgba(1, 1, 1, 0.12)
    visible: img.status !== Image.Ready
    Text {
      anchors.centerIn: parent
      text: String(root.fallbackText || "?").replace(/^[^A-Za-z0-9]+/, "").charAt(0).toUpperCase() || "?"
      color: root.foreground
      font.bold: true
      font.pixelSize: Math.max(9, Math.round(root.width * 0.42))
    }
  }

  Image {
    id: img
    anchors.fill: parent
    anchors.margins: 1
    source: root.resolved
    fillMode: Image.PreserveAspectFit
    asynchronous: true
    cache: true
    smooth: true
    mipmap: true
    sourceSize.width: Math.max(32, Math.round(root.width * Screen.devicePixelRatio))
    sourceSize.height: Math.max(32, Math.round(root.height * Screen.devicePixelRatio))
    visible: status === Image.Ready
  }
}
