"""Local YAML and authorization checks; never sends files to external services."""
from pathlib import Path
import copy
import json
import re
import yaml

ROOT = Path(__file__).resolve().parents[1]


def check_workflow(flow):
    assert flow['permissions'] == {'contents':'read'}
    jobs = flow['jobs']
    for name in ('backend','frontend'):
        assert 'permissions' not in jobs[name] or 'id-token' not in jobs[name]['permissions']
    upload = jobs['upload']
    assert 'head.repo.full_name == github.repository' in upload['if']
    assert upload['permissions']['id-token'] == 'write'
    assert not any('run' in step for step in upload['steps']), 'Privileged upload job executes product scripts'
    for job in jobs.values():
        assert job['timeout-minutes'] > 0
        for step in job['steps']:
            if 'uses' in step:
                assert re.search(r'@[0-9a-f]{40}$', step['uses']), 'Unpinned action'
            if step.get('uses','').startswith('codecov/'):
                assert step['with']['files']
                assert step['with']['disable_search'] is True
                assert step['with']['fail_ci_if_error'] is True
                assert step['with']['use_oidc'] is True
                assert 'token' not in step['with']


def main():
    proposals = ROOT/'docs/quality/proposals'
    flow = yaml.safe_load((proposals/'coverage.yml.example').read_text(encoding='utf-8'))
    # YAML 1.1 parses GitHub's bare `on` as True; actionlint separately validates Actions syntax.
    assert ('on' in flow or True in flow) and 'pull_request_target' not in str(flow.get('on',flow.get(True)))
    check_workflow(flow)
    quality = yaml.safe_load((proposals/'rust-quality.yml.example').read_text(encoding='utf-8'))
    assert quality['permissions'] == {'contents':'read'}
    assert 'pull_request_target' not in str(quality.get('on',quality.get(True)))
    for job in quality['jobs'].values():
        assert 'id-token' not in job.get('permissions',{})
        for step in job['steps']:
            if 'uses' in step:
                assert re.search(r'@[0-9a-f]{40}$', step['uses'])
    config = yaml.safe_load((proposals/'codecov.yml.example').read_text(encoding='utf-8'))
    status = config['coverage']['status']
    assert status['project']['default']['informational'] is True
    assert status['project']['backend']['flags'] == ['backend']
    assert status['project']['frontend']['flags'] == ['frontend']
    critical = status['patch']['critical-policy-parsing']
    assert critical['target'] == '90%' and critical['informational'] is False and critical['paths']
    assert all(flag['carryforward'] is False for flag in config['flags'].values())
    bad = copy.deepcopy(flow); bad['jobs']['upload']['if'] = 'always()'
    try:
        check_workflow(bad)
    except AssertionError:
        pass
    else:
        raise AssertionError('Unsafe fork upload gate accepted')
    bad = copy.deepcopy(flow); bad['jobs']['backend']['permissions'] = {'id-token':'write'}
    try:
        check_workflow(bad)
    except AssertionError:
        pass
    else:
        raise AssertionError('Untrusted tests received OIDC permission')
    result = {'status':'passed','yaml_files':3,'negative_authorization_cases':2,'service_side_validation':'not performed; automatic approval review rejected external transfer','workflow_execution':'blocked by missing workspaces and account setup'}
    (ROOT/'docs/environment').mkdir(parents=True, exist_ok=True)
    (ROOT/'docs/environment/quality-proposal-verification.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
