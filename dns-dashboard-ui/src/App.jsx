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

  useEffect(() => {
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
