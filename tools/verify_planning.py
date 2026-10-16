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


def validate_task_fields(task, ids):
    required = ('id','phase','title','goal','spec','scope','files','acceptance','tests','security','quality','complexity','status','definition_of_done','delivery','scope_change')
    assert all(task.get(k) for k in required), f"Missing task evidence field: {task['id']}"
    assert task['complexity'] in {'S','M','L'}, 'Unknown complexity'
    assert task['delivery'] in DELIVERIES, 'Unknown delivery'
    assert task['scope_change'] == 'CR-0001', 'Missing scope amendment'
    assert task['status'] in {'planned', 'in_progress', 'in_review', 'completed'}, 'Unknown task status'
    assert all(1 <= section <= 27 for section in task['spec']), 'Unknown spec section'
    assert all(p in ids for p in task['prerequisites']), f"Unknown prerequisite: {task['id']}"


def validate_dependency_order(by_id):
    for task in by_id.values():
        for prior in task['prerequisites']:
            assert DELIVERIES[by_id[prior]['delivery']] <= DELIVERIES[task['delivery']], f"Later-release dependency: {task['id']} -> {prior}"


def validate_dependency_cycles(by_id):
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
    for task_id in by_id:
        visit(task_id)


def validate_tasks(tasks):
    ids = {task['id'] for task in tasks}
    assert len(ids) == len(tasks), 'Duplicate task ID'
    by_id = {task['id']: task for task in tasks}
    for task in tasks:
        validate_task_fields(task, ids)
    validate_dependency_order(by_id)
    validate_dependency_cycles(by_id)
    for task in tasks:
        if task['status'] != 'planned':
            assert all(by_id[p]['status'] == 'completed' for p in task['prerequisites']), f"Unfinished prerequisite: {task['id']}"
    assert 'P3-07' in by_id['P4-08']['prerequisites'], 'Missing required secure-DNS gate'
    assert 'P2-07' in by_id['P2-05']['prerequisites'], 'Missing required catalog gate'
    return ids


def invalid_task_cases(tasks):
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
    unknown_status = copy.deepcopy(tasks); unknown_status[0]['status'] = 'unknown'; invalid.append(unknown_status)
    premature = copy.deepcopy(tasks)
    next(t for t in premature if t['id'] == 'P0-01')['status'] = 'planned'
    next(t for t in premature if t['id'] == 'P0-02')['status'] = 'in_progress'
    invalid.append(premature)
    reasons = ('Missing task evidence field', 'Unknown prerequisite', 'Dependency cycle',
               'Duplicate task ID', 'Later-release dependency',
               'Missing required secure-DNS gate', 'Missing required catalog gate',
               'Unknown task status', 'Unfinished prerequisite')
    return list(zip(invalid, reasons))


def self_test(tasks):
    cases = invalid_task_cases(tasks)
    for mutated, reason in cases:
        try:
            validate_tasks(mutated)
        except AssertionError as error:
            assert reason in str(error), f'Wrong negative-test rejection: {error}'
            continue
        raise AssertionError('Negative validator self-test was incorrectly accepted')
    return len(cases)


def validate_release(tasks):
    by_id = {task['id']: task for task in tasks}
    counts = {d: sum(t['delivery'] == d for t in tasks) for d in DELIVERIES}
    assert counts == {'MVP': 61, 'POST_MVP_PROTOCOLS': 11, 'POST_MVP_MULTI_NODE': 11, 'FUTURE': 5}, 'Unexpected delivery inventory'
    def ancestors(task_id):
        result = set()
        for prior in by_id[task_id]['prerequisites']:
            result.add(prior)
            result.update(ancestors(prior))
        return result
    release = ancestors('P11-06') | {'P11-06'}
    assert release == {t['id'] for t in tasks if t['delivery'] == 'MVP'}, 'MVP release must cover all and only current tasks'
    for task in tasks:
        if task['delivery'] == 'POST_MVP_PROTOCOLS':
            assert 'P11-06' in ancestors(task['id']), 'Protocol expansion must follow MVP release'
    return counts, release


def validate_requirement_mapping(item, by_id, ids):
    assert item['tasks'] and all(task in ids for task in item['tasks']), f"Unmapped requirement {item['id']}"
    assert item['delivery'] == sorted({by_id[t]['delivery'] for t in item['tasks']}), 'Stale trace delivery'
    assert (ROOT/item['source_file']).is_file(), 'Invalid trace source'


def validate_deferred_requirements(indexed):
    for number in range(7, 11):
        item = indexed[f'R-24.1-{number:02}']
        assert item['delivery'] == ['POST_MVP_PROTOCOLS'] and 'pending' in item['scope'], 'Original engine checklist must remain deferred, not passed as MVP'


def validate_scope_amendments(plan, indexed):
    amendment = indexed['R-CR-0001']
    assert amendment['source_file'] == 'docs/architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md'
    assert amendment['kind'] == 'Accepted user instruction'
    proposal = indexed['R-CR-0002']
    assert proposal['kind'] == 'Accepted user instruction', 'Missing accepted CR-0002 trace'
    assert proposal['source_file'] == 'docs/architecture/adr/ADR-0017-profile-endpoint-dns-product-model.md'
    assert plan['design_proposals'][0]['status'] == 'accepted'


def validate_traceability(plan, trace, ids):
    by_id = {task['id']: task for task in plan['tasks']}
    for item in trace['requirements']:
        validate_requirement_mapping(item, by_id, ids)
    indexed = {item['id']: item for item in trace['requirements']}
    assert len(indexed) == len(trace['requirements']) == 188, 'Unexpected trace inventory'
    validate_deferred_requirements(indexed)
    validate_scope_amendments(plan, indexed)


def validate_source_hashes(trace):
    source = ROOT/'Routira_Project_Specification_EN.md'
    assert hashlib.sha256(source.read_bytes()).hexdigest() == trace['source_sha256'], 'English source changed'
    ru = ROOT/'Routira_Project_Specification_RU.md'
    assert hashlib.sha256(ru.read_bytes()).hexdigest() == '25fb27b433fc6b2e72ccf0392a731721be0505e562e539bb93cb8fb20debee46', 'Russian source changed'


def validate_source_specs(trace):
    validate_source_hashes(trace)
    source = ROOT/'Routira_Project_Specification_EN.md'
    headings = re.findall(r'^###? (\d+(?:\.\d+)?)\. ', source.read_text(encoding='utf-8'), flags=re.M)
    mapped = {item['section'] for item in trace['requirements']}
    assert set(headings) <= mapped, 'A spec heading was not considered'
    assert {str(i) for i in range(1,28)} <= mapped, 'A top-level spec section was omitted'
    return headings


def validate_links():
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
    return link_count


def validate_adr(adr):
    content = adr.read_text(encoding='utf-8')
    expected = 'Accepted' if adr.name.startswith(('ADR-0016-', 'ADR-0017-')) else 'Proposed'
    assert f'Status: **{expected}**' in content
    for section in ('Context','Decision','Alternatives','Reasons','Consequences','Risks','Validation approach'):
        assert f'## {section}' in content, f'{adr.name}: {section}'


def validate_adrs():
    adrs = list((ROOT/'docs/architecture/adr').glob('ADR-*.md'))
    assert len(adrs) == 17
    assert {p.name.split('-')[1] for p in adrs} == {f'{n:04}' for n in range(1,18)}
    for adr in adrs:
        validate_adr(adr)
    return adrs


def validate_review_boundary(plan):
    assert plan['foundation_review']['status'] == 'accepted', 'Missing owner acceptance'
    assert plan['foundation_review']['source'], 'Missing owner decision evidence'
    assert plan['execution_authorization']['through_task'] == 'P1-01', 'Unexpected execution boundary'
    assert plan['execution_authorization']['source'], 'Missing stage authorization'
    by_id = {task['id']: task for task in plan['tasks']}
    assert by_id['P0-01']['status'] == 'completed', 'Foundation gate is still open'
    assert by_id['P0-02']['status'] == 'completed', 'Workspace gate is still open'
    assert by_id['P0-05']['status'] == 'completed', 'Dependency qualification gate is still open'
    assert all(task['status'] == 'planned' for task in plan['tasks'] if task['id'] not in {'P0-01', 'P0-02', 'P0-03', 'P0-04', 'P0-05', 'P1-01'}), 'Work exceeds P1-01'
    assert plan['network_lab_decision']['source'], 'Missing lab authorization'
    assert by_id['P0-03']['status'] == 'completed', 'Native Debian 13 fixture gate is not complete'
    native = plan['network_lab_decision'].get('native_acceptance', {})
    assert native.get('status') == 'passed' and native.get('source') and native.get('report_sha256'), 'Missing native Debian 13 evidence or authorization'
    assert (ROOT/'Cargo.toml').is_file() and (ROOT/'Cargo.lock').is_file(), 'Missing workspace'
    assert {p.name for p in (ROOT/'crates').iterdir()} == {'routira-common', 'routira-config', 'routira-policy'}, 'Unexpected crate inventory'
    assert not (ROOT/'apps').exists(), 'Runtime/frontend is outside P1-01'
    agents = (ROOT/'AGENTS.md').read_text(encoding='utf-8')
    assert 'mcp__node_repl__js' in agents and 'sky.list_apps()' in agents and 'browser-only' in agents


def main():
    plan = json.loads((ROOT/'docs/planning/backlog.json').read_text(encoding='utf-8'))
    ids = validate_tasks(plan['tasks'])
    counts, release = validate_release(plan['tasks'])
    trace = json.loads((ROOT/'docs/planning/traceability.json').read_text(encoding='utf-8'))
    validate_traceability(plan, trace, ids)
    headings = validate_source_specs(trace)
    link_count = validate_links()
    adrs = validate_adrs()
    validate_review_boundary(plan)
    negatives = self_test(plan['tasks']) if '--self-test' in sys.argv else 0
    result = {'status':'passed','scope_amendment':'CR-0001','accepted_design':'CR-0002','execution_boundary':'P1-01','tasks':len(ids),'deliveries':counts,'mvp_release_dependency_closure':len(release),'requirements':len(trace['requirements']),'source_headings':len(headings),'local_links_checked':link_count,'adrs':len(adrs),'accepted_adrs':2,'negative_validator_cases':negatives,'source_specs_unchanged':True,'workspace_crates':3,'runtime_implementation_absent':True}
    (ROOT/'docs/environment').mkdir(parents=True, exist_ok=True)
    (ROOT/'docs/environment/planning-verification.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
