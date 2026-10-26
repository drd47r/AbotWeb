use axum::{
    body::Body,
    extract::State,
    http::{header, Response, StatusCode},
    response::Html,
    routing::get,
    Json, Router,
};
use maud::{html, Markup, PreEscaped, DOCTYPE};
use serde::Serialize;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

struct TelemetryTracker {
    last_idle: AtomicU64,
    last_total: AtomicU64,
}

#[derive(Serialize)]
struct SystemStats {
    cpu_usage_pct: f32,
    mem_used_mb: u64,
    mem_total_mb: u64,
    mem_pct: f32,
    uptime_secs: u64,
    load_avg: String,
}

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let tracker = Arc::new(TelemetryTracker {
            last_idle: AtomicU64::new(0),
            last_total: AtomicU64::new(0),
        });

        let app = Router::new()
            .route("/", get(dashboard_view))
            .route("/api/telemetry", get(get_telemetry))
            .route("/static/style.css", get(style_css))
            .with_state(tracker);

        let addr = "0.0.0.0:9010";
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Failed to bind port 3000");

        println!("============================================================");
        println!(" [SYS.ACTIVE] Cybernetic Technician Terminal bound to :3000");
        println!(" Access dashboard at: http://localhost:3000");
        println!("============================================================");

        axum::serve(listener, app)
            .await
            .expect("Server failed to run");
    });
}

/// Reads Linux /proc counters with near-zero CPU/RAM impact
async fn get_telemetry(State(tracker): State<Arc<TelemetryTracker>>) -> Json<SystemStats> {
    // 1. CPU Usage calculation from /proc/stat
    let mut cpu_pct = 0.0f32;
    if let Ok(stat) = fs::read_to_string("/proc/stat") {
        if let Some(line) = stat.lines().next() {
            let parts: Vec<u64> = line
                .split_whitespace()
                .skip(1)
                .filter_map(|s| s.parse::<u64>().ok())
                .collect();
            if parts.len() >= 4 {
                let idle = parts[3];
                let total: u64 = parts.iter().sum();
                let prev_idle = tracker.last_idle.swap(idle, Ordering::Relaxed);
                let prev_total = tracker.last_total.swap(total, Ordering::Relaxed);

                let diff_idle = idle.saturating_sub(prev_idle);
                let diff_total = total.saturating_sub(prev_total);
                if diff_total > 0 {
                    cpu_pct = (1.0 - (diff_idle as f32 / diff_total as f32)) * 100.0;
                }
            }
        }
    }

    // 2. Memory Info from /proc/meminfo
    let mut total_kb = 0u64;
    let mut avail_kb = 0u64;
    if let Ok(mem) = fs::read_to_string("/proc/meminfo") {
        for line in mem.lines() {
            if line.starts_with("MemTotal:") {
                total_kb = parse_kb(line);
            } else if line.starts_with("MemAvailable:") {
                avail_kb = parse_kb(line);
            }
            if total_kb > 0 && avail_kb > 0 {
                break;
            }
        }
    }
    let used_kb = total_kb.saturating_sub(avail_kb);
    let mem_used_mb = used_kb / 1024;
    let mem_total_mb = total_kb / 1024;
    let mem_pct = if mem_total_mb > 0 {
        (mem_used_mb as f32 / mem_total_mb as f32) * 100.0
    } else {
        0.0
    };

    // 3. Uptime & Load from /proc/uptime and /proc/loadavg
    let uptime_secs = fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok())
        .map(|s| s as u64)
        .unwrap_or(0);

    let load_avg = fs::read_to_string("/proc/loadavg")
        .map(|s| s.split_whitespace().take(3).collect::<Vec<_>>().join(" "))
        .unwrap_or_else(|_| "0.00 0.00 0.00".into());

    Json(SystemStats {
        cpu_usage_pct: cpu_pct.clamp(0.0, 100.0),
        mem_used_mb,
        mem_total_mb,
        mem_pct: mem_pct.clamp(0.0, 100.0),
        uptime_secs,
        load_avg,
    })
}

fn parse_kb(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
}

/// Embedded Micro-CSS (< 2 KB) with CRT scanline filter
async fn style_css() -> Response<Body> {
    const CSS: &str = r#"
:root {
    --bg: #050a05;
    --card: #081208;
    --border: #143514;
    --primary: #33ff33;
    --dim: #1e6b1e;
    --alert: #ffaa00;
}
* { box-sizing: border-box; margin: 0; padding: 0; font-family: ui-monospace, "Cascadia Code", "Fira Code", monospace; }
body {
    background: var(--bg);
    color: var(--primary);
    padding: 1.5rem;
    min-height: 100vh;
    text-shadow: 0 0 6px rgba(51, 255, 51, 0.4);
    position: relative;
}
/* CRT Scanline overlay */
body::before {
    content: " ";
    position: fixed;
    top: 0; left: 0; bottom: 0; right: 0;
    background: linear-gradient(rgba(18, 16, 16, 0) 50%, rgba(0, 0, 0, 0.35) 50%),
                linear-gradient(90deg, rgba(255, 0, 0, 0.03), rgba(0, 255, 0, 0.01), rgba(0, 0, 255, 0.03));
    background-size: 100% 3px, 6px 100%;
    pointer-events: none;
    z-index: 100;
}
header pre {
    font-size: 0.65rem;
    line-height: 1.15;
    white-space: pre;
    color: var(--primary);
    overflow-x: auto;
    margin-bottom: 1.5rem;
}
.grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
    margin-bottom: 1.5rem;
}
.card {
    background: var(--card);
    border: 1px solid var(--border);
    padding: 1rem;
}
.card-title {
    font-size: 0.75rem;
    color: var(--dim);
    text-transform: uppercase;
    letter-spacing: 0.1em;
}
.card-val {
    font-size: 1.7rem;
    font-weight: bold;
    margin: 0.5rem 0;
}
.bar-track {
    background: #020502;
    border: 1px solid var(--border);
    height: 10px;
    width: 100%;
}
.bar-fill {
    background: var(--primary);
    height: 100%;
    width: 0%;
    transition: width 0.4s ease;
    box-shadow: 0 0 8px var(--primary);
}
.console-box {
    background: var(--card);
    border: 1px solid var(--border);
    padding: 1rem;
}
.console-log {
    height: 140px;
    overflow-y: auto;
    font-size: 0.8rem;
    line-height: 1.5;
    color: var(--dim);
    border-bottom: 1px dashed var(--border);
    padding-bottom: 0.5rem;
    margin-bottom: 0.5rem;
}
.console-log p span { color: var(--primary); }
.cmd-prompt {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
}
.cursor { animation: blink 1s step-end infinite; }
@keyframes blink { 50% { opacity: 0; } }
"#;

    Response::builder()
        .header(header::CONTENT_TYPE, "text/css; charset=utf-8")
        .body(Body::from(CSS))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap()
        })
}

/// Server-side compiled HTML dashboard
async fn dashboard_view() -> Html<String> {
    let markup: Markup = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { "TELEMETRY // RUST-CORE" }
                link rel="stylesheet" href="/static/style.css";
            }
            body {
                header {
                    pre {
                        (r#"
   ___ _   _ ___ _____     ____ ___  ____  _____
  / __\ \ / / __|_   _|   / ___/ _ \|  _ \| ____|
 | |   \ V /\__ \ | |    | |  | | | | |_) |  _|
 | |___ | | |___/ | |    | |__| |_| |  _ <| |___
  \____||_| |___/ |_|     \____\___/|_| \_\_____|
 [ SYS.MONITOR // LINUX KERNEL LIVE METRICS // AXUM ZERO-BLOAT ]"#)
                    }
                }

                div class="grid" {
                    div class="card" {
                        div class="card-title" { "CPU UTILIZATION" }
                        div class="card-val" id="cpu-txt" { "--%" }
                        div class="bar-track" { div class="bar-fill" id="cpu-bar" {} }
                    }
                    div class="card" {
                        div class="card-title" { "MEMORY CONSUMPTION" }
                        div class="card-val" id="mem-txt" { "-- / -- MB" }
                        div class="bar-track" { div class="bar-fill" id="mem-bar" {} }
                    }
                    div class="card" {
                        div class="card-title" { "SYS LOAD (1m, 5m, 15m)" }
                        div class="card-val" id="load-txt" { "-- -- --" }
                    }
                    div class="card" {
                        div class="card-title" { "NODE UPTIME" }
                        div class="card-val" id="uptime-txt" { "--" }
                    }
                }

                div class="console-box" {
                    div class="console-log" id="log-feed" {
                        p { "[SYSTEM] " span { "Kernel telemetry link initialized via /proc interface." } }
                        p { "[SYSTEM] " span { "Asynchronous event loop active on tokio thread-pool." } }
                        p { "[SYSTEM] " span { "Client connected. Streaming state vector every 1000ms." } }
                    }
                    div class="cmd-prompt" {
                        span { "operator@node01:~$ " }
                        span id="clock" { "--:--:--" }
                        span class="cursor" { " █" }
                    }
                }

                script {
                    (PreEscaped(r#"
async function refreshTelemetry() {
    try {
        const res = await fetch('/api/telemetry');
        const data = await res.json();

        document.getElementById('cpu-txt').textContent = data.cpu_usage_pct.toFixed(1) + '%';
        document.getElementById('cpu-bar').style.width = data.cpu_usage_pct + '%';

        document.getElementById('mem-txt').textContent = `${data.mem_used_mb} / ${data.mem_total_mb} MB`;
        document.getElementById('mem-bar').style.width = data.mem_pct + '%';

        document.getElementById('load-txt').textContent = data.load_avg;

        const h = Math.floor(data.uptime_secs / 3600);
        const m = Math.floor((data.uptime_secs % 3600) / 60);
        const s = data.uptime_secs % 60;
        document.getElementById('uptime-txt').textContent = `${h}h ${m}m ${s}s`;

        document.getElementById('clock').textContent = new Date().toLocaleTimeString();
    } catch(e) {
        console.error('Telemetry fetch error:', e);
    }
}
setInterval(refreshTelemetry, 1000);
refreshTelemetry();
"#))
                }
            }
        }
    };

    Html(markup.into_string())
}
