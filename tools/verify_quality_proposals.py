"""Local YAML and authorization checks; never sends files to external services."""
from pathlib import Path
import copy
import json
import re
import yaml

ROOT = Path(__file__).resolve().parents[1]


def check_step(step):
    if 'uses' in step:
        assert re.search(r'@[0-9a-f]{40}$', step['uses']), 'Unpinned action'
    if step.get('uses','').startswith('codecov/'):
        assert step['with']['files']
        assert step['with']['disable_search'] is True
        assert step['with']['fail_ci_if_error'] is True
        assert step['with']['use_oidc'] is True
        assert 'token' not in step['with']


def check_job(job):
    assert job['timeout-minutes'] > 0
    for step in job['steps']:
        check_step(step)


def check_workflow(flow):
    assert flow['permissions'] == {'contents':'read'}
    jobs = flow['jobs']
    for name, job in jobs.items():
        if name != 'upload':
            assert 'id-token' not in job.get('permissions', {})
    upload = jobs['upload']
    assert 'head.repo.full_name == github.repository' in upload['if']
    assert upload['permissions']['id-token'] == 'write'
    assert not any('run' in step for step in upload['steps']), 'Privileged upload job executes product scripts'
    for job in jobs.values():
        check_job(job)


def check_active_codecov(active_config):
    for kind in ('project', 'patch'):
        statuses = active_config['coverage']['status'][kind]
        assert statuses['default'] is False
        assert statuses['planning']['flags'] == ['planning']
        assert statuses['planning']['informational'] is True
    assert active_config['flags']['planning']['carryforward'] is False


def check_proposed_quality(quality):
    assert quality['permissions'] == {'contents':'read'}
    assert 'pull_request_target' not in str(quality.get('on',quality.get(True)))
    for job in quality['jobs'].values():
        assert 'id-token' not in job.get('permissions',{})
        for step in job['steps']:
            if 'uses' in step:
                assert re.search(r'@[0-9a-f]{40}$', step['uses'])


def check_proposed_codecov(config):
    status = config['coverage']['status']
    assert status['project']['default']['informational'] is True
    assert status['project']['backend']['flags'] == ['backend']
    assert status['project']['frontend']['flags'] == ['frontend']
    critical = status['patch']['critical-policy-parsing']
    assert critical['target'] == '90%' and critical['informational'] is False and critical['paths']
    assert all(flag['carryforward'] is False for flag in config['flags'].values())


def permission_mutations(flow, test_job):
    fork = copy.deepcopy(flow)
    fork['jobs']['upload']['if'] = 'always()'
    oidc = copy.deepcopy(flow)
    oidc['jobs'][test_job]['permissions'] = {'id-token': 'write'}
    return [fork, oidc]


def upload_step_mutations(flow):
    script = copy.deepcopy(flow)
    script['jobs']['upload']['steps'].append({'run': 'python tools/render_planning.py'})
    action = copy.deepcopy(flow)
    action['jobs']['upload']['steps'][0]['uses'] = 'actions/checkout@main'
    return [script, action]


def reject_unsafe_workflow(flow):
    try:
        check_workflow(flow)
    except AssertionError:
        return
    raise AssertionError('Unsafe workflow accepted')


def check_negative_authorization(flow, active):
    cases = permission_mutations(flow, 'backend')
    cases += permission_mutations(active, 'verify')
    cases += upload_step_mutations(active)
    for case in cases:
        reject_unsafe_workflow(case)


def read_yaml(path):
    return yaml.safe_load(path.read_text(encoding='utf-8'))


def main():
    proposals = ROOT/'docs/quality/proposals'
    flow = read_yaml(proposals/'coverage.yml.example')
    # YAML 1.1 parses GitHub's bare `on` as True; actionlint checks Actions syntax.
    assert ('on' in flow or True in flow) and 'pull_request_target' not in str(flow.get('on', flow.get(True)))
    check_workflow(flow)
    active = read_yaml(ROOT/'.github/workflows/planning.yml')
    assert 'pull_request_target' not in str(active.get('on', active.get(True)))
    check_workflow(active)
    check_active_codecov(read_yaml(ROOT/'codecov.yml'))
    check_proposed_quality(read_yaml(proposals/'rust-quality.yml.example'))
    check_proposed_codecov(read_yaml(proposals/'codecov.yml.example'))
    check_negative_authorization(flow, active)
    result = {'status':'passed','yaml_files':5,'negative_authorization_cases':6,'service_side_validation':'not performed by this local validator','workflow_execution':'local configuration checks only; remote runs recorded in integration documentation'}
    (ROOT/'docs/environment').mkdir(parents=True, exist_ok=True)
    (ROOT/'docs/environment/quality-proposal-verification.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
