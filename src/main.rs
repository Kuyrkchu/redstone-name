use anyhow::{Context, Result};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;
use teloxide::{
    prelude::*,
    types::{ChatKind, InlineKeyboardButton, InlineKeyboardMarkup, InputFile, MessageKind},
    utils::command::BotCommands,
};

fn admin_telegram_id() -> u64 {
    std::env::var("ADMIN_TELEGRAM_ID")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}

const SINGBOX_PROFILE: &str = "/root/redstone-bot/redstone-bot/assets/redstone-sing-box.json";
const WIREGUARD_PROFILE: &str = "/root/redstone-bot/redstone-bot/assets/redstone-phone.conf";
const SINGBOX_GUIDE: &str = "/root/redstone-bot/redstone-bot/assets/instructions-sing-box.png";
const WIREGUARD_GUIDE: &str = "/root/redstone-bot/redstone-bot/assets/instructions-wireguard.png";
const SEND_SINGBOX: &str = "__send_singbox_profile__";
const SEND_WIREGUARD: &str = "__send_wireguard_profile__";

#[derive(BotCommands, Clone)]
#[command(rename_rule = "snake_case", description = "Команды Redstouns VPN")]
enum Command {
    #[command(description = "Главное меню")]
    Start,
    #[command(description = "Инструкция с картинкой")]
    Help,
    #[command(description = "Инструкция с картинкой")]
    Guide,
    #[command(description = "Инструкция WireGuard с картинкой")]
    WireguardHelp,
    #[command(description = "Проверка подключения")]
    Status,
    #[command(description = "Информация о сервере")]
    Servers,
    #[command(description = "Тарифы")]
    Plans,
    #[command(description = "Профиль sing-box")]
    Config,
    #[command(description = "Профиль WireGuard")]
    Wireguard,
    #[command(description = "Поддержка")]
    Support,
    #[command(description = "Удалить аккаунт бота")]
    DeleteAccount,
    #[command(description = "Панель администратора")]
    Admin,
}

fn menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("📥 Профиль sing-box", "config")],
        vec![InlineKeyboardButton::callback("📥 Профиль WireGuard", "wireguard_config")],
        vec![InlineKeyboardButton::callback("📘 Инструкция sing-box", "help")],
        vec![InlineKeyboardButton::callback("📘 Инструкция WireGuard", "wireguard_guide")],
        vec![InlineKeyboardButton::callback("📊 Проверка", "status")],
        vec![
            InlineKeyboardButton::callback("🌍 Сервер", "servers"),
            InlineKeyboardButton::callback("💬 Поддержка", "support"),
        ],
    ])
}

async fn register(db: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO users (telegram_id) VALUES (?)")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

async fn response(db: &SqlitePool, id: i64, action: &str) -> Result<String> {
    let blocked: i64 = sqlx::query_scalar("SELECT blocked FROM users WHERE telegram_id = ?")
        .bind(id)
        .fetch_one(db)
        .await?;
    if blocked != 0 && action != "delete_account" {
        return Ok("Доступ к аккаунту приостановлен. Обратитесь в поддержку.".into());
    }

    Ok(match action {
        "start" => "⚡ REDSTOUNS VPN\n━━━━━━━━━━━━━━━━\nОбычный защищённый VPN-туннель на WireGuard.\n\n📲 Выбери приложение и следуй короткой инструкции.\n🔐 Не пересылай конфигурацию: в ней личный ключ.\n\nВыберите действие:".into(),
        "help" | "guide" => "📲 SING-BOX EXTENDED · ANDROID\n\n1. Скачай профиль sing-box в этом боте.\n2. Добавь его как локальную конфигурацию.\n3. Запусти профиль и подтверди запрос Android.\n4. Проверь значок VPN и счётчики трафика.\n\nЕсли используешь приложение WireGuard, открой отдельную инструкцию. Не запускай два VPN одновременно. Не пересылай конфигурацию: в ней личный ключ.".into(),
        "wireguard_guide" => "📲 WIREGUARD · ANDROID\n\n1. Скачай профиль WireGuard в этом боте.\n2. Открой WireGuard, нажми «+» и импортируй файл.\n3. Включи туннель и подтверди запрос Android.\n4. Проверь значок VPN и счётчики трафика.\n\nНе запускай его одновременно с sing-box. Не пересылай файл: в нём личный ключ.".into(),
        "status" => "📊 ПРОВЕРКА ПОДКЛЮЧЕНИЯ\n\nБот не может удалённо увидеть состояние телефона. Проверь системный значок VPN и счётчики отправленных/полученных данных в приложении. Если трафик не идёт, выключи другие VPN и напиши в поддержку текст ошибки. Не присылай файл конфигурации.".into(),
        "servers" => "🌍 СЕРВЕР\n\n🇳🇱 Нидерланды\n🔐 WireGuard\n📡 UDP · IPv4 и IPv6\n\nДоступен профиль для sing-box Extended и запасной файл для приложения WireGuard.".into(),
        "plans" => "💎 ТАРИФЫ\n\nОплата и подписки в боте пока не подключены. Бот не принимает платежи и не обещает платный доступ.".into(),
        "config" if id as u64 == admin_telegram_id() => SEND_SINGBOX.into(),
        "wireguard_config" if id as u64 == admin_telegram_id() => SEND_WIREGUARD.into(),
        "config" | "wireguard_config" => "🔐 Выдача профилей доступна только владельцу сервера. Обратитесь к администратору.".into(),
        "support" => "💬 ПОДДЕРЖКА\n\nНапишите администратору: @daniilkuyrkchu\n\nНе отправляйте пароль VPS, токены бота, файлы профилей или приватные ключи.".into(),
        "delete_account" => {
            sqlx::query("DELETE FROM users WHERE telegram_id = ?")
                .bind(id)
                .execute(db)
                .await?;
            "Аккаунт бота удалён. Отзыв VPN-профиля выполняется отдельно на сервере.".into()
        }
        "admin" if id as u64 == admin_telegram_id() => {
            "🛡 ПАНЕЛЬ ВЛАДЕЛЬЦА\n\nПрофили выдаются только через кнопки и только в личном чате.".into()
        }
        "admin" => "Нет доступа к панели администратора.".into(),
        _ => "Неизвестная команда. Откройте /start.".into(),
    })
}

async fn send_profile(
    bot: &Bot,
    chat_id: ChatId,
    action: &str,
) -> ResponseResult<()> {
    let (path, caption) = if action == "wireguard_config" {
        (
            WIREGUARD_PROFILE,
            "🔐 Личный профиль WireGuard\n\nИмпортируй файл в приложение WireGuard. Не пересылай его и не устанавливай на нескольких устройствах.",
        )
    } else {
        (
            SINGBOX_PROFILE,
            "🔐 Личный профиль sing-box Extended\n\nДобавь JSON как локальную конфигурацию. Не пересылай файл: в нём личный ключ.",
        )
    };
    if !Path::new(path).is_file() {
        bot.send_message(chat_id, "Профиль временно недоступен. Попробуйте позже.")
            .reply_markup(menu())
            .await?;
        return Ok(());
    }
    bot.send_document(chat_id, InputFile::file(path))
        .caption(caption)
        .reply_markup(menu())
        .await?;
    Ok(())
}

async fn present(
    bot: &Bot,
    chat_id: ChatId,
    user_id: i64,
    db: &SqlitePool,
    action: &str,
) -> ResponseResult<()> {
    match response(db, user_id, action).await {
        Ok(body) if body == SEND_SINGBOX => send_profile(bot, chat_id, "config").await?,
        Ok(body) if body == SEND_WIREGUARD => send_profile(bot, chat_id, "wireguard_config").await?,
        Ok(body) if action == "help" || action == "guide" || action == "wireguard_guide" => {
            let guide = if action == "wireguard_guide" { WIREGUARD_GUIDE } else { SINGBOX_GUIDE };
            if Path::new(guide).is_file() {
                bot.send_photo(chat_id, InputFile::file(guide))
                    .caption(body)
                    .reply_markup(menu())
                    .await?;
            } else {
                bot.send_message(chat_id, body).reply_markup(menu()).await?;
            }
        }
        Ok(body) => {
            bot.send_message(chat_id, body).reply_markup(menu()).await?;
        }
        Err(error) => tracing::error!(error = %error, "request failed"),
    }
    Ok(())
}

async fn on_message(bot: Bot, msg: Message, cmd: Command, db: SqlitePool) -> ResponseResult<()> {
    if !matches!(msg.kind, MessageKind::Common(_)) || !matches!(msg.chat.kind, ChatKind::Private(_)) {
        return Ok(());
    }
    let Some(user) = msg.from.as_ref() else { return Ok(()); };
    let Ok(id) = i64::try_from(user.id.0) else { return Ok(()); };
    if let Err(error) = register(&db, id).await {
        tracing::error!(error = %error, "database error");
        return Ok(());
    }
    let action = match cmd {
        Command::Start => "start",
        Command::Help => "help",
        Command::Guide => "guide",
        Command::WireguardHelp => "wireguard_guide",
        Command::Status => "status",
        Command::Servers => "servers",
        Command::Plans => "plans",
        Command::Config => "config",
        Command::Wireguard => "wireguard_config",
        Command::Support => "support",
        Command::DeleteAccount => "delete_account",
        Command::Admin => "admin",
    };
    present(&bot, msg.chat.id, id, &db, action).await
}

async fn on_callback(bot: Bot, q: CallbackQuery, db: SqlitePool) -> ResponseResult<()> {
    bot.answer_callback_query(q.id.clone()).await?;
    let Some(message) = q.message else { return Ok(()); };
    if !matches!(message.chat().kind, ChatKind::Private(_)) {
        return Ok(());
    }
    let Ok(id) = i64::try_from(q.from.id.0) else { return Ok(()); };
    if let Err(error) = register(&db, id).await {
        tracing::error!(error = %error, "database error");
        return Ok(());
    }
    let action = q.data.as_deref().unwrap_or("");
    present(&bot, message.chat().id, id, &db, action).await
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .without_time()
        .init();
    let token = std::env::var("TELEGRAM_BOT_TOKEN").context("TELEGRAM_BOT_TOKEN must be set")?;
    anyhow::ensure!(!token.trim().is_empty(), "TELEGRAM_BOT_TOKEN cannot be empty");
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://redstone.db?mode=rwc".into());
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS users (telegram_id INTEGER PRIMARY KEY, blocked INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)")
        .execute(&db)
        .await?;
    let bot = Bot::new(token);
    let handler = dptree::entry()
        .branch(Update::filter_message().filter_command::<Command>().endpoint(on_message))
        .branch(Update::filter_callback_query().endpoint(on_callback));
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}
