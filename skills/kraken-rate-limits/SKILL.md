---
name: kraken-rate-limits
version: 1.0.0
description: "Understand and manage API rate limit budgets across spot and futures."
metadata:
  openclaw:
    category: "finance"
  requires:
    bins: ["kraken"]
---

# kraken-rate-limits

Use this skill for:
- understanding call budgets per tier (starter, intermediate, pro)
- spacing requests to avoid rate limit errors
- managing separate spot and futures rate limits
- building agent loops that stay within bounds

## Spot Rate Limits

Kraken spot uses a counter/decay model. Each call adds to a counter; the counter decays over time. If the counter exceeds the tier limit, requests are rejected.

| Tier | Max Counter | Decay Rate |
|------|-------------|------------|
| Starter | 15 | 1 per 3s |
| Intermediate | 20 | 1 per 2s |
| Pro | 20 | 1 per 1s |

Most calls cost 1 point. Heavier calls (ledgers, trade history, recent trades, query-ledgers) cost 2. Check your tier:

```bash
kraken volume -o json 2>/dev/null
```

Set the tier in config for accurate local tracking:

```toml
[settings]
rate_tier = "starter"
```

## Futures Rate Limits

Futures uses a token-bucket model. Tokens refill at a fixed rate. Each call consumes tokens; if the bucket is empty, requests queue or fail.

The CLI handles futures rate limiting internally. Agents do not need to track futures buckets manually.

## Agent Spacing Patterns

For starter tier with 15-point budget and 3s decay:

- Safe burst: 10 calls, then pause 30s
- Sustained: 1 call per 3s indefinitely
- Mixed read/trade: alternate reads (1 point) and trades (1 point), 1 per 3s

For polling loops, prefer intervals that stay well under the limit:

```bash
# Safe polling interval for starter tier
while true; do
  kraken ticker BTCUSD -o json 2>/dev/null
  sleep 5
done
```

## Prefer WebSocket Over Polling

Streaming does not consume REST rate limit points. For real-time data, always prefer:

```bash
kraken ws ticker BTC/USD -o json 2>/dev/null
```

Over repeated REST calls.

## Handling Rate Limit Errors

When the CLI receives a rate limit error, the agent should:

1. Parse the `rate_limit` error category.
2. Back off for at least 5 seconds (starter) or 3 seconds (intermediate/pro).
3. Reduce polling frequency going forward.
4. Do not retry immediately; the counter needs time to decay.

## Multi-Command Budget Planning

Before executing a sequence, estimate total cost:

```bash
# This sequence costs ~5 points:
kraken ticker BTCUSD -o json 2>/dev/null       # 1 point
kraken balance -o json 2>/dev/null             # 1 point
kraken open-orders -o json 2>/dev/null         # 1 point
kraken trades-history -o json 2>/dev/null      # 2 points (heavy call)
```

Leave headroom for retries and unexpected calls.
