"""Harness regression tests. These results do not certify the macOS product."""
import copy
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('l0_verify', ROOT / 'script/l0_verify.py')
verify = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verify)


class CatalogTests(unittest.TestCase):
    def setUp(self):
        self.data = verify.load_json(ROOT / verify.CATALOG)

    def test_all_66_original_scenarios_and_actual_source_names(self):
        self.assertEqual(verify.validate_catalog(self.data)['scenarios'], 66)

    def test_missing_scenario_fails(self):
        self.data['cases'].pop()
        with self.assertRaises(ValueError):
            verify.validate_catalog(self.data)

    def test_duplicate_scenario_fails(self):
        self.data['cases'][1] = self.data['cases'][0]
        with self.assertRaises(ValueError):
            verify.validate_catalog(self.data)

    def test_unknown_scenario_fails(self):
        self.data['cases'][0]['id'] = 'L0-99-T99'
        with self.assertRaises(ValueError):
            verify.validate_catalog(self.data)

    def test_missing_function_fails(self):
        self.data['cases'][0]['automated']['references'][0]['function'] = 'not_a_real_test'
        with self.assertRaises(ValueError):
            verify.validate_catalog(self.data)

    def test_missing_serial_reverse_mapping_fails(self):
        self.data['cases'][0]['serial_workflows'] = ['CUA-01']
        with self.assertRaises(ValueError):
            verify.validate_catalog(self.data)

    def test_candidate_identity_workflow_cannot_be_removed(self):
        self.data['serial_workflows'] = [w for w in self.data['serial_workflows']
                                         if w['id'] != 'CUA-01']
        with self.assertRaises(ValueError):
            verify.validate_catalog(self.data)

    def test_related_test_cannot_be_reported_as_full_coverage(self):
        self.data['cases'][0]['automated']['mapping_status'] = 'PASS'
        with self.assertRaises(ValueError):
            verify.validate_catalog(self.data)

    def test_missing_atomic_assertions_fail(self):
        self.data['serial_workflows'][0].pop('assertion_cases')
        with self.assertRaisesRegex(ValueError, 'Missing atomic assertions'):
            verify.validate_catalog(self.data)

    def test_duplicate_or_misrouted_atomic_assertion_fails(self):
        assertions = self.data['serial_workflows'][0]['assertion_cases']
        original_id = assertions[1]['id']
        for invalid_id in (assertions[0]['id'], 'CUA-99-A01'):
            with self.subTest(invalid_id=invalid_id):
                assertions[1]['id'] = invalid_id
                with self.assertRaises(ValueError):
                    verify.validate_catalog(self.data)
        assertions[1]['id'] = original_id

    def test_atomic_assertion_requires_expected_result(self):
        self.data['serial_workflows'][0]['assertion_cases'][0]['expected'] = ''
        with self.assertRaisesRegex(ValueError, 'Incomplete atomic assertion'):
            verify.validate_catalog(self.data)

    def test_actual_inventory_requires_unique_registration(self):
        inventories = {}
        for case in self.data['cases']:
            for ref in case['automated']['references']:
                entry = ('test-bin', 'module::' + ref['function'])
                package = inventories.setdefault(ref['package'], [])
                if entry not in package:
                    package.append(entry)
        self.assertTrue(verify.validate_catalog(self.data, inventories=inventories)['registration_checked'])
        inventories['warp'].append(inventories['warp'][0])
        with self.assertRaises(ValueError):
            verify.validate_catalog(self.data, inventories=inventories)

    def test_empty_or_truncated_inventory_fails(self):
        for data in ({}, {'rust-suites': {}}, {'rust-suites': {'bin': {}}},
                     {'rust-suites': {'bin': {'testcases': {}}}}):
            with self.subTest(data=data), self.assertRaises(ValueError):
                verify.inventory_names(data)


class AtomicResultTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / 'sample.txt').write_text('synthetic observation')
        assertion = {'id': 'CUA-03-A01', 'action': 'paste into selection',
                     'expected': 'selection replaced, suffix preserved'}
        self.data = {'serial_workflows': [{'assertion_cases': [assertion]}]}
        self.record = dict(assertion, status='PASS', observed='expected text visible',
                           evidence=['sample.txt'], reason='')
        self.results = {'assertions': [self.record]}

    def test_evidence_accounting_does_not_approve_product(self):
        result = verify.check_assertions(self.data, self.results, self.root)
        self.assertEqual(result['product_acceptance'], 'REQUIRES_INDEPENDENT_ORACLE_REVIEW')

    def test_omitted_or_duplicated_assertion_fails(self):
        for records in ([], [self.record, self.record]):
            with self.subTest(count=len(records)), self.assertRaises(ValueError):
                verify.check_assertions(self.data, {'assertions': records}, self.root)

    def test_partial_cannot_hide_unexecuted_assertions(self):
        self.record['status'] = 'PARTIAL'
        with self.assertRaisesRegex(ValueError, 'Invalid atomic status'):
            verify.check_assertions(self.data, self.results, self.root)

    def test_pass_needs_observation_evidence_and_original_contract(self):
        for key, value in [('observed', ''), ('evidence', []), ('expected', 'weaker assertion')]:
            with self.subTest(key=key):
                record = dict(self.record, **{key: value})
                with self.assertRaises(ValueError):
                    verify.check_assertions(self.data, {'assertions': [record]}, self.root)

    def test_blocker_requires_reason_and_does_not_accept(self):
        self.record.update(status='BLOCKED', evidence=[], reason='')
        with self.assertRaisesRegex(ValueError, 'Missing non-pass reason'):
            verify.check_assertions(self.data, self.results, self.root)
        self.record['reason'] = 'No marked-text received through this input path'
        result = verify.check_assertions(self.data, self.results, self.root)
        self.assertEqual(result['product_acceptance'], 'NOT_ACCEPTED')
        self.assertEqual(result['status_counts']['BLOCKED'], 1)

    def test_missing_or_escaping_evidence_fails(self):
        for path in ('missing.txt', '../sample.txt'):
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.record['evidence'] = [path]
                verify.check_assertions(self.data, self.results, self.root)


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.data = {'cases': [{'id': 'L0-01-T01', 'serial_workflows': ['CUA-02']}],
                     'serial_workflows': [
                         {'id': 'CUA-01', 'scenarios': [],
                          'evidence_types': ['identity', 'screenshot', 'process_tree']},
                         {'id': 'CUA-02', 'scenarios': ['L0-01-T01'],
                          'evidence_types': ['screenshot', 'pty']}]}
        self.report = verify.result_template(self.data, 'a' * 40, 'b' * 40, 'c' * 64)

    def complete(self):
        record = self.report['cases'][0]
        record.update(status='PASS', automated_assertions_reviewed=True,
                      command_exit_codes=[0], reviewer='independent fixture reviewer',
                      oracle_review='Synthetic integrity test; not a product verdict',
                      serial_results=[{'workflow': 'CUA-02', 'status': 'PASS',
                                       'desktop_lock_held': True, 'model': 'test model',
                                       'harness': 'test harness', 'observed_utc': '2026-09-24T00:00:00Z'}])
        for kind in ['identity', 'command_log', 'screenshot', 'pty', 'process_tree']:
            path = self.root / kind
            path.write_bytes(b'Synthetic evidence fixture, not real macOS evidence')
            record['artifacts'].append({'type': kind, 'path': kind,
                                        'sha256': verify.sha256(path), 'candidate_head': 'a' * 40})
        prerequisite = copy.deepcopy(record)
        prerequisite['id'] = 'CUA-01'
        prerequisite['serial_results'][0]['workflow'] = 'CUA-01'
        self.report['prerequisites'] = [prerequisite]
        return record

    def test_missing_candidate_identity_workflow_fails(self):
        self.complete()
        del self.report['prerequisites']
        with self.assertRaises(ValueError):
            verify.check_results(self.data, self.report, self.root)

    def test_incomplete_candidate_identity_workflow_fails(self):
        self.complete()
        self.report['prerequisites'][0]['status'] = 'INCOMPLETE'
        with self.assertRaises(ValueError):
            verify.check_results(self.data, self.report, self.root)

    def test_candidate_identity_requires_process_tree_evidence(self):
        self.complete()
        record = self.report['prerequisites'][0]
        record['artifacts'] = [a for a in record['artifacts'] if a['type'] != 'process_tree']
        with self.assertRaises(ValueError):
            verify.check_results(self.data, self.report, self.root)

    def test_templates_start_not_run_and_fail_acceptance(self):
        self.assertEqual(self.report['cases'][0]['status'], 'NOT_RUN')
        self.assertEqual(self.report['prerequisites'][0]['id'], 'CUA-01')
        self.assertEqual(self.report['prerequisites'][0]['status'], 'NOT_RUN')
        with self.assertRaises(ValueError):
            verify.check_results(self.data, self.report, self.root)

    def test_complete_integrity_is_not_a_product_verdict(self):
        self.complete()
        result = verify.check_results(self.data, self.report, self.root)
        self.assertEqual(result['product_acceptance'], 'REQUIRES_INDEPENDENT_ORACLE_REVIEW')

    def test_hash_tamper_fails(self):
        self.complete()
        (self.root / 'pty').write_bytes(b'changed')
        with self.assertRaises(ValueError):
            verify.check_results(self.data, self.report, self.root)

    def test_mixed_candidate_fails(self):
        record = self.complete()
        record['candidate_head'] = 'd' * 40
        with self.assertRaises(ValueError):
            verify.check_results(self.data, self.report, self.root)

    def test_prose_cannot_replace_visual_evidence(self):
        record = self.complete()
        record['artifacts'] = [a for a in record['artifacts'] if a['type'] != 'screenshot']
        with self.assertRaises(ValueError):
            verify.check_results(self.data, self.report, self.root)

    def test_nonzero_or_missing_command_fails(self):
        record = self.complete()
        for codes in ([], [1], [False]):
            record['command_exit_codes'] = codes
            with self.assertRaises(ValueError):
                verify.check_results(self.data, self.report, self.root)

    def test_path_traversal_and_symlink_escape_fail(self):
        with self.assertRaises(ValueError):
            verify.confined(self.root, '../outside')
        (self.root / 'escape').symlink_to(ROOT / 'README.md')
        with self.assertRaises(ValueError):
            verify.confined(self.root, 'escape')

    def test_different_processes_cannot_share_desktop_lease(self):
        lock = self.root / 'lease'
        command = [sys.executable, '-c',
                   'import fcntl,os,sys; f=os.open(sys.argv[1],os.O_RDWR); '
                   'fcntl.flock(f,fcntl.LOCK_EX|fcntl.LOCK_NB)', str(lock)]
        with verify.desktop_lease(lock):
            result = subprocess.run(command, capture_output=True, check=False)
            self.assertNotEqual(result.returncode, 0)
        self.assertEqual(subprocess.run(command, capture_output=True, check=False).returncode, 0)

    def test_lock_symlink_is_rejected(self):
        (self.root / 'target').write_text('target')
        (self.root / 'link').symlink_to(self.root / 'target')
        with self.assertRaises(OSError), verify.desktop_lease(self.root / 'link'):
            pass


if __name__ == '__main__':
    unittest.main(verbosity=2)
