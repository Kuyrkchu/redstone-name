use anyhow::{Context, Result};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use teloxide::{prelude::*, types::{InlineKeyboardButton, InlineKeyboardMarkup, ChatKind, MessageKind}, utils::command::BotCommands};

const ADMIN_ID: u64 = 9215677457;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "snake_case", description = "Команды Redstone VPN")]
enum Command {
    #[command(description = "Главное меню")]
    Start,
    #[command(description = "Помощь")]
    Help,
    #[command(description = "Мой статус")]
    Status,
    #[command(description = "Серверы")]
    Servers,
    #[command(description = "Тарифы")]
    Plans,
    #[command(description = "Конфигурация")]
    Config,
    #[command(description = "Поддержка")]
    Support,
    #[command(description = "Удалить аккаунт")]
    DeleteAccount,
    #[command(description = "Администрирование")]
    Admin,
}

fn menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("📊 Статус", "status"), InlineKeyboardButton::callback("🌍 Серверы", "servers")],
        vec![InlineKeyboardButton::callback("💎 Тарифы", "plans"), InlineKeyboardButton::callback("🔑 Конфигурация", "config")],
        vec![InlineKeyboardButton::callback("🆘 Поддержка", "support")],
    ])
}

async fn register(db: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO users (telegram_id) VALUES (?)")
        .bind(id).execute(db).await?;
    Ok(())
}

async fn response(db: &SqlitePool, id: i64, action: &str) -> Result<String> {
    let blocked: i64 = sqlx::query_scalar("SELECT blocked FROM users WHERE telegram_id = ?")
        .bind(id).fetch_one(db).await?;
    if blocked != 0 && action != "delete_account" { return Ok("Доступ к аккаунту заблокирован. Обратитесь в поддержку.".into()); }
    Ok(match action {
        "start" => "🔴 REDSTONE VPN\nЗащищённое подключение через разрешённые VPN-серверы. Выберите раздел ниже.".into(),
        "help" => "Команды: /start /status /servers /plans /config /support /delete_account".into(),
        "status" => "📊 Аккаунт создан. Активной подписки пока нет.".into(),
        "servers" => "🌍 VPN-серверы пока не подключены администратором.".into(),
        "plans" => "💎 Тарифы ещё не опубликованы. Оплата пока отключена.".into(),
        "config" => "🔑 Выдача VPN-конфигураций пока отключена: VPN-контроллер ещё не подключён.".into(),
        "support" => "🆘 Для поддержки отправьте сообщение администратору через согласованный канал. Автоматическая передача обращений ещё не подключена.".into(),
        "delete_account" => { sqlx::query("DELETE FROM users WHERE telegram_id = ?").bind(id).execute(db).await?; "Ваш аккаунт удалён из базы бота. Если у вас были VPN-ключи, их отзыв должен выполняться отдельно на VPN-сервере.".into() },
        "admin" if id as u64 == ADMIN_ID => "🛡 Вы — первоначальный администратор. Панель управления ещё не подключена.".into(),
        "admin" => "Нет доступа.".into(),
        _ => "Неизвестная команда. Используйте /help.".into(),
    })
}

async fn on_message(bot: Bot, msg: Message, cmd: Command, db: SqlitePool) -> ResponseResult<()> {
    if !matches!(msg.kind, MessageKind::Common(_)) || !matches!(msg.chat.kind, ChatKind::Private(_)) { return Ok(()); }
    let Some(user) = msg.from.as_ref() else { return Ok(()); };
    let id = match i64::try_from(user.id.0) { Ok(id) => id, Err(_) => return Ok(()) };
    if let Err(e) = register(&db, id).await { tracing::error!(error = %e, "database error"); return Ok(()); }
    let action = match cmd { Command::Start => "start", Command::Help => "help", Command::Status => "status", Command::Servers => "servers", Command::Plans => "plans", Command::Config => "config", Command::Support => "support", Command::DeleteAccount => "delete_account", Command::Admin => "admin" };
    match response(&db, id, action).await {
        Ok(body) => { bot.send_message(msg.chat.id, body).reply_markup(menu()).await?; },
        Err(e) => tracing::error!(error = %e, "request failed"),
    }
    Ok(())
}

async fn on_callback(bot: Bot, q: CallbackQuery, db: SqlitePool) -> ResponseResult<()> {
    bot.answer_callback_query(q.id.clone()).await?;
    let Some(message) = q.message else { return Ok(()); };
    let id = match i64::try_from(q.from.id.0) { Ok(id) => id, Err(_) => return Ok(()) };
    if register(&db, id).await.is_err() { return Ok(()); }
    let action = q.data.as_deref().unwrap_or("");
    if let Ok(body) = response(&db, id, action).await {
        bot.send_message(message.chat().id, body).reply_markup(menu()).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").without_time().init();
    let token = std::env::var("TELEGRAM_BOT_TOKEN").context("TELEGRAM_BOT_TOKEN must be set")?;
    anyhow::ensure!(!token.trim().is_empty(), "TELEGRAM_BOT_TOKEN cannot be empty");
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://redstone.db?mode=rwc".into());
    let db = SqlitePoolOptions::new().max_connections(1).connect(&database_url).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS users (telegram_id INTEGER PRIMARY KEY, blocked INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)").execute(&db).await?;
    let bot = Bot::new(token);
    let handler = dptree::entry()
        .branch(Update::filter_message().filter_command::<Command>().endpoint(on_message))
        .branch(Update::filter_callback_query().endpoint(on_callback));
    Dispatcher::builder(bot, handler).dependencies(dptree::deps![db]).enable_ctrlc_handler().build().dispatch().await;
    Ok(())
}
