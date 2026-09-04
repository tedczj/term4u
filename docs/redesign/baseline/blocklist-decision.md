# M5 blocklist decision

DECISION: route-a
BASE_HEAD: d18d63a53b21418052ce7328d743f0d5b25c207f
OFFLINE_API_CALL_SITES: 88
BLOCKLIST_TEST_FILES: 59
PHASE2_REUSE_DECISION: 不直接复用；未来按 agent_protocol 重新设计适配层
APPROVED_BY: M5/M6 implementation owner
DATE_UTC: 2026-09-04T10:29:17Z

## Evidence

- Call-site inventory: "/tmp/m5-blocklist-server-api.txt" was generated with the command in
  [10 §3.1](../10-M5-M6未完成项实施详设.md#s3); the measured count is 88.
- Test-file inventory: "/tmp/m5-blocklist-test-files.txt" was generated with the command in
  [10 §3.1](../10-M5-M6未完成项实施详设.md#s3); the measured count is 59.
- Route A is selected because the blocklist directly consumes cloud agent APIs and there is no
  frozen phase-2 local agent protocol. The existing implementation will not be carried forward.
