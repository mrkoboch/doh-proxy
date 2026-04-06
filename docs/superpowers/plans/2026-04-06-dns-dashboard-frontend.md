# DNS Dashboard Frontend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a React + Vite SPA that polls the dns-dashboard backend every 5 seconds and displays DNS query stats in a dark-themed dashboard.

**Architecture:** Single-page app in `dns-dashboard-ui/` at the repo root. A Vite dev-server proxy forwards `/api` requests to `http://localhost:4000` so the frontend makes same-origin calls. All data lives in App-level state updated by a `setInterval` effect. Components receive data as props and are pure display.

**Tech Stack:** React 18, Vite 5, recharts, axios, Vitest, @testing-library/react

---

## Prerequisites

Node.js 18+ must be installed. If `node --version` fails, install via NVM:

```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash
source ~/.bashrc   # or ~/.zshrc
nvm install 20
nvm use 20
```

---

## File Map

| Path | Purpose |
|---|---|
| `dns-dashboard-ui/` | Vite project root |
| `dns-dashboard-ui/vite.config.js` | Vite config with `/api` proxy to port 4000 |
| `dns-dashboard-ui/src/api.js` | Three axios functions: `fetchStats`, `fetchTopDomains`, `fetchRecentQueries` |
| `dns-dashboard-ui/src/utils.js` | `bucketByMinute(queries)` — pure function, testable |
| `dns-dashboard-ui/src/App.jsx` | Root component: polling effect, state, layout |
| `dns-dashboard-ui/src/App.css` | Global dark theme, grid layout |
| `dns-dashboard-ui/src/components/StatCards.jsx` | Three stat cards |
| `dns-dashboard-ui/src/components/TopDomains.jsx` | Horizontal BarChart of top domains |
| `dns-dashboard-ui/src/components/QueryFeed.jsx` | Scrollable table of recent queries |
| `dns-dashboard-ui/src/components/QueriesOverTime.jsx` | LineChart of queries per minute |
| `dns-dashboard-ui/src/test-setup.js` | Vitest setup: mock ResizeObserver (required for Recharts in jsdom) |
| `dns-dashboard-ui/src/api.test.js` | Tests: correct URLs called |
| `dns-dashboard-ui/src/utils.test.js` | Tests: bucketing logic |
| `dns-dashboard-ui/src/components/StatCards.test.jsx` | Tests: card values rendered |
| `dns-dashboard-ui/src/components/QueryFeed.test.jsx` | Tests: row rendering, time format, badges |

---

## Task 1: Project Scaffold

**Files:**
- Create: `dns-dashboard-ui/` (entire Vite project)
- Modify: `dns-dashboard-ui/vite.config.js`

- [ ] **Step 1: Scaffold a new Vite + React project**

```bash
cd /opt/doh_proxy
npm create vite@latest dns-dashboard-ui -- --template react
cd dns-dashboard-ui
npm install
```

- [ ] **Step 2: Install runtime dependencies**

```bash
npm install recharts axios
```

- [ ] **Step 3: Install dev dependencies**

```bash
npm install -D vitest @vitest/coverage-v8 @testing-library/react @testing-library/jest-dom @testing-library/user-event jsdom
```

- [ ] **Step 4: Configure Vite proxy and Vitest**

Replace the full contents of `dns-dashboard-ui/vite.config.js`:

```js
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:4000',
        changeOrigin: true,
      },
    },
  },
  test: {
    environment: 'jsdom',
    setupFiles: './src/test-setup.js',
    globals: true,
  },
})
```

- [ ] **Step 5: Create the Vitest setup file**

Create `dns-dashboard-ui/src/test-setup.js`:

```js
import '@testing-library/jest-dom'

// Recharts uses ResizeObserver which is not available in jsdom
global.ResizeObserver = class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}
```

- [ ] **Step 6: Add a test script to package.json**

Edit `dns-dashboard-ui/package.json` — in the `"scripts"` section, add:

```json
"test": "vitest run",
"test:watch": "vitest"
```

The full `scripts` block should look like:

```json
"scripts": {
  "dev": "vite",
  "build": "vite build",
  "preview": "vite preview",
  "test": "vitest run",
  "test:watch": "vitest"
}
```

- [ ] **Step 7: Verify the scaffold compiles**

```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm run build 2>&1 | tail -5
```

Expected: `✓ built in` with no errors.

- [ ] **Step 8: Commit**

```bash
cd /opt/doh_proxy
git add dns-dashboard-ui/
git commit -m "chore: scaffold dns-dashboard-ui with Vite+React, recharts, axios, vitest"
```

---

## Task 2: API Layer (`src/api.js`)

**Files:**
- Create: `dns-dashboard-ui/src/api.js`
- Create: `dns-dashboard-ui/src/api.test.js`

- [ ] **Step 1: Write failing tests**

Create `dns-dashboard-ui/src/api.test.js`:

```js
import { describe, it, expect, vi, beforeEach } from 'vitest'
import axios from 'axios'
import { fetchStats, fetchTopDomains, fetchRecentQueries } from './api'

vi.mock('axios')

beforeEach(() => {
  vi.resetAllMocks()
})

describe('fetchStats', () => {
  it('calls GET /api/stats and returns data', async () => {
    const data = { total: 10, blocked: 2, avg_latency_ms: 15.5 }
    axios.get.mockResolvedValue({ data })
    const result = await fetchStats()
    expect(axios.get).toHaveBeenCalledWith('/api/stats')
    expect(result).toEqual(data)
  })
})

describe('fetchTopDomains', () => {
  it('calls GET /api/queries/top-domains?limit=10 and returns data', async () => {
    const data = [{ domain: 'example.com', count: 5 }]
    axios.get.mockResolvedValue({ data })
    const result = await fetchTopDomains()
    expect(axios.get).toHaveBeenCalledWith('/api/queries/top-domains?limit=10')
    expect(result).toEqual(data)
  })
})

describe('fetchRecentQueries', () => {
  it('calls GET /api/queries/recent?limit=50 and returns data', async () => {
    const data = [{ id: 1, domain: 'a.com', query_type: 'A' }]
    axios.get.mockResolvedValue({ data })
    const result = await fetchRecentQueries()
    expect(axios.get).toHaveBeenCalledWith('/api/queries/recent?limit=50')
    expect(result).toEqual(data)
  })
})
```

- [ ] **Step 2: Run to confirm failure**

```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm test 2>&1 | tail -10
```

Expected: FAIL — `Cannot find module './api'`

- [ ] **Step 3: Create `src/api.js`**

```js
import axios from 'axios'

export async function fetchStats() {
  const { data } = await axios.get('/api/stats')
  return data
}

export async function fetchTopDomains() {
  const { data } = await axios.get('/api/queries/top-domains?limit=10')
  return data
}

export async function fetchRecentQueries() {
  const { data } = await axios.get('/api/queries/recent?limit=50')
  return data
}
```

- [ ] **Step 4: Run tests**

```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm test 2>&1 | tail -10
```

Expected: `3 passed`

- [ ] **Step 5: Commit**

```bash
cd /opt/doh_proxy
git add dns-dashboard-ui/src/api.js dns-dashboard-ui/src/api.test.js
git commit -m "feat(dns-dashboard-ui): api.js with fetchStats, fetchTopDomains, fetchRecentQueries"
```

---

## Task 3: Data Utilities (`src/utils.js`)

**Files:**
- Create: `dns-dashboard-ui/src/utils.js`
- Create: `dns-dashboard-ui/src/utils.test.js`

`bucketByMinute` takes a list of query objects (each with a `timestamp` ISO8601 string) and returns an array of `{ time: "HH:MM", count: N }` objects sorted chronologically, for use in the LineChart.

- [ ] **Step 1: Write failing tests**

Create `dns-dashboard-ui/src/utils.test.js`:

```js
import { describe, it, expect } from 'vitest'
import { bucketByMinute } from './utils'

describe('bucketByMinute', () => {
  it('returns empty array for empty input', () => {
    expect(bucketByMinute([])).toEqual([])
  })

  it('groups queries into 1-minute buckets', () => {
    const queries = [
      { timestamp: '2026-04-06T12:00:05Z' },
      { timestamp: '2026-04-06T12:00:45Z' },
      { timestamp: '2026-04-06T12:01:10Z' },
    ]
    const result = bucketByMinute(queries)
    expect(result).toHaveLength(2)
    expect(result[0].count).toBe(2) // 12:00
    expect(result[1].count).toBe(1) // 12:01
  })

  it('labels each bucket as HH:MM', () => {
    const queries = [{ timestamp: '2026-04-06T09:07:00Z' }]
    const result = bucketByMinute(queries)
    expect(result[0].time).toBe('09:07')
  })

  it('sorts buckets chronologically oldest-first', () => {
    const queries = [
      { timestamp: '2026-04-06T12:02:00Z' },
      { timestamp: '2026-04-06T12:00:00Z' },
      { timestamp: '2026-04-06T12:01:00Z' },
    ]
    const result = bucketByMinute(queries)
    expect(result.map(r => r.time)).toEqual(['12:00', '12:01', '12:02'])
  })
})
```

- [ ] **Step 2: Run to confirm failure**

```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm test 2>&1 | tail -10
```

Expected: FAIL — `Cannot find module './utils'`

- [ ] **Step 3: Create `src/utils.js`**

```js
/**
 * Groups an array of query objects by the minute of their timestamp.
 * @param {Array<{timestamp: string}>} queries
 * @returns {Array<{time: string, count: number}>} sorted chronologically
 */
export function bucketByMinute(queries) {
  const counts = {}
  for (const q of queries) {
    const d = new Date(q.timestamp)
    const hh = String(d.getUTCHours()).padStart(2, '0')
    const mm = String(d.getUTCMinutes()).padStart(2, '0')
    const key = `${hh}:${mm}`
    counts[key] = (counts[key] ?? 0) + 1
  }
  return Object.entries(counts)
    .sort(([a], [b]) => (a < b ? -1 : 1))
    .map(([time, count]) => ({ time, count }))
}
```

- [ ] **Step 4: Run tests**

```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm test 2>&1 | tail -10
```

Expected: `7 passed` (3 api + 4 utils)

- [ ] **Step 5: Commit**

```bash
cd /opt/doh_proxy
git add dns-dashboard-ui/src/utils.js dns-dashboard-ui/src/utils.test.js
git commit -m "feat(dns-dashboard-ui): bucketByMinute utility with tests"
```

---

## Task 4: StatCards Component

**Files:**
- Create: `dns-dashboard-ui/src/components/StatCards.jsx`
- Create: `dns-dashboard-ui/src/components/StatCards.test.jsx`

Props: `stats` — object with `total`, `blocked`, `avg_latency_ms` (nullable).

- [ ] **Step 1: Write failing tests**

Create `dns-dashboard-ui/src/components/StatCards.test.jsx`:

```jsx
import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import StatCards from './StatCards'

const sampleStats = { total: 100, blocked: 25, avg_latency_ms: 42.5 }

describe('StatCards', () => {
  it('renders total queries count', () => {
    render(<StatCards stats={sampleStats} />)
    expect(screen.getByText('100')).toBeInTheDocument()
  })

  it('renders blocked queries count', () => {
    render(<StatCards stats={sampleStats} />)
    expect(screen.getByText('25')).toBeInTheDocument()
  })

  it('renders block rate percentage', () => {
    render(<StatCards stats={sampleStats} />)
    expect(screen.getByText('25.0% block rate')).toBeInTheDocument()
  })

  it('renders avg latency rounded to 1 decimal', () => {
    render(<StatCards stats={sampleStats} />)
    expect(screen.getByText('42.5 ms')).toBeInTheDocument()
  })

  it('renders — when avg_latency_ms is null', () => {
    render(<StatCards stats={{ total: 0, blocked: 0, avg_latency_ms: null }} />)
    expect(screen.getByText('— ms')).toBeInTheDocument()
  })

  it('renders 0.0% block rate when total is 0', () => {
    render(<StatCards stats={{ total: 0, blocked: 0, avg_latency_ms: null }} />)
    expect(screen.getByText('0.0% block rate')).toBeInTheDocument()
  })
})
```

- [ ] **Step 2: Run to confirm failure**

```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm test 2>&1 | tail -10
```

Expected: FAIL — `Cannot find module './StatCards'`

- [ ] **Step 3: Create `src/components/StatCards.jsx`**

```jsx
export default function StatCards({ stats }) {
  const { total, blocked, avg_latency_ms } = stats
  const blockRate = total > 0 ? ((blocked / total) * 100).toFixed(1) : '0.0'
  const latency = avg_latency_ms != null ? avg_latency_ms.toFixed(1) : '—'

  return (
    <>
      <style>{`
        .stat-cards {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 16px;
        }
        .stat-card {
          background: #1a1d27;
          border-radius: 8px;
          padding: 20px 24px;
        }
        .stat-card__label {
          font-size: 12px;
          color: #94a3b8;
          text-transform: uppercase;
          letter-spacing: 0.05em;
          margin-bottom: 8px;
        }
        .stat-card__value {
          font-size: 32px;
          font-weight: 700;
          color: #4f9eff;
        }
        .stat-card__sub {
          font-size: 12px;
          color: #64748b;
          margin-top: 4px;
        }
      `}</style>
      <div className="stat-cards">
        <div className="stat-card">
          <div className="stat-card__label">Total Queries</div>
          <div className="stat-card__value">{total}</div>
        </div>
        <div className="stat-card">
          <div className="stat-card__label">Blocked Queries</div>
          <div className="stat-card__value">{blocked}</div>
          <div className="stat-card__sub">{blockRate}% block rate</div>
        </div>
        <div className="stat-card">
          <div className="stat-card__label">Avg Latency</div>
          <div className="stat-card__value">{latency} ms</div>
        </div>
      </div>
    </>
  )
}
```

- [ ] **Step 4: Run tests**

```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm test 2>&1 | tail -10
```

Expected: `13 passed`

- [ ] **Step 5: Commit**

```bash
cd /opt/doh_proxy
git add dns-dashboard-ui/src/components/
git commit -m "feat(dns-dashboard-ui): StatCards component"
```

---

## Task 5: QueryFeed Component

**Files:**
- Create: `dns-dashboard-ui/src/components/QueryFeed.jsx`
- Create: `dns-dashboard-ui/src/components/QueryFeed.test.jsx`

Props: `queries` — array of `{ id, timestamp, domain, query_type, latency_ms, blocked, resolver }`.

Time must be formatted as `HH:MM:SS` from the ISO8601 timestamp string. `blocked` renders as a green (`CLEAN`) or red (`BLOCKED`) badge. `latency_ms` renders as `Xms` or `—` if null. `resolver` renders as-is or `—` if null.

- [ ] **Step 1: Write failing tests**

Create `dns-dashboard-ui/src/components/QueryFeed.test.jsx`:

```jsx
import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import QueryFeed from './QueryFeed'

const sampleQueries = [
  {
    id: 1,
    timestamp: '2026-04-06T09:07:42Z',
    domain: 'example.com',
    query_type: 'A',
    latency_ms: 35,
    blocked: false,
    resolver: 'https://dns.example/dns-query',
  },
  {
    id: 2,
    timestamp: '2026-04-06T09:07:55Z',
    domain: 'ads.tracker.io',
    query_type: 'AAAA',
    latency_ms: null,
    blocked: true,
    resolver: null,
  },
]

describe('QueryFeed', () => {
  it('renders domain names', () => {
    render(<QueryFeed queries={sampleQueries} />)
    expect(screen.getByText('example.com')).toBeInTheDocument()
    expect(screen.getByText('ads.tracker.io')).toBeInTheDocument()
  })

  it('formats timestamp as HH:MM:SS', () => {
    render(<QueryFeed queries={sampleQueries} />)
    expect(screen.getByText('09:07:42')).toBeInTheDocument()
    expect(screen.getByText('09:07:55')).toBeInTheDocument()
  })

  it('renders latency with ms suffix', () => {
    render(<QueryFeed queries={sampleQueries} />)
    expect(screen.getByText('35ms')).toBeInTheDocument()
  })

  it('renders — for null latency', () => {
    render(<QueryFeed queries={sampleQueries} />)
    // first occurrence of — is for null latency
    expect(screen.getAllByText('—').length).toBeGreaterThan(0)
  })

  it('renders BLOCKED badge for blocked queries', () => {
    render(<QueryFeed queries={sampleQueries} />)
    expect(screen.getByText('BLOCKED')).toBeInTheDocument()
  })

  it('renders CLEAN badge for non-blocked queries', () => {
    render(<QueryFeed queries={sampleQueries} />)
    expect(screen.getByText('CLEAN')).toBeInTheDocument()
  })

  it('renders resolver URL when present', () => {
    render(<QueryFeed queries={sampleQueries} />)
    expect(screen.getByText('https://dns.example/dns-query')).toBeInTheDocument()
  })

  it('renders empty state message when queries is empty', () => {
    render(<QueryFeed queries={[]} />)
    expect(screen.getByText('No queries yet.')).toBeInTheDocument()
  })
})
```

- [ ] **Step 2: Run to confirm failure**

```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm test 2>&1 | tail -10
```

Expected: FAIL — `Cannot find module './QueryFeed'`

- [ ] **Step 3: Create `src/components/QueryFeed.jsx`**

```jsx
function formatTime(isoString) {
  const d = new Date(isoString)
  const hh = String(d.getUTCHours()).padStart(2, '0')
  const mm = String(d.getUTCMinutes()).padStart(2, '0')
  const ss = String(d.getUTCSeconds()).padStart(2, '0')
  return `${hh}:${mm}:${ss}`
}

export default function QueryFeed({ queries }) {
  return (
    <>
      <style>{`
        .query-feed {
          background: #1a1d27;
          border-radius: 8px;
          overflow: hidden;
          display: flex;
          flex-direction: column;
          max-height: 480px;
        }
        .query-feed__title {
          padding: 16px 20px;
          font-size: 14px;
          font-weight: 600;
          color: #94a3b8;
          text-transform: uppercase;
          letter-spacing: 0.05em;
          border-bottom: 1px solid #2d3148;
        }
        .query-feed__scroll {
          overflow-y: auto;
          flex: 1;
        }
        .query-feed table {
          width: 100%;
          border-collapse: collapse;
          font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
          font-size: 12px;
        }
        .query-feed th {
          padding: 8px 12px;
          text-align: left;
          color: #64748b;
          font-weight: 500;
          background: #0f1117;
          position: sticky;
          top: 0;
        }
        .query-feed td {
          padding: 7px 12px;
          color: #e2e8f0;
          border-bottom: 1px solid #1e2235;
          white-space: nowrap;
          max-width: 200px;
          overflow: hidden;
          text-overflow: ellipsis;
        }
        .query-feed tr:hover td {
          background: #1e2235;
        }
        .badge {
          padding: 2px 6px;
          border-radius: 4px;
          font-size: 10px;
          font-weight: 700;
          letter-spacing: 0.05em;
        }
        .badge--clean { background: #14532d; color: #4ade80; }
        .badge--blocked { background: #4c0519; color: #f87171; }
        .empty-state {
          padding: 32px;
          text-align: center;
          color: #475569;
          font-size: 14px;
        }
      `}</style>
      <div className="query-feed">
        <div className="query-feed__title">Recent Queries</div>
        {queries.length === 0 ? (
          <div className="empty-state">No queries yet.</div>
        ) : (
          <div className="query-feed__scroll">
            <table>
              <thead>
                <tr>
                  <th>Time</th>
                  <th>Domain</th>
                  <th>Type</th>
                  <th>Latency</th>
                  <th>Blocked</th>
                  <th>Resolver</th>
                </tr>
              </thead>
              <tbody>
                {queries.map((q) => (
                  <tr key={q.id}>
                    <td>{formatTime(q.timestamp)}</td>
                    <td title={q.domain}>{q.domain}</td>
                    <td>{q.query_type}</td>
                    <td>{q.latency_ms != null ? `${q.latency_ms}ms` : '—'}</td>
                    <td>
                      {q.blocked ? (
                        <span className="badge badge--blocked">BLOCKED</span>
                      ) : (
                        <span className="badge badge--clean">CLEAN</span>
                      )}
                    </td>
                    <td title={q.resolver ?? ''}>{q.resolver ?? '—'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </>
  )
}
```

- [ ] **Step 4: Run tests**

```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm test 2>&1 | tail -10
```

Expected: `21 passed`

- [ ] **Step 5: Commit**

```bash
cd /opt/doh_proxy
git add dns-dashboard-ui/src/components/QueryFeed.jsx dns-dashboard-ui/src/components/QueryFeed.test.jsx
git commit -m "feat(dns-dashboard-ui): QueryFeed component with time formatting and badges"
```

---

## Task 6: TopDomains Component

**Files:**
- Create: `dns-dashboard-ui/src/components/TopDomains.jsx`

Props: `domains` — array of `{ domain: string, count: number }`.

Horizontal bar chart: domain on Y axis, count on X axis. No separate test file — Recharts internals are SVG and impractical to assert on. A build-time check (no compile error) is sufficient.

- [ ] **Step 1: Create `src/components/TopDomains.jsx`**

```jsx
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from 'recharts'

export default function TopDomains({ domains }) {
  return (
    <>
      <style>{`
        .top-domains {
          background: #1a1d27;
          border-radius: 8px;
          padding: 20px;
        }
        .top-domains__title {
          font-size: 14px;
          font-weight: 600;
          color: #94a3b8;
          text-transform: uppercase;
          letter-spacing: 0.05em;
          margin-bottom: 16px;
        }
        .top-domains__empty {
          color: #475569;
          font-size: 14px;
          text-align: center;
          padding: 32px 0;
        }
      `}</style>
      <div className="top-domains">
        <div className="top-domains__title">Top Domains</div>
        {domains.length === 0 ? (
          <div className="top-domains__empty">No data yet.</div>
        ) : (
          <ResponsiveContainer width="100%" height={domains.length * 36 + 20}>
            <BarChart
              data={domains}
              layout="vertical"
              margin={{ top: 0, right: 24, bottom: 0, left: 8 }}
            >
              <XAxis
                type="number"
                tick={{ fill: '#64748b', fontSize: 11 }}
                axisLine={false}
                tickLine={false}
              />
              <YAxis
                type="category"
                dataKey="domain"
                width={160}
                tick={{ fill: '#e2e8f0', fontSize: 12, fontFamily: 'monospace' }}
                axisLine={false}
                tickLine={false}
              />
              <Tooltip
                contentStyle={{
                  background: '#1a1d27',
                  border: '1px solid #2d3148',
                  borderRadius: 6,
                  color: '#e2e8f0',
                }}
                cursor={{ fill: 'rgba(79,158,255,0.08)' }}
              />
              <Bar dataKey="count" radius={[0, 4, 4, 0]}>
                {domains.map((_, i) => (
                  <Cell key={i} fill="#4f9eff" />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        )}
      </div>
    </>
  )
}
```

- [ ] **Step 2: Verify build still passes**

```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm run build 2>&1 | tail -5
```

Expected: `✓ built in` with no errors.

- [ ] **Step 3: Commit**

```bash
cd /opt/doh_proxy
git add dns-dashboard-ui/src/components/TopDomains.jsx
git commit -m "feat(dns-dashboard-ui): TopDomains horizontal BarChart component"
```

---

## Task 7: QueriesOverTime Component

**Files:**
- Create: `dns-dashboard-ui/src/components/QueriesOverTime.jsx`

Props: `queries` — the raw recent queries array (same shape as QueryFeed). Calls `bucketByMinute` client-side to derive per-minute counts.

- [ ] **Step 1: Create `src/components/QueriesOverTime.jsx`**

```jsx
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from 'recharts'
import { bucketByMinute } from '../utils'

export default function QueriesOverTime({ queries }) {
  const data = bucketByMinute(queries)

  return (
    <>
      <style>{`
        .queries-over-time {
          background: #1a1d27;
          border-radius: 8px;
          padding: 20px;
        }
        .queries-over-time__title {
          font-size: 14px;
          font-weight: 600;
          color: #94a3b8;
          text-transform: uppercase;
          letter-spacing: 0.05em;
          margin-bottom: 16px;
        }
        .queries-over-time__empty {
          color: #475569;
          font-size: 14px;
          text-align: center;
          padding: 32px 0;
        }
      `}</style>
      <div className="queries-over-time">
        <div className="queries-over-time__title">Queries Over Time</div>
        {data.length === 0 ? (
          <div className="queries-over-time__empty">No data yet.</div>
        ) : (
          <ResponsiveContainer width="100%" height={200}>
            <LineChart data={data} margin={{ top: 4, right: 24, bottom: 0, left: 0 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="#1e2235" />
              <XAxis
                dataKey="time"
                tick={{ fill: '#64748b', fontSize: 11 }}
                axisLine={false}
                tickLine={false}
              />
              <YAxis
                allowDecimals={false}
                tick={{ fill: '#64748b', fontSize: 11 }}
                axisLine={false}
                tickLine={false}
                width={28}
              />
              <Tooltip
                contentStyle={{
                  background: '#1a1d27',
                  border: '1px solid #2d3148',
                  borderRadius: 6,
                  color: '#e2e8f0',
                }}
              />
              <Line
                type="monotone"
                dataKey="count"
                stroke="#4f9eff"
                strokeWidth={2}
                dot={false}
                activeDot={{ r: 4, fill: '#4f9eff' }}
              />
            </LineChart>
          </ResponsiveContainer>
        )}
      </div>
    </>
  )
}
```

- [ ] **Step 2: Verify build still passes**

```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm run build 2>&1 | tail -5
```

Expected: no errors.

- [ ] **Step 3: Run full test suite**

```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm test 2>&1 | tail -10
```

Expected: `21 passed` (no regressions).

- [ ] **Step 4: Commit**

```bash
cd /opt/doh_proxy
git add dns-dashboard-ui/src/components/QueriesOverTime.jsx
git commit -m "feat(dns-dashboard-ui): QueriesOverTime LineChart component"
```

---

## Task 8: App Layout, Polling, and Global Styles

**Files:**
- Modify: `dns-dashboard-ui/src/App.jsx`
- Modify: `dns-dashboard-ui/src/App.css`
- Delete: `dns-dashboard-ui/src/assets/react.svg` (generated by scaffold, unused)

- [ ] **Step 1: Replace `src/App.css` with dark theme and grid layout**

```css
*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

body {
  background: #0f1117;
  color: #e2e8f0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  min-height: 100vh;
}

.app {
  max-width: 1400px;
  margin: 0 auto;
  padding: 0 24px 40px;
}

.app-header {
  padding: 24px 0 20px;
  border-bottom: 1px solid #1e2235;
  margin-bottom: 24px;
}

.app-header h1 {
  font-size: 22px;
  font-weight: 700;
  color: #e2e8f0;
  letter-spacing: -0.02em;
}

.app-header h1 span {
  color: #4f9eff;
}

.app-header__sub {
  font-size: 12px;
  color: #475569;
  margin-top: 4px;
}

.section {
  margin-bottom: 20px;
}

.bottom-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
}

@media (max-width: 900px) {
  .bottom-grid {
    grid-template-columns: 1fr;
  }
}
```

- [ ] **Step 2: Replace `src/App.jsx` with full implementation**

```jsx
import { useState, useEffect } from 'react'
import './App.css'
import { fetchStats, fetchTopDomains, fetchRecentQueries } from './api'
import StatCards from './components/StatCards'
import TopDomains from './components/TopDomains'
import QueryFeed from './components/QueryFeed'
import QueriesOverTime from './components/QueriesOverTime'

const EMPTY_STATS = { total: 0, blocked: 0, avg_latency_ms: null }

export default function App() {
  const [stats, setStats] = useState(EMPTY_STATS)
  const [domains, setDomains] = useState([])
  const [queries, setQueries] = useState([])

  async function refresh() {
    try {
      const [s, d, q] = await Promise.all([
        fetchStats(),
        fetchTopDomains(),
        fetchRecentQueries(),
      ])
      setStats(s)
      setDomains(d)
      setQueries(q)
    } catch (err) {
      console.error('Failed to refresh dashboard data:', err)
    }
  }

  useEffect(() => {
    refresh()
    const id = setInterval(refresh, 5000)
    return () => clearInterval(id)
  }, [])

  return (
    <div className="app">
      <header className="app-header">
        <h1><span>DNS</span> Stats</h1>
        <div className="app-header__sub">Refreshes every 5 seconds</div>
      </header>

      <div className="section">
        <StatCards stats={stats} />
      </div>

      <div className="section">
        <QueriesOverTime queries={queries} />
      </div>

      <div className="bottom-grid">
        <TopDomains domains={domains} />
        <QueryFeed queries={queries} />
      </div>
    </div>
  )
}
```

- [ ] **Step 3: Clean up scaffold files**

```bash
cd /opt/doh_proxy/dns-dashboard-ui
rm -f src/assets/react.svg public/vite.svg
```

Remove the logo import from `src/main.jsx` if it references deleted files. Replace `src/main.jsx` with the minimal version:

```jsx
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.jsx'

createRoot(document.getElementById('root')).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
```

Replace `src/index.css` (the scaffold default) with just a reset:

```css
:root {
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
}
```

- [ ] **Step 4: Update the page title**

Edit `dns-dashboard-ui/index.html` — change the `<title>` tag:

```html
<title>DNS Stats Dashboard</title>
```

- [ ] **Step 5: Build to confirm no errors**

```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm run build 2>&1 | tail -8
```

Expected: `✓ built in` with no errors.

- [ ] **Step 6: Run full test suite**

```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm test 2>&1 | tail -10
```

Expected: `21 passed`.

- [ ] **Step 7: Smoke test in browser**

Start the backend (in a separate terminal):
```bash
cd /opt/doh_proxy && DATABASE_URL="sqlite://dashboard.db" cargo run -p dns-dashboard
```

Start the frontend dev server:
```bash
cd /opt/doh_proxy/dns-dashboard-ui && npm run dev
```

Open `http://localhost:5173` in a browser. Verify:
- Dark background, "DNS Stats" header
- Three stat cards showing 0 counts
- Empty state messages in charts
- No console errors

- [ ] **Step 8: Commit**

```bash
cd /opt/doh_proxy
git add dns-dashboard-ui/src/App.jsx dns-dashboard-ui/src/App.css dns-dashboard-ui/src/main.jsx dns-dashboard-ui/src/index.css dns-dashboard-ui/index.html
git commit -m "feat(dns-dashboard-ui): App layout, polling, dark theme, final wiring"
```

---

## Self-Review

### Spec coverage

| Spec requirement | Task |
|---|---|
| Vite + React (no TypeScript) | Task 1 |
| recharts, axios | Task 1 |
| Vite proxy `/api` → `http://localhost:4000` | Task 1 |
| `src/api.js` — fetchRecentQueries, fetchTopDomains, fetchStats | Task 2 |
| `src/components/StatCards.jsx` — 3 cards, block rate %, avg latency | Task 4 |
| `src/components/TopDomains.jsx` — horizontal BarChart, domain Y axis, count X axis | Task 6 |
| `src/components/QueryFeed.jsx` — table with Time/Domain/Type/Latency/Blocked/Resolver | Task 5 |
| `src/components/QueriesOverTime.jsx` — LineChart, client-side bucketing into 1-min intervals | Task 7 |
| `src/App.jsx` — dark header "DNS Stats", StatCards top, QueriesOverTime full-width, TopDomains+QueryFeed side-by-side | Task 8 |
| Poll all endpoints every 5s with setInterval in useEffect | Task 8 |
| Store all data in state | Task 8 |
| Dark theme: #0f1117 bg, #1a1d27 card, #4f9eff accent, #e2e8f0 text | Tasks 4,5,6,7,8 |
| Monospace font in query feed | Task 5 |
| `src/App.css` for global styles | Task 8 |
| Component-level `<style>` tags | Tasks 4,5,6,7 |

All requirements covered. No gaps.

### Placeholder scan

No TBD/TODO/placeholder language. All code steps are complete.

### Type consistency

- `fetchRecentQueries()` defined in Task 2, called in Task 8 `App.jsx` ✓
- `fetchTopDomains()` defined in Task 2, called in Task 8 ✓
- `fetchStats()` defined in Task 2, called in Task 8 ✓
- `bucketByMinute(queries)` defined in Task 3, imported in Task 7 from `'../utils'` ✓
- `<StatCards stats={stats} />` — `stats` is `{ total, blocked, avg_latency_ms }` from `fetchStats()` ✓
- `<TopDomains domains={domains} />` — `domains` is `Array<{ domain, count }>` from `fetchTopDomains()` ✓
- `<QueryFeed queries={queries} />` — `queries` is the recent queries array from `fetchRecentQueries()` ✓
- `<QueriesOverTime queries={queries} />` — same array, passed to `bucketByMinute` inside ✓
