import { createController } from 'remix/router'

import { ApiOrigin } from '../../middleware/api-origin.ts'
import { routes } from '../../routes.ts'
import { getDeviceLogs } from './data.ts'
import { DeviceLogsPage } from './show-page.tsx'

export default createController(routes.deviceLogs, {
  actions: {
    async show(context) {
      try {
        let { device_logs, pagination } = await getDeviceLogs(
          context.get(ApiOrigin),
          context.request.signal,
        )
        return context.render(
          <DeviceLogsPage deviceLogs={device_logs} pagination={pagination} />,
        )
      } catch (error) {
        if (context.request.signal.aborted) throw error

        console.error('Failed to retrieve device logs from the API', error)
        return new Response('Unable to retrieve device logs', { status: 502 })
      }
    },
  },
})
