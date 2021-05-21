CREATE TABLE IF NOT EXISTS contents
(
    id          INTEGER PRIMARY KEY NOT NULL,
    chat_id     INTEGER             NOT NULL,
    is_image    BOOLEAN             NOT NULL,
    name        TEXT                NOT NULL,
    words       TEXT                NOT NULL,
    data        BLOB                NOT NULL
);