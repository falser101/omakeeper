import QtQuick
import qs.Commons
import "I18n.js" as I18n

Item {
  id: root
  property string animal: "cat"
  property string mood: "idle" // idle | busy | celebrate
  property bool spinning: false
  property real creatureSize: Style.space(280)
  property string headline: ""
  property string subline: ""
  property color foreground: Color.menu.text
  property color accent: Color.accent
  property color urgent: Color.urgent
  property string fontFamily: Style.font.menuFamily
  property bool showCaption: true
  property bool interactive: true
  property string uiLang: "en"

  readonly property bool busy: root.mood === "busy" || root.spinning
  readonly property bool celebrating: root.mood === "celebrate"
  readonly property bool sprinting: root.busy || root.boosted
  // Cat uses an 8-frame photo run cycle; other mascots stay on 6 procedural frames.
  readonly property int frameCount: root.animal === "cat" ? 8 : 6
  property int frame: 0
  property bool boosted: false
  property int boostTicks: 0
  property real runPhase: 0
  property real hoverScale: 1.0
  property real petPulse: 0
  readonly property url pose: Qt.resolvedUrl("assets/" + root.animal + "-" + (root.frame % root.frameCount) + ".png")
  readonly property bool photoSprite: root.animal === "cat"

  signal petted()

  implicitHeight: root.creatureSize + (root.showCaption ? Style.space(88) : 0)

  // Frame cycle — sprint when busy/boosted, soft jog when idle, bounce when celebrate
  Timer {
    interval: root.sprinting ? (root.photoSprite ? 55 : 70)
      : (root.celebrating ? 100 : (root.photoSprite ? 90 : 160))
    running: root.visible
    repeat: true
    onTriggered: {
      root.frame = (root.frame + 1) % root.frameCount
      if (root.boosted) {
        root.boostTicks -= 1
        if (root.boostTicks <= 0)
          root.boosted = false
      }
      if (root.petPulse > 0)
        root.petPulse = Math.max(0, root.petPulse - 0.08)
    }
  }

  // Horizontal run progress (wraps); faster when sprinting
  Timer {
    interval: 16
    running: root.visible
    repeat: true
    onTriggered: {
      var speed = root.sprinting ? 0.028 : (root.celebrating ? 0.012 : 0.014)
      if (root.boosted)
        speed *= 1.7
      root.runPhase = (root.runPhase + speed) % 1.0
    }
  }

  onAnimalChanged: root.frame = 0

  function pet() {
    if (!root.interactive)
      return
    root.boosted = true
    root.boostTicks = 36
    root.petPulse = 1.0
    sparkleBurst.burst()
    dustBurst.burst(true)
    root.petted()
  }

  Item {
    id: stage
    anchors.horizontalCenter: parent.horizontalCenter
    y: 0
    width: parent.width
    height: root.creatureSize
    clip: false

    // Match run-cycle bob (photo cat is 8 frames; others 6)
    readonly property real bobLift: {
      var lifts = root.photoSprite
        ? [0.02, 0.05, 0.1, 0.14, 0.1, 0.06, 0.03, 0.01]
        : [0, 0.02, 0.06, 0.14, 0.1, 0.03]
      return lifts[root.frame % lifts.length] + root.petPulse * 0.08
    }

    // Travel lane: jog in place when idle/celebrate, cross the stage when busy
    readonly property real travelX: {
      if (!root.busy && !root.boosted)
        return parent.width * 0.5 + Math.sin(root.runPhase * Math.PI * 2) * Style.space(18)
      // Ping-pong across the stage
      var lane = root.runPhase < 0.5 ? (root.runPhase * 2) : (2 - root.runPhase * 2)
      var margin = root.creatureSize * 0.55
      return margin + lane * Math.max(Style.space(40), parent.width - margin * 2)
    }
    readonly property bool facingLeft: root.busy && root.runPhase >= 0.5

    // No ground shadow — the dark oval read as a stray bar on the page.

    // Speed lines behind the cat when sprinting
    Repeater {
      model: root.sprinting ? 5 : 0
      Rectangle {
        required property int index
        readonly property real t: (root.runPhase * 3.0 + index * 0.18) % 1.0
        width: Style.space(18) + index * Style.space(6)
        height: Math.max(2, Style.space(2))
        radius: height / 2
        color: Util.alpha(root.accent, 0.18 - index * 0.02)
        x: runner.x - Style.space(28) - t * Style.space(90) - index * Style.space(10)
        y: runner.y + runner.height * (0.28 + index * 0.1)
        opacity: root.boosted ? 0.9 : 0.55
      }
    }

    // Dust puffs under paws
    Repeater {
      id: dustRepeater
      model: dustBurst.particles
      Rectangle {
        required property var modelData
        width: modelData.size
        height: modelData.size * 0.55
        radius: width / 2
        color: Util.alpha(root.accent, modelData.life * 0.45)
        x: modelData.x
        y: modelData.y
        opacity: modelData.life
      }
    }

    // Sparkle / heart burst on pet
    Repeater {
      model: sparkleBurst.particles
      Item {
        required property var modelData
        x: modelData.x
        y: modelData.y
        opacity: modelData.life
        scale: 0.6 + modelData.life * 0.6
        Text {
          anchors.centerIn: parent
          text: modelData.glyph
          color: modelData.heart ? root.urgent : root.accent
          font.pixelSize: modelData.heart ? Style.space(18) : Style.space(12)
        }
      }
    }

    Item {
      id: runner
      width: root.creatureSize
      height: root.creatureSize
      x: stage.travelX - width / 2
      y: -stage.bobLift * root.creatureSize * 0.55
      scale: root.hoverScale * (1.0 + root.petPulse * 0.12)
      transformOrigin: Item.Bottom

      Behavior on scale {
        NumberAnimation { duration: 140; easing.type: Easing.OutBack }
      }

      Image {
        id: sprite
        anchors.fill: parent
        source: root.pose
        fillMode: Image.PreserveAspectFit
        smooth: root.photoSprite
        mipmap: root.photoSprite
        asynchronous: false
        cache: true
        transform: Scale {
          origin.x: sprite.width / 2
          origin.y: sprite.height / 2
          xScale: stage.facingLeft ? -1 : 1
        }
      }

      // Hover glow (soft oval under paws only — no frame around the cat)
      Rectangle {
        visible: petArea.containsMouse && root.interactive
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Style.space(6)
        width: parent.width * 0.55
        height: Style.space(10)
        radius: height / 2
        color: Util.alpha(root.accent, 0.22)
      }

      MouseArea {
        id: petArea
        anchors.fill: parent
        hoverEnabled: root.interactive
        cursorShape: root.interactive ? Qt.PointingHandCursor : Qt.ArrowCursor
        enabled: root.interactive
        onEntered: root.hoverScale = 1.08
        onExited: root.hoverScale = 1.0
        onClicked: root.pet()
      }
    }

    // Ambient floating motes when idle / celebrate
    Repeater {
      model: (!root.busy || root.celebrating) ? 6 : 0
      Rectangle {
        required property int index
        readonly property real seed: index * 1.618
        width: Style.space(4)
        height: width
        radius: width / 2
        color: Util.alpha(root.accent, 0.35)
        x: (parent.width * ((seed * 0.37 + root.runPhase * 0.4) % 1.0))
        y: parent.height * 0.15 + Math.sin((root.runPhase + seed) * Math.PI * 2) * Style.space(24)
          + index * Style.space(8)
        opacity: 0.25 + 0.35 * Math.abs(Math.sin((root.runPhase + seed) * Math.PI))
      }
    }
  }

  // Particle controllers (plain JS objects in lists)
  QtObject {
    id: dustBurst
    property var particles: []
    property int tick: 0

    function burst(heavy) {
      var baseX = runner.x + runner.width * (stage.facingLeft ? 0.65 : 0.35)
      var baseY = runner.y + runner.height * 0.82
      var n = heavy ? 10 : 4
      var next = dustBurst.particles.slice()
      for (var i = 0; i < n; i++) {
        next.push({
          x: baseX + (Math.random() - 0.5) * Style.space(28),
          y: baseY + (Math.random() - 0.3) * Style.space(10),
          size: Style.space(6) + Math.random() * Style.space(10),
          life: 1.0,
          vx: (stage.facingLeft ? 1 : -1) * (1.2 + Math.random() * 2.4),
          vy: -0.4 - Math.random() * 1.2
        })
      }
      if (next.length > 28)
        next = next.slice(next.length - 28)
      dustBurst.particles = next
    }
  }

  QtObject {
    id: sparkleBurst
    property var particles: []

    function burst() {
      var cx = runner.x + runner.width * 0.5
      var cy = runner.y + runner.height * 0.35
      var glyphs = ["✦", "✧", "♥", "·", "✧", "♥"]
      var next = sparkleBurst.particles.slice()
      for (var i = 0; i < 12; i++) {
        var ang = (Math.PI * 2 * i) / 12 + Math.random() * 0.4
        var heart = glyphs[i % glyphs.length] === "♥"
        next.push({
          x: cx,
          y: cy,
          life: 1.0,
          vx: Math.cos(ang) * (2.2 + Math.random() * 2.5),
          vy: Math.sin(ang) * (1.6 + Math.random() * 2.2) - 1.5,
          glyph: glyphs[i % glyphs.length],
          heart: heart
        })
      }
      if (next.length > 36)
        next = next.slice(next.length - 36)
      sparkleBurst.particles = next
    }
  }

  Timer {
    interval: 32
    running: root.visible
    repeat: true
    onTriggered: {
      // Auto dust while sprinting
      dustBurst.tick++
      if (root.sprinting && dustBurst.tick % 3 === 0)
        dustBurst.burst(false)

      function step(list, drag) {
        var out = []
        for (var i = 0; i < list.length; i++) {
          var p = list[i]
          var life = p.life - 0.045
          if (life <= 0)
            continue
          out.push({
            x: p.x + p.vx,
            y: p.y + p.vy,
            size: p.size || Style.space(8),
            life: life,
            vx: p.vx * drag,
            vy: p.vy + 0.12,
            glyph: p.glyph || "·",
            heart: !!p.heart
          })
        }
        return out
      }
      dustBurst.particles = step(dustBurst.particles, 0.92)
      sparkleBurst.particles = step(sparkleBurst.particles, 0.96)
    }
  }

  Text {
    visible: root.showCaption
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: stage.bottom
    anchors.topMargin: Style.spacing.md
    text: root.headline
    color: root.foreground
    font.family: root.fontFamily
    font.pixelSize: Style.font.heading + 8
    font.bold: true
  }

  Text {
    visible: root.showCaption
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: stage.bottom
    anchors.topMargin: Style.spacing.md + Style.font.heading + 14
    text: root.subline + (root.interactive ? ("  ·  " + (root.boosted ? I18n.t("clean.boosted", null, root.uiLang) : I18n.t("clean.pet", null, root.uiLang))) : "")
    color: root.foreground
    opacity: 0.65
    font.family: root.fontFamily
    font.pixelSize: Style.font.body
  }
}
