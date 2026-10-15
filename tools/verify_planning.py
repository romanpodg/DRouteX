"""Validate planning contracts, links and source preservation, not product behavior."""
from pathlib import Path
import copy
import hashlib
import json
import re
import sys
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]
DELIVERIES = {'MVP': 0, 'POST_MVP_PROTOCOLS': 1, 'POST_MVP_MULTI_NODE': 2, 'FUTURE': 3}


def validate_tasks(tasks):
    ids = {task['id'] for task in tasks}
    assert len(ids) == len(tasks), 'Duplicate task ID'
    required = ('id','phase','title','goal','spec','scope','files','acceptance','tests','security','quality','complexity','status','definition_of_done','delivery','scope_change')
    by_id = {task['id']:task for task in tasks}
    for task in tasks:
        assert all(task.get(k) for k in required), f"Missing task evidence field: {task['id']}"
        assert task['complexity'] in {'S','M','L'}, 'Unknown complexity'
        assert task['delivery'] in DELIVERIES, 'Unknown delivery'
        assert task['scope_change'] == 'CR-0001', 'Missing scope amendment'
        assert all(1 <= section <= 27 for section in task['spec']), 'Unknown spec section'
        assert all(p in ids for p in task['prerequisites']), f"Unknown prerequisite: {task['id']}"
    for task in tasks:
        for prior in task['prerequisites']:
            assert DELIVERIES[by_id[prior]['delivery']] <= DELIVERIES[task['delivery']], f"Later-release dependency: {task['id']} -> {prior}"
    done, active = set(), set()
    def visit(task_id):
        assert task_id not in active, f'Dependency cycle at {task_id}'
        if task_id in done:
            return
        active.add(task_id)
        for prior in by_id[task_id]['prerequisites']:
            visit(prior)
        active.remove(task_id)
        done.add(task_id)
    for task_id in ids:
        visit(task_id)
    assert 'P3-07' in by_id['P4-08']['prerequisites'], 'Missing required secure-DNS gate'
    assert 'P2-07' in by_id['P2-05']['prerequisites'], 'Missing required catalog gate'
    return ids


def self_test(tasks):
    invalid = []
    missing = copy.deepcopy(tasks); missing[0]['acceptance'] = ''; invalid.append(missing)
    unknown = copy.deepcopy(tasks); unknown[0]['prerequisites'] = ['MISSING']; invalid.append(unknown)
    cycle = copy.deepcopy(tasks); cycle[0]['prerequisites'] = [cycle[1]['id']]; cycle[1]['prerequisites'] = [cycle[0]['id']]; invalid.append(cycle)
    duplicate = copy.deepcopy(tasks); duplicate.append(duplicate[0]); invalid.append(duplicate)
    later = copy.deepcopy(tasks)
    next(t for t in later if t['id'] == 'P4-05')['prerequisites'].append('P5-02')
    invalid.append(later)
    insecure = copy.deepcopy(tasks)
    next(t for t in insecure if t['id'] == 'P4-08')['prerequisites'].remove('P3-07')
    invalid.append(insecure)
    unpinned = copy.deepcopy(tasks)
    next(t for t in unpinned if t['id'] == 'P2-05')['prerequisites'].remove('P2-07')
    invalid.append(unpinned)
    reasons = ('Missing task evidence field', 'Unknown prerequisite', 'Dependency cycle',
               'Duplicate task ID', 'Later-release dependency',
               'Missing required secure-DNS gate', 'Missing required catalog gate')
    for mutated, reason in zip(invalid, reasons):
        try:
            validate_tasks(mutated)
        except AssertionError as error:
            assert reason in str(error), f'Wrong negative-test rejection: {error}'
            continue
        raise AssertionError('Negative validator self-test was incorrectly accepted')
    return len(invalid)


def main():
    plan = json.loads((ROOT/'docs/planning/backlog.json').read_text(encoding='utf-8'))
    ids = validate_tasks(plan['tasks'])
    by_id = {task['id']: task for task in plan['tasks']}
    counts = {d: sum(t['delivery'] == d for t in plan['tasks']) for d in DELIVERIES}
    assert counts == {'MVP': 61, 'POST_MVP_PROTOCOLS': 11, 'POST_MVP_MULTI_NODE': 11, 'FUTURE': 5}, 'Unexpected delivery inventory'
    def ancestors(task_id):
        result = set()
        for prior in by_id[task_id]['prerequisites']:
            result.add(prior)
            result.update(ancestors(prior))
        return result
    release = ancestors('P11-06') | {'P11-06'}
    assert release == {t['id'] for t in plan['tasks'] if t['delivery'] == 'MVP'}, 'MVP release must cover all and only current tasks'
    for task in plan['tasks']:
        if task['delivery'] == 'POST_MVP_PROTOCOLS':
            assert 'P11-06' in ancestors(task['id']), 'Protocol expansion must follow MVP release'
    trace = json.loads((ROOT/'docs/planning/traceability.json').read_text(encoding='utf-8'))
    for item in trace['requirements']:
        assert item['tasks'] and all(task in ids for task in item['tasks']), f"Unmapped requirement {item['id']}"
        assert item['delivery'] == sorted({by_id[t]['delivery'] for t in item['tasks']}), 'Stale trace delivery'
        assert (ROOT/item['source_file']).is_file(), 'Invalid trace source'
    indexed = {item['id']: item for item in trace['requirements']}
    assert len(indexed) == len(trace['requirements']) == 188, 'Unexpected trace inventory'
    for number in range(7, 11):
        item = indexed[f'R-24.1-{number:02}']
        assert item['delivery'] == ['POST_MVP_PROTOCOLS'] and 'pending' in item['scope'], 'Original engine checklist must remain deferred, not passed as MVP'
    amendment = indexed['R-CR-0001']
    assert amendment['source_file'] == 'docs/architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md'
    assert amendment['kind'] == 'Accepted user instruction'
    proposal = indexed['R-CR-0002']
    assert proposal['kind'] == 'Proposed design', 'Research must not silently accept a design'
    assert proposal['source_file'] == 'docs/architecture/adr/ADR-0017-profile-endpoint-dns-product-model.md'
    assert plan['design_proposals'][0]['status'] == 'proposed'
    source = ROOT/'Routira_Project_Specification_EN.md'
    assert hashlib.sha256(source.read_bytes()).hexdigest() == trace['source_sha256'], 'English source changed'
    ru = ROOT/'Routira_Project_Specification_RU.md'
    assert hashlib.sha256(ru.read_bytes()).hexdigest() == '25fb27b433fc6b2e72ccf0392a731721be0505e562e539bb93cb8fb20debee46', 'Russian source changed'
    headings = re.findall(r'^###? (\d+(?:\.\d+)?)\. ', source.read_text(encoding='utf-8'), flags=re.M)
    mapped = {item['section'] for item in trace['requirements']}
    assert set(headings) <= mapped, 'A spec heading was not considered'
    assert {str(i) for i in range(1,28)} <= mapped, 'A top-level spec section was omitted'
    docs = list((ROOT/'docs').rglob('*.md')) + [ROOT/'README.md', ROOT/'AGENTS.md']
    broken = []
    link_count = 0
    for doc in docs:
        content = re.sub(r'```.*?```', '', doc.read_text(encoding='utf-8'), flags=re.S)
        for target in re.findall(r'\]\(([^)]+)\)', content):
            target = target.strip('<> ').split('#')[0]
            if not target or re.match(r'^[a-zA-Z]+:', target):
                continue
            link_count += 1
            resolved = (doc.parent/unquote(target)).resolve()
            if not resolved.exists():
                broken.append(f'{doc.relative_to(ROOT)} -> {target}')
    assert not broken, '\n'.join(broken)
    adrs = list((ROOT/'docs/architecture/adr').glob('ADR-*.md'))
    assert len(adrs) == 17
    assert {p.name.split('-')[1] for p in adrs} == {f'{n:04}' for n in range(1,18)}
    for adr in adrs:
        content = adr.read_text(encoding='utf-8')
        expected = 'Accepted' if adr.name.startswith('ADR-0016-') else 'Proposed'
        assert f'Status: **{expected}**' in content
        for section in ('Context','Decision','Alternatives','Reasons','Consequences','Risks','Validation approach'):
            assert f'## {section}' in content, f'{adr.name}: {section}'
    assert not (ROOT/'Cargo.toml').exists(), 'Unexpected application workspace'
    assert not (ROOT/'apps').exists() and not (ROOT/'crates').exists(), 'Unexpected production code'
    agents = (ROOT/'AGENTS.md').read_text(encoding='utf-8')
    assert 'mcp__node_repl__js' in agents and 'sky.list_apps()' in agents and 'browser-only' in agents
    negatives = self_test(plan['tasks']) if '--self-test' in sys.argv else 0
    result = {'status':'passed','scope_amendment':'CR-0001','design_proposal':'CR-0002','tasks':len(ids),'deliveries':counts,'mvp_release_dependency_closure':len(release),'requirements':len(trace['requirements']),'source_headings':len(headings),'local_links_checked':link_count,'adrs':len(adrs),'accepted_adrs':1,'negative_validator_cases':negatives,'source_specs_unchanged':True,'application_code_absent':True}
    (ROOT/'docs/environment').mkdir(parents=True, exist_ok=True)
    (ROOT/'docs/environment/planning-verification.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
