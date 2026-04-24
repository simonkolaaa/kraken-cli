use std::sync::Arc;
use axum::{
    routing::get,
    Router,
    Json,
    response::Html,
    extract::State,
};
use tokio::sync::RwLock;
use serde_json::json;
use crate::paper::PaperState;

pub(crate) async fn start_dashboard(state: Arc<RwLock<PaperState>>) {
    let app = Router::new()
        .route("/", get(serve_html))
        .route("/api/state", get(api_state))
        .with_state(state);

    let listener = match tokio::net::TcpListener::bind("0.0.0.0:3000").await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to bind dashboard server to port 3000: {}", e);
            return;
        }
    };
    
    tracing::info!("Dashboard server running on http://0.0.0.0:3000");
    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("Dashboard server error: {}", e);
    }
}

async fn serve_html() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Kraken Bot Dashboard</title>
    <style>
        body {
            background-color: #0d1117;
            color: #00ff00;
            font-family: 'Courier New', Courier, monospace;
            padding: 20px;
        }
        h1 { color: #00ff00; border-bottom: 1px solid #00ff00; padding-bottom: 10px; }
        .card {
            background-color: #161b22;
            border: 1px solid #30363d;
            border-radius: 6px;
            padding: 15px;
            margin-bottom: 20px;
        }
        table { width: 100%; border-collapse: collapse; }
        th, td { border: 1px solid #30363d; padding: 8px; text-align: left; }
        th { background-color: #21262d; }
        .buy { color: #56d364; }
        .sell { color: #f85149; }
    </style>
</head>
<body>
    <h1>🤖 Bot Dashboard (Live)</h1>
    <div class="card">
        <h2>Balances</h2>
        <div id="balances">Loading...</div>
    </div>
    <div class="card">
        <h2>Recent Trades</h2>
        <table id="trades-table">
            <thead>
                <tr>
                    <th>Time</th>
                    <th>Pair</th>
                    <th>Side</th>
                    <th>Volume</th>
                    <th>Price</th>
                </tr>
            </thead>
            <tbody id="trades-body">
            </tbody>
        </table>
    </div>

    <script>
        async function fetchState() {
            try {
                const res = await fetch('/api/state');
                const data = await res.json();
                
                // Render Balances
                let balHtml = '<ul>';
                for (const [asset, amount] of Object.entries(data.balances || {})) {
                    if (amount > 0) {
                        balHtml += `<li><strong>${asset}</strong>: ${amount.toFixed(4)}</li>`;
                    }
                }
                balHtml += '</ul>';
                document.getElementById('balances').innerHTML = balHtml;

                // Render Trades
                const tbody = document.getElementById('trades-body');
                tbody.innerHTML = '';
                if (data.filled_trades && data.filled_trades.length > 0) {
                    for (const t of data.filled_trades.slice().reverse().slice(0, 50)) {
                        const tr = document.createElement('tr');
                        const sideClass = t.side.toLowerCase() === 'buy' ? 'buy' : 'sell';
                        tr.innerHTML = `
                            <td>${t.filled_at || '-'}</td>
                            <td>${t.pair}</td>
                            <td class="${sideClass}">${t.side}</td>
                            <td>${t.volume.toFixed(4)}</td>
                            <td>${t.price.toFixed(4)}</td>
                        `;
                        tbody.appendChild(tr);
                    }
                } else {
                    tbody.innerHTML = '<tr><td colspan="5">No trades yet</td></tr>';
                }

            } catch (err) {
                console.error(err);
            }
        }

        setInterval(fetchState, 5000);
        fetchState();
    </script>
</body>
</html>
    "#)
}

async fn api_state(State(state): State<Arc<RwLock<PaperState>>>) -> Json<serde_json::Value> {
    let s = state.read().await;
    Json(json!(*s))
}
