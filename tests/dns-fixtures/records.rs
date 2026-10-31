// Hand-written wire values, independent of Hickory's serializer.
pub struct RecordCase {
    pub code: u16,
    pub data: &'static [u8],
    pub expected: &'static str,
}

pub fn record_cases() -> [RecordCase; 11] {
    [
        RecordCase { code: 1, data: b"\xcb\x00\x71\x09", expected: "203.0.113.9" },
        RecordCase { code: 28, data: b"\x20\x01\x0d\xb8\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x09", expected: "2001:db8::9" },
        RecordCase { code: 5, data: b"\x06target\x04test\x00", expected: "target.test." },
        RecordCase { code: 16, data: b"\x03abc", expected: "abc" },
        RecordCase { code: 2, data: b"\x02ns\x04test\x00", expected: "ns.test." },
        RecordCase { code: 12, data: b"\x06target\x04test\x00", expected: "target.test." },
        RecordCase { code: 15, data: b"\x00\x0a\x04mail\x04test\x00", expected: "10 mail.test." },
        RecordCase { code: 33, data: b"\x00\x01\x00\x02\x01\xbb\x06target\x04test\x00", expected: "1 2 443 target.test." },
        RecordCase { code: 6, data: b"\x02ns\x04test\x00\x0ahostmaster\x04test\x00\x00\x00\x00\x01\x00\x00\x0e\x10\x00\x00\x02\x58\x00\x01\x51\x80\x00\x00\x00\x3c", expected: "ns.test. hostmaster.test. 1 3600 600 86400 60" },
        RecordCase { code: 64, data: b"\x00\x01\x00", expected: "1 ." },
        RecordCase { code: 65, data: b"\x00\x01\x00", expected: "1 ." },
    ]
}
