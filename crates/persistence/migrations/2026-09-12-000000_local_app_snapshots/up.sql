CREATE TABLE local_app_snapshots (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    snapshot TEXT NOT NULL
);
