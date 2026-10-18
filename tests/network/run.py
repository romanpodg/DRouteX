"""Trusted, local-only Linux lab. Never run arbitrary PR code on a privileged runner."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import platform
import secrets
import select
import shutil
import signal
import ssl
import subprocess
import sys
import tempfile
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
from fixtures import PORTS, address


def command(*args, input=None):
    return subprocess.run(args, input=input, text=True, capture_output=True,
                          check=True, timeout=20).stdout.strip()


def host_state():
    return [command('ip', '-j', 'link'), command('ip', '-j', 'addr'),
            command('ip', '-j', 'route'), command('ip', '-6', '-j', 'route'),
            command('ip', 'netns', 'list'), command('nft', '-j', 'list', 'ruleset')]


def wait_line(process, expected=None):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if select.select([process.stdout], [], [], 0.2)[0]:
            line = process.stdout.readline()
            if not line:
                raise RuntimeError('fixture exited before readiness')
            if expected is None or line.strip() == expected:
                return line
        if process.poll() is not None:
            raise RuntimeError('fixture exited before readiness')
    raise TimeoutError('fixture readiness deadline')


def certificates(directory):
    ca, ca_key = directory/'ca.pem', directory/'ca.key'
    command('openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-noenc', '-days', '1',
            '-config', '/dev/null', '-addext', 'basicConstraints=critical,CA:TRUE',
            '-addext', 'keyUsage=critical,keyCertSign,cRLSign', '-addext', 'subjectKeyIdentifier=hash',
            '-subj', '/CN=DRouteX disposable fixture CA', '-keyout', str(ca_key), '-out', str(ca))
    result = {}
    for role, hostname in [('origin', 'origin.test'), ('proxy', 'proxy.test'),
                           ('resolver', '*.resolver.test')]:
        key, csr, cert, ext = [directory/f'{role}.{suffix}' for suffix in ('key', 'csr', 'pem', 'ext')]
        ext.write_text(f'subjectAltName=DNS:{hostname}\nextendedKeyUsage=serverAuth\n'
                       'basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\n'
                       'subjectKeyIdentifier=hash\nauthorityKeyIdentifier=keyid,issuer\n', encoding='ascii')
        command('openssl', 'req', '-newkey', 'rsa:2048', '-noenc', '-subj', '/CN=' + hostname,
                '-config', '/dev/null',
                '-keyout', str(key), '-out', str(csr))
        command('openssl', 'x509', '-req', '-in', str(csr), '-CA', str(ca), '-CAkey', str(ca_key),
                '-set_serial', str(secrets.randbits(64) or 1), '-days', '1', '-extfile', str(ext), '-out', str(cert))
        for path in (key, cert):
            os.chown(path, 0, 65534)
            path.chmod(0o640)
        result[role] = {'certfile': str(cert), 'keyfile': str(key)}
    return str(ca), result


class Lab:
    def __init__(self, directory):
        self.directory = directory
        self.prefix = 'rtp003-' + secrets.token_hex(3) + '-'
        self.names = {}
        self.processes = []

    def net(self, role, *args, input=None):
        return command('ip', 'netns', 'exec', self.names[role], *args, input=input)

    def topology(self, fail=False):
        for role in ('a', 'b', 'mismatch', 'edge', 'dp', 'origin', 'proxy'):
            name = self.prefix + role
            command('ip', 'netns', 'add', name)
            self.names[role] = name
            command('ip', '-n', name, 'link', 'set', 'dev', 'lo', 'up')
        connections = [('edge', 'dp', 'dp', 'edge', 'edge'),
                       ('dp', 'origin', 'origin', 'dp', 'origin'),
                       ('dp', 'proxy', 'proxy', 'dp', 'proxy')]
        connections += [('edge', client, client, 'edge', client) for client in ('a', 'b', 'mismatch')]
        for index, (left, right, left_if, right_if, link) in enumerate(connections):
            first, second = f'p{index}', f'q{index}'
            command('ip', 'link', 'add', 'name', first, 'type', 'veth', 'peer', 'name', second)
            for side, (role, temporary, interface) in enumerate(((left, first, left_if), (right, second, right_if)), 1):
                command('ip', 'link', 'set', 'dev', temporary, 'netns', self.names[role])
                command('ip', '-n', self.names[role], 'link', 'set', 'dev', temporary, 'name', interface)
                for family, prefix in ((4, 30), (6, 64)):
                    options = ['nodad'] if family == 6 else []
                    command('ip', '-n', self.names[role], 'addr', 'add',
                            f'{address(link, side, family)}/{prefix}', 'dev', interface, *options)
                command('ip', '-n', self.names[role], 'link', 'set', 'dev', interface, 'up')
            if fail:
                raise RuntimeError('injected topology failure')
        for role in ('edge', 'dp'):
            self.net(role, 'sysctl', '-q', '-w', 'net.ipv4.ip_forward=1', 'net.ipv6.conf.all.forwarding=1')
        for family in (4, 6):
            flag = '-4' if family == 4 else '-6'
            for role, link, side in [('a', 'a', 1), ('b', 'b', 1), ('mismatch', 'mismatch', 1),
                                     ('edge', 'edge', 2), ('origin', 'origin', 1), ('proxy', 'proxy', 1)]:
                command('ip', '-n', self.names[role], flag, 'route', 'add', 'default', 'via', address(link, side, family))
            prefix = '198.18.0.0/15' if family == 4 else 'fd42:726f:7574::/48'
            command('ip', '-n', self.names['dp'], flag, 'route', 'add', prefix, 'via', address('edge', 1, family))
        self.net('edge', 'nft', '-f', '-', input='''table ip rtp003 {
 chain postrouting { type nat hook postrouting priority srcnat; policy accept;
 ip saddr { 198.18.10.0/30, 198.18.11.0/30 } oifname "dp" snat to 198.18.0.1
 }
}
table ip6 rtp003 {
 chain postrouting { type nat hook postrouting priority srcnat; policy accept;
 ip6 saddr { fd42:726f:7574:a::/64, fd42:726f:7574:b::/64 } oifname "dp" snat to fd42:726f:7574::1
 }
}
''')

    def start(self, role, config):
        namespace = role if role in ('origin', 'proxy') else 'dp'
        process = subprocess.Popen(['ip', 'netns', 'exec', self.names[namespace],
                                    'setpriv', '--reuid=65534', '--regid=65534', '--clear-groups',
                                    '--no-new-privs', sys.executable, '-I', str(HERE/'fixtures.py'),
                                    'serve', role, str(config)], stdout=subprocess.PIPE,
                                   stderr=subprocess.DEVNULL, text=True)
        self.processes.append(process)
        wait_line(process, 'READY')
        return process

    def probe(self, client, config, outage=False):
        args = ['ip', 'netns', 'exec', self.names[client], 'setpriv', '--reuid=65534',
                '--regid=65534', '--clear-groups', '--no-new-privs', sys.executable, '-I',
                str(HERE/'fixtures.py'), 'outage' if outage else 'probe', client, str(config)]
        return json.loads(command(*args))

    def cleanup(self):
        for process in reversed(self.processes):
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=2)
            if process.stdout:
                process.stdout.close()
        for name in reversed(list(self.names.values())):
            if not name.startswith(self.prefix):
                raise RuntimeError('refusing foreign namespace deletion')
            command('ip', 'netns', 'del', name)
        assert not command('ip', 'netns', 'list'), 'fixture namespaces leaked'


def inside(args):
    # Refuse host execution before any mount, sysctl, interface or firewall mutation.
    if (os.getpid() != 1 or os.readlink('/proc/self/ns/net') == args.parent_net
            or os.readlink('/proc/self/ns/mnt') == args.parent_mount):
        raise RuntimeError('missing outer namespace isolation')
    links = json.loads(command('ip', '-j', 'link'))
    if len(links) != 1 or links[0]['ifname'] != 'lo':
        raise RuntimeError('outer network is not empty')
    command('mount', '--make-rprivate', '/')
    command('mount', '-t', 'tmpfs', '-o', 'mode=755', 'tmpfs', '/run')
    Path('/run/netns').mkdir()
    directory = args.directory.resolve(strict=True)
    if directory.parent != Path(tempfile.gettempdir()) or not directory.name.startswith('droutex-p003-'):
        raise RuntimeError('unexpected fixture directory')
    # Keep keys/config in private tmpfs even if the outer supervisor is killed.
    command('mount', '-t', 'tmpfs', '-o', 'size=16m,mode=750,gid=65534', 'tmpfs', str(directory))
    os.chown(directory, 0, 65534)
    directory.chmod(0o750)
    lab = Lab(directory)
    capture = None
    try:
        lab.topology(fail=args.mode == 'setup-failure')
        ca, tls = certificates(directory)
        config = {'ca': ca, 'tls': tls, 'tokens': {secrets.token_hex(16): p for p in ('a', 'b')},
                  'netns': {role: f'net:[{Path("/run/netns", name).stat().st_ino}]' for role, name in lab.names.items()},
                  'proxy_auth': 'Basic ' + base64.b64encode(('fixture:' + secrets.token_hex(16)).encode()).decode(),
                  'origin_sha256': hashlib.sha256(ssl.PEM_cert_to_DER_cert(Path(tls['origin']['certfile']).read_text())).hexdigest()}
        config_path = directory/'config.json'
        config_path.write_text(json.dumps(config), encoding='utf-8')
        os.chown(config_path, 0, 65534)
        config_path.chmod(0o640)
        processes = {role: lab.start(role, config_path) for role in PORTS}
        capture = subprocess.Popen(['ip', 'netns', 'exec', lab.names['dp'], 'tcpdump', '--immediate-mode', '-i', 'any',
                                    '-nn', '-U', '-s', '128', '-c', '5000', '-Z', 'root',
                                    '-w', str(directory/'traffic.pcap')], stdout=subprocess.DEVNULL,
                                   stderr=subprocess.PIPE, bufsize=0)
        # tcpdump announces readiness after opening the interface and output file.
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if select.select([capture.stderr], [], [], 0.2)[0] and b'listening on' in capture.stderr.readline():
                break
            if capture.poll() is not None:
                raise RuntimeError('capture startup failed')
        else:
            raise TimeoutError('capture readiness deadline')
        namespaces = [os.readlink('/proc/self/ns/mnt')]
        namespaces += [f'net:[{Path("/run/netns", name).stat().st_ino}]' for name in lab.names.values()]
        print(json.dumps({'event': 'ready', 'namespaces': namespaces}), flush=True)
        if args.mode in ('term', 'kill'):
            signal.pause()
        results = [lab.probe(client, config_path) for client in ('a', 'b', 'mismatch')]
        processes['proxy'].terminate()
        processes['proxy'].wait(timeout=2)
        results.append(lab.probe('a', config_path, outage=True))
        print(json.dumps({'event': 'result', 'clients': results}), flush=True)
        return 0
    except InterruptedError:
        return 143
    except Exception as error:
        failure = {'event': 'error', 'type': type(error).__name__}
        if isinstance(error, subprocess.CalledProcessError):
            failure.update(command=error.cmd[0], exit_code=error.returncode)
            if str(HERE/'fixtures.py') not in error.cmd:
                failure.update(arguments=error.cmd[1:], diagnostic=error.stderr[:1000])
            else:
                diagnostic = error.stderr
                for secret in [*config['tokens'], config['proxy_auth'], config['proxy_auth'].removeprefix('Basic ')]:
                    diagnostic = diagnostic.replace(secret, '[redacted]')
                failure['diagnostic'] = diagnostic[-2000:]
        print(json.dumps(failure), flush=True)
        return 23
    finally:
        try:
            if capture:
                if capture.poll() is None:
                    capture.send_signal(signal.SIGINT)
                    capture.wait(timeout=3)
                capture.stderr.close()
                if args.mode == 'normal':
                    output = args.artifacts.resolve(strict=True)
                    if output.parent != (ROOT/'.setup'/'network').resolve():
                        raise RuntimeError('unexpected capture directory')
                    shutil.copyfile(directory/'traffic.pcap', output/'traffic.pcap')
        finally:
            lab.cleanup()
            print(json.dumps({'event': 'cleanup', 'remaining_namespaces': 0}), flush=True)


def remaining_processes(namespaces):
    found = []
    for entry in Path('/proc').iterdir():
        if entry.name.isdecimal():
            try:
                if any(os.readlink(entry/'ns'/kind) in namespaces for kind in ('net', 'mnt')):
                    found.append(int(entry.name))
            except (FileNotFoundError, PermissionError, ProcessLookupError):
                pass
    return found


def run_case(mode, artifacts):
    with tempfile.TemporaryDirectory(prefix='droutex-p003-') as temporary:
        args = ['timeout', '--preserve-status', '--signal=TERM', '--kill-after=5s', '60s',
                'unshare', '--mount', '--net', '--pid', '--fork', '--kill-child', '--mount-proc',
                sys.executable, '-I', str(HERE/'run.py'), '--inside', '--mode', mode,
                '--directory', temporary, '--artifacts', str(artifacts),
                '--parent-net', os.readlink('/proc/self/ns/net'),
                '--parent-mount', os.readlink('/proc/self/ns/mnt')]
        # Avoid prefetching later events before communicate() reads the file descriptor.
        process = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   bufsize=0, start_new_session=True)
        events, namespaces = [], []
        try:
            first = json.loads(wait_line(process))
            events.append(first)
            if first['event'] == 'ready':
                namespaces = first['namespaces']
                if mode in ('term', 'kill'):
                    os.killpg(process.pid, signal.SIGTERM if mode == 'term' else signal.SIGKILL)
            output, errors = process.communicate(timeout=60)
            events += [json.loads(line) for line in output.splitlines() if line]
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=5)
        deadline = time.monotonic() + 3
        while remaining_processes(namespaces) and time.monotonic() < deadline:
            time.sleep(0.05)
        assert not remaining_processes(namespaces), 'fixture processes leaked'
        expected = {'normal': 0, 'setup-failure': 23, 'term': 143, 'kill': -9}[mode]
        if process.returncode != expected:
            raise RuntimeError(f'{mode} failed ({process.returncode}): {events}; {errors[:2000]}')
        if mode != 'kill':
            assert events[-1] == {'event': 'cleanup', 'remaining_namespaces': 0}
        result = {'mode': mode, 'exit_code': process.returncode, 'remaining_processes': 0}
        if mode == 'normal':
            result['clients'] = next(e['clients'] for e in events if e['event'] == 'result')
            capture = artifacts/'traffic.pcap'
            packets = command('tcpdump', '-nn', '-r', str(capture)).splitlines()
            assert any(' IP ' in (' ' + p) for p in packets) and any(' IP6 ' in (' ' + p) for p in packets)
            result['source_mismatch_pairs'] = []
            for family in (4, 6):
                observed = [next(r['peer'] for r in client['results'] if r['case'] == 'doh' and r['family'] == family)
                            for client in result['clients'][:2]]
                assert observed[0] == observed[1] == address('edge', 1, family)
                source = address('mismatch', 2, family)
                assert command('tcpdump', '-nn', '-r', str(capture), '-c', '1',
                               'src', 'host', source, 'and', 'dst', 'port', str(PORTS['gateway']))
                result['source_mismatch_pairs'].append({'family': family, 'dns_peer': observed[0],
                                                       'tcp_source': source, 'different_source_denied': True})
            result['capture'] = {'packets': len(packets), 'sha256': hashlib.sha256(capture.read_bytes()).hexdigest()}
        return result


def main():
    os.environ['PATH'] = '/usr/sbin:/usr/bin:/sbin:/bin'
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inside', action='store_true', help=argparse.SUPPRESS)
    parser.add_argument('--mode', choices=['normal', 'setup-failure', 'term', 'kill'], default='normal')
    parser.add_argument('--directory', type=Path)
    parser.add_argument('--artifacts', type=Path)
    parser.add_argument('--parent-net')
    parser.add_argument('--parent-mount')
    args = parser.parse_args()
    if sys.platform != 'linux' or os.geteuid() != 0 or sys.flags.optimize:
        parser.error('requires a trusted disposable Linux lab, root setup and enabled assertions')
    if args.inside:
        def interrupted(_number, _frame):
            signal.signal(signal.SIGTERM, signal.SIG_IGN)
            raise InterruptedError('fixture interrupted')
        signal.signal(signal.SIGTERM, interrupted)
        return inside(args)
    for tool in ('ip', 'nft', 'openssl', 'tcpdump', 'unshare', 'mount', 'setpriv', 'timeout'):
        if not shutil.which(tool):
            parser.error('missing lab tool: ' + tool)
    artifacts = ROOT/'.setup'/'network'/secrets.token_hex(8)
    if not artifacts.resolve().is_relative_to(ROOT):
        raise RuntimeError('artifact path escaped workspace')
    artifacts.mkdir(parents=True, mode=0o700)
    before = host_state()
    try:
        results = [run_case(mode, artifacts) for mode in ('normal', 'setup-failure', 'term', 'kill')]
    finally:
        assert host_state() == before, 'host network state changed'
    release = platform.freedesktop_os_release()
    report = {'status': 'passed', 'scope': 'P0-03 fixtures, not DRouteX acceptance',
              'os': release['PRETTY_NAME'], 'kernel': platform.release(), 'python': platform.python_version(),
              'openssl': ssl.OPENSSL_VERSION, 'iproute2': command('ip', '-Version'),
              'nftables': command('nft', '--version'), 'tcpdump': command('tcpdump', '--version').splitlines()[0],
              'package_versions': command('dpkg-query', '-W', '-f=${Package}=${Version}\n',
                                          'python3', 'openssl', 'iproute2', 'nftables', 'tcpdump', 'util-linux').splitlines(),
              'native_debian13': release['ID'] == 'debian' and release['VERSION_ID'] == '13' and 'microsoft' not in platform.release().lower(),
              'host_network_unchanged': True, 'cases': results}
    (artifacts/'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps({'status': 'passed', 'report': str(artifacts/'report.json'),
                      'cleanup_modes': [r['mode'] for r in results], 'host_network_unchanged': True}))
    return 0


if __name__ == '__main__':
    sys.exit(main())
