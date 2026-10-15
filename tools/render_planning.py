"""Render reviewable plans; preserve base requirements and show accepted amendments."""
from pathlib import Path
import json
import re

ROOT = Path(__file__).resolve().parents[1]
SCOPE_ADR = 'docs/architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md'
DESIGN_ADR = 'docs/architecture/adr/ADR-0017-profile-endpoint-dns-product-model.md'
FENCE = chr(96) * 3


def render_backlog(plan):
    tasks = plan['tasks']
    text = ['# Implementation backlog', '',
            f"**{plan['status']}.**", '',
            'Accepted scope: CR-0001, DNS/proxy MVP before tunnels. See [ADR-0016](../architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md).', '',
            'Proposed CR-0002: [Profile/Endpoint DNS model](../architecture/DNS_PRODUCT_MODEL.md), including personal DoH/DoT and versioned filter/service catalogs. Technical acceptance remains P0-01.', '',
            'Canonical status/delivery source: [backlog.json](backlog.json). Regenerate with python3.12 tools/render_planning.py.', '',
            'Workstream IDs are stable, not chronological release order. Delivery tags govern release applicability.', '',
            'S/M/L indicates relative complexity, not elapsed time. Quality bundles and full Definition of Done: [QUALITY_STRATEGY](../quality/QUALITY_STRATEGY.md).', '']
    for phase, name in plan['phases'].items():
        members = [task for task in tasks if task['phase'] == phase]
        text += [f'## {phase}: {name} ({len(members)} tasks)', '']
        for task in members:
            text += [f"### {task['id']}: {task['title']}", '',
                     f"Status: **{task['status']}**. Delivery: **{task['delivery']}** (CR-0001). Complexity: **{task['complexity']}**. Specification: §§{','.join(map(str, task['spec']))}.", '',
                     '| Field | Deliverable / evidence |', '|---|---|',
                     f"| Goal and implementation scope | {task['scope']} |",
                     f"| Prerequisites | {', '.join(task['prerequisites']) or 'Explicit roadmap approval and registered product decisions'} |",
                     f"| Expected files/modules | {'; '.join(task['files'])} |",
                     f"| Acceptance criteria | {task['acceptance']} |",
                     f"| Tests | {task['tests']} |",
                     f"| Security considerations | {task['security']} |",
                     f"| Quality checks | {task['quality']} |",
                     f"| Definition of Done | {task['definition_of_done']} |", '']
    (ROOT / 'docs/planning/IMPLEMENTATION_BACKLOG.md').write_text('\n'.join(text), encoding='utf-8')


def render_section_mappings(headings, mappings, add):
    for section, title, lineno in headings:
        item = mappings['requirements'].get(section)
        if item is None:
            children = [v for k, v in mappings['requirements'].items() if k.startswith(section + '.')]
            if not children:
                raise ValueError(f'Unmapped specification heading {section}')
            item = {'tasks': sorted({t for v in children for t in v['tasks']}),
                    'requirement': f'{title}: all mapped child obligations below',
                    'kind': 'Section rollup', 'scope': 'See child requirements'}
        add({'id': 'R-' + section, 'section': section, 'line': lineno, 'title': title, **item})


def section_bullets(spec_lines, section, ids):
    start = next(i for i, line in enumerate(spec_lines) if re.match(rf'^### {re.escape(section)}\. ', line))
    end = next((i for i in range(start + 1, len(spec_lines)) if spec_lines[i].startswith(('### ', '## '))), len(spec_lines))
    bullets = [(i + 1, re.sub(r'^(?:\d+\. |\- \[ \] )', '', spec_lines[i]))
               for i in range(start + 1, end) if re.match(r'^(?:\d+\. |\- \[ \] )', spec_lines[i])]
    if len(bullets) != len(ids):
        raise ValueError(f'Specification {section} list changed: review detailed mapping')
    return bullets


def checklist_scope(section, index, task_id):
    scope, work = 'MVP', [task_id, 'P7-07', 'P10-07']
    if section == '24.1' and 7 <= index <= 10:
        scope, work = 'Post-MVP protocols under CR-0001; original checkbox pending', [task_id, 'P5-10']
    elif section == '19.2' and index == 10:
        scope, work = 'MVP denies OS network actions; custom helper before privileged G3 features', ['P10-02', 'P11-01', 'P10-01']
    return scope, work


def render_checklists(spec_lines, trace, add):
    detailed_groups = [
        ('19.2', 'Baseline security restrictions', ['P3-01','P4-01','P4-01','P4-02','P4-02','P10-04','P6-01','P6-01','P1-07','P10-01','P1-05','P11-05']),
        ('24.1', 'Original MVP checklist with amended applicability', ['P11-01','P3-01','P7-02','P3-04','P7-03','P1-06','P5-04','P5-05','P5-06','P5-08','P1-07','P11-03','P10-05','P7-05']),
    ]
    for section, title, ids in detailed_groups:
        bullets = section_bullets(spec_lines, section, ids)
        trace += ['', f'## {title}', '',
                  '| ID | Source obligation (unchanged) | Kind | Effective scope | Task deliveries | Work items |',
                  '|---|---|---|---|---|---|']
        for index, ((line, obligation), task_id) in enumerate(zip(bullets, ids), 1):
            scope, work = checklist_scope(section, index, task_id)
            add({'id': f'R-{section}-{index:02}', 'section': section, 'line': line,
                 'title': obligation, 'requirement': obligation, 'kind': 'Explicit base requirement',
                 'scope': scope, 'tasks': work, 'scope_change': 'CR-0001'})


def network_scope(index):
    scope, work = 'MVP supported DNS/proxy boundary; DNS-only endpoint limits explicit', ['P10-05', 'P7-07']
    if index == 9:
        scope, work = 'MVP proxy/component failure; remote node outage at G4', ['P4-06', 'P10-06', 'P9-02']
    elif index == 10:
        scope, work = 'Post-MVP split-tunnel clients under CR-0001', ['P5-04', 'P5-10', 'F-02']
    elif index == 11:
        scope, work = 'MVP DNS/proxy access revocation; peer credential tests at G3', ['P6-03', 'P4-08', 'P5-07', 'P5-10']
    return scope, work


def render_network_matrix(spec_lines, trace, add):
    start = next(i for i, line in enumerate(spec_lines) if line.startswith('### 22.3.'))
    end = next(i for i in range(start + 1, len(spec_lines)) if spec_lines[i].startswith('### '))
    cases = [(i + 1, spec_lines[i]) for i in range(start, end) if spec_lines[i].startswith('|')][2:]
    trace += ['', '## Network limitation acceptance matrix', '',
              '| ID | Source case and expected result | Kind | Effective scope | Task deliveries | Work items |',
              '|---|---|---|---|---|---|']
    for index, (line, case) in enumerate(cases, 1):
        obligation = ' → '.join(cell.strip() for cell in case.strip('|').split('|'))
        scope, work = network_scope(index)
        add({'id': f'R-22.3-{index:02}', 'section': '22.3', 'line': line, 'title': obligation,
             'requirement': obligation, 'kind': 'Explicit base requirement', 'scope': scope,
             'tasks': work, 'scope_change': 'CR-0001'})


def render_amendments(trace, add):
    trace += ['', '## Accepted user scope amendment', '',
              '| ID | Accepted instruction | Kind | Effective scope | Task deliveries | Work items |',
              '|---|---|---|---|---|---|']
    add({'id': 'R-CR-0001', 'section': 'scope-change', 'line': 1, 'source_file': SCOPE_ADR,
         'title': 'DNS/proxy MVP before tunnels',
         'requirement': 'Release DNS/proxy MVP without VPNs, managed engines, TUN or a custom privileged networking helper; retain real later protocol acceptance',
         'kind': 'Accepted user instruction', 'scope': 'Overrides first-release ordering in §§23.2/24.1/24.2; no implementation approval',
         'tasks': ['P4-08', 'P7-07', 'P11-06', 'P5-10'], 'scope_change': 'CR-0001'})
    trace += ['', '## Proposed Control D reference design', '',
              '| ID | Proposed design | Kind | Effective scope | Task deliveries | Work items |',
              '|---|---|---|---|---|---|']
    add({'id': 'R-CR-0002', 'section': 'design-proposal', 'line': 1, 'source_file': DESIGN_ADR,
         'title': 'Profile and endpoint DNS product model',
         'requirement': 'Separate Profiles, DNS Endpoints, resolver capabilities, filter/service revisions, private records and gateway grants; personal DoH/DoT in revised MVP',
         'kind': 'Proposed design', 'scope': 'Authorized research and plan edits; technical acceptance pending P0-01',
         'tasks': ['P1-01', 'P2-07', 'P2-02', 'P3-07', 'P4-01', 'P6-03', 'P7-02', 'P7-04'],
         'scope_change': 'CR-0002'})


def render_traceability(mappings, by_id):
    spec_lines = (ROOT / mappings['source']).read_text(encoding='utf-8').splitlines()
    headings = []
    for number, line in enumerate(spec_lines, 1):
        match = re.match(r'^(##|###) (\d+(?:\.\d+)?)\. (.+)$', line)
        if match:
            headings.append((match[2], match[3], number))
    trace = ['# Requirements traceability', '', '**Proposed implementation plan, not product test evidence.**', '',
             f"Base source: [{mappings['source']}](../../{mappings['source']}); SHA-256 {mappings['source_sha256']}.", '',
             'Accepted CR-0001: [ADR-0016](../architecture/adr/ADR-0016-dns-proxy-mvp-before-tunnels.md). Original source wording is preserved; effective release applicability is explicit below.', '',
             'All numbered headings/subsections were considered. Parent rows aggregate children. Delivery tags derive from mapped tasks and may span releases; they do not mark deferred requirements complete.', '',
             '| ID / source section | Requirement or planned interpretation | Kind | Effective scope | Task deliveries | Work items |',
             '|---|---|---|---|---|---|']
    trace_records = []

    def add(record):
        record['delivery'] = sorted({by_id[t]['delivery'] for t in record['tasks']})
        record.setdefault('source_file', mappings['source'])
        trace_records.append(record)
        trace.append(f"| {record['id']} | {record['requirement']} | {record['kind']} | {record['scope']} | {', '.join(record['delivery'])} | {', '.join(record['tasks'])} |")

    render_section_mappings(headings, mappings, add)
    render_checklists(spec_lines, trace, add)
    render_network_matrix(spec_lines, trace, add)
    render_amendments(trace, add)
    trace += ['', f'Total mapping records (including rollups, checklist details, accepted amendment and proposed design): **{len(trace_records)}**.', '',
              'Task status is recorded in backlog.json. A mapping or deferral is neither implementation nor a passed acceptance test.', '']
    (ROOT / 'docs/planning/REQUIREMENTS_TRACEABILITY.md').write_text('\n'.join(trace), encoding='utf-8')
    (ROOT / 'docs/planning/traceability.json').write_text(json.dumps(
        {'source_sha256': mappings['source_sha256'], 'scope_amendment': 'CR-0001', 'requirements': trace_records}, indent=2), encoding='utf-8')

    return trace_records


def render_dependencies(plan):
    tasks = plan['tasks']
    by_id = {task['id']: task for task in tasks}
    dependency_doc = ['# Dependency map', '', f"Status: {plan['status']}. Edges are planned prerequisites, not implemented crate dependencies.", '',
                      '## Current release chain', '',
                      'P0 contracts/DNS-proxy lab/inventory → P1 Profiles/Endpoints/storage/secrets/snapshots → P2 catalogs/layered policy → P3 legacy + personal secure DNS and P4 separate gateway grants/origin/TCP/proxy → P4-08 real DNS/proxy slice → P6/P7 administration → P10/P11 security/recovery/install → P7-07/P11-06 MVP.', '',
                      'Authentication and security begin early. No MVP task depends on P5 or P10-01, directly or transitively. G3 protocols/helper start after P11-06; G4 agents/groups follow P5-10.', '',
                      '## Workstream flow', '', FENCE + 'mermaid', 'flowchart LR',
                      '  P0[Approval and DNS/proxy foundation] --> P1[Domain and snapshots]',
                      '  P1 --> POL[Pure policy]', '  P1 --> AUTH[Early authentication]',
                      '  POL --> BIND[Ingress and destination binding]', '  BIND --> DNS[DNS redirection]',
                      '  BIND --> GW[TCP and protected proxy outbound]',
                      '  DNS --> SLICE[Actual DNS/proxy slice]', '  GW --> SLICE',
                      '  AUTH --> UI[Private API and UI]', '  SLICE --> MVP[Security restore packaging MVP]',
                      '  UI --> MVP', '  MVP --> PROTO[Later engines tunnels helper and exports]',
                      '  PROTO --> MULTI[Later remote agents and multi-node]',
                      '  MULTI --> FUT[Approved future features]', FENCE, '',
                      '## Exact task dependencies', '', '| Task | Delivery | Prerequisites |', '|---|---|---|']
    for task in tasks:
        dependency_doc.append(f"| {task['id']} | {task['delivery']} | {', '.join(task['prerequisites']) or 'Roadmap approval'} |")
    memo = {}

    def chain(task_id):
        if task_id not in memo:
            prior = max((chain(p) for p in by_id[task_id]['prerequisites']), key=len, default=[])
            memo[task_id] = prior + [task_id]
        return memo[task_id]

    mvp_longest = max((chain(t['id']) for t in tasks if t['delivery'] == 'MVP'), key=len)
    longest = max((chain(t['id']) for t in tasks), key=len)
    dependency_doc += ['', '## Computed longest MVP prerequisite chain', '', ' → '.join(mvp_longest), '',
                       '## Computed longest full-plan prerequisite chain', '', ' → '.join(longest), '',
                       'These count tasks, not elapsed time. Stable workstream numbering does not dictate execution order.', '',
                       'External MVP dependencies: supported Linux target, restricted ingress binding, protected proxy/reference services, CodeScene baseline and GitHub/Codecov activation. None implies a completed product test. Engines/clients/kernel bridges are G3 evidence, not MVP prerequisites.', '']
    (ROOT / 'docs/planning/DEPENDENCY_MAP.md').write_text('\n'.join(dependency_doc), encoding='utf-8')
    return mvp_longest, longest


def render() -> None:
    plan = json.loads((ROOT / 'docs/planning/backlog.json').read_text(encoding='utf-8'))
    mappings = json.loads((ROOT / 'docs/planning/requirement-map.json').read_text(encoding='utf-8'))
    tasks = plan['tasks']
    by_id = {task['id']: task for task in tasks}
    render_backlog(plan)
    trace_records = render_traceability(mappings, by_id)
    mvp_longest, longest = render_dependencies(plan)
    print(json.dumps({'tasks': len(tasks), 'requirements': len(trace_records),
                      'deliveries': {d: sum(t['delivery'] == d for t in tasks) for d in sorted({t['delivery'] for t in tasks})},
                      'longest_mvp_chain_tasks': len(mvp_longest), 'longest_chain_tasks': len(longest)}))


if __name__ == '__main__':
    render()
