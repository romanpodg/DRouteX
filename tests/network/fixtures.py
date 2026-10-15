"""Bounded P0-03 protocol fixtures, never Routira production implementations."""
import argparse
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import select
import socket
import ssl
import struct
import threading
import time

PORTS = {'dns': 5353, 'dot': 8853, 'doh': 8443, 'origin': 9443,
         'proxy': 9444, 'gateway': 8444, 'gateway_proxy': 8445}
LINKS = {'edge': 0, 'origin': 1, 'proxy': 2, 'a': 10, 'b': 11, 'mismatch': 12}


def address(link, side, family):
    number = LINKS[link]
    value = f'198.18.{number}.{side}' if family == 4 else f'fd42:726f:7574:{number:x}::{side}'
    return str(ipaddress.ip_address(value))


def profile_address(profile, family):
    number = {'a': 10, 'b': 20, 'network': 30}[profile]
    return f'198.18.100.{number}' if family == 4 else f'fd42:726f:7574:100::{number}'


def query(name, qtype):
    labels = name.encode('ascii').split(b'.')
    return struct.pack('!6H', 1234, 256, 1, 0, 0, 0) + b''.join(
        bytes([len(label)]) + label for label in labels) + b'\0' + struct.pack('!2H', qtype, 1)


def question(packet):
    # shortcut: uncompressed single-question A/AAAA fixtures only; P3 owns the real DNS parser.
    if not 17 <= len(packet) <= 512 or struct.unpack('!H', packet[4:6])[0] != 1:
        raise ValueError('invalid fixture question')
    offset, labels = 12, []
    while True:
        if offset >= len(packet):
            raise ValueError('truncated label')
        size = packet[offset]
        offset += 1
        if size == 0:
            break
        if size > 63 or offset + size > len(packet):
            raise ValueError('invalid label')
        labels.append(packet[offset:offset + size].decode('ascii'))
        offset += size
    if offset + 4 > len(packet):
        raise ValueError('truncated question')
    qtype, qclass = struct.unpack('!2H', packet[offset:offset + 4])
    if qclass != 1 or qtype not in (1, 28):
        raise ValueError('unsupported fixture question')
    return '.'.join(labels), qtype, offset + 4


def answer(packet, profile, peer):
    name, qtype, end = question(packet)
    if end != len(packet):
        raise ValueError('extra request bytes')
    family = 4 if qtype == 1 else 6
    if name == 'profile.test':
        value = profile_address(profile, family)
    elif name == 'peer.test' and ipaddress.ip_address(peer).version == family:
        value = peer
    else:
        return packet[:2] + struct.pack('!5H', 0x8183, 1, 0, 0, 0) + packet[12:end]
    data = ipaddress.ip_address(value).packed
    return (packet[:2] + struct.pack('!5H', 0x8180, 1, 1, 0, 0) + packet[12:end]
            + b'\xc0\x0c' + struct.pack('!2HIH', qtype, 1, 1, len(data)) + data)


def decoded_answer(packet, request):
    _, qtype, end = question(request)
    size = 4 if qtype == 1 else 16
    expected_header = request[:2] + struct.pack('!5H', 0x8180, 1, 1, 0, 0)
    if (len(packet) != end + 12 + size or packet[:12] != expected_header
            or packet[12:end] != request[12:end]
            or packet[end:end + 2] != b'\xc0\x0c'
            or struct.unpack('!2HIH', packet[end + 2:end + 12]) != (qtype, 1, 1, size)):
        raise ValueError('unexpected fixture answer')
    return str(ipaddress.ip_address(packet[-size:]))


def read_exact(stream, size):
    if not 0 <= size <= 8192:
        raise ValueError('frame limit')
    data = bytearray()
    while len(data) < size:
        chunk = stream.recv(size - len(data))
        if not chunk:
            raise EOFError('closed fixture stream')
        data.extend(chunk)
    return bytes(data)


def read_headers(stream):
    data = bytearray()
    while not data.endswith(b'\r\n\r\n'):
        data.extend(read_exact(stream, 1))
        if len(data) > 8192:
            raise ValueError('header limit')
    lines = data.decode('ascii').split('\r\n')
    headers = {}
    for line in lines[1:-2]:
        key, value = line.split(':', 1)
        key = key.lower()
        if key in headers:
            raise ValueError('duplicate header')
        headers[key] = value.strip()
    return lines[0], headers


def respond(stream, status, body=b'', content_type='application/json'):
    stream.sendall((f'HTTP/1.1 {status}\r\nContent-Length: {len(body)}\r\n'
                    f'Content-Type: {content_type}\r\nConnection: close\r\n\r\n').encode('ascii') + body)


def client_context(config):
    context = ssl.create_default_context(cafile=config['ca'])
    context.minimum_version = ssl.TLSVersion.TLSv1_2
    return context


def server_context(config, role):
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.minimum_version = ssl.TLSVersion.TLSv1_2
    context.load_cert_chain(**config['tls'][role])
    if role == 'resolver':
        def select_endpoint(stream, hostname, _context):
            stream.fixture_profile = config['tokens'].get((hostname or '').removesuffix('.resolver.test'))
        context.set_servername_callback(select_endpoint)
    return context


def relay(left, right):
    readers = [left, right]
    deadline = time.monotonic() + 10
    while readers and time.monotonic() < deadline:
        ready = [s for s in readers if isinstance(s, ssl.SSLSocket) and s.pending()]
        if not ready:
            ready, _, _ = select.select(readers, [], [], 0.1)
        for stream in ready:
            other = right if stream is left else left
            data = stream.recv(8192)
            if data:
                other.sendall(data)
            else:
                readers.remove(stream)
                other.shutdown(socket.SHUT_WR)


def handle(stream, peer, role, config, context):
    family = ipaddress.ip_address(peer).version
    if role.startswith('gateway') or role == 'dns':
        if peer != address('edge', 1, family):
            return
    if context:
        stream = context.wrap_socket(stream, server_side=True)
    with stream:
        if role in ('dns', 'dot'):
            profile = 'network' if role == 'dns' else stream.fixture_profile
            if profile is None:
                return
            request = read_exact(stream, struct.unpack('!H', read_exact(stream, 2))[0])
            reply = answer(request, profile, peer)
            stream.sendall(struct.pack('!H', len(reply)) + reply)
        elif role == 'doh':
            line, headers = read_headers(stream)
            method, path, version = line.split(' ')
            token = path.removeprefix('/dns-query/') if path.startswith('/dns-query/') else ''
            profile = config['tokens'].get(token)
            if method != 'POST' or version != 'HTTP/1.1' or profile is None:
                respond(stream, '403 Forbidden')
                return
            if headers.get('content-type') != 'application/dns-message':
                respond(stream, '415 Unsupported Media Type')
                return
            reply = answer(read_exact(stream, int(headers['content-length'])), profile, peer)
            respond(stream, '200 OK', reply, 'application/dns-message')
        elif role == 'origin':
            read_headers(stream)
            respond(stream, '200 OK', json.dumps({'peer': peer, 'uid': os.geteuid()}).encode())
        elif role == 'proxy':
            line, headers = read_headers(stream)
            if headers.get('proxy-authorization') != config['proxy_auth']:
                respond(stream, '407 Proxy Authentication Required')
                return
            origin = address('origin', 2, family)
            target = f'[{origin}]' if family == 6 else origin
            if line != f'CONNECT {target}:{PORTS["origin"]} HTTP/1.1':
                respond(stream, '403 Forbidden')
                return
            with socket.create_connection((origin, PORTS['origin']), timeout=3) as outbound:
                stream.sendall(b'HTTP/1.1 200 Connection Established\r\n\r\n')
                relay(stream, outbound)
        else:
            if role == 'gateway':
                outbound = socket.create_connection((address('origin', 2, family), PORTS['origin']), timeout=3)
            else:
                raw = socket.create_connection((address('proxy', 2, family), PORTS['proxy']), timeout=3)
                outbound = client_context(config).wrap_socket(raw, server_hostname='proxy.test')
                origin = address('origin', 2, family)
                target = f'[{origin}]' if family == 6 else origin
                outbound.sendall((f'CONNECT {target}:{PORTS["origin"]} HTTP/1.1\r\n'
                                  f'Proxy-Authorization: {config["proxy_auth"]}\r\n\r\n').encode())
                if read_headers(outbound)[0] != 'HTTP/1.1 200 Connection Established':
                    outbound.close()
                    return
            with outbound:
                relay(stream, outbound)


def serve(role, config):
    if os.geteuid() == 0:
        raise RuntimeError('fixture services must be unprivileged')
    namespace = role if role in ('origin', 'proxy') else 'dp'
    if os.readlink('/proc/self/ns/net') != config['netns'][namespace]:
        raise RuntimeError('fixture service is outside its lab namespace')
    tls_role = {'dot': 'resolver', 'doh': 'resolver', 'origin': 'origin', 'proxy': 'proxy'}.get(role)
    context = server_context(config, tls_role) if tls_role else None
    sockets, workers = [], threading.BoundedSemaphore(8)

    def worker(stream, peer):
        try:
            with stream:
                stream.settimeout(3)
                handle(stream, peer, role, config, context)
        except (OSError, EOFError, ValueError, KeyError):
            pass
        finally:
            workers.release()

    for family in (socket.AF_INET, socket.AF_INET6):
        for kind in ([socket.SOCK_STREAM, socket.SOCK_DGRAM] if role == 'dns' else [socket.SOCK_STREAM]):
            stream = socket.socket(family, kind)
            stream.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            if family == socket.AF_INET6:
                stream.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 1)
            stream.bind(('0.0.0.0' if family == socket.AF_INET else '::', PORTS[role]))
            if kind == socket.SOCK_STREAM:
                stream.listen(8)
            sockets.append(stream)
    print('READY', flush=True)
    while True:
        ready, _, _ = select.select(sockets, [], [], 1)
        for stream in ready:
            if stream.type == socket.SOCK_DGRAM:
                data, peer = stream.recvfrom(513)
                if peer[0] == address('edge', 1, ipaddress.ip_address(peer[0]).version):
                    try:
                        stream.sendto(answer(data, 'network', peer[0]), peer)
                    except ValueError:
                        pass
            else:
                accepted, peer = stream.accept()
                if workers.acquire(blocking=False):
                    threading.Thread(target=worker, args=(accepted, peer[0]), daemon=True).start()
                else:
                    accepted.close()


def dns_request(config, family, transport, token, name='profile.test'):
    request = query(name, 1 if family == 4 else 28)
    endpoint = address('edge', 2, family)
    if transport == 'udp':
        with socket.socket(socket.AF_INET if family == 4 else socket.AF_INET6, socket.SOCK_DGRAM) as stream:
            stream.settimeout(2)
            stream.connect((endpoint, PORTS['dns']))
            stream.send(request)
            return decoded_answer(stream.recv(512), request)
    raw = socket.create_connection((endpoint, PORTS[transport if transport in ('dot', 'doh') else 'dns']), timeout=3)
    if transport == 'dot':
        raw = client_context(config).wrap_socket(raw, server_hostname=token + '.resolver.test')
    elif transport == 'doh':
        raw = client_context(config).wrap_socket(raw, server_hostname='dns.resolver.test')
    with raw as stream:
        if transport == 'doh':
            stream.sendall((f'POST /dns-query/{token} HTTP/1.1\r\nHost: dns.resolver.test\r\n'
                            f'Content-Type: application/dns-message\r\nContent-Length: {len(request)}\r\n\r\n').encode() + request)
            status, headers = read_headers(stream)
            if status != 'HTTP/1.1 200 OK':
                raise PermissionError('resolver capability denied')
            reply = read_exact(stream, int(headers['content-length']))
        else:
            stream.sendall(struct.pack('!H', len(request)) + request)
            reply = read_exact(stream, struct.unpack('!H', read_exact(stream, 2))[0])
    return decoded_answer(reply, request)


def denied(operation):
    try:
        operation()
    except (OSError, EOFError):
        return
    raise AssertionError('unauthorized fixture operation succeeded')


def probe(client, config, outage=False):
    assert os.geteuid() != 0
    assert os.readlink('/proc/self/ns/net') == config['netns'][client]
    results = []
    tokens = {profile: token for token, profile in config['tokens'].items()}
    for family in (4, 6):
        if outage:
            def unavailable_proxy():
                raw = socket.create_connection((address('edge', 2, family), PORTS['gateway_proxy']), timeout=3)
                with client_context(config).wrap_socket(raw, server_hostname='origin.test'):
                    pass
            denied(unavailable_proxy)
            results.append({'case': 'proxy-outage', 'family': family, 'no_direct_fallback': True})
            continue
        peer = address('mismatch', 2, family) if client == 'mismatch' else address('edge', 1, family)
        for transport in ('doh', 'dot'):
            for profile in ('a', 'b'):
                assert dns_request(config, family, transport, tokens[profile]) == str(ipaddress.ip_address(profile_address(profile, family)))
            assert dns_request(config, family, transport, tokens['a'], 'peer.test') == peer
            denied(lambda: dns_request(config, family, transport, 'unknown'))
            results.append({'case': transport, 'family': family, 'peer': peer, 'profiles': ['a', 'b'], 'unknown_denied': True})
        if client == 'mismatch':
            denied(lambda: dns_request(config, family, 'tcp', ''))
        else:
            for transport in ('udp', 'tcp'):
                assert dns_request(config, family, transport, '') == str(ipaddress.ip_address(profile_address('network', family)))
                results.append({'case': transport, 'family': family, 'peer': peer, 'profile': 'network'})
        for gateway in ('gateway', 'gateway_proxy'):
            def visit():
                raw = socket.create_connection((address('edge', 2, family), PORTS[gateway]), timeout=3)
                with client_context(config).wrap_socket(raw, server_hostname='origin.test') as stream:
                    fingerprint = hashlib.sha256(stream.getpeercert(binary_form=True)).hexdigest()
                    assert fingerprint == config['origin_sha256']
                    stream.sendall(b'GET / HTTP/1.1\r\nHost: origin.test\r\n\r\n')
                    status, headers = read_headers(stream)
                    assert status == 'HTTP/1.1 200 OK'
                    body = json.loads(read_exact(stream, int(headers['content-length'])))
                    assert body['uid'] != 0
                    expected = address('origin', 1, family) if gateway == 'gateway' else address('proxy', 2, family)
                    assert body['peer'] == expected
                    results.append({'case': gateway, 'family': family, 'origin_peer': body['peer'], 'origin_certificate_preserved': True})
            if client == 'mismatch':
                denied(visit)
                results.append({'case': gateway, 'family': family, 'source_mismatch_denied': True})
            else:
                visit()
        if client == 'a' and family == 4:
            for context, hostname in [(client_context(config), 'wrong.test'),
                                      (ssl.create_default_context(), 'origin.test')]:
                with socket.create_connection((address('edge', 2, family), PORTS['gateway']), timeout=3) as raw:
                    try:
                        with context.wrap_socket(raw, server_hostname=hostname):
                            pass
                    except ssl.SSLCertVerificationError:
                        continue
                    raise AssertionError('invalid origin trust or hostname was accepted')
            results.append({'case': 'tls-verification', 'family': family, 'wrong_hostname_denied': True, 'untrusted_ca_denied': True})
        for authorization, target, expected in [('', '127.0.0.1:22', '407'),
                                                 (config['proxy_auth'], '127.0.0.1:22', '403')]:
            raw = socket.create_connection((address('proxy', 2, family), PORTS['proxy']), timeout=3)
            with client_context(config).wrap_socket(raw, server_hostname='proxy.test') as stream:
                stream.sendall((f'CONNECT {target} HTTP/1.1\r\nProxy-Authorization: {authorization}\r\n\r\n').encode())
                assert read_headers(stream)[0].split(' ')[1] == expected
        results.append({'case': 'protected-proxy-denials', 'family': family, 'missing_auth_denied': True, 'arbitrary_target_denied': True})
    print(json.dumps({'client': client, 'results': results}), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=['serve', 'probe', 'outage'])
    parser.add_argument('role', choices=list(PORTS) + ['a', 'b', 'mismatch'])
    parser.add_argument('config', type=Path)
    args = parser.parse_args()
    configuration = json.loads(args.config.read_text(encoding='utf-8'))
    if args.operation == 'serve':
        serve(args.role, configuration)
    else:
        probe(args.role, configuration, outage=args.operation == 'outage')
