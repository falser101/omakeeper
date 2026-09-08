import QtQuick
import qs.Commons
import qs.Ui
import "App.js" as App
import "I18n.js" as I18n

Item {
  id: root
  property var packages: []
  property var selected: ({})
  property var leftovers: ({})
  property var expanded: ({})
  property int revision: 0
  property string query: ""
  property string subtab: "remove"
  property bool scanning: false
  property bool applying: false
  property color foreground: Color.menu.text
  property color selectedBg: Color.menu.selectedBackground
  property color accent: Color.accent
  property color urgent: Color.urgent
  property string fontFamily: Style.font.menuFamily
  property string uiLang: "en"
  function tr(key, vars) { return I18n.t(key, vars, root.uiLang) }
  property var appLibrary: null
  property var desktopIndex: ({})

  signal togglePkg(string name)
  signal expandPkg(string name)
  signal applyRequested()
  signal queryChangedByUser(string value)

  readonly property var visiblePkgs: {
    var _ = root.revision
    return App.filterPackages(root.packages, root.query)
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
      for (var j = 0; j < rows.length; j++) n += Number(rows[j].bytes || 0)
    }
    return n
  }

  Row {
    id: subtabs
    spacing: Style.spacing.sm
    y: 0

    Repeater {
      model: [
        { id: "remove", label: tr("soft.remove") },
        { id: "updates", label: tr("soft.updates") },
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
    anchors.right: parent.right
    width: Style.space(220)
    placeholderText: tr("soft.search")
    text: root.query
    onTextEdited: root.queryChangedByUser(text)
  }

  AnimalHero {
    id: hero
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: subtabs.bottom
    anchors.topMargin: Style.spacing.sm
    width: parent.width
    animal: "octopus"
    mood: (root.scanning || root.applying) ? "busy" : "idle"
    creatureSize: Style.space(140)
    showCaption: false
    uiLang: root.uiLang
    accent: root.accent
    urgent: root.urgent
  }

  ListView {
    id: list
    visible: root.subtab === "remove"
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: hero.bottom
    anchors.topMargin: Style.spacing.md
    anchors.bottom: footer.top
    anchors.bottomMargin: Style.spacing.md
    clip: true
    spacing: 2
    model: root.visiblePkgs
    boundsBehavior: Flickable.StopAtBounds

    delegate: Rectangle {
      required property var modelData
      width: list.width
      height: body.implicitHeight + Style.space(16)
      radius: Style.cornerRadius
      color: mouse.containsMouse || root.selected[modelData.name] ? Util.alpha(root.foreground, 0.07) : "transparent"

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
        anchors.margins: Style.spacing.sm
        spacing: Style.spacing.xs

        Row {
          width: parent.width
          spacing: Style.spacing.md
          height: Style.space(36)

          Item {
            width: Style.space(28)
            height: Style.space(28)
            anchors.verticalCenter: parent.verticalCenter
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
            width: parent.width - Style.space(180)
            anchors.verticalCenter: parent.verticalCenter
            Text {
              text: modelData.name
              color: root.foreground
              font.family: root.fontFamily
              font.pixelSize: Style.font.subtitle
              font.bold: true
            }
            Text {
              width: parent.width
              text: App.formatBytes(modelData.bytes) + "  ·  " + (modelData.description || "")
              elide: Text.ElideRight
              color: root.foreground
              opacity: 0.55
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
            }
          }

          Item { width: 1; height: 1 }

          Text {
            anchors.verticalCenter: parent.verticalCenter
            text: App.formatBytes(modelData.bytes)
            color: root.foreground
            opacity: 0.7
            font.family: root.fontFamily
          }

          CheckGlyph {
            anchors.verticalCenter: parent.verticalCenter
            checkState: root.selected[modelData.name] ? "all" : "none"
            foreground: root.foreground
            onClicked: root.togglePkg(modelData.name)
          }
        }

        Column {
          visible: !!root.expanded[modelData.name]
          width: parent.width
          leftPadding: Style.space(40)
          spacing: 2

          Repeater {
            model: root.leftovers[modelData.name] || []
            delegate: Text {
              required property var modelData
              width: parent.width - Style.space(40)
              text: "  " + App.formatBytes(modelData.bytes) + "   " + modelData.path
              elide: Text.ElideMiddle
              color: root.foreground
              opacity: 0.7
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
            }
          }
          Text {
            visible: (root.leftovers[modelData.name] || []).length === 0
            text: tr("soft.noneLeft")
            color: root.foreground
            opacity: 0.4
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }
        }
      }
    }
  }

  Text {
    visible: root.subtab !== "remove"
    anchors.centerIn: parent
    text: root.subtab === "updates" ? tr("soft.updatesSoon") : tr("soft.autostartSoon")
    color: root.foreground
    opacity: 0.5
    font.family: root.fontFamily
  }

  Rectangle {
    id: footer
    visible: root.subtab === "remove"
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
