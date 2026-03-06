# kraken-cli Runtime Context for AI Agents

**This is experimental software. Commands interact with the live Kraken exchange and can result in real financial transactions. The user who deploys this tool is responsible for all outcomes. Test with `kraken paper` before using real funds. See `DISCLAIMER.md` for full terms.**

This file is optimized for runtime agent use. It defines how to call `kraken` safely and reliably.

## Core Invocation Contract

Always call:

```bash
kraken <command> [args...] -o json 2>/dev/null
```

Rules:
- Always pass `-o json`.
- Treat `stdout` as the only machine data channel.
- Treat `stderr` as diagnostics only.
- Exit code `0` means success.
- Non-zero exit means failure and `stdout` should contain a JSON error envelope.

## Authentication

For private endpoints:

```bash
export KRAKEN_API_KEY="your-key"
export KRAKEN_API_SECRET="your-secret"
```

Optional futures credentials:

```bash
export KRAKEN_FUTURES_API_KEY="your-futures-key"
export KRAKEN_FUTURES_API_SECRET="your-futures-secret"
```

Public market data and paper trading do not require credentials.

## Safety Rules

1. Never place live orders or withdrawals without explicit human approval.
2. Prefer `kraken paper ...` for strategy testing.
3. Validate orders before execution with `--validate`.
4. Use `cancel-after` for unattended sessions.
5. Never log or echo API secrets.

The authoritative list of dangerous commands is in `agents/tool-catalog.json` (every entry with `"dangerous": true`). Common examples:
- `kraken order buy/sell` and all batch/amend/edit order variants
- `kraken futures order buy/sell` and futures cancel/batch/edit variants
- `kraken withdraw`, `kraken wallet-transfer`
- `kraken order cancel-all`, `kraken futures cancel-all`
- `kraken earn allocate/deallocate`
- `kraken subaccount transfer`
- All WebSocket order mutations (`ws add-order`, `ws amend-order`, `ws cancel-order`, `ws batch-add`, `ws batch-cancel`)

This list is non-exhaustive. Always check the `dangerous` field in `tool-catalog.json`.

## Error Handling Contract

On failure, parse:

```json
{"error":"<category>","message":"<detail>"}
```

Route on `error`, not on `message`.

Common categories:
- `auth`: re-authenticate
- `rate_limit`: back off and retry
- `network`: retry with exponential backoff
- `validation`: fix inputs, do not retry unchanged request
- `api`: inspect request and parameters

For full envelopes and retry guidance: `agents/error-catalog.json`.

## Context Window Efficiency

Prefer narrower responses:
- Use limits like `--count`, `--depth`, `--offset`, `--limit` where available.
- Prefer pair-scoped calls over broad list calls.
- Prefer WebSocket NDJSON for streaming instead of high-frequency polling.

## High-Value Patterns

Public price read:

```bash
kraken ticker BTCUSD -o json
```

Safe order flow:

```bash
kraken order buy BTCUSD 0.001 --type limit --price 50000 --validate -o json
# ask for human approval
kraken order buy BTCUSD 0.001 --type limit --price 50000 -o json
```

Paper trading loop:

```bash
kraken paper init --balance 10000 -o json
kraken paper buy BTCUSD 0.01 -o json
kraken paper status -o json
```

## MCP Server

For MCP-compatible clients (Claude Desktop, ChatGPT, Codex, Gemini CLI, Cursor, VS Code, Windsurf), use the built-in MCP server:

```bash
kraken mcp -s market,trade,paper
```

This exposes CLI commands as structured MCP tools over stdio. No subprocess wrappers needed.

Security note:
- MCP is local-only and should run on your own machine.
- Agents connected to the same MCP server share the same configured Kraken account permissions.
- Do not expose or share this server outside systems you control.
- Always use `https://` and `wss://` endpoints.

## Tool Discovery

Use these machine-readable files:
- `agents/tool-catalog.json`: full command catalog (134 commands with parameter schemas and `dangerous` flags)
- `agents/error-catalog.json`: error categories and retry policy
