import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons
import "PixelField.js" as F

Item {
  id: root

  property string mood: "idle" // idle | busy | celebrate
  property bool spinning: false
  property real creatureSize: Style.space(220)
  property string headline: ""
  property string subline: ""
  property color foreground: Color.menu.text
  property color accent: Color.accent
  property color urgent: Color.urgent
  property string fontFamily: Style.font.menuFamily
  property bool showCaption: true
  property bool interactive: true
  property string uiLang: "en"
  property string animal: "" // ignored; kept so call sites can drop in
  // 1 = hero width (idle). After scan, callers drop this so the mark
  // stays proportional but cedes space to the list.
  property real markScale: 1.0
  // Full-window edge field (homepage four-side dither). No music.
  property bool fillHost: false
  property bool showMark: true
  property bool drawField: true
  property bool marquee: false
  property real marqueePhase: 0
  // Etch only on the idle clean landing. Stamps (click / auto OMA mark) off.
  property bool etch: false
  property bool stamps: false

  readonly property bool busy: root.mood === "busy" || root.spinning
  readonly property bool celebrating: root.mood === "celebrate"

  signal petted()

  readonly property color fieldBg: Color.menu.background
  readonly property color fieldDim: mixColor(Color.menu.background, root.accent, 0.42)
  readonly property color fieldMid: mixColor(Color.menu.background, root.accent, 0.68)
  readonly property color fieldLit: root.accent
  readonly property color fieldHover: mixColor(root.accent, Qt.rgba(1, 1, 1, 1), 0.4)
  readonly property color fieldCrest: mixColor(root.accent, Qt.rgba(1, 1, 1, 1), 0.72)

  readonly property real slotW: {
    var full = Math.min(Math.max(root.width * 0.88, 1), Style.space(896))
    return Math.max(1, full * root.markScale)
  }
  readonly property real wmCW: Math.max(2, root.slotW / F.WORD_W)
  readonly property real wmCH: root.wmCW * 50 / 51
  readonly property real wordH: root.wmCH * F.WORD_H
  readonly property real fieldH: root.wordH * (root.markScale >= 0.9 ? 1.45 : 1.22)

  Behavior on markScale {
    NumberAnimation { duration: 220; easing.type: Easing.OutCubic }
  }

  implicitHeight: stage.height + (root.showCaption ? Style.space(72) : 0)

  property var noise: []
  property var jitter: []
  property var pings: []
  property var etchCells: []
  property bool etching: false
  property bool etchedOnce: false
  property real strength: 0
  property real targetStrength: 0
  property real pointerX: -1e4
  property real pointerY: -1e4
  property real spriteX: 0
  property real spriteY: 0
  property real spriteStrength: 0
  property real spriteStampAt: 0
  property var holding: null
  property real t0: 0

  function mixColor(a, b, t) {
    if (t <= 0) return a
    if (t >= 1) return b
    var ar = a.r, ag = a.g, ab = a.b
    var br = b.r, bg = b.g, bb = b.b
    return Qt.rgba(ar + (br - ar) * t, ag + (bg - ag) * t, ab + (bb - ab) * t, 1)
  }

  function cssOf(c) {
    return "rgb(" + Math.round(c.r * 255) + "," + Math.round(c.g * 255) + "," + Math.round(c.b * 255) + ")"
  }

  function bandCss(key) {
    if (key === "crest") return cssOf(root.fieldCrest)
    if (key === "hover") return cssOf(root.fieldHover)
    if (key === "lit") return cssOf(root.fieldLit)
    if (key === "mid") return cssOf(root.fieldMid)
    return cssOf(root.fieldDim)
  }

  function restInk(row) {
    return bandCss(F.bandKey(row))
  }

  function themeInk(packed) {
    var l = F.lumaPacked(packed)
    var bands = [
      { css: cssOf(root.fieldDim), l: root.fieldDim.r * 0.3 + root.fieldDim.g * 0.59 + root.fieldDim.b * 0.11 },
      { css: cssOf(root.fieldMid), l: root.fieldMid.r * 0.3 + root.fieldMid.g * 0.59 + root.fieldMid.b * 0.11 },
      { css: cssOf(root.fieldLit), l: root.fieldLit.r * 0.3 + root.fieldLit.g * 0.59 + root.fieldLit.b * 0.11 },
      { css: cssOf(root.fieldHover), l: root.fieldHover.r * 0.3 + root.fieldHover.g * 0.59 + root.fieldHover.b * 0.11 },
      { css: cssOf(root.fieldCrest), l: root.fieldCrest.r * 0.3 + root.fieldCrest.g * 0.59 + root.fieldCrest.b * 0.11 }
    ]
    var best = bands[0]
    for (var i = 1; i < bands.length; i++) {
      if (Math.abs(bands[i].l - l) < Math.abs(best.l - l))
        best = bands[i]
    }
    return best.css
  }

  function launch(x, y, charge, now) {
    var from = 0.45 + 1.6 * charge
    var list = root.pings.slice(-3)
    list.push({
      x: x,
      y: y,
      born: now,
      from: from,
      to: (from + 1.0 + 3.2 * charge) * (0.92 + Math.random() * 0.16),
      life: (0.65 + 0.55 * charge) * (0.92 + Math.random() * 0.16)
    })
    root.pings = list
  }

  function startEtch() {
    if (!root.etch || root.marquee || !root.showMark || !root.visible || etchProc.running)
      return
    root.etchCells = []
    root.etching = true
    etchProc.running = true
  }

  Component.onCompleted: {
    root.noise = F.buildNoise(0x9ece6a)
    var tile = []
    var state = 0x0a1f14
    for (var i = 0; i < 64 * 64; i++) {
      state = (Math.imul(state, 1664525) + 1013904223) >>> 0
      tile.push(state / 4294967296)
    }
    root.jitter = tile
    root.t0 = Date.now()
    if (root.visible && root.etch)
      Qt.callLater(root.startEtch)
  }

  onVisibleChanged: {
    if (root.visible && root.etch && !root.etchedOnce)
      root.startEtch()
    if (!root.visible && etchProc.running)
      etchProc.running = false
  }

  onMarkScaleChanged: canvas.requestPaint()

  Item {
    id: stage
    x: root.fillHost ? 0 : Math.round((parent.width - width) / 2)
    y: 0
    width: root.fillHost ? parent.width : Math.ceil(F.WORD_W * root.wmCW)
    height: root.fillHost ? parent.height : root.fieldH
    clip: true

    Canvas {
      id: canvas
      anchors.fill: parent
      antialiasing: false
      renderTarget: Canvas.Image
      renderStrategy: Canvas.Cooperative

      onPaint: {
        var ctx = getContext("2d")
        if (!ctx)
          return
        var noise = root.noise
        if (!noise || !noise.length)
          return

        var w = canvas.width
        var h = canvas.height
        var dpr = 1
        var wmCW = root.wmCW
        var wmCH = root.wmCH
        var wmX = (w - F.WORD_W * wmCW) / 2
        var wmY = (h - F.WORD_H * wmCH) / 2
        var cMin = -Math.ceil(wmX / wmCW) - 1
        var rMin = -Math.ceil(wmY / wmCH) - 1
        var cols = Math.ceil((w - wmX) / wmCW) - cMin + 1
        var rows = Math.ceil((h - wmY) / wmCH) - rMin + 1
        var now = Date.now()
        var t = ((now - root.t0) / 1000)
        if (root.busy)
          t *= 2.4
        else if (root.celebrating)
          t *= 1.25

        var reachCells = root.busy ? 16 : 12

        if (root.drawField)
          ctx.fillStyle = cssOf(root.fieldBg)
        else
          ctx.clearRect(0, 0, w, h)
        if (root.drawField)
          ctx.fillRect(0, 0, w, h)

        var glows = []
        if (root.strength > 0.01) {
          glows.push({
            x: root.pointerX,
            y: root.pointerY,
            strength: root.strength,
            reach: reachCells * wmCW * (0.45 + 0.55 * root.strength)
          })
        }
        if (root.spriteStrength > 0.01) {
          glows.push({
            x: root.spriteX,
            y: root.spriteY,
            strength: root.spriteStrength,
            reach: reachCells * wmCW * (0.45 + 0.55 * root.spriteStrength)
          })
        }

        var stamps = []
        var p
        for (p = 0; p < root.pings.length; p++) {
          var ping = root.pings[p]
          var age = (now - ping.born) / 1000 / ping.life
          if (age >= 1)
            continue
          var grow = 1 - Math.pow(1 - age, 3)
          stamps.push({
            x: ping.x,
            y: ping.y,
            cellPx: wmCW * (ping.from + (ping.to - ping.from) * grow),
            amp: Math.pow(1 - age, 1.7)
          })
        }
        if (root.holding) {
          var charge = Math.min((now - root.holding.start) / 1100, 1)
          stamps.push({
            x: root.holding.x,
            y: root.holding.y,
            cellPx: wmCW * (0.45 + 1.6 * charge),
            amp: 0.9
          })
        }

        function stampAt(cx, cy) {
          var amp = 0
          for (var s = 0; s < stamps.length; s++) {
            var st = stamps[s]
            var lx = Math.floor((cx - st.x) / st.cellPx + F.LOGO_SIZE / 2)
            var ly = Math.floor((cy - st.y) / st.cellPx + F.LOGO_SIZE / 2)
            if (F.logoAt(lx, ly) && st.amp > amp)
              amp = st.amp
          }
          return amp
        }

        var jitter = root.jitter
        var r, c, col, row, yTop, y, cellH, cy, xLeft, cx, shade, nx, ny, rr, lum
        var u, v, base, twinkle, glowAmount, g, dx, dy, dist, falloff, amount
        var waveAmount, heat, threshold, x

        for (r = 0; root.drawField && r < rows; r++) {
          row = rMin + r
          yTop = wmY + row * wmCH
          y = Math.round(yTop)
          cellH = Math.round(yTop + wmCH) - y
          cy = yTop + wmCH / 2
          ny = (cy / h) * 2 - 1
          for (c = 0; c < cols; c++) {
            col = cMin + c
            if (root.showMark && F.lit(row, col) && !root.etching)
              continue

            xLeft = wmX + col * wmCW
            cx = xLeft + wmCW / 2
            nx = (cx / w) * 2 - 1
            rr = Math.sqrt(nx * nx + ny * ny * 0.82)
            shade = Math.min(1, Math.max(0, (rr - 0.42) / 0.85))
            shade = shade * shade
            shade *= Math.min(1, Math.max(0.2, cy / 70))
            if (root.busy)
              shade = Math.min(1, shade * 1.28 + 0.08)

            lum = 0
            if (shade > 0.002 && noise.length) {
              u = col / 9
              v = row / 9
              base = 0.6 * F.sample(noise, u + t * 0.14, v - t * 0.055)
                + 0.4 * F.sample(noise, u * 0.55 - t * 0.08, v * 0.55 + t * 0.06)
              twinkle = 0.5 + 0.5 * Math.sin(t * 1.1 + (jitter[(row * 37 + col * 11) & 4095] || 0) * 6.283)
              lum = shade * (0.3 + 0.52 * base * base + 0.18 * twinkle) * 0.62
            }

            glowAmount = 0
            for (g = 0; g < glows.length; g++) {
              dx = cx - glows[g].x
              dy = cy - glows[g].y
              dist = Math.sqrt(dx * dx + dy * dy)
              if (dist < glows[g].reach) {
                falloff = 1 - dist / glows[g].reach
                amount = falloff * falloff * glows[g].strength
                if (amount > glowAmount)
                  glowAmount = amount
              }
            }
            lum += glowAmount * 0.6
            waveAmount = stamps.length ? stampAt(cx, cy) : 0
            lum += waveAmount * 1.15

            threshold = 0.78 * F.bayerAt(row, col) + 0.22 * (jitter[(row & 63) * 64 + (col & 63)] || 0)
            if (lum <= threshold)
              continue

            heat = Math.max(glowAmount, waveAmount)
            ctx.fillStyle = heat > 0.34 ? cssOf(root.fieldLit)
              : (heat > 0.1 ? cssOf(root.fieldMid) : cssOf(root.fieldDim))
            x = Math.round(xLeft)
            ctx.fillRect(x, y, Math.round(xLeft + wmCW) - x, cellH)
          }
        }

        if (root.showMark && !root.etching) {
          var origins = []
          if (root.marquee) {
            var markW = F.WORD_W * wmCW
            var period = markW + wmCW * 12
            var ox = -((root.marqueePhase % 1) * period)
            while (ox < w) {
              origins.push(ox)
              ox += period
            }
          } else {
            origins.push(wmX)
          }
          var oi
          for (oi = 0; oi < origins.length; oi++) {
            var origin = origins[oi]
            for (row = 0; row < F.WORD_H; row++) {
              yTop = wmY + row * wmCH
              y = Math.round(yTop)
              cellH = Math.round(yTop + wmCH) - y
              for (col = 0; col < F.WORD_W; col++) {
                if (!F.lit(row, col))
                  continue
                xLeft = origin + col * wmCW
                cx = xLeft + wmCW / 2
                cy = yTop + wmCH / 2
                waveAmount = stamps.length ? stampAt(cx, cy) : 0
                glowAmount = 0
                for (g = 0; g < glows.length; g++) {
                  dx = cx - glows[g].x
                  dy = cy - glows[g].y
                  dist = Math.sqrt(dx * dx + dy * dy)
                  if (dist < glows[g].reach) {
                    falloff = 1 - dist / glows[g].reach
                    amount = falloff * falloff * glows[g].strength
                    if (amount > glowAmount)
                      glowAmount = amount
                  }
                }
                var crest = Math.max(waveAmount, glowAmount)
                ctx.fillStyle = crest > 0.45 ? cssOf(root.fieldCrest)
                  : (crest > 0.12 ? cssOf(root.fieldHover) : root.restInk(row))
                x = Math.round(xLeft)
                ctx.fillRect(x, y, Math.round(xLeft + wmCW) - x, cellH)
              }
            }
          }
        }

        var cells = root.etchCells
        if (root.showMark && root.etching && cells && cells.length) {
          var stroke = Math.max(1, Math.round(wmCW / 5))
          ctx.lineCap = "butt"
          for (var i = 0; i < cells.length; i++) {
            var cell = cells[i]
            col = Number(cell.c)
            row = Number(cell.r)
            xLeft = wmX + col * wmCW
            yTop = wmY + row * wmCH
            x = Math.round(xLeft)
            y = Math.round(yTop)
            var cw = Math.round(xLeft + wmCW) - x
            var ch = Math.round(yTop + wmCH) - y
            var desc = F.describe(Number(cell.s))
            var ink = root.themeInk(Number(cell.x))
            if (desc.kind === "block") {
              ctx.fillStyle = ink
              ctx.fillRect(x, y, cw, ch)
              continue
            }
            if (desc.kind === "part" && desc.parts) {
              ctx.fillStyle = ink
              for (var pi = 0; pi < desc.parts.length; pi++) {
                var pr = desc.parts[pi]
                var x0 = Math.round(xLeft + pr[0] * wmCW)
                var y0 = Math.round(yTop + pr[1] * wmCH)
                ctx.fillRect(
                  x0, y0,
                  Math.max(1, Math.round(xLeft + (pr[0] + pr[2]) * wmCW) - x0),
                  Math.max(1, Math.round(yTop + (pr[1] + pr[3]) * wmCH) - y0)
                )
              }
              continue
            }
            if (desc.kind === "line") {
              ctx.strokeStyle = ink
              ctx.lineWidth = stroke
              ctx.beginPath()
              if (desc.line === "bar") {
                ctx.moveTo(x + cw / 2, y)
                ctx.lineTo(x + cw / 2, y + ch)
              } else if (desc.line === "dash") {
                ctx.moveTo(x, y + ch / 2)
                ctx.lineTo(x + cw, y + ch / 2)
              } else if (desc.line === "down") {
                ctx.moveTo(x, y)
                ctx.lineTo(x + cw, y + ch)
              } else {
                ctx.moveTo(x, y + ch)
                ctx.lineTo(x + cw, y)
              }
              ctx.stroke()
              continue
            }
            ctx.fillStyle = ink
            var size = Math.max(1, Math.round(cw * Math.sqrt(desc.weight || 0.4)))
            ctx.fillRect(x + Math.round((cw - size) / 2), y + Math.round((ch - size) / 2), size, size)
          }
        }
      }
    }

    MouseArea {
      id: hover
      anchors.fill: parent
      hoverEnabled: root.interactive
      acceptedButtons: root.interactive ? Qt.LeftButton : Qt.NoButton
      onPositionChanged: function(ev) {
        root.pointerX = ev.x
        root.pointerY = ev.y
        root.targetStrength = root.interactive ? 1 : 0
      }
      onExited: root.targetStrength = 0
      onPressed: function(ev) {
        if (!root.interactive || !root.stamps)
          return
        root.targetStrength = 0
        root.holding = { x: ev.x, y: ev.y, start: Date.now() }
      }
      onReleased: function(ev) {
        if (!root.holding)
          return
        if (root.stamps) {
          var now = Date.now()
          var charge = Math.min((now - root.holding.start) / 1100, 1)
          root.launch(root.holding.x, root.holding.y, charge, now)
        }
        root.holding = null
        root.targetStrength = hover.containsMouse ? 1 : 0
        root.petted()
      }
      onCanceled: root.holding = null
    }
  }

  Column {
    visible: root.showCaption
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: stage.bottom
    anchors.topMargin: Style.spacing.sm
    width: parent.width
    spacing: 4
    Text {
      width: parent.width
      text: root.headline
      color: root.foreground
      horizontalAlignment: Text.AlignHCenter
      font.family: root.fontFamily
      font.pixelSize: Style.font.heading
      font.bold: true
      wrapMode: Text.WordWrap
    }
    Text {
      width: parent.width
      text: root.subline
      color: root.foreground
      opacity: 0.62
      horizontalAlignment: Text.AlignHCenter
      font.family: root.fontFamily
      font.pixelSize: Style.font.caption
      wrapMode: Text.WordWrap
    }
  }

  Timer {
    interval: 25
    running: root.visible
    repeat: true
    onTriggered: {
      var now = Date.now()
      var ts = (now - root.t0) / 1000
      var rx = 0.44 * (1 + 0.1 * Math.sin(ts * 0.11))
      var ry = 0.38 * (1 + 0.1 * Math.sin(ts * 0.09 + 2))
      root.spriteX = stage.width * (0.5 + rx * Math.sin(ts * 0.65))
      root.spriteY = stage.height * (0.48 + ry * Math.sin(ts * 0.39 + 1.1))
      var goal = 0
      if (!root.etching) {
        if (root.busy)
          goal = 0.62
        else if (root.stamps)
          goal = 0.28
      }
      root.spriteStrength += (goal - root.spriteStrength) * 0.08
      root.strength += (root.targetStrength - root.strength) * 0.3

      if (root.marquee)
        root.marqueePhase = (root.marqueePhase + (root.busy ? 0.016 : 0.01)) % 1

      if (root.stamps && !root.etching) {
        var wait = root.busy ? 1600 : 4000
        if (root.spriteStampAt === 0)
          root.spriteStampAt = now + 4000
        if (now >= root.spriteStampAt) {
          root.launch(root.spriteX, root.spriteY, 0.06 + Math.random() * 0.08, now)
          root.spriteStampAt = now + wait + Math.random() * 1200
        }
      }
      if (root.pings.length) {
        var keep = []
        for (var i = 0; i < root.pings.length; i++) {
          if ((now - root.pings[i].born) / 1000 < root.pings[i].life)
            keep.push(root.pings[i])
        }
        if (keep.length !== root.pings.length)
          root.pings = keep
      }
      canvas.requestPaint()
    }
  }

  Process {
    id: etchProc
    running: false
    command: [(Quickshell.env("HOME") || "") + "/.cargo/bin/omakeeper", "etch", "--json"]
    stdout: SplitParser {
      onRead: function(line) {
        var data = null
        try { data = JSON.parse(String(line)) } catch (e) { return }
        if (!data || !data.event)
          return
        if (data.event === "frame")
          root.etchCells = data.cells || []
        else if (data.event === "done" || data.event === "error") {
          root.etching = false
          root.etchedOnce = true
          root.etchCells = []
        }
      }
    }
    onExited: function(code) {
      root.etching = false
      root.etchedOnce = true
      if (code !== 0)
        root.etchCells = []
    }
  }
}
