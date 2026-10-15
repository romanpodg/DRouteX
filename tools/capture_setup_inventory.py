"""Capture non-secret installation provenance and compare installed skill resources."""
from pathlib import Path
import hashlib
import json

ROOT = Path(__file__).resolve().parents[1]


def read(name):
    return json.loads((ROOT/'.setup'/name).read_text(encoding='utf-8-sig'))


def main():
    skills = []
    source_skills = {p.parent.name:p.parent for p in (ROOT/'.setup/rust-skills/skills').rglob('SKILL.md')}
    for installed in sorted((ROOT/'.agents/skills').iterdir()):
        if not installed.is_dir():
            continue
        source = source_skills[installed.name]
        source_files = [p for p in source.rglob('*') if p.is_file()]
        assert len(source_files) == len([p for p in installed.rglob('*') if p.is_file()])
        for file in source_files:
            copied = installed/file.relative_to(source)
            assert copied.read_bytes() == file.read_bytes(), f'Skill resource mismatch: {copied}'
        skills.append({'name':installed.name,'scope':'project','path':str(installed.relative_to(ROOT)).replace('\\','/')+'/SKILL.md','files':len(source_files),'skill_sha256':hashlib.sha256((installed/'SKILL.md').read_bytes()).hexdigest(),'source':'https://github.com/rewrite-rs/skills','ref':'ae0fd62485d071437ae5e6c4036c986b0dd122a0','license':'BSD-3-Clause','all_resources_match_upstream':True})
    assert len(skills) == 16
    discovery = read('skill-discovery.json')
    assert not discovery['missing'] and not discovery['errors']
    binaries = read('tool-downloads.json')
    for tool in binaries:
        executable = {'nextest':'cargo-nextest.exe','cov':'cargo-llvm-cov.exe','deny':'cargo-deny.exe','audit':'cargo-audit.exe'}[tool['tool']]
        actual = hashlib.sha256((ROOT/'.tools/cargo/bin'/executable).read_bytes()).hexdigest()
        assert actual == tool['binary_sha256']
    metadata = {'date':'2026-10-15','timezone':'Asia/Omsk','rust_toolchain':'1.99.0','rustup':'1.29.1','host':'x86_64-pc-windows-msvc','components':['rustfmt','clippy','rust-analyzer','llvm-tools-preview'],
                'cargo_tools':binaries,'actionlint':read('actionlint-inventory.json'),'crate_candidates':read('crate-candidates.json'),
                'sdk':{'version':'10.0.28000.2705','core_sha256':hashlib.sha256((ROOT/'.setup/windows-sdk-cpp.zip').read_bytes()).hexdigest(),'x64_sha256':hashlib.sha256((ROOT/'.setup/windows-sdk-x64.zip').read_bytes()).hexdigest(),'verification':'Both official NuGet catalog SHA512 hashes verified'},
                'codescene':{'version':'1.0.53','build':'c46e3cba9e527cb84281355e7cd6d7d2ead627df','archive_sha256':hashlib.sha256((ROOT/'.setup/cs.zip').read_bytes()).hexdigest(),'verification':'Official HTTPS download, local hash recorded; version/help checked, no publisher checksum claimed','authentication':'not signed in'},
                'graphify':{'package':'graphifyy','version':'0.9.54','scope':'existing isolated uv tool; project Codex skill installed','license':'Apache-2.0 with packaged MIT license and NOTICE retained','api_backend':'none enabled'},
                'ponytail':{'version':'5.2.0','scope':'user Codex plugin','source_ref':'2961b52fc1fe6a5a87c7a959a79512e5c2f3ffe5','hook_trust':'not enabled by this session'},
                'skills':skills,'discovery':{'method':discovery['method'],'names':[s['name'] for s in discovery['evidence']],'missing':[],'errors':[]}}
    (ROOT/'docs/environment').mkdir(parents=True, exist_ok=True)
    (ROOT/'docs/environment/SETUP_INVENTORY.json').write_text(json.dumps(metadata,indent=2),encoding='utf-8')
    print(json.dumps({'rust_skills':len(skills),'resources_compared':sum(s['files'] for s in skills),'discovered_entrypoints':len(discovery['evidence']),'cargo_binary_checksums':len(binaries)}))


if __name__ == '__main__':
    main()
