import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import { getDeviceLogs } from './data.ts'

describe('getDeviceLogs', () => {
  it('retrieves and validates device logs from the API', async () => {
    let result = await getDeviceLogs(
      new URL('http://api.example.test:8000'),
      undefined,
      async (input) => {
        assert.equal(input.toString(), 'http://api.example.test:8000/device-logs')

        return Response.json({
          device_logs: [
            {
              id: '7f3df51e-0e66-4e70-98ee-80d775adf76f',
              device_mac: 'aa:bb:cc:dd:ee:ff',
              crash_number: 42,
              uptime_ms: 123456,
              received_at: '2026-08-01T12:00:00Z',
              software_version: '0.1.0',
              reset_reason: 1,
              program_counter: 1074270772,
              exception_cause: 6,
              task_name: 'main',
            },
          ],
          pagination: { page: 1, per_page: 20 },
        })
      },
    )

    assert.equal(result.device_logs[0]?.device_mac, 'aa:bb:cc:dd:ee:ff')
    assert.equal(result.device_logs[0]?.program_counter, 1074270772)
    assert.deepEqual(result.pagination, { page: 1, per_page: 20 })
  })

  it('accepts nullable crash details', async () => {
    let result = await getDeviceLogs(
      new URL('http://api.example.test:8000'),
      undefined,
      async () =>
        Response.json({
          device_logs: [
            {
              id: '7f3df51e-0e66-4e70-98ee-80d775adf76f',
              device_mac: 'aa:bb:cc:dd:ee:ff',
              crash_number: 42,
              uptime_ms: 123456,
              received_at: '2026-08-01T12:00:00Z',
              software_version: null,
              reset_reason: 1,
              program_counter: null,
              exception_cause: null,
              task_name: null,
            },
          ],
          pagination: { page: 1, per_page: 20 },
        }),
    )

    assert.equal(result.device_logs[0]?.program_counter, null)
  })

  it('rejects a response that does not match the API contract', async () => {
    await assert.rejects(
      getDeviceLogs(new URL('http://api.example.test:8000'), undefined, async () =>
        Response.json({ device_logs: 'not-an-array', pagination: {} }),
      ),
    )
  })
})
