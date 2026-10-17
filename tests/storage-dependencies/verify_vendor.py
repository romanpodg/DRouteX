"""Verify the reviewed SQLx source snapshot before compiling its one-line patch."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VENDOR = ROOT / 'vendor/sqlx-sqlite'


def fingerprint(path):
    return hashlib.sha256(path.read_bytes().replace(b'\r\n', b'\n')).hexdigest()


def verify():
    proof = json.loads((VENDOR / 'SOURCE.json').read_text(encoding='utf-8'))
    expected = proof['files_SHA256_LF']
    actual = {p.relative_to(VENDOR).as_posix() for p in VENDOR.rglob('*') if p.is_file()}
    assert actual == set(expected) | {'SOURCE.json', 'README.routira.md'}, 'Unexpected source inventory'
    for name, digest in expected.items():
        assert '..' not in Path(name).parts and not Path(name).is_absolute()
        assert fingerprint(VENDOR / name) == digest, f'Reviewed source changed: {name}'
    assert proof['upstream_patch_commit'] == '94aafe3a68884d923b0798a767c8d7f6cfda89d2'
    assert proof['modified_archive_files'] == ['Cargo.toml', 'LICENSE-APACHE', 'LICENSE-MIT']
    assert 'version = ">=0.30.1, <0.39.0"' in (VENDOR / 'Cargo.toml').read_text()
    assert (VENDOR / 'LICENSE-APACHE').stat().st_size > 10_000
    assert (VENDOR / 'LICENSE-MIT').stat().st_size > 1_000
    print(f'Verified {len(expected)} SQLx source files; one manifest line and restored license texts')


if __name__ == '__main__':
    verify()
