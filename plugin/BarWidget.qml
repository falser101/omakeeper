import QtQuick
import qs.Commons
import qs.Ui

BarWidget {
  id: root
  moduleName: "io.github.falser101.omakeeper"

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  function toggle() {
    if (!root.bar || !root.bar.shell || typeof root.bar.shell.toggle !== "function")
      return
    root.bar.shell.toggle("io.github.falser101.omakeeper", "{}")
  }

  BarIconButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: "󰃢"
    slotSize: Style.bar.statusSlot
    fontSize: Style.font.caption
    tooltipText: "Omakeeper"
    onPressed: root.toggle()
  }
}
