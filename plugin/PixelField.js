.pragma library

var WORD_W = 81
var WORD_H = 19

var WORDMARK = [
  "000000000000000001110000000000000000000000000000000000000000000000000000000000000",
  "001111100000011111111111000000111111100001111111000011111110000100010000001000100",
  "011111110000111111111111100001111111100011111111000111111110001100011000011000110",
  "111000111001110001110001110011100011100111000111001110001110011100011100111000111",
  "111000111001110001110001110011100011100111000111001110001110011100011100111000111",
  "111000111001110001110001110011100011100111000111001110001100011100011100111000111",
  "111000111001110001110001110011100011100111000111001110001000011100011100111000111",
  "111000111001110001110001110011100011100111000111001110000000011100011100111000111",
  "111000111001110001110001110111111111101111111110001110000000111111111110111111111",
  "111000111001110001110001110111111111101111111100001110000001111111111100111111111",
  "111000111001110001110001110011100011100111000000001110000000011100011100000000111",
  "111000111001110001110001110011100011101111111111001110001000011100011100011000111",
  "111000111001110001110001110011100011101111111111001110001100011100011100111000111",
  "111000111001110001110001110011100011100111000111001110001110011100011100111000111",
  "111000111001110001110001110011100011100111000111001110001110011100011100111000111",
  "011111110000110001110001100011100011000111000111001111111100011100011000011111110",
  "001111100000010001110001000011100010000111000111001111111000011100010000001111100",
  "000000000000000000000000000000000000000111000110000000000000000000000000000000000",
  "000000000000000000000000000000000000000111000100000000000000000000000000000000000"
]

var LOGO_SIZE = 15
var LOGO = [
  "111111111111111",
  "100000010000001",
  "101111110001101",
  "101000000000101",
  "101000000000101",
  "101000000000101",
  "101000000000101",
  "111000000000101",
  "101000000000101",
  "101000000000101",
  "101000000000101",
  "101000000000101",
  "101111111111101",
  "100000010000001",
  "111111110111111"
]

var BAYER = [
  0, 32, 8, 40, 2, 34, 10, 42,
  48, 16, 56, 24, 50, 18, 58, 26,
  12, 44, 4, 36, 14, 46, 6, 38,
  60, 28, 52, 20, 62, 30, 54, 22,
  3, 35, 11, 43, 1, 33, 9, 41,
  51, 19, 59, 27, 49, 17, 57, 25,
  15, 47, 7, 39, 13, 45, 5, 37,
  63, 31, 55, 23, 61, 29, 53, 21
]

var LASER_BANDS = [
  "crest", "crest", "crest", "crest", "crest",
  "hover", "hover",
  "lit", "lit", "lit", "lit",
  "mid", "mid", "mid",
  "dim", "dim", "dim", "dim", "dim"
]

var NOISE_SIZE = 64
var LINES = {
  47: "up", 9585: "up",
  92: "down", 9586: "down",
  124: "bar", 9474: "bar", 9475: "bar",
  45: "dash", 9472: "dash", 9473: "dash", 95: "dash"
}

function lit(row, col) {
  if (row < 0 || col < 0 || row >= WORD_H || col >= WORD_W)
    return false
  return WORDMARK[row].charAt(col) === "1"
}

function logoAt(lx, ly) {
  if (lx < 0 || ly < 0 || lx >= LOGO_SIZE || ly >= LOGO_SIZE)
    return false
  return LOGO[ly].charAt(lx) === "1"
}

function bayerAt(row, col) {
  return (BAYER[(row & 7) * 8 + (col & 7)] + 0.5) / 64
}

function bandKey(row) {
  var i = Math.floor((row / WORD_H) * LASER_BANDS.length)
  if (i < 0) i = 0
  if (i >= LASER_BANDS.length) i = LASER_BANDS.length - 1
  return LASER_BANDS[i]
}

function buildNoise(seed) {
  var size = NOISE_SIZE
  var state = seed >>> 0
  function rnd() {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0
    return state / 4294967296
  }
  var field = new Array(size * size)
  var i
  for (i = 0; i < field.length; i++) field[i] = rnd()
  var pass, x, y, dx, dy, sum, sx, sy
  for (pass = 0; pass < 2; pass++) {
    var next = new Array(size * size)
    for (y = 0; y < size; y++) {
      for (x = 0; x < size; x++) {
        sum = 0
        for (dy = -1; dy <= 1; dy++) {
          for (dx = -1; dx <= 1; dx++) {
            sx = (x + dx + size) % size
            sy = (y + dy + size) % size
            sum += field[sy * size + sx]
          }
        }
        next[y * size + x] = sum / 9
      }
    }
    field = next
  }
  var min = 1, max = 0
  for (i = 0; i < field.length; i++) {
    if (field[i] < min) min = field[i]
    if (field[i] > max) max = field[i]
  }
  var span = max - min || 1
  for (i = 0; i < field.length; i++) field[i] = (field[i] - min) / span
  return field
}

function sample(field, x, y) {
  var size = NOISE_SIZE
  var xi = Math.floor(x)
  var yi = Math.floor(y)
  var fx = x - xi
  var fy = y - yi
  var x0 = ((xi % size) + size) % size
  var y0 = ((yi % size) + size) % size
  var x1 = (x0 + 1) % size
  var y1 = (y0 + 1) % size
  var sx = fx * fx * (3 - 2 * fx)
  var sy = fy * fy * (3 - 2 * fy)
  var a = field[y0 * size + x0]
  var b = field[y0 * size + x1]
  var c = field[y1 * size + x0]
  var d = field[y1 * size + x1]
  return (a * (1 - sx) + b * sx) * (1 - sy) + (c * (1 - sx) + d * sx) * sy
}

function describe(symbol) {
  if (symbol === 0x2588) return { kind: "block", weight: 1 }
  if (symbol === 0x2580) return { kind: "part", parts: [[0, 0, 1, 0.5]] }
  if (symbol === 0x2584) return { kind: "part", parts: [[0, 0.5, 1, 0.5]] }
  if (symbol === 0x258c) return { kind: "part", parts: [[0, 0, 0.5, 1]] }
  if (symbol === 0x2590) return { kind: "part", parts: [[0.5, 0, 0.5, 1]] }
  var line = LINES[symbol]
  if (line) return { kind: "line", line: line, weight: 0.2 }
  var ch = String.fromCodePoint(symbol)
  var w = 0.5
  if (".,'`·".indexOf(ch) >= 0) w = 0.12
  else if (":;^~\"".indexOf(ch) >= 0) w = 0.2
  else if ("*+=<>()[]{}!?".indexOf(ch) >= 0) w = 0.35
  else if ("#@%&$MW".indexOf(ch) >= 0) w = 0.7
  return { kind: "mark", weight: w }
}

function packedCss(n) {
  var r = (n >> 16) & 255
  var g = (n >> 8) & 255
  var b = n & 255
  return "rgb(" + r + "," + g + "," + b + ")"
}

function lumaPacked(n) {
  var r = (n >> 16) & 255
  var g = (n >> 8) & 255
  var b = n & 255
  return (r * 299 + g * 587 + b * 114) / 255000
}
