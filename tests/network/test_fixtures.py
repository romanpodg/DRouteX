"""Portable protocol-boundary regression checks; no namespace privileges."""
import struct
import unittest
from types import SimpleNamespace
from unittest.mock import patch

from fixtures import answer, decoded_answer, query, question
import run as lab


class DnsFixtureTests(unittest.TestCase):
    def test_real_a_and_aaaa_frames_keep_transaction_and_question(self):
        for qtype, peer in ((1, '198.18.0.1'), (28, 'fd42:726f:7574::1')):
            packet = query('peer.test', qtype)
            self.assertEqual(decoded_answer(answer(packet, 'a', peer), packet), peer)

    def test_truncation_compression_extra_data_and_unsupported_type_are_rejected(self):
        valid = query('profile.test', 1)
        invalid = [b'', valid[:12], valid[:-1], valid + b'X',
                   valid[:12] + b'\xc0\x0c' + valid[-4:],
                   valid[:-4] + struct.pack('!2H', 255, 1), b'X' * 513]
        for packet in invalid:
            with self.subTest(size=len(packet)), self.assertRaises(ValueError):
                answer(packet, 'a', '198.18.0.1')

    def test_wrong_transaction_or_frame_is_not_a_success(self):
        packet = query('profile.test', 1)
        reply = answer(packet, 'a', '198.18.0.1')
        for bad in (reply[:-1], b'XX' + reply[2:], reply + b'X'):
            with self.assertRaises(ValueError):
                decoded_answer(bad, packet)
        self.assertEqual(question(packet)[0], 'profile.test')


class LabBoundaryTests(unittest.TestCase):
    def test_host_execution_is_rejected_before_any_mutation(self):
        with patch.object(lab.os, 'getpid', return_value=100), patch.object(lab, 'command') as command:
            with self.assertRaisesRegex(RuntimeError, 'missing outer namespace isolation'):
                lab.inside(SimpleNamespace())
            command.assert_not_called()


if __name__ == '__main__':
    unittest.main()
