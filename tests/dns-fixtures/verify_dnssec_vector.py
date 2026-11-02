"""Independently check the static DNSSEC fixture with the OpenSSL CLI.

This is a fixture check, not a DNSSEC chain validator or a network client.
"""
import json
from pathlib import Path
import struct
import subprocess
import tempfile


def main():
    vector = json.loads(Path(__file__).with_name("dnssec-ed25519.json").read_text())
    public = bytes.fromhex(vector["public_key"])
    signature = bytes.fromhex(vector["rrsig_rdata"])
    name = b"\x07example\x04test\x00"
    dnskey = b"\x01\x01\x03\x0f" + public
    checksum = sum(byte if index & 1 else byte << 8 for index, byte in enumerate(dnskey))
    checksum += (checksum >> 16) & 0xFFFF
    fields = struct.unpack("!HBBIIIH", signature[:18])
    assert fields == (1, 15, 2, 60, 2145916800, 1577836800, checksum & 0xFFFF)
    assert signature[18:32] == name and len(signature[32:]) == 64
    record = name + struct.pack("!HHIH", 1, 1, 60, 4) + bytes([203, 0, 113, 9])
    signed = signature[:32] + record
    with tempfile.TemporaryDirectory() as directory:
        directory = Path(directory)
        (directory / "key.der").write_bytes(bytes.fromhex("302a300506032b6570032100") + public)
        (directory / "sig").write_bytes(signature[32:])
        (directory / "data").write_bytes(signed)
        command = ["openssl", "pkeyutl", "-verify", "-pubin", "-keyform", "DER",
                   "-inkey", str(directory / "key.der"), "-rawin", "-in", str(directory / "data"),
                   "-sigfile", str(directory / "sig")]
        subprocess.run(command, check=True, capture_output=True)
        (directory / "data").write_bytes(signed[:-1] + bytes([signed[-1] ^ 1]))
        assert subprocess.run(command, capture_output=True).returncode != 0
    print("Ed25519 RRSIG verifies; modified RRset is rejected")


if __name__ == "__main__":
    main()
