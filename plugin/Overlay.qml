import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import qs.Commons
import qs.Ui

Item {
  id: root

  property var shell: null
  property var manifest: null
  property string omarchyPath: Quickshell.env("OMARCHY_PATH")
  property bool opened: false
  property int selectedIndex: 0

  property color background: Color.menu.background
  property color foreground: Color.menu.text
  property color border: Color.menu.border
  property var borderSpec: Border.surfaceSpec("menu", "border", border, Math.max(1, Style.space(2)))
  property color scrim: Color.menu.scrim
  property color selectedBackground: Color.menu.selectedBackground
  property color selectedText: Color.menu.selectedText
  readonly property int cornerRadius: Style.cornerRadius
  property string fontFamily: Style.font.menuFamily
  property int contentMargin: Style.spacing.panelPadding
  property int cardWidth: Math.min(Style.space(720), panel.width - Style.gapsOut * 2)
  property int cardHeight: Math.min(Style.space(420), panel.height - Style.gapsOut * 2)

  readonly property var tools: [
    { title: "Clean", subtitle: "Caches, trash, browser & dev junk", icon: "user-trash-symbolic", cmd: ["omakeeper", "clean", "--dry-run"], planned: false },
    { title: "Purge", subtitle: "Project build artifacts", icon: "folder-saved-search-symbolic", cmd: ["omakeeper", "purge", "--dry-run"], planned: false },
    { title: "Installer", subtitle: "Leftover installers in Downloads", icon: "package-x-generic-symbolic", cmd: ["omakeeper", "installer", "--dry-run"], planned: false },
    { title: "Uninstall", subtitle: "Packages + leftovers (soon)", icon: "application-x-executable-symbolic", cmd: [], planned: true },
    { title: "Optimize", subtitle: "Bounded maintenance (soon)", icon: "preferences-system-symbolic", cmd: [], planned: true },
    { title: "Analyze", subtitle: "Disk explorer (soon)", icon: "drive-harddisk-symbolic", cmd: [], planned: true },
    { title: "Status", subtitle: "System health (soon)", icon: "computer-symbolic", cmd: [], planned: true },
    { title: "History", subtitle: "Recent operations", icon: "document-open-recent-symbolic", cmd: ["omakeeper", "history"], planned: false }
  ]

  function open(payloadJson) {
    root.opened = true
    root.selectedIndex = 0
    Qt.callLater(function() { keyCatcher.forceActiveFocus() })
  }

  function close() {
    root.opened = false
  }

  function dismiss() {
    root.opened = false
    if (root.shell && typeof root.shell.hide === "function")
      root.shell.hide((root.manifest && root.manifest.id) || "io.github.falser101.omakeeper")
  }

  function toggle() {
    if (root.opened) root.dismiss()
    else root.open("{}")
  }

  function activateIndex(index) {
    if (index < 0 || index >= root.tools.length) return
    var tool = root.tools[index]
    if (tool.planned || !tool.cmd || tool.cmd.length === 0) return
    root.dismiss()
    // Open a terminal running the CLI so the user can review dry-run output.
    Quickshell.execDetached([
      "omarchy", "launch", "terminal", "bash", "-lc",
      tool.cmd.join(" ") + "; echo; read -r -p 'Press Enter to close…' _"
    ])
  }

  PanelWindow {
    id: panel
    visible: root.opened
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"
    WlrLayershell.namespace: "omakeeper"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
    exclusionMode: ExclusionMode.Ignore

    Rectangle {
      anchors.fill: parent
      color: root.scrim
    }

    MouseArea {
      anchors.fill: parent
      onClicked: root.dismiss()
    }

    BorderSurface {
      id: card
      width: root.cardWidth
      height: root.cardHeight
      radius: root.cornerRadius
      anchors.centerIn: parent
      color: root.background
      borderSpec: root.borderSpec
      padding: root.contentMargin

      MouseArea { anchors.fill: parent; onClicked: {} }

      Item {
        id: keyCatcher
        anchors.fill: parent
        focus: true
        Keys.priority: Keys.BeforeItem
        Keys.onPressed: function(event) {
          if (event.key === Qt.Key_Escape) {
            root.dismiss()
            event.accepted = true
          } else if (event.key === Qt.Key_Left) {
            root.selectedIndex = (root.selectedIndex + root.tools.length - 1) % root.tools.length
            event.accepted = true
          } else if (event.key === Qt.Key_Right) {
            root.selectedIndex = (root.selectedIndex + 1) % root.tools.length
            event.accepted = true
          } else if (event.key === Qt.Key_Up) {
            root.selectedIndex = (root.selectedIndex + root.tools.length - 4) % root.tools.length
            event.accepted = true
          } else if (event.key === Qt.Key_Down) {
            root.selectedIndex = (root.selectedIndex + 4) % root.tools.length
            event.accepted = true
          } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            root.activateIndex(root.selectedIndex)
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

        Text {
          text: "Omakeeper"
          color: root.foreground
          font.family: root.fontFamily
          font.pixelSize: Style.font.heading
          font.bold: true
        }

        Text {
          text: "System maintenance for Omarchy — Esc to close"
          color: root.foreground
          opacity: 0.6
          font.family: root.fontFamily
          font.pixelSize: Style.font.body
        }

        Grid {
          id: grid
          width: parent.width
          columns: 4
          rowSpacing: Style.spacing.md
          columnSpacing: Style.spacing.md

          Repeater {
            model: root.tools
            delegate: Rectangle {
              required property var modelData
              required property int index
              width: (grid.width - grid.columnSpacing * 3) / 4
              height: Style.space(110)
              radius: root.cornerRadius
              color: index === root.selectedIndex ? root.selectedBackground : Qt.rgba(1, 1, 1, 0.04)
              opacity: modelData.planned ? 0.55 : 1
              border.width: index === root.selectedIndex ? 1 : 0
              border.color: root.border

              MouseArea {
                anchors.fill: parent
                hoverEnabled: true
                onEntered: root.selectedIndex = index
                onClicked: root.activateIndex(index)
              }

              Column {
                anchors.fill: parent
                anchors.margins: Style.spacing.md
                spacing: Style.spacing.sm

                Text {
                  text: modelData.title
                  color: index === root.selectedIndex ? root.selectedText : root.foreground
                  font.family: root.fontFamily
                  font.pixelSize: Style.font.subtitle
                  font.bold: true
                }

                Text {
                  width: parent.width
                  text: modelData.subtitle
                  wrapMode: Text.WordWrap
                  color: index === root.selectedIndex ? root.selectedText : root.foreground
                  opacity: 0.75
                  font.family: root.fontFamily
                  font.pixelSize: Style.font.caption
                }
              }
            }
          }
        }
      }
    }
  }
}
