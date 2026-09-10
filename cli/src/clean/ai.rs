use super::Candidate;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const KEEP: &[&str] = &[
    "claude",
    "cursor",
    "codex",
    "grok",
    "gemini",
    "openclaw",
    "hermes",
    "qwen",
    "opencode",
    "workbuddy",
    "antigravity",
    "ollama",
    "huggingface",
    "continue",
    "aider",
    "windsurf",
    "codeium",
    "crush",
    "goose",
    "tabnine",
    "copilot",
    "cline",
    "trae",
    "lmstudio",
    "lm-studio",
    "amazon-q",
];

const ELECTRON_LEAVES: &[&str] = &[
    "Cache",
    "Code Cache",
    "GPUCache",
    "CachedData",
    "DawnCache",
    "DawnWebGPUCache",
    "DawnGraphiteCache",
    "CachedExtensionVSIXs",
    "CachedProfilesData",
    "blob_storage",
    "GPUPersistentCache",
];

const DESKTOP_EXTRAS: &[&str] = &["logs", "Logs", "sentry", "Crashpad"];

#[derive(Clone, Copy)]
enum Kind {
    Path,
    Electron,
    BakFiles,
}

struct Item {
    rel: &'static str,
    key: &'static str,
    label: &'static str,
    default_on: bool,
    kind: Kind,
}

struct Agent {
    id: &'static str,
    label: &'static str,
    busy: &'static [&'static str],
    items: &'static [Item],
}

const AGENTS: &[Agent] = &[
    Agent {
        id: "claude",
        label: "Claude",
        busy: &["claude", "claude-desktop"],
        items: &[
            Item { rel: ".config/Claude/vm_bundles", key: "vmBundles", label: "Desktop sandbox image", default_on: true, kind: Kind::Path },
            Item { rel: ".config/Claude", key: "desktop", label: "Desktop", default_on: true, kind: Kind::Electron },
            Item { rel: ".config/Claude-3p", key: "desktop3p", label: "Desktop 3p", default_on: true, kind: Kind::Electron },
            Item { rel: ".config/Claude/claude-code", key: "cliInstall", label: "CLI install cache", default_on: true, kind: Kind::Path },
            Item { rel: ".config/Claude/claude-code-vm", key: "vmInstall", label: "Sandbox install cache", default_on: true, kind: Kind::Path },
            Item { rel: ".claude/cache", key: "cliCache", label: "CLI cache", default_on: true, kind: Kind::Path },
            Item { rel: ".claude/paste-cache", key: "pasteCache", label: "Paste cache", default_on: true, kind: Kind::Path },
            Item { rel: ".claude/telemetry", key: "telemetry", label: "Telemetry", default_on: true, kind: Kind::Path },
            Item { rel: ".claude/debug", key: "debugLogs", label: "Debug logs", default_on: true, kind: Kind::Path },
            Item { rel: ".claude/metrics", key: "metrics", label: "Metrics", default_on: true, kind: Kind::Path },
            Item { rel: ".local/share/claude/versions", key: "oldCliVersions", label: "Old CLI versions", default_on: true, kind: Kind::Path },
        ],
    },
    Agent {
        id: "codex",
        label: "Codex",
        busy: &["codex"],
        items: &[
            Item { rel: ".cache/codex-runtimes", key: "runtimes", label: "Runtimes", default_on: true, kind: Kind::Path },
            Item { rel: ".codex/.tmp", key: "tempFiles", label: "Temp files", default_on: true, kind: Kind::Path },
            Item { rel: ".codex/cache", key: "cliCache", label: "CLI cache", default_on: true, kind: Kind::Path },
            Item { rel: ".codex/plugins/cache", key: "pluginCache", label: "Plugin cache", default_on: true, kind: Kind::Path },
            Item { rel: ".codex/plugins/.plugin-appserver", key: "pluginRuntime", label: "Plugin runtime", default_on: true, kind: Kind::Path },
            Item { rel: ".config/Codex", key: "desktop", label: "Desktop", default_on: true, kind: Kind::Electron },
        ],
    },
    Agent {
        id: "cursor",
        label: "Cursor",
        busy: &["cursor", "cursor-agent"],
        items: &[
            Item { rel: ".config/Cursor", key: "desktop", label: "Desktop", default_on: true, kind: Kind::Electron },
            Item { rel: ".cursor/ai-tracking", key: "aiTracking", label: "AI tracking", default_on: true, kind: Kind::Path },
        ],
    },
    Agent {
        id: "grok",
        label: "Grok",
        busy: &["grok"],
        items: &[
            Item { rel: ".grok/marketplace-cache", key: "marketplaceCache", label: "Marketplace cache", default_on: true, kind: Kind::Path },
            Item { rel: ".grok/logs", key: "logs", label: "Logs", default_on: true, kind: Kind::Path },
            Item { rel: ".grok/memtrace", key: "memtrace", label: "Memtrace", default_on: true, kind: Kind::Path },
            Item { rel: ".grok/debug", key: "debugLogs", label: "Debug logs", default_on: true, kind: Kind::Path },
            Item { rel: ".config/Grok Bot", key: "desktop", label: "Desktop", default_on: true, kind: Kind::Electron },
            Item { rel: ".config/grok-desktop", key: "desktopApp", label: "Desktop app", default_on: true, kind: Kind::Electron },
            Item { rel: ".config/grok-build-desktop", key: "buildDesktop", label: "Build desktop", default_on: true, kind: Kind::Electron },
            Item { rel: ".grok/worktrees", key: "worktrees", label: "Worktrees", default_on: false, kind: Kind::Path },
        ],
    },
    Agent {
        id: "gemini",
        label: "Gemini",
        busy: &["gemini"],
        items: &[
            Item { rel: ".gemini/antigravity-cli/log", key: "cliLogs", label: "CLI logs", default_on: true, kind: Kind::Path },
            Item { rel: ".config/Antigravity", key: "antigravity", label: "Antigravity", default_on: true, kind: Kind::Electron },
            Item { rel: ".config/Antigravity IDE", key: "antigravityIde", label: "Antigravity IDE", default_on: true, kind: Kind::Electron },
        ],
    },
    Agent {
        id: "openclaw",
        label: "OpenClaw",
        busy: &["openclaw"],
        items: &[
            Item { rel: ".openclaw/plugin-runtime-deps", key: "pluginRuntimes", label: "Plugin runtimes", default_on: true, kind: Kind::Path },
            Item { rel: ".openclaw/qclaw-quarantined-extensions", key: "quarantined", label: "Quarantined extensions", default_on: true, kind: Kind::Path },
            Item { rel: ".openclaw/browser", key: "browser", label: "Browser", default_on: true, kind: Kind::Electron },
            Item { rel: ".openclaw/media", key: "mediaCache", label: "Media cache", default_on: false, kind: Kind::Path },
        ],
    },
    Agent {
        id: "hermes",
        label: "Hermes",
        busy: &["hermes"],
        items: &[
            Item { rel: ".hermes/logs", key: "logs", label: "Logs", default_on: true, kind: Kind::Path },
            Item { rel: ".hermes/cache", key: "cache", label: "Cache", default_on: true, kind: Kind::Path },
            Item { rel: ".hermes/bootstrap-cache", key: "bootstrapCache", label: "Bootstrap cache", default_on: true, kind: Kind::Path },
            Item { rel: ".hermes/browser_recordings", key: "browserRecordings", label: "Browser recordings", default_on: true, kind: Kind::Path },
            Item { rel: ".hermes", key: "stateBackup", label: "State backup", default_on: true, kind: Kind::BakFiles },
        ],
    },
    Agent {
        id: "trae",
        label: "Trae",
        busy: &["trae"],
        items: &[
            Item { rel: ".config/Trae", key: "desktop", label: "Desktop", default_on: true, kind: Kind::Electron },
            Item { rel: ".config/Trae CN", key: "desktopCn", label: "Trae CN", default_on: true, kind: Kind::Electron },
            Item { rel: ".config/TRAE SOLO", key: "solo", label: "Trae SOLO", default_on: true, kind: Kind::Electron },
        ],
    },
    Agent {
        id: "qwen",
        label: "Qwen",
        busy: &["qwen"],
        items: &[
            Item { rel: ".qwen/debug", key: "debugLogs", label: "Debug logs", default_on: true, kind: Kind::Path },
            Item { rel: ".qwen/tmp", key: "tempFiles", label: "Temp files", default_on: true, kind: Kind::Path },
        ],
    },
    Agent {
        id: "opencode",
        label: "OpenCode",
        busy: &["opencode"],
        items: &[
            Item { rel: ".config/opencode", key: "desktop", label: "Desktop", default_on: true, kind: Kind::Electron },
            Item { rel: ".config/ai.opencode.desktop", key: "desktopApp", label: "Desktop app", default_on: true, kind: Kind::Electron },
        ],
    },
    Agent {
        id: "workbuddy",
        label: "WorkBuddy",
        busy: &["workbuddy"],
        items: &[
            Item { rel: ".workbuddy/traces", key: "traces", label: "Traces", default_on: true, kind: Kind::Path },
            Item { rel: ".workbuddy/cache", key: "cache", label: "Cache", default_on: true, kind: Kind::Path },
            Item { rel: ".workbuddy/shell-snapshots", key: "shellSnapshots", label: "Shell snapshots", default_on: true, kind: Kind::Path },
        ],
    },
    Agent {
        id: "pi",
        label: "Pi",
        busy: &[],
        items: &[
            Item { rel: ".pi/agent/npm", key: "npmCache", label: "npm cache", default_on: true, kind: Kind::Path },
        ],
    },
    Agent {
        id: "zed",
        label: "Zed",
        busy: &["zed"],
        items: &[
            Item { rel: ".local/share/zed/external_agents", key: "agentRuntimes", label: "Agent runtimes", default_on: true, kind: Kind::Path },
        ],
    },
    Agent {
        id: "continue",
        label: "Continue",
        busy: &[],
        items: &[
            Item { rel: ".continue/index", key: "index", label: "Index", default_on: true, kind: Kind::Path },
            Item { rel: ".continue/logs", key: "logs", label: "Logs", default_on: true, kind: Kind::Path },
        ],
    },
    Agent {
        id: "windsurf",
        label: "Windsurf",
        busy: &["windsurf"],
        items: &[
            Item { rel: ".codeium", key: "codeium", label: "Codeium", default_on: true, kind: Kind::Electron },
            Item { rel: ".windsurf", key: "desktop", label: "Desktop", default_on: true, kind: Kind::Electron },
            Item { rel: ".config/Windsurf", key: "desktop", label: "Desktop", default_on: true, kind: Kind::Electron },
        ],
    },
    Agent {
        id: "amazonq",
        label: "Amazon Q",
        busy: &["amazon-q"],
        items: &[
            Item { rel: ".aws/amazonq", key: "cache", label: "Cache", default_on: true, kind: Kind::Path },
            Item { rel: ".config/amazon-q", key: "desktop", label: "Desktop", default_on: true, kind: Kind::Electron },
        ],
    },
    Agent {
        id: "crush",
        label: "Crush",
        busy: &["crush"],
        items: &[
            Item { rel: ".config/crush", key: "desktop", label: "Desktop", default_on: true, kind: Kind::Electron },
            Item { rel: ".local/share/crush", key: "cache", label: "Cache", default_on: true, kind: Kind::Path },
        ],
    },
    Agent {
        id: "goose",
        label: "Goose",
        busy: &["goose"],
        items: &[
            Item { rel: ".config/goose", key: "desktop", label: "Desktop", default_on: true, kind: Kind::Electron },
            Item { rel: ".local/share/goose", key: "cache", label: "Cache", default_on: true, kind: Kind::Path },
        ],
    },
    Agent {
        id: "models",
        label: "Hugging Face",
        busy: &[],
        items: &[
            Item { rel: ".cache/huggingface", key: "huggingFace", label: "Hugging Face", default_on: false, kind: Kind::Path },
            Item { rel: ".cache/torch", key: "pytorch", label: "PyTorch", default_on: false, kind: Kind::Path },
            Item { rel: ".cache/whisper", key: "whisper", label: "Whisper", default_on: false, kind: Kind::Path },
            Item { rel: ".cache/modelscope", key: "modelScope", label: "ModelScope", default_on: false, kind: Kind::Path },
            Item { rel: ".ollama/models", key: "ollama", label: "Ollama", default_on: false, kind: Kind::Path },
            Item { rel: ".local/share/ollama", key: "ollamaData", label: "Ollama data", default_on: false, kind: Kind::Path },
            Item { rel: ".cache/lm-studio", key: "lmStudio", label: "LM Studio", default_on: false, kind: Kind::Path },
            Item { rel: ".cache/llama.cpp", key: "llamaCpp", label: "llama.cpp", default_on: false, kind: Kind::Path },
        ],
    },
];

pub fn keep_dir_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    KEEP.iter().any(|k| match_keep(&n, k))
}

fn match_keep(n: &str, k: &str) -> bool {
    if n == k {
        return true;
    }
    if n.starts_with(&format!("{k}-")) || n.starts_with(&format!("{k}.")) || n.starts_with(&format!("{k} ")) {
        return true;
    }
    k.len() >= 5 && n.contains(k)
}

fn electron_leaf_key(leaf: &str) -> &'static str {
    match leaf {
        "Cache" => "cache",
        "Code Cache" => "codeCache",
        "GPUCache" => "gpuCache",
        "CachedData" => "cachedData",
        "CachedExtensionVSIXs" => "extensionCache",
        "CachedProfilesData" => "profileCache",
        "DawnCache" | "DawnWebGPUCache" | "DawnGraphiteCache" => "dawnCache",
        "blob_storage" => "blobCache",
        "GPUPersistentCache" => "gpuPersistent",
        "logs" | "Logs" => "logs",
        "sentry" => "sentry",
        "Crashpad" => "crashDumps",
        _ => "cache",
    }
}

fn electron_label(prefix: &str, leaf: &str) -> String {
    let leaf_l = match electron_leaf_key(leaf) {
        "cache" => "cache",
        "codeCache" => "code cache",
        "gpuCache" => "GPU cache",
        "cachedData" => "cached data",
        "extensionCache" => "extension cache",
        "profileCache" => "profile cache",
        "dawnCache" => "Dawn cache",
        "blobCache" => "blob cache",
        "gpuPersistent" => "GPU persistent cache",
        "logs" => "logs",
        "sentry" => "sentry",
        "crashDumps" => "crash dumps",
        other => other,
    };
    format!("{prefix} {leaf_l}")
}

pub fn scan(home: &Path, seen: &mut HashSet<PathBuf>, out: &mut Vec<Candidate>) {
    for agent in AGENTS {
        for item in agent.items {
            match item.kind {
                Kind::Path => super::push_ai(
                    seen,
                    out,
                    agent.id,
                    agent.label,
                    item.label,
                    item.key,
                    home.join(item.rel),
                    agent.busy,
                    item.default_on,
                ),
                Kind::Electron => {
                    let root = home.join(item.rel);
                    for leaf in ELECTRON_LEAVES.iter().chain(DESKTOP_EXTRAS) {
                        let path = root.join(leaf);
                        if path.is_dir() {
                            let label = electron_label(item.label, leaf);
                            let label_key = format!("{}.{}", item.key, electron_leaf_key(leaf));
                            super::push_ai(
                                seen,
                                out,
                                agent.id,
                                agent.label,
                                &label,
                                &label_key,
                                path,
                                agent.busy,
                                item.default_on,
                            );
                        }
                    }
                }
                Kind::BakFiles => {
                    let root = home.join(item.rel);
                    let Ok(entries) = fs::read_dir(&root) else {
                        continue;
                    };
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        if name.ends_with(".bak") || name.contains(".bak.") {
                            super::push_ai(
                                seen,
                                out,
                                agent.id,
                                agent.label,
                                item.label,
                                item.key,
                                entry.path(),
                                agent.busy,
                                item.default_on,
                            );
                        }
                    }
                }
            }
        }
    }
}
