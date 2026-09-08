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
  return order.map(function(c) { return map[c] })
}

function historyTotals(entries) {
  var out = { cleaned: 0, uninstalled: 0, optimized: 0 }
  if (!entries || !entries.length) return out
  for (var i = 0; i < entries.length; i++) {
    var e = entries[i]
    if (e.dry_run) continue
    var n = Number(e.freed_bytes || 0)
    if (e.command === "clean" || e.command === "purge" || e.command === "installer")
      out.cleaned += n
    else if (e.command === "uninstall")
      out.uninstalled += 1
    else if (e.command === "optimize")
      out.optimized += Number(e.items || 0)
  }
  return out
}

function filterPackages(pkgs, query) {
  var q = String(query || "").trim().toLowerCase()
  if (!q) return pkgs
  var out = []
  for (var i = 0; i < pkgs.length; i++) {
    var p = pkgs[i]
    var hay = (p.name + " " + (p.description || "")).toLowerCase()
    if (hay.indexOf(q) >= 0) out.push(p)
  }
  return out
}

function squarify(entries, x, y, w, h) {
  var items = []
  var total = 0
  for (var i = 0; i < entries.length; i++) {
    var sz = Number(entries[i].size || 0)
    if (sz <= 0) continue
    items.push(entries[i])
    total += sz
  }
  if (total <= 0 || w <= 0 || h <= 0) return []
  items.sort(function(a, b) { return Number(b.size) - Number(a.size) })
  var maxTiles = Math.min(items.length, 18)
  items = items.slice(0, maxTiles)
  total = 0
  for (i = 0; i < items.length; i++) total += Number(items[i].size)
  return sliceDice(items, total, x, y, w, h, true)
}

function sliceDice(items, total, x, y, w, h, vertical) {
  var rects = []
  var cursor = vertical ? y : x
  for (var i = 0; i < items.length; i++) {
    var frac = Number(items[i].size) / total
    var span = (vertical ? h : w) * frac
    var rect = vertical
      ? { x: x, y: cursor, w: w, h: span }
      : { x: cursor, y: y, w: span, h: h }
    rect.entry = items[i]
    rects.push(rect)
    cursor += span
  }
  return rects
}

function tileColor(index, name) {
  var palette = [
    "#d6b36a", "#c47a4a", "#b04a3a", "#8a6aa8", "#6a7a8a",
    "#c9a35c", "#9b5a3c", "#7a8b6a", "#5a6d8a", "#a07050"
  ]
  return palette[index % palette.length]
}

function healthLabel(score) {
  score = Number(score || 0)
  if (score >= 85) return "很好"
  if (score >= 70) return "良好"
  if (score >= 50) return "一般"
  return "压力偏高"
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
  var all = {
    user: {
      title: "用户缓存",
      hint: "回收站、缩略图和字体缓存",
      asset: "cat-user.png"
    },
    browser: {
      title: "浏览器",
      hint: "浏览器缓存。Cookie 与登录保持不变",
      asset: "cat-browser.png"
    },
    dev: {
      title: "开发工具",
      hint: "编译器和包管理器缓存，下次构建会变慢",
      asset: "cat-dev.png"
    },
    packages: {
      title: "软件包",
      hint: "AUR 助手缓存",
      asset: "cat-packages.png"
    },
    apps: {
      title: "应用缓存",
      hint: "应用临时文件，下次启动会重新生成",
      asset: "cat-apps.png"
    },
    logs: {
      title: "日志",
      hint: "诊断日志。占用中的文件会跳过",
      asset: "cat-logs.png"
    }
  }
  return all[key] || {
    title: key,
    hint: "其他可清理项",
    asset: "cat-other.png"
  }
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

function nestKey(it) {
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
    if (!b.nest || b.items.length < 2) {
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
          label: String(child.path || "").split("/").filter(function(s) { return s }).pop() || child.label,
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
