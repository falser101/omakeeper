.pragma library

function systemLang() {
  var n = ""
  try { n = String(Qt.locale().name || "") } catch (e) { n = "" }
  if (!n) {
    try {
      var langs = Qt.locale().uiLanguages
      if (langs && langs.length) n = String(langs[0])
    } catch (e2) { n = "" }
  }
  n = n.toLowerCase().replace("-", "_")
  return n.indexOf("zh") === 0 ? "zh" : "en"
}

function resolveLang(pref) {
  if (pref === "zh" || pref === "en") return pref
  return systemLang()
}

function t(key, vars, lang) {
  var code = resolveLang(lang)
  var table = code === "zh" ? ZH : EN
  var s = table[key]
  if (s === undefined) s = EN[key]
  if (s === undefined) s = key
  if (vars) {
    for (var k in vars)
      s = String(s).split("{" + k + "}").join(String(vars[k]))
  }
  return s
}

var EN = {
  "tab.clean": "Clean",
  "tab.software": "Software",
  "tab.optimize": "Optimize",
  "tab.analyze": "Analyze",
  "tab.status": "Status",
  "tab.settings": "Settings",
  "esc.close": "Esc to close",
  "confirm": "Confirm",
  "cancel": "Cancel",

  "cat.user.title": "User cache",
  "cat.user.hint": "Trash, thumbnails, and font caches",
  "cat.browser.title": "Browsers",
  "cat.browser.hint": "Browser caches. Cookies and logins stay",
  "cat.dev.title": "Dev tools",
  "cat.dev.hint": "Compiler and package-manager caches; next build is slower",
  "cat.packages.title": "Packages",
  "cat.packages.hint": "AUR helper caches",
  "cat.apps.title": "App caches",
  "cat.apps.hint": "App temp files, rebuilt on next launch",
  "cat.logs.title": "Logs",
  "cat.logs.hint": "Diagnostic logs. Busy files are skipped",
  "cat.other.title": "Other",
  "cat.other.hint": "Other cleanable items",
  "cat.selected": "{picked}/{total} selected",

  "clean.ready": "Ready to clean",
  "clean.scanning": "Scanning",
  "clean.applying": "Cleaning",
  "clean.scanHint": "Tap Clean below, or tap the cat to sprint",
  "clean.scanningHint": "The cat is scanning caches",
  "clean.applyingHint": "Deleting selected items",
  "clean.doneHint": "Cleaned · total {bytes}",
  "clean.reviewHint": "{count} / {total} selected · tap a category to expand",
  "clean.sprint": "Sprinting · {name}  ·  {n}/{t}",
  "clean.doneSub": "≈ {mins} min of 4K  ·  {skipped} skipped  ·  total {bytes}",
  "clean.back": "Back to clean",
  "clean.scan": "Clean",
  "clean.scanningBtn": "Scanning…",
  "clean.apply": "Clean · {bytes}",
  "clean.applyingBtn": "Cleaning…",
  "clean.selectedCount": "{n} selected",
  "clean.selectAll": "Select all",
  "clean.selectNone": "Select none",
  "clean.rescan": "Rescan",
  "clean.ask": "Delete {bytes} of cache?",
  "clean.boosted": "sprinting",
  "clean.pet": "tap the cat",

  "soft.remove": "Uninstall",
  "soft.updates": "Updates",
  "soft.autostart": "Startup",
  "soft.search": "Search software",
  "soft.noneLeft": "  No extra leftovers, or still scanning…",
  "soft.updatesSoon": "App updates still go through pacman / omarchy update",
  "soft.autostartSoon": "Startup-item management comes in a later version",
  "soft.selected": "{n} apps  ·  {bytes}",
  "soft.removeN": "Remove {n}",
  "soft.removing": "Removing…",
  "soft.ask": "Uninstall {n} apps and leftovers? Needs admin.",

  "opt.title": "System optimize",
  "opt.scanning": "Checking maintenance tasks",
  "opt.applying": "Deep-optimizing the system",
  "opt.readyHint": "{n} ready · user-level, no kernel changes",
  "opt.progress": "Done {n} / {t}",
  "opt.refresh": "Refresh",
  "opt.start": "Start optimize",
  "opt.running": "Optimizing…",
  "opt.ask": "Run the selected maintenance tasks?",

  "analyze.scanning": "Scanning disk…",
  "analyze.ask": "Move this item to Trash?\n{path}",

  "status.network": "Network",
  "status.zombies": "Zombie processes {n}",
  "status.cpuBars": "CPU bars",
  "status.proc": "Process",
  "status.pid": "PID",
  "status.cpu": "CPU",
  "status.mem": "Memory",
  "status.health": "Health",
  "status.disk": "Disk",
  "status.load": "Load {load} / {n} cores",
  "status.used": "used {pct}",
  "health.great": "Great",
  "health.good": "Good",
  "health.ok": "Fair",
  "health.high": "Stressed",
  "analyze.overview": "Overview",
  "opt.done": "Optimize finished",
  "clean.items": "{n} items",

  "settings.title": "Settings",
  "settings.language": "Language",
  "settings.appearance": "Appearance",
  "settings.followSystem": "Follow system",
  "settings.zh": "Chinese",
  "settings.en": "English",
  "settings.light": "Light",
  "settings.dark": "Dark",
  "settings.langHint": "Chinese, English, or match the system locale",
  "settings.themeHint": "Light, dark, or match the current Omarchy theme"
}

var ZH = {
  "tab.clean": "清理",
  "tab.software": "软件",
  "tab.optimize": "优化",
  "tab.analyze": "分析",
  "tab.status": "状态",
  "tab.settings": "设置",
  "esc.close": "Esc 关闭窗口",
  "confirm": "确认",
  "cancel": "取消",

  "cat.user.title": "用户缓存",
  "cat.user.hint": "回收站、缩略图和字体缓存",
  "cat.browser.title": "浏览器",
  "cat.browser.hint": "浏览器缓存。Cookie 与登录保持不变",
  "cat.dev.title": "开发工具",
  "cat.dev.hint": "编译器和包管理器缓存，下次构建会变慢",
  "cat.packages.title": "软件包",
  "cat.packages.hint": "AUR 助手缓存",
  "cat.apps.title": "应用缓存",
  "cat.apps.hint": "应用临时文件，下次启动会重新生成",
  "cat.logs.title": "日志",
  "cat.logs.hint": "诊断日志。占用中的文件会跳过",
  "cat.other.title": "其他",
  "cat.other.hint": "其他可清理项",
  "cat.selected": "{picked}/{total} 已选",

  "clean.ready": "准备清理",
  "clean.scanning": "正在扫描",
  "clean.applying": "正在清理",
  "clean.scanHint": "点下方清理，或点猫咪加速慢跑",
  "clean.scanningHint": "猫咪奔跑扫描中",
  "clean.applyingHint": "正在删除已选项",
  "clean.doneHint": "已清理 · 累计 {bytes}",
  "clean.reviewHint": "{count} / {total} 项已选 · 点分类展开明细",
  "clean.sprint": "狂奔清理 · {name}  ·  {n}/{t}",
  "clean.doneSub": "≈ {mins} 分钟 4K  ·  {skipped} 已跳过  ·  累计 {bytes}",
  "clean.back": "回到清理",
  "clean.scan": "清理",
  "clean.scanningBtn": "扫描中…",
  "clean.apply": "清理 · {bytes}",
  "clean.applyingBtn": "清理中…",
  "clean.selectedCount": "{n} 项已选",
  "clean.selectAll": "全选",
  "clean.selectNone": "全不选",
  "clean.rescan": "重新扫描",
  "clean.ask": "删除 {bytes} 缓存？",
  "clean.boosted": "加速中",
  "clean.pet": "点猫咪互动",

  "soft.remove": "卸载",
  "soft.updates": "更新",
  "soft.autostart": "启动项",
  "soft.search": "搜索软件",
  "soft.noneLeft": "  无额外残留，或仍在扫描…",
  "soft.updatesSoon": "软件更新仍走 pacman / omarchy update",
  "soft.autostartSoon": "启动项管理将在后续版本加入",
  "soft.selected": "{n} 个软件  ·  {bytes}",
  "soft.removeN": "移除 {n} 项",
  "soft.removing": "移除中…",
  "soft.ask": "卸载 {n} 个软件及其残留？需要管理员权限。",

  "opt.title": "系统优化",
  "opt.scanning": "检查维护任务",
  "opt.applying": "正在深度优化系统",
  "opt.readyHint": "{n} 项就绪 · 用户态维护，不碰内核",
  "opt.progress": "已完成 {n} / {t}",
  "opt.refresh": "刷新",
  "opt.start": "开始优化",
  "opt.running": "优化中…",
  "opt.ask": "执行选中的系统维护任务？",

  "analyze.scanning": "扫描磁盘…",
  "analyze.ask": "将此项移到回收站？\n{path}",

  "status.network": "网络",
  "status.zombies": "僵尸进程 {n}",
  "status.cpuBars": "CPU 柱状",
  "status.proc": "进程",
  "status.pid": "PID",
  "status.cpu": "CPU",
  "status.mem": "内存",
  "status.health": "健康度",
  "status.disk": "磁盘",
  "status.load": "负载 {load} / {n} 核",
  "status.used": "已用 {pct}",
  "health.great": "很好",
  "health.good": "良好",
  "health.ok": "一般",
  "health.high": "压力偏高",
  "analyze.overview": "概览",
  "opt.done": "优化完成",
  "clean.items": "{n} 项",

  "settings.title": "设置",
  "settings.language": "语言",
  "settings.appearance": "外观",
  "settings.followSystem": "跟随系统",
  "settings.zh": "中文",
  "settings.en": "English",
  "settings.light": "亮色",
  "settings.dark": "暗色",
  "settings.langHint": "中文、英文，或跟随系统语言",
  "settings.themeHint": "亮色、暗色，或跟随当前 Omarchy 主题"
}
