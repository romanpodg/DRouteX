-- Encrypted envelopes only; master keys and plaintext never enter this database.
-- No foreign key to a configuration revision: retained revisions/backups may refer
-- to an immutable logical secret. Re-encryption preserves that public reference.
CREATE TABLE encrypted_secrets (
    id BLOB PRIMARY KEY NOT NULL CHECK(typeof(id)='blob' AND length(id)=16),
    envelope BLOB NOT NULL CHECK(typeof(envelope)='blob' AND length(envelope) BETWEEN 75 AND 65610)
) STRICT;
