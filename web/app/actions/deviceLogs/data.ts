import * as s from 'remix/data-schema'
import type { InferOutput } from 'remix/data-schema'

const deviceLogSchema = s.object({
  id: s.string(),
  device_mac: s.string(),
  crash_number: s.number(),
  uptime_ms: s.number(),
  received_at: s
    .string()
    .refine((value) => !Number.isNaN(Date.parse(value)), 'Expected a valid date'),
  software_version: s.nullable(s.string()),
  reset_reason: s.number(),
  program_counter: s.nullable(s.number()),
  exception_cause: s.nullable(s.number()),
  task_name: s.nullable(s.string()),
})

const deviceLogsResponseSchema = s.object({
  device_logs: s.array(deviceLogSchema),
  pagination: s.object({
    page: s.number(),
    per_page: s.number(),
  }),
})

export type DeviceLog = InferOutput<typeof deviceLogSchema>
export type DeviceLogsResponse = InferOutput<typeof deviceLogsResponseSchema>

export async function getDeviceLogs(
  apiOrigin: URL,
  signal?: AbortSignal,
  fetcher: typeof fetch = fetch,
): Promise<DeviceLogsResponse> {
  let url = new URL('/device-logs', apiOrigin)
  let response = await fetcher(url, {
    headers: { Accept: 'application/json' },
    signal,
  })

  if (!response.ok) {
    throw new Error(`Device logs API returned ${response.status} ${response.statusText}`)
  }

  return s.parse(deviceLogsResponseSchema, await response.json())
}
