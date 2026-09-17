//! Fullscreen at-home face. Same commands as the line REPL.
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste,
        EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame, Terminal,
};
use serde_json::Value;
use std::fs;
use std::io::{self, Stdout};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const USER_TAG: &str = "哥哥";
const SPIN: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

enum Kind {
    Sys,
    User,
    Mother,
    Out,
    Stream,
}

enum StepSt {
    Run,
    Ok,
    Fail,
}

struct Step {
    key: String,
    title: String,
    detail: String,
    st: StepSt,
    t0: Instant,
    ms: u64,
}

#[derive(Clone, Default)]
struct HostView {
    ident: String,
    cpu_pct: u16,
    load: String,
    tasks: String,
    temp: String,
    mem_used: u64,
    mem_total: u64,
    swap_used: u64,
    swap_total: u64,
    disk_used: u64,
    disk_total: u64,
    net_down: String,
    net_up: String,
    rooms: String,
    mother: String,
}

struct Msg {
    kind: Kind,
    text: String,
}

enum Job {
    Progress(String),
    Done(String),
    Nl(String),
}

enum Secret {
    Off,
    On { adapter: String, buf: String },
}

struct App {
    msgs: Vec<Msg>,
    input: String,
    cursor: usize,
    hist: Vec<String>,
    hist_idx: Option<usize>,
    draft: String,
    progress: Option<String>,
    progress_at: Instant,
    busy: bool,
    leave: bool,
    follow: bool,
    scroll: u16,
    keys: bool,
    nick: String,
    face: HostView,
    cpu_prev: Option<(u64, u64)>,
    net_prev: Option<(u64, u64, Instant)>,
    stream: Vec<Step>,
    spin: usize,
    secret: Secret,
    status_at: Instant,
    job_rx: Receiver<Job>,
    job_tx: Sender<Job>,
    portal: bool,
}

impl App {
    fn new() -> Self {
        let (job_tx, job_rx) = mpsc::channel();
        let mut hist = Vec::new();
        if let Some(p) = super::history_path() {
            if let Ok(s) = std::fs::read_to_string(p) {
                for line in s.lines() {
                    let t = line.trim();
                    if t.is_empty() || t.starts_with('#') {
                        continue;
                    }
                    hist.push(t.to_string());
                }
            }
        }
        let mut cpu_prev = None;
        let mut net_prev = None;
        let _ = sample_host(&mut cpu_prev, &mut net_prev);
        thread::sleep(Duration::from_millis(60));
        let face = sample_host(&mut cpu_prev, &mut net_prev);
        let app = Self {
            msgs: Vec::new(),
            input: String::new(),
            cursor: 0,
            hist,
            hist_idx: None,
            draft: String::new(),
            progress: None,
            progress_at: Instant::now(),
            busy: false,
            leave: false,
            follow: true,
            scroll: 0,
            keys: false,
            nick: super::load_nick(),
            face,
            cpu_prev,
            net_prev,
            stream: Vec::new(),
            spin: 0,
            secret: Secret::Off,
            status_at: Instant::now(),
            job_rx,
            job_tx,
            portal: true,
        };
        app
    }

    fn refresh_face(&mut self) {
        self.face = sample_host(&mut self.cpu_prev, &mut self.net_prev);
        self.status_at = Instant::now();
    }

    fn push(&mut self, kind: Kind, text: String) {
        if text.trim().is_empty() {
            return;
        }
        self.portal = false;
        self.msgs.push(Msg { kind, text });
        if self.msgs.len() > 400 {
            self.msgs.drain(0..self.msgs.len() - 400);
        }
        if self.follow {
            self.scroll = 0;
        }
    }

    fn save_hist(&self) {
        if let Some(p) = super::history_path() {
            let _ = std::fs::write(
                p,
                self.hist
                    .iter()
                    .skip(self.hist.len().saturating_sub(500))
                    .fold(String::new(), |mut a, l| {
                        a.push_str(l);
                        a.push('\n');
                        a
                    }),
            );
        }
    }
}

fn hum_bytes(n: u64) -> String {
    const K: f64 = 1024.0;
    let x = n as f64;
    if x >= K * K * K * K {
        format!("{:.1}T", x / (K * K * K * K))
    } else if x >= K * K * K {
        format!("{:.1}G", x / (K * K * K))
    } else if x >= K * K {
        format!("{:.0}M", x / (K * K))
    } else if x >= K {
        format!("{:.0}K", x / K)
    } else {
        format!("{n}B")
    }
}

fn hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            fs::read_to_string("/proc/sys/kernel/hostname")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "aios".into())
}

fn cpu_line() -> String {
    let raw = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let mut model = None;
    let mut threads = 0u32;
    let mut cores: Option<u32> = None;
    for line in raw.lines() {
        let (k, v) = match line.split_once(':') {
            Some(p) => (p.0.trim(), p.1.trim()),
            None => continue,
        };
        if k == "processor" {
            threads += 1;
        }
        if model.is_none() && k == "model name" {
            model = Some(v.to_string());
        }
        if cores.is_none() && k == "cpu cores" {
            cores = v.parse().ok();
        }
    }
    let mut m = model.unwrap_or_else(|| "CPU".into());
    for p in ["AMD ", "Intel(R) ", "Intel "] {
        if let Some(s) = m.strip_prefix(p) {
            m = s.to_string();
        }
    }
    if let Some(i) = m.find(" w/") {
        m.truncate(i);
    }
    if let Some(i) = m.find(" CPU") {
        m.truncate(i);
    }
    m = m.replace("(R)", "").replace("(TM)", "");
    let m = m.trim().to_string();
    let th = if threads == 0 { 1 } else { threads };
    match cores {
        Some(c) if c > 0 && c != th => format!("{m}  {c}核/{th}线程"),
        _ => format!("{m}  {th}核"),
    }
}

fn meminfo_kib(raw: &str, key: &str) -> Option<u64> {
    for line in raw.lines() {
        if let Some(v) = line.strip_prefix(key) {
            return Some(v.split_whitespace().next()?.parse::<u64>().ok()? * 1024);
        }
    }
    None
}

fn mem_swap() -> (Option<(u64, u64)>, Option<(u64, u64)>) {
    let raw = match fs::read_to_string("/proc/meminfo") {
        Ok(s) => s,
        Err(_) => return (None, None),
    };
    let mem = match (meminfo_kib(&raw, "MemTotal:"), meminfo_kib(&raw, "MemAvailable:")) {
        (Some(total), Some(avail)) if total > 0 => Some((total.saturating_sub(avail), total)),
        _ => None,
    };
    let swap = match (meminfo_kib(&raw, "SwapTotal:"), meminfo_kib(&raw, "SwapFree:")) {
        (Some(total), Some(free)) if total > 0 => Some((total.saturating_sub(free), total)),
        _ => None,
    };
    (mem, swap)
}

fn load_tasks() -> (String, String) {
    let s = fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let mut it = s.split_whitespace();
    let a = it.next().unwrap_or("?").to_string();
    let b = it.next().unwrap_or("?").to_string();
    let c = it.next().unwrap_or("?").to_string();
    let tasks = it.next().unwrap_or("").to_string();
    (format!("{a}  {b}  {c}"), tasks)
}

fn cpu_stat() -> Option<(u64, u64)> {
    let raw = fs::read_to_string("/proc/stat").ok()?;
    let line = raw.lines().next()?;
    if !line.starts_with("cpu ") {
        return None;
    }
    let nums: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|x| x.parse().ok())
        .collect();
    if nums.len() < 4 {
        return None;
    }
    let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
    let total: u64 = nums.iter().take(8).sum();
    if total == 0 {
        return None;
    }
    Some((idle, total))
}

fn cpu_pct(prev: &mut Option<(u64, u64)>) -> u16 {
    let Some((idle, total)) = cpu_stat() else {
        return 0;
    };
    let pct = if let Some((pi, pt)) = *prev {
        let di = idle.saturating_sub(pi);
        let dt = total.saturating_sub(pt);
        if dt == 0 {
            0
        } else {
            ((dt.saturating_sub(di)) * 100 / dt) as u16
        }
    } else {
        0
    };
    *prev = Some((idle, total));
    pct.min(100)
}

fn skip_iface(name: &str) -> bool {
    let n = name.trim_end_matches(':');
    n == "lo"
        || n.starts_with("docker")
        || n.starts_with("veth")
        || n.starts_with("br-")
        || n.starts_with("virbr")
        || n.starts_with("wg")
        || n.starts_with("tun")
        || n.starts_with("tap")
        || n.starts_with("cni")
        || n.starts_with("flannel")
}

fn net_counters() -> (u64, u64) {
    let raw = fs::read_to_string("/proc/net/dev").unwrap_or_default();
    let mut rx = 0u64;
    let mut tx = 0u64;
    for line in raw.lines().skip(2) {
        let (name, rest) = match line.split_once(':') {
            Some(p) => (p.0.trim(), p.1),
            None => continue,
        };
        if skip_iface(name) {
            continue;
        }
        let mut it = rest.split_whitespace();
        if let Some(v) = it.next().and_then(|x| x.parse::<u64>().ok()) {
            rx += v;
        }
        for _ in 0..7 {
            it.next();
        }
        if let Some(v) = it.next().and_then(|x| x.parse::<u64>().ok()) {
            tx += v;
        }
    }
    (rx, tx)
}

fn hum_rate(bps: u64) -> String {
    format!("{}/s", hum_bytes(bps))
}

fn net_rate(prev: &mut Option<(u64, u64, Instant)>) -> (String, String) {
    let (rx, tx) = net_counters();
    let now = Instant::now();
    let (dn, up) = if let Some((prx, ptx, at)) = *prev {
        let dt = now.duration_since(at).as_secs_f64().max(0.2);
        let db = rx.saturating_sub(prx) as f64 / dt;
        let ub = tx.saturating_sub(ptx) as f64 / dt;
        (hum_rate(db as u64), hum_rate(ub as u64))
    } else {
        ("—".into(), "—".into())
    };
    *prev = Some((rx, tx, now));
    (dn, up)
}

fn cpu_temp() -> String {
    let Ok(dir) = fs::read_dir("/sys/class/thermal") else {
        return String::new();
    };
    for e in dir.flatten() {
        let p = e.path();
        let Ok(raw) = fs::read_to_string(p.join("temp")) else {
            continue;
        };
        let Ok(m) = raw.trim().parse::<i64>() else {
            continue;
        };
        if (15_000..=110_000).contains(&m) {
            return format!("{}°C", m / 1000);
        }
    }
    String::new()
}

fn clock_hm() -> String {
    Command::new("date")
        .arg("+%H:%M")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
}

fn uptime_zh() -> Option<String> {
    let s = fs::read_to_string("/proc/uptime").ok()?;
    let secs: u64 = s.split_whitespace().next()?.parse::<f64>().ok()? as u64;
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    Some(if d > 0 {
        format!("{d}天{h}时")
    } else if h > 0 {
        format!("{h}时{m}分")
    } else {
        format!("{m}分")
    })
}

fn disk_root() -> Option<(u64, u64)> {
    let o = Command::new("df")
        .args(["-B1", "--output=used,size", "/"])
        .output()
        .ok()?;
    if !o.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&o.stdout);
    let line = s.lines().nth(1)?;
    let mut it = line.split_whitespace();
    let used: u64 = it.next()?.parse().ok()?;
    let size: u64 = it.next()?.parse().ok()?;
    if size == 0 {
        return None;
    }
    Some((used, size))
}

fn rooms_line() -> (String, bool) {
    match super::call("/status") {
        Ok(raw) => {
            if let Ok(v) = serde_json::from_str::<Value>(raw.trim()) {
                let body = v.get("body").unwrap_or(&v);
                match body.get("leases").and_then(|x| x.as_array()) {
                    Some(ls) if !ls.is_empty() => {
                        let s = ls
                            .iter()
                            .filter_map(|l| {
                                let id = l.get("project_id")?.as_str()?;
                                let st = l.get("state")?.as_str()?;
                                Some(format!("{id} {}", super::state_zh(st)))
                            })
                            .collect::<Vec<_>>()
                            .join("  ");
                        (s, true)
                    }
                    _ => ("无房间".into(), true),
                }
            } else {
                ("状态?".into(), false)
            }
        }
        Err(_) => ("母未就绪".into(), false),
    }
}

fn sample_host(
    cpu_prev: &mut Option<(u64, u64)>,
    net_prev: &mut Option<(u64, u64, Instant)>,
) -> HostView {
    let (mem, swap) = mem_swap();
    let disk = disk_root();
    let (load, tasks) = load_tasks();
    let (net_down, net_up) = net_rate(net_prev);
    let (rooms, _ok) = rooms_line();
    let aliases = super::load_aliases();
    let mother = aliases
        .get("chat-mother")
        .and_then(|v| v.as_str())
        .unwrap_or("grok")
        .to_string();
    let mut ident_parts = vec![hostname(), cpu_line()];
    if let Some(u) = uptime_zh() {
        ident_parts.push(format!("已开 {u}"));
    }
    let clk = clock_hm();
    if !clk.is_empty() {
        ident_parts.push(clk);
    }
    HostView {
        ident: ident_parts.join("  ·  "),
        cpu_pct: cpu_pct(cpu_prev),
        load,
        tasks,
        temp: cpu_temp(),
        mem_used: mem.map(|x| x.0).unwrap_or(0),
        mem_total: mem.map(|x| x.1).unwrap_or(0),
        swap_used: swap.map(|x| x.0).unwrap_or(0),
        swap_total: swap.map(|x| x.1).unwrap_or(0),
        disk_used: disk.map(|x| x.0).unwrap_or(0),
        disk_total: disk.map(|x| x.1).unwrap_or(0),
        net_down,
        net_up,
        rooms,
        mother,
    }
}

fn ch_width(c: char) -> usize {
    UnicodeWidthChar::width(c).unwrap_or(0)
}

fn vis_col(s: &str, cur: usize) -> usize {
    s.chars().take(cur).map(ch_width).sum()
}

fn ascii_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '+' | '=' | '@' | '%')
}

fn wrap_text(s: &str, width: usize) -> Vec<String> {
    if width < 4 {
        return s.lines().map(|l| l.to_string()).collect();
    }
    let mut lines = Vec::new();
    for para in s.split('\n') {
        if para.is_empty() {
            lines.push(String::new());
            continue;
        }
        let chars: Vec<char> = para.chars().collect();
        let mut i = 0usize;
        let mut cur = String::new();
        let mut w = 0usize;
        while i < chars.len() {
            let c = chars[i];
            if c.is_ascii_whitespace() {
                let cw = ch_width(c);
                if w + cw > width && !cur.is_empty() {
                    lines.push(std::mem::take(&mut cur));
                    w = 0;
                    i += 1;
                    continue;
                }
                if w == 0 {
                    i += 1;
                    continue;
                }
                cur.push(c);
                w += cw;
                i += 1;
                continue;
            }
            let start = i;
            if ascii_word_char(c) {
                i += 1;
                while i < chars.len() && ascii_word_char(chars[i]) {
                    i += 1;
                }
            } else {
                i += 1;
            }
            let atom: String = chars[start..i].iter().collect();
            let aw: usize = atom.chars().map(ch_width).sum();
            if aw > width {
                for ch in atom.chars() {
                    let cw = ch_width(ch);
                    if w + cw > width && !cur.is_empty() {
                        lines.push(std::mem::take(&mut cur));
                        w = 0;
                    }
                    cur.push(ch);
                    w += cw;
                }
                continue;
            }
            if w + aw > width && !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
                w = 0;
            }
            cur.push_str(&atom);
            w += aw;
        }
        lines.push(cur);
    }
    lines
}

fn ingest_nl(raw: &str) -> Vec<(Kind, String)> {
    let t = raw.trim();
    let Ok(v) = serde_json::from_str::<Value>(t) else {
        return vec![(Kind::Out, super::human("", raw))];
    };
    if v.get("say").and_then(|x| x.as_str()).is_none() {
        return vec![(Kind::Sys, super::human("", raw))];
    }
    let mut out = Vec::new();
    let say = v.get("say").and_then(|x| x.as_str()).unwrap_or("").trim();
    if !say.is_empty() {
        out.push((Kind::Mother, format!("{say}\n")));
    }
    let do_line = v.get("do").and_then(|x| x.as_str()).unwrap_or("").trim();
    if let Some(core) = v.get("core").filter(|c| !c.is_null()) {
        let ptr = super::after_spawn_pointer(do_line, core);
        if !ptr.trim().is_empty() {
            out.push((Kind::Sys, ptr));
        }
    }
    if out.is_empty() {
        out.push((Kind::Sys, super::human("", raw)));
    }
    out
}

fn restore(term: &mut Terminal<CrosstermBackend<Stdout>>) {
    let _ = disable_raw_mode();
    let _ = execute!(
        term.backend_mut(),
        DisableMouseCapture,
        DisableBracketedPaste,
        LeaveAlternateScreen
    );
    let _ = term.show_cursor();
}

pub fn run() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    if let Err(e) = execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    ) {
        let _ = disable_raw_mode();
        return Err(e);
    }
    let mut term = match Terminal::new(CrosstermBackend::new(stdout)) {
        Ok(t) => t,
        Err(e) => {
            let mut out = io::stdout();
            let _ = execute!(
                out,
                DisableMouseCapture,
                DisableBracketedPaste,
                LeaveAlternateScreen
            );
            let _ = disable_raw_mode();
            return Err(e);
        }
    };
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let mut out = io::stdout();
        let _ = execute!(
            out,
            DisableMouseCapture,
            DisableBracketedPaste,
            LeaveAlternateScreen
        );
        hook(info);
    }));
    let mut app = App::new();
    let res = loop_app(&mut term, &mut app);
    restore(&mut term);
    app.save_hist();
    res
}

fn loop_app(term: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        while let Ok(job) = app.job_rx.try_recv() {
            match job {
                Job::Progress(s) => {
                    app.busy = true;
                    if let Some(j) = s.strip_prefix("STEP ") {
                        apply_step(app, j);
                    } else if app.progress.as_deref() != Some(s.as_str()) {
                        app.progress_at = Instant::now();
                        app.progress = Some(s.clone());
                    }
                }
                Job::Done(s) => {
                    finish_stream(app, true);
                    app.busy = false;
                    app.progress = None;
                    app.push(Kind::Out, s);
                    app.refresh_face();
                }
                Job::Nl(raw) => {
                    finish_stream(app, true);
                    app.busy = false;
                    app.progress = None;
                    for (k, t) in ingest_nl(&raw) {
                        app.push(k, t);
                    }
                    app.refresh_face();
                }
            }
        }
        if app.leave {
            break;
        }
        if app.status_at.elapsed() > Duration::from_millis(1000) {
            app.refresh_face();
        }
        if app.stream.iter().any(|s| matches!(s.st, StepSt::Run)) {
            app.spin = app.spin.wrapping_add(1);
        }
        term.draw(|f| draw(f, app))?;
        if !event::poll(Duration::from_millis(80))? {
            continue;
        }
        match event::read()? {
            Event::Resize(_, _) => {}
            Event::Paste(s) => {
                if app.keys {
                    continue;
                }
                match &mut app.secret {
                    Secret::On { buf, .. } => buf.push_str(&s),
                    Secret::Off => insert(&mut app.input, &mut app.cursor, &s),
                }
            }
            Event::Mouse(m) => {
                if app.keys {
                    continue;
                }
                match m.kind {
                    MouseEventKind::ScrollUp => {
                        app.follow = false;
                        app.scroll = app.scroll.saturating_add(3);
                    }
                    MouseEventKind::ScrollDown => {
                        if app.scroll <= 3 {
                            app.scroll = 0;
                            app.follow = true;
                        } else {
                            app.scroll = app.scroll.saturating_sub(3);
                        }
                    }
                    _ => {}
                }
            }
            Event::Key(k) if k.kind == KeyEventKind::Press || k.kind == KeyEventKind::Repeat => {
                if handle_key(app, k.code, k.modifiers) {
                    break;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn insert(s: &mut String, cur: &mut usize, add: &str) {
    let i = s.chars().take(*cur).map(|c| c.len_utf8()).sum();
    s.insert_str(i, add);
    *cur += add.chars().count();
}

fn delete_word(s: &mut String, cur: &mut usize) {
    if *cur == 0 {
        return;
    }
    let chars: Vec<char> = s.chars().collect();
    let mut i = *cur;
    while i > 0 && chars[i - 1].is_whitespace() {
        i -= 1;
    }
    while i > 0 && !chars[i - 1].is_whitespace() {
        i -= 1;
    }
    let start: usize = chars.iter().take(i).map(|c| c.len_utf8()).sum();
    let end: usize = chars.iter().take(*cur).map(|c| c.len_utf8()).sum();
    s.replace_range(start..end, "");
    *cur = i;
}

fn handle_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> bool {
    if mods.contains(KeyModifiers::CONTROL) {
        match code {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                app.keys = false;
                app.input.clear();
                app.cursor = 0;
                app.hist_idx = None;
                app.secret = Secret::Off;
                if !app.portal {
                    app.push(Kind::Sys, "^C\n".into());
                }
                return false;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if app.input.is_empty() && matches!(app.secret, Secret::Off) && !app.busy {
                    return true;
                }
                return false;
            }
            KeyCode::Char('l') | KeyCode::Char('L') => return false,
            KeyCode::Char('u') | KeyCode::Char('U') => {
                app.input.clear();
                app.cursor = 0;
                return false;
            }
            KeyCode::Char('w') | KeyCode::Char('W') => {
                delete_word(&mut app.input, &mut app.cursor);
                return false;
            }
            _ => {}
        }
    }
    if code == KeyCode::F(1) {
        app.keys = !app.keys;
        return false;
    }
    if app.keys {
        match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Char('?') => {
                app.keys = false;
            }
            _ => {}
        }
        return false;
    }
    if let Secret::On { adapter, buf } = &mut app.secret {
        match code {
            KeyCode::Esc => {
                app.secret = Secret::Off;
                app.push(Kind::Sys, "已取消投钥\n".into());
            }
            KeyCode::Enter => {
                let ad = adapter.clone();
                let key = std::mem::take(buf);
                app.secret = Secret::Off;
                if key.trim().is_empty() {
                    app.push(Kind::Sys, "失败  空钥\n".into());
                } else {
                    let line = format!("model key {ad} {key}");
                    app.push(Kind::Out, super::handle_model(&line));
                    app.refresh_face();
                }
            }
            KeyCode::Backspace => {
                buf.pop();
            }
            KeyCode::Char(c) if !mods.contains(KeyModifiers::CONTROL) => buf.push(c),
            _ => {}
        }
        return false;
    }
    match code {
        KeyCode::Esc => {
            if !app.input.is_empty() {
                app.input.clear();
                app.cursor = 0;
                app.hist_idx = None;
            }
        }
        KeyCode::Char('?') if app.input.is_empty() => {
            app.keys = true;
        }
        KeyCode::Enter => submit(app),
        KeyCode::Backspace => {
            if app.cursor > 0 {
                let i = app.input.chars().take(app.cursor - 1).map(|c| c.len_utf8()).sum();
                let ch = app
                    .input[i..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                app.input.replace_range(i..i + ch, "");
                app.cursor -= 1;
            }
        }
        KeyCode::Delete => {
            let i = app.input.chars().take(app.cursor).map(|c| c.len_utf8()).sum();
            if i < app.input.len() {
                let ch = app
                    .input[i..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                app.input.replace_range(i..i + ch, "");
            }
        }
        KeyCode::Left => {
            if app.cursor > 0 {
                app.cursor -= 1;
            }
        }
        KeyCode::Right => {
            if app.cursor < app.input.chars().count() {
                app.cursor += 1;
            }
        }
        KeyCode::Home => app.cursor = 0,
        KeyCode::End => app.cursor = app.input.chars().count(),
        KeyCode::Up => hist_prev(app),
        KeyCode::Down => hist_next(app),
        KeyCode::PageUp => {
            app.follow = false;
            app.scroll = app.scroll.saturating_add(8);
        }
        KeyCode::PageDown => {
            if app.scroll <= 8 {
                app.scroll = 0;
                app.follow = true;
            } else {
                app.scroll = app.scroll.saturating_sub(8);
            }
        }
        KeyCode::Char(c) if !mods.contains(KeyModifiers::CONTROL) => {
            insert(&mut app.input, &mut app.cursor, &c.to_string());
        }
        _ => {}
    }
    false
}

fn hist_prev(app: &mut App) {
    if app.hist.is_empty() {
        return;
    }
    match app.hist_idx {
        None => {
            app.draft = app.input.clone();
            app.hist_idx = Some(app.hist.len() - 1);
        }
        Some(0) => return,
        Some(i) => app.hist_idx = Some(i - 1),
    }
    if let Some(i) = app.hist_idx {
        app.input = app.hist[i].clone();
        app.cursor = app.input.chars().count();
    }
}

fn hist_next(app: &mut App) {
    let Some(i) = app.hist_idx else {
        return;
    };
    if i + 1 >= app.hist.len() {
        app.hist_idx = None;
        app.input = std::mem::take(&mut app.draft);
        app.cursor = app.input.chars().count();
        return;
    }
    app.hist_idx = Some(i + 1);
    app.input = app.hist[i + 1].clone();
    app.cursor = app.input.chars().count();
}

fn submit(app: &mut App) {
    if app.busy {
        return;
    }
    let line = app.input.trim().to_string();
    if line.is_empty() {
        return;
    }
    app.input.clear();
    app.cursor = 0;
    app.hist_idx = None;
    if line == "exit" || line == "quit" {
        app.leave = true;
        return;
    }
    if super::history_ok(&line) {
        app.hist.push(line.clone());
    }
    app.push(Kind::User, format!("{line}\n"));
    if super::as_nick_line(&line).is_some() {
        let msg = super::handle_nick(&line);
        app.nick = super::load_nick();
        app.push(Kind::Out, msg);
        return;
    }
    if let Some(ad) = super::model_key_needs_prompt(&line) {
        app.secret = Secret::On {
            adapter: ad.clone(),
            buf: String::new(),
        };
        app.push(
            Kind::Sys,
            format!("钥 {ad}（不回显，回车确认，Esc 取消）\n"),
        );
        return;
    }
    app.busy = true;
    app.progress_at = Instant::now();
    let tx = app.job_tx.clone();
    if super::is_direct_cmd(&line) {
        app.progress = Some("在办".into());
        thread::spawn(move || {
            let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                super::dispatch_pretty(&line)
            }));
            let s = match out {
                Ok(s) => s,
                Err(_) => "失败  内部错误\n".into(),
            };
            let _ = tx.send(Job::Done(s));
        });
    } else {
        app.progress = Some("推理".into());
        app.stream.clear();
        app.stream.push(Step {
            key: "think".into(),
            title: "推理".into(),
            detail: String::new(),
            st: StepSt::Run,
            t0: Instant::now(),
            ms: 0,
        });
        let tx_prog = tx.clone();
        thread::spawn(move || {
            let progress = move |s: &str| {
                let _ = tx_prog.send(Job::Progress(s.to_string()));
            };
            let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                super::talk_nl_with(&line, progress)
            }));
            match out {
                Ok(Ok(raw)) => {
                    let _ = tx.send(Job::Nl(raw));
                }
                Ok(Err(e)) => {
                    let _ = tx.send(Job::Done(format!("{e}\n")));
                }
                Err(_) => {
                    let _ = tx.send(Job::Done("失败  内部错误\n".into()));
                }
            }
        });
    }
}

fn end_running(app: &mut App, ok: bool) {
    let now = Instant::now();
    for s in app.stream.iter_mut().rev() {
        if matches!(s.st, StepSt::Run) {
            s.st = if ok { StepSt::Ok } else { StepSt::Fail };
            s.ms = now.saturating_duration_since(s.t0).as_millis() as u64;
            break;
        }
    }
}

fn apply_step(app: &mut App, raw: &str) {
    let Ok(v) = serde_json::from_str::<Value>(raw.trim()) else {
        return;
    };
    let op = v.get("op").and_then(|x| x.as_str()).unwrap_or("");
    let key = v.get("key").and_then(|x| x.as_str()).unwrap_or("");
    match op {
        "start" => {
            let title = v
                .get("title")
                .and_then(|x| x.as_str())
                .unwrap_or("步骤")
                .to_string();
            let detail = v
                .get("detail")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(s) = app
                .stream
                .iter_mut()
                .rev()
                .find(|s| s.key == key && matches!(s.st, StepSt::Run))
            {
                if !title.is_empty() {
                    s.title = title.clone();
                }
                if !detail.is_empty() {
                    s.detail = detail.clone();
                }
            } else {
                end_running(app, true);
                app.stream.push(Step {
                    key: key.to_string(),
                    title: title.clone(),
                    detail: detail.clone(),
                    st: StepSt::Run,
                    t0: Instant::now(),
                    ms: 0,
                });
            }
            let p = if detail.is_empty() {
                title
            } else {
                format!("{title}  {detail}")
            };
            app.progress = Some(p);
            app.progress_at = Instant::now();
        }
        "end" => {
            let ok = v.get("ok").and_then(|x| x.as_bool()).unwrap_or(true);
            if let Some(s) = app
                .stream
                .iter_mut()
                .rev()
                .find(|s| (key.is_empty() || s.key == key) && matches!(s.st, StepSt::Run))
            {
                s.st = if ok { StepSt::Ok } else { StepSt::Fail };
                s.ms = s.t0.elapsed().as_millis() as u64;
            }
        }
        _ => {}
    }
}

fn fmt_ms(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
    }
}

fn stream_text(steps: &[Step], spin: usize) -> String {
    let mut out = String::new();
    for s in steps {
        let mark = match s.st {
            StepSt::Run => SPIN[spin % SPIN.len()].to_string(),
            StepSt::Ok => "✓".into(),
            StepSt::Fail => "✗".into(),
        };
        let ms = match s.st {
            StepSt::Run => s.t0.elapsed().as_millis() as u64,
            _ => s.ms,
        };
        if s.detail.is_empty() {
            out.push_str(&format!("  {mark}  {:<4}  {}\n", s.title, fmt_ms(ms)));
        } else {
            out.push_str(&format!(
                "  {mark}  {:<4}  {}  {}\n",
                s.title,
                s.detail,
                fmt_ms(ms)
            ));
        }
    }
    out
}

fn finish_stream(app: &mut App, ok: bool) {
    if app.stream.is_empty() {
        return;
    }
    end_running(app, ok);
    let text = stream_text(&app.stream, 0);
    if !text.trim().is_empty() {
        app.push(Kind::Stream, text);
    }
    app.stream.clear();
}

fn live_progress(app: &App) -> String {
    let p = app.progress.as_deref().unwrap_or("在办");
    let s = app.progress_at.elapsed().as_secs();
    if s == 0 {
        format!("正在  {p}")
    } else {
        format!("正在  {p}  {s}秒")
    }
}

fn keys_hint(app: &App) -> (String, Color) {
    if app.keys {
        return ("Esc 关闭键位面板".into(), Color::Magenta);
    }
    if app.busy {
        return (format!("{}    F1 键位", live_progress(app)), Color::Magenta);
    }
    if matches!(app.secret, Secret::On { .. }) {
        return ("回车确认  Esc 取消  F1 键位".into(), Color::Yellow);
    }
    (
        "F1 键位   Enter 发送   ^C 清   ^D 离开   PgUp 滚".into(),
        Color::DarkGray,
    )
}

fn keys_body() -> String {
    "\
Enter            发送
Ctrl-C           清输入（不离开）
Ctrl-D           空行离开
Ctrl-U           清整行
Ctrl-W           删一词
Esc              清输入；面板开着则关闭
F1               打开/关闭本面板
空输入 ?         打开本面板
↑ ↓              历史
PgUp PgDn / 滚轮 对话滚动
Home End ← →     光标

exit / quit      离开
nick             看母昵称
nick 某某        改母昵称（输入栏标题）
nick 默认        复位为 OSER
help  /help      命令索引
model            模型
run              家里跑 PATH 命令名"
        .into()
}

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(w) / 2,
        y: area.y + area.height.saturating_sub(h) / 2,
        width: w,
        height: h,
    }
}

fn draw_keys_panel(f: &mut Frame, area: Rect) {
    let panel = centered(area, 62, 22);
    f.render_widget(Clear, panel);
    let block = Block::new()
        .borders(Borders::ALL)
        .title("键位")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(panel);
    f.render_widget(block, panel);
    f.render_widget(
        Paragraph::new(keys_body()).wrap(Wrap { trim: false }),
        inner,
    );
}

fn bar(pct: u16, w: usize) -> String {
    let w = w.max(6);
    let fill = ((pct as usize) * w / 100).min(w);
    let mut s = String::new();
    for i in 0..w {
        s.push(if i < fill { '█' } else { '░' });
    }
    s
}

fn pct_color(pct: u16) -> Color {
    if pct >= 90 {
        Color::Red
    } else if pct >= 70 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn ratio_pct(used: u64, total: u64) -> u16 {
    if total == 0 {
        0
    } else {
        ((used * 100) / total).min(100) as u16
    }
}

fn meter<'a>(label: &'a str, pct: u16, rest: String, bar_w: usize) -> Line<'a> {
    let color = pct_color(pct);
    Line::from(vec![
        Span::styled(pad_disp(label, 4), Style::default().fg(Color::Gray)),
        Span::styled(format!(" {} ", bar(pct, bar_w)), Style::default().fg(color)),
        Span::styled(format!("{pct:>3}%  {rest}"), Style::default().fg(color)),
    ])
}

fn draw_status(f: &mut Frame, area: Rect, app: &App, head_h: u16) {
    let hv = &app.face;
    let block = Block::new().borders(Borders::ALL).title("系统状态");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let w = inner.width.max(20) as usize;
    let bar_w = ((w / 4).clamp(8, 18)) as usize;
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        hv.ident.clone(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    let mut cpu_rest = format!("负载 {}", hv.load);
    if !hv.tasks.is_empty() {
        cpu_rest.push_str("  任务 ");
        cpu_rest.push_str(&hv.tasks);
    }
    if !hv.temp.is_empty() {
        cpu_rest.push_str("  ");
        cpu_rest.push_str(&hv.temp);
    }
    lines.push(meter("CPU", hv.cpu_pct, cpu_rest, bar_w));
    if hv.mem_total > 0 {
        let mut rest = format!(
            "{}/{}",
            hum_bytes(hv.mem_used),
            hum_bytes(hv.mem_total)
        );
        if hv.swap_total > 0 {
            rest.push_str(&format!(
                "  交换 {}/{}",
                hum_bytes(hv.swap_used),
                hum_bytes(hv.swap_total)
            ));
        }
        lines.push(meter(
            "内存",
            ratio_pct(hv.mem_used, hv.mem_total),
            rest,
            bar_w,
        ));
    }
    if hv.disk_total > 0 && head_h >= 6 {
        lines.push(meter(
            "存储",
            ratio_pct(hv.disk_used, hv.disk_total),
            format!(
                "{}/{}  /",
                hum_bytes(hv.disk_used),
                hum_bytes(hv.disk_total)
            ),
            bar_w,
        ));
    }
    if head_h >= 7 {
        lines.push(Line::from(vec![
            Span::styled("网络", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("  ↓ {}  ↑ {}", hv.net_down, hv.net_up),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!("    闲聊 {}", hv.mother),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }
    if head_h >= 8 {
        lines.push(Line::from(vec![
            Span::styled("房间", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("  {}", hv.rooms),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }
    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }),
        inner,
    );
}

fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    if area.width < 40 || area.height < 12 {
        f.render_widget(
            Paragraph::new("窗口太小，请拉大 SSH/真机终端。"),
            area,
        );
        return;
    }
    let head_h = if area.height >= 28 {
        8
    } else if area.height >= 22 {
        7
    } else if area.height >= 16 {
        6
    } else {
        5
    };
    let chunks = Layout::vertical([
        Constraint::Length(head_h),
        Constraint::Min(4),
        Constraint::Length(3),
        Constraint::Length(2),
    ])
    .split(area);

    draw_status(f, chunks[0], app, head_h);

    draw_chat(f, chunks[1], app);

    let (title, shown, cur) = if let Secret::On { adapter, buf } = &app.secret {
        (
            format!("{} · 投钥 {adapter}", app.nick),
            "*".repeat(buf.chars().count()),
            buf.chars().count(),
        )
    } else if app.busy {
        (app.nick.clone(), live_progress(app), 0)
    } else {
        (app.nick.clone(), app.input.clone(), app.cursor)
    };
    let input_block = Block::new()
        .borders(Borders::ALL)
        .title(title)
        .border_style(if app.busy {
            Style::default().fg(Color::Magenta)
        } else {
            Style::default().fg(Color::Green)
        });
    let inner = input_block.inner(chunks[2]);
    f.render_widget(input_block, chunks[2]);
    let (line, cur_x) = input_view("", &shown, cur, inner.width as usize);
    let input_style = if app.busy {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default()
    };
    f.render_widget(Paragraph::new(Span::styled(line, input_style)), inner);
    if !app.busy && !app.keys {
        let x = inner.x.saturating_add(cur_x);
        let x = x.min(inner.x.saturating_add(inner.width.saturating_sub(1)));
        f.set_cursor_position((x, inner.y));
    }

    let (hint, hint_color) = keys_hint(app);
    let keys_block = Block::new().borders(Borders::TOP).title("键");
    let keys_inner = keys_block.inner(chunks[3]);
    f.render_widget(keys_block, chunks[3]);
    f.render_widget(
        Paragraph::new(Span::styled(hint, Style::default().fg(hint_color))),
        keys_inner,
    );

    if app.keys {
        draw_keys_panel(f, area);
    }
}

fn pad_disp(s: &str, min_w: usize) -> String {
    let w = UnicodeWidthStr::width(s);
    if w >= min_w {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(min_w - w))
    }
}

fn pad_tag(name: &str, min_w: usize) -> String {
    let w = UnicodeWidthStr::width(name);
    if w >= min_w {
        format!("{name}  ")
    } else {
        format!("{name}{}  ", " ".repeat(min_w - w))
    }
}

const PORTAL_FULL: &[&str] = &[
    " █████╗ ██╗ ██████╗ ███████╗",
    "██╔══██╗██║██╔═══██╗██╔════╝",
    "███████║██║██║   ██║███████╗",
    "██╔══██║██║██║   ██║╚════██║",
    "██║  ██║██║╚██████╔╝███████║",
    "╚═╝  ╚═╝╚═╝ ╚═════╝ ╚══════╝",
];

const PORTAL_MINI: &[&str] = &["╔═════════╗", "║  AIOS   ║", "╚═════════╝"];

fn portal_art(width: u16, height: u16) -> &'static [&'static str] {
    let fw = PORTAL_FULL
        .iter()
        .map(|l| UnicodeWidthStr::width(*l))
        .max()
        .unwrap_or(0);
    if (width as usize) >= fw && (height as usize) >= PORTAL_FULL.len() {
        PORTAL_FULL
    } else {
        PORTAL_MINI
    }
}

fn draw_portal(f: &mut Frame, inner: Rect) {
    let art = portal_art(inner.width, inner.height);
    let w = art
        .iter()
        .map(|l| UnicodeWidthStr::width(*l) as u16)
        .max()
        .unwrap_or(1)
        .min(inner.width)
        .max(1);
    let h = (art.len() as u16).min(inner.height).max(1);
    let area = centered(inner, w, h);
    let style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let lines: Vec<Line> = art
        .iter()
        .map(|l| Line::from(Span::styled(*l, style)))
        .collect();
    f.render_widget(Paragraph::new(lines), area);
}

fn draw_chat(f: &mut Frame, area: Rect, app: &mut App) {
    let title = if app.portal { "" } else { "对话" };
    let block = Block::new().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if app.portal {
        draw_portal(f, inner);
        return;
    }
    let width = inner.width.max(1) as usize;
    let tag_w = UnicodeWidthStr::width(app.nick.as_str())
        .max(UnicodeWidthStr::width(USER_TAG))
        .max(UnicodeWidthStr::width("输出"))
        .max(2);
    let mut lines: Vec<Line> = Vec::new();
    for m in &app.msgs {
        let (tag, style) = match m.kind {
            Kind::Sys => (
                pad_tag("·", tag_w),
                Style::default().fg(Color::DarkGray),
            ),
            Kind::User => (
                pad_tag(USER_TAG, tag_w),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Kind::Mother => (
                pad_tag(&app.nick, tag_w),
                Style::default().fg(Color::Cyan),
            ),
            Kind::Out => (
                pad_tag("输出", tag_w),
                Style::default().fg(Color::Yellow),
            ),
            Kind::Stream => (
                pad_tag("流", tag_w),
                Style::default().fg(Color::Magenta),
            ),
        };
        let tw = UnicodeWidthStr::width(tag.as_str());
        let body_w = width.saturating_sub(tw).max(8);
        let wrapped = wrap_text(m.text.trim_end(), body_w);
        for (i, w) in wrapped.iter().enumerate() {
            if i == 0 {
                lines.push(Line::from(vec![
                    Span::styled(tag.clone(), style),
                    Span::styled(w.clone(), style),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw(" ".repeat(tw)),
                    Span::styled(w.clone(), style),
                ]));
            }
        }
        lines.push(Line::from(""));
    }
    if !app.stream.is_empty() {
        let tag = pad_tag("流", tag_w);
        let style = Style::default().fg(Color::Magenta);
        let tw = UnicodeWidthStr::width(tag.as_str());
        let body_w = width.saturating_sub(tw).max(8);
        let live = stream_text(&app.stream, app.spin);
        let wrapped = wrap_text(live.trim_end(), body_w);
        for (i, w) in wrapped.iter().enumerate() {
            if i == 0 {
                lines.push(Line::from(vec![
                    Span::styled(tag.clone(), style),
                    Span::styled(w.clone(), style),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw(" ".repeat(tw)),
                    Span::styled(w.clone(), style),
                ]));
            }
        }
        lines.push(Line::from(""));
    }
    let h = inner.height as usize;
    let total = lines.len();
    let max_off = total.saturating_sub(h);
    let off = if app.follow {
        max_off
    } else {
        max_off.saturating_sub(app.scroll as usize)
    };
    if app.follow {
        app.scroll = 0;
    } else if app.scroll as usize > max_off {
        app.scroll = max_off as u16;
    }
    let view: Vec<Line> = lines.into_iter().skip(off).take(h).collect();
    f.render_widget(Paragraph::new(view).wrap(Wrap { trim: false }), inner);
}

fn input_view(prefix: &str, shown: &str, cur: usize, width: usize) -> (String, u16) {
    let pre_w = UnicodeWidthStr::width(prefix);
    let avail = width.saturating_sub(pre_w).max(1);
    let col = vis_col(shown, cur);
    let start = if col + 1 > avail {
        col + 1 - avail
    } else {
        0
    };
    let mut skipped = 0usize;
    let mut view = String::new();
    let mut w = 0usize;
    for ch in shown.chars() {
        let cw = ch_width(ch);
        if skipped < start {
            skipped += cw;
            continue;
        }
        if w + cw > avail {
            break;
        }
        view.push(ch);
        w += cw;
    }
    (
        format!("{prefix}{view}"),
        (pre_w + col.saturating_sub(start)) as u16,
    )
}

#[cfg(test)]
mod wrap_tests {
    use super::wrap_text;

    #[test]
    fn does_not_split_ascii_words() {
        let lines = wrap_text("PID 1 是 systemd; 眼前这段 CPU/内存占用都很低。", 16);
        let blob = lines.join("\n");
        assert!(
            lines.iter().any(|l| l.contains("systemd")),
            "systemd split: {blob:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("CPU")),
            "CPU split: {blob:?}"
        );
        assert!(!blob.contains("sy\nst"), "{blob:?}");
        assert!(!blob.contains("C\nPU"), "{blob:?}");
    }

    #[test]
    fn keeps_explicit_newlines() {
        assert_eq!(wrap_text("a\nb\n\nc", 10), vec!["a", "b", "", "c"]);
    }

    #[test]
    fn portal_picks_full_when_room() {
        let a = super::portal_art(40, 8);
        assert!(a.iter().any(|l| l.contains("██")), "{a:?}");
        let b = super::portal_art(8, 3);
        assert!(b.iter().any(|l| l.contains("AIOS")), "{b:?}");
    }
}
