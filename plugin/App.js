// Shared lockup for every tab's content hero (not idle splash / marquee).
var CONTENT_MARK_SCALE = 0.56

function landingHeroY(parentH, heroH, minTop) {
  minTop = minTop || 24
  return Math.max(minTop, (Number(parentH || 0) - Number(heroH || 0)) / 2)
}

function formatBytes(n) {
  n = Number(n || 0)
  if (n < 1024) return n + " B"
  var units = ["KB", "MB", "GB", "TB"]
  var v = n
  var i = -1
  do {
    v /= 1024
    i++
  } while (v >= 1024 && i < units.length - 1)
  return (v >= 10 ? v.toFixed(1) : v.toFixed(2)) + " " + units[i]
}

function formatBytesPair(used, total) {
  used = Number(used || 0)
  total = Number(total || 0)
  if (total < 1024 && used < 1024)
    return Math.round(used) + " / " + Math.round(total) + " B"
  var units = ["KB", "MB", "GB", "TB"]
  var v = Math.max(used, total)
  var i = -1
  do {
    v /= 1024
    i++
  } while (v >= 1024 && i < units.length - 1)
  function part(n) {
    var x = n
    for (var k = 0; k <= i; k++) x /= 1024
    return x >= 10 ? x.toFixed(1) : x.toFixed(2)
  }
  return part(used) + " / " + part(total) + " " + units[i]
}

function parseJson(text) {
  try {
    return JSON.parse(String(text || ""))
  } catch (e) {
    return null
  }
}

function lastJsonValue(text) {
  var raw = String(text || "").trim()
  if (!raw) return null
  var parsed = parseJson(raw)
  if (parsed) return parsed
  var start = raw.lastIndexOf("\n{")
  if (start >= 0)
    return parseJson(raw.substring(start + 1))
  start = raw.lastIndexOf("\n[")
  if (start >= 0)
    return parseJson(raw.substring(start + 1))
  return null
}

function selectedBytes(items, selected) {
  var total = 0
  for (var i = 0; i < items.length; i++) {
    var it = items[i]
    if (selected[it.id]) total += Number(it.bytes || 0)
  }
  return total
}

function selectedCount(items, selected) {
  var n = 0
  for (var i = 0; i < items.length; i++) {
    if (selected[items[i].id]) n++
  }
  return n
}

function selectedItems(items, selected) {
  var out = []
  for (var i = 0; i < (items || []).length; i++) {
    if (selected && selected[items[i].id]) out.push(items[i])
  }
  return out
}

function fourKMinutes(bytes) {
  var n = Number(bytes || 0) / (258 * 1024 * 1024)
  if (Number(bytes || 0) <= 0) return "0"
  if (n < 1) return "< 1"
  return String(Math.round(n))
}

function selectedIds(items, selected) {
  var out = []
  for (var i = 0; i < items.length; i++) {
    if (selected[items[i].id]) out.push(items[i].id)
  }
  return out
}

function primeSelection(items) {
  var sel = {}
  for (var i = 0; i < items.length; i++) sel[items[i].id] = false
  return sel
}

function allBytes(items) {
  var total = 0
  for (var i = 0; i < items.length; i++) total += Number(items[i].bytes || 0)
  return total
}

function groupClean(items) {
  var preferred = ["ai", "user", "browser", "apps", "dev", "packages", "flatpak", "logs", "leftovers", "downloads", "other"]
  var order = []
  var map = {}
  for (var i = 0; i < items.length; i++) {
    var it = items[i]
    var cat = String(it.category || "other")
    if (!map[cat]) {
      map[cat] = { category: cat, items: [], bytes: 0 }
      order.push(cat)
    }
    map[cat].items.push(it)
    map[cat].bytes += Number(it.bytes || 0)
  }
  order.sort(function(a, b) {
    var ia = preferred.indexOf(a)
    var ib = preferred.indexOf(b)
    if (ia < 0) ia = preferred.length
    if (ib < 0) ib = preferred.length
    return ia - ib
  })
  return order.map(function(c) { return map[c] })
}

function parseLogTs(ts) {
  var s = String(ts || "")
  var m = s.match(/^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})([+-]\d{2})(\d{2})$/)
  if (m) s = m[1] + m[2] + ":" + m[3]
  var t = Date.parse(s)
  return isNaN(t) ? 0 : t
}

function lastOptimizeAt(entries) {
  if (!entries || !entries.length) return 0
  for (var i = 0; i < entries.length; i++) {
    var e = entries[i]
    if (e && e.command === "optimize" && !e.dry_run) {
      var t = parseLogTs(e.ts)
      if (t) return t
    }
  }
  return 0
}

function relativeWhen(ms, tr) {
  if (!ms) return ""
  var sec = Math.max(0, (Date.now() - Number(ms)) / 1000)
  if (sec < 45) return tr("opt.whenJust")
  if (sec < 3600) return tr("opt.whenMinutes", { n: Math.max(1, Math.round(sec / 60)) })
  if (sec < 86400) return tr("opt.whenHours", { n: Math.max(1, Math.round(sec / 3600)) })
  var days = Math.round(sec / 86400)
  if (days < 30) return tr("opt.whenDays", { n: days })
  return tr("opt.whenWeeks", { n: Math.max(1, Math.round(days / 7)) })
}

function optimizeDue(ms, tasks) {
  var journalDue = false
  var list = tasks || []
  for (var i = 0; i < list.length; i++) {
    var t = list[i]
    if ((t.id === "journal-user" || t.id === "journal-system")
        && t.status === "ready"
        && Number(t.bytes || 0) >= 64 * 1024 * 1024) {
      journalDue = true
      break
    }
  }
  if (!ms) return "never"
  if (journalDue) return "journal"
  var age = Date.now() - Number(ms)
  if (age < 20 * 3600 * 1000) return "fresh"
  if (age >= 7 * 86400 * 1000) return "stale"
  return "ok"
}

function historyTotals(entries) {
  var out = { cleaned: 0, uninstalled: 0, optimized: 0, lastOptimizeAt: 0 }
  if (!entries || !entries.length) return out
  for (var i = 0; i < entries.length; i++) {
    var e = entries[i]
    if (e.dry_run) continue
    var n = Number(e.freed_bytes || 0)
    if (e.command === "clean" || e.command === "purge" || e.command === "installer")
      out.cleaned += n
    else if (e.command === "uninstall")
      out.uninstalled += 1
    else if (e.command === "optimize") {
      out.optimized += Number(e.items || 0)
      if (!out.lastOptimizeAt) {
        var t = parseLogTs(e.ts)
        if (t) out.lastOptimizeAt = t
      }
    }
  }
  return out
}

function packageLetter(name) {
  var c = String(name || "").charAt(0).toUpperCase()
  if (c >= "A" && c <= "Z") return c
  return "#"
}

function filterPackages(pkgs, query) {
  var q = String(query || "").trim().toLowerCase()
  var list = pkgs || []
  var out = []
  for (var i = 0; i < list.length; i++) {
    var p = list[i]
    if (q) {
      var hay = (p.name + " " + (p.description || "")).toLowerCase()
      if (hay.indexOf(q) < 0) continue
    }
    out.push(p)
  }
  out.sort(function(a, b) {
    var na = String(a.name || "")
    var nb = String(b.name || "")
    var la = packageLetter(na)
    var lb = packageLetter(nb)
    if (la === "#" && lb !== "#") return -1
    if (lb === "#" && la !== "#") return 1
    var xa = na.toLowerCase()
    var xb = nb.toLowerCase()
    if (xa < xb) return -1
    if (xa > xb) return 1
    return 0
  })
  return out
}

function sumSizes(items) {
  var t = 0
  for (var i = 0; i < items.length; i++) t += Number(items[i].size || 0)
  return t
}

function clampNum(lo, hi, v) {
  return Math.max(lo, Math.min(hi, v))
}

function minTileSize(w, h) {
  return { minW: 96, minH: 72 }
}

function worstAspect(row, side, scale) {
  if (!row.length || side <= 0) return Infinity
  var s = sumSizes(row) * scale
  if (s <= 0) return Infinity
  var worst = 0
  var thick = s / side
  if (thick <= 0) return Infinity
  for (var i = 0; i < row.length; i++) {
    var a = Number(row[i].size) * scale
    var along = a / thick
    if (along <= 0) return Infinity
    var ar = thick > along ? thick / along : along / thick
    if (ar > worst) worst = ar
  }
  return worst
}

function layoutRow(rects, row, x, y, thickW, thickH, vertical) {
  var s = sumSizes(row)
  if (s <= 0) return
  if (vertical) {
    var cy = y
    var left = thickH
    for (var i = 0; i < row.length; i++) {
      var hh = i === row.length - 1 ? left : thickH * (Number(row[i].size) / s)
      if (hh < 0) hh = 0
      rects.push({ x: x, y: cy, w: thickW, h: hh, entry: row[i] })
      cy += hh
      left -= hh
    }
  } else {
    var cx = x
    var leftW = thickW
    for (i = 0; i < row.length; i++) {
      var ww = i === row.length - 1 ? leftW : thickW * (Number(row[i].size) / s)
      if (ww < 0) ww = 0
      rects.push({ x: cx, y: y, w: ww, h: thickH, entry: row[i] })
      cx += ww
      leftW -= ww
    }
  }
}

function remainderEntry(hidden) {
  return {
    name: "",
    path: "",
    size: sumSizes(hidden),
    is_dir: true,
    protected: true,
    remainder: true,
    leftover: hidden.length
  }
}

function squarifyLayout(items, x, y, w, h) {
  var rects = []
  var rest = items.slice()
  while (rest.length) {
    if (w < 1 || h < 1) break
    var total = sumSizes(rest)
    if (total <= 0) break
    var scale = (w * h) / total
    var shortest = Math.min(w, h)
    var vertical = w >= h
    var row = []
    while (rest.length) {
      var trial = row.concat([rest[0]])
      if (!row.length || worstAspect(row, shortest, scale) >= worstAspect(trial, shortest, scale)) {
        row = trial
        rest.shift()
      } else {
        break
      }
    }
    var s = sumSizes(row)
    if (vertical) {
      var colW = w * (s / total)
      layoutRow(rects, row, x, y, colW, h, true)
      x += colW
      w -= colW
    } else {
      var rowH = h * (s / total)
      layoutRow(rects, row, x, y, w, rowH, false)
      y += rowH
      h -= rowH
    }
  }
  return rects
}

function packShown(shown, hidden) {
  var out = shown.slice()
  if (hidden.length) out.push(remainderEntry(hidden))
  return out
}

function tileFits(rect, minW, minH) {
  if (!rect) return false
  var short = Math.min(rect.w, rect.h)
  var long = Math.max(rect.w, rect.h)
  if (rect.w + 0.5 < minW || rect.h + 0.5 < minH) return false
  if (short > 0 && long / short > 4.2) return false
  return true
}

function allTilesFit(rects, minW, minH) {
  for (var i = 0; i < rects.length; i++) {
    if (!tileFits(rects[i], minW, minH)) return false
  }
  return rects.length > 0
}

function layoutDominantWithStrip(main, restItems, x, y, w, h, minW, minH) {
  if (!restItems.length)
    return [{ x: x, y: y, w: w, h: h, entry: main }]
  var rest = remainderEntry(restItems)
  var total = Number(main.size) + Number(rest.size)
  if (total <= 0)
    return [{ x: x, y: y, w: w, h: h, entry: main }]
  var vertical = w >= h
  var span = vertical ? w : h
  var minStrip = vertical ? Math.max(minW, 136) : Math.max(minH, 88)
  var prop = span * (Number(rest.size) / total)
  var thick = Math.max(prop, minStrip)
  thick = Math.min(thick, span * 0.38)
  var mainSpan = span - thick
  var minMain = vertical ? minW : minH
  if (mainSpan < minMain)
    return [{ x: x, y: y, w: w, h: h, entry: main }]
  var rects = []
  if (vertical) {
    layoutRow(rects, [main], x, y, mainSpan, h, true)
    rects.push({ x: x + mainSpan, y: y, w: thick, h: h, entry: rest })
  } else {
    layoutRow(rects, [main], x, y, w, mainSpan, false)
    rects.push({ x: x, y: y + mainSpan, w: w, h: thick, entry: rest })
  }
  return rects
}

function squarify(entries, x, y, w, h) {
  w = Number(w || 0)
  h = Number(h || 0)
  if (w < 8 || h < 8) return []
  var items = []
  for (var i = 0; i < entries.length; i++) {
    if (Number(entries[i].size || 0) > 0) items.push(entries[i])
  }
  if (!items.length) return []
  items.sort(function(a, b) { return Number(b.size) - Number(a.size) })

  var min = minTileSize(w, h)
  var total = sumSizes(items) || 1
  var mapArea = w * h
  var minArea = min.minW * min.minH
  var cols = Math.max(1, Math.floor(w / min.minW))
  var rows = Math.max(1, Math.floor(h / min.minH))
  var maxNamed = Math.round(clampNum(3, 8, cols * rows / 10))

  var shown = []
  for (i = 0; i < items.length && shown.length < maxNamed; i++) {
    var itemArea = mapArea * (Number(items[i].size) / total)
    if (shown.length >= 1 && itemArea < minArea * 0.8)
      break
    shown.push(items[i])
  }
  if (!shown.length) shown = items.slice(0, 1)
  var hidden = items.slice(shown.length)

  function layout() {
    return squarifyLayout(packShown(shown, hidden), x, y, w, h)
  }

  var rects = layout()
  while (hidden.length && shown.length < maxNamed) {
    shown.push(hidden[0])
    hidden = hidden.slice(1)
    var trial = layout()
    if (!allTilesFit(trial, min.minW, min.minH)) {
      hidden.unshift(shown.pop())
      break
    }
    rects = trial
  }
  while (shown.length > 1 && !allTilesFit(rects, min.minW, min.minH)) {
    hidden.unshift(shown.pop())
    rects = layout()
  }
  if (!allTilesFit(rects, min.minW, min.minH) && hidden.length) {
    rects = layoutDominantWithStrip(shown[0], shown.slice(1).concat(hidden), x, y, w, h, min.minW, min.minH)
  }
  return rects
}

function tileColor(index, name) {
  var palette = [
    "#c9a15a", "#c47248", "#b04a3a", "#8a6aa0", "#6a7a88",
    "#c9a35c", "#9b5a3c", "#6e8b6a", "#5a6d8a", "#a07050"
  ]
  return palette[index % palette.length]
}

function healthKey(score) {
  score = Number(score || 0)
  if (score >= 85) return "health.great"
  if (score >= 70) return "health.good"
  if (score >= 50) return "health.ok"
  return "health.high"
}

var ICON_SUFFIXES = ["-bin", "-git", "-debug", "-appimage", "-nightly", "-stable", "-git-bin", "-follow-system"]

function normalizeKey(value) {
  var s = String(value || "").toLowerCase()
  s = s.replace(/\.(desktop|png|svg|xpm|jpg|jpeg)$/g, "")
  var changed = true
  while (changed) {
    changed = false
    for (var i = 0; i < ICON_SUFFIXES.length; i++) {
      var suf = ICON_SUFFIXES[i]
      if (s.length > suf.length && s.substring(s.length - suf.length) === suf) {
        s = s.substring(0, s.length - suf.length)
        changed = true
      }
    }
  }
  return s.replace(/[^a-z0-9]+/g, "")
}

function indexPut(idx, key, icon) {
  if (!idx || !icon) return
  var raw = String(key || "").trim()
  if (!raw) return
  var low = raw.toLowerCase()
  if (!idx[low]) idx[low] = icon
  var n = normalizeKey(raw)
  if (n && !idx[n]) idx[n] = icon
}

function buildDesktopIndex(entries) {
  var idx = {}
  if (!entries) return idx
  for (var i = 0; i < entries.length; i++) {
    var e = entries[i]
    if (!e) continue
    var icon = String(e.icon || "")
    if (!icon) continue
    indexPut(idx, e.id, icon)
    indexPut(idx, e.name, icon)
    indexPut(idx, e.genericName, icon)
    indexPut(idx, e.startupClass, icon)
    indexPut(idx, icon, icon)
    var id = String(e.id || "")
    var parts = id.split(".")
    if (parts.length >= 2) {
      var last = parts[parts.length - 1]
      if (last && last.length >= 3 && last !== "desktop") indexPut(idx, last, icon)
    }
  }
  return idx
}

function buildIndexFromLibrary(appLibrary) {
  if (!appLibrary || typeof appLibrary.sortedEntries !== "function") return {}
  var rows = []
  try { rows = appLibrary.sortedEntries("") || [] } catch (e) { return {} }
  var entries = []
  for (var i = 0; i < rows.length; i++) {
    var e = rows[i] && rows[i].entry
    if (!e) continue
    entries.push({
      id: String(e.id || ""),
      name: String(e.name || ""),
      genericName: String(e.genericName || ""),
      icon: String(e.icon || ""),
      startupClass: String(e.startupClass || "")
    })
  }
  return buildDesktopIndex(entries)
}

function mergeIconMaps(iconFiles, appLibrary, desktopValues) {
  var idx = {}
  var src = iconFiles || {}
  for (var k in src) idx[k] = src[k]
  if (appLibrary && appLibrary.iconIndex) {
    var icons = appLibrary.iconIndex
    for (var name in icons) indexPut(idx, name, icons[name])
  }
  var fromLib = buildIndexFromLibrary(appLibrary)
  for (var key in fromLib) {
    if (!idx[key]) idx[key] = fromLib[key]
  }
  if (desktopValues && desktopValues.length) {
    var entries = []
    for (var i = 0; i < desktopValues.length; i++) {
      var e = desktopValues[i]
      if (!e) continue
      entries.push({
        id: String(e.id || ""),
        name: String(e.name || ""),
        genericName: String(e.genericName || ""),
        icon: String(e.icon || ""),
        startupClass: String(e.startupClass || e.startupWmClass || "")
      })
    }
    var fromDesk = buildDesktopIndex(entries)
    for (var dk in fromDesk) {
      if (!idx[dk]) idx[dk] = fromDesk[dk]
    }
  }
  return idx
}

function cleanCategoryMeta(cat) {
  var key = String(cat || "other")
  var assets = {
    ai: "cat-ai.png",
    user: "cat-user.png",
    browser: "cat-browser.png",
    apps: "cat-apps.png",
    dev: "cat-dev.png",
    packages: "cat-packages.png",
    flatpak: "cat-packages.png",
    logs: "cat-logs.png",
    leftovers: "cat-other.png",
    downloads: "cat-folder.png"
  }
  if (!assets[key]) key = "other"
  return { key: key, asset: assets[key] || "cat-other.png" }
}

function categoryCheck(items, selected, category) {
  var ids = []
  var picked = 0
  var pickedBytes = 0
  var selectable = 0
  var totalBytes = 0
  selected = selected || {}
  category = String(category || "other")
  for (var i = 0; i < items.length; i++) {
    var it = items[i]
    if (String(it.category || "other") !== category) continue
    totalBytes += Number(it.bytes || 0)
    if (it.skip_reason) continue
    selectable++
    ids.push(it.id)
    if (selected[it.id]) {
      picked++
      pickedBytes += Number(it.bytes || 0)
    }
  }
  var state = "none"
  if (selectable > 0 && picked === selectable) state = "all"
  else if (picked > 0) state = "partial"
  return {
    state: state,
    ids: ids,
    selectedBytes: pickedBytes,
    bytes: totalBytes,
    picked: picked,
    selectable: selectable
  }
}

function idsCheck(items, selected, ids) {
  var want = {}
  ids = ids || []
  for (var i = 0; i < ids.length; i++) want[ids[i]] = true
  var subset = []
  for (i = 0; i < (items || []).length; i++) {
    if (want[items[i].id]) subset.push(items[i])
  }
  var picked = 0
  var pickedBytes = 0
  var selectable = 0
  var totalBytes = 0
  var outIds = []
  selected = selected || {}
  for (i = 0; i < subset.length; i++) {
    var it = subset[i]
    totalBytes += Number(it.bytes || 0)
    if (it.skip_reason) continue
    selectable++
    outIds.push(it.id)
    if (selected[it.id]) {
      picked++
      pickedBytes += Number(it.bytes || 0)
    }
  }
  var state = "none"
  if (selectable > 0 && picked === selectable) state = "all"
  else if (picked > 0) state = "partial"
  return {
    state: state,
    ids: outIds,
    selectedBytes: pickedBytes,
    bytes: totalBytes,
    picked: picked,
    selectable: selectable
  }
}

function shortPath(path, home) {
  path = String(path || "")
  home = String(home || "")
  if (home && path.indexOf(home) === 0)
    return "~" + path.substring(home.length)
  return path
}

function isCacheLeafName(name) {
  return name === "Cache" || name === "Code Cache" || name === "GPUCache"
    || name === "CachedData" || name === "DawnCache" || name === "CachedExtensions"
    || name === "ShaderCache" || name === "GPUCache" || name === "blob_storage"
}

function translateOr(tr, key, fallback) {
  if (!tr || !key) return fallback || ""
  var t = tr(key)
  return t && t !== key ? t : (fallback || "")
}

function optimizeLabel(task, tr) {
  if (!task) return ""
  var named = translateOr(tr, "opt.task." + String(task.id || ""), "")
  return named || String(task.label || "")
}

function optimizeDetail(task, tr) {
  if (!task) return ""
  var d = String(task.detail || "")
  if (!d) return ""
  if (d === "already small") return tr("opt.detail.alreadySmall")
  if (d === "nothing to do") return tr("opt.detail.nothing")
  if (d === "no user icon themes") return tr("opt.detail.noIconThemes")
  if (d === "journalctl --user not available") return tr("opt.detail.journalctlMissing")
  if (d.indexOf("needs sudo") === 0) return tr("opt.detail.needsSudo")
  if (d.indexOf("current ") === 0)
    return tr("opt.detail.current", { size: d.substring("current ".length) })
  var themeAt = d.indexOf(" theme")
  if (themeAt > 0) {
    var n = d.substring(0, themeAt)
    if (/^\d+$/.test(n)) return tr("opt.detail.themes", { n: n })
  }
  var missing = " not installed"
  var cut = d.indexOf(missing)
  if (cut > 0 && cut + missing.length === d.length)
    return tr("opt.detail.notInstalled", { bin: d.substring(0, cut) })
  return translateOr(tr, "opt.hint." + String(task.id || ""), d)
}

function joinLocalized(prefix, leaf, lang) {
  if (!leaf) return prefix
  if (lang === "zh") {
    var lc = leaf.charAt(0)
    var pc = prefix.charAt(0)
    var ascii = function(c) {
      return (c >= "A" && c <= "Z") || (c >= "a" && c <= "z") || (c >= "0" && c <= "9")
    }
    if (ascii(lc) || ascii(pc))
      return prefix + " " + leaf
    return prefix + leaf
  }
  return prefix + " " + leaf
}

function cleanItemLabel(it, tr, lang) {
  if (!it) return ""
  var key = String(it.label_key || "")
  if (key) {
    var direct = translateOr(tr, "ai.item." + key, "")
    if (direct) return direct
    var dot = key.indexOf(".")
    if (dot > 0) {
      var p = translateOr(tr, "ai.prefix." + key.substring(0, dot), "")
      var l = translateOr(tr, "ai.leaf." + key.substring(dot + 1), "")
      if (p && l) return joinLocalized(p, l, lang)
    }
  }
  return String(it.label || "")
}

function cleanRowLabel(row, tr, lang) {
  if (row && row.kind === "group" && row.group)
    return translateOr(tr, "ai.group." + row.group, row.label)
  return cleanItemLabel(row, tr, lang)
}

function cleanSkipReason(reason, tr) {
  var s = String(reason || "")
  if (!s) return ""
  if (s.length > 5 && s.substring(s.length - 5) === " busy")
    return tr("clean.busy", { name: s.substring(0, s.length - 5) })
  return s
}

function nestKey(it) {
  if (it && it.group) {
    return {
      key: "g:" + it.category + ":" + it.group,
      title: it.group_label || it.group,
      group: it.group,
      parentPath: "",
      always: true
    }
  }
  var path = String((it && it.path) || "")
  var segs = path.split("/").filter(function(s) { return s.length > 0 })
  if (segs.length < 2) return null
  var last = segs[segs.length - 1]
  if (!isCacheLeafName(last)) return null
  var parent = segs[segs.length - 2]
  var parentPath = "/" + segs.slice(0, -1).join("/")
  return {
    key: "g:" + it.category + ":" + parentPath,
    title: parent,
    parentPath: parentPath
  }
}

function nestCategoryItems(items) {
  var map = {}
  var keys = []
  for (var i = 0; i < items.length; i++) {
    var it = items[i]
    var nk = nestKey(it)
    var k = nk ? nk.key : "leaf:" + it.id
    if (!map[k]) {
      map[k] = { nest: nk, items: [], bytes: 0 }
      keys.push(k)
    }
    map[k].items.push(it)
    map[k].bytes += Number(it.bytes || 0)
  }
  var rows = []
  for (i = 0; i < keys.length; i++) {
    var b = map[keys[i]]
    if (!b.nest || (!b.nest.always && b.items.length < 2)) {
      for (var j = 0; j < b.items.length; j++) {
        rows.push({ kind: "item", item: b.items[j], bytes: Number(b.items[j].bytes || 0) })
      }
    } else {
      rows.push({ kind: "group", nest: b.nest, items: b.items, bytes: b.bytes })
    }
  }
  rows.sort(function(a, b) { return b.bytes - a.bytes })
  return rows
}

function pushCleanItem(out, it, kind, depth) {
  out.push({
    kind: kind,
    id: it.id,
    label: it.label,
    label_key: it.label_key || "",
    group: it.group || "",
    bytes: it.bytes,
    path: it.path,
    category: it.category,
    skip_reason: it.skip_reason,
    depth: depth || 1,
    count: 0,
    open: false,
    childIds: []
  })
}

function groupCleanRows(items, expanded) {
  var groups = groupClean(items)
  var out = []
  expanded = expanded || {}
  for (var i = 0; i < groups.length; i++) {
    var g = groups[i]
    var open = !!expanded[g.category]
    out.push({
      kind: "header",
      id: "h:" + g.category,
      category: g.category,
      bytes: g.bytes,
      count: g.items.length,
      open: open,
      path: "",
      label: g.category,
      depth: 0,
      childIds: []
    })
    if (!open) continue
    var nested = nestCategoryItems(g.items)
    for (var n = 0; n < nested.length; n++) {
      var row = nested[n]
      if (row.kind === "item") {
        pushCleanItem(out, row.item, "item", 1)
        continue
      }
      var gopen = !!expanded[row.nest.key]
      var childIds = []
      for (var c = 0; c < row.items.length; c++) childIds.push(row.items[c].id)
      out.push({
        kind: "group",
        id: row.nest.key,
        category: g.category,
        label: row.nest.title,
        group: row.nest.group || "",
        path: row.nest.parentPath,
        bytes: row.bytes,
        count: row.items.length,
        open: gopen,
        depth: 1,
        skip_reason: "",
        childIds: childIds
      })
      if (!gopen) continue
      for (c = 0; c < row.items.length; c++) {
        var child = row.items[c]
        var leaf = {
          id: child.id,
          label: row.nest.always
            ? child.label
            : (String(child.path || "").split("/").filter(function(s) { return s }).pop() || child.label),
          label_key: child.label_key || "",
          group: child.group || "",
          bytes: child.bytes,
          path: child.path,
          category: child.category,
          skip_reason: child.skip_reason
        }
        pushCleanItem(out, leaf, "child", 2)
      }
    }
  }
  return out
}

function lookupDesktopIcon(name, desktopIndex) {
  if (!name || !desktopIndex) return ""
  var raw = String(name)
  var low = raw.toLowerCase()
  if (desktopIndex[low]) return desktopIndex[low]
  if (desktopIndex[raw]) return desktopIndex[raw]
  var n = normalizeKey(raw)
  if (n && desktopIndex[n]) return desktopIndex[n]
  var s = raw
  var stripped = true
  while (stripped) {
    stripped = false
    for (var i = 0; i < ICON_SUFFIXES.length; i++) {
      var suf = ICON_SUFFIXES[i]
      if (s.length > suf.length && s.substring(s.length - suf.length).toLowerCase() === suf) {
        s = s.substring(0, s.length - suf.length)
        stripped = true
      }
    }
  }
  if (desktopIndex[s.toLowerCase()]) return desktopIndex[s.toLowerCase()]
  var ns = normalizeKey(s)
  if (ns && desktopIndex[ns]) return desktopIndex[ns]
  var parts = s.split(/[-_ ./]+/)
  while (parts.length > 1) {
    parts.pop()
    var prefix = parts.join("-")
    if (desktopIndex[prefix.toLowerCase()]) return desktopIndex[prefix.toLowerCase()]
    var np = normalizeKey(prefix)
    if (np && desktopIndex[np]) return desktopIndex[np]
  }
  return ""
}

function iconFromPath(path, desktopIndex) {
  var segs = String(path || "").split("/")
  var skip = {
    cache: 1, gpu: 1, gpucache: 1, codecache: 1, dawncache: 1, cacheddata: 1,
    config: 1, local: 1, share: 1, state: 1, home: 1, var: 1, app: 1,
    files: 1, info: 1, logs: 1, log: 1, thumbnails: 1, trash: 1
  }
  for (var i = segs.length - 1; i >= 0; i--) {
    var s = segs[i]
    if (!s) continue
    if (s.charAt(0) === ".") s = s.slice(1)
    var key = normalizeKey(s)
    if (!key || skip[key]) continue
    var hit = lookupDesktopIcon(s, desktopIndex)
    if (hit) return hit
  }
  return ""
}

function iconNameForPackage(name, desktopIndex) {
  var hit = lookupDesktopIcon(name, desktopIndex)
  if (hit) return hit
  return String(name || "application-x-executable")
}

function attachPackageIcons(packages, desktopIndex, appLibrary) {
  if (!packages) return packages
  var rows = []
  if (appLibrary && typeof appLibrary.sortedEntries === "function") {
    try { rows = appLibrary.sortedEntries("") || [] } catch (e) { rows = [] }
  }
  for (var i = 0; i < packages.length; i++) {
    var pkg = packages[i]
    if (!pkg) continue
    var name = String(pkg.name || "")
    var icon = lookupDesktopIcon(name, desktopIndex)
    if (!icon && rows.length) {
      var want = normalizeKey(name)
      var parts = String(name).split("-")
      for (var j = 0; j < rows.length && !icon; j++) {
        var e = rows[j] && rows[j].entry
        if (!e) continue
        var idn = normalizeKey(e.id)
        var nm = normalizeKey(e.name)
        if (idn === want || nm === want)
          icon = String(e.icon || "")
      }
      while (!icon && parts.length > 1) {
        parts.pop()
        var prefix = normalizeKey(parts.join("-"))
        if (!prefix) continue
        for (j = 0; j < rows.length && !icon; j++) {
          e = rows[j] && rows[j].entry
          if (!e) continue
          if (normalizeKey(e.id) === prefix || normalizeKey(e.name) === prefix)
            icon = String(e.icon || "")
        }
      }
    }
    if (icon && appLibrary && typeof appLibrary.iconSource === "function") {
      if (icon.indexOf("file://") !== 0 && icon.indexOf("image://") !== 0)
        icon = String(appLibrary.iconSource(icon) || icon)
    }
    pkg.icon = icon || pkg.icon || ""
  }
  return packages
}

var AI_BUNDLED_ICONS = {
  claude: "claude",
  cursor: "cursor",
  codex: "codex",
  grok: "grok",
  gemini: "gemini",
  qwen: "qwen",
  models: "models",
  copilot: "copilot",
  trae: "trae",
  windsurf: "windsurf",
  goose: "goose",
  hermes: "hermes",
  workbuddy: "workbuddy",
  zed: "zed",
  openclaw: "openclaw",
  pi: "pi",
  opencode: "opencode"
}

var AI_GROUP_DESKTOP = {
  claude: ["claude", "claude-desktop"],
  cursor: ["cursor"],
  codex: ["codex"],
  grok: ["grok-bot", "grok"],
  gemini: ["gemini", "antigravity-ide"],
  openclaw: ["openclaw"],
  hermes: ["hermes-desktop", "hermes"],
  trae: ["trae"],
  qwen: ["qwen"],
  opencode: ["opencode", "ai.opencode.desktop"],
  workbuddy: ["workbuddy"],
  pi: ["pi"],
  zed: ["zed"],
  copilot: ["copilot", "github-copilot"],
  continue: ["continue"],
  aider: ["aider"],
  windsurf: ["windsurf", "codeium"],
  amazonq: ["amazon-q"],
  crush: ["crush"],
  goose: ["goose"],
  models: ["ollama", "huggingface"]
}

function aiGroupIcon(group, desktopIndex) {
  group = String(group || "")
  if (AI_BUNDLED_ICONS[group])
    return "assets/agents/" + AI_BUNDLED_ICONS[group] + ".png"
  var list = AI_GROUP_DESKTOP[group] || (group ? [group] : [])
  for (var i = 0; i < list.length; i++) {
    var hit = lookupDesktopIcon(list[i], desktopIndex)
    if (hit) return hit
  }
  return ""
}

function lookupAppIcon(row, desktopIndex) {
  if (!row) return ""
  var label = String(row.label || "")
  var path = String(row.path || "")
  var hit = lookupDesktopIcon(label, desktopIndex)
  if (hit) return hit
  var base = label.replace(/\s+(cache|logs|log)$/i, "")
  if (base && base !== label) {
    hit = lookupDesktopIcon(base, desktopIndex)
    if (hit) return hit
  }
  hit = iconFromPath(path, desktopIndex)
  if (hit) return hit
  var words = label.split(/[\s_/]+/)
  for (var i = 0; i < words.length; i++) {
    var w = words[i]
    if (!w || w.toLowerCase() === "cache" || w.toLowerCase() === "logs") continue
    hit = lookupDesktopIcon(w, desktopIndex)
    if (hit) return hit
  }
  return ""
}

function iconNameForCleanItem(item, desktopIndex) {
  var label = String((item && item.label) || "")
  var path = String((item && item.path) || "")
  var cat = String((item && item.category) || "")
  var low = label.toLowerCase()
  var pathLow = path.toLowerCase()
  var hit = lookupDesktopIcon(label, desktopIndex)
  if (hit) return hit
  if (low.indexOf("firefox") >= 0 || pathLow.indexOf("mozilla") >= 0) return lookupDesktopIcon("firefox", desktopIndex) || "firefox"
  if (low.indexOf("chromium") >= 0 || pathLow.indexOf("chromium") >= 0) return lookupDesktopIcon("chromium", desktopIndex) || "chromium"
  if (low.indexOf("chrome") >= 0) return lookupDesktopIcon("google-chrome", desktopIndex) || "google-chrome"
  if (low.indexOf("brave") >= 0) return lookupDesktopIcon("brave", desktopIndex) || "brave-browser"
  if (low.indexOf("zen") >= 0) return lookupDesktopIcon("zen", desktopIndex) || "zen-browser"
  if (low.indexOf("vivaldi") >= 0) return lookupDesktopIcon("vivaldi", desktopIndex) || "vivaldi"
  if (low.indexOf("edge") >= 0) return lookupDesktopIcon("microsoft-edge", desktopIndex) || "microsoft-edge"
  if (low.indexOf("trash") >= 0) return lookupDesktopIcon("user-trash", desktopIndex) || "user-trash"
  if (low.indexOf("thumbnail") >= 0) return "image-x-generic"
  if (low.indexOf("font") >= 0) return "preferences-desktop-font"
  if (low.indexOf("pip") >= 0 || low.indexOf("pypoetry") >= 0 || low.indexOf("uv ") >= 0) return lookupDesktopIcon("python", desktopIndex) || "python"
  if (low.indexOf("npm") >= 0 || low.indexOf("pnpm") >= 0 || low.indexOf("yarn") >= 0 || low.indexOf("bun") >= 0) return lookupDesktopIcon("nodejs", desktopIndex) || "nodejs"
  if (low.indexOf("cargo") >= 0) return "applications-development"
  if (low.indexOf("yay") >= 0 || low.indexOf("paru") >= 0 || cat === "packages") return "package-x-generic"
  if (cat === "logs") return "text-x-log"
  if (cat === "browser") return lookupDesktopIcon("web-browser", desktopIndex) || "web-browser"
  if (cat === "dev") return "applications-development"
  var fromPath = iconFromPath(path, desktopIndex)
  if (fromPath) return fromPath
  var words = label.split(/[\s_/]+/)
  for (var i = 0; i < words.length; i++) {
    hit = lookupDesktopIcon(words[i], desktopIndex)
    if (hit) return hit
  }
  if (cat === "apps") return "application-x-executable"
  if (cat === "user") return "folder"
  return "application-x-executable"
}

function iconNameForProcess(name, desktopIndex) {
  var raw = String(name || "")
  var hit = lookupDesktopIcon(raw, desktopIndex)
  if (hit) return hit
  var stripped = raw.replace(/(appex|helper|gpu|renderer|plugin|service|bin|daemon)$/i, "")
  if (stripped && stripped !== raw) {
    hit = lookupDesktopIcon(stripped, desktopIndex)
    if (hit) return hit
  }
  var camel = raw.match(/^[A-Z][a-z]+/)
  if (camel) {
    hit = lookupDesktopIcon(camel[0], desktopIndex)
    if (hit) return hit
  }
  return raw || "application-x-executable"
}

function bar(frac, width) {
  frac = Math.max(0, Math.min(1, Number(frac || 0)))
  width = width || 16
  var n = Math.round(frac * width)
  var s = ""
  for (var i = 0; i < width; i++) s += i < n ? "█" : "░"
  return s
}
