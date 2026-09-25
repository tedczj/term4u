#!/usr/bin/env python3
"""Validate L0 routing/evidence and exclusively lease a macOS test desktop.

Evidence validation checks provenance/completeness, not visual correctness or a product PASS.
No model, cloud API, real clipboard or application process is opened by this tool.
"""
import argparse
import contextlib
import fcntl
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import stat
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
CATALOG = Path('test-data/localization/l0-cases.json')
ID_PATTERN = r'L0-\d{2}-T\d{2}'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def load_json(path):
    with Path(path).open(encoding='utf-8') as source:
        return json.load(source)


def confined(root, relative):
    path = Path(relative)
    require(not path.is_absolute() and '..' not in path.parts, 'Unsafe evidence/source path')
    resolved = (root / path).resolve()
    require(resolved.is_relative_to(root.resolve()), 'Path escapes its root')
    require(resolved.is_file(), f'Missing file: {relative}')
    return resolved


def sha256(path):
    digest = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def inventory_names(data):
    suites = data.get('rust-suites')
    require(isinstance(suites, dict) and suites, 'Missing/empty actual nextest suites')
    names = []
    for binary, suite in suites.items():
        require(isinstance(suite, dict), 'Malformed nextest suite')
        tests = suite.get('testcases')
        require(isinstance(tests, dict), 'Malformed/truncated nextest testcases')
        names.extend((binary, name) for name in tests)
    require(names, 'Empty actual nextest inventory')
    return names


def validate_catalog(data, root=ROOT, inventories=None):
    require(data.get('schema_version') == 1, 'Unsupported catalogue version')
    expected = re.findall(r'^\| (' + ID_PATTERN + r') \|',
                          (root / 'docs/DESIGN.md').read_text(), re.M)
    require(len(expected) == 66 and len(set(expected)) == 66,
            'Canonical design must contain the 66 unique existing scenario IDs')
    cases = data.get('cases', [])
    ids = [case['id'] for case in cases]
    require(len(ids) == len(set(ids)), 'Duplicate case ID')
    require(set(ids) == set(expected), 'Missing or unknown canonical scenario IDs')
    workflows = data.get('serial_workflows', [])
    lookup = {item['id']: item for item in workflows}
    require(len(lookup) == len(workflows), 'Duplicate serial workflow ID')
    require('CUA-01' in lookup and lookup['CUA-01']['scenarios'] == [],
            'Missing mandatory candidate identity workflow CUA-01')
    for item in workflows:
        require(set(item['scenarios']) <= set(expected), 'Unknown serial scenario')
        for key in ('preconditions', 'steps', 'oracles', 'evidence_types', 'cleanup'):
            require(item.get(key), f"Incomplete workflow {item['id']}: {key}")
    refs = 0
    unmapped = 0
    for case in cases:
        auto = case['automated']
        require(auto.get('additional_assertions'), f"No check specification: {case['id']}")
        require(case.get('serial_workflows'), f"No real-path workflow: {case['id']}")
        for workflow in case['serial_workflows']:
            require(workflow in lookup and case['id'] in lookup[workflow]['scenarios'],
                    f"Inconsistent serial mapping: {case['id']}")
        references = auto['references']
        unmapped += not references
        require(auto['mapping_status'] == ('PARTIAL' if references else 'UNMAPPED'),
                'A related test is not full scenario coverage')
        for reference in references:
            refs += 1
            source = confined(root, reference['path']).read_text()
            name = reference['function']
            require(re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*', name), 'Invalid function name')
            require(re.search(r'^\s*(?:async\s+)?fn\s+' + re.escape(name) + r'\s*\(',
                              source, re.M), f'Missing source test: {name}')
            require(reference['coverage'] == 'RELATED_PARTIAL_NOT_SCENARIO_PASS',
                    'Do not turn related tests into scenario PASS')
            if inventories is not None:
                package = reference['package']
                require(package in inventories, f'Missing actual inventory for {package}')
                matches = [(binary, test) for binary, test in inventories[package]
                           if test.rsplit('::', 1)[-1] == name]
                require(len(matches) == 1, f'Unregistered/ambiguous actual test: {name}')
    return {'scenarios': len(cases), 'related_references': refs,
            'unmapped_scenarios': unmapped, 'serial_workflows': len(workflows),
            'registration_checked': inventories is not None,
            'product_acceptance': 'NOT_EVALUATED'}


def result_template(data, source_head, source_tree, binary_sha256):
    for value, length in [(source_head, 40), (source_tree, 40), (binary_sha256, 64)]:
        require(re.fullmatch('[0-9a-f]{' + str(length) + '}', value), 'Invalid candidate hash')

    def pending(ident):
        return {'id': ident, 'candidate_head': source_head, 'status': 'NOT_RUN',
                'automated_assertions_reviewed': False, 'command_exit_codes': [],
                'oracle_review': '', 'reviewer': '', 'artifacts': [], 'serial_results': []}

    return {'schema_version': 1,
            'candidate': {'source_head': source_head, 'source_tree': source_tree,
                          'binary_sha256': binary_sha256, 'diff_sha256': None},
            'prerequisites': [pending('CUA-01')],
            'cases': [pending(case['id']) for case in data['cases']]}


def check_results(data, results, evidence_root):
    """Reject incomplete/tampered reports; passing here is only an integrity check."""
    candidate = results.get('candidate', {})
    for key, length in [('source_head', 40), ('source_tree', 40), ('binary_sha256', 64)]:
        require(re.fullmatch('[0-9a-f]{' + str(length) + '}', candidate.get(key, '')),
                f'Missing candidate identity: {key}')
    require(candidate.get('diff_sha256') is None or
            re.fullmatch('[0-9a-f]{64}', candidate['diff_sha256']), 'Bad source diff hash')
    expected = {case['id']: case for case in data['cases']}
    records = results.get('cases', [])
    require(len(records) == len(expected), 'Incomplete result count')
    require(len({r['id'] for r in records}) == len(records), 'Duplicate results')
    require({r['id'] for r in records} == set(expected), 'Unknown/missing results')
    workflows = {item['id']: item for item in data['serial_workflows']}
    prerequisites = results.get('prerequisites', [])
    require(len(prerequisites) == 1 and prerequisites[0]['id'] == 'CUA-01',
            'Missing mandatory candidate identity result CUA-01')
    expected['CUA-01'] = {'serial_workflows': ['CUA-01']}
    for record in prerequisites + records:
        ident = record['id']
        require(record['status'] == 'PASS', f"Incomplete/failing case {ident}: {record['status']}")
        require(record.get('candidate_head') == candidate['source_head'], 'Mixed candidate heads')
        require(record.get('automated_assertions_reviewed') is True,
                f'Unreviewed full assertion coverage: {ident}')
        codes = record.get('command_exit_codes')
        require(isinstance(codes, list) and codes and all(type(n) is int and n == 0 for n in codes),
                f'No successful real command results: {ident}')
        require(record.get('reviewer') and record.get('oracle_review'), 'Missing independent review')
        artifacts = record.get('artifacts', [])
        types = set()
        for artifact in artifacts:
            file = confined(evidence_root, artifact['path'])
            require(file.stat().st_size > 0, 'Empty evidence')
            require(sha256(file) == artifact['sha256'], 'Evidence hash mismatch')
            require(artifact.get('candidate_head') == candidate['source_head'], 'Mixed artifact heads')
            types.add(artifact['type'])
        require('command_log' in types and 'identity' in types, 'Missing raw command/identity evidence')
        serial = record.get('serial_results', [])
        require(len({r['workflow'] for r in serial}) == len(serial), 'Duplicate serial results')
        require({r['workflow'] for r in serial} == set(expected[ident]['serial_workflows']),
                'Missing required serial workflows')
        for item in serial:
            require(item['status'] == 'PASS', 'Serial workflow not passed')
            require(item.get('desktop_lock_held') is True, 'Desktop ownership not confirmed')
            require(item.get('model') and item.get('harness') and item.get('observed_utc'),
                    'Missing computer-use execution identity')
            require(set(workflows[item['workflow']]['evidence_types']) <= types,
                    'Missing real-path evidence; prose cannot replace screenshots/PTY')
    return {'reports': len(records), 'evidence_integrity': 'VALID',
            'product_acceptance': 'REQUIRES_INDEPENDENT_ORACLE_REVIEW'}


@contextlib.contextmanager
def desktop_lease(path=None):
    """Fixed per-user, machine-wide lease; never scoped to a profile or a model."""
    path = path or Path('/tmp') / f'term4u-l0-desktop-{os.getuid()}.lock'
    fd = os.open(path, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    try:
        info = os.fstat(fd)
        require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid(), 'Unsafe desktop lock owner')
        require(info.st_mode & 0o077 == 0, 'Desktop lock must be private')
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise ValueError('Desktop already leased by another test process') from error
        os.ftruncate(fd, 0)
        os.write(fd, (json.dumps({'pid': os.getpid(), 'uid': os.getuid()}) + '\n').encode())
        yield fd
    finally:
        # Do not unlink: doing so allows a second inode to bypass a still-held lease.
        os.close(fd)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    sub = parser.add_subparsers(dest='command', required=True)
    validate = sub.add_parser('validate')
    validate.add_argument('--inventory', action='append', default=[], metavar='PACKAGE=PATH')
    template = sub.add_parser('init-results')
    template.add_argument('--source-head', required=True)
    template.add_argument('--source-tree', required=True)
    template.add_argument('--binary-sha256', required=True)
    template.add_argument('--output', type=Path, required=True)
    results = sub.add_parser('check-results')
    results.add_argument('report', type=Path)
    results.add_argument('--evidence-root', type=Path, required=True)
    lease = sub.add_parser('serial')
    lease.add_argument('argv', nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    try:
        if args.command == 'serial':
            require(sys.platform == 'darwin' and platform.machine() == 'arm64',
                    'Real desktop execution requires macOS Apple Silicon')
            command = args.argv[1:] if args.argv[:1] == ['--'] else args.argv
            require(command, 'Supply the existing computer-use harness command')
            with desktop_lease() as fd:
                # The child retains the lease even if this wrapper is terminated first.
                return subprocess.call(command, pass_fds=(fd,))
        data = load_json(args.root / CATALOG)
        inventories = None
        if args.command == 'validate' and args.inventory:
            inventories = {}
            for entry in args.inventory:
                package, path = entry.split('=', 1)
                require(package not in inventories, 'Duplicate inventory package')
                inventories[package] = inventory_names(load_json(path))
        summary = validate_catalog(data, args.root, inventories)
        if args.command == 'init-results':
            report = result_template(data, args.source_head, args.source_tree, args.binary_sha256)
            with args.output.open('x', encoding='utf-8') as out:
                json.dump(report, out, ensure_ascii=False, indent=2)
                out.write('\n')
            summary = {'created': str(args.output), 'status': 'ALL_NOT_RUN'}
        elif args.command == 'check-results':
            summary = check_results(data, load_json(args.report), args.evidence_root)
        print(json.dumps(summary, ensure_ascii=False, indent=2))
        return 0
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f'ERROR: {error}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
