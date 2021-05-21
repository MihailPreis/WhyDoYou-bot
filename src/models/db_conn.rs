use crate::models::content_model::ContentModel;
use crate::models::error::HandlerError;
use lazy_static::lazy_static;

#[cfg(feature = "db")]
use {
    crate::utils::string_utils::normalize_words,
    rand::seq::SliceRandom,
    sqlx::migrate::MigrateDatabase,
    sqlx::sqlite::SqliteConnectOptions,
    sqlx::sqlite::SqlitePoolOptions,
    sqlx::ConnectOptions,
    sqlx::{Pool, Sqlite},
    std::collections::HashSet,
    std::str::FromStr,
    std::time::Duration,
};

lazy_static! {
    static ref DB_URL: String =
        std::env::var("DATABASE_URL").unwrap_or(String::from("sqlite:data.db"));
}

/// Database connection wrapper
/// TODO: Expand this for another bot implementation or divide implementations
pub struct DBConn {
    #[cfg(feature = "db")]
    pool: Pool<Sqlite>,
}

#[cfg(not(feature = "db"))]
pub async fn setup_db() -> Result<(), HandlerError> {
    Ok(())
}

#[cfg(feature = "db")]
pub async fn setup_db() -> Result<(), HandlerError> {
    if !Sqlite::database_exists(&**DB_URL).await? {
        Sqlite::create_database(&**DB_URL).await?;
    }
    DBConn::new().await?.migrate().await?;
    Ok(())
}

#[cfg(feature = "db")]
impl DBConn {
    pub async fn new() -> Result<Self, HandlerError> {
        let mut connection_options = SqliteConnectOptions::from_str(&**DB_URL)?;
        connection_options
            .log_statements(log::LevelFilter::Debug)
            .log_slow_statements(log::LevelFilter::Warn, Duration::from_secs(1));
        let pool: Pool<Sqlite> = SqlitePoolOptions::new()
            .connect_with(connection_options)
            .await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), HandlerError> {
        sqlx::migrate!().run(&self.pool).await?;
        Ok(())
    }

    pub async fn get_words(&self, chat_id: i64) -> Result<String, HandlerError> {
        struct PrivateWords {
            words: String,
        }
        let items: Vec<PrivateWords> = sqlx::query_as!(
            PrivateWords,
            "SELECT words FROM contents WHERE chat_id = ?",
            chat_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(items
            .iter()
            .map(|i| i.words.clone())
            .collect::<HashSet<String>>()
            .into_iter()
            .collect::<Vec<String>>()
            .join(","))
    }

    pub async fn get_random_content(
        &self,
        chat_id: i64,
        is_image: bool,
        words: Vec<String>,
    ) -> Result<ContentModel, HandlerError> {
        let mut buff: Vec<ContentModel> = Vec::new();
        for word in words {
            let regex_word: String = format!("%{}%", word);
            let query = sqlx::query_as!(
                ContentModel,
                "SELECT * FROM contents WHERE chat_id == ? AND is_image == ? AND words LIKE ? ORDER BY RANDOM() LIMIT 1",
                chat_id,
                is_image,
                regex_word
            );
            if let Ok(item) = query.fetch_one(&self.pool).await {
                buff.push(item);
            }
        }
        buff.choose(&mut rand::thread_rng())
            .and_then(|i| Some(i.clone()))
            .ok_or(HandlerError::empty())
    }

    pub async fn get_all_contents(
        &self,
        chat_id: i64,
        is_image: bool,
    ) -> Result<Vec<ContentModel>, HandlerError> {
        Ok(sqlx::query_as!(
            ContentModel,
            "SELECT * FROM contents WHERE chat_id == ? AND is_image == ?",
            chat_id,
            is_image
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn add_content(&self, item: ContentModel) -> Result<(), HandlerError> {
        sqlx::query!(
            "INSERT INTO contents (chat_id, is_image, name, words, data) VALUES (?, ?, ?, ?, ?)",
            item.chat_id,
            item.is_image,
            item.name,
            item.words,
            item.data
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn rm_content(
        &self,
        chat_id: i64,
        is_image: bool,
        name: String,
    ) -> Result<(), HandlerError> {
        sqlx::query!(
            "DELETE FROM contents WHERE (chat_id, is_image, name) IN
            (SELECT chat_id, is_image, name FROM contents WHERE chat_id == ? AND is_image == ? AND name == ?)",
            chat_id,
            is_image,
            name
        )
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn change_words(
        &self,
        chat_id: i64,
        is_image: bool,
        name: String,
        new_words: String,
    ) -> Result<(), HandlerError> {
        let words = normalize_words(new_words);
        sqlx::query!(
            "UPDATE contents SET words = ? WHERE chat_id = ? AND is_image = ? AND name = ?",
            words,
            chat_id,
            is_image,
            name
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(not(feature = "db"))]
impl DBConn {
    fn create_error() -> HandlerError {
        HandlerError::from_str("DB feature not enabled")
    }

    pub async fn new() -> Result<Self, HandlerError> {
        Err(DBConn::create_error())
    }

    pub async fn get_words(&self, _chat_id: i64) -> Result<String, HandlerError> {
        Err(DBConn::create_error())
    }

    pub async fn get_random_content(
        &self,
        _chat_id: i64,
        _is_image: bool,
        _words: Vec<String>,
    ) -> Result<ContentModel, HandlerError> {
        Err(DBConn::create_error())
    }
}
