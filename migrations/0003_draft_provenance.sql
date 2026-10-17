CREATE TABLE configuration_provenance (
    revision BLOB NOT NULL PRIMARY KEY REFERENCES configuration_revisions(id),
    parent BLOB UNIQUE REFERENCES configuration_provenance(revision),
    author BLOB NOT NULL CHECK(length(author) = 16),
    unix_millis INTEGER NOT NULL CHECK(unix_millis >= 0),
    correlation BLOB NOT NULL CHECK(length(correlation) = 16),
    CHECK(parent IS NULL OR parent != revision)
) STRICT;

CREATE TABLE draft_head (
    singleton INTEGER NOT NULL PRIMARY KEY CHECK(singleton = 1),
    revision BLOB NOT NULL REFERENCES configuration_provenance(revision)
) STRICT;
