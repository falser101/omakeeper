import QtQuick
import qs.Commons
import qs.Ui
import "App.js" as App
import "I18n.js" as I18n

Item {
  id: root
  property string uiLang: "en"
  property string langPref: "system"
  property string appearancePref: "system"
  property color foreground: Color.menu.text
  property color selectedBg: Color.menu.selectedBackground
  property color selectedFg: Color.menu.selectedText
  property string fontFamily: Style.font.menuFamily
  property color accent: Color.accent
  property color urgent: Color.urgent

  signal langPrefChosen(string value)
  signal appearancePrefChosen(string value)

  function tr(key, vars) { return I18n.t(key, vars, root.uiLang) }

  PixelField {
    id: hero
    anchors.horizontalCenter: parent.horizontalCenter
    y: Style.space(12)
    width: parent.width
    mood: "idle"
    drawField: false
    markScale: App.CONTENT_MARK_SCALE
    creatureSize: Style.space(96)
    headline: root.tr("settings.title")
    subline: root.tr("settings.langHint")
    foreground: root.foreground
    fontFamily: root.fontFamily
    uiLang: root.uiLang
    accent: root.accent
    urgent: root.urgent
  }

  Column {
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: hero.bottom
    anchors.topMargin: Style.spacing.lg
    width: Math.min(parent.width - Style.space(48), Style.space(560))
    spacing: Style.spacing.lg

    Column {
      width: parent.width
      spacing: Style.spacing.sm
      Text {
        text: root.tr("settings.language")
        color: root.foreground
        font.family: root.fontFamily
        font.pixelSize: Style.font.subtitle
        font.bold: true
      }
      Text {
        text: root.tr("settings.langHint")
        color: root.foreground
        opacity: 0.5
        font.family: root.fontFamily
        font.pixelSize: Style.font.caption
      }
      ChoiceRow {
        width: parent.width
        model: [
          { id: "system", label: root.tr("settings.followSystem") },
          { id: "zh", label: root.tr("settings.zh") },
          { id: "en", label: root.tr("settings.en") }
        ]
        current: root.langPref
        foreground: root.foreground
        selectedBg: root.selectedBg
        selectedFg: root.selectedFg
        fontFamily: root.fontFamily
        onPicked: function(id) { root.langPrefChosen(id) }
      }
    }

    Column {
      width: parent.width
      spacing: Style.spacing.sm
      Text {
        text: root.tr("settings.appearance")
        color: root.foreground
        font.family: root.fontFamily
        font.pixelSize: Style.font.subtitle
        font.bold: true
      }
      Text {
        text: root.tr("settings.themeHint")
        color: root.foreground
        opacity: 0.5
        font.family: root.fontFamily
        font.pixelSize: Style.font.caption
      }
      ChoiceRow {
        width: parent.width
        model: [
          { id: "system", label: root.tr("settings.followSystem") },
          { id: "light", label: root.tr("settings.light") },
          { id: "dark", label: root.tr("settings.dark") }
        ]
        current: root.appearancePref
        foreground: root.foreground
        selectedBg: root.selectedBg
        selectedFg: root.selectedFg
        fontFamily: root.fontFamily
        onPicked: function(id) { root.appearancePrefChosen(id) }
      }
    }
  }

  component ChoiceRow: Row {
    id: row
    property var model: []
    property string current: ""
    property color foreground: Color.menu.text
    property color selectedBg: Color.menu.selectedBackground
    property color selectedFg: Color.menu.selectedText
    property string fontFamily: Style.font.menuFamily
    signal picked(string id)
    spacing: Style.space(8)

    Repeater {
      model: row.model
      delegate: Rectangle {
        required property var modelData
        width: Math.max(Style.space(88), chipLabel.implicitWidth + Style.space(28))
        height: Style.space(34)
        radius: height / 2
        color: row.current === modelData.id ? row.selectedBg : Util.alpha(row.foreground, 0.06)
        Text {
          id: chipLabel
          anchors.centerIn: parent
          text: modelData.label
          color: row.current === modelData.id ? row.selectedFg : row.foreground
          font.family: row.fontFamily
          font.pixelSize: Style.font.body
          font.bold: row.current === modelData.id
        }
        MouseArea {
          anchors.fill: parent
          cursorShape: Qt.PointingHandCursor
          onClicked: row.picked(modelData.id)
        }
      }
    }
  }
}
