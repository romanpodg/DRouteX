"""Independent dnspython client for the opt-in controlled Rust DNS fixture."""
import json
import sys
from pathlib import Path

import dns.dnssec
import dns.flags
import dns.message
import dns.query
import dns.rcode
import dns.rdata
import dns.rdataclass
import dns.rdatatype
import dns.rrset


def chain(address, port):
    vector = json.loads(Path(__file__).with_name("dnssec-chain.json").read_text())

    def query(name, kind):
        request = dns.message.make_query(name, kind, want_dnssec=True)
        request.flags |= dns.flags.CD
        response = dns.query.tcp(request, address, port=int(port), timeout=3)
        assert response.rcode() == dns.rcode.NOERROR and not response.flags & dns.flags.AD
        kind = dns.rdatatype.from_text(kind)
        rrset = response.find_rrset(response.answer, name, dns.rdataclass.IN, kind)
        signatures = response.find_rrset(response.answer, name, dns.rdataclass.IN,
                                         dns.rdatatype.RRSIG, covers=kind)
        return rrset, signatures

    root, root_signatures = query(".", "DNSKEY")
    assert len(root) == 1 and root[0].to_text() == vector["root_dnskey"]
    parent_name = dns.name.root
    parent_keys = {parent_name: root}
    dns.dnssec.validate(root, root_signatures, parent_keys)
    for name in ["test.", "example.test."]:
        ds, ds_signatures = query(name, "DS")
        dns.dnssec.validate(ds, ds_signatures, parent_keys)
        wire = ds[0].to_wire()
        altered = wire[:-1] + bytes([wire[-1] ^ 1])
        bad_ds = dns.rdata.from_wire(dns.rdataclass.IN, dns.rdatatype.DS, altered, 0, len(altered))
        tampered_ds = dns.rrset.from_rdata(name, 60, bad_ds)
        try:
            dns.dnssec.validate(tampered_ds, ds_signatures, parent_keys)
        except dns.dnssec.ValidationFailure:
            pass
        else:
            raise AssertionError("modified delegation unexpectedly validated")
        keys, signatures = query(name, "DNSKEY")
        assert len(keys) == 1 and dns.dnssec.make_ds(name, keys[0], "SHA256") in ds
        parent_name = dns.name.from_text(name)
        parent_keys = {parent_name: keys}
        dns.dnssec.validate(keys, signatures, parent_keys)
    answer, signatures = query("example.test.", "A")
    assert answer[0].address == "203.0.113.9"
    dns.dnssec.validate(answer, signatures, parent_keys)


def main():
    address, port, mode = sys.argv[1:]
    if mode == "chain":
        chain(address, port)
        return
    vector = json.loads(Path(__file__).with_name("dnssec-ed25519.json").read_text())
    request = dns.message.make_query("example.test.", "A", want_dnssec=True)
    request.flags |= dns.flags.CD
    answer = dns.query.tcp(request, address, port=int(port), timeout=3)
    assert answer.rcode() == dns.rcode.NOERROR
    assert not answer.flags & dns.flags.AD
    rrset = answer.find_rrset(answer.answer, "example.test.", dns.rdataclass.IN, dns.rdatatype.A)
    if mode == "override":
        assert rrset[0].address == "192.0.2.1"
        assert not any(rr.rdtype == dns.rdatatype.RRSIG for rr in answer.answer)
        return
    assert mode == "signed" and rrset[0].address == "203.0.113.9"
    signatures = answer.find_rrset(answer.answer, "example.test.", dns.rdataclass.IN,
                                   dns.rdatatype.RRSIG, covers=dns.rdatatype.A)
    key_wire = b"\x01\x01\x03\x0f" + bytes.fromhex(vector["public_key"])
    key = dns.rdata.from_wire(dns.rdataclass.IN, dns.rdatatype.DNSKEY, key_wire, 0, len(key_wire))
    keys = {dns.name.from_text("example.test."): dns.rrset.from_rdata("example.test.", 60, key)}
    dns.dnssec.validate(rrset, signatures, keys)
    tampered = dns.rrset.from_text("example.test.", 60, "IN", "A", "203.0.113.10")
    try:
        dns.dnssec.validate(tampered, signatures, keys)
    except dns.dnssec.ValidationFailure:
        return
    raise AssertionError("modified RRset unexpectedly validated")


if __name__ == "__main__":
    main()
