use sqlx::migrate::MigrateDatabase;
use sqlx::Sqlite;
use std::collections::HashSet;
use why_do_you_bot::models::content_model::ContentModel;
use why_do_you_bot::models::db_conn::DBConn;

const DB_URL: &str = "sqlite:.test.db";
const CHAT_ID: i64 = -100500;
const FIRST_ITEM_NAME: &str = "content1";
const SECOND_ITEM_NAME: &str = "content2";

const TEST_WORD1: &str = "qqq";
const TEST_WORD2: &str = "www";
const TEST_WORD3: &str = "eee";
const NEW_WORD: &str = "test";

async fn get_db_conn() -> DBConn {
    std::env::set_var("DATABASE_URL", &DB_URL);
    if Sqlite::database_exists(&DB_URL).await.unwrap() {
        Sqlite::drop_database(&DB_URL).await.unwrap()
    }
    Sqlite::create_database(&DB_URL).await.unwrap();
    let conn: DBConn = DBConn::new().await.unwrap();
    conn.migrate().await.unwrap();
    return conn;
}

#[tokio::test(flavor = "multi_thread")]
async fn full_test_db() {
    let conn = get_db_conn().await;

    let items = vec![
        ContentModel::from(
            CHAT_ID,
            true,
            format!("{},{}", TEST_WORD1, TEST_WORD2),
            FIRST_ITEM_NAME.to_string(),
            Vec::new(),
        ),
        ContentModel::from(
            CHAT_ID,
            true,
            format!("{},{}", TEST_WORD2, TEST_WORD3),
            SECOND_ITEM_NAME.to_string(),
            Vec::new(),
        ),
    ];

    for item in items.clone() {
        conn.add_content(item.clone()).await.unwrap();
    }
    assert_eq!(
        conn.get_all_contents(CHAT_ID, true).await.unwrap().len(),
        items.len(),
        "Number of items does not match."
    );

    let mut from_db = conn
        .get_words(CHAT_ID)
        .await
        .unwrap()
        .split(",")
        .collect::<HashSet<&str>>()
        .into_iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>();
    from_db.sort();
    let mut sample = vec![
        TEST_WORD1.to_string(),
        TEST_WORD2.to_string(),
        TEST_WORD3.to_string(),
    ];
    sample.sort();
    assert_eq!(from_db, sample, "Keywords don't match.");

    let model = conn
        .get_random_content(CHAT_ID, true, sample)
        .await
        .unwrap();
    assert!(
        items
            .iter()
            .any(|i| i.words == model.words && i.name == model.name),
        "Randomly obtained item was not found in list of test items."
    );

    conn.rm_content(CHAT_ID, true, String::from(FIRST_ITEM_NAME))
        .await
        .unwrap();
    assert_eq!(
        conn.get_all_contents(CHAT_ID, true).await.unwrap().len(),
        items.len() - 1,
        "New number of items does not match after deleting."
    );

    conn.change_words(
        CHAT_ID,
        true,
        String::from(SECOND_ITEM_NAME),
        NEW_WORD.to_string(),
    )
    .await
    .unwrap();
    assert_eq!(
        conn.get_words(CHAT_ID).await.unwrap(),
        NEW_WORD.to_string(),
        "Edited keywords don't match."
    );
}
