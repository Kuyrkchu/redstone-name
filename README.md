# Redstone VPN Bot — initial Rust implementation

This is an initial bot, **not a production VPN service**. VPN provisioning, payments, subscription enforcement, support ticket routing, idempotency persistence, audit and complete account deletion are not yet implemented. Do not sell subscriptions or distribute VPN configurations with this build.

## Run (Ubuntu)
Install a supported Rust toolchain and build dependencies, then:

```bash
export TELEGRAM_BOT_TOKEN='your private token' # enter only in your own shell/server
cargo run --release
```

For production, inject TELEGRAM_BOT_TOKEN from a secret manager or a protected systemd EnvironmentFile. Never publish it. SQLite DB is persisted in `redstone.db` by default.

Commands: /start, /help, /status, /servers, /plans, /config, /support, /delete_account, /admin.

Security: Admin check uses numeric Telegram ID 9215677457, not username. Only private-chat commands are handled. Callback routing must be restricted to private chats before production rollout. The code has not been compiled or integration-tested in this environment.
