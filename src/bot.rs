use std::sync::Arc;

use color_eyre::{eyre::Report, Result};

use teloxide::{
    dispatching::{UpdateFilterExt, UpdateHandler},
    prelude::Dispatcher,
    requests::Requester,
    respond,
    types::{
        CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, InlineQuery, InlineQueryResult,
        InlineQueryResultArticle, InputMessageContent, InputMessageContentText, Message, Update,
    },
    Bot,
};

use teloxide::prelude::*;

use crate::{callback_data::CallbackData, concurrent_hash_map::ConcurrentHashMap, game::Game};

async fn schema() -> UpdateHandler<Report> {
    dptree::entry()
        .branch(Update::filter_inline_query().endpoint(inline_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler))
}

async fn inline_handler(bot: Bot, q: InlineQuery) -> Result<(), Report> {
    let button = InlineKeyboardButton::callback("Create a game", CallbackData::Join.to_string());

    let article = InlineQueryResultArticle::new(
        "play".to_string(),
        "Play",
        InputMessageContent::Text(InputMessageContentText::new(
            "TikTakToe\n\nPress a button to create a game",
        )),
    )
    .reply_markup(InlineKeyboardMarkup::new(vec![vec![button]]));

    let results = vec![InlineQueryResult::Article(article)];

    bot.answer_inline_query(&q.id, results).await?;

    respond(())?;

    Ok(())
}

pub async fn update_message(bot: Bot, q: CallbackQuery, game: &Game) -> Result<(), Report> {
    let (text, keyboard) = game.as_message();

    bot.answer_callback_query(&q.id).await?;

    if let Some(Message { id, chat, .. }) = q.message {
        bot.edit_message_text(chat.id, id, text)
            .reply_markup(keyboard)
            .await?;
    } else if let Some(id) = q.inline_message_id {
        bot.edit_message_text_inline(id, text)
            .reply_markup(keyboard)
            .await?;
    }

    Ok(())
}

pub async fn callback_handler(
    bot: Bot,
    q: CallbackQuery,
    storage: Arc<ConcurrentHashMap<String, Game>>,
) -> Result<(), Report> {
    let state = storage
        .get_or_default(q.inline_message_id.as_ref().unwrap())
        .await;

    let mut lock = state.lock().await;

    match lock.process_callback(q.clone()) {
        Ok(_) => update_message(bot, q, &lock).await?,
        Err(error) => {
            bot.answer_callback_query(q.id)
                .text(error.to_string())
                .show_alert(true)
                .await?;
        }
    };

    Ok(())
}

pub async fn bot_main() -> Result<()> {
    log::info!("Starting bot...");

    let bot = Bot::from_env();

    let state_storage = Arc::new(ConcurrentHashMap::<String, Game>::new());

    Dispatcher::builder(bot, schema().await)
        .enable_ctrlc_handler()
        .dependencies(dptree::deps![state_storage])
        .build()
        .dispatch()
        .await;

    Ok(())
}
