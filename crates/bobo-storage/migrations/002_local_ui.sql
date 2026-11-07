-- Local-only state. Does not introduce remote identities or change message ownership.
CREATE TABLE channel_drafts (
    channel_id TEXT PRIMARY KEY NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    body TEXT NOT NULL CHECK(length(body) <= 65536)
);
CREATE TABLE local_navigation (
    singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton = 1),
    space_id TEXT REFERENCES spaces(id) ON DELETE SET NULL,
    channel_id TEXT REFERENCES channels(id) ON DELETE SET NULL
);
INSERT INTO local_navigation(singleton) VALUES (1);
-- messages_by_channel(channel_id, created_at, id) from 001 already supports
-- reverse keyset scans. No redundant index or modification of 001 is needed.
