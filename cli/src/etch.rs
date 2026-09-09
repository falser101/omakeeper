use anyhow::{Context, Result};
use serde::Serialize;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

const WORDMARK: &[&str] = &[
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
    "000000000000000000000000000000000000000111000100000000000000000000000000000000000",
];

const WORD_W: usize = 81;
const WORD_H: usize = 19;
const PAD_C: usize = 6;
const PAD_R: usize = 4;
const TARGET_FRAMES: usize = 120;
const PLAY_MS: u64 = 28;

const SKIP: &[&str] = &["matrix", "thunderstorm", "swarm", "spotlights", "rings"];

#[derive(Clone, Serialize)]
struct Cell {
    c: i16,
    r: i16,
    s: u32,
    x: u32,
}

pub fn run(effect: Option<&str>) -> Result<()> {
    let cols = WORD_W + PAD_C * 2;
    let rows = WORD_H + PAD_R * 2;
    let input = wordmark_text(cols);

    let mut cmd = Command::new("ttfx");
    cmd.arg("--parity-dump")
        .arg("--seed")
        .arg(format!("{}", seed_now()))
        .arg("--canvas-width")
        .arg(cols.to_string())
        .arg("--canvas-height")
        .arg(rows.to_string())
        .arg("--ignore-terminal-dimensions")
        .arg("--no-eol")
        .arg("--no-restore-cursor")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    match effect {
        Some(name) if name != "random" => {
            cmd.arg(name);
        }
        _ => {
            cmd.arg("--random-effect");
            for skip in SKIP {
                cmd.arg("--exclude-effects").arg(skip);
            }
        }
    }

    let mut child = cmd.spawn().context("ttfx is not installed")?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes())?;
    }
    let mut stdout = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        out.read_to_end(&mut stdout)?;
    }
    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("ttfx exited {}", status.code().unwrap_or(-1));
    }

    let frames = parse_parity(&stdout);
    if frames.is_empty() {
        anyhow::bail!("ttfx produced no frames");
    }
    let played = resample(&frames, TARGET_FRAMES);

    let mut stdout_lock = std::io::stdout().lock();
    writeln!(
        stdout_lock,
        "{}",
        serde_json::json!({
            "event": "meta",
            "cols": cols,
            "rows": rows,
            "padC": PAD_C,
            "padR": PAD_R,
            "frames": played.len(),
        })
    )?;
    stdout_lock.flush()?;

    for cells in &played {
        writeln!(
            stdout_lock,
            "{}",
            serde_json::json!({ "event": "frame", "cells": cells })
        )?;
        stdout_lock.flush()?;
        std::thread::sleep(Duration::from_millis(PLAY_MS));
    }
    writeln!(stdout_lock, "{}", serde_json::json!({ "event": "done" }))?;
    stdout_lock.flush()?;
    Ok(())
}

fn wordmark_text(cols: usize) -> String {
    let blank = " ".repeat(cols);
    let mut lines = Vec::with_capacity(WORD_H + PAD_R * 2);
    for _ in 0..PAD_R {
        lines.push(blank.clone());
    }
    for row in WORDMARK {
        let mut line = " ".repeat(PAD_C);
        for ch in row.chars() {
            line.push(if ch == '1' { '█' } else { ' ' });
        }
        line.push_str(&" ".repeat(PAD_C));
        lines.push(line);
    }
    for _ in 0..PAD_R {
        lines.push(blank.clone());
    }
    lines.join("\n") + "\n"
}

fn seed_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9ece6a)
}

fn parse_parity(bytes: &[u8]) -> Vec<Vec<Cell>> {
    let mut frames = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            i += 1;
            continue;
        }
        let Some(nl) = bytes[i..].iter().position(|&b| b == b'\n') else {
            break;
        };
        let header = &bytes[i..i + nl];
        if header.is_empty() {
            i += nl + 1;
            continue;
        }
        let Ok(n) = std::str::from_utf8(header).unwrap_or("").parse::<usize>() else {
            break;
        };
        let start = i + nl + 1;
        let end = (start + n).min(bytes.len());
        frames.push(parse_frame(&bytes[start..end]));
        i = end;
    }
    frames
}

fn parse_frame(payload: &[u8]) -> Vec<Cell> {
    let mut cells = Vec::new();
    let mut fg: Option<u32> = None;
    let mut row: i16 = 0;
    let mut col: i16 = 0;
    let mut i = 0;
    while i < payload.len() {
        if payload[i] == 0x1b {
            if i + 1 < payload.len() && payload[i + 1] == b'[' {
                if let Some(end) = payload[i + 2..].iter().position(|&b| b == b'm') {
                    let seq = &payload[i + 2..i + 2 + end];
                    fg = parse_sgr(seq, fg);
                    i = i + 3 + end;
                    continue;
                }
            }
            i += 1;
            continue;
        }
        if payload[i] == b'\n' {
            row += 1;
            col = 0;
            i += 1;
            continue;
        }
        if payload[i] == b'\r' {
            col = 0;
            i += 1;
            continue;
        }
        let (sym, n) = decode_utf8(&payload[i..]);
        if n == 0 {
            break;
        }
        if sym != 32 && sym != 0 {
            if let Some(rgb) = fg {
                cells.push(Cell {
                    c: col - PAD_C as i16,
                    r: row - PAD_R as i16,
                    s: sym,
                    x: rgb,
                });
            }
        }
        col += 1;
        i += n;
    }
    cells
}

fn parse_sgr(seq: &[u8], current: Option<u32>) -> Option<u32> {
    let Ok(text) = std::str::from_utf8(seq) else {
        return current;
    };
    if text.is_empty() || text == "0" {
        return None;
    }
    let parts: Vec<&str> = text.split(';').collect();
    if parts.len() >= 5 && parts[0] == "38" && parts[1] == "2" {
        if let (Ok(r), Ok(g), Ok(b)) = (
            parts[2].parse::<u32>(),
            parts[3].parse::<u32>(),
            parts[4].parse::<u32>(),
        ) {
            return Some((r << 16) | (g << 8) | b);
        }
    }
    current
}

fn decode_utf8(bytes: &[u8]) -> (u32, usize) {
    if bytes.is_empty() {
        return (0, 0);
    }
    let b0 = bytes[0];
    if b0 < 0x80 {
        return (b0 as u32, 1);
    }
    if (0xc0..0xe0).contains(&b0) && bytes.len() >= 2 {
        let cp = ((b0 as u32 & 0x1f) << 6) | (bytes[1] as u32 & 0x3f);
        return (cp, 2);
    }
    if (0xe0..0xf0).contains(&b0) && bytes.len() >= 3 {
        let cp = ((b0 as u32 & 0x0f) << 12)
            | ((bytes[1] as u32 & 0x3f) << 6)
            | (bytes[2] as u32 & 0x3f);
        return (cp, 3);
    }
    if b0 >= 0xf0 && bytes.len() >= 4 {
        let cp = ((b0 as u32 & 0x07) << 18)
            | ((bytes[1] as u32 & 0x3f) << 12)
            | ((bytes[2] as u32 & 0x3f) << 6)
            | (bytes[3] as u32 & 0x3f);
        return (cp, 4);
    }
    (b0 as u32, 1)
}

fn resample(frames: &[Vec<Cell>], target: usize) -> Vec<Vec<Cell>> {
    if frames.len() <= target {
        return frames.to_vec();
    }
    let mut out = Vec::with_capacity(target);
    for i in 0..target {
        let idx = i * (frames.len() - 1) / (target - 1).max(1);
        out.push(frames[idx].clone());
    }
    out
}
