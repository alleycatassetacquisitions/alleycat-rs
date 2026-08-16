# Operator guide

The current UI displays players and device crash reports. It has no
registration, editing, searching, or administrative controls.

## Open the app

Get the shared app URL from the maintainer. For a local instance, use
<http://localhost:3000>. Anyone with the URL can access the pages and API; do
not enter sensitive personal data.

The home page does not link to the lists yet. Open:

| Page | Local URL | Purpose |
| --- | --- | --- |
| Players | <http://localhost:3000/players> | See recently registered players and their assigned codes |
| Device logs | <http://localhost:3000/device-logs> | See recently received crash/reset reports, newest first |

On the shared server, append the same path to its origin, for example
`https://<app-origin>/players`.

## Players

Each player card shows:

- display name;
- four-digit PDN code;
- current mode (`unassigned`, `hunter`, or `bounty`); and
- server registration date and time.

Only the 20 newest players are shown; there are no paging controls.

## Device logs

Each report shows:

- device MAC address and device-assigned crash number;
- when the server received it;
- uptime in milliseconds and firmware/software version;
- raw reset reason, program counter, and exception cause; and
- the active FreeRTOS task, if the device supplied one.

An em dash (`—`) means the device omitted that optional field. Program counters
are displayed as eight-digit hexadecimal addresses. Reset-reason and exception
numbers are raw device-platform codes; this server does not translate them to
human descriptions.

Only the 20 newest reports are shown; there are no paging, filtering, search,
export, or detail controls.

Times use the server's timezone, not the browser's timezone.

## Field glossary

| Term | Meaning in the current app |
| --- | --- |
| PDN code | Four-digit player code |
| Player mode | One primary gameplay state: `unassigned`, `hunter`, or `bounty` |
| Device MAC | Device-reported hardware address |
| Crash number | Report number assigned by the device |
| Uptime | Milliseconds since the device booted, as reported by the device |
| Reset reason | Raw numeric platform reason for the reset |
| Program counter | Instruction address active at the exception, shown in hexadecimal |
| Exception cause | Raw numeric platform exception code |
| Received | Time the server first stored the report, not necessarily when the crash happened |

## When something looks wrong

- “No players have registered yet” or “No device logs have been received yet”
  means this app has no records to show. Confirm that you opened the right app.
- “Unable to retrieve players” or “Unable to retrieve device logs” means a
  server problem. Report it to the maintainer.

Include the app URL, page, approximate time, expected result, and exact error.
Do not include passwords or other secrets.
