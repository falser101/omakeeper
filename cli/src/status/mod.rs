use crate::util;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::stdout;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct CpuInfo {
    pub usage: f32,
    pub logical_cpu: usize,
    pub load: [f32; 3],
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryInfo {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub used_percent: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub mount: String,
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub used_percent: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetInfo {
    pub down_bps: u64,
    pub up_bps: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcInfo {
    pub pid: u32,
    pub name: String,
    pub rss: u64,
    pub cpu: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ZombieParent {
    pub pid: u32,
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusSnapshot {
    pub host: String,
    pub health_score: u32,
    pub uptime: String,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub network: NetInfo,
    pub processes: Vec<ProcInfo>,
    pub zombie_count: usize,
    pub zombie_parents: Vec<ZombieParent>,
}

#[derive(Clone, Copy, Default)]
struct CpuTimes {
    idle: u64,
    total: u64,
}

#[derive(Clone, Copy, Default)]
struct NetTimes {
    rx: u64,
    tx: u64,
}

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
    }
}

fn read_cpu_times() -> CpuTimes {
    let Ok(text) = fs::read_to_string("/proc/stat") else {
        return CpuTimes::default();
    };
    let Some(line) = text.lines().next() else {
        return CpuTimes::default();
    };
    let mut parts = line.split_whitespace();
    if parts.next() != Some("cpu") {
        return CpuTimes::default();
    }
    let nums: Vec<u64> = parts.filter_map(|s| s.parse().ok()).collect();
    if nums.len() < 4 {
        return CpuTimes::default();
    }
    let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
    let total = nums.iter().sum();
    CpuTimes { idle, total }
}

fn cpu_usage(prev: CpuTimes, next: CpuTimes) -> f32 {
    let dt = next.total.saturating_sub(prev.total) as f32;
    if dt <= 0.0 {
        return 0.0;
    }
    let di = next.idle.saturating_sub(prev.idle) as f32;
    ((dt - di) / dt * 100.0).clamp(0.0, 100.0)
}

fn read_load() -> [f32; 3] {
    let Ok(text) = fs::read_to_string("/proc/loadavg") else {
        return [0.0, 0.0, 0.0];
    };
    let mut it = text.split_whitespace();
    let a = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let b = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let c = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    [a, b, c]
}

fn read_memory() -> MemoryInfo {
    let mut total = 0u64;
    let mut available = 0u64;
    if let Ok(text) = fs::read_to_string("/proc/meminfo") {
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                total = parse_kb(rest);
            } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
                available = parse_kb(rest);
            }
        }
    }
    let used = total.saturating_sub(available);
    let used_percent = if total == 0 {
        0.0
    } else {
        used as f32 / total as f32 * 100.0
    };
    MemoryInfo {
        total,
        used,
        available,
        used_percent,
    }
}

fn parse_kb(s: &str) -> u64 {
    s.split_whitespace()
        .next()
        .and_then(|n| n.parse::<u64>().ok())
        .unwrap_or(0)
        * 1024
}

fn read_uptime() -> String {
    let Ok(text) = fs::read_to_string("/proc/uptime") else {
        return "?".into();
    };
    let secs: u64 = text
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0) as u64;
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{days}d {hours}h {mins}m")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

fn read_disks() -> Vec<DiskInfo> {
    let Ok(out) = std::process::Command::new("df")
        .args(["-B1", "-P"])
        .output()
    else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut disks = Vec::new();
    for line in text.lines().skip(1) {
        let cols: Vec<_> = line.split_whitespace().collect();
        if cols.len() < 6 {
            continue;
        }
        let fs = cols[0];
        let mount = cols[5];
        if fs.starts_with("tmpfs")
            || fs.starts_with("devtmpfs")
            || fs.starts_with("overlay")
            || fs.starts_with("squashfs")
            || mount.starts_with("/run")
            || mount.starts_with("/dev")
            || mount.starts_with("/sys")
            || mount.starts_with("/proc")
            || mount.starts_with("/boot/efi")
        {
            continue;
        }
        let total: u64 = cols[1].parse().unwrap_or(0);
        let used: u64 = cols[2].parse().unwrap_or(0);
        let available: u64 = cols[3].parse().unwrap_or(0);
        if total == 0 {
            continue;
        }
        disks.push(DiskInfo {
            mount: mount.to_string(),
            total,
            used,
            available,
            used_percent: used as f32 / total as f32 * 100.0,
        });
    }
    disks
}

fn read_net() -> NetTimes {
    let Ok(text) = fs::read_to_string("/proc/net/dev") else {
        return NetTimes::default();
    };
    let mut rx = 0u64;
    let mut tx = 0u64;
    for line in text.lines().skip(2) {
        let Some((iface, rest)) = line.split_once(':') else {
            continue;
        };
        let iface = iface.trim();
        if iface == "lo" {
            continue;
        }
        let cols: Vec<_> = rest.split_whitespace().collect();
        if cols.len() < 10 {
            continue;
        }
        rx += cols[0].parse().unwrap_or(0);
        tx += cols[8].parse().unwrap_or(0);
    }
    NetTimes { rx, tx }
}

fn hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "linux".into())
}

struct ProcSample {
    pid: u32,
    name: String,
    ppid: u32,
    state: char,
    rss: u64,
    ticks: u64,
}

fn read_procs() -> Vec<ProcSample> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid_s) = name.to_str() else { continue };
        let Ok(pid) = pid_s.parse::<u32>() else { continue };
        let stat = fs::read_to_string(entry.path().join("stat")).ok();
        let Some(stat) = stat else { continue };
        let Some(open) = stat.find('(') else { continue };
        let Some(close) = stat.rfind(')') else { continue };
        let comm = &stat[open + 1..close];
        let rest = &stat[close + 2..];
        let cols: Vec<_> = rest.split_whitespace().collect();
        if cols.len() < 22 {
            continue;
        }
        let state = cols[0].chars().next().unwrap_or('?');
        let ppid: u32 = cols[1].parse().unwrap_or(0);
        let utime: u64 = cols[11].parse().unwrap_or(0);
        let stime: u64 = cols[12].parse().unwrap_or(0);
        let rss_pages: u64 = cols[21].parse().unwrap_or(0);
        out.push(ProcSample {
            pid,
            name: comm.to_string(),
            ppid,
            state,
            rss: rss_pages * 4096,
            ticks: utime + stime,
        });
    }
    out
}

fn health_score(cpu: f32, mem: f32, disk: f32, load_per_cpu: f32) -> u32 {
    let cpu_s = (100.0 - cpu).clamp(0.0, 100.0);
    let mem_s = (100.0 - mem).clamp(0.0, 100.0);
    let disk_s = (100.0 - disk).clamp(0.0, 100.0);
    let load_s = (100.0 - (load_per_cpu * 50.0).min(100.0)).clamp(0.0, 100.0);
    ((cpu_s * 0.3 + mem_s * 0.3 + disk_s * 0.25 + load_s * 0.15) as u32).min(100)
}

fn snapshot(prev_cpu: CpuTimes, prev_net: NetTimes, elapsed: f32, prev_ticks: &HashMap<u32, u64>) -> (StatusSnapshot, CpuTimes, NetTimes, HashMap<u32, u64>) {
    let next_cpu = read_cpu_times();
    let next_net = read_net();
    let usage = cpu_usage(prev_cpu, next_cpu);
    let logical = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let load = read_load();
    let memory = read_memory();
    let disks = read_disks();
    let disk_pct = disks
        .iter()
        .find(|d| d.mount == "/")
        .or(disks.first())
        .map(|d| d.used_percent)
        .unwrap_or(0.0);
    let down = if elapsed > 0.0 {
        ((next_net.rx.saturating_sub(prev_net.rx)) as f32 / elapsed) as u64
    } else {
        0
    };
    let up = if elapsed > 0.0 {
        ((next_net.tx.saturating_sub(prev_net.tx)) as f32 / elapsed) as u64
    } else {
        0
    };

    let procs = read_procs();
    let mut zombies = Vec::new();
    let mut live = Vec::new();
    let mut next_ticks = HashMap::new();
    for p in &procs {
        next_ticks.insert(p.pid, p.ticks);
        if p.state == 'Z' {
            zombies.push(p);
        } else {
            live.push(p);
        }
    }
    let hz = 100.0;
    let mut ranked: Vec<ProcInfo> = live
        .iter()
        .map(|p| {
            let cpu = prev_ticks.get(&p.pid).copied().map(|prev| {
                let dticks = p.ticks.saturating_sub(prev) as f32;
                if elapsed > 0.0 {
                    (dticks / hz / elapsed * 100.0).clamp(0.0, 100.0 * logical as f32)
                } else {
                    0.0
                }
            });
            ProcInfo {
                pid: p.pid,
                name: p.name.clone(),
                rss: p.rss,
                cpu,
            }
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.cpu
            .partial_cmp(&a.cpu)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.rss.cmp(&a.rss))
    });
    ranked.truncate(8);

    let mut parent_counts: HashMap<u32, usize> = HashMap::new();
    for z in &zombies {
        *parent_counts.entry(z.ppid).or_insert(0) += 1;
    }
    let names: HashMap<u32, String> = procs.iter().map(|p| (p.pid, p.name.clone())).collect();
    let mut zombie_parents: Vec<ZombieParent> = parent_counts
        .into_iter()
        .map(|(pid, count)| ZombieParent {
            pid,
            name: names.get(&pid).cloned().unwrap_or_else(|| "?".into()),
            count,
        })
        .collect();
    zombie_parents.sort_by(|a, b| b.count.cmp(&a.count));
    zombie_parents.truncate(3);

    let score = health_score(usage, memory.used_percent, disk_pct, load[0] / logical as f32);
    let snap = StatusSnapshot {
        host: hostname(),
        health_score: score,
        uptime: read_uptime(),
        cpu: CpuInfo {
            usage,
            logical_cpu: logical,
            load,
        },
        memory,
        disks,
        network: NetInfo {
            down_bps: down,
            up_bps: up,
        },
        processes: ranked,
        zombie_count: zombies.len(),
        zombie_parents,
    };
    (snap, next_cpu, next_net, next_ticks)
}

fn collect_once() -> StatusSnapshot {
    let cpu0 = read_cpu_times();
    let net0 = read_net();
    let ticks0: HashMap<u32, u64> = read_procs().into_iter().map(|p| (p.pid, p.ticks)).collect();
    std::thread::sleep(Duration::from_millis(250));
    snapshot(cpu0, net0, 0.25, &ticks0).0
}

fn gauge<'a>(title: &'a str, pct: f32, extra: String) -> Gauge<'a> {
    let color = if pct >= 90.0 {
        Color::Red
    } else if pct >= 75.0 {
        Color::Yellow
    } else {
        Color::Green
    };
    Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(title))
        .gauge_style(Style::default().fg(color))
        .ratio((pct / 100.0).clamp(0.0, 1.0) as f64)
        .label(extra)
}

fn draw_status(frame: &mut Frame, snap: &StatusSnapshot) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(area);

    let mem_gb = snap.memory.total as f32 / (1024.0 * 1024.0 * 1024.0);
    let header = Paragraph::new(format!(
        "Omakeeper Status  Health ● {}  {} · {:.0}GB · uptime {}",
        snap.health_score, snap.host, mem_gb, snap.uptime
    ))
    .style(Style::default().fg(Color::Cyan).bold())
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    frame.render_widget(
        gauge(
            "CPU",
            snap.cpu.usage,
            format!(
                "{:.1}%  load {:.2} / {:.2} / {:.2}  ({} cores)",
                snap.cpu.usage,
                snap.cpu.load[0],
                snap.cpu.load[1],
                snap.cpu.load[2],
                snap.cpu.logical_cpu
            ),
        ),
        chunks[1],
    );
    frame.render_widget(
        gauge(
            "Memory",
            snap.memory.used_percent,
            format!(
                "{:.1}%  {} / {}",
                snap.memory.used_percent,
                util::format_bytes(snap.memory.used),
                util::format_bytes(snap.memory.total)
            ),
        ),
        chunks[2],
    );

    let disk = snap
        .disks
        .iter()
        .find(|d| d.mount == "/")
        .or(snap.disks.first());
    let (dpct, dlabel) = match disk {
        Some(d) => (
            d.used_percent,
            format!(
                "{:.1}%  {} free on {}",
                d.used_percent,
                util::format_bytes(d.available),
                d.mount
            ),
        ),
        None => (0.0, "no disks".into()),
    };
    frame.render_widget(gauge("Disk", dpct, dlabel), chunks[3]);

    let mut lines = vec![
        Line::from(format!(
            "Network  ↓ {}/s   ↑ {}/s",
            util::format_bytes(snap.network.down_bps),
            util::format_bytes(snap.network.up_bps)
        )),
        Line::from(format!(
            "Zombies  {}{}",
            snap.zombie_count,
            if snap.zombie_parents.is_empty() {
                String::new()
            } else {
                let p = snap
                    .zombie_parents
                    .iter()
                    .map(|z| format!("{} ({})", z.name, z.count))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("  parents: {p}")
            }
        )),
        Line::from("Processes"),
    ];
    for p in &snap.processes {
        let cpu = p
            .cpu
            .map(|c| format!("{c:5.1}%"))
            .unwrap_or_else(|| "    —".into());
        lines.push(Line::from(format!(
            "  {:>6}  {cpu}  {:>8}  {}",
            p.pid,
            util::format_bytes(p.rss),
            p.name
        )));
    }
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Live ")),
        chunks[4],
    );
    frame.render_widget(
        Paragraph::new("q quit").style(Style::default().fg(Color::DarkGray)),
        chunks[5],
    );
}

fn run_watch(interval: u64, json: bool) -> Result<()> {
    let mut cpu = read_cpu_times();
    let mut net = read_net();
    let mut ticks: HashMap<u32, u64> = read_procs().into_iter().map(|p| (p.pid, p.ticks)).collect();
    if json || !util::is_tty() {
        loop {
            std::thread::sleep(Duration::from_secs(interval.max(1)));
            let (snap, ncpu, nnet, nticks) = snapshot(cpu, net, interval as f32, &ticks);
            cpu = ncpu;
            net = nnet;
            ticks = nticks;
            println!("{}", serde_json::to_string(&snap)?);
        }
    }

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let _guard = Guard;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut snap = collect_once();
    loop {
        terminal.draw(|frame| draw_status(frame, &snap))?;
        if event::poll(Duration::from_secs(interval.max(1)))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
                {
                    return Ok(());
                }
            }
        }
        let (next, ncpu, nnet, nticks) = snapshot(cpu, net, interval as f32, &ticks);
        cpu = ncpu;
        net = nnet;
        ticks = nticks;
        snap = next;
    }
}

pub fn run(watch: bool, interval: u64, json: bool) -> Result<()> {
    if watch {
        return run_watch(interval, json);
    }
    let snap = collect_once();
    if json || !util::is_tty() {
        println!("{}", serde_json::to_string_pretty(&snap)?);
        return Ok(());
    }
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let _guard = Guard;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    loop {
        terminal.draw(|frame| draw_status(frame, &snap))?;
        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
                {
                    return Ok(());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_penalizes_pressure() {
        let high = health_score(10.0, 20.0, 30.0, 0.2);
        let low = health_score(95.0, 95.0, 95.0, 3.0);
        assert!(high > low);
        assert!(high <= 100);
        assert!(low < 40);
    }

    #[test]
    fn cpu_usage_from_deltas() {
        let a = CpuTimes { idle: 100, total: 200 };
        let b = CpuTimes { idle: 150, total: 300 };
        let u = cpu_usage(a, b);
        assert!((u - 50.0).abs() < 0.1);
    }
}
