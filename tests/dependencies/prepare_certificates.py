"""Generate disposable qualification identities with existing OpenSSL, outside Git."""
from pathlib import Path
import subprocess
import sys

directory = Path(sys.argv[1]).resolve()
directory.mkdir(parents=True, exist_ok=True)

def openssl(*args):
    subprocess.run(['openssl', *args], cwd=directory, check=True,
                   capture_output=True, timeout=30)

openssl('req', '-x509', '-newkey', 'rsa:2048', '-noenc', '-days', '1',
        '-subj', '/CN=DRouteX disposable qualification CA',
        '-addext', 'basicConstraints=critical,CA:TRUE',
        '-addext', 'keyUsage=critical,keyCertSign,cRLSign',
        '-keyout', 'ca.key', '-out', 'ca.pem')
openssl('req', '-newkey', 'rsa:2048', '-noenc', '-subj', '/CN=proxy.test',
        '-keyout', 'proxy.key', '-out', 'proxy.csr')
(directory/'proxy.ext').write_text('subjectAltName=DNS:proxy.test\n'
    'extendedKeyUsage=serverAuth\nbasicConstraints=critical,CA:FALSE\n'
    'keyUsage=critical,digitalSignature,keyEncipherment\n', encoding='ascii')
openssl('x509', '-req', '-in', 'proxy.csr', '-CA', 'ca.pem', '-CAkey', 'ca.key',
        '-set_serial', '2', '-days', '1', '-extfile', 'proxy.ext', '-out', 'proxy.pem')
for name in ('ca', 'proxy'):
    openssl('x509', '-in', name+'.pem', '-outform', 'DER', '-out', name+'.der')
openssl('pkcs8', '-topk8', '-nocrypt', '-in', 'proxy.key', '-outform', 'DER', '-out', 'proxy-key.der')
print('Disposable test certificates generated; do not commit keys or certificates.')
