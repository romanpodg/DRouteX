-- References only. P1-07 must protect verifier material before any bytes are stored.
ALTER TABLE credential_metadata ADD COLUMN verifier_ref BLOB
    CHECK(verifier_ref IS NULL OR length(verifier_ref) = 16);
