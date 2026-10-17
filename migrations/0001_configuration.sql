CREATE TABLE configuration_revisions (
    id BLOB NOT NULL PRIMARY KEY CHECK(length(id) = 16),
    format INTEGER NOT NULL CHECK(format IN (0, 1, 2)),
    document BLOB NOT NULL CHECK(length(document) BETWEEN 1 AND 1048576)
) STRICT;

CREATE TABLE catalog_revisions (
    kind TEXT NOT NULL CHECK(kind IN ('filter', 'service')),
    id BLOB NOT NULL CHECK(length(id) = 16),
    sha256 BLOB NOT NULL CHECK(length(sha256) = 32),
    PRIMARY KEY(kind, id)
) STRICT;

CREATE TABLE configuration_catalog_refs (
    revision BLOB NOT NULL REFERENCES configuration_revisions(id),
    kind TEXT NOT NULL,
    catalog_id BLOB NOT NULL,
    PRIMARY KEY(revision, kind, catalog_id),
    FOREIGN KEY(kind, catalog_id) REFERENCES catalog_revisions(kind, id)
) STRICT;

CREATE TABLE configuration_profiles (
    revision BLOB NOT NULL REFERENCES configuration_revisions(id),
    id BLOB NOT NULL CHECK(length(id) = 16),
    PRIMARY KEY(revision, id)
) STRICT;

CREATE TABLE endpoint_assignments (
    revision BLOB NOT NULL,
    id BLOB NOT NULL CHECK(length(id) = 16),
    profile BLOB NOT NULL,
    enabled INTEGER NOT NULL CHECK(enabled IN (0, 1)),
    device BLOB CHECK(device IS NULL OR length(device) = 16),
    PRIMARY KEY(revision, id),
    FOREIGN KEY(revision, profile) REFERENCES configuration_profiles(revision, id)
) STRICT;

CREATE TABLE credential_metadata (
    revision BLOB NOT NULL,
    id BLOB NOT NULL CHECK(length(id) = 16),
    endpoint BLOB NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('active', 'disabled', 'expired', 'revoked')),
    PRIMARY KEY(revision, id),
    FOREIGN KEY(revision, endpoint) REFERENCES endpoint_assignments(revision, id)
) STRICT;

CREATE TABLE configuration_listeners (
    revision BLOB NOT NULL REFERENCES configuration_revisions(id),
    id BLOB NOT NULL CHECK(length(id) = 16),
    PRIMARY KEY(revision, id)
) STRICT;

CREATE TABLE configuration_network_groups (
    revision BLOB NOT NULL,
    id BLOB NOT NULL CHECK(length(id) = 16),
    traffic_profile BLOB NOT NULL,
    PRIMARY KEY(revision, id),
    FOREIGN KEY(revision, traffic_profile) REFERENCES configuration_profiles(revision, id)
) STRICT;

CREATE TABLE legacy_dns_grants (
    revision BLOB NOT NULL,
    id BLOB NOT NULL CHECK(length(id) = 16),
    source TEXT NOT NULL,
    listener BLOB NOT NULL,
    network_group BLOB NOT NULL,
    endpoint BLOB NOT NULL,
    PRIMARY KEY(revision, id),
    FOREIGN KEY(revision, listener) REFERENCES configuration_listeners(revision, id),
    FOREIGN KEY(revision, network_group) REFERENCES configuration_network_groups(revision, id),
    FOREIGN KEY(revision, endpoint) REFERENCES endpoint_assignments(revision, id)
) STRICT;

CREATE TABLE gateway_network_grants (
    revision BLOB NOT NULL,
    id BLOB NOT NULL CHECK(length(id) = 16),
    source TEXT NOT NULL,
    listener BLOB NOT NULL,
    network_group BLOB NOT NULL,
    PRIMARY KEY(revision, id),
    FOREIGN KEY(revision, listener) REFERENCES configuration_listeners(revision, id),
    FOREIGN KEY(revision, network_group) REFERENCES configuration_network_groups(revision, id)
) STRICT;
