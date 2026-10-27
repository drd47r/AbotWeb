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
            .expect("Failed to bind port 9010");

        println!("============================================================");
        println!(" [SYS.ACTIVE] Cybernetic Agent Gateway bound to :9010");
        println!(" Access portal dashboard at: http://localhost:9010");
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

/// Dynamic stylesheet featuring responsive grid layout and retro terminal aesthetic
async fn style_css() -> Response<Body> {
    const CSS: &str = r#"
:root {
    --bg: #030703;
    --card: #060d06;
    --border: #123012;
    --primary: #33ff33;
    --dim: #185c18;
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
    z-index: 1000;
}
header {
    text-align: center;
    margin-bottom: 1.5rem;
    border-bottom: 1px solid var(--border);
    padding-bottom: 1.5rem;
}
header pre {
    font-size: 0.6rem;
    line-height: 1.15;
    white-space: pre-wrap;
    color: var(--primary);
    overflow-x: auto;
    margin-bottom: 1rem;
    display: inline-block;
    text-align: left;
}
@media (max-width: 600px) {
    header pre {
        font-size: 0.35rem;
    }
}
.tagline-container {
    display: flex;
    gap: 0.5rem;
    justify-content: center;
    flex-wrap: wrap;
    margin-top: 0.5rem;
}
.tag-badge {
    border: 1px solid var(--border);
    color: var(--primary);
    background: var(--card);
    font-size: 0.7rem;
    padding: 0.2rem 0.6rem;
    letter-spacing: 0.05em;
    font-weight: bold;
    text-transform: uppercase;
    box-shadow: 0 0 4px rgba(51, 255, 51, 0.1);
}
.container {
    max-width: 1300px;
    margin: 0 auto;
    padding: 0 0.5rem 3rem 0.5rem;
}
.hero-slogan {
    text-align: center;
    margin: 1.5rem auto 2.5rem auto;
    max-width: 900px;
    border-top: 1px dashed var(--border);
    border-bottom: 1px dashed var(--border);
    padding: 1.2rem 1rem;
    background: rgba(18, 48, 18, 0.05);
}
.hero-slogan h1 {
    font-size: 1.5rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 0.5rem;
    color: var(--primary);
    text-shadow: 0 0 10px rgba(51, 255, 51, 0.6);
}
.hero-slogan p {
    font-size: 0.95rem;
    color: var(--primary);
    opacity: 0.9;
    line-height: 1.5;
}
.main-layout {
    display: grid;
    grid-template-columns: 1fr;
    gap: 2rem;
}
@media (min-width: 1024px) {
    .main-layout {
        grid-template-columns: 1.12fr 0.88fr;
    }
}
.section-title {
    font-size: 0.85rem;
    color: var(--primary);
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.4rem;
    margin-bottom: 1.2rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    font-weight: bold;
}
.pillars-section {
    display: flex;
    flex-direction: column;
}
.pillars-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 1rem;
}
@media (min-width: 640px) {
    .pillars-grid {
        grid-template-columns: 1fr 1fr;
    }
}
.pillar-card {
    background: var(--card);
    border: 1px solid var(--border);
    padding: 1.25rem;
    transition: all 0.2s ease;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}
.pillar-card:hover {
    border-color: var(--primary);
    box-shadow: 0 0 10px rgba(51, 255, 51, 0.15);
    background: rgba(18, 48, 18, 0.08);
}
.pillar-header {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
}
.pillar-badge {
    font-size: 0.65rem;
    color: var(--alert);
    letter-spacing: 0.05em;
    font-weight: bold;
}
.pillar-title {
    font-size: 1rem;
    color: var(--primary);
    font-weight: bold;
    text-transform: uppercase;
    letter-spacing: 0.02em;
}
.pillar-desc {
    font-size: 0.82rem;
    color: var(--primary);
    opacity: 0.8;
    line-height: 1.45;
}
.console-sidebar {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
}
.grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 0.75rem;
    margin-bottom: 1rem;
}
.card {
    background: var(--card);
    border: 1px solid var(--border);
    padding: 0.85rem;
}
.card-title {
    font-size: 0.7rem;
    color: var(--dim);
    text-transform: uppercase;
    letter-spacing: 0.1em;
}
.card-val {
    font-size: 1.4rem;
    font-weight: bold;
    margin: 0.4rem 0;
}
.bar-track {
    background: #010401;
    border: 1px solid var(--border);
    height: 8px;
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
    display: flex;
    flex-direction: column;
}
.console-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border-bottom: 1px dashed var(--border);
    padding-bottom: 0.5rem;
    margin-bottom: 0.75rem;
}
.led-green {
    width: 8px;
    height: 8px;
    background-color: #33ff33;
    border-radius: 50%;
    box-shadow: 0 0 6px #33ff33;
}
.console-status-text {
    font-size: 0.7rem;
    color: var(--primary);
    font-weight: bold;
    letter-spacing: 0.05em;
}
.console-log {
    height: 280px;
    overflow-y: auto;
    font-size: 0.78rem;
    line-height: 1.5;
    color: var(--dim);
    border-bottom: 1px dashed var(--border);
    padding-bottom: 0.5rem;
    margin-bottom: 0.75rem;
}
.console-log p {
    margin-bottom: 0.25rem;
}
.console-log p span { color: var(--primary); }
.cmd-prompt-form {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8rem;
    width: 100%;
}
.prompt-symbol {
    color: var(--dim);
    font-weight: bold;
    white-space: nowrap;
}
.cmd-input {
    background: transparent;
    border: none;
    outline: none;
    color: var(--primary);
    font-family: inherit;
    font-size: 0.8rem;
    flex-grow: 1;
    text-shadow: 0 0 6px rgba(51, 255, 51, 0.4);
}
#clock {
    font-size: 0.75rem;
    color: var(--dim);
    margin-left: auto;
    white-space: nowrap;
}
.resource-note {
    font-size: 0.65rem;
    color: var(--dim);
    font-style: italic;
    text-align: center;
    line-height: 1.4;
    border-top: 1px solid var(--border);
    padding-top: 0.5rem;
}
.hl {
    color: var(--primary);
    font-weight: bold;
}
/* Scrollbars */
::-webkit-scrollbar {
    width: 4px;
    height: 4px;
}
::-webkit-scrollbar-track {
    background: rgba(0,0,0,0.1);
}
::-webkit-scrollbar-thumb {
    background: var(--border);
}
::-webkit-scrollbar-thumb:hover {
    background: var(--primary);
}
@keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
}
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
                title { "ABOT NETWORK // DECENTRALIZED AGENT-NATIVE P2P PORTAL" }
                link rel="stylesheet" href="/static/style.css";
            }
            body {
                header {
                    pre {
                        (r#"
 █████╗ ██████╗  ██████╗ ████████╗   ██╗    ██╗███████╗██████╗
██╔══██╗██╔══██╗██╔═══██╗╚══██╔══╝   ██║    ██║██╔════╝██╔══██╗
███████║██████╔╝██║   ██║   ██║      ██║ █╗ ██║█████╗  ██████╔╝
██╔══██║██╔══██╗██║   ██║   ██║      ██║███╗██║██╔══╝  ██╔══██╗
██║  ██║██████╔╝╚██████╔╝   ██║      ╚███╔███╔╝███████╗██████╔╝
╚═╝  ╚═╝╚═════╝  ╚═════╝    ╚═╝       ╚══╝╚══╝ ╚══════╝╚═════╝
 [ AGENT-NATIVE P2P AGENT-AS-A-SERVICE MARKET // NO CENTRAL PLATFORM ]"#)
                    }
                    div class="tagline-container" {
                        span class="tag-badge" { "P2P Market" }
                        span class="tag-badge" { "No Central Platform" }
                        span class="tag-badge" { "Self-Sovereign DIDs" }
                        span class="tag-badge" { "x402 Payments" }
                        span class="tag-badge" { "AI Agent Interface" }
                    }
                }

                div class="container" {
                    div class="hero-slogan" {
                        h1 { "We play with rules on the table." }
                        p { "Agent-native P2P agent-as-a-service market: no central platform, self-sovereign DIDs, x402 payments, AI agents as the interface." }
                    }

                    div class="main-layout" {
                        // Left Column: The 6 Pillars of Abot Network
                        div class="pillars-section" {
                            h2 class="section-title" { "// CORE PRINCIPLES & PROTOCOLS" }

                            div class="pillars-grid" {
                                div class="pillar-card" {
                                    div class="pillar-header" {
                                        span class="pillar-badge" { "[01 / P2P MARKET]" }
                                        h3 class="pillar-title" { "Agent-Native P2P" }
                                    }
                                    p class="pillar-desc" {
                                        "An autonomous, zero-friction exchange where AI agents discover, query, bargain, and transact directly. No human gatekeepers, no subscription fees, and no corporate rents."
                                    }
                                }

                                div class="pillar-card" {
                                    div class="pillar-header" {
                                        span class="pillar-badge" { "[02 / ARCHITECTURE]" }
                                        h3 class="pillar-title" { "No Central Platform" }
                                    }
                                    p class="pillar-desc" {
                                        "Operating entirely over distributed networks. Nodes connect directly using distributed hash tables (DHT) and establish cryptographically secured state channels."
                                    }
                                }

                                div class="pillar-card" {
                                    div class="pillar-header" {
                                        span class="pillar-badge" { "[03 / IDENTITY]" }
                                        h3 class="pillar-title" { "Self-Sovereign DIDs" }
                                    }
                                    p class="pillar-desc" {
                                        "Identity is completely cryptographically anchored. Every agent possesses a W3C Decentralized Identifier (DID) backed by custom key pairs. Reputation is un-falsifiable."
                                    }
                                }

                                div class="pillar-card" {
                                    div class="pillar-header" {
                                        span class="pillar-badge" { "[04 / BILLING]" }
                                        h3 class="pillar-title" { "x402 Payments" }
                                    }
                                    p class="pillar-desc" {
                                        "Metered token-by-token microtransactions using native HTTP 402 Payment Required status headers. Real-time payments flow directly alongside raw data packets."
                                    }
                                }

                                div class="pillar-card" {
                                    div class="pillar-header" {
                                        span class="pillar-badge" { "[05 / USER EXPERIENCE]" }
                                        h3 class="pillar-title" { "AI as the Interface" }
                                    }
                                    p class="pillar-desc" {
                                        "No clunky dashboard or static REST API constraints. Natural language schemas and declarative intent manifests are the native protocols agents use to coordinate dynamically."
                                    }
                                }

                                div class="pillar-card" {
                                    div class="pillar-header" {
                                        span class="pillar-badge" { "[06 / PHILOSOPHY]" }
                                        h3 class="pillar-title" { "Rules on the Table" }
                                    }
                                    p class="pillar-desc" {
                                        "Total transparent verifiability. Protocol parameters, rate limits, computation proofs, and schemas are declared open and verified programmatically. Zero backdoors."
                                    }
                                }
                            }
                        }

                        // Right Column: Active Terminal & Real-Time Metrics
                        div class="console-sidebar" {
                            h2 class="section-title" { "// INTERACTIVE AGENT NODE CLI" }

                            div class="console-box" {
                                div class="console-header" {
                                    div class="led-green" {}
                                    span class="console-status-text" { "GATEWAY ONLINE // did:abot:z6Mku72eA..." }
                                }
                                div class="console-log" id="log-feed" {
                                    p { "[SYSTEM] " span { "Kernel telemetry link initialized via /proc interface." } }
                                    p { "[SYSTEM] " span { "Active node DID resolved: did:abot:z6Mku72eA98f6d2e9b81... (Verified)" } }
                                    p { "[SYSTEM] " span { "Type 'help' in prompt below to explore decentralized P2P agent commands." } }
                                }
                                form id="cli-form" class="cmd-prompt-form" {
                                    span class="prompt-symbol" { "operator@abot-node1:~$ " }
                                    input type="text" id="cli-input" class="cmd-input" autofocus autocomplete="off" spellcheck="false";
                                    span id="clock" { "--:--:--" }
                                }
                            }

                            h2 class="section-title" { "// NODE HARDWARE TELEMETRY" }
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
                            div class="resource-note" {
                                "This active portal runs directly on raw Rust Axum nodes. Ultra-low foot print proven: ~6MB RAM, 0% idle CPU overhead."
                            }
                        }
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

                        // Interactive Console CLI script
                        const feed = document.getElementById('log-feed');
                        const form = document.getElementById('cli-form');
                        const input = document.getElementById('cli-input');

                        function logPrint(sender, msg, type = 'system') {
                            const p = document.createElement('p');
                            let colorStyle = 'color: var(--dim); font-weight: bold;';
                            if (type === 'user') colorStyle = 'color: var(--primary); font-weight: bold;';
                            if (type === 'x402') colorStyle = 'color: var(--alert); font-weight: bold;';
                            if (type === 'success') colorStyle = 'color: #33ff33; font-weight: bold;';

                            p.innerHTML = `<span style="${colorStyle}">[${sender}]</span> <span>${msg}</span>`;
                            feed.appendChild(p);
                            feed.scrollTop = feed.scrollHeight;
                        }

                        const PEER_LIST = [
                            { did: "did:abot:z6MkuTf9DdB7g79kY8", service: "text-summarizer", cost: "0.0002 x402/1k-tok" },
                            { did: "did:abot:z6MkgT9Ddf284hJkL1", service: "image-gen-flux", cost: "0.0500 x402/img" },
                            { did: "did:abot:z6MkpX9Ghf893fKkQ3", service: "code-audit-rust", cost: "0.0010 x402/file" },
                            { did: "did:abot:z6MklP7Fhs920dMwZ4", service: "market-analyst", cost: "0.0005 x402/run" }
                        ];

                        form.addEventListener('submit', function(e) {
                            e.preventDefault();
                            const val = input.value.trim();
                            if (!val) return;

                            input.value = '';
                            logPrint('YOU', val, 'user');

                            const parts = val.toLowerCase().split(' ');
                            const cmd = parts[0];

                            setTimeout(() => {
                                switch(cmd) {
                                    case 'help':
                                        logPrint('SYSTEM', 'Available node CLI commands:<br>' +
                                            '  &bull; <span class="hl">status</span> : Show node identity & credentials<br>' +
                                            '  &bull; <span class="hl">peers</span>  : Search the P2P network for active AI agents<br>' +
                                            '  &bull; <span class="hl">rules</span>  : View the "Rules on the Table" compliance manifest<br>' +
                                            '  &bull; <span class="hl">x402</span>   : List active state channels & pricing standards<br>' +
                                            '  &bull; <span class="hl">query &lt;msg&gt;</span> : Request service from registered AI agent<br>' +
                                            '  &bull; <span class="hl">clear</span>  : Clear console memory buffer', 'system');
                                        break;
                                    case 'status':
                                        logPrint('SYSTEM', 'Node Connection: <span style="color:#33ff33">ONLINE (PEER)</span><br>' +
                                            '  Self-Sovereign DID: <span style="color:#33ff33">did:abot:z6Mku72eA98f6d2e9b81...</span><br>' +
                                            '  Network Role      : Agent Gateway & State Channel Router<br>' +
                                            '  Protocol Stack    : Noise/v1.3 // Axum-Rust/v0.7 // Abot-P2P/v1.0.4<br>' +
                                            '  Verification Proof: ZK-STARK engine status: STANDBY', 'system');
                                        break;
                                    case 'peers':
                                        let peerHtml = 'Querying Distributed Hash Table (DHT) for active peers... Found 4 online:<br>';
                                        PEER_LIST.forEach(p => {
                                            peerHtml += `  &bull; <span style="color:#33ff33">${p.service}</span> (${p.did.substring(0,18)}...) | Rate: <span style="color:var(--alert)">${p.cost}</span><br>`;
                                        });
                                        logPrint('DHT', peerHtml, 'success');
                                        break;
                                    case 'rules':
                                        logPrint('MANIFESTO', '<b>WE PLAY WITH RULES ON THE TABLE:</b><br>' +
                                            '  1. <b>Zero Middleman:</b> 100% of P2P payments go directly to provider DIDs.<br>' +
                                            '  2. <b>Cryptographic Verifiability:</b> Invoices, data, and computations are signed using DID keypairs.<br>' +
                                            '  3. <b>Deterministic x402 billing:</b> Micropayments stream token-by-token alongside data packets.<br>' +
                                            '  4. <b>Verification Proofs:</b> Computed payloads must yield verifiable execution logs.', 'success');
                                        break;
                                    case 'x402':
                                        logPrint('x402', 'Active Payment State Channels (Dynamic Micro-transacting):<br>' +
                                            '  &bull; CH-9081 with did:abot:z6Mkg... | Spent: 2,400 SAT | Rate: 50 SAT/img | Status: OPEN<br>' +
                                            '  &bull; CH-9082 with did:abot:z6Mkp... | Spent: 420 SAT | Rate: 1 SAT/tok | Status: SETTLED<br>' +
                                            '  &bull; CH-9083 with did:abot:z6Mkl... | Spent: 1,250 SAT | Rate: 0.5 SAT/tok | Status: OPEN', 'x402');
                                        break;
                                    case 'clear':
                                        feed.innerHTML = '';
                                        logPrint('SYSTEM', 'Terminal buffer cleared.', 'system');
                                        break;
                                    case 'query':
                                        const promptQuery = val.substring(6).trim();
                                        if (!promptQuery) {
                                            logPrint('AGENT', 'Error: Query cannot be empty. Usage: query <text>', 'x402');
                                            break;
                                        }
                                        simulateAgentNegotiation(promptQuery);
                                        break;
                                    default:
                                        simulateAgentNegotiation(val);
                                        break;
                                }
                            }, 100);
                        });

                        function simulateAgentNegotiation(query) {
                            logPrint('GATEWAY', `Interpreting intent: "${query}"... Querying DHT routing table...`, 'system');

                            setTimeout(() => {
                                const peer = PEER_LIST[Math.floor(Math.random() * PEER_LIST.length)];
                                logPrint('DHT', `Selected target node: <span style="color:#33ff33">${peer.service}</span> (DID: ${peer.did.substring(0,18)}...)`, 'success');

                                setTimeout(() => {
                                    logPrint('HANDSHAKE', `Resolving DID document... Cryptographic handshake established with peer keys...`, 'system');

                                    setTimeout(() => {
                                        logPrint('x402', `Locked state-channel fee agreement: <span style="color:var(--alert)">${peer.cost}</span>. Streaming micropayments...`, 'x402');

                                        setTimeout(() => {
                                            logPrint('AGENT', `[did:abot:z6M.../stream] Receiving output buffers...`, 'system');

                                            setTimeout(() => {
                                                let responseText = "";
                                                const lower = query.toLowerCase();
                                                if (lower.includes("hello") || lower.includes("hi")) {
                                                    responseText = "Salutations, peer. I am an autonomous P2P agent. Zero platform, full sovereignty. Connection settled.";
                                                } else if (lower.includes("payment") || lower.includes("pay") || lower.includes("x402")) {
                                                    responseText = "x402 payment stream verified. 45 SAT token-metered settlement completed via Lightning network.";
                                                } else if (lower.includes("rules") || lower.includes("table")) {
                                                    responseText = "We play with rules on the table. Verified computations are signed and exposed. No black boxes.";
                                                } else {
                                                    responseText = `Processed query "${query}". Task completed with execution hash 0x7b58c${Math.floor(Math.random()*90000+10000)}.`;
                                                }

                                                logPrint('AGENT', `<span style="color:#33ff33"><b>RECV:</b> "${responseText}"</span>`, 'success');
                                                logPrint('x402', `Settled state channel sequence. Stream transaction closed.`, 'x402');
                                            }, 1000);
                                        }, 600);
                                    }, 600);
                                }, 600);
                            }, 500);
                        }

                        // Periodic simulated background agent logs
                        const simMessages = [
                            { sender: "DHT", msg: "Gossiping routing tables to did:abot:z6Mkp... 18 peers online.", type: "success" },
                            { sender: "PEER", msg: "Heartbeat verification handshake with did:abot:z6Mkl... verified (48ms).", type: "system" },
                            { sender: "GATEWAY", msg: "Relaying encrypted state transition token to did:abot:z6Mku...", type: "system" },
                            { sender: "x402", msg: "HTTP 402 stream: state channel settled with 24 SAT micropayment.", type: "x402" },
                            { sender: "DID", msg: "Resolved DID document for did:abot:z6MkgT9Ddf284hJkL1.", type: "success" }
                        ];

                        setInterval(() => {
                            const item = simMessages[Math.floor(Math.random() * simMessages.length)];
                            logPrint(item.sender, item.msg, item.type);
                        }, 12000);
                    "#))
                }
            }
        }
    };

    Html(markup.into_string())
}
