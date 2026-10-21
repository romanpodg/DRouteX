"""Regression checks for planning output, authorization and local provenance."""
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
import copy
import hashlib
import json
import shutil
import tempfile
import unittest
from unittest.mock import patch

import build_doc_graph as graph
import capture_setup_inventory as inventory
import render_planning as renderer
import verify_planning as planning
import verify_quality_proposals as quality

ROOT = Path(__file__).resolve().parents[1]


class TemporaryProject(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)

    def write(self, relative, content):
        target = self.root / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding='utf-8')
        return target


class RendererTests(TemporaryProject):
    def test_generated_planning_matches_reviewed_documents(self):
        inputs = ('docs/planning/backlog.json', 'docs/planning/requirement-map.json',
                  'DRouteX_Project_Specification_EN.md')
        for name in inputs:
            self.write(name, (ROOT / name).read_text(encoding='utf-8'))
        with patch.object(renderer, 'ROOT', self.root), redirect_stdout(StringIO()):
            renderer.render()
        outputs = ('IMPLEMENTATION_BACKLOG.md', 'REQUIREMENTS_TRACEABILITY.md',
                   'traceability.json', 'DEPENDENCY_MAP.md')
        for name in outputs:
            relative = Path('docs/planning') / name
            self.assertEqual((self.root / relative).read_text(encoding='utf-8'),
                             (ROOT / relative).read_text(encoding='utf-8'), name)

    def test_unmapped_heading_and_changed_checklist_are_rejected(self):
        with self.assertRaisesRegex(ValueError, 'Unmapped specification heading'):
            renderer.render_section_mappings([('1', 'Unknown', 1)], {'requirements': {}}, list().append)
        with self.assertRaisesRegex(ValueError, 'list changed'):
            renderer.section_bullets(['### 1. Scope', '1. Obligation'], '1', [])
        with self.assertRaises(StopIteration):
            renderer.render_network_matrix(['### 22.3. Matrix'], [], list().append)


class ValidatorTests(unittest.TestCase):
    def test_pure_simulation_output_contract_preserves_honesty_and_redaction(self):
        folder = ROOT / 'api/schemas/explain'
        schema = json.loads((folder / 'simulation-v1.schema.json').read_text(encoding='utf-8'))
        report = json.loads((folder / 'simulation-v1.example.json').read_text(encoding='utf-8'))
        examples = json.loads((folder / 'selection-examples.json').read_text(encoding='utf-8'))
        self.assertIs(schema['properties']['network_test_performed']['const'], False)
        self.assertEqual(schema['properties']['effective_route']['type'], 'null')
        self.assertIs(schema['additionalProperties'], False)
        self.assertEqual(set(report), set(schema['required']))
        self.assertEqual(set(report), set(schema['properties']))
        self.assertIs(report['network_test_performed'], False)
        self.assertIsNone(report['effective_route'])
        definitions = schema['$defs']
        self.assertEqual(set(definitions['private_owner']['properties']), {'id', 'record_type'})
        self.assertIs(definitions['private_owner']['additionalProperties'], False)
        self.assertEqual(len(definitions['entry']['properties']['source']['oneOf']), 3)
        for value in [report['selection'], *examples.values()]:
            self.assertEqual(set(value), set(definitions['selection']['properties']))
            self.assertEqual(value['inspected_entries'], len(value['trace']))
            self.assertEqual(value['emitted_entries'], len(value['trace']))
            self.assertLessEqual(len(value['trace']), 10000)
            for entry in value['trace']:
                self.assertEqual(set(entry), set(definitions['entry']['properties']))
                self.assertNotEqual(entry['source']['kind'], 'default')
                if entry['source']['kind'] in {'filter', 'service'}:
                    self.assertRegex(entry['source']['resource_hash'], r'^[0-9a-f]{64}$')
            if value['private_owners']:
                self.assertEqual(value['private_response_precedence'], 'pending_p3')
                for owner in value['private_owners']:
                    self.assertEqual(set(owner), {'id', 'record_type'})
        self.assertEqual(examples['tcp_udp']['resolved_intent']['code'], 'unsupported_transport')
        self.assertIn('ipv6_authorization_not_established', examples['tcp_ipv4_ech_nat']['limitations'])
        self.assertIn('ipv4_authorization_not_established', examples['tcp_ipv6']['limitations'])

    def setUp(self):
        self.tasks = json.loads((ROOT / 'docs/planning/backlog.json').read_text(encoding='utf-8'))['tasks']

    def test_task_rejections_and_mvp_closure(self):
        self.assertEqual(planning.self_test(self.tasks), 9)
        counts, release = planning.validate_release(self.tasks)
        self.assertEqual(len(release), counts['MVP'])

    def test_public_dns_cannot_skip_current_admission(self):
        tasks = copy.deepcopy(self.tasks)
        next(t for t in tasks if t['id'] == 'P3-01')['prerequisites'].remove('P4-01')
        with self.assertRaisesRegex(AssertionError, 'current-admission'):
            planning.validate_tasks(tasks)

    def test_invalid_task_fields_are_rejected(self):
        cases = {'complexity': '?', 'delivery': 'UNKNOWN', 'scope_change': 'UNKNOWN', 'spec': [28], 'status': 'unknown'}
        for field, value in cases.items():
            with self.subTest(field=field), self.assertRaises(AssertionError):
                tasks = copy.deepcopy(self.tasks)
                tasks[0][field] = value
                planning.validate_tasks(tasks)

    def test_foundation_acceptance_and_execution_limit_are_required(self):
        plan = json.loads((ROOT / 'docs/planning/backlog.json').read_text(encoding='utf-8'))
        planning.validate_review_boundary(plan)
        missing_acceptance = copy.deepcopy(plan)
        missing_acceptance['foundation_review']['status'] = 'proposed'
        missing_evidence = copy.deepcopy(plan)
        missing_evidence['foundation_review']['source'] = ''
        later_work = copy.deepcopy(plan)
        next(t for t in later_work['tasks'] if t['id'] == 'P3-01')['status'] = 'in_progress'
        unfinished_dependencies = copy.deepcopy(plan)
        next(t for t in unfinished_dependencies['tasks'] if t['id'] == 'P0-05')['status'] = 'in_review'
        unfinished_identity = copy.deepcopy(plan)
        next(t for t in unfinished_identity['tasks'] if t['id'] == 'P1-01')['status'] = 'in_review'
        unfinished_configuration = copy.deepcopy(plan)
        next(t for t in unfinished_configuration['tasks'] if t['id'] == 'P1-02')['status'] = 'in_review'
        unfinished_storage = copy.deepcopy(plan)
        next(t for t in unfinished_storage['tasks'] if t['id'] == 'P1-03')['status'] = 'in_review'
        unaccepted_drafts = copy.deepcopy(plan)
        unaccepted_drafts['draft_revision_decision']['status'] = 'proposed'
        unfinished_provenance = copy.deepcopy(plan)
        next(t for t in unfinished_provenance['tasks'] if t['id'] == 'P1-04')['status'] = 'in_review'
        unaccepted_secrets = copy.deepcopy(plan)
        unaccepted_secrets['secret_storage_decision']['status'] = 'proposed'
        unfinished_secrets = copy.deepcopy(plan)
        next(t for t in unfinished_secrets['tasks'] if t['id'] == 'P1-07')['status'] = 'in_review'
        unaccepted_signature = copy.deepcopy(plan)
        unaccepted_signature['snapshot_auth_qualification_decision']['status'] = 'proposed'
        missing_signature_source = copy.deepcopy(plan)
        missing_signature_source['snapshot_auth_qualification_decision']['source'] = ''
        unaccepted_activation = copy.deepcopy(plan)
        unaccepted_activation['activation_storage_decision']['status'] = 'proposed'
        missing_activation_source = copy.deepcopy(plan)
        missing_activation_source['activation_storage_decision']['source'] = ''
        unfinished_snapshot = copy.deepcopy(plan)
        next(t for t in unfinished_snapshot['tasks'] if t['id'] == 'P1-05')['status'] = 'in_review'
        unaccepted_domain = copy.deepcopy(plan)
        unaccepted_domain['domain_normalization_decision']['status'] = 'proposed'
        missing_domain_source = copy.deepcopy(plan)
        missing_domain_source['domain_normalization_decision']['source'] = ''
        unfinished_activation = copy.deepcopy(plan)
        next(t for t in unfinished_activation['tasks'] if t['id'] == 'P1-06')['status'] = 'in_review'
        unfinished_domain = copy.deepcopy(plan)
        next(t for t in unfinished_domain['tasks'] if t['id'] == 'P2-01')['status'] = 'in_review'
        unaccepted_evaluation = copy.deepcopy(plan)
        unaccepted_evaluation['policy_evaluation_decision']['status'] = 'proposed'
        missing_evaluation_source = copy.deepcopy(plan)
        missing_evaluation_source['policy_evaluation_decision']['source'] = ''
        unfinished_evaluation = copy.deepcopy(plan)
        next(t for t in unfinished_evaluation['tasks'] if t['id'] == 'P2-02')['status'] = 'in_review'
        unaccepted_publication = copy.deepcopy(plan)
        unaccepted_publication['publication_validation_decision']['status'] = 'proposed'
        missing_publication_source = copy.deepcopy(plan)
        missing_publication_source['publication_validation_decision']['source'] = ''
        unfinished_publication = copy.deepcopy(plan)
        next(t for t in unfinished_publication['tasks'] if t['id'] == 'P2-03')['status'] = 'in_review'
        unaccepted_actions = copy.deepcopy(plan)
        unaccepted_actions['action_resolution_decision']['status'] = 'proposed'
        missing_actions_source = copy.deepcopy(plan)
        missing_actions_source['action_resolution_decision']['source'] = ''
        unfinished_actions = copy.deepcopy(plan)
        next(t for t in unfinished_actions['tasks'] if t['id'] == 'P2-04')['status'] = 'in_review'
        unaccepted_catalogs = copy.deepcopy(plan)
        unaccepted_catalogs['catalog_import_decision']['status'] = 'proposed'
        missing_catalog_source = copy.deepcopy(plan)
        missing_catalog_source['catalog_import_decision']['source'] = ''
        unfinished_catalogs = copy.deepcopy(plan)
        next(t for t in unfinished_catalogs['tasks'] if t['id'] == 'P2-07')['status'] = 'in_review'
        unaccepted_compilation = copy.deepcopy(plan)
        unaccepted_compilation['compilation_decision']['status'] = 'proposed'
        missing_compilation_source = copy.deepcopy(plan)
        missing_compilation_source['compilation_decision']['source'] = ''
        unfinished_compilation = copy.deepcopy(plan)
        next(t for t in unfinished_compilation['tasks'] if t['id'] == 'P2-05')['status'] = 'in_review'
        unaccepted_explain = copy.deepcopy(plan)
        unaccepted_explain['explain_simulation_decision']['status'] = 'proposed'
        missing_explain_source = copy.deepcopy(plan)
        missing_explain_source['explain_simulation_decision']['source'] = ''
        unfinished_explain = copy.deepcopy(plan)
        next(t for t in unfinished_explain['tasks'] if t['id'] == 'P2-06')['status'] = 'in_review'
        unaccepted_order = copy.deepcopy(plan)
        unaccepted_order['authorization_before_dns_decision']['status'] = 'proposed'
        missing_order_source = copy.deepcopy(plan)
        missing_order_source['authorization_before_dns_decision']['source'] = ''
        unaccepted_binding = copy.deepcopy(plan)
        unaccepted_binding['runtime_binding_decision']['status'] = 'proposed'
        missing_binding_source = copy.deepcopy(plan)
        missing_binding_source['runtime_binding_decision']['source'] = ''
        missing_lab = copy.deepcopy(plan)
        missing_lab['network_lab_decision']['source'] = ''
        missing_native = copy.deepcopy(plan)
        missing_native['network_lab_decision'].pop('native_acceptance')
        unapproved_native = copy.deepcopy(plan)
        unapproved_native['network_lab_decision']['native_acceptance']['source'] = ''
        unfinished_native = copy.deepcopy(plan)
        next(t for t in unfinished_native['tasks'] if t['id'] == 'P0-03')['status'] = 'in_review'
        for case in (missing_acceptance, missing_evidence, later_work, missing_lab,
                     missing_native, unapproved_native, unfinished_native, unfinished_dependencies,
                     unfinished_identity, unfinished_configuration, unfinished_storage, unaccepted_drafts,
                     unfinished_provenance, unaccepted_secrets, unfinished_secrets,
                     unaccepted_signature, missing_signature_source, unaccepted_activation,
                     missing_activation_source, unfinished_snapshot, unaccepted_domain,
                     missing_domain_source, unfinished_activation, unfinished_domain,
                     unaccepted_evaluation, missing_evaluation_source, unfinished_evaluation,
                     unaccepted_publication, missing_publication_source, unfinished_publication,
                     unaccepted_actions, missing_actions_source, unfinished_actions,
                     unaccepted_catalogs, missing_catalog_source, unfinished_catalogs,
                     unaccepted_compilation, missing_compilation_source, unfinished_compilation,
                     unaccepted_explain, missing_explain_source, unfinished_explain,
                     unaccepted_order, missing_order_source, unaccepted_binding, missing_binding_source):
            with self.subTest(case=case['foundation_review']), self.assertRaises(AssertionError):
                planning.validate_review_boundary(case)

    def test_every_upload_mutation_is_rejected(self):
        flow = quality.read_yaml(ROOT / '.github/workflows/planning.yml')
        quality.check_workflow(flow)
        cases = quality.permission_mutations(flow, 'verify') + quality.upload_step_mutations(flow)
        for index, case in enumerate(cases):
            with self.subTest(case=index), self.assertRaises(AssertionError):
                quality.check_workflow(case)
        with self.assertRaisesRegex(AssertionError, 'Unsafe workflow accepted'):
            quality.reject_unsafe_workflow(flow)

    def test_ci_rejects_write_tokens_and_private_runners(self):
        flow = quality.read_yaml(ROOT / '.github/workflows/planning.yml')
        for field, value in [('permissions', {'contents': 'write'}), ('runs-on', 'self-hosted')]:
            bad = copy.deepcopy(flow)
            bad['jobs']['backend'][field] = value
            with self.subTest(field=field), self.assertRaises(AssertionError):
                quality.check_workflow(bad)

    def test_codecov_step_guards_remain_required(self):
        flow = quality.read_yaml(ROOT / '.github/workflows/planning.yml')
        step = flow['jobs']['upload']['steps'][-1]
        for key in ('files', 'disable_search', 'fail_ci_if_error', 'use_oidc'):
            bad = copy.deepcopy(step)
            bad['with'][key] = False
            with self.subTest(key=key), self.assertRaises(AssertionError):
                quality.check_step(bad)
        bad = copy.deepcopy(step)
        bad['with']['token'] = 'test-only'
        with self.assertRaises(AssertionError):
            quality.check_step(bad)

    def test_backend_coverage_cannot_omit_a_workspace_crate(self):
        config = quality.read_yaml(ROOT / 'codecov.yml')
        quality.check_active_codecov(config)
        for path in ('crates/droutex-common/src/', 'crates/droutex-config/src/',
                     'crates/droutex-policy/src/', 'crates/droutex-storage/src/',
                     'crates/droutex-security/src/', 'crates/droutex-core/src/'):
            bad = copy.deepcopy(config)
            bad['flags']['backend']['paths'] = [
                entry for entry in bad['flags']['backend']['paths'] if entry != path]
            with self.subTest(path=path), self.assertRaises(AssertionError):
                quality.check_active_codecov(bad)


class InventoryTests(TemporaryProject):
    def setUp(self):
        super().setUp()
        self.source = self.write('source/demo/SKILL.md', 'skill').parent
        self.write('source/demo/references/example.md', 'resource')
        self.installed = self.root / '.agents/skills/demo'
        shutil.copytree(self.source, self.installed)

    def verify(self):
        with patch.object(inventory, 'ROOT', self.root):
            return inventory.verify_skill_resources(self.installed, self.source)

    def test_matching_resources_include_provenance(self):
        result = self.verify()
        self.assertEqual(result['files'], 2)
        self.assertEqual(result['skill_sha256'], hashlib.sha256(b'skill').hexdigest())
        self.assertTrue(result['all_resources_match_upstream'])

    def test_modified_resource_is_rejected(self):
        (self.installed / 'references/example.md').write_text('changed', encoding='utf-8')
        with self.assertRaisesRegex(AssertionError, 'Skill resource mismatch'):
            self.verify()

    def test_missing_and_extra_resources_are_rejected(self):
        (self.installed / 'references/example.md').unlink()
        with self.assertRaises(AssertionError):
            self.verify()
        self.write('.agents/skills/demo/references/example.md', 'resource')
        self.write('.agents/skills/demo/extra.md', 'extra')
        with self.assertRaises(AssertionError):
            self.verify()

    def test_cargo_binary_checksum_mismatch_is_rejected(self):
        self.write('.tools/cargo/bin/cargo-deny.exe', 'binary')
        expected = hashlib.sha256(b'binary').hexdigest()
        self.write('.setup/tool-downloads.json', json.dumps([{'tool': 'deny', 'binary_sha256': expected}]))
        with patch.object(inventory, 'ROOT', self.root):
            self.assertEqual(len(inventory.verify_cargo_binaries()), 1)
            self.write('.tools/cargo/bin/cargo-deny.exe', 'modified')
            with self.assertRaises(AssertionError):
                inventory.verify_cargo_binaries()


class GraphTests(TemporaryProject):
    def imported_node(self, source='spec.md', label='Scope'):
        return {'source_file': source, 'label': label,
                'droutex_origin': 'deterministic-document-import'}

    def test_heading_relocation_is_verified_but_arbitrary_deletion_is_not(self):
        node = self.imported_node()
        self.assertTrue(graph.verified_heading_refresh([node], {('spec.md', 'Scope')}, []))
        self.assertFalse(graph.verified_heading_refresh([node], set(), []))
        self.assertFalse(graph.verified_heading_refresh([], set(), []))

    def test_duplicate_task_heading_requires_imported_origin(self):
        node = self.imported_node('docs/planning/IMPLEMENTATION_BACKLOG.md', 'P0-01: Contract')
        duplicates = graph.duplicate_task_headings([node])
        self.assertEqual(duplicates, [node])
        self.assertTrue(graph.verified_heading_refresh([node], set(), duplicates))
        node['droutex_origin'] = 'native-ast'
        self.assertFalse(graph.verified_heading_refresh([node], set(), duplicates))

    def test_explicit_refresh_preserves_native_and_unscanned_nodes(self):
        node = self.imported_node()
        with patch.object(graph, 'ROOT', self.root):
            files = {(self.root / 'spec.md').resolve(): 'spec'}
            self.assertTrue(graph.explicit_planning_refresh([node], files, True))
            self.assertFalse(graph.explicit_planning_refresh([node], files, False))
            self.assertFalse(graph.explicit_planning_refresh([node], {}, True))
            self.assertFalse(graph.explicit_planning_refresh([], files, True))
            node['droutex_origin'] = 'native-ast'
            self.assertFalse(graph.explicit_planning_refresh([node], files, True))

    def test_document_import_preserves_explicit_links_and_task_identity(self):
        self.write('spec.md', '# Scope\n[Local](other%20doc.md#part) [Web](https://example.com) [Missing](absent.md) [Anchor](#scope)\n')
        self.write('other doc.md', '# Other\n')
        self.write('docs/planning/IMPLEMENTATION_BACKLOG.md', '# Tasks\n### P0-01: Contract\n')
        tasks = [{'id': 'P0-01', 'title': 'Contract', 'delivery': 'MVP', 'prerequisites': []}]
        self.write('docs/planning/backlog.json', json.dumps({'tasks': tasks}))
        requirement = {'id': 'R-1', 'title': 'Scope', 'source_file': 'spec.md',
                       'line': 1, 'delivery': ['MVP'], 'tasks': ['P0-01']}
        self.write('docs/planning/traceability.json', json.dumps({'requirements': [requirement]}))
        with patch.object(graph, 'ROOT', self.root):
            data, files = graph.document_extraction(graph.document_files())
        self.assertEqual(len([n for n in data['nodes'] if n['label'].startswith('P0-01')]), 1)
        references = [e for e in data['edges'] if e['relation'] == 'references']
        self.assertEqual([(e['source'], e['target']) for e in references], [('spec_md', 'other_doc_md')])
        self.assertEqual({e['confidence'] for e in data['edges']}, {'EXTRACTED'})
        self.assertIn((self.root / 'spec.md').resolve(), files)


if __name__ == '__main__':
    unittest.main()
