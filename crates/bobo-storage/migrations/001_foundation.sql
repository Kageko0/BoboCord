CREATE TABLE spaces (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE CHECK(length(trim(name)) BETWEEN 1 AND 80),
    created_at INTEGER NOT NULL
);
CREATE TABLE channels (
    id TEXT PRIMARY KEY NOT NULL,
    space_id TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL COLLATE NOCASE CHECK(length(trim(name)) BETWEEN 1 AND 80),
    kind TEXT NOT NULL CHECK(kind = 'text'),
    created_at INTEGER NOT NULL,
    UNIQUE(space_id, name)
);
CREATE TABLE local_profile (
    singleton INTEGER NOT NULL PRIMARY KEY CHECK(singleton = 1),
    id TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL CHECK(length(trim(display_name)) BETWEEN 1 AND 80),
    avatar_path TEXT
);
CREATE TABLE messages (
    id TEXT PRIMARY KEY NOT NULL,
    channel_id TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id TEXT NOT NULL REFERENCES local_profile(id),
    body TEXT NOT NULL CHECK(length(trim(body)) BETWEEN 1 AND 4000),
    created_at INTEGER NOT NULL
);
CREATE INDEX messages_by_channel ON messages(channel_id, created_at, id);
CREATE TABLE preferences (
    singleton INTEGER NOT NULL PRIMARY KEY CHECK(singleton = 1),
    theme TEXT NOT NULL CHECK(theme IN ('dark', 'light'))
);
INSERT INTO preferences(singleton, theme) VALUES (1, 'dark');
