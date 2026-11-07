"""Create a NEW v1 fixture only; never modifies an existing database.
Usage: python scripts/seed_load.py target/f1-load
"""
import json
from pathlib import Path
import sqlite3
import sys
import uuid


def seed(directory):
    directory = Path(directory).resolve()
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / "bobocord.sqlite3"
    if path.exists():
        raise SystemExit(f"Refusing to overwrite existing database: {path}")
    root = Path(__file__).resolve().parents[1]
    author, space = str(uuid.uuid4()), str(uuid.uuid4())
    channels = [str(uuid.uuid4()) for _ in range(4)]
    with sqlite3.connect(path) as db:
        db.executescript((root / "crates/bobo-storage/migrations/001_foundation.sql").read_text(encoding="utf-8"))
        db.execute("INSERT INTO local_profile VALUES(1,?,'Perfil de carga',NULL)", (author,))
        db.execute("INSERT INTO spaces VALUES(?,'Carga F1',1)", (space,))
        for index, channel in enumerate(channels):
            db.execute("INSERT INTO channels VALUES(?,?,?,'text',?)", (channel, space, "carga" if index == 0 else f"quiet-{index}", index))
        for index in range(10_000):
            channel = channels[0] if index < 9000 else channels[1 + index % 3]
            body = f"Mensagem de carga {index:05d}"
            if index % 7 == 0:
                body += "\nSegunda linha para testar altura variável e ancoragem."
            db.execute("INSERT INTO messages VALUES(?,?,?,?,?)", (str(uuid.uuid4()), channel, author, body, 1_700_000_000_000 + index // 100))
        db.execute("PRAGMA user_version=1")
    print(json.dumps({"path": str(path), "messages": 10000, "active_channel_messages": 9000, "channels": 4}))


if __name__ == "__main__":
    seed(sys.argv[1])
