// Read-only Codex app-server registry check; starts no thread or model turn.
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const readline = require('node:readline');
const root = path.resolve(__dirname, '..');
const server = spawn('codex', ['--no-daemon', 'app-server', '--stdio'], {
  cwd: root, stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true,
});
let errors = '';
server.stderr.on('data', chunk => { errors += chunk.toString(); });
const timeout = setTimeout(() => finish(new Error('skills/list timed out')), 45000);
function send(message) { server.stdin.write(JSON.stringify(message) + '\n'); }
function finish(error, result) {
  clearTimeout(timeout);
  server.kill();
  if (error) { console.error(error.message); process.exitCode = 1; return; }
  const rows = result.data.flatMap(entry => entry.skills || []);
  const wanted = ['graphify', 'idiomatic-rust', 'ownership-not-clone', 'rust-errors', 'type-driven-design', 'rust-api-design', 'async-rust', 'rust-concurrency', 'rust-performance', 'rust-observability', 'unsafe-rust', 'rust-testing', 'rust-code-review', 'rust-supply-chain', 'rust-serde', 'setup-rust-ci', 'setup-rust-pre-commit', 'ponytail', 'ponytail-review', 'ponytail-audit', 'ponytail-debt'];
  const baseName = row => row.name.split(':').pop();
  const evidence = rows.filter(row => wanted.includes(baseName(row)));
  const missing = wanted.filter(name => !evidence.some(row => baseName(row) === name));
  const out = { date: '2026-10-15', method: 'Codex app-server skills/list forceReload; no model turn', evidence, errors: result.data.flatMap(entry => entry.errors || []), missing };
  fs.mkdirSync(path.join(root, '.setup'), {recursive: true});
  fs.writeFileSync(path.join(root, '.setup', 'skill-discovery.json'), JSON.stringify(out, null, 2));
  console.log(JSON.stringify({discovered: evidence.map(row => row.name), missing, errors: out.errors}, null, 2));
  if (missing.length || out.errors.length) process.exitCode = 1;
}
readline.createInterface({input: server.stdout}).on('line', line => {
  let message;
  try { message = JSON.parse(line); } catch { return; }
  if (message.id === 1) {
    if (message.error) return finish(new Error(JSON.stringify(message.error)));
    send({method: 'initialized', params: {}});
    send({id: 2, method: 'skills/list', params: {cwds: [root], forceReload: true}});
  }
  if (message.id === 2) {
    if (message.error) return finish(new Error(JSON.stringify(message.error)));
    finish(null, message.result);
  }
});
server.on('error', error => finish(error));
send({id: 1, method: 'initialize', params: {clientInfo: {name: 'routira_registry_probe', version: '1.0'}, capabilities: {experimentalApi: true}}});
