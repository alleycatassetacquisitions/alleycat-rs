import type { Handle, RemixNode } from 'remix/ui'
import { css } from 'remix/ui'

import { Document } from '../../ui/document.tsx'
import type { DeviceLog, DeviceLogsResponse } from './data.ts'

interface DeviceLogsPageProps {
  deviceLogs: DeviceLog[]
  pagination: DeviceLogsResponse['pagination']
}

export function DeviceLogsPage(handle: Handle<DeviceLogsPageProps>) {
  return () => {
    let { deviceLogs, pagination } = handle.props

    return (
      <Document title="Device logs">
        <main
          mix={css({
            maxWidth: '72rem',
            margin: '0 auto',
            padding: '4rem 1.5rem',
            fontFamily: 'system-ui, sans-serif',
            color: '#111827',
          })}
        >
          <header mix={css({ marginBottom: '2rem' })}>
            <h1 mix={css({ margin: 0 })}>Device logs</h1>
            <p mix={css({ margin: '0.5rem 0 0', color: '#4b5563' })}>
              Crash reports received from Alley Cat devices, newest first.
            </p>
          </header>

          {deviceLogs.length === 0 ? (
            <p>No device logs have been received yet.</p>
          ) : (
            <ol
              mix={css({
                listStyle: 'none',
                margin: 0,
                padding: 0,
                display: 'grid',
                gap: '1rem',
              })}
            >
              {deviceLogs.map((deviceLog) => (
                <li
                  key={deviceLog.id}
                  mix={css({
                    border: '1px solid #d1d5db',
                    borderRadius: '0.75rem',
                    padding: '1.25rem',
                    boxShadow: '0 1px 2px rgb(0 0 0 / 0.05)',
                  })}
                >
                  <div
                    mix={css({
                      display: 'flex',
                      alignItems: 'baseline',
                      justifyContent: 'space-between',
                      flexWrap: 'wrap',
                      gap: '0.5rem 1rem',
                    })}
                  >
                    <strong mix={css({ fontSize: '1.125rem' })}>
                      <code>{deviceLog.device_mac}</code>
                    </strong>
                    <span mix={css({ color: '#4b5563' })}>
                      Crash #{deviceLog.crash_number.toLocaleString('en-US')}
                    </span>
                  </div>

                  <dl
                    mix={css({
                      display: 'grid',
                      gridTemplateColumns: 'repeat(auto-fit, minmax(11rem, 1fr))',
                      gap: '1rem',
                      margin: '1.25rem 0 0',
                    })}
                  >
                    <LogField label="Received">
                      <time dateTime={deviceLog.received_at}>
                        {new Date(deviceLog.received_at).toLocaleString('en-US')}
                      </time>
                    </LogField>
                    <LogField label="Uptime">
                      {deviceLog.uptime_ms.toLocaleString('en-US')} ms
                    </LogField>
                    <LogField label="Software">
                      {deviceLog.software_version ?? '—'}
                    </LogField>
                    <LogField label="Reset reason">{deviceLog.reset_reason}</LogField>
                    <LogField label="Program counter">
                      {formatProgramCounter(deviceLog.program_counter)}
                    </LogField>
                    <LogField label="Exception cause">
                      {deviceLog.exception_cause ?? '—'}
                    </LogField>
                    <LogField label="FreeRTOS task">{deviceLog.task_name ?? '—'}</LogField>
                  </dl>
                </li>
              ))}
            </ol>
          )}

          <p mix={css({ marginTop: '2rem', color: '#4b5563' })}>
            Page {pagination.page} · Up to {pagination.per_page} logs per page
          </p>
        </main>
      </Document>
    )
  }
}

interface LogFieldProps {
  children?: RemixNode
  label: string
}

function LogField(handle: Handle<LogFieldProps>) {
  return () => {
    let { children, label } = handle.props

    return (
      <div>
        <dt
          mix={css({
            marginBottom: '0.25rem',
            color: '#6b7280',
            fontSize: '0.75rem',
            fontWeight: 600,
            letterSpacing: '0.04em',
            textTransform: 'uppercase',
          })}
        >
          {label}
        </dt>
        <dd mix={css({ margin: 0, overflowWrap: 'anywhere' })}>{children}</dd>
      </div>
    )
  }
}

function formatProgramCounter(value: number | null): string {
  return value === null ? '—' : `0x${value.toString(16).padStart(8, '0')}`
}
