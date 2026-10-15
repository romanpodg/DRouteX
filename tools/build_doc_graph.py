"""Import explicit planning relationships into Graphify without a cloud/LLM pass.

Requires the isolated graphifyy environment. This is a document/citation/task
baseline, not a claim that proposed runtime components are implemented.
"""
from pathlib import Path
import importlib.metadata
import json
import re
import sys
from urllib.parse import unquote
from graphify.build import build
from graphify.export import to_json
from graphify.validate import validate_extraction

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT/'graphify-out'


def nid(path):
    return re.sub(r'[^a-z0-9_]', '_', str(path).lower().replace('\\','/'))


def main():
    OUT.mkdir(exist_ok=True)
    ast_file = OUT/'ast-baseline.json'
    if not ast_file.exists():
        ast_file.write_text((OUT/'graph.json').read_text(encoding='utf-8'), encoding='utf-8')
    ast = json.loads(ast_file.read_text(encoding='utf-8'))
    files = sorted([*ROOT.glob('*.md'), *ROOT.glob('*.toml'), *ROOT.glob('*.json'), *ROOT.glob('.gitignore'), *ROOT.glob('.graphifyignore'), *(ROOT/'docs').rglob('*'), *(ROOT/'tools').rglob('*')])
    files = [p for p in files if p.is_file() and '__pycache__' not in p.parts]
    nodes, edges = {}, []
    def node(key, label, file, line=1, kind='document'):
        nodes[key] = {'id':key, 'label':label, 'file_type':kind, 'source_file':str(file.resolve()), 'source_location':f'L{line}', '_origin':'semantic', 'routira_origin':'deterministic-document-import', 'routira_state':'planning only'}
    def edge(source, target, relation, file, line=1):
        edges.append({'source':source,'target':target,'relation':relation,'confidence':'EXTRACTED','confidence_score':1.0,'source_file':str(file.resolve()),'source_location':f'L{line}','weight':1.0,'_origin':'semantic','rationale':'Explicit document citation or proposed task prerequisite; no implemented runtime dependency claim'})
    file_ids = {}
    for file in files:
        key = nid(file.relative_to(ROOT))
        file_ids[file.resolve()] = key
        node(key, str(file.relative_to(ROOT)).replace('\\','/'), file)
    for file in files:
        if file.suffix != '.md':
            continue
        key = file_ids[file.resolve()]
        lines = file.read_text(encoding='utf-8').splitlines()
        for lineno, line in enumerate(lines, 1):
            heading = re.match(r'^#{1,4} (.+)$', line)
            # Task identities already have a canonical JSON node; don't mint
            # duplicate Markdown task labels that make CLI lookup ambiguous.
            is_task_heading = file.name == 'IMPLEMENTATION_BACKLOG.md' and re.match(r'^[PF]\d*-\d+:', heading[1] if heading else '')
            if heading and not is_task_heading:
                heading_id = key + '_heading_' + str(lineno)
                node(heading_id, heading[1], file, lineno, 'concept')
                edge(key, heading_id, 'contains', file, lineno)
            for target in re.findall(r'\]\(([^)]+)\)', line):
                target = target.strip('<> ').split('#')[0]
                if not target or re.match(r'^[a-zA-Z]+:', target):
                    continue
                resolved = (file.parent/unquote(target)).resolve()
                if resolved in file_ids:
                    edge(key, file_ids[resolved], 'references', file, lineno)
    backlog_file = ROOT/'docs/planning/backlog.json'
    tasks = json.loads(backlog_file.read_text(encoding='utf-8'))['tasks']
    for task in tasks:
        node('task_'+nid(task['id']), task['id']+' '+task['title'], backlog_file, kind='concept')
        nodes['task_'+nid(task['id'])]['routira_delivery'] = task['delivery']
    for task in tasks:
        key = 'task_'+nid(task['id'])
        edge(file_ids[backlog_file.resolve()], key, 'contains', backlog_file)
        for prior in task['prerequisites']:
            edge(key, 'task_'+nid(prior), 'planned_prerequisite', backlog_file)
    trace_file = ROOT/'docs/planning/traceability.json'
    trace = json.loads(trace_file.read_text(encoding='utf-8'))['requirements']
    for item in trace:
        spec_file = ROOT/item['source_file']
        key = 'requirement_'+nid(item['id'])
        node(key, item['id']+' '+item['title'], spec_file, item['line'], 'concept')
        nodes[key]['routira_delivery'] = ','.join(item['delivery'])
        edge(file_ids[spec_file.resolve()], key, 'contains', spec_file, item['line'])
        for task_id in item['tasks']:
            edge(key, 'task_'+nid(task_id), 'planned_work_item', trace_file)
    document_data = {'nodes':list(nodes.values()), 'edges':edges, 'hyperedges':[], 'input_tokens':0, 'output_tokens':0}
    errors = validate_extraction(document_data)
    if errors:
        raise ValueError(errors)
    G = build([ast, document_data], directed=True, dedup=False, root=ROOT)
    G.graph.update({'routira_baseline':'Explicit document headings/citations, proposed task dependencies and native AST of planning tools', 'graphify_version':importlib.metadata.version('graphifyy'), 'routira_external_api_calls':0, 'routira_document_files':len(files)})
    old = json.loads((OUT/'graph.json').read_text(encoding='utf-8'))
    dropped = [n for n in old['nodes'] if n['id'] not in G]
    (OUT/'DOCUMENT_REFRESH_DROPS.json').write_text(json.dumps(dropped,indent=2),encoding='utf-8')
    duplicate_tasks = [n for n in dropped if (
        n.get('source_file') == 'docs/planning/IMPLEMENTATION_BACKLOG.md'
        and re.match(r'^[PF]\d*-\d+:', n.get('label','')))]
    new_locations = {(n.get('source_file'),n.get('label')) for _, n in G.nodes(data=True)}
    verified_refresh = bool(dropped) and all(
        n.get('routira_origin') == 'deterministic-document-import' and
        (n in duplicate_tasks or (n.get('source_file'),n.get('label')) in new_locations)
        for n in dropped)
    # An explicit planning refresh may replace obsolete imported headings, but
    # never discard native/external nodes or relationships from other sources.
    planning_refresh = '--refresh-planning' in sys.argv and bool(dropped) and all(
        n.get('routira_origin') == 'deterministic-document-import' and
        (ROOT/n.get('source_file', '')).resolve() in file_ids
        for n in dropped)
    # Every removed node must be a proved duplicate task or the same document
    # heading retained at its new source line. Any other shrink remains refused.
    if not to_json(G, {}, str(OUT/'graph.json'), force=verified_refresh or planning_refresh):
        raise RuntimeError('Graphify refused graph write')
    summary = {'nodes':G.number_of_nodes(), 'edges':G.number_of_edges(), 'files':len(files), 'document_nodes':len(nodes), 'document_edges':len(edges), 'verified_duplicate_task_headings_removed':len(duplicate_tasks) if verified_refresh else 0, 'verified_relocated_headings':len(dropped)-len(duplicate_tasks) if verified_refresh else 0, 'explicit_planning_refresh_nodes':len(dropped) if planning_refresh else 0, 'external_api_calls':0, 'document_inference':'none; explicit relationships only', 'runtime_implementation':'absent'}
    (OUT/'BASELINE_EVIDENCE.json').write_text(json.dumps(summary,indent=2), encoding='utf-8')
    print(json.dumps(summary))


if __name__ == '__main__':
    main()
